use crate::{core::handle, utils::resolve::window::build_new_window};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
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

/// 操作类型（用于分开防抖，避免不同操作互相限流）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowOpType {
    Show,
    Hide,
    Toggle,
    Destroy,
}

impl WindowOpType {
    const fn as_index(self) -> usize {
        match self {
            Self::Show => 0,
            Self::Hide => 1,
            Self::Toggle => 2,
            Self::Destroy => 3,
        }
    }
}

/// 按操作类型分开的上次操作时间戳数组
/// 索引对应 WindowOpType 的 as_index()
static LAST_WINDOW_OP_MS: [AtomicU64; 4] = [
    AtomicU64::new(0), // Show
    AtomicU64::new(0), // Hide
    AtomicU64::new(0), // Toggle
    AtomicU64::new(0), // Destroy
];

/// 自适应防抖检查（按操作类型分开）：
/// - 距离上次同类型操作超过 IDLE_THRESHOLD（3s）→ 立即允许
/// - 距离上次同类型操作小于 IDLE_THRESHOLD → 使用 DEBOUNCE 间隔
///
/// 使用 fetch_update 将 load-判断-store 合并为单个原子操作，避免并发调用时的 TOCTOU 竞态。
fn should_handle_window_operation(op_type: WindowOpType) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let last_op_atomic = &LAST_WINDOW_OP_MS[op_type.as_index()];
    last_op_atomic
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

/// 持久化的窗口尺寸（退出轻量模式 / 重启时恢复上一次尺寸）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowSizeState {
    width: f64,
    height: f64,
}

/// 保存当前窗口 outer 尺寸到 app 数据目录的 `window_state.json`
///
/// 采用 read-modify-write：在已有 JSON 上 merge width/height，
/// 避免整文件覆盖把 `tauri_plugin_window_state` 管理的 x/y/最大化/全屏 等键抹除
/// （此前每次 resize 的整盖写会导致轻量唤醒后窗口位置也回退默认）。
async fn save_window_size(width: f64, height: f64) {
    let Some(home) = crate::utils::dirs::app_home_dir().ok() else {
        return;
    };
    let path = home.join("window_state.json");
    // 读取现有内容（可能含插件写入的 position/maximized/fullscreen/decorations），不存在则用空对象
    let mut value = match tokio::fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::Value::Object(Default::default())),
        Err(_) => serde_json::Value::Object(Default::default()),
    };
    if let Some(obj) = value.as_object_mut() {
        obj.insert("width".into(), serde_json::Value::from(width));
        obj.insert("height".into(), serde_json::Value::from(height));
    }
    if let Ok(json) = serde_json::to_string(&value) {
        let _ = tokio::fs::write(&path, json).await;
    }
}

/// 窗口尺寸变化时的持久化（带 250ms 节流，避免拖拽期间频繁写盘）
static LAST_RESIZE_SAVE_MS: AtomicI64 = AtomicI64::new(0);

pub async fn save_window_size_on_resize(width: f64, height: f64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let last = LAST_RESIZE_SAVE_MS.load(Ordering::Relaxed);
    if now - last < 250 {
        return;
    }
    LAST_RESIZE_SAVE_MS.store(now, Ordering::Relaxed);
    save_window_size(width, height).await;
}

/// 读取上次保存的窗口尺寸，失败或不存在返回 None
pub async fn restore_window_size() -> Option<(f64, f64)> {
    let home = crate::utils::dirs::app_home_dir().ok()?;
    let path = home.join("window_state.json");
    let content = tokio::fs::read_to_string(&path).await.ok()?;
    let state: WindowSizeState = serde_json::from_str(&content).ok()?;
    // 兜底：保存值必须大于极简窗口最小值，防止异常数据导致窗口不可交互
    if state.width >= crate::utils::resolve::window::MINIMAL_WIDTH
        && state.height >= crate::utils::resolve::window::MINIMAL_HEIGHT
    {
        Some((state.width, state.height))
    } else {
        None
    }
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
        Self::get_main_window_with_state().1
    }

    /// 获取主窗口实例
    pub fn get_main_window() -> Option<WebviewWindow<Wry>> {
        let app_handle = handle::Handle::app_handle();
        app_handle.get_webview_window("main")
    }

    /// 智能显示主窗口
    pub async fn show_main_window() -> WindowOperationResult {
        // 防抖检查
        if !should_handle_window_operation(WindowOpType::Show) {
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
                    // 【注意】此处 50ms 等待是为了给前端页面渲染留出时间
                    // 窗口创建后 WebView 需要加载页面，若立即返回 Created，
                    // 调用方可能在页面未渲染完成时就执行操作导致异常
                    // 更好的方案是等待 PageLoadEvent，但那需要更复杂的同步机制
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
                    Self::activate_window(&window).await
                } else {
                    WindowOperationResult::Failed
                }
            }
        }
    }

    /// 切换主窗口显示状态（显示/隐藏）
    pub async fn toggle_main_window() -> WindowOperationResult {
        if !should_handle_window_operation(WindowOpType::Toggle) {
            return WindowOperationResult::RateLimited;
        }

        let (window, state) = Self::get_main_window_with_state();

        logging!(debug, Type::Window, "当前状态: {:?}", state);

        match state {
            WindowState::NotExist => Self::handle_not_exist_toggle().await,
            WindowState::VisibleFocused | WindowState::VisibleUnfocused => {
                Self::hide_main_window_internal(window.as_ref())
            }
            WindowState::Minimized | WindowState::Hidden => Self::activate_existing_main_window(window.as_ref()).await,
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
    async fn activate_existing_main_window(window: Option<&WebviewWindow<Wry>>) -> WindowOperationResult {
        logging!(info, Type::Window, "窗口存在但被隐藏或最小化，将激活窗口");
        if let Some(window) = window {
            Self::activate_window(window).await
        } else {
            logging!(warn, Type::Window, "无法获取窗口实例");
            WindowOperationResult::Failed
        }
    }

    /// 激活窗口（取消最小化、显示、设置焦点）
    ///
    /// 【根因修复】此前用 `std::sync::mpsc::channel` + `rx.recv()` 同步阻塞等待主线程响应，
    /// 在 async 上下文中会占住 tokio worker 线程；一旦多个 async 任务同时调用
    /// （托盘点击、单例唤醒、轻量模式退出等多路径），会耗尽 tokio worker（默认 = CPU 核心数），
    /// 导致整个 runtime 停止调度，引发心跳探针超时、IPC 无响应、UI 卡死等连锁反应。
    ///
    /// 现改用 `tokio::sync::oneshot` + `rx.await` 真异步等待，让出 tokio worker 线程；
    /// 并加 5 秒超时保护，防止主线程被 Windows 模态循环（如原生 resize loop / COM 调用）
    /// 长时间占用导致永久阻塞。与 `destroy_main_window` 的实现模式保持一致。
    async fn activate_window(window: &WebviewWindow<Wry>) -> WindowOperationResult {
        logging!(info, Type::Window, "开始激活窗口");

        let app_handle = handle::Handle::app_handle();
        let label = window.label().to_string();
        let app_handle_clone = app_handle.clone();

        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();

        match app_handle.run_on_main_thread(move || {
            let Some(w) = app_handle_clone.get_webview_window(&label) else {
                let _ = tx.send(false);
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

            let _ = tx.send(success);
        }) {
            Ok(_) => {
                #[cfg(target_os = "macos")]
                {
                    logging!(info, Type::Window, "应用 macOS 特定的激活策略");
                    handle::Handle::global().set_activation_policy_regular();
                }
                logging!(info, Type::Window, "已成功调度窗口激活任务到主线程");
                // 异步等待主线程执行结果，让出 tokio worker 线程，避免死锁
                // 加 5 秒超时保护：主线程被模态循环占用时不会永久阻塞
                match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
                    Ok(Ok(true)) => WindowOperationResult::Shown,
                    Ok(Ok(false)) => WindowOperationResult::Failed,
                    Ok(Err(_)) => {
                        logging!(warn, Type::Window, "接收窗口激活结果失败（oneshot 发送端被丢弃）");
                        WindowOperationResult::Failed
                    }
                    Err(_) => {
                        logging!(
                            error,
                            Type::Window,
                            "窗口激活超时 5s（主线程可能被模态循环阻塞或 IPC 队列堆积），返回 Failed"
                        );
                        WindowOperationResult::Failed
                    }
                }
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

    pub async fn create_window(should_create: bool) -> bool {
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
    }

    /// 摧毁窗口
    /// 必须在主线程执行，避免与 UI 事件循环竞态导致崩溃
    /// 异步等待主线程闭包执行完毕后返回，确保窗口真正销毁后才释放调用方的锁
    pub async fn destroy_main_window() -> WindowOperationResult {
        let Some(window) = Self::get_main_window() else {
            return WindowOperationResult::NoAction;
        };

        let app_handle = handle::Handle::app_handle();
        let label = window.label().to_string();
        let app_handle_clone = app_handle.clone();

        // 必须在主线程读取窗口几何量（off-main-thread 读取可能失败，导致尺寸从未保存），
        // 通过 oneshot 回传，等销毁闭包执行完后再持久化到 window_state.json
        let (tx, rx) = tokio::sync::oneshot::channel::<Option<(f64, f64)>>();

        match app_handle.run_on_main_thread(move || {
            let saved = if let Some(w) = app_handle_clone.get_webview_window(&label) {
                if let Ok(size) = w.outer_size() {
                    let scale = w
                        .current_monitor()
                        .ok()
                        .flatten()
                        .map(|m| m.scale_factor())
                        .unwrap_or(1.0);
                    Some((size.width as f64 / scale, size.height as f64 / scale))
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(w) = app_handle_clone.get_webview_window(&label) {
                if let Err(e) = w.destroy() {
                    logging!(debug, Type::Window, "销毁窗口时出错: {}", e);
                }
            }
            let _ = tx.send(saved);
        }) {
            Ok(_) => {
                // 等待主线程上的销毁闭包执行完毕，并持久化读取到的尺寸
                // 加 5 秒超时保护：主线程被模态循环/COM 调用占用时不会永久阻塞（与 activate_window 一致）
                match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
                    Ok(Ok(Some((w, h)))) => {
                        save_window_size(w, h).await;
                        logging!(info, Type::Window, "窗口已摧毁");
                        #[cfg(target_os = "macos")]
                        {
                            logging!(info, Type::Window, "应用 macOS 特定的激活策略");
                            handle::Handle::global().set_activation_policy_accessory();
                        }
                        WindowOperationResult::Destroyed
                    }
                    Ok(Ok(None)) => {
                        // 主线程执行了闭包但未能读取尺寸（窗口已销毁或读失败），仍视为已销毁
                        logging!(info, Type::Window, "窗口已摧毁（未获取到尺寸）");
                        #[cfg(target_os = "macos")]
                        {
                            logging!(info, Type::Window, "应用 macOS 特定的激活策略");
                            handle::Handle::global().set_activation_policy_accessory();
                        }
                        WindowOperationResult::Destroyed
                    }
                    Ok(Err(_)) => {
                        // oneshot 发送端被丢弃（主线程闭包未回报），不再乐观宣称已摧毁
                        logging!(warn, Type::Window, "接收窗口销毁结果失败（oneshot 发送端被丢弃），返回 Failed");
                        WindowOperationResult::Failed
                    }
                    Err(_) => {
                        // 主线程 5s 内未执行闭包（被模态循环/COM 阻塞），窗口保持原状，避免永久挂死
                        logging!(
                            error,
                            Type::Window,
                            "窗口销毁超时 5s（主线程可能被模态循环阻塞或 IPC 队列堆积），返回 Failed"
                        );
                        WindowOperationResult::Failed
                    }
                }
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
