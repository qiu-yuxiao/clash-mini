use tauri::webview::PageLoadEvent;
use tauri::{Manager as _, Theme, WebviewWindow};

use crate::{config::Config, core::handle, utils::resolve::window_script::build_window_initial_script};
use clash_verge_logging::{Type, logging, logging_error};

#[cfg(not(target_os = "windows"))]
use dark_light::{Mode as SystemTheme, detect as detect_system_theme};
use tauri::utils::config::Color;

const DARK_BACKGROUND_COLOR: Color = Color(46, 48, 61, 255);
const LIGHT_BACKGROUND_COLOR: Color = Color(245, 245, 245, 255);
const DARK_BACKGROUND_HEX: &str = "#2E303D";
const LIGHT_BACKGROUND_HEX: &str = "#F5F5F5";

pub const DEFAULT_WIDTH: f64 = 285.0;
pub const DEFAULT_HEIGHT: f64 = 680.0;
const MAX_WIDTH: f64 = 640.0;
const MAX_HEIGHT: f64 = 860.0;
// 极简窗口最小尺寸；必须与前端 src/constants.ts 的 MINI_WIDTH_THRESHOLD / MINI_HEIGHT_THRESHOLD 保持一致
pub const MINIMAL_WIDTH: f64 = 285.0;
// 极简窗口最小高度；对应前端 MINI_HEIGHT_THRESHOLD
pub const MINIMAL_HEIGHT: f64 = 135.0;
const DEFAULT_DECORATIONS: bool = false;

pub async fn build_new_window() -> Result<WebviewWindow, String> {
    let app_handle = handle::Handle::app_handle();
    let app_handle_clone = app_handle.clone();
    let config = Config::verge().await;
    let latest = config.latest_arc();
    let always_on_top = latest.enable_always_on_top.unwrap_or(false);
    let start_page = latest.start_page.as_deref().unwrap_or("/");
    let initial_theme_mode = match latest.theme_mode.as_deref() {
        Some("dark") => "dark",
        Some("light") => "light",
        _ => "system",
    };

    let resolved_theme = match initial_theme_mode {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    };

    #[cfg(not(target_os = "windows"))]
    let prefers_dark_background = match resolved_theme {
        Some(Theme::Dark) => true,
        Some(Theme::Light) => false,
        _ => !matches!(detect_system_theme().ok(), Some(SystemTheme::Light)),
    };

    #[cfg(target_os = "windows")]
    let prefers_dark_background = match resolved_theme {
        Some(Theme::Dark) => true,
        Some(Theme::Light) => false,
        _ => false,
    };

    let background_color = if prefers_dark_background {
        DARK_BACKGROUND_COLOR
    } else {
        LIGHT_BACKGROUND_COLOR
    };

    let initial_script = build_window_initial_script(initial_theme_mode, DARK_BACKGROUND_HEX, LIGHT_BACKGROUND_HEX);

    let mut builder = tauri::WebviewWindowBuilder::new(app_handle, "main", tauri::WebviewUrl::App(start_page.into()))
        .center()
        .decorations(DEFAULT_DECORATIONS)
        .fullscreen(false)
        .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
        .max_inner_size(MAX_WIDTH, MAX_HEIGHT)
        .min_inner_size(MINIMAL_WIDTH, MINIMAL_HEIGHT)
        .visible(false)
        .initialization_script(&initial_script)
        .general_autofill_enabled(false)
        .on_page_load(move |window, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }

            logging_error!(Type::Window, window.set_title(&get_bold_window_title()));
            logging_error!(Type::Window, window.show());
            logging_error!(Type::Window, window.set_focus());
            let label = window.label().to_string();
            let ah = app_handle_clone.clone();
            let ah2 = ah.clone();
            // 【注意】on_page_load 回调本身就在主线程执行，此处 run_on_main_thread 是冗余的
            // 保留是为了安全起见，确保 set_always_on_top 在主线程调用，即使未来回调上下文变化
            let _ = ah.run_on_main_thread(move || {
                use tauri::Manager as _;
                if let Some(w) = ah2.get_webview_window(&label) {
                    let _ = w.set_always_on_top(always_on_top);
                }
            });
        });

    #[cfg(target_os = "windows")]
    {
        builder = builder.transparent(false).additional_browser_args(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --enable-features=CalculateNativeWinOcclusion --disk-cache-size=31457280",
        );
    }

    if let Some(theme) = resolved_theme {
        builder = builder.theme(Some(theme));
    }

    builder = builder.background_color(background_color);

    match builder.build() {
        Ok(window) => {
            #[cfg(not(target_os = "windows"))]
            {
                logging_error!(Type::Window, window.set_background_color(Some(background_color)));
            }

            // 窗口创建后立即清除 WS_CAPTION 并重置 outer 尺寸为 285×680。
            // 必须在 build() 之后立即执行，不能放在 RunEvent::Ready（handle_ready_resumed），
            // 因为 RunEvent::Ready 比窗口创建的异步任务早约 0.5 秒触发，此时 get_webview_window
            // 返回 None，修复代码被跳过，导致初次启动窗口保持 300px。
            // 注意：保留 WS_THICKFRAME（仅清 WS_CAPTION）——WS_THICKFRAME 是 startResizeDragging
            // 生效的前提；可见边框由 tao 的 WM_NCCALCSIZE 子类化消除（decorations=false 时 insets=0）。
            #[cfg(target_os = "windows")]
            {
                strip_caption_style(&window);
                force_set_window_outer_size(&window, DEFAULT_WIDTH, DEFAULT_HEIGHT);
            }

            // 超时兜底：如果页面加载超时（默认 10 秒），强制显示窗口
            // 避免页面加载卡住导致用户看不到窗口
            let window_clone = window.clone();
            let window_label = window.label().to_string();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                // L2-04: 退出阶段不操作窗口；通过 label 重新获取确认窗口仍存在
                if crate::core::handle::Handle::global().is_exiting() {
                    return;
                }
                let app_handle = crate::core::handle::Handle::app_handle();
                if app_handle.get_webview_window(&window_label).is_none() {
                    return;
                }
                // 检查窗口是否仍然存在且不可见
                if let Ok(is_visible) = window_clone.is_visible() {
                    if !is_visible {
                        // 页面加载超时，强制显示窗口
                        let _ = window_clone.show();
                        let _ = window_clone.set_focus();
                        logging!(info, Type::Window, "页面加载超时，强制显示窗口");
                    }
                }
            });

            Ok(window)
        }
        Err(e) => Err(e.to_string()),
    }
}

fn get_bold_window_title() -> String {
    format!("Clash Mini Ver.{}", env!("CARGO_PKG_VERSION"))
}

/// 剥掉窗口的 WS_CAPTION 样式位（保留 WS_THICKFRAME）。
///
/// 背景：Tauri 在 Windows 上创建 `decorations(false)` 窗口时，底层 tao 只设置
/// `MARKER_DECORATIONS` 标记，并未从 GWL_STYLE 中清除 `WS_CAPTION`。窗口初次创建时，
/// tao 的 WM_NCCALCSIZE 子类化尚未安装，Windows 会按 style 自动补上标题栏，
/// 导致 outer 尺寸比 `inner_size` 设定的 logical 值多出约 15-22px。
///
/// 此处主动清掉 WS_CAPTION 并用 SWP_FRAMECHANGE 触发一次重算，让窗口立即变成真正无边框。
///
/// 关键：保留 WS_THICKFRAME。WS_THICKFRAME 是 `startResizeDragging` 生效的前提，
/// 移除它会导致窗口彻底无法调整尺寸。可见边框由 tao 的 WM_NCCALCSIZE 子类化消除
/// （decorations=false 时返回 0 insets），无需移除 WS_THICKFRAME 来消除可见边框。
#[cfg(target_os = "windows")]
fn strip_caption_style(window: &WebviewWindow) {
    use raw_window_handle::HasWindowHandle as _;
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetWindowLongPtrW, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WS_CAPTION,
    };

    let handle = match window.window_handle() {
        Ok(h) => h,
        Err(_) => return,
    };
    let hwnd = match handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(h) => {
            windows::Win32::Foundation::HWND(h.hwnd.get() as *mut std::ffi::c_void)
        }
        _ => return,
    };

    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let mut new_style = current;
        new_style &= !(WS_CAPTION.0 as isize);
        if new_style == current {
            return;
        }
        SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
        logging!(
            info,
            Type::Window,
            "已清除 WS_CAPTION 样式位 (0x{:X} -> 0x{:X})，保留 WS_THICKFRAME",
            current,
            new_style
        );
    }
}

/// 直接用 Win32 API 设置窗口 outer 尺寸，绕过 tao 的 set_inner_size 计算。
///
/// tao 的 set_inner_size 在计算 outer size 时会用到窗口创建时缓存的 style
/// （仍含 WS_CAPTION | WS_THICKFRAME），即使已清除 GWL_STYLE，tao 仍会按
/// 有边框算出 outer = inner + 边框 = 285 + 15 = 300，导致 set_size 无效。
/// 直接用 SetWindowPos 设置 outer 尺寸，再由 tao 的 WM_NCCALCSIZE 处理
/// （decorations=false 时返回 0 insets）让 client = outer = 285。
#[cfg(target_os = "windows")]
fn force_set_window_outer_size(window: &WebviewWindow, width: f64, height: f64) {
    use raw_window_handle::HasWindowHandle as _;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOZORDER, SetWindowPos,
    };

    let handle = match window.window_handle() {
        Ok(h) => h,
        Err(_) => return,
    };
    let hwnd = match handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(h) => {
            windows::Win32::Foundation::HWND(h.hwnd.get() as *mut std::ffi::c_void)
        }
        _ => return,
    };

    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        let scale = dpi as f64 / 96.0;
        let physical_w = (width * scale) as i32;
        let physical_h = (height * scale) as i32;
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            physical_w,
            physical_h,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOZORDER | SWP_NOMOVE,
        );
        logging!(
            info,
            Type::Window,
            "强制设置窗口 outer 尺寸: {}x{} (DPI={}, scale={:.2})",
            physical_w,
            physical_h,
            dpi,
            scale
        );
    }
}
