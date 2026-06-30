use crate::{config::Config, core::tray::Tray, process::AsyncHandler};

use clash_verge_logging::{Type, logging};

use crate::utils::window_manager::{WindowManager, WindowOperationResult};
use anyhow::Result;
use std::sync::atomic::{AtomicU8, Ordering};

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
    let is_enable_auto = verge_config.data_arc().enable_auto_light_weight_mode.unwrap_or(false);
    let is_silent_start = verge_config.data_arc().enable_silent_start.unwrap_or(false);
    if is_enable_auto {
        enable_auto_light_weight_mode();
    }
    if is_silent_start {
        entry_lightweight_mode().await;
    }
    Ok(())
}

/// 轻量模式延迟触发定时器已废弃（关闭窗口直接触发替代）。
pub fn enable_auto_light_weight_mode() {
    logging!(info, Type::Lightweight, "开启自动轻量模式（关闭窗口即刻进入）");
}

/// 已废弃定时器机制，现为兼容空壳。
pub fn disable_auto_light_weight_mode() {
    logging!(info, Type::Lightweight, "关闭自动轻量模式");
}

pub async fn entry_lightweight_mode() -> bool {
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
    let result = WindowManager::destroy_main_window();
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
    });

    true
}

pub async fn exit_lightweight_mode() -> bool {
    if !transition_and_log(LightweightState::In, LightweightState::Exiting) {
        logging!(
            debug,
            Type::Lightweight,
            "轻量模式不在退出条件（可能已退出或正在退出），跳过调用"
        );
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
    // 唤醒常驻监测线程以立即重置为前台周期（15秒）
    crate::module::monitor::MONITOR_WAKEUP_NOTIFY.notify_one();
    true
}
