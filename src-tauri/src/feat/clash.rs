use crate::{
    config::Config,
    constants::timing::NODE_DELAY_MAX_MS,
    core::handle,
    feat::{clean_async, prepare_exit},
    process::AsyncHandler,
};

use clash_verge_logging::{Type, logging};
use once_cell::sync::Lazy;
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::sync::Arc;

/// 互斥保护 change_clash_mode / patch_clash 的并发调用，防止配置修改竞态
pub static CLASH_PATCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// SAFETY: TLS 配置构建使用的是内置的 ring provider 和安全默认协议版本，
// 这些都是经过验证的配置，构建失败的概率极低。
// 这是启动时初始化的全局配置，若失败说明运行环境有严重问题，fail-fast 是合理的。
#[allow(clippy::expect_used)]
static TLS_CONFIG: Lazy<Arc<rustls::ClientConfig>> = Lazy::new(|| {
    let root_store = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("Failed to set TLS versions")
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Arc::new(config)
});

/// Restart the application
pub async fn restart_app() {
    logging!(debug, Type::System, "启动重启应用流程");
    prepare_exit().await;

    logging!(info, Type::System, "开始异步清理资源");
    let cleanup_result = clean_async().await;

    logging!(
        info,
        Type::System,
        "资源清理完成，退出代码: {}",
        if cleanup_result { 0 } else { 1 }
    );

    // 最终刷新日志
    crate::core::logger::Logger::global().shutdown();

    let app_handle = handle::Handle::app_handle();
    app_handle.restart();
}

fn after_change_clash_mode() {
    // M2-03: 直接调用 close_all_connections，避免逐个关闭数千连接时阻塞
    AsyncHandler::spawn(move || async {
        let mihomo = handle::Handle::mihomo().await.clone();
        if let Err(err) = mihomo.close_all_connections().await {
            logging!(error, Type::Core, "Failed to close all connections: {err}");
        }
    });
}

/// Change Clash mode (rule/global/direct/script)
pub async fn change_clash_mode(mode: String) -> anyhow::Result<()> {
    let _guard = CLASH_PATCH_LOCK.lock().await;

    let mut mapping = Mapping::new();
    mapping.insert(Value::from("mode"), Value::from(mode.as_str()));
    let json_value = serde_json::json!({
        "mode": mode
    });
    logging!(debug, Type::Core, "change clash mode to {mode}");
    let mihomo = handle::Handle::mihomo().await.clone();
    match mihomo.patch_base_config(&json_value).await {
        Ok(_) => {
            let clash = Config::clash().await;
            clash.edit_draft(|d| d.patch_config(&mapping));
            clash.apply();

            let clash_data = clash.data_arc();
            if clash_data.save_config().is_ok() {
                handle::Handle::refresh_clash();
            }

            let is_auto_close_connection = Config::verge().await.data_arc().auto_close_connection.unwrap_or(false);
            if is_auto_close_connection {
                after_change_clash_mode();
            }

            Ok(())
        }
        Err(err) => {
            logging!(error, Type::Core, "{err}");
            Err(anyhow::anyhow!("{err}"))
        }
    }
}

/// Test delay to a URL through proxy.
/// HTTPS: measures TLS handshake time. HTTP: measures HEAD round-trip time.
///
/// Note: The TCP stream and TLS connector are created inside the timeout block,
/// so they are automatically dropped when the timeout fires, ensuring no
/// lingering connections remain in the background.
pub async fn test_delay(url: String) -> anyhow::Result<u32> {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::TcpStream;
    use tokio::time::Instant;

    let parsed = tauri::Url::parse(&url)?;
    let is_https = parsed.scheme() == "https";
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?
        .to_string();
    let port = parsed.port().unwrap_or(if is_https { 443 } else { 80 });

    let verge = Config::verge().await.latest_arc();
    let proxy_enabled = verge.enable_system_proxy.unwrap_or(false) || verge.enable_tun_mode.unwrap_or(false);
    // 【注意】此处读取的 proxy_port 在后续使用时可能已变化（如用户修改端口）
    // 但 test_delay 是一次性操作，端口在短时间内变化的概率极低，影响可忽略
    let proxy_port = if proxy_enabled {
        Some(match verge.verge_mixed_port {
            Some(p) => p,
            None => Config::clash().await.data_arc().get_mixed_port(),
        })
    } else {
        None
    };

    tokio::time::timeout(Duration::from_millis(NODE_DELAY_MAX_MS as u64), async {
        let start = Instant::now();
        // L2-01: 改用栈数组避免每次堆分配
        let mut buf = [0u8; 1024];

        if is_https {
            let stream = match proxy_port {
                Some(pp) => {
                    let mut s = TcpStream::connect(format!("127.0.0.1:{pp}")).await?;
                    s.write_all(format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n").as_bytes())
                        .await?;
                    let n = s.read(&mut buf).await?;
                    if !buf[..n].windows(3).any(|w| w == b"200") {
                        return Err(anyhow::anyhow!("Proxy CONNECT failed"));
                    }
                    s
                }
                None => TcpStream::connect(format!("{host}:{port}")).await?,
            };
            let connector = tokio_rustls::TlsConnector::from(Arc::clone(&TLS_CONFIG));
            let server_name = rustls::pki_types::ServerName::try_from(host.as_str())
                .map_err(|_| anyhow::anyhow!("Invalid DNS name: {host}"))?
                .to_owned();
            connector.connect(server_name, stream).await?;
        } else {
            let (mut stream, req) = match proxy_port {
                Some(pp) => (
                    TcpStream::connect(format!("127.0.0.1:{pp}")).await?,
                    format!("HEAD {url} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
                ),
                None => (
                    TcpStream::connect(format!("{host}:{port}")).await?,
                    format!("HEAD / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
                ),
            };
            stream.write_all(req.as_bytes()).await?;
            let _ = stream.read(&mut buf).await?;
        }

        // frontend treats 0 as timeout
        Ok((start.elapsed().as_millis() as u32).max(1))
    })
    .await
    .unwrap_or(Ok(NODE_DELAY_MAX_MS))
}
