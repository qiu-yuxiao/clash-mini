use crate::{config::Config, core::handle::Handle, utils::schtasks, utils::sysinfo::is_current_app_handle_admin};
use anyhow::Result;
use clash_verge_logging::{Type, logging};

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

    let is_admin = is_current_app_handle_admin(Handle::app_handle());
    schtasks::set_auto_launch(is_enable, is_admin)?;

    Ok(())
}

pub fn get_launch_status() -> Result<bool> {
    let enabled = schtasks::is_auto_launch_enabled();
    if let Ok(status) = enabled {
        logging!(info, Type::System, "Auto-launch status (scheduled task): {status}");
    }
    enabled
}
