use crate::{config::Config, core::tray::Tray, process::AsyncHandler};

use clash_verge_logging::{Type, logging};

use crate::utils::window_manager::{WindowManager, WindowOperationResult};
use anyhow::Result;
use std::sync::atomic::{AtomicU8, Ordering};

// 引入全局异步互斥排队锁，彻底消除轻量模式极速开关时，销毁与创建窗口在异步层面的竞态冲突
static LIGHTWEIGHT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 轻量模式 cleanup 任务的 JoinHandle，用于退出时主动中止
static LIGHTWEIGHT_CLEANUP_HANDLE: std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>> =
    std::sync::Mutex::new(None);

/// 中止轻量模式 cleanup 后台任务
pub fn abort_lightweight_cleanup() {
    let value = LIGHTWEIGHT_CLEANUP_HANDLE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    if let Some(handle) = value {
        handle.abort();
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LightweightState {
    Normal = 0,
    In = 1,
    Exiting = 2,
}

impl From<u8> for LightweightState {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::In,
            2 => Self::Exiting,
            _ => Self::Normal,
        }
    }
}

impl LightweightState {
    const fn as_u8(self) -> u8 {
        self as u8
    }
}

static LIGHTWEIGHT_STATE: AtomicU8 = AtomicU8::new(LightweightState::Normal as u8);

#[inline]
fn get_state() -> LightweightState {
    LIGHTWEIGHT_STATE.load(Ordering::Acquire).into()
}

fn transition_and_log(from: LightweightState, to: LightweightState) -> bool {
    if LIGHTWEIGHT_STATE
        .compare_exchange(from.as_u8(), to.as_u8(), Ordering::AcqRel, Ordering::Relaxed)
        .is_ok()
    {
        match to {
            LightweightState::Normal => logging!(info, Type::Lightweight, "轻量模式已关闭"),
            LightweightState::In => logging!(info, Type::Lightweight, "轻量模式已开启"),
            LightweightState::Exiting => logging!(info, Type::Lightweight, "正在退出轻量模式"),
        }
        true
    } else {
        false
    }
}

#[inline]
pub fn is_in_lightweight_mode() -> bool {
    get_state() == LightweightState::In
}

async fn refresh_lightweight_tray_state() {
    if let Err(err) = Tray::global().update_menu().await {
        logging!(warn, Type::Lightweight, "更新托盘轻量模式状态失败: {err}");
    }
}

pub async fn auto_lightweight_boot() -> Result<()> {
    let verge_config = Config::verge().await;
    let is_silent_start = verge_config.data_arc().enable_silent_start.unwrap_or(false);
    if is_silent_start {
        entry_lightweight_mode().await;
    }
    Ok(())
}

pub async fn entry_lightweight_mode() -> bool {
    let _guard = LIGHTWEIGHT_LOCK.lock().await;
    let verge = Config::verge().await;
    if !verge.data_arc().enable_auto_light_weight_mode.unwrap_or(false) {
        let _ = WindowManager::hide_main_window();
        refresh_lightweight_tray_state().await;
        crate::core::tray::update_lite_mode_menu(false);
        return true;
    }

    if !transition_and_log(LightweightState::Normal, LightweightState::In) {
        logging!(debug, Type::Lightweight, "无需进入轻量模式，跳过调用");
        refresh_lightweight_tray_state().await;
        // BUG-001 修复：按当前实际状态更新对勾，防止在已处于轻量模式时错误清除对勾标记
        crate::core::tray::update_lite_mode_menu(is_in_lightweight_mode());
        return false;
    }
    let result = WindowManager::destroy_main_window().await;
    if result == WindowOperationResult::Failed {
        logging!(warn, Type::Lightweight, "销毁主窗口失败，回滚轻量模式状态");
        transition_and_log(LightweightState::In, LightweightState::Normal);
        refresh_lightweight_tray_state().await;
        crate::core::tray::update_lite_mode_menu(false);
        return false;
    }
    refresh_lightweight_tray_state().await;
    crate::core::tray::update_lite_mode_menu(true);

    // 💡 说明：进入轻量模式时【不再】激进清空所有网络连接 (GC)。
    //    按用户要求，轻量模式不等于关闭程序——正在下载的链接不能被切断重连，
    //    故仅保留下方资源回收动作，不影响用户实际代理流量。
    // 💡 建议 3：彻底熔断外壳 Rust 后端与内核的常驻数据流订阅 - BUG-259
    // 进入前 abort 上一个 cleanup 任务，避免累积
    abort_lightweight_cleanup();
    let handle = AsyncHandler::spawn(|| async {
        if !is_in_lightweight_mode() {
            return;
        }
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();

        logging!(
            info,
            Type::Lightweight,
            "[轻量模式] 触发进入时连接清理与数据订阅熔断..."
        );

        if !is_in_lightweight_mode() {
            return;
        }
        if !is_in_lightweight_mode() {
            return;
        }
        // 清理所有 WebSocket 连接 (熔断订阅)
        if let Err(err) = mihomo.clear_all_ws_connections().await {
            logging!(
                error,
                Type::Lightweight,
                "[轻量模式] 彻底熔断外壳与内核的常驻数据流订阅失败: {err}"
            );
        } else {
            logging!(
                info,
                Type::Lightweight,
                "[轻量模式] 彻底熔断外壳与内核的常驻数据流订阅成功"
            );
        }

        if !is_in_lightweight_mode() {
            return;
        }
        // 进入轻量模式时触发节点自愈恢复与自动选点
        if let Some(uid) = crate::module::monitor::get_current_profile_uid().await {
            if crate::module::monitor::wait_for_clash_ready().await {
                let _ = crate::module::monitor::restore_profile_selected_nodes(&uid).await;
                // 委托后端执行轻量模式进入时的自愈选点；结果经事件回写前端 UI
                let _ = crate::module::monitor::trigger_backend_auto_select(&uid, None, 0, true, false).await;
            }
        }

        // 💡 说明：按 Agreement §1.2 规范，彻底删除 SetProcessWorkingSetSize 欺骗性内存修剪，
        // 由 Windows 和 Chromium 自然管理物理内存页，彻底根除窗口恢复时的 Severe Page Fault 与 WebView2 挂起。
    });

    *LIGHTWEIGHT_CLEANUP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);

    true
}

pub async fn exit_lightweight_mode() -> bool {
    let _guard = LIGHTWEIGHT_LOCK.lock().await;

    // 🛡️ [FM-09 修复] 退出轻量模式时，立即 abort 在途的 cleanup 任务，
    // 防止 2 秒内快速连续进出时，后台 cleanup 任务在主窗口已经恢复挂载后误分发清理动作。
    abort_lightweight_cleanup();

    // 状态为 Normal 时，窗口可能仅被隐藏（enable_auto_light_weight_mode=false 路径），
    // 直接显示窗口即可恢复，避免"窗口隐藏但无法退出"的卡死状态。
    if get_state() == LightweightState::Normal {
        logging!(debug, Type::Lightweight, "轻量模式未激活，直接显示窗口");
        let _ = WindowManager::show_main_window().await;
        refresh_lightweight_tray_state().await;
        crate::core::tray::update_lite_mode_menu(false);
        return true;
    }

    if !transition_and_log(LightweightState::In, LightweightState::Exiting) {
        logging!(debug, Type::Lightweight, "轻量模式正在退出中，跳过重复调用");
        refresh_lightweight_tray_state().await;
        return false;
    }
    let result = WindowManager::show_main_window().await;
    match result {
        WindowOperationResult::Shown | WindowOperationResult::Created | WindowOperationResult::NoAction => {
            transition_and_log(LightweightState::Exiting, LightweightState::Normal);
        }
        _ => {
            logging!(
                warn,
                Type::Lightweight,
                "智能显示主窗口未完成/被防抖限流，回滚轻量模式状态"
            );
            transition_and_log(LightweightState::Exiting, LightweightState::In);
            refresh_lightweight_tray_state().await;
            crate::core::tray::update_lite_mode_menu(true);
            return false;
        }
    }
    refresh_lightweight_tray_state().await;
    // 退出轻量模式后，更新托盘菜单中的「轻量模式」选项状态
    crate::core::tray::update_lite_mode_menu(false);
    // 说明：进入轻量模式时已熔断 WebSocket 订阅并清空连接（见 entry_lightweight_mode），
    // 退出时无需后端手动重建 WS 订阅——前端在窗口显示后会通过 useEffect 自动重新订阅
    // WebSocket，MihomoManager 的 refresh_proxy 也会自动重连。此处仅唤醒监测线程。
    // 唤醒常驻监测线程以立即重置为前台周期（15秒），必须放在最后调用
    crate::module::monitor::MONITOR_WAKEUP_NOTIFY.notify_one();
    true
}
