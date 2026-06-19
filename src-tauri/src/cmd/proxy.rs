use super::CmdResult;
use super::StringifyErr as _;
use crate::core::tray::Tray;
use crate::process::AsyncHandler;
use crate::utils::dirs;
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static TRAY_SYNC_RUNNING: AtomicBool = AtomicBool::new(false);
static TRAY_SYNC_PENDING: AtomicBool = AtomicBool::new(false);

/// 同步托盘和GUI的代理选择状态
#[tauri::command]
pub async fn sync_tray_proxy_selection() -> CmdResult<()> {
    if TRAY_SYNC_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        AsyncHandler::spawn(move || async move {
            run_tray_sync_loop().await;
        });
    } else {
        TRAY_SYNC_PENDING.store(true, Ordering::Release);
    }

    Ok(())
}

async fn run_tray_sync_loop() {
    loop {
        match Tray::global().update_menu().await {
            Ok(_) => {
                logging!(info, Type::Cmd, "Tray proxy selection synced successfully");
            }
            Err(e) => {
                logging!(error, Type::Cmd, "Failed to sync tray proxy selection: {e}");
            }
        }

        if !TRAY_SYNC_PENDING.swap(false, Ordering::AcqRel) {
            TRAY_SYNC_RUNNING.store(false, Ordering::Release);

            if TRAY_SYNC_PENDING.swap(false, Ordering::AcqRel)
                && TRAY_SYNC_RUNNING
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                continue;
            }

            break;
        }
    }
}

/// 根据节点名称和provider名称获取其server和port
#[tauri::command]
pub async fn get_proxy_addr(
    name: std::string::String,
    provider: Option<std::string::String>,
) -> CmdResult<Option<(std::string::String, u16)>> {
    let app_dir = dirs::app_home_dir().stringify_err()?;

    // 1. 如果指定了 provider，优先从 provider 的配置文件中查询
    if let Some(provider_name) = provider {
        let config_path = app_dir.join("clash-mini.yaml");
        if config_path.exists()
            && let Ok(config_content) = tokio::fs::read_to_string(&config_path).await
            && let Ok(Value::Mapping(config_map)) = serde_yaml_ng::from_str::<Value>(&config_content)
            && let Some(Value::Mapping(providers)) = config_map.get("proxy-providers")
            && let Some(Value::Mapping(provider_info)) = providers.get(provider_name.as_str())
            && let Some(Value::String(path)) = provider_info.get("path")
        {
            let mut provider_path = PathBuf::from(path.as_str());
            if provider_path.is_relative() {
                provider_path = app_dir.join(provider_path);
            }
            if provider_path.exists()
                && let Ok(provider_content) = tokio::fs::read_to_string(&provider_path).await
                && let Ok(Value::Mapping(provider_map)) = serde_yaml_ng::from_str::<Value>(&provider_content)
                && let Some(Value::Sequence(proxies)) = provider_map.get("proxies")
            {
                for proxy in proxies {
                    if let Some(proxy_map) = proxy.as_mapping()
                        && let Some(Value::String(proxy_name)) = proxy_map.get("name")
                        && proxy_name.as_str() == name
                    {
                        let server = proxy_map
                            .get("server")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let port = proxy_map.get("port").and_then(|v| v.as_i64()).unwrap_or(0) as u16;
                        return Ok(Some((server, port)));
                    }
                }
            }
        }
    }

    // 2. 默认 fallback: 从 clash-mini.yaml 的 proxies 列表中查询
    let config_path = app_dir.join("clash-mini.yaml");
    if config_path.exists()
        && let Ok(config_content) = tokio::fs::read_to_string(&config_path).await
        && let Ok(Value::Mapping(config_map)) = serde_yaml_ng::from_str::<Value>(&config_content)
        && let Some(Value::Sequence(proxies)) = config_map.get("proxies")
    {
        for proxy in proxies {
            if let Some(proxy_map) = proxy.as_mapping()
                && let Some(Value::String(proxy_name)) = proxy_map.get("name")
                && proxy_name.as_str() == name
            {
                let server = proxy_map
                    .get("server")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let port = proxy_map.get("port").and_then(|v| v.as_i64()).unwrap_or(0) as u16;
                return Ok(Some((server, port)));
            }
        }
    }

    Ok(None)
}

/// 保存前端的代理组头部状态（过滤、排序选择）到本地硬盘
#[tauri::command]
pub async fn save_proxy_head_state(state: serde_json::Value) -> CmdResult<()> {
    let path = dirs::app_home_dir().stringify_err()?.join("proxy_head_state.json");
    let content = serde_json::to_string_pretty(&state).stringify_err()?;
    tokio::fs::write(path, content).await.stringify_err()?;
    Ok(())
}

/// 从本地硬盘读取代理组头部状态
#[tauri::command]
pub async fn get_proxy_head_state() -> CmdResult<serde_json::Value> {
    let path = dirs::app_home_dir().stringify_err()?.join("proxy_head_state.json");
    if !path.exists() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    let content = tokio::fs::read_to_string(path).await.stringify_err()?;
    let val = serde_json::from_str(&content).stringify_err()?;
    Ok(val)
}

/// 触发后端自动优选并切换到最快节点（如果是手动触发，则返回最优节点延迟信息）
#[tauri::command]
/// 触发后台自动选点
/// is_manual 参数已废弃（保留向后兼容），无论 true/false 都返回完整结果
/// sort_type: None=从配置文件读取, Some(0)=同前, Some(1)=按延迟, Some(2)=按名称
pub async fn trigger_auto_select(
    _is_manual: bool,  // 修复编译警告：添加下划线前缀
    sort_type: Option<i32>,
) -> CmdResult<Vec<(std::string::String, u32)>> {
    let profiles = crate::config::Config::profiles().await;
    if let Some(ref current_uid) = profiles.data_arc().current {
        let current_uid_str = current_uid.to_string();
        let res = crate::module::monitor::trigger_backend_auto_select(
            &current_uid_str,
            sort_type.unwrap_or(1),
        )
        .await
        .stringify_err()?;
        return Ok(res);
    }
    Ok(vec![])
}
