use crate::{config::Config, core::handle, utils::resolve::window_script::build_window_initial_script};
use clash_verge_logging::{Type, logging, logging_error};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use tauri::webview::PageLoadEvent;
use tauri::{Manager as _, Theme, WebviewWindow};

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

fn window_hwnd(window: &WebviewWindow) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::HasWindowHandle as _;

    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(h) => {
            Some(windows::Win32::Foundation::HWND(h.hwnd.get() as *mut std::ffi::c_void))
        }
        _ => None,
    }
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

const WM_SIZING_SUBCLASS_ID: usize = 0x434D_535A; // 'CMSZ'

#[link(name = "comctl32")]
unsafe extern "system" {
    fn SetWindowSubclass(
        hwnd: windows::Win32::Foundation::HWND,
        pfn_subclass: Option<
            unsafe extern "system" fn(
                windows::Win32::Foundation::HWND,
                u32,
                windows::Win32::Foundation::WPARAM,
                windows::Win32::Foundation::LPARAM,
                usize,
                usize,
            ) -> windows::Win32::Foundation::LRESULT,
        >,
        u_id_subclass: usize,
        dw_ref_data: usize,
    ) -> i32;

    fn DefSubclassProc(
        hwnd: windows::Win32::Foundation::HWND,
        u_msg: u32,
        w_param: windows::Win32::Foundation::WPARAM,
        l_param: windows::Win32::Foundation::LPARAM,
    ) -> windows::Win32::Foundation::LRESULT;

    fn RemoveWindowSubclass(
        hwnd: windows::Win32::Foundation::HWND,
        pfn_subclass: Option<
            unsafe extern "system" fn(
                windows::Win32::Foundation::HWND,
                u32,
                windows::Win32::Foundation::WPARAM,
                windows::Win32::Foundation::LPARAM,
                usize,
                usize,
            ) -> windows::Win32::Foundation::LRESULT,
        >,
        u_id_subclass: usize,
    ) -> i32;
}

pub(crate) fn setup_wm_sizing_hook_hwnd(hwnd: windows::Win32::Foundation::HWND) {
    if hwnd.0.is_null() {
        return;
    }

    // SAFETY: hwnd 来自当前有效 WebviewWindow，sizing_subclass_proc 遵循 SUBCLASSPROC ABI。
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(sizing_subclass_proc), WM_SIZING_SUBCLASS_ID, 0);
    }
}

/// 安装 WM_SIZING 消息处理器（现代 SetWindowSubclass 架构，BUG-332 / BUG-350），
/// 防止上边缘缩放到最小高度后窗口 Y 坐标继续下移。
///
/// 相比旧版 SetWindowLongPtrW + 全局静态 HOOKED_HWND：
/// 1. SetWindowSubclass 对同一 (hwnd, proc, id) 天然幂等，系统睡眠恢复重复调用安全无栈溢出；
/// 2. 在 WM_NCDESTROY 时自动解绑释放，彻底根除轻量模式销毁旧窗口并重建新窗口时因 Windows
///    句柄表复用相同 HWND 数值导致误跳过钩子挂载的缺陷。
/// 3. 仅在 UI 主线程（`handle_ready_resumed`）调用，同步登记 `MAIN_WINDOW_HWND`。
pub(crate) fn setup_wm_sizing_hook(window: &WebviewWindow) {
    let Some(hwnd) = window_hwnd(window) else {
        return;
    };
    set_main_window_hwnd(hwnd);
    setup_wm_sizing_hook_hwnd(hwnd);
}

/// WM_SIZING 子类化窗口过程（BUG-332 / BUG-350 / BUG-386）。
/// 1. 在 `WM_ENTERSIZEMOVE` / `WM_EXITSIZEMOVE` 同步 `NATIVE_WINDOW_RESIZING` 并在结束时落盘终值；
/// 2. 在 `WM_SIZING` 对全部 8 个拖拽方向统一执行 `[MINIMAL, MAX]` 物理像素钳制与对边锁定；
/// 3. 在 `WM_NCDESTROY` 自动卸载子类化钩子并重置拖拽标志。
unsafe extern "system" fn sizing_subclass_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _u_id_subclass: usize,
    _dw_ref_data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCDESTROY, WM_SIZING};

    if msg == WM_ENTERSIZEMOVE {
        NATIVE_WINDOW_RESIZING.store(true, Ordering::Release);
    } else if msg == WM_EXITSIZEMOVE {
        NATIVE_WINDOW_RESIZING.store(false, Ordering::Release);
        save_window_state_from_hwnd(hwnd);
    } else if msg == WM_SIZING {
        let side = wparam.0 as u32;
        // SAFETY: lparam 在 WM_SIZING 消息中指向有效的 RECT 结构
        unsafe {
            let rect = &mut *(lparam.0 as *mut windows::Win32::Foundation::RECT);
            let dpi = GetDpiForWindow(hwnd);
            let scale = scale_from_dpi(dpi);
            let mut outer = OuterRectPx {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
            if clamp_wm_sizing_rect(side, &mut outer, scale) {
                rect.left = outer.left;
                rect.top = outer.top;
                rect.right = outer.right;
                rect.bottom = outer.bottom;
                return windows::Win32::Foundation::LRESULT(1);
            }
        }
    } else if msg == WM_NCDESTROY {
        NATIVE_WINDOW_RESIZING.store(false, Ordering::Release);
        *RESIZE_SESSION.lock() = None;
        *PENDING_SWP_RECT.lock() = None;
        clear_main_window_hwnd(Some(hwnd));
        // SAFETY: 在 WM_NCDESTROY 中移除自身子类化节点是 Microsoft 官方规范做法
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(sizing_subclass_proc), WM_SIZING_SUBCLASS_ID);
        }
    }

    // SAFETY: 委托子类化链中的下一个过程处理其余消息
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

// =========================================================================
// 原生原子 Win32 指针缩放管线（BUG-386）
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeDirection {
    N,
    S,
    E,
    W,
    Ne,
    Nw,
    Se,
    Sw,
}

impl ResizeDirection {
    const fn has_north(self) -> bool {
        matches!(self, Self::N | Self::Ne | Self::Nw)
    }

    const fn has_south(self) -> bool {
        matches!(self, Self::S | Self::Se | Self::Sw)
    }

    const fn has_west(self) -> bool {
        matches!(self, Self::W | Self::Nw | Self::Sw)
    }

    const fn has_east(self) -> bool {
        matches!(self, Self::E | Self::Ne | Self::Se)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowResizeSnapshot {
    pub width: i32,
    pub height: i32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OuterRectPx {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl OuterRectPx {
    const fn width(self) -> i32 {
        self.right - self.left
    }

    const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Copy)]
struct NativeResizeSession {
    seq: u64,
    hwnd_raw: usize,
    direction: ResizeDirection,
    start_rect: OuterRectPx,
    frame_w: i32,
    frame_h: i32,
    start_cursor_x: i32,
    start_cursor_y: i32,
    scale: f64,
    last_applied: OuterRectPx,
}

static NATIVE_WINDOW_RESIZING: AtomicBool = AtomicBool::new(false);
static RESIZE_SESSION: Mutex<Option<NativeResizeSession>> = Mutex::new(None);
static PENDING_SWP_RECT: Mutex<Option<(usize, OuterRectPx)>> = Mutex::new(None);
static RESIZE_SWP_QUEUED: AtomicBool = AtomicBool::new(false);

/// 当前是否正处于原生或网页指针缩放拖拽期（供 `lib.rs` 跳过逐像素冗余查询）
pub fn is_native_window_resizing() -> bool {
    NATIVE_WINDOW_RESIZING.load(Ordering::Acquire)
}

const fn scale_from_dpi(dpi: u32) -> f64 {
    if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 }
}

fn clamp_wm_sizing_rect(side: u32, rect: &mut OuterRectPx, scale: f64) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        WMSZ_BOTTOM, WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
    };

    let min_w = (MINIMAL_WIDTH * scale).round() as i32;
    let max_w = (MAX_WIDTH * scale).round() as i32;
    let min_h = (MINIMAL_HEIGHT * scale).round() as i32;
    let max_h = (MAX_HEIGHT * scale).round() as i32;

    let mut changed = false;
    let cur_w = rect.width();
    if matches!(side, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) {
        let clamped_w = cur_w.clamp(min_w, max_w);
        if clamped_w != cur_w {
            rect.left = rect.right - clamped_w;
            changed = true;
        }
    } else if matches!(side, WMSZ_RIGHT | WMSZ_TOPRIGHT | WMSZ_BOTTOMRIGHT) {
        let clamped_w = cur_w.clamp(min_w, max_w);
        if clamped_w != cur_w {
            rect.right = rect.left + clamped_w;
            changed = true;
        }
    }

    let cur_h = rect.height();
    if matches!(side, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT) {
        let clamped_h = cur_h.clamp(min_h, max_h);
        if clamped_h != cur_h {
            rect.top = rect.bottom - clamped_h;
            changed = true;
        }
    } else if matches!(side, WMSZ_BOTTOM | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT) {
        let clamped_h = cur_h.clamp(min_h, max_h);
        if clamped_h != cur_h {
            rect.bottom = rect.top + clamped_h;
            changed = true;
        }
    }

    changed
}

const fn clamp_cursor_axis_delta(
    start_client: i32,
    min_client: i32,
    max_client: i32,
    positive_grow: bool,
    negative_grow: bool,
    cur_cursor: i32,
    start_cursor: &mut i32,
) -> i32 {
    let mut delta = cur_cursor - *start_cursor;
    if !positive_grow && !negative_grow {
        return delta;
    }

    let (min_d, max_d) = if positive_grow {
        (min_client - start_client, max_client - start_client)
    } else {
        (start_client - max_client, start_client - min_client)
    };

    if delta < min_d {
        *start_cursor = cur_cursor - min_d;
        delta = min_d;
    } else if delta > max_d {
        *start_cursor = cur_cursor - max_d;
        delta = max_d;
    }
    delta
}

/// 纯函数：由不可变起始外矩形、内边框偏移与全局物理光标位移计算目标外矩形。
/// 非拖拽轴直接原样透传（不做二次 clamp），西/北方向由不可变右边/底边反推以锁定锚定边。
pub(crate) fn compute_native_resize_outer_rect(
    direction: ResizeDirection,
    start_rect: OuterRectPx,
    frame_w: i32,
    frame_h: i32,
    dx: i32,
    dy: i32,
    scale: f64,
) -> OuterRectPx {
    let start_client_w = (start_rect.width() - frame_w).max(1);
    let start_client_h = (start_rect.height() - frame_h).max(1);

    let min_client_w = (MINIMAL_WIDTH * scale).round() as i32;
    let max_client_w = (MAX_WIDTH * scale).round() as i32;
    let min_client_h = (MINIMAL_HEIGHT * scale).round() as i32;
    let max_client_h = (MAX_HEIGHT * scale).round() as i32;

    let new_client_w = if direction.has_east() {
        (start_client_w + dx).clamp(min_client_w, max_client_w)
    } else if direction.has_west() {
        (start_client_w - dx).clamp(min_client_w, max_client_w)
    } else {
        start_client_w
    };

    let new_client_h = if direction.has_south() {
        (start_client_h + dy).clamp(min_client_h, max_client_h)
    } else if direction.has_north() {
        (start_client_h - dy).clamp(min_client_h, max_client_h)
    } else {
        start_client_h
    };

    let new_outer_w = new_client_w + frame_w;
    let new_outer_h = new_client_h + frame_h;

    let (left, right) = if direction.has_west() {
        (start_rect.right - new_outer_w, start_rect.right)
    } else {
        (start_rect.left, start_rect.left + new_outer_w)
    };

    let (top, bottom) = if direction.has_north() {
        (start_rect.bottom - new_outer_h, start_rect.bottom)
    } else {
        (start_rect.top, start_rect.top + new_outer_h)
    };

    OuterRectPx {
        left,
        top,
        right,
        bottom,
    }
}

fn read_hwnd_geometry_and_dpi(hwnd: windows::Win32::Foundation::HWND) -> Result<(OuterRectPx, i32, i32, f64), String> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};

    let mut win_rect = RECT::default();
    let mut client_rect = RECT::default();
    // SAFETY: hwnd 来自有效 Win32 窗口句柄，传入栈上有效 RECT 指针
    unsafe {
        GetWindowRect(hwnd, &mut win_rect).map_err(|e| format!("GetWindowRect failed: {e}"))?;
        GetClientRect(hwnd, &mut client_rect).map_err(|e| format!("GetClientRect failed: {e}"))?;
    }
    // SAFETY: GetDpiForWindow 仅读取句柄 DPI，无副作用
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let scale = scale_from_dpi(dpi);
    let outer = OuterRectPx {
        left: win_rect.left,
        top: win_rect.top,
        right: win_rect.right,
        bottom: win_rect.bottom,
    };
    let client_w = client_rect.right - client_rect.left;
    let client_h = client_rect.bottom - client_rect.top;
    let frame_w = (outer.width() - client_w).max(0);
    let frame_h = (outer.height() - client_h).max(0);
    Ok((outer, frame_w, frame_h, scale))
}

fn save_window_state_from_hwnd(hwnd: windows::Win32::Foundation::HWND) {
    if let Ok((outer, frame_w, frame_h, scale)) = read_hwnd_geometry_and_dpi(hwnd) {
        let client_w = (outer.width() - frame_w).max(0) as f64 / scale;
        let client_h = (outer.height() - frame_h).max(0) as f64 / scale;
        let x = outer.left as f64 / scale;
        let y = outer.top as f64 / scale;
        crate::utils::window_manager::save_window_state_on_geometry_change(client_w, client_h, Some(x), Some(y));
    }
}

fn main_hwnd_raw() -> Result<usize, String> {
    get_main_window_hwnd()
        .map(|h| h.0 as usize)
        .ok_or_else(|| String::from("main window not found"))
}

fn drain_pending_swp_queue() {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindow, SWP_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOOWNERZORDER, SWP_NOZORDER, SetWindowPos,
    };

    loop {
        let pending = { PENDING_SWP_RECT.lock().take() };
        let Some((hwnd_raw, rect)) = pending else {
            RESIZE_SWP_QUEUED.store(false, Ordering::Release);
            let has_more = { PENDING_SWP_RECT.lock().is_some() };
            if has_more && !RESIZE_SWP_QUEUED.swap(true, Ordering::AcqRel) {
                continue;
            }
            break;
        };

        let hwnd = HWND(hwnd_raw as *mut std::ffi::c_void);
        // SAFETY: 在 UI 主线程执行，且先用 IsWindow 校验句柄仍然有效
        unsafe {
            if IsWindow(Some(hwnd)).as_bool() {
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.width(),
                    rect.height(),
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOCOPYBITS,
                );
            }
        }
    }
}

fn step_active_session_locked(session: &mut NativeResizeSession) -> Result<Option<(usize, OuterRectPx)>, String> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut cursor = POINT::default();
    // SAFETY: 传入有效栈上 POINT 指针
    unsafe {
        GetCursorPos(&mut cursor).map_err(|e| format!("GetCursorPos failed: {e}"))?;
    }

    let start_client_w = (session.start_rect.width() - session.frame_w).max(1);
    let start_client_h = (session.start_rect.height() - session.frame_h).max(1);
    let min_client_w = (MINIMAL_WIDTH * session.scale).round() as i32;
    let max_client_w = (MAX_WIDTH * session.scale).round() as i32;
    let min_client_h = (MINIMAL_HEIGHT * session.scale).round() as i32;
    let max_client_h = (MAX_HEIGHT * session.scale).round() as i32;

    let dx = clamp_cursor_axis_delta(
        start_client_w,
        min_client_w,
        max_client_w,
        session.direction.has_east(),
        session.direction.has_west(),
        cursor.x,
        &mut session.start_cursor_x,
    );
    let dy = clamp_cursor_axis_delta(
        start_client_h,
        min_client_h,
        max_client_h,
        session.direction.has_south(),
        session.direction.has_north(),
        cursor.y,
        &mut session.start_cursor_y,
    );

    let target = compute_native_resize_outer_rect(
        session.direction,
        session.start_rect,
        session.frame_w,
        session.frame_h,
        dx,
        dy,
        session.scale,
    );
    if target == session.last_applied {
        return Ok(None);
    }
    session.last_applied = target;
    Ok(Some((session.hwnd_raw, target)))
}

/// 开始一次原生指针窗口缩放会话：单次原子采集外矩形、客户区、DPI 与全局硬件光标。
pub fn begin_native_window_resize(direction: ResizeDirection, seq: u64) -> Result<(), String> {
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let hwnd_raw = main_hwnd_raw()?;
    let hwnd = HWND(hwnd_raw as *mut std::ffi::c_void);
    let (start_rect, frame_w, frame_h, scale) = read_hwnd_geometry_and_dpi(hwnd)?;

    let mut cursor = POINT::default();
    // SAFETY: 传入有效栈上 POINT 指针
    unsafe {
        GetCursorPos(&mut cursor).map_err(|e| format!("GetCursorPos failed: {e}"))?;
    }

    *PENDING_SWP_RECT.lock() = None;
    let session = NativeResizeSession {
        seq,
        hwnd_raw,
        direction,
        start_rect,
        frame_w,
        frame_h,
        start_cursor_x: cursor.x,
        start_cursor_y: cursor.y,
        scale,
        last_applied: start_rect,
    };
    *RESIZE_SESSION.lock() = Some(session);
    NATIVE_WINDOW_RESIZING.store(true, Ordering::Release);
    Ok(())
}

/// 读取当前全局硬件光标位置并将最新缩放矩形合并投递至 UI 主线程执行原子 `SetWindowPos`。
pub fn step_native_window_resize(seq: u64) -> Result<bool, String> {
    let maybe_update = {
        let mut guard = RESIZE_SESSION.lock();
        match guard.as_mut() {
            Some(session) if session.seq == seq => step_active_session_locked(session)?,
            _ => return Ok(false),
        }
    };

    let Some(update) = maybe_update else {
        return Ok(false);
    };

    *PENDING_SWP_RECT.lock() = Some(update);
    if !RESIZE_SWP_QUEUED.swap(true, Ordering::AcqRel) {
        let _ = handle::Handle::app_handle().run_on_main_thread(drain_pending_swp_queue);
    }
    Ok(true)
}

/// 结束原生指针窗口缩放会话，刷新最后一帧 `SetWindowPos`，复位标志并持久化终值。
pub async fn end_native_window_resize(seq: u64) -> Result<Option<WindowResizeSnapshot>, String> {
    let ended = {
        let mut guard = RESIZE_SESSION.lock();
        let Some(session_ref) = guard.as_mut() else {
            return Ok(None);
        };
        if session_ref.seq != seq {
            return Ok(None);
        }
        let maybe_update = step_active_session_locked(session_ref).ok().flatten();
        let session = *session_ref;
        *guard = None;
        drop(guard);
        (session, maybe_update)
    };

    let (session, maybe_update) = ended;
    if let Some(update) = maybe_update {
        *PENDING_SWP_RECT.lock() = Some(update);
    }

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    if handle::Handle::app_handle()
        .run_on_main_thread(move || {
            drain_pending_swp_queue();
            let _ = tx.send(());
        })
        .is_ok()
    {
        let _ = tokio::time::timeout(std::time::Duration::from_millis(300), rx).await;
    }

    let still_idle = { RESIZE_SESSION.lock().is_none() };
    if still_idle {
        NATIVE_WINDOW_RESIZING.store(false, Ordering::Release);
    }

    let last = session.last_applied;
    let client_w = (last.width() - session.frame_w).max(1);
    let client_h = (last.height() - session.frame_h).max(1);

    let logical_w = client_w as f64 / session.scale;
    let logical_h = client_h as f64 / session.scale;
    let logical_x = last.left as f64 / session.scale;
    let logical_y = last.top as f64 / session.scale;
    crate::utils::window_manager::save_window_state_on_geometry_change(
        logical_w,
        logical_h,
        Some(logical_x),
        Some(logical_y),
    );

    Ok(Some(WindowResizeSnapshot {
        width: client_w,
        height: client_h,
        scale_factor: session.scale,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod resize_tests {
    use super::*;

    #[test]
    fn east_and_south_resize_keep_origin_anchored() {
        let start = OuterRectPx {
            left: 500,
            top: 300,
            right: 800,
            bottom: 500,
        };
        let east = compute_native_resize_outer_rect(ResizeDirection::E, start, 0, 0, 50, 0, 1.0);
        assert_eq!(east.left, 500);
        assert_eq!(east.width(), 350);
        assert_eq!(east.height(), 200);

        let south = compute_native_resize_outer_rect(ResizeDirection::S, start, 0, 0, 0, 80, 1.0);
        assert_eq!(south.top, 300);
        assert_eq!(south.width(), 300);
        assert_eq!(south.height(), 280);
    }

    #[test]
    fn west_north_and_nw_resize_lock_right_and_bottom_edges_at_150_dpi() {
        let scale = 1.5;
        let start = OuterRectPx {
            left: 103,
            top: 207,
            right: 103 + 450,
            bottom: 207 + 300,
        };
        let nw = compute_native_resize_outer_rect(ResizeDirection::Nw, start, 0, 0, -37, -23, scale);
        assert_eq!(nw.right, start.right);
        assert_eq!(nw.bottom, start.bottom);
        assert_eq!(nw.width(), 487);
        assert_eq!(nw.height(), 323);

        // 超过最小尺寸向内挤压，右边/底边仍零漂移
        let clamped_w = compute_native_resize_outer_rect(ResizeDirection::W, start, 0, 0, 200, 0, scale);
        assert_eq!(clamped_w.right, start.right);
        assert_eq!(clamped_w.width(), (MINIMAL_WIDTH * scale).round() as i32);
    }

    #[test]
    fn non_dragged_axis_passes_through_without_reclamping() {
        let scale = 1.5;
        // 150% 下允许存在 406 物理像素窄边；仅拖上下边时宽度不得被重算顶大为 407
        let start = OuterRectPx {
            left: 100,
            top: 200,
            right: 100 + 406,
            bottom: 200 + 300,
        };
        let north = compute_native_resize_outer_rect(ResizeDirection::N, start, 0, 0, 0, -40, scale);
        assert_eq!(north.width(), 406);
        assert_eq!(north.height(), 340);
        assert_eq!(north.bottom, start.bottom);
    }

    #[test]
    fn anti_recoil_cursor_anchor_sliding_eliminates_boundary_deadzone() {
        let mut start_cursor = 500;
        // 向右拖拽超出最小宽度 271 很多（cursor 滑到 600）
        let clamped_dx = clamp_cursor_axis_delta(300, 271, 640, false, true, 600, &mut start_cursor);
        assert_eq!(clamped_dx, 29);
        assert_eq!(start_cursor, 571);
        // 反向向左刚挪 1px，立即可拉宽至 272px，零死区
        let reverse_dx = clamp_cursor_axis_delta(300, 271, 640, false, true, 599, &mut start_cursor);
        assert_eq!(reverse_dx, 28);
        assert_eq!(300 - reverse_dx, 272);
    }

    #[test]
    fn wm_sizing_clamps_both_width_and_height_anchored_to_opposite_edge() {
        use windows::Win32::UI::WindowsAndMessaging::{WMSZ_BOTTOMRIGHT, WMSZ_TOPLEFT};

        let mut tl = OuterRectPx {
            left: 400,
            top: 400,
            right: 500,
            bottom: 480,
        };
        assert!(clamp_wm_sizing_rect(WMSZ_TOPLEFT, &mut tl, 1.0));
        assert_eq!(tl.right, 500);
        assert_eq!(tl.bottom, 480);
        assert_eq!(tl.width(), MINIMAL_WIDTH as i32);
        assert_eq!(tl.height(), MINIMAL_HEIGHT as i32);

        let mut br = OuterRectPx {
            left: 100,
            top: 100,
            right: 900,
            bottom: 1200,
        };
        assert!(clamp_wm_sizing_rect(WMSZ_BOTTOMRIGHT, &mut br, 1.0));
        assert_eq!(br.left, 100);
        assert_eq!(br.top, 100);
        assert_eq!(br.width(), MAX_WIDTH as i32);
        assert_eq!(br.height(), MAX_HEIGHT as i32);
    }
}
