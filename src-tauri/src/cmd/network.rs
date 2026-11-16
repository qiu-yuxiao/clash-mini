use super::CmdResult;
use crate::cmd::StringifyErr as _;
use crate::core::sysopt::Sysopt;
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::Mapping;
use sysproxy::Sysproxy;

/// get the system proxy
#[tauri::command]
pub async fn get_sys_proxy() -> CmdResult<Mapping> {
    logging!(debug, Type::Network, "异步获取系统代理配置");

    Sysopt::global().wait_idle().await;
    let sys_proxy = Sysproxy::get_system_proxy().stringify_err()?;
    let Sysproxy {
        ref host,
        ref bypass,
        ref port,
        ref enable,
    } = sys_proxy;

    let mut map = Mapping::new();
    map.insert("enable".into(), (*enable).into());
    map.insert("server".into(), format!("{}:{}", host, port).into());
    map.insert("bypass".into(), bypass.as_str().into());

    logging!(
        debug,
        Type::Network,
        "返回系统代理配置: enable={}, {}:{}",
        sys_proxy.enable,
        sys_proxy.host,
        sys_proxy.port
    );
    Ok(map)
}

#[tauri::command]
pub async fn is_port_in_use(port: u16) -> bool {
    // M2-12: 改用异步 TcpListener 避免阻塞 IPC 线程
    tokio::net::TcpListener::bind(("127.0.0.1", port)).await.is_err()
}
