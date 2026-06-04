//! FEAT-003: Hide/restore native window chrome via Tauri set_decorations.

use super::CmdResult;
use crate::cmd::StringifyErr as _;

#[tauri::command]
pub fn hide_window_chrome(window: tauri::WebviewWindow) -> CmdResult<()> {
    window.set_decorations(false).stringify_err()
}

#[tauri::command]
pub fn restore_window_chrome(window: tauri::WebviewWindow) -> CmdResult<()> {
    window.set_decorations(true).stringify_err()
}
