use super::CmdResult;
use crate::config::Config;
use serde_yaml_ng::Mapping;

/// 获取运行时配置
#[tauri::command]
pub async fn get_runtime_config() -> CmdResult<Option<Mapping>> {
    Ok(Config::runtime().await.latest_arc().config.clone())
}
