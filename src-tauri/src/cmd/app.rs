use super::CmdResult;
use crate::cmd::StringifyErr as _;
use crate::core::autostart;
use smartstring::alias::String;

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
