use crate::{config::Config, core::tray::Tray, process::AsyncHandler};

use clash_verge_logging::{Type, logging};

use crate::utils::window_manager::{WindowManager, WindowOperationResult};
use anyhow::Result;
use std::sync::atomic::{AtomicU8, Ordering};

// 引入全局异步互斥排队锁，彻底消除轻量模式极速开关时，销毁与创建窗口在异步层面的竞态冲突
// 【锁顺序约定】LIGHTWEIGHT_LOCK → AUTO_SELECT_RUNNING（AtomicBool）
// - entry_lightweight_mode / exit_lightweight_mode 持有 LIGHTWEIGHT_LOCK 时可能调用 trigger_backend_auto_select
// - trigger_backend_auto_select 内部使用 AUTO_SELECT_RUNNING AtomicBool 做互斥（非阻塞式）
// - 由于 AUTO_SELECT_RUNNING 是 AtomicBool 而非阻塞锁，不存在反向等待导致的死锁风险
static LIGHTWEIGHT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
    // Exiting 状态视为仍在轻量模式中（过渡状态），避免状态切换期间逻辑混乱
    matches!(get_state(), LightweightState::In | LightweightState::Exiting)
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

    // 💡 建议 2：进入轻量模式时触发 Mihomo 内核的激进连接清理 (GC) - BUG-258
    // 💡 建议 3：彻底熔断外壳 Rust 后端与内核的常驻数据流订阅 - BUG-259
    AsyncHandler::spawn(|| async {
        // 应用退出检查：在每个关键 await 点后检查 is_exiting，避免退出时仍在执行
        if crate::core::handle::Handle::global().is_exiting() {
            return;
        }
        if !is_in_lightweight_mode() {
            return;
        }
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();

        logging!(
            info,
            Type::Lightweight,
            "[轻量模式] 触发进入时连接清理与数据订阅熔断..."
        );

        if crate::core::handle::Handle::global().is_exiting() {
            return;
        }
        if !is_in_lightweight_mode() {
            return;
        }
        // 激进清空所有网络连接 (GC)
        if let Err(err) = mihomo.close_all_connections().await {
            logging!(
                error,
                Type::Lightweight,
                "[轻量模式] 触发进入时网络连接垃圾回收 (GC) 失败: {err}"
            );
        } else {
            logging!(
                info,
                Type::Lightweight,
                "[轻量模式] 触发进入时网络连接垃圾回收 (GC) 成功"
            );
        }

        if crate::core::handle::Handle::global().is_exiting() {
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

        if crate::core::handle::Handle::global().is_exiting() {
            return;
        }
        if !is_in_lightweight_mode() {
            return;
        }
        // 进入轻量模式时触发节点自愈恢复与自动选点
        if let Some(uid) = crate::module::monitor::get_current_profile_uid().await {
            if crate::core::handle::Handle::global().is_exiting() {
                return;
            }
            if crate::module::monitor::wait_for_clash_ready().await {
                // wait_for_clash_ready 可能阻塞很久，期间用户可能退出轻量模式并手动选点，
                // 必须重新检查状态，避免覆盖用户的手动选择
                if crate::core::handle::Handle::global().is_exiting() {
                    return;
                }
                if !is_in_lightweight_mode() {
                    logging!(info, Type::Lightweight, "[轻量模式] 等待内核就绪期间用户已退出轻量模式，跳过选点");
                } else {
                    let _ = crate::module::monitor::restore_profile_selected_nodes(&uid).await;
                    // 委托后端执行轻量模式进入时的自愈选点；结果经事件回写前端 UI
                    let _ = crate::module::monitor::trigger_backend_auto_select(&uid, None, 0, true, false).await;
                }
            }
        }

        if crate::core::handle::Handle::global().is_exiting() {
            return;
        }
        if !is_in_lightweight_mode() {
            return;
        }

        // 短暂延时等待 WebView 销毁完成以及 Mihomo GC 内存归还分配器
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        if crate::core::handle::Handle::global().is_exiting() {
            return;
        }
        if is_in_lightweight_mode() {
            trim_working_set();
        }
    });

    true
}

#[cfg(target_os = "windows")]
fn trim_working_set() {
    use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
    unsafe {
        // -1 (usize::MAX) 表示强制操作系统修剪工作集页表，将进程不活跃内存交换出物理内存，降低占用表现
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
    logging!(
        info,
        Type::Lightweight,
        "[轻量模式] 已强制修剪 Windows 进程工作集内存，释放空闲缓存"
    );
}

#[cfg(not(target_os = "windows"))]
fn trim_working_set() {
    // 非 Windows 平台（macOS/Linux）依赖原生系统的自动页面换出机制，暂不实现强制释放
}

pub async fn exit_lightweight_mode() -> bool {
    let _guard = LIGHTWEIGHT_LOCK.lock().await;

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
        WindowOperationResult::RateLimited => {
            // 被防抖限流时，检查窗口实际状态再决定是否回滚
            // 因为窗口可能已经显示了（只是 show_main_window 被限流）
            let actual_state = WindowManager::get_main_window_state();
            let is_visible = matches!(
                actual_state,
                crate::utils::window_manager::WindowState::VisibleFocused
                    | crate::utils::window_manager::WindowState::VisibleUnfocused
            );
            if is_visible {
                logging!(
                    info,
                    Type::Lightweight,
                    "显示窗口被防抖限流，但窗口实际已可见，正常退出轻量模式"
                );
                transition_and_log(LightweightState::Exiting, LightweightState::Normal);
            } else {
                logging!(
                    warn,
                    Type::Lightweight,
                    "智能显示主窗口被防抖限流且窗口未显示，回滚轻量模式状态"
                );
                transition_and_log(LightweightState::Exiting, LightweightState::In);
                refresh_lightweight_tray_state().await;
                crate::core::tray::update_lite_mode_menu(true);
                return false;
            }
        }
        _ => {
            logging!(
                warn,
                Type::Lightweight,
                "智能显示主窗口失败，回滚轻量模式状态"
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
