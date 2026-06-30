use crate::{core::handle, utils::resolve::window::build_new_window};
use clash_verge_logging::{Type, logging};
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager as _, WebviewWindow, Wry};

/// 窗口操作结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowOperationResult {
    /// 窗口已显示并获得焦点
    Shown,
    /// 窗口已隐藏
    Hidden,
    /// 创建了新窗口
    Created,
    /// 摧毁了窗口
    Destroyed,
    /// 操作失败
    Failed,
    /// 无需操作
    NoAction,
    /// 被防抖限流
    RateLimited,
}

/// 窗口状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowState {
    /// 窗口可见且有焦点
    VisibleFocused,
    /// 窗口可见但无焦点
    VisibleUnfocused,
    /// 窗口最小化
    Minimized,
    /// 窗口隐藏
    Hidden,
    /// 窗口不存在
    NotExist,
}

// 窗口操作防抖机制
/// 窗口操作最大防抖间隔（快速连续点击时的保护阈值）
const WINDOW_OPERATION_DEBOUNCE_MS: u64 = 625;
/// 空闲判定阈值 — 超过此时间无操作，下次点击跳过防抖（立即响应）
const WINDOW_IDLE_THRESHOLD_MS: u64 = 3000;
/// 上次成功执行窗口操作的时间戳（毫秒）
static LAST_WINDOW_OP_MS: AtomicU64 = AtomicU64::new(0);

/// 自适应防抖检查：
/// - 距离上次操作超过 IDLE_THRESHOLD（3s）→ 立即允许（用户长时间未操作，无需防抖）
/// - 距离上次操作小于 IDLE_THRESHOLD → 使用 DEBOUNCE 间隔（用户在频繁操作，防抽风）
/// 使用 fetch_update 将 load-判断-store 合并为单个原子操作，避免并发调用时的 TOCTOU 竞态。
fn should_handle_window_operation() -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    LAST_WINDOW_OP_MS
        .fetch_update(Ordering::SeqCst, Ordering::Relaxed, |last| {
            let elapsed = now.saturating_sub(last);
            let threshold = if elapsed > WINDOW_IDLE_THRESHOLD_MS || last == 0 {
                0
            } else {
                WINDOW_OPERATION_DEBOUNCE_MS
            };
            if elapsed < threshold {
                None // 限流
            } else {
                Some(now) // 放行并更新时间戳
            }
        })
        .is_ok()
}

/// 统一的窗口管理器
pub struct WindowManager;

impl WindowManager {
    pub fn get_main_window_with_state() -> (Option<WebviewWindow<Wry>>, WindowState) {
        let Some(window) = Self::get_main_window() else {
            return (None, WindowState::NotExist);
        };

        let is_minimized = window.is_minimized().unwrap_or(false);
        let is_visible = window.is_visible().unwrap_or(false);
        let is_focused = window.is_focused().unwrap_or(false);

        let state = if is_minimized {
            WindowState::Minimized
        } else if !is_visible {
            WindowState::Hidden
        } else if is_focused {
            WindowState::VisibleFocused
        } else {
            WindowState::VisibleUnfocused
        };

        (Some(window), state)
    }

    pub fn get_main_window_state() -> WindowState {
        match Self::get_main_window() {
            Some(window) => {
                let is_minimized = window.is_minimized().unwrap_or(false);
                let is_visible = window.is_visible().unwrap_or(false);
                let is_focused = window.is_focused().unwrap_or(false);

                if is_minimized {
                    return WindowState::Minimized;
                }

                if !is_visible {
                    return WindowState::Hidden;
                }

                if is_focused {
                    WindowState::VisibleFocused
                } else {
                    WindowState::VisibleUnfocused
                }
            }
            None => WindowState::NotExist,
        }
    }

    /// 获取主窗口实例
    pub fn get_main_window() -> Option<WebviewWindow<Wry>> {
        let app_handle = handle::Handle::app_handle();
        app_handle.get_webview_window("main")
    }

    /// 智能显示主窗口
    pub async fn show_main_window() -> WindowOperationResult {
        // 防抖检查
        if !should_handle_window_operation() {
            return WindowOperationResult::RateLimited;
        }

        logging!(info, Type::Window, "开始智能显示主窗口");
        logging!(debug, Type::Window, "{}", Self::get_window_status_info());

        let current_state = Self::get_main_window_state();

        match current_state {
            WindowState::NotExist => {
                logging!(info, Type::Window, "窗口不存在，创建新窗口");
                if Self::create_window(true).await {
                    logging!(info, Type::Window, "窗口创建成功");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    WindowOperationResult::Created
                } else {
                    logging!(warn, Type::Window, "窗口创建失败");
                    WindowOperationResult::Failed
                }
            }
            WindowState::VisibleFocused => {
                logging!(info, Type::Window, "窗口已经可见且有焦点，无需操作");
                WindowOperationResult::NoAction
            }
            WindowState::VisibleUnfocused | WindowState::Minimized | WindowState::Hidden => {
                let (window, state_after_check) = Self::get_main_window_with_state();
                if state_after_check == WindowState::VisibleFocused {
                    logging!(info, Type::Window, "窗口在检查期间已变为可见和有焦点状态");
                    return WindowOperationResult::NoAction;
                }
                if let Some(window) = window {
                    Self::activate_window(&window)
                } else {
                    WindowOperationResult::Failed
                }
            }
        }
    }

    /// 切换主窗口显示状态（显示/隐藏）
    pub async fn toggle_main_window() -> WindowOperationResult {
        if !should_handle_window_operation() {
            return WindowOperationResult::RateLimited;
        }

        let (window, state) = Self::get_main_window_with_state();

        logging!(debug, Type::Window, "当前状态: {:?}", state);

        match state {
            WindowState::NotExist => Self::handle_not_exist_toggle().await,
            WindowState::VisibleFocused | WindowState::VisibleUnfocused => Self::hide_main_window_internal(window.as_ref()),
            WindowState::Minimized | WindowState::Hidden => Self::activate_existing_main_window(window.as_ref()),
        }
    }

    // 窗口不存在时创建新窗口
    async fn handle_not_exist_toggle() -> WindowOperationResult {
        logging!(info, Type::Window, "窗口不存在，将创建新窗口");
        // 由于已经有防抖保护，直接调用内部方法
        if Self::create_window(true).await {
            WindowOperationResult::Created
        } else {
            WindowOperationResult::Failed
        }
    }

    /// 隐藏主窗口
    pub fn hide_main_window() -> WindowOperationResult {
        let (window, state) = Self::get_main_window_with_state();

        if state == WindowState::NotExist {
            return WindowOperationResult::NoAction;
        }

        if state == WindowState::Hidden || state == WindowState::Minimized {
            return WindowOperationResult::NoAction;
        }

        Self::hide_main_window_internal(window.as_ref())
    }

    // 隐藏主窗口（内部实现）
    fn hide_main_window_internal(window: Option<&WebviewWindow<Wry>>) -> WindowOperationResult {
        logging!(info, Type::Window, "窗口可见，将隐藏窗口");
        let Some(window) = window else {
            logging!(warn, Type::Window, "无法获取窗口实例");
            return WindowOperationResult::Failed;
        };

        let app_handle = handle::Handle::app_handle();
        let label = window.label().to_string();
        let app_handle_clone = app_handle.clone();

        match app_handle.run_on_main_thread(move || {
            if let Some(w) = app_handle_clone.get_webview_window(&label) {
                if let Err(e) = w.hide() {
                    logging!(debug, Type::Window, "隐藏窗口时出错: {}", e);
                }
            }
        }) {
            Ok(_) => {
                logging!(info, Type::Window, "窗口已成功隐藏");
                WindowOperationResult::Hidden
            }
            Err(e) => {
                logging!(warn, Type::Window, "调度窗口隐藏到主线程失败: {}", e);
                WindowOperationResult::Failed
            }
        }
    }

    // 激活已存在的主窗口
    fn activate_existing_main_window(window: Option<&WebviewWindow<Wry>>) -> WindowOperationResult {
        logging!(info, Type::Window, "窗口存在但被隐藏或最小化，将激活窗口");
        if let Some(window) = window {
            Self::activate_window(window)
        } else {
            logging!(warn, Type::Window, "无法获取窗口实例");
            WindowOperationResult::Failed
        }
    }

    /// 激活窗口（取消最小化、显示、设置焦点）
    fn activate_window(window: &WebviewWindow<Wry>) -> WindowOperationResult {
        logging!(info, Type::Window, "开始激活窗口");

        let app_handle = handle::Handle::app_handle();
        let label = window.label().to_string();
        let app_handle_clone = app_handle.clone();

        match app_handle.run_on_main_thread(move || {
            let Some(w) = app_handle_clone.get_webview_window(&label) else {
                return;
            };

            let mut success = true;

            if w.is_minimized().unwrap_or(false) {
                logging!(info, Type::Window, "窗口已最小化，正在取消最小化");
                if let Err(e) = w.unminimize() {
                    logging!(warn, Type::Window, "取消最小化失败: {}", e);
                    success = false;
                }
            }

            if let Err(e) = w.show() {
                logging!(warn, Type::Window, "显示窗口失败: {}", e);
                success = false;
            }

            if let Err(e) = w.set_focus() {
                logging!(warn, Type::Window, "设置窗口焦点失败: {}", e);
                success = false;
            }

            #[cfg(target_os = "windows")]
            {
                if let Err(e) = w.set_always_on_top(true) {
                    logging!(debug, Type::Window, "设置置顶失败（非关键）: {}", e);
                }
                if let Err(e) = w.set_always_on_top(false) {
                    logging!(debug, Type::Window, "取消置顶失败（非关键）: {}", e);
                }
            }

            if success {
                logging!(info, Type::Window, "窗口激活操作在主线程执行成功");
            } else {
                logging!(warn, Type::Window, "窗口激活操作在主线程部分失败");
            }
        }) {
            Ok(_) => {
                #[cfg(target_os = "macos")]
                {
                    logging!(info, Type::Window, "应用 macOS 特定的激活策略");
                    handle::Handle::global().set_activation_policy_regular();
                }
                logging!(info, Type::Window, "已成功调度窗口激活任务到主线程");
                WindowOperationResult::Shown
            }
            Err(e) => {
                logging!(warn, Type::Window, "调度窗口激活到主线程失败: {}", e);
                WindowOperationResult::Failed
            }
        }
    }

    /// 检查窗口是否可见
    pub fn is_main_window_visible(window: Option<&WebviewWindow<Wry>>) -> bool {
        window.map(|w| w.is_visible().unwrap_or(false)).unwrap_or(false)
    }

    /// 检查窗口是否有焦点
    pub fn is_main_window_focused(window: Option<&WebviewWindow<Wry>>) -> bool {
        window.map(|w| w.is_focused().unwrap_or(false)).unwrap_or(false)
    }

    /// 检查窗口是否最小化
    pub fn is_main_window_minimized(window: Option<&WebviewWindow<Wry>>) -> bool {
        window.map(|w| w.is_minimized().unwrap_or(false)).unwrap_or(false)
    }

    /// 创建新窗口,防抖避免重复调用
    /// 窗口创建后保持隐藏，由前端 index.html 在 overlay 渲染后调用 show，避免主题闪烁
    pub fn create_window(should_create: bool) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        Box::pin(async move {
            logging!(info, Type::Window, "开始创建主窗口, should_create={}", should_create);

            if !should_create {
                return false;
            }

            match build_new_window().await {
                Ok(_) => {
                    logging!(info, Type::Window, "新窗口创建成功，等待前端渲染后显示");

                    #[cfg(target_os = "macos")]
                    {
                        handle::Handle::global().set_activation_policy_regular();
                    }

                    true
                }
                Err(e) => {
                    logging!(error, Type::Window, "新窗口创建失败: {}", e);
                    false
                }
            }
        })
    }

    /// 摧毁窗口
    /// 必须在主线程执行，避免与 UI 事件循环竞态导致崩溃
    pub fn destroy_main_window() -> WindowOperationResult {
        let Some(window) = Self::get_main_window() else {
            return WindowOperationResult::NoAction;
        };

        let app_handle = handle::Handle::app_handle();
        let label = window.label().to_string();
        let app_handle_clone = app_handle.clone();

        match app_handle.run_on_main_thread(move || {
            if let Some(w) = app_handle_clone.get_webview_window(&label) {
                if let Err(e) = w.destroy() {
                    logging!(debug, Type::Window, "销毁窗口时出错: {}", e);
                }
            }
        }) {
            Ok(_) => {
                logging!(info, Type::Window, "窗口已摧毁");
                #[cfg(target_os = "macos")]
                {
                    logging!(info, Type::Window, "应用 macOS 特定的激活策略");
                    handle::Handle::global().set_activation_policy_accessory();
                }
                WindowOperationResult::Destroyed
            }
            Err(e) => {
                logging!(warn, Type::Window, "调度窗口销毁到主线程失败: {}", e);
                WindowOperationResult::Failed
            }
        }
    }

    /// 获取详细的窗口状态信息
    fn get_window_status_info() -> String {
        let (window, state) = Self::get_main_window_with_state();
        let is_visible = Self::is_main_window_visible(window.as_ref());
        let is_focused = Self::is_main_window_focused(window.as_ref());
        let is_minimized = Self::is_main_window_minimized(window.as_ref());

        format!("窗口状态: {state:?} | 可见: {is_visible} | 有焦点: {is_focused} | 最小化: {is_minimized}")
    }
}
