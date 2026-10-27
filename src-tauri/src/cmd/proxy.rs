use super::CmdResult;
use super::StringifyErr as _;
use crate::utils::dirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
struct ProxyAddrEntry {
    name: Option<String>,
    server: Option<String>,
    port: Option<u16>,
}

#[derive(Deserialize)]
struct ProviderEntry {
    path: Option<String>,
}

#[derive(Deserialize)]
struct QuickConfigLookup {
    #[serde(rename = "proxy-providers")]
    providers: Option<HashMap<String, ProviderEntry>>,
    proxies: Option<Vec<ProxyAddrEntry>>,
}

#[derive(Deserialize)]
struct ProviderFileLookup {
    proxies: Option<Vec<ProxyAddrEntry>>,
}

/// 前端委托后端执行群发测速与择优（根治前后端重复测速/选点冲突）
/// - `profile_uid`: 当前活动配置 UID
/// - `node_names`: 待测节点子集；传非空列表则在「该子集」内测速并挑最快，
///   传 None/空则后端自取 PROXY 全量节点（F1/自动选点场景）
/// - `sort_type`: 0=从配置读取, 1=按延迟升序, 2=按名称（仅影响展示顺序）
/// - `select`: true=测速后将 PROXY 切换至最快节点；false=仅测速填充展示，不切换
#[tauri::command]
pub async fn trigger_auto_select(
    profile_uid: String,
    node_names: Option<Vec<String>>,
    sort_type: i32,
    select: bool,
) -> CmdResult<Vec<(String, u32)>> {
    let outcome =
        crate::module::monitor::trigger_backend_auto_select(&profile_uid, node_names, sort_type, select, false)
            .await
            .stringify_err()?;
    Ok(outcome.display)
}

/// 根据节点名称和provider名称获取其server和port
#[tauri::command]
pub async fn get_proxy_addr(
    name: std::string::String,
    provider: Option<std::string::String>,
) -> CmdResult<Option<(std::string::String, u16)>> {
    let app_dir = dirs::app_home_dir().stringify_err()?;
    let config_path = app_dir.join("clash-mini.yaml");

    let Ok(config_content) = tokio::fs::read_to_string(&config_path).await else {
        return Ok(None);
    };
    let Ok(parsed_config) = serde_yaml_ng::from_str::<QuickConfigLookup>(&config_content) else {
        return Ok(None);
    };

    // 1. 如果指定了 provider，优先从 provider 的配置文件中查询
    if let Some(provider_name) = provider
        && let Some(providers) = &parsed_config.providers
        && let Some(provider_info) = providers.get(provider_name.as_str())
        && let Some(path) = &provider_info.path
    {
        let mut provider_path = PathBuf::from(path.as_str());
        if provider_path.is_relative() {
            provider_path = app_dir.join(provider_path);
        }
        if let Ok(provider_content) = tokio::fs::read_to_string(&provider_path).await
            && let Ok(provider_parsed) = serde_yaml_ng::from_str::<ProviderFileLookup>(&provider_content)
            && let Some(proxies) = provider_parsed.proxies
        {
            for proxy in proxies {
                if proxy.name.as_deref() == Some(name.as_str()) {
                    let server = proxy.server.unwrap_or_default();
                    let port = proxy.port.unwrap_or(0);
                    return Ok(Some((server, port)));
                }
            }
        }
    }

    // 2. 默认 fallback: 复用已解析的 clash-mini.yaml 的 proxies 列表查询（避免二次磁盘读取与反序列化）
    if let Some(proxies) = parsed_config.proxies {
        for proxy in proxies {
            if proxy.name.as_deref() == Some(name.as_str()) {
                let server = proxy.server.unwrap_or_default();
                let port = proxy.port.unwrap_or(0);
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
    crate::utils::help::save_json(&path, &state).stringify_err()?;
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
