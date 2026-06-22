use crate::{
    config::Config,
    core::{timer::Timer, tray::Tray},
};

use clash_verge_logging::{Type, logging};

use crate::utils::window_manager::WindowManager;
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

#[inline]
fn try_transition(from: LightweightState, to: LightweightState) -> bool {
    LIGHTWEIGHT_STATE
        .compare_exchange(from.as_u8(), to.as_u8(), Ordering::AcqRel, Ordering::Relaxed)
        .is_ok()
}

#[inline]
fn record_state_and_log(state: LightweightState) {
    LIGHTWEIGHT_STATE.store(state.as_u8(), Ordering::Release);
    match state {
        LightweightState::Normal => logging!(info, Type::Lightweight, "轻量模式已关闭"),
        LightweightState::In => logging!(info, Type::Lightweight, "轻量模式已开启"),
        LightweightState::Exiting => logging!(info, Type::Lightweight, "正在退出轻量模式"),
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
        enable_auto_light_weight_mode().await;
    }
    if is_silent_start {
        entry_lightweight_mode().await;
    }
    Ok(())
}

/// 已废弃定时器机制，现为关闭按钮直接触发轻量模式的兼容空壳。
pub async fn enable_auto_light_weight_mode() {
    if let Err(e) = Timer::global().init().await {
        logging!(error, Type::Lightweight, "Failed to initialize timer: {e}");
        return;
    }
    logging!(info, Type::Lightweight, "开启自动轻量模式（关闭窗口即刻进入）");
}

/// 已废弃定时器机制，现为兼容空壳。
pub fn disable_auto_light_weight_mode() {
    logging!(info, Type::Lightweight, "关闭自动轻量模式");
}

pub async fn entry_lightweight_mode() -> bool {
    if !try_transition(LightweightState::Normal, LightweightState::In) {
        logging!(debug, Type::Lightweight, "无需进入轻量模式，跳过调用");
        refresh_lightweight_tray_state().await;
        return false;
    }
    record_state_and_log(LightweightState::In);
    WindowManager::destroy_main_window();
    refresh_lightweight_tray_state().await;
    true
}

pub async fn exit_lightweight_mode() -> bool {
    if !try_transition(LightweightState::In, LightweightState::Exiting) {
        logging!(
            debug,
            Type::Lightweight,
            "轻量模式不在退出条件（可能已退出或正在退出），跳过调用"
        );
        refresh_lightweight_tray_state().await;
        return false;
    }
    record_state_and_log(LightweightState::Exiting);
    WindowManager::show_main_window().await;
    record_state_and_log(LightweightState::Normal);
    refresh_lightweight_tray_state().await;
    // 退出轻量模式后，重新启用托盘菜单中的「轻量模式」选项
    crate::core::tray::enable_lite_mode_menu_item();
    true
}
