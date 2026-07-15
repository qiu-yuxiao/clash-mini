use std::sync::Arc;

use clash_verge_logging::{Type, logging};

use crate::core::{CoreManager, manager::RunningMode};

/// 获取当前内核运行模式
#[tauri::command]
pub async fn get_running_mode() -> Result<Arc<RunningMode>, String> {
    Ok(CoreManager::global().get_running_mode())
}

/// 前端日志转发：将前端诊断日志写入后端日志文件
/// 用于在 UI 线程卡死时（DevTools 无法打开），仍能在 latest.log 中看到前端 IPC 调用时间线
#[tauri::command]
pub async fn frontend_log(level: String, message: String) -> Result<(), String> {
    match level.as_str() {
        "error" => logging!(error, Type::Frontend, "[Frontend] {}", message),
        "warn" => logging!(warn, Type::Frontend, "[Frontend] {}", message),
        _ => logging!(info, Type::Frontend, "[Frontend] {}", message),
    }
    Ok(())
}
