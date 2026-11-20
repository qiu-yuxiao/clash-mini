use super::{
    MAX_HEIGHT, MAX_WIDTH, MINIMAL_HEIGHT, MINIMAL_WIDTH, clear_main_window_hwnd, get_main_window_hwnd,
    set_main_window_hwnd, window_hwnd,
};
use crate::core::handle;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::WebviewWindow;

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
    pub(super) const fn width(self) -> i32 {
        self.right - self.left
    }

    pub(super) const fn height(self) -> i32 {
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

pub(super) fn clamp_wm_sizing_rect(side: u32, rect: &mut OuterRectPx, scale: f64) -> bool {
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

pub(super) const fn clamp_cursor_axis_delta(
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
