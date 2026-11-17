use crate::config::IVerge;
use crate::module::lightweight;
use crate::process::AsyncHandler;
use crate::singleton;
use crate::utils::window_manager::WindowManager;
use crate::{Type, feat, logging};
use clash_verge_limiter::{Limiter, SystemClock, SystemLimiter};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::handle;
use anyhow::Result;
use parking_lot::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{
    AppHandle, Wry,
    menu::{IsMenuItem, MenuEvent, MenuItem},
};

mod menu_def;
use menu_def::MenuIds;

const TRAY_CLICK_DEBOUNCE_MS: u64 = 300;

/// 轻量模式菜单项引用，用于在进入/退出轻量模式时动态设置 enabled 状态
static LITE_MODE_MENU_ITEM: OnceLock<MenuItem<Wry>> = OnceLock::new();

/// 托盘更新互斥锁，防止并发调用 update_icon 导致竞态
///
/// 使用 parking_lot::Mutex 而非 tokio::sync::Mutex，因为持锁期间无 .await 操作
/// （run_on_main_thread 是同步返回 Result 的调度方法）
static TRAY_UPDATE_LOCK: Mutex<()> = parking_lot::const_mutex(());

pub struct Tray {
    limiter: SystemLimiter,
}

impl Default for Tray {
    fn default() -> Self {
        Self {
            limiter: Limiter::new(Duration::from_millis(TRAY_CLICK_DEBOUNCE_MS), SystemClock),
        }
    }
}

singleton!(Tray, TRAY);

impl Tray {
    fn new() -> Self {
        Self::default()
    }

    pub async fn init(&self, app_handle: &AppHandle) -> Result<()> {
        if handle::Handle::global().is_exiting() {
            logging!(debug, Type::Tray, "应用正在退出，跳过托盘初始化");
            return Ok(());
        }

        logging!(info, Type::Tray, "正在从AppHandle创建静态系统托盘");

        // 提前读取配置，在主线程创建托盘时直接使用正确的图标，避免创建后竞态刷新失败
        let verge_data = crate::config::Config::verge().await.latest_arc();
        let tun_enabled = verge_data.enable_tun_mode.unwrap_or(false);
        let sys_proxy = verge_data.enable_system_proxy.unwrap_or(false);

        let icon_bytes: &'static [u8] = if tun_enabled {
            &include_bytes!("../../../icons/tray-icon-tun.png")[..]
        } else if sys_proxy {
            &include_bytes!("../../../icons/tray-icon-sys.png")[..]
        } else {
            &include_bytes!("../../../icons/tray-icon.png")[..]
        };
        let image = tauri::image::Image::from_bytes(icon_bytes)?;

        logging!(
            info,
            Type::Tray,
            "托盘初始图标 — tun={}, sys_proxy={}",
            tun_enabled,
            sys_proxy
        );

        let app_handle_clone = app_handle.clone();
        app_handle.run_on_main_thread(move || {
            let lite_mode = match MenuItem::with_id(
                &app_handle_clone,
                MenuIds::LITE_MODE,
                "轻量模式 / Lite mode",
                true,
                None::<&str>,
            ) {
                Ok(item) => {
                    let _ = LITE_MODE_MENU_ITEM.set(item);
                    // 修复编译错误：使用 if let Some(item) 代替 unwrap/expect
                    if let Some(item) = LITE_MODE_MENU_ITEM.get() {
                        item.clone()
                    } else {
                        unreachable!("LITE_MODE_MENU_ITEM 刚刚设置，不可能为 None")
                    }
                }
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to create lite mode menu item: {}", e);
                    return;
                }
            };

            // 根据当前轻量模式状态初始化菜单项状态
            update_lite_mode_menu(lightweight::is_in_lightweight_mode());

            let quit = match MenuItem::with_id(&app_handle_clone, MenuIds::EXIT, "退出 (Exit)", true, None::<&str>) {
                Ok(item) => item,
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to create exit menu item: {}", e);
                    return;
                }
            };

            let menu = match tauri::menu::MenuBuilder::new(&app_handle_clone)
                .items(&[&lite_mode as &dyn IsMenuItem<Wry>, &quit as &dyn IsMenuItem<Wry>])
                .build()
            {
                Ok(m) => m,
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to build static menu: {}", e);
                    return;
                }
            };

            let mut builder = TrayIconBuilder::with_id("clash-mini-dev-tray")
                .icon(image)
                .menu(&menu)
                .icon_as_template(false);

            builder = builder.show_menu_on_left_click(false);

            match builder.build(&app_handle_clone) {
                Ok(tray) => {
                    let _ = tray.set_tooltip(Some("Clash Mini"));
                    tray.on_tray_icon_event(on_tray_icon_event);
                    tray.on_menu_event(on_menu_event);
                    if let Some(hwnd) = find_own_tray_hwnd() {
                        TRAY_HWND_RAW.store(hwnd.0 as isize, Ordering::Release);
                    }
                    log::info!(target: "app", "[Tray] System tray created successfully");
                }
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to build tray icon on main thread: {}", e);
                }
            }
        })?;

        Ok(())
    }

    /// 根据当前流量接管模式自动切换托盘图标
    ///
    /// 优先级：TUN 模式 > 系统代理 > 默认（手动模式）
    /// 三套图标均通过 include_bytes! 编译时嵌入，运行时按当前状态选择
    ///
    /// 线程安全：通过 TRAY_UPDATE_LOCK 互斥锁保护，避免并发调用导致 UI 竞态
    pub fn update_icon(&self, verge: &IVerge) -> Result<()> {
        let _guard = TRAY_UPDATE_LOCK.lock();

        let tun_enabled = verge.enable_tun_mode.unwrap_or(false);
        let sys_proxy = verge.enable_system_proxy.unwrap_or(false);

        // 所有图标编译时嵌入，运行时按模式选择
        let icon_bytes: &'static [u8] = if tun_enabled {
            &include_bytes!("../../../icons/tray-icon-tun.png")[..]
        } else if sys_proxy {
            &include_bytes!("../../../icons/tray-icon-sys.png")[..]
        } else {
            &include_bytes!("../../../icons/tray-icon.png")[..]
        };

        let image = tauri::image::Image::from_bytes(icon_bytes)?;
        let app_handle = crate::core::handle::Handle::app_handle().clone();
        let app_handle_for_thread = app_handle.clone();

        app_handle
            .run_on_main_thread(move || {
                if let Some(tray) = app_handle_for_thread.tray_by_id("clash-mini-dev-tray") {
                    let _ = tray.set_icon(Some(image));
                }
            })
            .map_err(|e| anyhow::anyhow!("failed to run on main thread: {e}"))?;

        logging!(
            debug,
            Type::Tray,
            "托盘图标已切换 — tun={}, sys_proxy={}",
            tun_enabled,
            sys_proxy
        );

        Ok(())
    }

    /// 启动时刷新托盘状态（图标按当前接管模式初始化）
    pub async fn update_part(&self) -> Result<()> {
        let verge = crate::config::Config::verge().await.latest_arc();
        self.update_icon(&verge)
    }

    fn should_handle_tray_click(&self) -> bool {
        let allow = self.limiter.check();
        if !allow {
            logging!(debug, Type::Tray, "tray click rate limited");
        }
        allow
    }

    /// 以 Win32 原生托盘气泡（`Shell_NotifyIconW(NIF_INFO)`）提示用户程序已在后台静默运行，
    /// 不创建任何 GUI 窗口，避免触发 360 关窗残留记录（BUG-395）。带 2 秒防抖避免连点刷屏。
    pub async fn notify_silent_running(&self) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let can_notify = LAST_SILENT_BALLOON_MS
            .fetch_update(Ordering::SeqCst, Ordering::Acquire, |last| {
                if now_ms.saturating_sub(last) < SILENT_BALLOON_DEBOUNCE_MS {
                    None
                } else {
                    Some(now_ms)
                }
            })
            .is_ok();
        if !can_notify {
            return;
        }

        let language = crate::config::Config::verge().await.latest_arc().language.clone();
        let (title, body) = silent_balloon_texts(language.as_deref());
        let app_handle = handle::Handle::app_handle().clone();
        if let Err(e) = app_handle.run_on_main_thread(move || {
            show_tray_info_balloon(title, body);
        }) {
            logging!(debug, Type::Tray, "调度托盘静默运行气泡提示失败: {}", e);
        }
    }
}

const SILENT_BALLOON_DEBOUNCE_MS: u64 = 2000;
static TRAY_HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static LAST_SILENT_BALLOON_MS: AtomicU64 = AtomicU64::new(0);
static MAIN_WINDOW_FOCUSED: AtomicBool = AtomicBool::new(false);

/// 记录主窗口最近一次失去焦点的毫秒时间戳与聚焦缓存（BUG-361 / BUG-394）
static LAST_MAIN_WINDOW_BLUR_MS: AtomicU64 = AtomicU64::new(0);

pub fn is_main_window_focused_cached() -> bool {
    MAIN_WINDOW_FOCUSED.load(Ordering::Acquire)
}

pub fn record_main_window_focus_change(focused: bool) {
    MAIN_WINDOW_FOCUSED.store(focused, Ordering::Release);
    if focused {
        LAST_MAIN_WINDOW_BLUR_MS.store(0, Ordering::Release);
    } else {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        LAST_MAIN_WINDOW_BLUR_MS.store(now_ms, Ordering::Release);
    }
}

fn silent_balloon_texts(language: Option<&str>) -> (&'static str, &'static str) {
    match language {
        Some("zhtw") | Some("zh-TW") | Some("zh-HK") => (
            "Clash Mini 已在背景靜默執行",
            "為防安全軟體誤殺未彈出主視窗；如需開啟介面，請左鍵點擊右下角系統匣圖示。",
        ),
        Some("zh") | Some("zh-CN") | None => (
            "Clash Mini 已在后台静默运行",
            "为防安全软件误杀未弹出主窗口；如需打开界面，请左键点击右下角托盘图标。",
        ),
        _ => (
            "Clash Mini is running silently in tray",
            "Main window stays hidden to prevent false-positive cleanup. Left-click the tray icon to open.",
        ),
    }
}

fn copy_wide_fixed<const N: usize>(dest: &mut [u16; N], text: &str) {
    for (slot, unit) in dest.iter_mut().take(N.saturating_sub(1)).zip(text.encode_utf16()) {
        *slot = unit;
    }
}

fn find_own_tray_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetWindowThreadProcessId};
    use windows::core::PCWSTR;

    // `tray-icon` 固定使用类名为 "tray_icon_app" 的顶层隐藏消息窗承载 Shell_NotifyIconW
    let class_name: [u16; 14] = [
        b't' as u16,
        b'r' as u16,
        b'a' as u16,
        b'y' as u16,
        b'_' as u16,
        b'i' as u16,
        b'c' as u16,
        b'o' as u16,
        b'n' as u16,
        b'_' as u16,
        b'a' as u16,
        b'p' as u16,
        b'p' as u16,
        0,
    ];
    let current_pid = std::process::id();
    let mut after: Option<HWND> = None;

    // SAFETY: 只读枚举系统内匹配指定类名且归属当前进程 PID 的托盘宿主窗口
    unsafe {
        while let Ok(hwnd) = FindWindowExW(None, after, PCWSTR::from_raw(class_name.as_ptr()), PCWSTR::null()) {
            if hwnd.0.is_null() {
                break;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == current_pid {
                return Some(hwnd);
            }
            after = Some(hwnd);
        }
    }
    None
}

fn show_tray_info_balloon(title: &str, body: &str) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{
        NIF_INFO, NIIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, NOTIFYICONDATAW_0, Shell_NotifyIconW,
    };
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;

    let cached_raw = TRAY_HWND_RAW.load(Ordering::Acquire);
    let hwnd = if cached_raw != 0 {
        let candidate = HWND(cached_raw as *mut std::ffi::c_void);
        // SAFETY: IsWindow 只读验证句柄有效
        if unsafe { IsWindow(Some(candidate)).as_bool() } {
            Some(candidate)
        } else {
            find_own_tray_hwnd()
        }
    } else {
        find_own_tray_hwnd()
    };

    let Some(hwnd) = hwnd else {
        logging!(debug, Type::Tray, "未找到本进程托盘消息窗口，跳过气泡提示");
        return;
    };
    TRAY_HWND_RAW.store(hwnd.0 as isize, Ordering::Release);

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        // `tray-icon` 内部静态 Counter 从 1 起分配，全应用仅初始化单枚托盘图标故恒为 1
        uID: 1,
        uFlags: NIF_INFO,
        dwInfoFlags: NIIF_INFO,
        Anonymous: NOTIFYICONDATAW_0 { uTimeout: 4000 },
        ..Default::default()
    };
    copy_wide_fixed(&mut nid.szInfoTitle, title);
    copy_wide_fixed(&mut nid.szInfo, body);

    // SAFETY: nid 字段已完整填充并与 Win32 NOTIFYICONDATAW 对齐
    if !unsafe { Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool() } {
        logging!(debug, Type::Tray, "Shell_NotifyIconW(NIF_INFO) 托盘气泡发送未成功");
    }
}

fn on_tray_icon_event(_tray_icon: &TrayIcon, tray_event: TrayIconEvent) {
    // 退出期间忽略托盘事件，避免在清理阶段触发新的操作
    if handle::Handle::global().is_exiting() {
        return;
    }

    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Down,
        ..
    } = tray_event
    {
        if !Tray::global().should_handle_tray_click() {
            return;
        }

        // 在投递异步任务前同步采样窗口状态：Windows 点击托盘图标瞬间 Shell_TrayWnd 会抢走焦点，
        // 若失焦发生在 450ms 以内且窗口当前可见未最小化，说明点击前主窗口正处于前台激活状态，左键单击应顺手隐藏收起（BUG-361）。
        let should_hide_foreground = if lightweight::is_in_lightweight_mode() {
            false
        } else {
            let state = WindowManager::get_main_window_state();
            match state {
                crate::utils::window_manager::WindowState::VisibleFocused => true,
                crate::utils::window_manager::WindowState::VisibleUnfocused => {
                    let blur_ms = LAST_MAIN_WINDOW_BLUR_MS.load(std::sync::atomic::Ordering::Acquire);
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    blur_ms > 0 && now_ms.saturating_sub(blur_ms) <= 450
                }
                _ => false,
            }
        };

        AsyncHandler::spawn(move || async move {
            if should_hide_foreground {
                logging!(info, Type::Tray, "主窗口处于前台激活态，左键点击托盘收起隐藏窗口");
                let _ = WindowManager::hide_main_window();
                return;
            }
            // exit_lightweight_mode 返回 true 表示成功退出（含直接显示窗口的兜底分支）；
            // 返回 false 表示正在切换/被限流或超时回滚为轻量态（超时不再重复建窗，避免二次阻塞）。
            if !lightweight::exit_lightweight_mode().await {
                logging!(
                    warn,
                    Type::Tray,
                    "轻量模式退出未完成（可能正在防抖限流或窗口初始化超时）"
                );
            }
        });
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    // 退出期间忽略菜单事件，避免在清理阶段触发新的操作
    if handle::Handle::global().is_exiting() {
        return;
    }

    if event.id.as_ref().is_empty() {
        return;
    }

    // EXIT 菜单项不走防抖，确保用户点退出一定有响应
    let is_exit = event.id.as_ref() == MenuIds::EXIT;
    if !is_exit && !Tray::global().should_handle_tray_click() {
        return;
    }
    let _app_clone = app.clone(); // 修复编译警告：添加下划线前缀
    AsyncHandler::spawn(|| async move {
        match event.id.as_ref() {
            MenuIds::LITE_MODE => {
                if lightweight::is_in_lightweight_mode() {
                    if lightweight::exit_lightweight_mode().await {
                        logging!(info, Type::Tray, "已退出轻量模式");
                    } else {
                        logging!(error, Type::Tray, "退出轻量模式失败");
                    }
                } else {
                    if lightweight::entry_lightweight_mode().await {
                        logging!(info, Type::Tray, "已进入轻量模式");
                    } else {
                        logging!(error, Type::Tray, "进入轻量模式失败");
                    }
                }
            }
            MenuIds::EXIT => {
                feat::quit().await;
            }
            _ => {
                logging!(debug, Type::Tray, "Unhandled tray menu event: {:?}", event.id);
            }
        }
    });
}

/// 根据轻量模式状态更新托盘菜单的对勾及可用性
/// 必须在主线程执行，与 update_icon 保持一致，避免跨线程修改原生菜单资源
pub fn update_lite_mode_menu(is_in: bool) {
    let Some(item) = LITE_MODE_MENU_ITEM.get() else {
        return;
    };
    let item = item.clone();
    let app_handle = crate::core::handle::Handle::app_handle().clone();
    let _ = app_handle.run_on_main_thread(move || {
        if is_in {
            let _ = item.set_text("✔ 轻量模式 / Lite mode");
        } else {
            let _ = item.set_text("轻量模式 / Lite mode");
        }
        let _ = item.set_enabled(true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_balloon_texts_cover_chinese_and_english_fallback() {
        let (title_zh, body_zh) = silent_balloon_texts(Some("zh"));
        assert!(title_zh.contains("静默"));
        assert!(body_zh.contains("左键点击"));

        let (title_tw, body_tw) = silent_balloon_texts(Some("zhtw"));
        assert!(title_tw.contains("靜默"));
        assert!(body_tw.contains("左鍵點擊"));

        let (title_en, body_en) = silent_balloon_texts(Some("en"));
        assert!(title_en.contains("silently"));
        assert!(body_en.contains("Left-click"));
    }

    #[test]
    fn copy_wide_fixed_null_terminates_and_truncates_safely() {
        let mut buf = [0u16; 6];
        copy_wide_fixed(&mut buf, "Clash Mini");
        assert_eq!(buf[5], 0);
        let decoded = String::from_utf16_lossy(&buf[..5]);
        assert_eq!(decoded, "Clash");
    }
}
