mod resize;
#[cfg(test)]
mod tests;

pub(crate) use self::resize::setup_wm_sizing_hook;
pub use self::resize::{
    ResizeDirection, WindowResizeSnapshot, begin_native_window_resize, end_native_window_resize,
    is_native_window_resizing, step_native_window_resize,
};

use self::resize::setup_wm_sizing_hook_hwnd;
use crate::{config::Config, core::handle, utils::resolve::window_script::build_window_initial_script};
use clash_verge_logging::{Type, logging, logging_error};
use std::sync::atomic::{AtomicIsize, Ordering};
use tauri::webview::PageLoadEvent;
use tauri::{Manager as _, Theme, WebviewWindow};

use tauri::utils::config::Color;

const DARK_BACKGROUND_COLOR: Color = Color(46, 48, 61, 255);
const LIGHT_BACKGROUND_COLOR: Color = Color(245, 245, 245, 255);
const DARK_BACKGROUND_HEX: &str = "#2E303D";
const LIGHT_BACKGROUND_HEX: &str = "#F5F5F5";

// 极简窗口几何尺寸边界（客户区逻辑像素，须与前端 src/constants.ts 一致）
pub const DEFAULT_WIDTH: f64 = 271.0;
pub const DEFAULT_HEIGHT: f64 = 680.0;
pub const MAX_WIDTH: f64 = 640.0;
pub const MAX_HEIGHT: f64 = 860.0;
pub const MINIMAL_WIDTH: f64 = 271.0;
pub const MINIMAL_HEIGHT: f64 = 126.0;
const DEFAULT_DECORATIONS: bool = false;

/// 主窗口原子 Win32 句柄缓存（BUG-394）：
/// 仅在 UI 主线程完成 `Message::CreateWindow` 后写入，并在 `WM_NCDESTROY` / 主动销毁时清零，
/// 使后台线程可在零跨线程通道阻塞（零 `window_getter!`）前提下直接读取窗口显隐/焦点/最小化态。
static MAIN_WINDOW_HWND: AtomicIsize = AtomicIsize::new(0);

pub fn set_main_window_hwnd(hwnd: windows::Win32::Foundation::HWND) {
    MAIN_WINDOW_HWND.store(hwnd.0 as isize, Ordering::Release);
}

pub fn clear_main_window_hwnd(expected: Option<windows::Win32::Foundation::HWND>) {
    if let Some(hwnd) = expected {
        let raw = hwnd.0 as isize;
        let _ = MAIN_WINDOW_HWND.compare_exchange(raw, 0, Ordering::AcqRel, Ordering::Acquire);
    } else {
        MAIN_WINDOW_HWND.store(0, Ordering::Release);
    }
}

pub fn get_main_window_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;

    let raw = MAIN_WINDOW_HWND.load(Ordering::Acquire);
    if raw == 0 {
        return None;
    }
    let hwnd = HWND(raw as *mut std::ffi::c_void);
    // SAFETY: Win32 IsWindow 只读校验句柄有效性，绝不跨线程阻塞
    if unsafe { IsWindow(Some(hwnd)).as_bool() } {
        Some(hwnd)
    } else {
        let _ = MAIN_WINDOW_HWND.compare_exchange(raw, 0, Ordering::AcqRel, Ordering::Acquire);
        None
    }
}

pub fn is_main_window_ready() -> bool {
    get_main_window_hwnd().is_some()
}

fn cleanup_timed_out_main_window() {
    clear_main_window_hwnd(None);
    let ah = handle::Handle::app_handle().clone();
    let ah_destroy = ah.clone();
    let _ = ah.run_on_main_thread(move || {
        clear_main_window_hwnd(None);
        if let Some(w) = ah_destroy.get_webview_window(crate::MAIN_WINDOW_LABEL) {
            let _ = w.destroy();
        }
    });
}

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

    let saved_state = crate::utils::window_manager::restore_window_state();
    let (win_w, win_h, win_x, win_y) = match saved_state {
        Some(s) => (s.width, s.height, s.x, s.y),
        None => (DEFAULT_WIDTH, DEFAULT_HEIGHT, None, None),
    };

    let mut builder = tauri::WebviewWindowBuilder::new(app_handle, "main", tauri::WebviewUrl::App(start_page.into()))
        .decorations(DEFAULT_DECORATIONS)
        // 必须关闭无框阴影（shadow(false)）以保持 outer == inner，使内外尺寸口径与最小尺寸声明一致
        .shadow(false)
        .fullscreen(false)
        .inner_size(win_w, win_h)
        .max_inner_size(MAX_WIDTH, MAX_HEIGHT)
        .min_inner_size(MINIMAL_WIDTH, MINIMAL_HEIGHT)
        .visible(false)
        .initialization_script(&initial_script)
        .general_autofill_enabled(false);

    if let (Some(x), Some(y)) = (win_x, win_y) {
        builder = builder.position(x, y);
    } else {
        builder = builder.center();
    }

    builder = builder.on_page_load(move |window, payload| {
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

    // 窗口创建阶段即禁用 OS 级 maximize（Win+上 / 双击标题栏 / 任务栏右键），
    // 改由前端 toggleMaximize 自定义 640×860 大尺寸模式。
    // 提前到创建阶段设置，消除挂载后 setMaximizable 的竞态窗口期。
    builder = builder.maximizable(false);

    builder = builder.transparent(false).additional_browser_args(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --enable-features=CalculateNativeWinOcclusion --disk-cache-size=31457280",
    );

    if let Some(theme) = resolved_theme {
        builder = builder.theme(Some(theme));
    }

    builder = builder.background_color(background_color);

    let app_handle_for_init = app_handle.clone();
    match builder.build() {
        Ok(window) => {
            // 将原生样式剥离、尺寸强制对齐、Win32 子类化挂载与句柄登记全部收拢至主线程闭包内就地执行（BUG-394）：
            // 严禁在 Tokio 工作线程上调用会阻塞等待主线程 `mpsc::rx.recv()` 的 `window.window_handle()`；
            // 若极端满载下 WebView2 / 主线程创建超过 12s 仍未完成，则超时释放并回滚清理。
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
            let ah_for_closure = app_handle_for_init.clone();
            if let Err(e) = app_handle_for_init.run_on_main_thread(move || {
                let Some(w) = ah_for_closure.get_webview_window(crate::MAIN_WINDOW_LABEL) else {
                    let _ = ready_tx.send(Err("主线程未找到已创建的 main 窗口".to_string()));
                    return;
                };
                let Some(hwnd) = window_hwnd(&w) else {
                    let _ = ready_tx.send(Err("主线程获取窗口 Win32 句柄失败".to_string()));
                    return;
                };
                set_main_window_hwnd(hwnd);
                strip_caption_style_hwnd(hwnd);
                force_set_window_outer_size_hwnd(hwnd, win_w, win_h);
                setup_wm_sizing_hook_hwnd(hwnd);
                let _ = ready_tx.send(Ok(()));
            }) {
                cleanup_timed_out_main_window();
                return Err(format!("调度主窗口初始化至主线程失败: {e}"));
            }

            match tokio::time::timeout(std::time::Duration::from_secs(12), ready_rx).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(reason))) => {
                    cleanup_timed_out_main_window();
                    return Err(reason);
                }
                Ok(Err(_)) => {
                    cleanup_timed_out_main_window();
                    return Err("主窗口初始化结果通道异常断开".to_string());
                }
                Err(_) => {
                    logging!(warn, Type::Window, "等待主线程完成主窗口创建超时 (12s)，回滚释放并清理");
                    cleanup_timed_out_main_window();
                    return Err("等待主线程创建主窗口超时 (12s)".to_string());
                }
            }

            // 超时兜底：如果页面加载超时（默认 10 秒），强制显示窗口
            // 避免页面加载卡住导致用户看不到窗口（通过原子 Win32 句柄检查可见性，不在 Tokio 线程阻塞查询）
            let window_label = window.label().to_string();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                // L2-04: 退出阶段不操作窗口；确认句柄仍然有效
                if crate::core::handle::Handle::global().is_exiting() {
                    return;
                }
                let Some(hwnd) = get_main_window_hwnd() else {
                    return;
                };
                // SAFETY: get_main_window_hwnd 已确认 hwnd 有效
                let is_visible = unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool() };
                if !is_visible {
                    let ah = crate::core::handle::Handle::app_handle().clone();
                    let ah_show = ah.clone();
                    let _ = ah.run_on_main_thread(move || {
                        if let Some(w) = ah_show.get_webview_window(&window_label) {
                            let _ = w.show();
                            let _ = w.set_focus();
                            logging!(info, Type::Window, "页面加载超时，强制显示窗口");
                        }
                    });
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

pub(super) fn window_hwnd(window: &WebviewWindow) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::HasWindowHandle as _;

    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(h) => {
            Some(windows::Win32::Foundation::HWND(h.hwnd.get() as *mut std::ffi::c_void))
        }
        _ => None,
    }
}

/// 清除 `WS_CAPTION`（保留 `WS_THICKFRAME` 以支持无框拖拽缩放），确保 Win32 无边框效果立即生效。
fn strip_caption_style_hwnd(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetWindowLongPtrW, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WS_CAPTION,
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

/// 直接通过 Win32 `SetWindowPos` 按逻辑尺寸和 DPI 设置窗口位置矩形（在 `shadow(false)` 下 outer == inner）。
fn force_set_window_outer_size_hwnd(hwnd: windows::Win32::Foundation::HWND, width: f64, height: f64) {
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOZORDER, SetWindowPos,
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
