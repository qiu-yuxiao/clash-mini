mod auto_select;
mod connections;
mod daemon;
mod observer;
mod probe;
#[cfg(test)]
mod tests;
mod verdict;

pub use auto_select::trigger_backend_auto_select;
pub(crate) use connections::persist_selected_node_if_matching;
pub use connections::{close_excluded_proxy_connections, close_stale_proxy_connections};
pub use daemon::start_background_monitor;
pub use observer::record_kernel_log_line;
pub(crate) use probe::{get_current_profile_uid, get_filter_and_sort_config};

use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::time::Instant;

/// 互斥锁：同一时间只允许一个 trigger_backend_auto_select 运行
pub(super) static AUTO_SELECT_RUNNING: AtomicBool = AtomicBool::new(false);

/// 后台 monitor 任务的 JoinHandle，运行于独立守护运行时 `DAEMON_RUNTIME`，退出时主动中止
static MONITOR_TASK_HANDLE: Mutex<Option<tokio::task::JoinHandle<()>>> = Mutex::new(None);

/// 中止后台 monitor 常驻任务
pub fn abort_monitor() {
    let value = MONITOR_TASK_HANDLE
        .lock()
        .unwrap_or_else(|e| {
            logging!(
                warn,
                Type::Lightweight,
                "MONITOR_TASK_HANDLE 锁被中毒线程污染，恢复继续"
            );
            e.into_inner()
        })
        .take();
    if let Some(handle) = value {
        handle.abort();
    }
}

/// 用于通知后台监测线程：活动配置已被切换
pub static PROFILE_SWITCH_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 用于通知/唤醒后台监测线程（如退出轻量模式时）
pub static MONITOR_WAKEUP_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 用于通知后台监测线程：前端刚完成手动节点切换，第 0ms 吸收宽限期时间戳
pub static NODE_SWITCH_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 节点延迟下限（毫秒）。低于此值视为伪造/广告延迟。
pub(super) const NODE_DELAY_MIN_MS: u32 = 30;

/// 节点延迟上限（毫秒）。超过此值即判不可用（断流），与全局 `constants::timing::NODE_DELAY_MAX_MS` 同源。
pub(super) use crate::constants::timing::NODE_DELAY_MAX_MS;

/// 批量测速探针超时上限（ms），与判死阈值 `NODE_DELAY_MAX_MS` (5000ms) 统一。
pub(super) const NODE_TEST_TIMEOUT_MS: u32 = NODE_DELAY_MAX_MS;

/// 空闲态节点健康数据保鲜窗口（秒）：无流量期间将静默拨测降频至 15 分钟。
pub(super) const IDLE_HEALTH_FRESH_SECS: u64 = 900;

/// 被动下行流量证活免除主动拨测上限（秒）：近 30 分钟存在下行回包且小于前端 45 分钟容忍窗。
pub(super) const PASSIVE_TRAFFIC_MAX_SKIP_SECS: u64 = 1800;

/// 单向流量停滞（有上行无下行）即时突破缓存复核的冷却时间（秒）。
pub(super) const STALL_REPROBE_COOLDOWN_SECS: u64 = 60;

/// 代理节点建连失败突发窗口（秒）、触发突破免测缓存的失败条数门槛及冷却保护（BUG-415）。
pub(super) const DIAL_FAIL_WINDOW_SECS: u64 = 30;
pub(super) const DIAL_FAIL_REPROBE_THRESHOLD: usize = 3;
pub(super) const DIAL_FAIL_REPROBE_COOLDOWN_SECS: u64 = 60;
/// TUN 默认网卡丢失/切换日志行的最大有效回溯窗口（5 分钟）：独立于 30s 建连突发窗口，防止刚跨过 30s 边界的网卡事件被静默丢弃。
pub(super) const TUN_EVENT_MAX_AGE_SECS: u64 = 300;
/// TUN 默认网卡切换待复核信号的存活上限（60 秒）：在冷却期结束后最多保留约 4 个守护周期，超时自动过期以避免跨周期误归因。
pub(super) const TUN_INTERFACE_CHANGED_TTL_SECS: u64 = 60;

/// Service 模式内核日志单次增量读取字节上限，与默认 128KB 轮转阈值对齐。
pub(super) const MAX_KERNEL_LOG_INCREMENTAL_BYTES: u64 = 128 * 1024;

/// 后台守护线程周期性 `info` 级稳态健康摘要输出间隔（秒，每 10 分钟 1 条，零额外 IPC）。
pub(super) const MONITOR_SUMMARY_INTERVAL_SECS: u64 = 600;

/// 疑似抖动窗口（秒）：失败时若最近成功记录仍在此窗口内则判 `Suspect`，避免瞬时抖动误判死节点。
pub(super) const SUSPECT_WINDOW_SECS: u64 = 30;

/// 间歇性黑洞/超时（Flapping / Brownout）滑动观察窗口（秒）与触发切点门槛（BUG-412）。
pub(super) const BROWNOUT_WINDOW_SECS: u64 = 600;
pub(super) const BROWNOUT_CONFIRM_FAILS: usize = 2;

/// 自愈/缺管快重启最小冷却时间（秒）。
pub(super) const SELF_HEAL_COOLDOWN_SECS: u64 = 25;

/// 检测到内核命名管道消失（`os error 2`）后的二次复核防抖延时（毫秒），避开热重启/拉起瞬态（BUG-411）。
pub(super) const PIPE_MISSING_CONFIRM_DELAY_MS: u64 = 500;

/// 两次自愈**实际切换**的最小间隔（秒），防止故障恢复期反复热切换。
pub(super) const MIN_AUTO_SWITCH_INTERVAL_SECS: u64 = 120;

/// 自愈切换速率上限的滑动窗口（秒）与窗口内最大切换次数。
pub(super) const AUTO_SWITCH_WINDOW_SECS: u64 = 600;
pub(super) const MAX_AUTO_SWITCHES_PER_WINDOW: usize = 3;

/// 节点切换宽限期（秒）：切换后此窗口内的失败仅记日志不计入连续失败次数。
pub(super) const SWITCH_GRACE_PERIOD_SECS: u64 = 20;

/// 子集测速复用近期全组探测缓存的最大年龄（秒）。
pub(super) const METRICS_REUSE_MAX_AGE_SECS: u64 = 30;

/// 并发拨测合并窗口（毫秒）：等待 `METRICS_PROBE_LOCK` 期间复用刚完成的全组结果。
pub(super) const PROBE_COALESCE_WINDOW_MS: u64 = 1500;

/// 自愈阶段复用刚刚产出的全组实时快照的最大年龄（秒）。
pub(super) const SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS: u64 = 10;

/// 全局测量组拨测互斥锁：串行化 `PROXY__METRICS` 全组拨测。
pub(super) static METRICS_PROBE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone)]
pub(super) struct MetricsProbeSnapshot {
    pub(super) completed_at: Instant,
    pub(super) delays: HashMap<String, u32>,
    pub(super) reported_live_to_ui: bool,
}

/// `probe_metrics_group` 的本轮拨测结论；当 `primary_url_distorted` 为真且活跃节点自身通过备用地址时免予判死。
#[derive(Debug, Clone)]
pub(super) struct MetricsProbeOutcome {
    pub(super) delays: HashMap<String, u32>,
    /// 主测速地址整体失败、备用地址至少有真实节点响应 ⇒ 主测速地址可能失真（需在裁决层核对活跃节点自身）。
    pub(super) primary_url_distorted: bool,
}

/// 最近一次成功执行的 `probe_metrics_group` 全量实时延迟快照。
pub(super) static LAST_METRICS_PROBE: parking_lot::Mutex<Option<MetricsProbeSnapshot>> = parking_lot::Mutex::new(None);

/// 「失败服务器」冷却时长（秒）：活跃节点判死时记录其服务器并优先避让，无其它候选时回退全量。
pub(super) const FAILED_SERVER_COOLDOWN_SECS: u64 = 600;

/// 默认主测速 URL。
pub(super) const DEFAULT_TEST_URL: &str = "https://cp.cloudflare.com/generate_204";

/// 备用测速 URL：主 URL 整体失败时复核以区分测速链路失真与节点真断流。
pub(super) const FALLBACK_TEST_URL: &str = "https://www.gstatic.com/generate_204";

/// 真 HTTPS 内容拉取验真 URL（非 `204`，用于识别“204 放行但网页不通”的假节点）
pub(super) const VERIFY_HTTPS_URL_PRIMARY: &str = "https://github.githubassets.com/favicons/favicon.svg";
/// 备用真 HTTPS 内容拉取验真 URL
pub(super) const VERIFY_HTTPS_URL_FALLBACK: &str = "https://www.wikipedia.org/static/favicon/wikipedia.ico";
/// 后台定期真 HTTPS 验真间隔（秒）
pub(super) const HTTPS_VERIFY_INTERVAL_SECS: u64 = 600;
/// 后台定期真 HTTPS 验真“够用即止”门限：活跃节点已验真且匹配子集内已有 >= 3 个已验真 Alive 备用时跳过全组探测。
pub(super) const PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS: usize = 3;
/// 单目标探测免打备用 URL 门限：期望集仅 1 个节点且主 URL 本轮已有 >= 3 个真实节点测通时直接跳过备用复核。
pub(super) const SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS: usize = 3;
/// 被动下行流量证活跳过 `TCP:53` 物理在线探针的最大时效（秒）
pub(super) const ONLINE_CHECK_FLOWING_SKIP_SECS: u64 = 60;
/// 物理网络连通性首选单路 `223.5.5.5:53` 探测超时（毫秒），超时后并发回退其余目标
pub(super) const PRIMARY_NET_PROBE_TIMEOUT_MS: u64 = 250;
/// 活跃节点未验真时触发即时真 HTTPS 复核的冷却时间（秒）
pub(super) const ACTIVE_VERIFY_COOLDOWN_SECS: u64 = 60;
/// 主/备真 HTTPS 验证 URL 全灭或 0 节点通过时的全组退避窗口（秒）
pub(super) const HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS: u64 = 600;

pub(super) static LAST_ACTIVE_VERIFY_CHECK: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
pub(super) static LAST_HTTPS_VERIFY_ALL_FAILED: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
pub(super) static LAST_FLOWING_TRAFFIC_AT: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
/// 内核报告 `[TUN] default interface lost / changed` 时置位，令主循环跳过 60s 在线免测立即探活（BUG-416）。
pub(super) static FORCE_ONLINE_RECHECK: AtomicBool = AtomicBool::new(false);

/// 单节点健康状态（由测量组维护的 history/alive 判定，TUN 下可靠）。
/// 置于 `monitor` 根模块供 `observer` 与 `verdict` 单向共享，消除子模块双向互引。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeStatus {
    /// 测量组尚未测过该节点（数据缺失），不动作、等待测量
    Unknown,
    /// 疑似抖动：`alive=false` 但上次成功仍处于 `SUSPECT_WINDOW_SECS` 内，不累计失败或自愈
    Suspect,
    /// 节点已确认不可用（测速超时/失败/假节点，且无近期成功记录）
    Dead,
    /// 节点健康，附带真实延迟
    Alive(u32),
}

/// 延迟结果数据来源语义标记（BUG-344）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayResultSource {
    /// 本次真实拨测 `PROXY__METRICS` 取得的新鲜实时结果
    LiveProbe,
    /// 前端批量子集测速在 `< METRICS_REUSE_MAX_AGE_SECS` (30s) 内复用的测量组近期缓存
    FreshMetricsCache,
    /// 测速未真正执行或测量链路整体不可用（未产生任何有效拨测结果）
    NotExecuted,
}

/// 自动选点执行结果
pub struct AutoSelectOutcome {
    /// 所有被测节点的延迟（含死节点/超时/错误），用于前端展示
    pub display: Vec<(String, u32)>,
    /// 是否存在可用（有效）节点并完成切换
    pub selected: bool,
    /// 本次实际切换后的 PROXY 新出口节点名（仅 `selected == true` 时为 `Some`）
    pub selected_node: Option<String>,
    /// 本次延迟结果的数据来源语义标记
    pub source: DelayResultSource,
    /// 本次展示数据是否完整覆盖当前 `PROXY` 全组真实节点（而非子集测速或主URL失真残缺结果）
    pub covers_all_group_nodes: bool,
}

/// 节点侧自愈所需的连续独立确认次数（§5.1：连续 2 次检测失败才自愈）
pub(super) const FAILOVER_CONFIRM_FAILS: u32 = 2;

/// 内核 API 通道连续异常达到该次数后，才允许进入统一自愈裁决（非节点侧故障，见 §5.1）
pub(super) const API_ERROR_HEAL_THRESHOLD: u32 = 3;

/// 正常健康检测间隔（秒）
pub(super) const NORMAL_CHECK_INTERVAL_SECS: u64 = 15;

/// 重试模式健康检测间隔（秒）
pub(super) const RETRY_CHECK_INTERVAL_SECS: u64 = 3;

/// 离线时主循环间隔（秒）：与离线网络探测间隔对齐，确保 5 秒快速探测不被 15 秒主循环限制
pub(super) const OFFLINE_CHECK_INTERVAL_SECS: u64 = 5;

/// 后台监测线程心跳日志周期（循环次数）
///
/// 【设计依据】
/// - 正常模式 15s/周期，20 个周期 ≈ 5 分钟
/// - 与 UI 线程心跳探针（5 秒）的"5 倍"原则保持一致：5s × 60 = 5min
/// - 远小于 v2.6.5 事件中的 4h22m 空白期，能快速区分"线程已死" vs "正常运行无事件"
/// - debug 级别，不影响生产日志体积（默认 Info 级别不显示）
///
/// 【设计盲区修复】
/// 此前 monitor 主循环只在状态变化时打日志（节点变化/网络变化/失败告警），
/// 导致正常运行时连续几小时无日志输出，与"线程已死"无法区分。
/// 加周期性心跳后，排障时启用 RUST_LOG=debug 即可看到心跳，立刻判定线程状态。
pub(super) const MONITOR_HEARTBEAT_CYCLE_COUNT: u64 = 20;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    pub filter_text: String,
}

// 识别广告/假节点的实现已迁移到 crate::utils::node::is_dummy_node
// 以便 enhance/mod.rs 和 lifecycle.rs 共享同一份判定逻辑，避免遗漏
pub use crate::utils::node::is_dummy_node;

/// 最近一次向前端同步的活跃节点状态（BUG-370 / BUG-387）
///
/// Mihomo 在拨测超时时不向 `proxy.history` 追加失败记录，若在推送 `0 (Timeout)` 时记录了
/// 断流前的旧成功 `history.last().time`，随后节点恢复健康时就会误判为「无需回传」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UiActiveSyncState {
    pub(super) node_name: String,
    pub(super) history_time: Option<String>,
    pub(super) reported_alive: bool,
}

pub(super) static LAST_NOTIFIED_HEALTH: parking_lot::Mutex<&'static str> =
    parking_lot::Mutex::new("network_health::ok");
pub(super) static LAST_MANUAL_NODE_SWITCH: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
pub(super) static MANUAL_SWITCH_EPOCH: AtomicU64 = AtomicU64::new(0);
pub(super) static LAST_UI_ACTIVE_SYNC: parking_lot::Mutex<Option<UiActiveSyncState>> = parking_lot::Mutex::new(None);
pub(super) static ALERT_DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// 读取当前手动切换代际号，供后台异步选点任务感知用户手动抢占并退让（BUG-414）。
pub fn current_manual_switch_epoch() -> u64 {
    MANUAL_SWITCH_EPOCH.load(Ordering::Acquire)
}

/// 记录前端手动切换产生的新节点切换时间戳并立即唤醒监测循环吸收（BUG-336），不触发额外全组拨测；
/// 同时递增全局手动切换代际号，使在途自动选点不会在测速结束后覆盖用户最新选择（BUG-388）。
pub fn notify_manual_node_switched() {
    MANUAL_SWITCH_EPOCH.fetch_add(1, Ordering::AcqRel);
    *LAST_MANUAL_NODE_SWITCH.lock() = Some(Instant::now());
    observer::notify_observer_manual_switch(chrono::Utc::now());
    NODE_SWITCH_NOTIFY.notify_one();
}

pub(super) fn reset_monitor_static_state() {
    *LAST_NOTIFIED_HEALTH.lock() = "network_health::ok";
    *LAST_MANUAL_NODE_SWITCH.lock() = None;
    MANUAL_SWITCH_EPOCH.store(0, Ordering::Release);
    *LAST_UI_ACTIVE_SYNC.lock() = None;
    *LAST_METRICS_PROBE.lock() = None;
    *LAST_ACTIVE_VERIFY_CHECK.lock() = None;
    *LAST_HTTPS_VERIFY_ALL_FAILED.lock() = None;
    *LAST_FLOWING_TRAFFIC_AT.lock() = None;
    FORCE_ONLINE_RECHECK.store(false, Ordering::Release);
    observer::reset_observer_state();
}

pub(super) fn emit_network_health_notice(next_status: &'static str) {
    let should_emit = {
        let mut guard = LAST_NOTIFIED_HEALTH.lock();
        if *guard != next_status {
            *guard = next_status;
            true
        } else {
            false
        }
    };
    if should_emit && !crate::module::lightweight::is_in_lightweight_mode() {
        crate::core::handle::Handle::notice_message(next_status, "");
    }
}
