use crate::config::Config;

use clash_verge_logging::{Type, logging};

use crate::utils::window_manager::{WindowManager, WindowOperationResult};
use anyhow::Result;
use std::sync::atomic::{AtomicU8, Ordering};

// 引入全局异步互斥排队锁，彻底消除轻量模式极速开关时，销毁与创建窗口在异步层面的竞态冲突
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
    get_state() == LightweightState::In
}

/// 判断配置列表中是否包含至少一条用户真实导入的订阅/本地节点配置（排除仅含内置 Merge/Script 的初始空态）
pub fn has_real_profile_items(items: Option<&[crate::config::PrfItem]>) -> bool {
    items.is_some_and(|list| {
        list.iter()
            .any(|item| matches!(item.itype.as_deref(), Some("remote" | "local")))
    })
}

/// 启动期全场景静默进托盘判定（BUG-395）：
/// 仅当开启 `enable_silent_start` 且已存在至少一条真实订阅/本地配置时，
/// 无论开机自启（含 `--silent`）还是手动双击图标启动，一律跳过主窗口直接静默进入托盘；
/// 尚无任何真实订阅的纯新用户仍弹出主窗口展示首次引导。
pub async fn should_enter_silent_tray_on_launch() -> bool {
    let is_silent_start = Config::verge().await.data_arc().enable_silent_start.unwrap_or(false);
    if !is_silent_start {
        return false;
    }
    let profiles = Config::profiles().await;
    has_real_profile_items(profiles.data_arc().get_items().map(Vec::as_slice))
}

pub async fn auto_lightweight_boot() -> Result<()> {
    if should_enter_silent_tray_on_launch().await {
        entry_lightweight_mode().await;
    }
    Ok(())
}

pub async fn entry_lightweight_mode() -> bool {
    let _guard = LIGHTWEIGHT_LOCK.lock().await;
    let verge = Config::verge().await;
    if !verge.data_arc().enable_auto_light_weight_mode.unwrap_or(false) {
        let _ = WindowManager::hide_main_window();
        crate::core::tray::update_lite_mode_menu(false);
        return true;
    }

    if !transition_and_log(LightweightState::Normal, LightweightState::In) {
        logging!(debug, Type::Lightweight, "无需进入轻量模式，跳过调用");
        // BUG-001 修复：按当前实际状态更新对勾，防止在已处于轻量模式时错误清除对勾标记
        crate::core::tray::update_lite_mode_menu(is_in_lightweight_mode());
        return false;
    }
    let result = WindowManager::destroy_main_window().await;
    if result == WindowOperationResult::Failed {
        logging!(warn, Type::Lightweight, "销毁主窗口失败，回滚轻量模式状态");
        transition_and_log(LightweightState::In, LightweightState::Normal);
        crate::core::tray::update_lite_mode_menu(false);
        return false;
    }
    // 💡 建议 3：彻底熔断外壳 Rust 后端与内核的常驻数据流订阅 - BUG-259
    // 在销毁窗口后【同步、不可 abort】地立即熔断旧 WS 订阅。
    // 关键点：entry/exit 共用同一把 LIGHTWEIGHT_LOCK，exit 必须等本函数释放锁后才能
    // 重建窗口，因此此处 clear 必然先于任何 show_main_window 执行——
    // 彻底杜绝“快速进→出轻量模式”时 cleanup 任务被 abort 导致旧 WS 沦为孤儿、
    // 内核 socket 与 tokio 任务累积泄漏的问题。
    {
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
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
    }

    crate::core::tray::update_lite_mode_menu(true);

    // 💡 说明：进入轻量模式时【不再】激进清空所有网络连接 (GC)。
    //    按用户要求，轻量模式不等于关闭程序——正在下载的链接不能被切断重连，
    //    故仅保留下方资源回收动作，不影响用户实际代理流量。
    // 【BUG-402】关窗进轻量模式与代理健康无关，不再回拨监测计时器或额外唤醒体检；
    // 后台守护线程维持恒定 15s 巡检节奏（冷启动无窗态则由 monitor::is_first_run 独立初始化）。

    true
}

pub async fn exit_lightweight_mode() -> bool {
    let _guard = LIGHTWEIGHT_LOCK.lock().await;

    // 状态为 Normal 时，窗口可能仅被隐藏（enable_auto_light_weight_mode=false 路径），
    // 直接显示窗口即可恢复，避免"窗口隐藏但无法退出"的卡死状态。
    if get_state() == LightweightState::Normal {
        logging!(debug, Type::Lightweight, "轻量模式未激活，直接显示窗口");
        let _ = WindowManager::show_main_window().await;
        crate::core::tray::update_lite_mode_menu(false);
        return true;
    }

    if !transition_and_log(LightweightState::In, LightweightState::Exiting) {
        logging!(debug, Type::Lightweight, "轻量模式正在退出中，跳过重复调用");
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
            crate::core::tray::update_lite_mode_menu(true);
            return false;
        }
    }
    // 退出轻量模式后，更新托盘菜单中的「轻量模式」选项状态
    crate::core::tray::update_lite_mode_menu(false);
    // 说明：进入轻量模式时已熔断 WebSocket 订阅并清空连接（见 entry_lightweight_mode），
    // 退出时无需后端手动重建 WS 订阅——前端在窗口显示后会通过 useEffect 自动重新订阅
    // WebSocket，MihomoManager 的 refresh_proxy 也会自动重连。此处仅唤醒监测线程。
    // 唤醒常驻监测线程以立即重置为前台周期（15秒），必须放在最后调用
    crate::module::monitor::MONITOR_WAKEUP_NOTIFY.notify_one();
    true
}

#[cfg(test)]
mod tests {
    use super::has_real_profile_items;
    use crate::config::PrfItem;

    #[test]
    fn real_profile_detection_ignores_empty_and_merge_only_lists() {
        assert!(!has_real_profile_items(None));
        assert!(!has_real_profile_items(Some(&[])));

        let merge_only = [
            PrfItem {
                itype: Some("merge".into()),
                ..Default::default()
            },
            PrfItem {
                itype: Some("script".into()),
                ..Default::default()
            },
        ];
        assert!(!has_real_profile_items(Some(&merge_only)));

        let with_remote = [
            PrfItem {
                itype: Some("merge".into()),
                ..Default::default()
            },
            PrfItem {
                itype: Some("remote".into()),
                ..Default::default()
            },
        ];
        assert!(has_real_profile_items(Some(&with_remote)));

        let with_local = [PrfItem {
            itype: Some("local".into()),
            ..Default::default()
        }];
        assert!(has_real_profile_items(Some(&with_local)));
    }
}
