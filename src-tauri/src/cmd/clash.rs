use super::CmdResult;
use crate::feat;
use crate::{
    cmd::StringifyErr as _,
    config::{ClashInfo, Config},
    core::{CoreManager, handle},
};
use clash_verge_logging::{Type, logging, logging_error};
use compact_str::CompactString;
use serde_yaml_ng::Mapping;
use smartstring::alias::String;

/// 复制Clash环境变量
#[tauri::command]
pub async fn copy_clash_env() -> CmdResult {
    feat::copy_clash_env().await;
    Ok(())
}

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

/// 启动核心
#[tauri::command]
pub async fn start_core() -> CmdResult {
    let result = CoreManager::global().start_core().await.stringify_err();
    if result.is_ok() {
        handle::Handle::refresh_clash();
    }
    result
}

/// 关闭核心
#[tauri::command]
pub async fn stop_core() -> CmdResult {
    logging_error!(Type::Core, Config::profiles().await.data_arc().save_file());
    let result = CoreManager::global().stop_core().await.stringify_err();
    if result.is_ok() {
        handle::Handle::refresh_clash();
    }
    result
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

// L2-08: 延迟测试失败时返回的魔法数字常量
pub const DELAY_ERROR_VALUE: u32 = 10000;

/// 测试URL延迟
#[tauri::command]
pub async fn test_delay(url: String) -> CmdResult<u32> {
    let result = match feat::test_delay(url).await {
        Ok(delay) => delay,
        Err(e) => {
            logging!(error, Type::Cmd, "{}", e);
            DELAY_ERROR_VALUE
        }
    };
    Ok(result)
}

#[tauri::command]
pub async fn get_clash_logs() -> CmdResult<Vec<CompactString>> {
    let logs = CoreManager::global().get_clash_logs().await.unwrap_or_default();
    Ok(logs)
}

/// 关闭所有连接
#[tauri::command]
pub async fn close_all_connections() -> CmdResult {
    let mihomo = handle::Handle::mihomo().await;
    mihomo.close_all_connections().await.stringify_err()?;
    drop(mihomo);
    Ok(())
}
