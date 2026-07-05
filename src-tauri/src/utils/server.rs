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

// 创建一个启用了地址与端口重用属性（SO_REUSEADDR / SO_REUSEPORT）的 TCP 绑定
fn bind_socket(port: u16) -> Result<std::net::TcpListener> {
    use socket2::{Socket, Domain, Type, Protocol, SockAddr};
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    
    #[cfg(all(unix, not(target_os = "solaris"), not(target_os = "illumos")))]
    socket.set_reuse_port(true)?;
    
    let address = std::net::SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        port,
    );
    socket.bind(&SockAddr::from(address))?;
    socket.listen(128)?;
    Ok(std::net::TcpListener::from(socket))
}

/// check whether there is already exists
pub async fn check_singleton() -> Result<()> {
    let port = IVerge::get_singleton_port();
    
    // 立即尝试绑定端口以占位，避免检查与占用之间的时间差，并设置 SO_REUSEADDR 解决 TIME_WAIT 冲突
    match bind_socket(port) {
        Ok(listener) => {
            // 绑定成功，说明当前是第一个运行的实例，将其存入全局变量供后续 embed_server 使用
            let _ = SINGLETON_LISTENER.set(Mutex::new(Some(listener)));
            Ok(())
        }
        Err(_) => {
            // 绑定失败，说明端口已被第一个实例或者其他服务占用，执行唤醒逻辑
            let client = ClientBuilder::new().timeout(Duration::from_millis(500)).build()?;
            let argvs: Vec<std::string::String> = std::env::args().collect();
            if argvs.len() > 1 {
                #[cfg(not(target_os = "macos"))]
                {
                    let param = argvs[1].as_str();
                    if param.starts_with("clash:") {
                        client
                            .get(format!("http://127.0.0.1:{port}/commands/scheme?param={param}"))
                            .send()
                            .await?;
                    }
                }
            } else {
                client
                    .get(format!("http://127.0.0.1:{port}/commands/visible"))
                    .send()
                    .await?;
            }
            // 唤醒已有实例后，当前进程静默退出
            logging!(info, Type::Window, "已有实例已通知，当前进程静默退出");
            std::process::exit(0);
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

    let visible = warp::path!("commands" / "visible").and_then(|| async {
        logging!(info, Type::Window, "检测到从单例模式恢复应用窗口");
        tauri::async_runtime::spawn(async {
            if !lightweight::exit_lightweight_mode().await {
                WindowManager::show_main_window().await;
            } else {
                logging!(error, Type::Window, "轻量模式退出失败，无法恢复应用窗口");
            }
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

    let commands = visible.or(scheme).or(pac);

    // 从全局缓存中取出第一个实例抢占的 std::net::TcpListener
    #[allow(clippy::expect_used)]
    let std_listener = {
        let lock = SINGLETON_LISTENER.get().expect("SINGLETON_LISTENER not set");
        let mut guard = lock.lock();
        guard.take().expect("TcpListener already taken")
    };
    
    // 设置非阻塞并转换为 tokio 的 TcpListener
    #[allow(clippy::expect_used)]
    std_listener.set_nonblocking(true).expect("failed to set nonblocking");
    #[allow(clippy::expect_used)]
    let tokio_listener = tokio::net::TcpListener::from_std(std_listener).expect("failed to convert TcpListener");

    AsyncHandler::spawn(move || async move {
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
