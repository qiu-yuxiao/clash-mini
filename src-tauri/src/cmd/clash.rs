use super::CmdResult;
use crate::feat;
use crate::{
    cmd::StringifyErr as _,
    config::{ClashInfo, Config},
    core::{CoreManager, handle},
};
use clash_verge_logging::{Type, logging_error};
use compact_str::CompactString;
use serde_yaml_ng::Mapping;
use smartstring::alias::String;

/// 获取Clash信息
#[tauri::command]
pub async fn get_clash_info() -> CmdResult<ClashInfo> {
    Ok(Config::clash().await.data_arc().get_client_info())
}

/// 修改Clash配置
#[tauri::command]
pub async fn patch_clash_config(payload: Mapping) -> CmdResult {
    feat::patch_clash(&payload).await.stringify_err()
}

/// 修改Clash模式
#[tauri::command]
pub async fn patch_clash_mode(payload: String) -> CmdResult {
    feat::change_clash_mode(payload).await.stringify_err()
}

/// 重启核心
#[tauri::command]
pub async fn restart_core() -> CmdResult {
    logging_error!(Type::Core, Config::profiles().await.data_arc().save_file());
    let result = CoreManager::global().restart_core().await.stringify_err();
    if result.is_ok() {
        handle::Handle::refresh_clash();
    }
    result
}

#[tauri::command]
pub async fn get_clash_logs() -> CmdResult<Vec<CompactString>> {
    let logs = CoreManager::global().get_clash_logs().await.unwrap_or_default();
    Ok(logs)
}
