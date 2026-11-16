use super::CmdResult;
use crate::core::autostart;
use crate::{cmd::StringifyErr as _, feat, utils::dirs};
use smartstring::alias::String;

/// 打开应用程序所在目录
#[tauri::command]
pub fn open_app_dir() -> CmdResult<()> {
    let app_dir = dirs::app_home_dir().stringify_err()?;
    open::that(app_dir).stringify_err()
}

/// 打开核心所在目录
#[tauri::command]
pub fn open_core_dir() -> CmdResult<()> {
    let core_dir = tauri::utils::platform::current_exe().stringify_err()?;
    let core_dir = core_dir.parent().ok_or("failed to get core dir")?;
    open::that(core_dir).stringify_err()
}

/// 打开日志目录
#[tauri::command]
pub fn open_logs_dir() -> CmdResult<()> {
    let log_dir = dirs::app_logs_dir().stringify_err()?;
    open::that(log_dir).stringify_err()
}

/// 在系统文件管理器中打开并高亮选中指定文件或目录
#[tauri::command]
pub fn show_in_folder(path: std::path::PathBuf) -> CmdResult<()> {
    crate::utils::help::show_in_folder(&path).stringify_err()
}

/// 打开网页链接（仅允许 http/https 协议）
#[tauri::command]
pub fn open_web_url(url: String) -> CmdResult<()> {
    let parsed = url::Url::parse(&url).map_err(|_| "invalid URL")?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("only http/https URLs are allowed".into());
    }
    open::that(url.as_str()).stringify_err()
}

// TODO 后续可以为前端提供接口，当前作为托盘菜单使用
/// 打开 Verge 最新日志
#[tauri::command]
pub fn open_app_log() -> CmdResult<()> {
    let log_path = dirs::app_latest_log().stringify_err()?;
    let log_path = crate::utils::help::snapshot_path(&log_path).stringify_err()?;
    open::that(log_path).stringify_err()
}

// TODO 后续可以为前端提供接口，当前作为托盘菜单使用
/// 打开 Clash 最新日志
#[tauri::command]
pub fn open_core_log() -> CmdResult<()> {
    let log_path = dirs::clash_latest_log().stringify_err()?;
    let log_path = crate::utils::help::snapshot_path(&log_path).stringify_err()?;
    open::that(log_path).stringify_err()
}

/// 退出应用
#[tauri::command]
pub async fn exit_app() {
    feat::quit().await;
}

/// 获取便携版标识
#[tauri::command]
pub fn get_portable_flag() -> bool {
    *dirs::PORTABLE_FLAG.get().unwrap_or(&false)
}

/// 获取应用目录
#[tauri::command]
pub fn get_app_dir() -> CmdResult<String> {
    let app_home_dir = dirs::app_home_dir().stringify_err()?.to_string_lossy().into();
    Ok(app_home_dir)
}

/// 获取当前自启动状态
#[tauri::command]
pub fn get_auto_launch_status() -> CmdResult<bool> {
    autostart::get_launch_status().stringify_err()
}

/// 开始原生 Win32 指针窗口缩放会话（BUG-386）
#[tauri::command]
pub fn begin_window_resize(direction: crate::utils::resolve::window::ResizeDirection, seq: u64) -> CmdResult<()> {
    crate::utils::resolve::window::begin_native_window_resize(direction, seq).map_err(Into::into)
}

/// 根据当前全局硬件光标步进一次原生 Win32 原子缩放（BUG-386）
#[tauri::command]
pub fn step_window_resize(seq: u64) -> CmdResult<bool> {
    crate::utils::resolve::window::step_native_window_resize(seq).map_err(Into::into)
}

/// 结束原生 Win32 指针窗口缩放会话并返回权威客户区物理尺寸快照（BUG-386）
#[tauri::command]
pub async fn end_window_resize(seq: u64) -> CmdResult<Option<crate::utils::resolve::window::WindowResizeSnapshot>> {
    crate::utils::resolve::window::end_native_window_resize(seq)
        .await
        .map_err(Into::into)
}
