use std::path::{Path, PathBuf};

use super::CmdResult;
use crate::cmd::StringifyErr as _;
use crate::core::autostart;
use smartstring::alias::String;

const ANTIVIRUS_GUIDE_FILENAME: &str = "轻量模式防误杀指南.txt";

fn collect_antivirus_guide_candidates(
    exe_dir: Option<&Path>,
    resources_dir: Option<&Path>,
    dev_workspace_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = exe_dir {
        candidates.push(dir.join(ANTIVIRUS_GUIDE_FILENAME));
        candidates.push(dir.join("resources").join(ANTIVIRUS_GUIDE_FILENAME));
    }
    if let Some(dir) = resources_dir {
        let candidate = dir.join(ANTIVIRUS_GUIDE_FILENAME);
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    if let Some(dir) = dev_workspace_dir {
        let candidate = dir.join(ANTIVIRUS_GUIDE_FILENAME);
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    candidates
}

fn select_existing_guide_path<I>(candidates: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    candidates.into_iter().find(|path| path.is_file())
}

fn resolve_antivirus_guide_path() -> Option<PathBuf> {
    let exe_path = tauri::utils::platform::current_exe().ok();
    let exe_dir = exe_path.as_deref().and_then(Path::parent);
    let resources_dir = crate::utils::dirs::app_resources_dir().ok();
    #[cfg(debug_assertions)]
    let dev_workspace_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent();
    #[cfg(not(debug_assertions))]
    let dev_workspace_dir: Option<&Path> = None;

    let candidates = collect_antivirus_guide_candidates(exe_dir, resources_dir.as_deref(), dev_workspace_dir);
    select_existing_guide_path(candidates)
}

/// 在系统文件管理器中打开并高亮选中指定文件或目录
#[tauri::command]
pub fn show_in_folder(path: std::path::PathBuf) -> CmdResult<()> {
    crate::utils::help::show_in_folder(&path).stringify_err()
}

/// 用系统默认文本查看器直接打开随包附带的《轻量模式防误杀指南.txt》（BUG-398）
#[tauri::command]
pub fn open_antivirus_guide() -> CmdResult<()> {
    let guide_path = resolve_antivirus_guide_path()
        .ok_or_else(|| format!("未找到《{ANTIVIRUS_GUIDE_FILENAME}》，请检查安装目录是否完整"))?;
    open::that(&guide_path).stringify_err()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn antivirus_guide_resolution_prefers_exe_root_then_resources() {
        let base = std::env::temp_dir().join(format!(
            "clash_mini_guide_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let exe_dir = base.join("install");
        let resources_dir = exe_dir.join("resources");
        let dev_dir = base.join("workspace");
        assert!(fs::create_dir_all(&resources_dir).is_ok());
        assert!(fs::create_dir_all(&dev_dir).is_ok());

        let root_file = exe_dir.join(ANTIVIRUS_GUIDE_FILENAME);
        let res_file = resources_dir.join(ANTIVIRUS_GUIDE_FILENAME);
        let dev_file = dev_dir.join(ANTIVIRUS_GUIDE_FILENAME);

        let empty = collect_antivirus_guide_candidates(Some(&exe_dir), Some(&resources_dir), Some(&dev_dir));
        assert_eq!(select_existing_guide_path(empty), None);

        assert!(fs::write(&dev_file, b"dev").is_ok());
        let with_dev = collect_antivirus_guide_candidates(Some(&exe_dir), Some(&resources_dir), Some(&dev_dir));
        assert_eq!(select_existing_guide_path(with_dev), Some(dev_file));

        assert!(fs::write(&res_file, b"res").is_ok());
        let with_res = collect_antivirus_guide_candidates(Some(&exe_dir), Some(&resources_dir), Some(&dev_dir));
        assert_eq!(with_res.len(), 3);
        assert_eq!(select_existing_guide_path(with_res), Some(res_file));

        assert!(fs::write(&root_file, b"root").is_ok());
        let with_root = collect_antivirus_guide_candidates(Some(&exe_dir), Some(&resources_dir), Some(&dev_dir));
        assert_eq!(select_existing_guide_path(with_root), Some(root_file));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn bundled_antivirus_guide_contains_all_verified_whitelist_files() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent();
        let guide = workspace_root.map(|d| d.join(ANTIVIRUS_GUIDE_FILENAME));
        assert!(matches!(&guide, Some(p) if p.is_file()));
        if let Some(path) = guide {
            let content = fs::read_to_string(path).unwrap_or_default();
            assert!(content.contains("whiteinternal.ini"));
            assert!(content.contains("useroptm2.ini"));
            assert!(content.contains("useroptm_game2.ini"));
            assert!(content.contains("[clash-mini.exe]"));
            assert!(content.contains("[clash mini_0]"));
            assert!(content.contains("[mini-mihomo.exe]"));
        }
    }
}
