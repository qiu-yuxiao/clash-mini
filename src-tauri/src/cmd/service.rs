use super::{CmdResult, StringifyErr as _};
use crate::core::service::{self, SERVICE_MANAGER, ServiceStatus};
use clash_verge_logging::{Type, logging};

async fn execute_service_operation_sync(status: ServiceStatus, op_type: &str) -> CmdResult {
    SERVICE_MANAGER
        .handle_service_status(status)
        .await
        // 【诊断留痕】服务操作失败此前只把错误返回前端、后端零日志，
        // 排查时 latest.log 在「需要安装服务」之后一片空白（BUG-298 现场即如此）。
        .map_err(|e| {
            logging!(error, Type::Service, "{op_type} service 操作失败: {e}");
            format!("{op_type} Service failed: {e}").into()
        })
}

#[tauri::command]
pub async fn install_service() -> CmdResult {
    execute_service_operation_sync(ServiceStatus::InstallRequired, "Install").await
}

#[tauri::command]
pub async fn uninstall_service() -> CmdResult {
    execute_service_operation_sync(ServiceStatus::UninstallRequired, "Uninstall").await
}

#[tauri::command]
pub async fn reinstall_service() -> CmdResult {
    execute_service_operation_sync(ServiceStatus::ReinstallRequired, "Reinstall").await
}

#[tauri::command]
pub async fn repair_service() -> CmdResult {
    execute_service_operation_sync(ServiceStatus::ForceReinstallRequired, "Repair").await
}

#[tauri::command]
pub async fn is_service_available() -> CmdResult<bool> {
    service::is_service_available().await.stringify_err()?;
    Ok(true)
}
