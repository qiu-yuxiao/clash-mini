use tauri::webview::PageLoadEvent;
use tauri::{Manager as _, Theme, WebviewWindow};

use crate::{config::Config, core::handle, utils::resolve::window_script::build_window_initial_script};
use clash_verge_logging::{Type, logging, logging_error};

use tauri::utils::config::Color;

const DARK_BACKGROUND_COLOR: Color = Color(46, 48, 61, 255);
const LIGHT_BACKGROUND_COLOR: Color = Color(245, 245, 245, 255);
const DARK_BACKGROUND_HEX: &str = "#2E303D";
const LIGHT_BACKGROUND_HEX: &str = "#F5F5F5";

// 极简窗口默认宽度（内容/客户区口径）。
// 【为什么是 271】旧版本窗口带无框阴影，客户区比窗口外框窄 22 物理像素（150% DPI 下），
// 即外框 285 时内容实际只有 270.67 CSS px。关闭阴影后 outer == inner，若继续用 285
// 会让窄窗内容凭空变宽约 14px、既有窄窗排版变松。为保持既有视觉密度，取 271（285−14）。
pub const DEFAULT_WIDTH: f64 = 271.0;
pub const DEFAULT_HEIGHT: f64 = 680.0;
pub const MAX_WIDTH: f64 = 640.0;
pub const MAX_HEIGHT: f64 = 860.0;
// 极简窗口最小尺寸；必须与前端 src/constants.ts 的 MIN_WINDOW_WIDTH 保持一致
// （注意：不是排版断点 MINI_WIDTH_THRESHOLD=285，断点只属于前端布局层）
pub const MINIMAL_WIDTH: f64 = 271.0;
// 极简窗口最小高度；对应前端 MIN_WINDOW_HEIGHT（与极小窗口判定门槛 126 同值）。
// 旧值 135 是带系统阴影时代的余量，关闭阴影（shadow(false)）后按 −14 对称修正到 121
// （BUG-300）；3.0.5 再 +5 到 126，使 150% 档物理落点（126×1.5=189）恰为整数，
// 消除极窄判定的取整歧义（BUG-301 时期一并调整）
pub const MINIMAL_HEIGHT: f64 = 126.0;
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
        // 【几何口径铁律】必须关闭无框阴影（默认是开启的）。
        // 开启时 tao 会置 MARKER_UNDECORATED_SHADOW，其 WM_NCCALCSIZE 给客户区加不可见内缩
        // （实测 150% DPI：左右各 11、上 2 下 11 物理像素），于是「窗口外尺寸」≠「客户区内尺寸」。
        // 而前端缩放拖拽以位置为基准、以 setSize（Tauri 语义 = 内尺寸）落地，两套口径差一个内缩量，
        // 会导致拖拽每帧都被放大一圈（最小宽度下表现为「点一下左键窗口就跳宽 22×13 物理像素」）。
        // 关掉阴影后内缩归零，outer == inner，全工程（后端 min/max_inner_size、前端 MIN/MAX 常量、
        // window_state.json 持久化）统一按内尺寸口径，不再需要到处补偿内缩量。
        // 代价：失去系统投影与 Win11 圆角/1px 描边 —— 本程序外框自绘（皮肤背景），可接受。
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

    match builder.build() {
        Ok(window) => {
            // 窗口创建后立即清除 WS_CAPTION 并重置 outer 尺寸为 271×680。
            // 必须在 build() 之后立即执行，不能放在 RunEvent::Ready（handle_ready_resumed），
            // 因为 RunEvent::Ready 比窗口创建的异步任务早约 0.5 秒触发，此时 get_webview_window
            // 返回 None，修复代码被跳过，导致初次启动窗口保持 300px。
            // 注意：保留 WS_THICKFRAME（仅清 WS_CAPTION）——WS_THICKFRAME 是 startResizeDragging
            // 生效的前提；可见边框由 tao 的 WM_NCCALCSIZE 子类化消除（decorations=false 时 insets=0）。
            strip_caption_style(&window);
            force_set_window_outer_size(&window, win_w, win_h);
            setup_wm_sizing_hook(&window);

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
/// —— 前提是 `decorations=false` **且** `shadow=false`（见 build_new_window 的几何口径铁律），
/// 此时 tao 不加任何 insets，客户区等于窗口矩形；若重新开启 shadow，客户区会被内缩
/// 11 物理像素/边，本项目所有「内尺寸 == 外尺寸」的假设将全部失效。
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
/// 有边框算出 outer = inner + 边框 = 271 + 15 = 286，导致 set_size 无效。
/// 直接用 SetWindowPos 设置 outer 尺寸，再由 tao 的 WM_NCCALCSIZE 处理
/// （`decorations=false` + `shadow=false` 时返回 0 insets）让 client = outer = 271。
///
/// 【口径说明】因为本项目已通过 `shadow(false)` 让 outer == inner，本函数虽然设的是 outer，
/// 效果与「设内尺寸」完全等价，也因此不会与 `min_inner_size` 冲突
/// （此前开启阴影时 outer=285 实际只有 inner≈271，使声明的最小宽度形同虚设）。
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

static OLD_WNDPROC: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
static HOOKED_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// 安装 WM_SIZING 消息处理器，防止上边缘缩放到最小高度后窗口 Y 坐标继续下移。
///
/// 标准 WM_GETMINMAXINFO 只钳制尺寸不钳制位置，而上边缘缩放时 Windows 会持续增加 Y 坐标，
/// 导致窗口缩到最小时整体向下平移。WM_SIZING 在系统应用矩形之前给出提议矩形，在此处
/// 同时钳制尺寸和 Y 位置可根除该问题。
pub(crate) fn setup_wm_sizing_hook(window: &WebviewWindow) {
    use raw_window_handle::HasWindowHandle as _;
    use std::sync::atomic::Ordering;
    use windows::Win32::UI::WindowsAndMessaging::{GWLP_WNDPROC, SetWindowLongPtrW};

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

    // 按 HWND 重入保护：
    // 1. 同一窗口在 RunEvent::Resumed（系统睡眠恢复）重复调用时跳过，防止 OLD_WNDPROC
    //    被覆盖为 sizing_wndproc 自身地址导致无限递归；
    // 2. 轻量模式销毁旧窗口并重建新窗口时，新 HWND 与旧 HWND 不同，允许为新窗口重新安装钩子。
    let hwnd_val = hwnd.0 as isize;
    if hwnd_val == 0 || HOOKED_HWND.load(Ordering::Acquire) == hwnd_val {
        return;
    }

    unsafe {
        let new_proc = Some(sizing_wndproc as unsafe extern "system" fn(_, _, _, _) -> _);
        let old_proc_val = SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            std::mem::transmute::<
                Option<
                    unsafe extern "system" fn(
                        windows::Win32::Foundation::HWND,
                        u32,
                        windows::Win32::Foundation::WPARAM,
                        windows::Win32::Foundation::LPARAM,
                    ) -> windows::Win32::Foundation::LRESULT,
                >,
                isize,
            >(new_proc),
        );
        if old_proc_val != 0 {
            OLD_WNDPROC.store(old_proc_val as *mut _, Ordering::Release);
            HOOKED_HWND.store(hwnd_val, Ordering::Release);
        }
    }
}

/// WM_SIZING 子类化窗口过程。
/// 拦截上边缘缩放（WMSZ_TOP / WMSZ_TOPLEFT / WMSZ_TOPRIGHT），
/// 当提议高度小于最小尺寸时，修正 rect.top，使窗口位置和高度同时被钳制。
unsafe extern "system" fn sizing_wndproc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use std::sync::atomic::Ordering;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, WM_SIZING, WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
    };

    if msg == WM_SIZING {
        let side = wparam.0 as u32;
        if side == WMSZ_TOP || side == WMSZ_TOPLEFT || side == WMSZ_TOPRIGHT {
            // SAFETY: lparam 在 WM_SIZING 消息中指向有效的 RECT 结构
            unsafe {
                let rect = &mut *(lparam.0 as *mut windows::Win32::Foundation::RECT);
                let dpi = GetDpiForWindow(hwnd);
                let scale = (dpi as f64) / 96.0;
                let min_height_px = (MINIMAL_HEIGHT * scale).round() as i32;
                let height = rect.bottom - rect.top;
                if height < min_height_px {
                    rect.top = rect.bottom - min_height_px;
                    return windows::Win32::Foundation::LRESULT(1);
                }
            }
        }
    }

    let old_proc = OLD_WNDPROC.load(Ordering::Acquire);
    if !old_proc.is_null() {
        // SAFETY: old_proc 是之前 SetWindowLongPtrW 返回的有效窗口过程
        unsafe {
            CallWindowProcW(
                Some(std::mem::transmute::<
                    *mut std::ffi::c_void,
                    unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> windows::Win32::Foundation::LRESULT,
                >(old_proc)),
                hwnd,
                msg,
                wparam,
                lparam,
            )
        }
    } else {
        // SAFETY: DefWindowProcW 是默认窗口过程，对所有参数安全
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}
