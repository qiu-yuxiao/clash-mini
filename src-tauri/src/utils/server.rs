use super::resolve;
use crate::{
    config::{Config, DEFAULT_PAC, IVerge},
    module::lightweight,
    process::AsyncHandler,
    utils::window_manager::WindowManager,
};
use anyhow::Result;
use clash_verge_logging::{Type, logging, logging_error};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use reqwest::ClientBuilder;
use smartstring::alias::String;
use std::time::Duration;
use tokio::sync::oneshot;
use warp::Filter as _;

#[derive(serde::Deserialize, Debug)]
struct QueryParam {
    param: String,
}

// 关闭 embedded server 的信号发送端
// 关闭 embedded server 的信号发送端
static SHUTDOWN_SENDER: OnceCell<Mutex<Option<oneshot::Sender<()>>>> = OnceCell::new();

// 暂存第一个实例初始 bind 成功的 TcpListener，防止 TOCTOU 时间差漏洞
static SINGLETON_LISTENER: OnceCell<Mutex<Option<std::net::TcpListener>>> = OnceCell::new();

// 创建独占监听的本地 TCP Socket（Windows 下严禁设置 SO_REUSEADDR：WinSock2 允许抢绑仍在监听的端口，破坏单实例互斥，BUG-414）
fn bind_socket(port: u16) -> Result<std::net::TcpListener> {
    use socket2::{Domain, Protocol, SockAddr, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;

    let address = std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), port);
    socket.bind(&SockAddr::from(address))?;
    socket.listen(128)?;
    Ok(std::net::TcpListener::from(socket))
}

enum SingletonProbeTarget {
    Scheme(std::string::String),
    Ping,
    Visible,
}

fn resolve_singleton_probe_target_from_args<'a>(args: impl IntoIterator<Item = &'a str>) -> SingletonProbeTarget {
    let mut saw_silent = false;
    for arg in args {
        let trimmed = arg.trim();
        if resolve::scheme::is_supported_scheme_url(trimmed) {
            return SingletonProbeTarget::Scheme(trimmed.to_string());
        }
        if trimmed == "--silent" {
            saw_silent = true;
        }
    }
    if saw_silent {
        SingletonProbeTarget::Ping
    } else {
        SingletonProbeTarget::Visible
    }
}

#[cfg(test)]
fn resolve_singleton_probe_target(first_arg: Option<&str>) -> SingletonProbeTarget {
    resolve_singleton_probe_target_from_args(first_arg)
}

fn is_valid_singleton_reply(expected_ping: bool, status: reqwest::StatusCode, body: &str) -> bool {
    if !status.is_success() {
        return false;
    }
    let trimmed = body.trim();
    if expected_ping {
        trimmed == "clash-mini-ok"
    } else {
        trimmed == "ok"
    }
}

enum WakeInstanceOutcome {
    NotifiedExisting,
    TransientConnectRefused,
    UnexpectedResponse(std::string::String),
}

async fn probe_or_notify_existing_instance<'a>(
    client: &reqwest::Client,
    port: u16,
    args: impl IntoIterator<Item = &'a str>,
) -> WakeInstanceOutcome {
    let target = resolve_singleton_probe_target_from_args(args);
    let (request, expected_ping) = match target {
        SingletonProbeTarget::Scheme(param) => (
            client
                .get(format!("http://127.0.0.1:{port}/commands/scheme"))
                .query(&[("param", param)]),
            false,
        ),
        SingletonProbeTarget::Ping => (client.get(format!("http://127.0.0.1:{port}/commands/ping")), true),
        SingletonProbeTarget::Visible => (client.get(format!("http://127.0.0.1:{port}/commands/visible")), false),
    };

    match request.send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if is_valid_singleton_reply(expected_ping, status, &body) {
                WakeInstanceOutcome::NotifiedExisting
            } else {
                WakeInstanceOutcome::UnexpectedResponse(format!("status={status}, body={}", body.trim()))
            }
        }
        Err(err) if err.is_connect() => WakeInstanceOutcome::TransientConnectRefused,
        Err(err) => WakeInstanceOutcome::UnexpectedResponse(err.to_string()),
    }
}

/// check whether there is already exists
pub async fn check_singleton() -> Result<()> {
    let port = IVerge::get_singleton_port();

    // 立即尝试独占绑定端口以占位，避免检查与占用之间的 TOCTOU 时间差
    match bind_socket(port) {
        Ok(listener) => {
            // 绑定成功，说明当前是第一个运行的实例，将其存入全局变量供后续 embed_server 使用
            let _ = SINGLETON_LISTENER.set(Mutex::new(Some(listener)));
            Ok(())
        }
        Err(first_bind_err) => {
            // 绑定失败，说明端口已被第一个实例或其它程序占用，先显式绕过代理探测既有实例
            let client = ClientBuilder::new()
                .no_proxy()
                .timeout(Duration::from_millis(1500))
                .build()?;
            let argvs: Vec<std::string::String> = std::env::args().collect();

            match probe_or_notify_existing_instance(
                &client,
                port,
                argvs.iter().skip(1).map(std::string::String::as_str),
            )
            .await
            {
                WakeInstanceOutcome::NotifiedExisting => {
                    logging!(info, Type::Window, "已有实例已通知，当前进程静默退出");
                    std::process::exit(0);
                }
                WakeInstanceOutcome::TransientConnectRefused => {
                    // 快速重启可能遇前序进程退出瞬间端口释放延迟，执行 3 × 150ms 短退避重绑
                    for _ in 0..3 {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        if let Ok(listener) = bind_socket(port) {
                            let _ = SINGLETON_LISTENER.set(Mutex::new(Some(listener)));
                            return Ok(());
                        }
                    }
                    Err(anyhow::anyhow!(
                        "单实例端口 127.0.0.1:{port} 绑定失败且既有实例无响应: {first_bind_err}"
                    ))
                }
                WakeInstanceOutcome::UnexpectedResponse(detail) => Err(anyhow::anyhow!(
                    "单实例端口 127.0.0.1:{port} 已被占用且未收到有效 Clash Mini 响应 ({detail})"
                )),
            }
        }
    }
}

/// The embed server only be used to implement singleton process
/// maybe it can be used as pac server later
pub fn embed_server() {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    #[allow(clippy::expect_used)]
    SHUTDOWN_SENDER
        .set(Mutex::new(Some(shutdown_tx)))
        .expect("failed to set shutdown signal for embedded server");

    // 从全局缓存中取出第一个实例抢占的 std::net::TcpListener
    #[allow(clippy::expect_used)]
    let std_listener = {
        let lock = SINGLETON_LISTENER.get().expect("SINGLETON_LISTENER not set");
        let mut guard = lock.lock();
        guard.take().expect("TcpListener already taken")
    };

    // 设置非阻塞并在物理隔离的 DAEMON_RUNTIME 内转换为 tokio TcpListener（BUG-394）
    #[allow(clippy::expect_used)]
    std_listener.set_nonblocking(true).expect("failed to set nonblocking");

    AsyncHandler::spawn_daemon(move || async move {
        let visible = warp::path!("commands" / "visible").and_then(|| async {
            AsyncHandler::spawn_daemon(|| async {
                // BUG-395：开启「启动时最小化」且已有订阅时，再次点击启动图标一律保持静默托盘驻留，
                // 仅通过原生气泡提示而不弹出主窗口，防止露出窗口再关闭触发 360 关窗残留清理。
                if lightweight::should_enter_silent_tray_on_launch().await {
                    logging!(
                        info,
                        Type::Window,
                        "已开启启动时最小化，点击启动图标保持托盘静默驻留，不弹出主窗口"
                    );
                    if !lightweight::is_in_lightweight_mode() && !WindowManager::is_main_window_ready() {
                        let _ = lightweight::entry_lightweight_mode().await;
                    }
                    crate::core::tray::Tray::global().notify_silent_running().await;
                    return;
                }

                logging!(info, Type::Window, "检测到从单例模式恢复应用窗口");
                AsyncHandler::spawn(|| async {
                    if !lightweight::exit_lightweight_mode().await {
                        logging!(
                            warn,
                            Type::Window,
                            "轻量模式退出未完成（可能正在防抖限流或窗口初始化超时）"
                        );
                    }
                });
            });
            Ok::<_, warp::Rejection>(warp::reply::with_status::<std::string::String>(
                "ok".to_string(),
                warp::http::StatusCode::OK,
            ))
        });

        let pac = warp::path!("commands" / "pac").and_then(|| async move {
            let verge_config = Config::verge().await;
            let clash_config = Config::clash().await;

            let pac_content = verge_config
                .data_arc()
                .pac_file_content
                .clone()
                .unwrap_or_else(|| DEFAULT_PAC.into());

            let pac_port = verge_config
                .data_arc()
                .verge_mixed_port
                .unwrap_or_else(|| clash_config.data_arc().get_mixed_port());
            let processed_content = pac_content.replace("%mixed-port%", &format!("{pac_port}"));
            Ok::<_, warp::Rejection>(
                warp::http::Response::builder()
                    .header("Content-Type", "application/x-ns-proxy-autoconfig")
                    .body(processed_content)
                    .unwrap_or_default(),
            )
        });

        // Use map instead of and_then to avoid Send issues
        let scheme = warp::path!("commands" / "scheme")
            .and(warp::query::<QueryParam>())
            .and_then(|query: QueryParam| async move {
                AsyncHandler::spawn(|| async move {
                    logging_error!(Type::Setup, resolve::resolve_scheme(&query.param).await);
                });
                Ok::<_, warp::Rejection>(warp::reply::with_status::<std::string::String>(
                    "ok".to_string(),
                    warp::http::StatusCode::OK,
                ))
            });

        let ping = warp::path!("commands" / "ping").and_then(|| async {
            Ok::<_, warp::Rejection>(warp::reply::with_status::<std::string::String>(
                "clash-mini-ok".to_string(),
                warp::http::StatusCode::OK,
            ))
        });

        let commands = visible.or(scheme).or(pac).or(ping);

        #[allow(clippy::expect_used)]
        let tokio_listener = tokio::net::TcpListener::from_std(std_listener).expect("failed to convert TcpListener");

        warp::serve(commands)
            .incoming(tokio_listener)
            .graceful(async {
                shutdown_rx.await.ok();
            })
            .run()
            .await;
    });
}

pub fn shutdown_embedded_server() {
    logging!(info, Type::Window, "shutting down embedded server");
    if let Some(sender) = SHUTDOWN_SENDER.get()
        && let Some(sender) = sender.lock().take()
    {
        sender.send(()).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleton_probe_target_and_reply_validation() {
        assert!(matches!(
            resolve_singleton_probe_target(None),
            SingletonProbeTarget::Visible
        ));
        assert!(matches!(
            resolve_singleton_probe_target(Some("--silent")),
            SingletonProbeTarget::Ping
        ));
        assert!(matches!(
            resolve_singleton_probe_target(Some("clash-mini://install-config?url=x")),
            SingletonProbeTarget::Scheme(_)
        ));
        assert!(matches!(
            resolve_singleton_probe_target(Some("Clash://install-config?url=x")),
            SingletonProbeTarget::Scheme(_)
        ));
        assert!(matches!(
            resolve_singleton_probe_target_from_args(["--silent", "clash://install-config?url=x"]),
            SingletonProbeTarget::Scheme(_)
        ));

        assert!(is_valid_singleton_reply(false, reqwest::StatusCode::OK, "ok\n"));
        assert!(!is_valid_singleton_reply(false, reqwest::StatusCode::OK, "other-app"));
        assert!(!is_valid_singleton_reply(false, reqwest::StatusCode::NOT_FOUND, "ok"));

        assert!(is_valid_singleton_reply(true, reqwest::StatusCode::OK, "clash-mini-ok"));
        assert!(!is_valid_singleton_reply(true, reqwest::StatusCode::OK, "ok"));
    }
}
