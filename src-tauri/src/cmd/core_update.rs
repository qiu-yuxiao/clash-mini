use super::{CmdResult, StringifyErr as _};
use crate::core::core_updater::{CoreUpdater, GithubRelease};

#[tauri::command]
pub async fn check_core_update() -> CmdResult<GithubRelease> {
    CoreUpdater::check_latest_release().await.stringify_err()
}

#[tauri::command]
pub async fn start_core_upgrade(release: GithubRelease, app_handle: tauri::AppHandle) -> CmdResult {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = CoreUpdater::upgrade_core(app_handle.clone(), release).await {
            clash_verge_logging::logging!(error, clash_verge_logging::Type::System, "Core upgrade failed: {}", e);
        }
    });
    Ok(())
}
