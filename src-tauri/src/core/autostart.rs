#[cfg(target_os = "windows")]
use crate::utils::schtasks;
use crate::{config::Config, core::handle::Handle};
use anyhow::Result;
#[cfg(not(target_os = "windows"))]
use clash_verge_logging::logging_error;
use clash_verge_logging::{Type, logging};
#[cfg(not(target_os = "windows"))]
use tauri_plugin_autostart::ManagerExt as _;
#[cfg(target_os = "windows")]
use crate::utils::sysinfo::is_current_app_handle_admin;

/// Update the application auto-launch configuration.
///
/// WARNING: DO NOT query `Config::verge().await.latest_arc().enable_auto_launch` to determine the target state during a configuration patch.
/// Doing so causes a timing race condition (the draft config hasn't been applied yet, so it reads the old state, causing toggling ON to disable, and toggling OFF to enable).
/// ALWAYS pass the target state (`enable_auto_launch`) from the patch.
pub async fn update_launch(enable_auto_launch: Option<bool>) -> Result<()> {
    let is_enable = match enable_auto_launch {
        Some(val) => val,
        None => Config::verge().await.latest_arc().enable_auto_launch.unwrap_or(false),
    };
    logging!(info, Type::System, "Setting auto-launch enabled state to: {is_enable}");

    #[cfg(target_os = "windows")]
    {
        let is_admin = is_current_app_handle_admin(Handle::app_handle());
        schtasks::set_auto_launch(is_enable, is_admin).await?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let app_handle = Handle::app_handle();
        let autostart_manager = app_handle.autolaunch();
        if is_enable {
            logging_error!(Type::System, "{:?}", autostart_manager.enable());
        } else {
            logging_error!(Type::System, "{:?}", autostart_manager.disable());
        }
    }

    Ok(())
}

pub fn get_launch_status() -> Result<bool> {
    #[cfg(target_os = "windows")]
    {
        let enabled = schtasks::is_auto_launch_enabled();
        if let Ok(status) = enabled {
            logging!(info, Type::System, "Auto-launch status (scheduled task): {status}");
        }
        enabled
    }

    #[cfg(not(target_os = "windows"))]
    {
        let app_handle = Handle::app_handle();
        let autostart_manager = app_handle.autolaunch();
        match autostart_manager.is_enabled() {
            Ok(status) => {
                logging!(info, Type::System, "Auto-launch status: {status}");
                Ok(status)
            }
            Err(e) => {
                logging!(error, Type::System, "Failed to get auto-launch status: {e}");
                Err(anyhow::anyhow!("Failed to get auto-launch status: {}", e))
            }
        }
    }
}
