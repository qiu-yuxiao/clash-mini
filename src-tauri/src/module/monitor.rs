use crate::{
    config::{Config, PrfItem, PrfSelected, profiles_patch_item_safe},
    process::AsyncHandler,
};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::time::{Duration, Instant, sleep};

/// 互斥锁：同一时间只允许一个 trigger_backend_auto_select 运行
static AUTO_SELECT_RUNNING: AtomicBool = AtomicBool::new(false);

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
const NODE_DELAY_MIN_MS: u32 = 30;

/// 节点延迟上限（毫秒）。超过此值即判不可用（断流），与全局 `constants::timing::NODE_DELAY_MAX_MS` 同源。
use crate::constants::timing::NODE_DELAY_MAX_MS;

/// 批量测速探针超时上限（ms），与判死阈值 `NODE_DELAY_MAX_MS` (5000ms) 统一。
const NODE_TEST_TIMEOUT_MS: u32 = NODE_DELAY_MAX_MS;

/// 空闲态节点健康数据保鲜窗口（秒）：无流量期间将静默拨测降频至 15 分钟。
const IDLE_HEALTH_FRESH_SECS: u64 = 900;

/// 被动下行流量证活免除主动拨测上限（秒）：近 30 分钟存在下行回包且小于前端 45 分钟容忍窗。
const PASSIVE_TRAFFIC_MAX_SKIP_SECS: u64 = 1800;

/// 单向流量停滞（有上行无下行）即时突破缓存复核的冷却时间（秒）。
const STALL_REPROBE_COOLDOWN_SECS: u64 = 60;

/// 疑似抖动窗口（秒）：失败时若最近成功记录仍在此窗口内则判 `Suspect`，避免瞬时抖动误判死节点。
const SUSPECT_WINDOW_SECS: u64 = 30;

/// 两次自愈**实际切换**的最小间隔（秒），防止故障恢复期反复热切换。
const MIN_AUTO_SWITCH_INTERVAL_SECS: u64 = 120;

/// 自愈切换速率上限的滑动窗口（秒）与窗口内最大切换次数。
const AUTO_SWITCH_WINDOW_SECS: u64 = 600;
const MAX_AUTO_SWITCHES_PER_WINDOW: usize = 3;

/// 节点切换宽限期（秒）：切换后此窗口内的失败仅记日志不计入连续失败次数。
const SWITCH_GRACE_PERIOD_SECS: u64 = 20;

/// 子集测速复用近期全组探测缓存的最大年龄（秒）。
const METRICS_REUSE_MAX_AGE_SECS: u64 = 30;

/// 并发拨测合并窗口（毫秒）：等待 `METRICS_PROBE_LOCK` 期间复用刚完成的全组结果。
const PROBE_COALESCE_WINDOW_MS: u64 = 1500;

/// 自愈阶段复用刚刚产出的全组实时快照的最大年龄（秒）。
const SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS: u64 = 10;

/// 全局测量组拨测互斥锁：串行化 `PROXY__METRICS` 全组拨测。
static METRICS_PROBE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone)]
struct MetricsProbeSnapshot {
    completed_at: Instant,
    delays: HashMap<String, u32>,
    reported_live_to_ui: bool,
}

/// `probe_metrics_group` 的本轮拨测结论；`primary_url_distorted` 为真时裁决层须返回 `NoAction`。
#[derive(Debug, Clone)]
struct MetricsProbeOutcome {
    delays: HashMap<String, u32>,
    /// 主测速地址整体失败、备用地址成功 ⇒ 测速地址失真，节点侧正常。
    primary_url_distorted: bool,
}

/// 最近一次成功执行的 `probe_metrics_group` 全量实时延迟快照。
static LAST_METRICS_PROBE: parking_lot::Mutex<Option<MetricsProbeSnapshot>> = parking_lot::Mutex::new(None);

/// 「失败服务器」冷却时长（秒）：活跃节点判死时记录其服务器并优先避让，无其它候选时回退全量。
const FAILED_SERVER_COOLDOWN_SECS: u64 = 600;

/// 默认主测速 URL。
const DEFAULT_TEST_URL: &str = "https://cp.cloudflare.com/generate_204";

/// 备用测速 URL：主 URL 整体失败时复核以区分测速链路失真与节点真断流。
const FALLBACK_TEST_URL: &str = "https://www.gstatic.com/generate_204";

/// 真 HTTPS 内容拉取验真 URL（非 `204`，用于识别“204 放行但网页不通”的假节点）
const VERIFY_HTTPS_URL_PRIMARY: &str = "https://github.githubassets.com/favicons/favicon.svg";
/// 备用真 HTTPS 内容拉取验真 URL
const VERIFY_HTTPS_URL_FALLBACK: &str = "https://www.wikipedia.org/static/favicon/wikipedia.ico";
/// 后台定期真 HTTPS 验真间隔（秒）
const HTTPS_VERIFY_INTERVAL_SECS: u64 = 600;
/// 后台定期真 HTTPS 验真“够用即止”门限：活跃节点已验真且匹配子集内已有 >= 3 个已验真 Alive 备用时跳过全组探测。
const PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS: usize = 3;
/// 单目标探测免打备用 URL 门限：期望集仅 1 个节点且主 URL 本轮已有 >= 3 个真实节点测通时直接跳过备用复核。
const SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS: usize = 3;
/// 被动下行流量证活跳过 `TCP:53` 物理在线探针的最大时效（秒）
const ONLINE_CHECK_FLOWING_SKIP_SECS: u64 = 60;
/// 物理网络连通性首选单路 `223.5.5.5:53` 探测超时（毫秒），超时后并发回退其余目标
const PRIMARY_NET_PROBE_TIMEOUT_MS: u64 = 250;
/// 活跃节点未验真时触发即时真 HTTPS 复核的冷却时间（秒）
const ACTIVE_VERIFY_COOLDOWN_SECS: u64 = 60;
/// 主/备真 HTTPS 验证 URL 全灭或 0 节点通过时的全组退避窗口（秒）
const HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS: u64 = 600;

static LAST_ACTIVE_VERIFY_CHECK: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
static LAST_HTTPS_VERIFY_ALL_FAILED: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
static LAST_FLOWING_TRAFFIC_AT: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);

/// 纯函数（便于单测锁定，BUG-403）：判断最近一次观测到活跃节点真实下行回包（`Flowing`）是否仍在在线证活窗内
const fn has_recent_flowing_traffic(elapsed: Option<Duration>) -> bool {
    matches!(elapsed, Some(d) if d.as_secs() < ONLINE_CHECK_FLOWING_SKIP_SECS)
}

fn is_recent_flowing_traffic_active() -> bool {
    has_recent_flowing_traffic(LAST_FLOWING_TRAFFIC_AT.lock().map(|t| t.elapsed()))
}

/// 纯函数（便于单测锁定，BUG-388）：判断当前是否处于真 HTTPS 验真全败退避冷却窗口内
const fn is_https_verify_backoff_active(last_failed_elapsed_secs: Option<u64>) -> bool {
    matches!(last_failed_elapsed_secs, Some(secs) if secs < HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS)
}

fn prioritize_verified_candidates_with<F>(candidates: Vec<(String, u32)>, is_verified: F) -> Vec<(String, u32)>
where
    F: Fn(&str) -> bool,
{
    if candidates.iter().any(|(name, _)| is_verified(name)) {
        candidates.into_iter().filter(|(name, _)| is_verified(name)).collect()
    } else {
        candidates
    }
}

fn select_fastest_candidate_with_filter<F>(
    candidates: Vec<(String, u32)>,
    latest_filter_text: &str,
    is_verified: F,
) -> Option<(String, u32)>
where
    F: Fn(&str) -> bool,
{
    let filtered: Vec<(String, u32)> = candidates
        .into_iter()
        .filter(|(name, _)| match_node_filter(name, latest_filter_text))
        .collect();
    let pool = prioritize_verified_candidates_with(filtered, is_verified);
    pool.into_iter().min_by_key(|(_, delay)| *delay)
}

fn select_fastest_candidate(candidates: Vec<(String, u32)>, latest_filter_text: &str) -> Option<(String, u32)> {
    select_fastest_candidate_with_filter(
        candidates,
        latest_filter_text,
        crate::utils::node::is_verified_true_node,
    )
}

/// 判断某条活跃连接在前台用户显式切换节点至 `target_node` 后是否应被断开（BUG-396）：
/// - 仅匹配经过 `group_name`（`"PROXY"`）代理组的连接，绝不触碰 `DIRECT` / `REJECT` 直连或拦截连接；
/// - 若连接已位于 `target_node` 上（如 F4/F1 重测后最快节点未变），返回 `false` 保持零中断；
/// - 若连接仍挂在任意非 `target_node` 的历史代理节点上（含多跳前残留），返回 `true` 立即清理。
pub fn should_close_stale_proxy_connection(chains: &[String], group_name: &str, target_node: &str) -> bool {
    let target = target_node.trim();
    let group = group_name.trim();
    !target.is_empty()
        && !group.is_empty()
        && !target.eq_ignore_ascii_case("DIRECT")
        && !target.eq_ignore_ascii_case("REJECT")
        && !target.eq_ignore_ascii_case("GLOBAL")
        && chains.iter().any(|c| c == group)
        && !chains.iter().any(|c| c == target)
}

/// 判断某条活跃代理连接的真实出口节点（`chains[0]`）是否已被当前有效过滤词排除（BUG-405）：
/// - 过滤词为空或处于半输入态时返回 `false`，不触碰任何连接；
/// - 仅匹配经过 `group`（`"PROXY"`）且末端为真实代理节点的连接，绝不误伤 `DIRECT` / `REJECT`。
pub(crate) fn should_close_excluded_proxy_connection(chains: &[String], group: &str, filter_text: &str) -> bool {
    if crate::utils::node::is_incomplete_filter_input(filter_text) || !chains.iter().any(|c| c == group) {
        return false;
    }
    let Some(exit_node) = chains.first().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return false;
    };
    if exit_node.eq_ignore_ascii_case("DIRECT")
        || exit_node.eq_ignore_ascii_case("REJECT")
        || exit_node.eq_ignore_ascii_case("REJECT-DROP")
        || exit_node.eq_ignore_ascii_case("PASS")
        || exit_node.eq_ignore_ascii_case("COMPATIBLE")
        || exit_node.eq_ignore_ascii_case("PROXY")
        || exit_node.eq_ignore_ascii_case("GLOBAL")
    {
        return false;
    }
    !match_node_filter(exit_node, filter_text.trim())
}

async fn close_matching_proxy_connections<F>(mut predicate: F) -> usize
where
    F: FnMut(&[String]) -> bool,
{
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let Ok(snapshot) = mihomo.get_connections().await else {
        return 0;
    };
    let ids: Vec<String> = snapshot
        .connections
        .unwrap_or_default()
        .into_iter()
        .filter(|conn| predicate(&conn.chains))
        .map(|conn| conn.id)
        .collect();
    let count = ids.len();
    for id in ids {
        let _ = mihomo.close_connection(&id).await;
    }
    count
}

/// 前台用户显式触发选点切换后，精准清理仍挂在非 `target_node` 旧出口上的 `PROXY` 存量长连接（BUG-396）。
/// 注意：后台自愈与常规冷启动保持零断流热切换；过滤越界清理由 `close_excluded_proxy_connections` 负责。
pub async fn close_stale_proxy_connections(target_node: &str) {
    let count =
        close_matching_proxy_connections(|chains| should_close_stale_proxy_connection(chains, "PROXY", target_node))
            .await;
    if count > 0 {
        logging!(
            info,
            Type::Lightweight,
            "[前台切点] 已精准清理 {} 条非当前目标节点（{}）的存量代理连接",
            count,
            target_node
        );
    }
}

/// 清理仍绑定在已被过滤词排除的代理节点上的存量连接（BUG-405），绝不触碰合规代理或 `DIRECT`/`REJECT`。
pub async fn close_excluded_proxy_connections(filter_text: &str) {
    let trimmed = filter_text.trim();
    if crate::utils::node::is_incomplete_filter_input(trimmed) {
        return;
    }
    let count =
        close_matching_proxy_connections(|chains| should_close_excluded_proxy_connection(chains, "PROXY", trimmed))
            .await;
    if count > 0 {
        logging!(
            info,
            Type::Lightweight,
            "[过滤边界] 已清理 {} 条滞留在已排除节点上的代理连接（过滤词: \"{}\"）",
            count,
            trimmed
        );
    }
}

pub(crate) async fn persist_selected_node_if_matching(profile_uid: &str, fastest_node: &str) {
    let filter_text = get_filter_and_sort_config(profile_uid)
        .await
        .0
        .filter_text
        .trim()
        .to_string();
    if match_node_filter(fastest_node, &filter_text) {
        let new_selected = vec![PrfSelected {
            name: Some("PROXY".into()),
            now: Some(fastest_node.into()),
        }];
        let patch_item = PrfItem {
            selected: Some(new_selected),
            ..Default::default()
        };
        let uid_smart: smartstring::alias::String = profile_uid.into();
        if let Err(e) = profiles_patch_item_safe(&uid_smart, &patch_item).await {
            logging!(warn, Type::Lightweight, "[后台监测] 回写 profile.selected 失败: {e}");
        }
    } else {
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] 最优节点 {} 不在当前过滤范围「{}」内，跳过回写 profile.selected 以免污染",
            fastest_node,
            filter_text
        );
    }
}

/// 判断是否需要触发「后台定期真 HTTPS 验真」（BUG-400）：
/// 1. 仅考察匹配当前 `filter_text` 的非伪、非软隔离真实候选节点；
/// 2. 仅当存在至少 1 个当前 `NodeStatus::Alive(_)`（已通过普通测速证实在线）但尚未 `verified_true`
///    的可疑节点时才可能触发，彻底排除长期离线/超时的普通死节点拖累全组反复陪跑 HTTPS 验真；
/// 3. 够用即止：若当前活跃节点已是 `verified_true`，且匹配子集内已有
///    `>= PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS` (3) 个 `Alive` + `verified_true` 备用节点，
///    跳过后台定期全组探测（切点缺候选时仍由「选点前置验真」兜底）。
fn should_run_periodic_https_verify_with<FA, FV>(
    members: &[String],
    active_node: Option<&str>,
    filter_text: &str,
    is_fake_node: impl Fn(&str) -> bool,
    is_alive: FA,
    is_verified: FV,
) -> bool
where
    FA: Fn(&str) -> bool,
    FV: Fn(&str) -> bool,
{
    let mut has_alive_unverified = false;
    let mut verified_alive_standbys = 0usize;

    for name in members {
        if is_dummy_node(name) || is_fake_node(name) || !match_node_filter(name, filter_text) {
            continue;
        }
        if !is_alive(name) {
            continue;
        }
        if is_verified(name) {
            if active_node != Some(name.as_str()) {
                verified_alive_standbys += 1;
            }
        } else {
            has_alive_unverified = true;
        }
    }

    if !has_alive_unverified {
        return false;
    }

    let active_verified = active_node.is_some_and(&is_verified);
    if active_verified && verified_alive_standbys >= PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS {
        return false;
    }

    true
}

/// 对当前运行节点执行一轮真 HTTPS（非 `204`）验真并推进持久化状态机：
/// - 若全部节点均已 `verified_true` 或处于 24h 软隔离 `is_confirmed_fake`，零流量直接返回；
/// - 物理断网或主/备真 HTTPS 地址无一通过时，绝不递增任何节点失败计数；
/// - 新进入 24h 软隔离的假节点且非轻量模式时，通知前端刷新代理列表以隐藏。
/// - 【S5 / Sync-23】与主 URL 全组测速共用 `METRICS_PROBE_LOCK` 并使用同一超时口径；
///   结果只写 verify 状态机，其写入的 `alive` / `history` 不会被当成免拨测轮的判死证据
///   （见 `evaluate_failover` 的本轮实拨门检）。
async fn run_https_verification_pass(mihomo: &tauri_plugin_mihomo::Mihomo, candidate_nodes: &[String], reason: &str) {
    if !crate::utils::node::has_pending_unverified_nodes(candidate_nodes) {
        return;
    }
    let backoff_elapsed = LAST_HTTPS_VERIFY_ALL_FAILED.lock().map(|t| t.elapsed().as_secs());
    if is_https_verify_backoff_active(backoff_elapsed) {
        logging!(
            debug,
            Type::Lightweight,
            "[HTTPS验真/{reason}] 处于全量验真受阻退避窗口内（距上次全败 {}s < {}s），跳过本轮探测",
            backoff_elapsed.unwrap_or(0),
            HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS
        );
        return;
    }
    if !is_recent_flowing_traffic_active() && !is_physical_network_online(2).await {
        return;
    }

    // 验真与 `probe_metrics_group` 都会对 `PROXY__METRICS` 发起全组拨测。未串行化时
    // 两者可能背靠背各打一轮全量握手，且后写的一方会覆盖前一方的 alive / history。
    let delays = {
        let _probe_guard = METRICS_PROBE_LOCK.lock().await;
        match mihomo
            .delay_group("PROXY__METRICS", VERIFY_HTTPS_URL_PRIMARY, NODE_TEST_TIMEOUT_MS)
            .await
        {
            Ok(d) if !d.is_empty() => d,
            primary_res => {
                if let Err(ref e) = primary_res {
                    logging!(
                        debug,
                        Type::Lightweight,
                        "[HTTPS验真/{reason}] 主验证 URL 探测失败({e})，尝试备用验证 URL"
                    );
                }
                match mihomo
                    .delay_group("PROXY__METRICS", VERIFY_HTTPS_URL_FALLBACK, NODE_TEST_TIMEOUT_MS)
                    .await
                {
                    Ok(d) => d,
                    Err(e) => {
                        *LAST_HTTPS_VERIFY_ALL_FAILED.lock() = Some(Instant::now());
                        logging!(
                            debug,
                            Type::Lightweight,
                            "[HTTPS验真/{reason}] 主/备验证 URL 均无响应({e})，记录退避并跳过本轮失败累计"
                        );
                        return;
                    }
                }
            }
        }
    };

    let passed_set: HashSet<String> = delays
        .iter()
        .filter(|(_, d)| (NODE_DELAY_MIN_MS..NODE_DELAY_MAX_MS).contains(d))
        .map(|(name, _)| name.clone())
        .collect();
    if passed_set.is_empty() {
        *LAST_HTTPS_VERIFY_ALL_FAILED.lock() = Some(Instant::now());
        logging!(
            debug,
            Type::Lightweight,
            "[HTTPS验真/{reason}] 本批 0 个节点通过真 HTTPS 验真（疑离线或校验受阻），记录退避并跳过本轮失败累计"
        );
        return;
    }
    *LAST_HTTPS_VERIFY_ALL_FAILED.lock() = None;

    let passed_names: Vec<String> = passed_set.iter().cloned().collect();
    let failed_names: Vec<String> = candidate_nodes
        .iter()
        .filter(|n| {
            !passed_set.contains(n.as_str()) && !is_dummy_node(n) && !crate::utils::node::is_confirmed_fake_node(n)
        })
        .cloned()
        .collect();

    let outcome = crate::utils::node::record_https_verify_results(&passed_names, &failed_names);
    if outcome.newly_verified > 0 || outcome.newly_confirmed_fake > 0 {
        logging!(
            info,
            Type::Lightweight,
            "[HTTPS验真/{reason}] 通过 {} 个(新验真 {})，未通过 {} 个，新确认软隔离(24h) {} 个",
            passed_names.len(),
            outcome.newly_verified,
            failed_names.len(),
            outcome.newly_confirmed_fake
        );
    }
    if outcome.newly_confirmed_fake > 0 && !crate::module::lightweight::is_in_lightweight_mode() {
        crate::core::handle::Handle::refresh_proxies();
    }
}

/// 服务器级失败登记表：server → 最近一次被判定失败的时刻。
static FAILED_SERVERS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn failed_servers() -> &'static Mutex<HashMap<String, Instant>> {
    FAILED_SERVERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 记录一次服务器级失败。活跃节点被判死（且该判定基于**有效测速数据**）时调用。
fn record_server_failure(server: &str) {
    if server.is_empty() {
        return;
    }
    let mut map = failed_servers().lock().unwrap_or_else(|e| {
        logging!(warn, Type::Lightweight, "[后台监测] 失败服务器登记表锁被污染，恢复继续");
        e.into_inner()
    });
    let now = Instant::now();
    // 顺手清理过期条目，避免表无界增长
    map.retain(|_, t| now.duration_since(*t).as_secs() < FAILED_SERVER_COOLDOWN_SECS);
    map.insert(server.to_string(), now);
}

/// 取出仍在冷却期内的失败服务器集合（顺带清理过期项）。
fn snapshot_failed_servers() -> HashSet<String> {
    let mut map = failed_servers().lock().unwrap_or_else(|e| {
        logging!(warn, Type::Lightweight, "[后台监测] 失败服务器登记表锁被污染，恢复继续");
        e.into_inner()
    });
    let now = Instant::now();
    map.retain(|_, t| now.duration_since(*t).as_secs() < FAILED_SERVER_COOLDOWN_SECS);
    map.keys().cloned().collect()
}

/// 读取运行时配置，构建「节点名 → 服务器地址」映射。
///
/// mihomo 的 `/proxies` API **不暴露 server 字段**（只有 name/type/history/alive 等），
/// 故直接解析生成的 `clash-mini.yaml` 的 `proxies` 段（该段为 enhance 流水线合并后的
/// 全量内联节点，含 server）。解析失败返回空表 —— 此时「失败服务器避让」自动退化关闭，
/// 不影响选点正确性。
async fn load_node_server_map() -> HashMap<String, String> {
    #[derive(Deserialize)]
    struct ProxyEntry {
        name: String,
        #[serde(default)]
        server: Option<String>,
    }
    #[derive(Deserialize)]
    struct ConfigSlice {
        #[serde(default)]
        proxies: Vec<ProxyEntry>,
    }

    let Ok(dir) = crate::utils::dirs::app_home_dir() else {
        return HashMap::new();
    };
    let path = dir.join(crate::constants::files::RUNTIME_CONFIG);
    let Ok(text) = tokio::fs::read_to_string(&path).await else {
        return HashMap::new();
    };
    match serde_yaml_ng::from_str::<ConfigSlice>(&text) {
        Ok(slice) => slice
            .proxies
            .into_iter()
            .filter_map(|e| e.server.filter(|s| !s.is_empty()).map(|s| (e.name, s)))
            .collect(),
        Err(e) => {
            logging!(
                debug,
                Type::Lightweight,
                "[后台监测] 解析运行时配置构建节点→服务器映射失败（失败服务器避让本次关闭）: {e}"
            );
            HashMap::new()
        }
    }
}

fn secondary_test_url(primary_url: &str) -> &'static str {
    if primary_url.trim().eq_ignore_ascii_case(FALLBACK_TEST_URL) {
        DEFAULT_TEST_URL
    } else {
        FALLBACK_TEST_URL
    }
}

const fn is_valid_probe_delay(delay: u32) -> bool {
    delay >= NODE_DELAY_MIN_MS && delay < NODE_DELAY_MAX_MS
}

/// 构造后台体检 `probe_metrics_group` 同步到前端展示的 `(节点名, 延迟)` 列表（BUG-387）。
///
/// - `!primary_url_distorted`：即便 `probed_delays` 为空（全员超时 `Ok({})`）也为全部可见真实节点生成 `0 (Timeout)`，
///   避免活跃节点已断流时前端顶栏与列表残留断流前的假绿延迟；
/// - `primary_url_distorted == true`：主测速 URL 全局失真时仅输出备用 URL 真正测得的有效延迟，
///   严禁把未响应节点批量漆成 `0 (Timeout)` 写入前端 30 分钟长缓存。
fn build_probe_ui_display_with<F>(
    all_nodes: &[String],
    probed_delays: &HashMap<String, u32>,
    primary_url_distorted: bool,
    is_fake_node: F,
) -> Vec<(String, u32)>
where
    F: Fn(&str) -> bool,
{
    all_nodes
        .iter()
        .filter(|n| !is_dummy_node(n) && !is_fake_node(n))
        .filter_map(|n| {
            let delay = probed_delays.get(n).copied();
            if primary_url_distorted {
                delay.filter(|&d| is_valid_probe_delay(d)).map(|d| (n.clone(), d))
            } else {
                Some((n.clone(), delay.unwrap_or(0)))
            }
        })
        .collect()
}

fn build_probe_ui_display(
    all_nodes: &[String],
    probed_delays: &HashMap<String, u32>,
    primary_url_distorted: bool,
) -> Vec<(String, u32)> {
    build_probe_ui_display_with(
        all_nodes,
        probed_delays,
        primary_url_distorted,
        crate::utils::node::is_confirmed_fake_node,
    )
}

/// 基于本轮已推向 UI 的展示批与当前活跃节点，推进 `LAST_UI_ACTIVE_SYNC` 状态（BUG-387）。
///
/// - 当批内包含活跃节点且延迟有效时，记 `reported_alive = true` 并更新 `history_time`；
/// - 当批内显示活跃节点为 `0 / 超时` 时，强制 `reported_alive = false, history_time = None`，
///   绝不用断流前的旧成功 `history.last().time` 污染去重键；
/// - 当批内未覆盖活跃节点（如子集测速或失真残缺探测）时，原样保留先前状态。
fn compute_next_ui_active_sync(
    active_node: &str,
    display: &[(String, u32)],
    active_hist_time: Option<String>,
    prev: Option<&UiActiveSyncState>,
) -> Option<UiActiveSyncState> {
    let Some((_, delay)) = display.iter().find(|(n, _)| n == active_node) else {
        return prev.cloned();
    };
    let reported_alive = is_valid_probe_delay(*delay);
    let history_time = if reported_alive {
        active_hist_time.or_else(|| {
            prev.filter(|s| s.node_name == active_node && s.reported_alive)
                .and_then(|s| s.history_time.clone())
        })
    } else {
        None
    };
    Some(UiActiveSyncState {
        node_name: active_node.to_string(),
        history_time,
        reported_alive,
    })
}

/// 判断免拨测轮在活跃节点 `NodeStatus::Alive` 时是否需要向前端补发可用延迟（BUG-370 / BUG-387）。
fn should_sync_alive_ui(
    health_not_ok: bool,
    last_sync: Option<&UiActiveSyncState>,
    active_node: &str,
    active_hist_time: Option<&str>,
) -> bool {
    if health_not_ok {
        return true;
    }
    let Some(sync) = last_sync else {
        return true;
    };
    if sync.node_name != active_node || !sync.reported_alive {
        return true;
    }
    active_hist_time.is_some_and(|t| sync.history_time.as_deref() != Some(t))
}

fn should_recheck_with_fallback(primary_delays: &HashMap<String, u32>, expected_nodes: &[String]) -> bool {
    if !expected_nodes.is_empty() {
        !expected_nodes
            .iter()
            .any(|name| primary_delays.get(name).copied().is_some_and(is_valid_probe_delay))
    } else {
        !primary_delays.iter().any(|(name, &d)| {
            !is_dummy_node(name) && !crate::utils::node::is_confirmed_fake_node(name) && is_valid_probe_delay(d)
        })
    }
}

/// 仅用于主 URL 失败/失真时的备用 URL 复核门控（BUG-401）：
/// 当开启 `allow_single_target_skip` 且期望目标仅 1 个节点（点击单点延迟徽章或首次体检失败）时，
/// 若主测速地址本轮已有至少 `SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS` (3) 个真实节点正常响应，
/// 说明主测速地址与全局代理通路均未失真，该单节点属单点故障，直接跳过备用 URL 第二轮全组复核。
/// 多节点子集与连续第 2 次判死前确认仍强制走备用 URL 复核，完整保留防失真能力。
fn should_recheck_primary_with_fallback_with<F>(
    primary_delays: &HashMap<String, u32>,
    expected_nodes: &[String],
    allow_single_target_skip: bool,
    is_fake_node: F,
) -> bool
where
    F: Fn(&str) -> bool,
{
    if !should_recheck_with_fallback(primary_delays, expected_nodes) {
        return false;
    }
    if allow_single_target_skip && expected_nodes.len() == 1 {
        let healthy_real_peers = primary_delays
            .iter()
            .filter(|(name, d)| {
                !name.eq_ignore_ascii_case("DIRECT")
                    && !name.eq_ignore_ascii_case("REJECT")
                    && !is_dummy_node(name)
                    && !is_fake_node(name)
                    && is_valid_probe_delay(**d)
            })
            .count();
        if healthy_real_peers >= SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS {
            return false;
        }
    }
    true
}

fn should_recheck_primary_with_fallback(
    primary_delays: &HashMap<String, u32>,
    expected_nodes: &[String],
    allow_single_target_skip: bool,
) -> bool {
    should_recheck_primary_with_fallback_with(
        primary_delays,
        expected_nodes,
        allow_single_target_skip,
        crate::utils::node::is_confirmed_fake_node,
    )
}

fn merge_probe_delays(mut primary: HashMap<String, u32>, fallback: HashMap<String, u32>) -> HashMap<String, u32> {
    for (name, fb_delay) in fallback {
        if is_valid_probe_delay(fb_delay) {
            primary.insert(name, fb_delay);
        } else if !primary.get(&name).copied().is_some_and(is_valid_probe_delay) {
            primary.entry(name).or_insert(fb_delay);
        }
    }
    primary
}

/// 复用最近一次完整 `probe_metrics_group` 的全量实时延迟快照。
///
/// 按完整拨测快照复用可避开 Mihomo 对死节点「只写 alive=false、不追加 history」导致的陈旧伪判定。
/// - 显式单点测速（`is_frontend_subset && subset_len <= 1`）严禁复用（§5.2 契约 4）；
/// - 故障自愈在 `< SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS` 内可复用刚刚完成的确认拨测结果，
///   并在尚未上屏时保留 `LiveProbe` 语义；
/// - 前端批量子集测速（`subset_len > 1`）在 `< METRICS_REUSE_MAX_AGE_SECS` 内复用，标记 `FreshMetricsCache`；
/// - 快照若对当前 `expected_nodes` 全灭，一律禁止复用（BUG-370 门控方向修正，Sync-18）。
fn try_reuse_probe_snapshot(
    snapshot: &mut Option<MetricsProbeSnapshot>,
    now: Instant,
    is_frontend_subset: bool,
    subset_len: usize,
    allow_self_heal_reuse: bool,
    expected_nodes: &[String],
) -> Option<(HashMap<String, u32>, DelayResultSource)> {
    if is_frontend_subset && subset_len <= 1 {
        return None;
    }
    let snap = snapshot.as_mut()?;
    if snap.delays.is_empty() {
        return None;
    }
    // 【门控方向修正（Sync-18）】原条件为 `!snap.dual_url_verified && 全灭`，等于给
    // 「经过主/备双地址复核」的快照开了后门：备用地址整体可用、但目标节点无有效延迟的
    // 残缺快照会在 10s 内直接成为自愈选点输入。全灭与是否复核无关，一律禁止复用。
    if should_recheck_with_fallback(&snap.delays, expected_nodes) {
        return None;
    }
    let age = now.saturating_duration_since(snap.completed_at);
    if allow_self_heal_reuse && age < Duration::from_secs(SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS) {
        let source = if !snap.reported_live_to_ui {
            snap.reported_live_to_ui = true;
            DelayResultSource::LiveProbe
        } else {
            DelayResultSource::FreshMetricsCache
        };
        return Some((snap.delays.clone(), source));
    }
    if is_frontend_subset && subset_len > 1 && age < Duration::from_secs(METRICS_REUSE_MAX_AGE_SECS) {
        return Some((snap.delays.clone(), DelayResultSource::FreshMetricsCache));
    }
    None
}

/// 带「测速失真复核」的整组拨测（含全局互斥与短窗口并发合并）。
///
/// 先用主测速 URL 拨测；若整体失败，或主 URL 仅部分外区/假节点响应而目标节点均无有效延迟（部分失真），
/// 立即用备用测速 URL 复核一次并合并有效延迟（当 `allow_single_target_skip == true` 且仅期望单节点、
/// 同组已有 >= 3 个真实节点通过主 URL 响应时跳过备用 URL 复核）。两次均整体失败才认定「测速链路不可用」。
///
/// 返回 `Some(outcome)` = 测速链路可用（`outcome.delays` 为本次测得的节点延迟）；
/// `None` = 主/备地址都整体失败，属测速链路问题，**不可据此判定节点已死**。
/// `outcome.primary_url_distorted` 为 true 时代表「主测速地址失真、节点侧正常」，
/// 调用方必须据此跳过健康裁决（见 `evaluate_failover`）。
async fn probe_metrics_group(
    mihomo: &tauri_plugin_mihomo::Mihomo,
    label: &str,
    allow_coalesce: bool,
    expected_nodes: &[String],
    allow_single_target_skip: bool,
) -> Option<MetricsProbeOutcome> {
    let _probe_guard = METRICS_PROBE_LOCK.lock().await;
    if allow_coalesce {
        let cached = {
            let guard = LAST_METRICS_PROBE.lock();
            guard.as_ref().and_then(|snap| {
                if !snap.delays.is_empty()
                    && snap.completed_at.elapsed() < Duration::from_millis(PROBE_COALESCE_WINDOW_MS)
                    && !should_recheck_with_fallback(&snap.delays, expected_nodes)
                {
                    Some(snap.delays.clone())
                } else {
                    None
                }
            })
        };
        if let Some(delays) = cached {
            logging!(
                info,
                Type::Lightweight,
                "[后台监测] [{label}] 复用刚结束的并发测量组实时拨测结果（{} 个存活节点），抑制重叠拨测风暴",
                delays.len()
            );
            return Some(MetricsProbeOutcome {
                delays,
                primary_url_distorted: false,
            });
        }
    }

    let record_snapshot = |delays: HashMap<String, u32>| -> HashMap<String, u32> {
        if delays.is_empty() {
            *LAST_METRICS_PROBE.lock() = None;
        } else {
            *LAST_METRICS_PROBE.lock() = Some(MetricsProbeSnapshot {
                completed_at: Instant::now(),
                delays: delays.clone(),
                reported_live_to_ui: false,
            });
        }
        delays
    };

    let test_url = get_test_url().await;
    let fallback_url = secondary_test_url(&test_url);
    match mihomo
        .delay_group("PROXY__METRICS", &test_url, NODE_TEST_TIMEOUT_MS)
        .await
    {
        Ok(primary_delays) => {
            if !should_recheck_primary_with_fallback(&primary_delays, expected_nodes, allow_single_target_skip) {
                return Some(MetricsProbeOutcome {
                    delays: record_snapshot(primary_delays),
                    primary_url_distorted: false,
                });
            }
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] [{label}] 主测速地址下目标节点均未取得有效延迟（总响应节点={}），改用备用地址复核: {fallback_url}",
                primary_delays.len()
            );
            match mihomo
                .delay_group("PROXY__METRICS", fallback_url, NODE_TEST_TIMEOUT_MS)
                .await
            {
                Ok(fallback_delays) => {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] [{label}] 备用测速地址复核完成（响应节点={}），合并主备有效延迟",
                        fallback_delays.len()
                    );
                    let merged = merge_probe_delays(primary_delays, fallback_delays);
                    return Some(MetricsProbeOutcome {
                        delays: record_snapshot(merged),
                        primary_url_distorted: false,
                    });
                }
                Err(e) => {
                    logging!(
                        warn,
                        Type::Lightweight,
                        "[后台监测] [{label}] 备用测速地址复核整体失败（{e}），保留主地址测速结果"
                    );
                    if primary_delays.is_empty() {
                        *LAST_METRICS_PROBE.lock() = None;
                        return None;
                    }
                    return Some(MetricsProbeOutcome {
                        delays: record_snapshot(primary_delays),
                        primary_url_distorted: false,
                    });
                }
            }
        }
        Err(e) => {
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] [{label}] 主测速地址整体失败（{e}），改用备用地址复核: {fallback_url}"
            );
        }
    }

    // 主 URL 整体失败 → 可能是机场劫持/不兼容该测试地址（测速失真），换备用 URL 复核
    match mihomo
        .delay_group("PROXY__METRICS", fallback_url, NODE_TEST_TIMEOUT_MS)
        .await
    {
        Ok(d) => {
            logging!(
                info,
                Type::Lightweight,
                "[后台监测] [{label}] 备用测速地址可用 → 判定为主测速地址失真、节点侧正常，本轮不判死、不触发自愈"
            );
            Some(MetricsProbeOutcome {
                delays: record_snapshot(d),
                primary_url_distorted: true,
            })
        }
        Err(e) => {
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] [{label}] 主/备测速地址均整体失败（{e}）→ 判定为测速链路不可用（不视为节点已死）"
            );
            *LAST_METRICS_PROBE.lock() = None;
            None
        }
    }
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
const FAILOVER_CONFIRM_FAILS: u32 = 2;

/// 内核 API 通道连续异常达到该次数后，才允许进入统一自愈裁决（非节点侧故障，见 §5.1）
const API_ERROR_HEAL_THRESHOLD: u32 = 3;

/// 正常健康检测间隔（秒）
const NORMAL_CHECK_INTERVAL_SECS: u64 = 15;

/// 重试模式健康检测间隔（秒）
const RETRY_CHECK_INTERVAL_SECS: u64 = 3;

/// 离线时主循环间隔（秒）：与离线网络探测间隔对齐，确保 5 秒快速探测不被 15 秒主循环限制
const OFFLINE_CHECK_INTERVAL_SECS: u64 = 5;

/// 后台监测线程心跳日志周期（循环次数）
///
/// 【设计依据】
/// - 正常模式 15s/周期，20 个周期 ≈ 5 分钟
/// - 与 UI 线程心跳探针（5 秒）的"5 倍"原则保持一致：5s × 60 = 5min
/// - 远小于 v2.6.5 事件中的 4h22m 空白期，能快速区分"线程已死" vs "正常运行无事件"
/// - debug 级别，不影响生产日志体积（默认 Info 级别不显示）
///
/// 【设计盲区修复】
/// 此前 monitor.rs 主循环只在状态变化时打日志（节点变化/网络变化/失败告警），
/// 导致正常运行时连续几小时无日志输出，与"线程已死"无法区分。
/// 加周期性心跳后，排障时启用 RUST_LOG=debug 即可看到心跳，立刻判定线程状态。
const MONITOR_HEARTBEAT_CYCLE_COUNT: u64 = 20;

/// 从当前 Verge 配置中读取测速 URL，多处复用避免重复代码
async fn get_test_url() -> String {
    let verge = Config::verge().await.latest_arc();
    verge
        .default_latency_test
        .as_deref()
        .unwrap_or(DEFAULT_TEST_URL)
        .to_string()
}

/// 将测速 URL 解析为 `lookup_host` 可用的 `"host:port"` 字符串。
/// 【关键修正】对 IPv6 地址必须使用 `[addr]:port` 格式，裸拼接会导致解析失败。
fn resolve_probe_target(test_url: &str) -> String {
    match url::Url::parse(test_url) {
        Ok(parsed_url) => {
            let port = parsed_url.port_or_known_default().unwrap_or(443);
            match parsed_url.host() {
                Some(url::Host::Ipv6(addr)) => format!("[{}]:{}", addr, port),
                Some(url::Host::Ipv4(addr)) => format!("{}:{}", addr, port),
                Some(url::Host::Domain(domain)) => format!("{}:{}", domain, port),
                None => "cp.cloudflare.com:443".to_string(),
            }
        }
        Err(_) => "cp.cloudflare.com:443".to_string(),
    }
}

/// 物理网络连通性探测（BUG-317 / BUG-345 / BUG-403）。
/// 优先单路直连主公网高可用 DNS `223.5.5.5:53`（限时 `PRIMARY_NET_PROBE_TIMEOUT_MS` = 250ms），
/// 连通即毫秒级返回，日常减少 2/3 公网握手；仅当主路超时或异常时，才并发直连备用两路
/// (`119.29.29.29:53` / `1.1.1.1:53`，单路 600ms)；若 3 路均失败（如局域网封禁出站 53 端口），
/// 仍在总超时预算内回退至域名 DNS 查询兜底，杜绝误判离线。
async fn is_physical_network_online(timeout_secs: u64) -> bool {
    let dur = Duration::from_secs(timeout_secs.max(2));
    let check = async {
        if let Ok(Ok(_)) = tokio::time::timeout(
            Duration::from_millis(PRIMARY_NET_PROBE_TIMEOUT_MS),
            tokio::net::TcpStream::connect("223.5.5.5:53"),
        )
        .await
        {
            return true;
        }

        let backup_targets = ["119.29.29.29:53", "1.1.1.1:53"];
        let probes = backup_targets.into_iter().map(|addr| {
            Box::pin(async move {
                match tokio::time::timeout(Duration::from_millis(600), tokio::net::TcpStream::connect(addr)).await {
                    Ok(Ok(stream)) => Ok(stream),
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(std::io::Error::from(std::io::ErrorKind::TimedOut)),
                }
            })
        });
        if futures::future::select_ok(probes).await.is_ok() {
            return true;
        }

        // 兜底：若特定局域网封禁出站 53 端口，在剩余超时预算内回退至域名 DNS 查询
        let test_url = get_test_url().await;
        let host_port = resolve_probe_target(&test_url);
        tokio::net::lookup_host(host_port).await.is_ok()
    };
    tokio::time::timeout(dur, check).await.unwrap_or(false)
}

/// 快速探测 mihomo 内核 API 是否可达。
///
/// 自愈连续失败后决定是否重启内核前调用：若 API 本身可达，说明失败原因是
/// 节点侧不可达（如国际出口中断），重启内核无益且会造成短暂断流；
/// 仅当 API 确实无响应时才重启内核以清除命名管道卡死状态。
async fn is_clash_api_alive() -> bool {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    tokio::time::timeout(Duration::from_secs(3), mihomo.get_base_config())
        .await
        .map(|res| res.is_ok())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    pub filter_text: String,
}

// 识别广告/假节点的实现已迁移到 crate::utils::node::is_dummy_node
// 以便 enhance/mod.rs 和 lifecycle.rs 共享同一份判定逻辑，避免遗漏
pub use crate::utils::node::is_dummy_node;
use crate::utils::node::match_node_filter;

/// 从 `proxy_head_state.json` 一次性读取并解析 FilterConfig 和 sort_type
pub(crate) async fn get_filter_and_sort_config(profile_uid: &str) -> (FilterConfig, Option<i32>) {
    let path = match crate::utils::dirs::app_home_dir() {
        Ok(dir) => dir.join("proxy_head_state.json"),
        Err(_) => return (FilterConfig::default(), None),
    };
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return (FilterConfig::default(), None),
    };
    let json_val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            logging!(
                warn,
                Type::Config,
                "[后台监测] proxy_head_state.json 解析失败，退化为默认（全量池）: {e}"
            );
            return (FilterConfig::default(), None);
        }
    };

    let group_state = &json_val[profile_uid]["PROXY"];
    if group_state.is_null() {
        return (FilterConfig::default(), None);
    }

    let filter_config = FilterConfig {
        filter_text: group_state["filterText"].as_str().unwrap_or("").to_string(),
    };
    let sort_type = group_state["sortType"].as_i64().map(|v| v as i32);
    (filter_config, sort_type)
}

/// 获取当前活动 Profile 的 UID
pub(crate) async fn get_current_profile_uid() -> Option<String> {
    let profiles = Config::profiles().await;
    profiles.data_arc().current.as_ref().map(|s| s.to_string())
}

/// 等待 Clash 内核 API 及代理组节点列表填充完毕
/// 如果期间检测到 Profile 切换通知，立即返回 false，让调用方重新用新 Profile 操作
pub(crate) async fn wait_for_clash_ready() -> bool {
    let start_time = Instant::now();
    // 记录进入等待时的 Profile：PROFILE_SWITCH_NOTIFY 改为 notify_waiters() 后不再存储许可，
    // 通知可能在等待间隙丢失。故各阶段以「Profile uid 是否变化」为权威判据，通知仅作加速。
    let initial_uid = get_current_profile_uid().await;

    // 阶段 1：等待内核 API 接口响应
    // 【注意】此处 clone mihomo 后跨 await 使用是安全的：
    // - MihomoManager 是全局单例，clone 的是 Arc，实例不会被替换
    // - 若后续改为可热替换，则需每次 await 后重新获取
    while start_time.elapsed().as_secs() < 30 {
        if crate::core::handle::Handle::global().is_exiting() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 1 中断：应用正在退出");
            return false;
        }
        if is_profile_switched(&initial_uid).await {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 1 中断：检测到 Profile 切换");
            return false;
        }
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        if mihomo.get_base_config().await.is_ok() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 1 完成：内核 API 已就绪");
            break;
        }
        tokio::select! {
            _ = sleep(Duration::from_millis(200)) => {}
            _ = PROFILE_SWITCH_NOTIFY.notified() => {
                logging!(info, Type::Lightweight, "[后台监测] 阶段 1 中断：检测到 Profile 切换");
                return false;
            }
        }
    }

    if start_time.elapsed().as_secs() >= 30 {
        logging!(warn, Type::Lightweight, "[后台监测] 阶段 1 失败：等待内核 API 响应超时");
        return false;
    }

    // 阶段 2：等待代理节点列表填充
    let start_time_2 = Instant::now();
    while start_time_2.elapsed().as_secs() < 20 {
        if crate::core::handle::Handle::global().is_exiting() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 2 中断：应用正在退出");
            return false;
        }
        if is_profile_switched(&initial_uid).await {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 2 中断：检测到 Profile 切换");
            return false;
        }
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        if let Ok(group_info) = mihomo.get_group_by_name("PROXY").await {
            if let Some(all) = group_info.all {
                if !all.is_empty() {
                    logging!(info, Type::Lightweight, "[后台监测] 阶段 2 完成：代理节点列表已填充");
                    // 阶段 3：等待物理网络探测通过
                    // 开机启动场景下内核 API 与节点列表就绪早于物理网络（DHCP/DNS 尚未完成），
                    // 若此时立即测速必然 all timeout。此处阻塞等待，从根源消除该现象。
                    // 【注意】本探测本身是一次 DNS 解析（lookup_host 测速网址域名），因此它失败
                    // 并不能证明链路不通，更不能据此放行——DNS 链未就绪时它恰好会一直失败。
                    // 故无论成功还是超时，都继续进入阶段 4，由内核侧确认解析链是否可用。
                    let start_time_3 = Instant::now();
                    let mut net_probe_ok = false;
                    while start_time_3.elapsed().as_secs() < 15 {
                        if crate::core::handle::Handle::global().is_exiting() {
                            logging!(info, Type::Lightweight, "[后台监测] 阶段 3 中断：应用正在退出");
                            return false;
                        }
                        if is_profile_switched(&initial_uid).await {
                            logging!(info, Type::Lightweight, "[后台监测] 阶段 3 中断：检测到 Profile 切换");
                            return false;
                        }
                        if is_physical_network_online(2).await {
                            logging!(info, Type::Lightweight, "[后台监测] 阶段 3 完成：物理网络已连通");
                            net_probe_ok = true;
                            break;
                        }
                        tokio::select! {
                            _ = sleep(Duration::from_secs(1)) => {}
                            _ = PROFILE_SWITCH_NOTIFY.notified() => {
                                logging!(info, Type::Lightweight, "[后台监测] 阶段 3 中断：检测到 Profile 切换");
                                return false;
                            }
                        }
                    }
                    if !net_probe_ok {
                        logging!(
                            warn,
                            Type::Lightweight,
                            "[后台监测] 阶段 3 超时：物理网络探测未通过，继续进入阶段 4 兜底"
                        );
                    }
                    // 阶段 4：等待 mihomo DNS 解析链就绪（实测解析一个稳定域名，通后才放行测速）。
                    // 阶段 3 只反映系统侧探测结果，仍需内核侧确认解析链可用，
                    // 否则冷启动首轮测速会全 timeout。
                    return wait_dns_resolver_ready(&initial_uid).await;
                }
            }
        }
        tokio::select! {
            _ = sleep(Duration::from_millis(500)) => {}
            _ = PROFILE_SWITCH_NOTIFY.notified() => {
                logging!(info, Type::Lightweight, "[后台监测] 阶段 2 中断：检测到 Profile 切换");
                return false;
            }
        }
    }

    logging!(
        warn,
        Type::Lightweight,
        "[后台监测] 阶段 2 失败：等待内核加载节点列表超时"
    );
    false
}

/// 判断当前 Profile 是否已从进入等待时发生切换（以状态对比为准，通知仅作加速）。
/// PROFILE_SWITCH_NOTIFY 自 v2.8.8 起改为 notify_waiters()，不再存储许可，
/// 通知可能在等待间隙丢失；因此各等待阶段以 uid 是否变化作为权威判据，
/// 避免丢通知导致等待方停滞到自身超时（阶段 1/2/3 最长 30/20/15 秒）。
async fn is_profile_switched(initial_uid: &Option<String>) -> bool {
    get_current_profile_uid().await != *initial_uid
}

/// 阶段 4：等待 mihomo DNS 解析链就绪（实测解析一个稳定域名，通后才放行测速）。
/// 【冷启动 DNS 鸡生蛋防护】订阅的 DNS 若全为加密上游（DoH/DoT），其服务器域名
/// 需先经 default-nameserver 解析，而后者冷启动窗口内可能因 TUN 接口绑定延迟、
/// 系统 DNS 被 dns-hijack any:53 劫持回环、8.8.8.8 大陆直连不可达而全断 →
/// 节点域名解析不出 → 首轮测速全 timeout（v2.8.5 实测：冷启动必全 timeout，
/// 约一分钟后自愈）。此处实测解析，避免无效测速与全 timeout 的 UI 展示。
/// 超时上限 15 秒：超过后放行，由下游自愈逻辑兜底，避免永久阻塞。
async fn wait_dns_resolver_ready(initial_uid: &Option<String>) -> bool {
    let start_time = Instant::now();
    while start_time.elapsed().as_secs() < 15 {
        if crate::core::handle::Handle::global().is_exiting() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 4 中断：应用正在退出");
            return false;
        }
        if is_profile_switched(initial_uid).await {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 4 中断：检测到 Profile 切换");
            return false;
        }
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        match mihomo.dns_query_ready("www.baidu.com").await {
            Ok(true) => {
                logging!(info, Type::Lightweight, "[后台监测] 阶段 4 完成：DNS 解析链已就绪");
                return true;
            }
            Ok(false) => {}
            Err(err) => {
                // 端点不可用等硬错误：继续空等没有意义，直接放行由下游兜底
                logging!(warn, Type::Lightweight, "[后台监测] 阶段 4 放弃等待（{err}），放行测速");
                return true;
            }
        }
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {}
            _ = PROFILE_SWITCH_NOTIFY.notified() => {
                logging!(info, Type::Lightweight, "[后台监测] 阶段 4 中断：检测到 Profile 切换");
                return false;
            }
        }
    }
    logging!(
        warn,
        Type::Lightweight,
        "[后台监测] 阶段 4 超时：DNS 解析链未就绪，放行测速（由下游兜底）"
    );
    true
}

/// 恢复当前活动 Profile 配置文件中所保存的上次选定的 PROXY 组节点
// 【核心架构约定】Mini 只有 PROXY 一个有效代理组（所有上游代理组已合并），
// 此函数只处理 PROXY 组，不遍历多组（上游 Clash Verge Rev 的多组遍历逻辑已移除）。
pub(crate) async fn restore_profile_selected_nodes(profile_uid: &str) -> anyhow::Result<()> {
    let profiles = Config::profiles().await;
    let latest = profiles.latest_arc();
    let item = latest.get_item(profile_uid)?;
    if let Some(selected) = &item.selected {
        // Mini 单组架构：只恢复 PROXY 组
        if let Some(entry) = selected.iter().find(|s| s.name.as_deref() == Some("PROXY")) {
            if let (Some(_group), Some(node)) = (&entry.name, &entry.now) {
                if !node.is_empty() {
                    // 【治本修复②】恢复前校验节点是否在当前 filterText 子集内，
                    // 越界（被污染的持久选择）或伪节点不还原，交由后续 auto-select 在子集内重选，
                    // 避免"轻量唤醒后前端显示节点 ≠ 内核实际选路"的污染复现。
                    let filter_text = get_filter_and_sort_config(profile_uid)
                        .await
                        .0
                        .filter_text
                        .trim()
                        .to_string();
                    if is_dummy_node(node)
                        || crate::utils::node::is_confirmed_fake_node(node)
                        || !match_node_filter(node, &filter_text)
                    {
                        logging!(
                            warn,
                            Type::Lightweight,
                            "[后台监测] 跳过恢复越界/伪节点 {}（不在过滤范围「{}」内，疑似被污染的持久选择），交由自动选点重选",
                            node,
                            filter_text
                        );
                        return Ok(());
                    }
                    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
                    logging!(info, Type::Lightweight, "[后台监测] 恢复 PROXY 组选择节点: {}", node);
                    let _ = mihomo.select_node_for_group("PROXY", node).await;
                }
            }
        }
    }
    Ok(())
}

/// 单节点健康状态（由测量组维护的 history/alive 判定，TUN 下可靠）。
/// S4 起由二值扩为三态：`Suspect` 吸收单次波动，仅 `Dead` 构成自愈证据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeStatus {
    /// 测量组尚未测过该节点（数据缺失），不动作、等待测量
    Unknown,
    /// 疑似抖动：`alive=false` 但上次成功仍处于 `SUSPECT_WINDOW_SECS` 内，不累计失败或自愈
    Suspect,
    /// 节点已确认不可用（测速超时/失败/假节点，且无近期成功记录）
    Dead,
    /// 节点健康，附带真实延迟
    Alive(u32),
}

/// 单节点健康三态分类（时间戳不可解析时保守判 `Dead`，防长期坏节点卡在门外）。
fn classify_node_status(alive: bool, last_delay: Option<u32>, age_secs: Option<u64>) -> NodeStatus {
    let Some(delay) = last_delay else {
        return NodeStatus::Unknown;
    };
    let valid_delay = (NODE_DELAY_MIN_MS..NODE_DELAY_MAX_MS).contains(&delay);
    if alive && valid_delay {
        return NodeStatus::Alive(delay);
    }
    if !alive && valid_delay && age_secs.is_some_and(|age| age < SUSPECT_WINDOW_SECS) {
        return NodeStatus::Suspect;
    }
    NodeStatus::Dead
}

/// 自愈前门检（S4）：只有**确证已死**的原节点才允许自愈切点。
/// `Suspect` / `Alive` / `Unknown` 都说明原节点并未确证失效——自愈是兜底，不是择优，
/// 「别的节点更快」永远不构成切点理由（那属于 F1 手动选点）。
const fn should_proceed_with_self_heal(dead_status: NodeStatus) -> bool {
    matches!(dead_status, NodeStatus::Dead)
}

/// 自愈切换速率上限（S4，纯函数）。
/// `last_switch_age_secs` 为距上次实际切换的秒数；`recent_switch_count` 为
/// `AUTO_SWITCH_WINDOW_SECS` 滑动窗口内已发生的实际切换次数（由调用方裁剪）。
fn auto_switch_permitted(last_switch_age_secs: Option<u64>, recent_switch_count: usize) -> bool {
    if last_switch_age_secs.is_some_and(|age| age < MIN_AUTO_SWITCH_INTERVAL_SECS) {
        return false;
    }
    recent_switch_count < MAX_AUTO_SWITCHES_PER_WINDOW
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 故障转移裁决
pub enum FailoverVerdict {
    /// 当前活跃节点健康或数据缺失，无需动作
    NoAction,
    /// 活跃节点已死且子集内存在健康候选，应触发选点（故障转移）
    ShouldFailover,
    /// 活跃节点**仍可用但不可信**（假/广告节点、验真永久拉黑、真 HTTPS 验真失败），
    /// 需迁移到已验真健康节点（S4：与「节点已死」区分，不适用原节点确已死门检）
    ShouldMigrate,
    /// 子集内全部节点均不可用
    AllDead,
}

/// 自愈请求来源（S4）：决定「原节点确已死」门检是否适用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelfHealOrigin {
    /// 节点侧故障转移：原节点经连续 2 次独立体检确认已死，必须过门检
    ConfirmedDead,
    /// 不可信节点迁移：原节点仍可用但假/验真失败，门检不适用
    NodeMigration,
    /// 内核 API 通道故障：无原节点信息，沿用 BUG-318 重启/告警链路
    ApiChannelDown,
}

/// 门检适用范围（纯函数，便于单测锁定）：仅节点侧故障转移需要「原节点确已死」复核。
const fn self_heal_requires_dead_gate(origin: SelfHealOrigin) -> bool {
    matches!(origin, SelfHealOrigin::ConfirmedDead)
}

/// 切换速率限制适用范围（BUG-388，纯函数便于单测锁定）：
/// 已通过连续体检与「原节点确已死」复核证实的 `ConfirmedDead` 死链必须允许立即抢修，不能被 180s 速率上限锁死在死节点；
/// `NodeMigration`（原节点仍可用时的不可信迁移）与 `ApiChannelDown` 仍受速率上限约束。
const fn self_heal_subject_to_switch_rate_limit(origin: SelfHealOrigin) -> bool {
    !matches!(origin, SelfHealOrigin::ConfirmedDead)
}

/// 连续性优先（BUG-388，纯函数便于单测锁定）：活跃节点存在真实下行流量（`Flowing`）时，
/// 禁止以“未通过真 HTTPS 验真”为由执行后台切换或即时全组 HTTPS 验真探测。
const fn should_allow_https_migration(traffic_state: ActiveTrafficState) -> bool {
    !matches!(traffic_state, ActiveTrafficState::Flowing)
}

/// `ApiChannelDown` 恢复保活门检（BUG-388，纯函数便于单测锁定）：
/// 当 API 已可读且当前活跃真实节点健康（`Alive(_)`）时，遵循连续性优先直接保活现有出口，跳过全组择优切点。
const fn should_keep_alive_on_api_recovery(is_valid_real_node: bool, status: NodeStatus) -> bool {
    is_valid_real_node && matches!(status, NodeStatus::Alive(_))
}

/// 手动切点抢占判定（BUG-388，纯函数便于单测锁定）：比较选点开始与切换落盘前代际号。
const fn is_manual_switch_preempted(start_epoch: u64, current_epoch: u64) -> bool {
    current_epoch != start_epoch
}

/// 识别属于并发/抢占让路而非真实故障的选点中断错误（BUG-388）。
fn is_benign_auto_select_abort(err_str: &str) -> bool {
    matches!(err_str, "AUTO_SELECT_BUSY" | "AUTO_SELECT_MANUAL_PREEMPTED")
}

/// 后台监测所有自愈请求的统一裁决输入：仅 `SelfHealDecision::Allow` 允许触发自愈选点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelfHealTrigger {
    /// 主路径：连续独立体检确认活跃节点失效
    NodeConfirmedDead { consecutive_fails: u32 },
    /// 物理网络离线转在线：只代表链路恢复，不构成节点故障证据
    NetworkRecovered,
    /// 内核 API 通道连续异常；`api_alive_on_recheck` 为阈值触发后的 API 存活复核结果
    ApiUnreachable {
        api_error_count: u32,
        api_alive_on_recheck: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelfHealDecision {
    Allow,
    Wait,
}

/// 统一裁决器：节点侧需连续 `FAILOVER_CONFIRM_FAILS` 次失败；网络恢复恒 `Wait`；API 异常须经存活复核仍失联。
const fn decide_self_heal(trigger: SelfHealTrigger) -> SelfHealDecision {
    match trigger {
        SelfHealTrigger::NodeConfirmedDead { consecutive_fails } => {
            if consecutive_fails >= FAILOVER_CONFIRM_FAILS {
                SelfHealDecision::Allow
            } else {
                SelfHealDecision::Wait
            }
        }
        SelfHealTrigger::NetworkRecovered => SelfHealDecision::Wait,
        SelfHealTrigger::ApiUnreachable {
            api_error_count,
            api_alive_on_recheck,
        } => {
            if api_error_count >= API_ERROR_HEAL_THRESHOLD && !api_alive_on_recheck {
                SelfHealDecision::Allow
            } else {
                SelfHealDecision::Wait
            }
        }
    }
}

/// 依据测量组（PROXY__METRICS，url-test）维护的 history/alive 判定节点状态。
/// 这是唯一权威健康来源：由 Mihomo 内核周期性直接拨测，TUN 下可靠，
/// 彻底取代旧设计「后端用 delay_proxy_by_name 自测活跃节点」的不可靠路径。
fn node_status(proxy: &tauri_plugin_mihomo::models::Proxy) -> NodeStatus {
    // 测量组尚未测过该节点 → 数据缺失，不误判
    let last_delay = proxy.history.last().map(|h| h.delay as u32);
    // 用同一份 history 时间戳判断「刚失败」还是「长期不可用」（S4 三态）
    let age_secs = history_age(proxy).map(|age| age.as_secs());
    classify_node_status(proxy.alive, last_delay, age_secs)
}

/// 节点上次拨测距现在的时长（基于 history 中 RFC3339 时间戳）。
/// 无论拨测由谁触发（测量组自然周期 / 即时 delay_group）都会刷新该时间，
/// 因此它是「缓存数据是否新鲜」的权威依据。无 history 或时间解析失败返回 None。
fn history_age(proxy: &tauri_plugin_mihomo::models::Proxy) -> Option<Duration> {
    let time_str = &proxy.history.last()?.time;
    let measured_at = chrono::DateTime::parse_from_rfc3339(time_str).ok()?;
    let age_secs = chrono::Utc::now()
        .signed_duration_since(measured_at.with_timezone(&chrono::Utc))
        .num_seconds();
    // 时钟偏差导致时间在未来时按 0（刚测过）处理
    Some(Duration::from_secs(age_secs.max(0) as u64))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveTrafficState {
    /// 近一周期活跃节点连接有真实下行回包（`delta_download > 0`），物理证实远端存活
    Flowing,
    /// 近一周期活跃节点连接有上行发包但零下行回包（`delta_upload > 0 && delta_download == 0`），疑似黑洞停滞
    Stalled,
    /// 无活跃节点上下行流量变化（空闲、切换节点首轮建基线或连接查询不可用）
    Idle,
}

#[derive(Default)]
struct ActiveTrafficTracker {
    active_node: Option<String>,
    conn_bytes: HashMap<String, (u64, u64)>,
    last_stall_probe_at: Option<Instant>,
}

static ACTIVE_TRAFFIC_TRACKER: parking_lot::Mutex<Option<ActiveTrafficTracker>> = parking_lot::Mutex::new(None);

/// 纯状态归因：对比前后两轮落在当前活跃出口节点上的连接上下行字节，判定流量走势并返回新基线。
fn classify_active_traffic_delta(
    prev_node: Option<&str>,
    active_node: &str,
    prev_conns: &HashMap<String, (u64, u64)>,
    current_conns: &[(String, u64, u64)],
) -> (ActiveTrafficState, HashMap<String, (u64, u64)>) {
    let mut next_map = HashMap::with_capacity(current_conns.len());
    for (id, up, down) in current_conns {
        next_map.insert(id.clone(), (*up, *down));
    }

    if prev_node != Some(active_node) {
        return (ActiveTrafficState::Idle, next_map);
    }

    let mut delta_up: u64 = 0;
    let mut delta_down: u64 = 0;
    for (id, up, down) in current_conns {
        if let Some(&(prev_up, prev_down)) = prev_conns.get(id) {
            delta_up = delta_up.saturating_add(up.saturating_sub(prev_up));
            delta_down = delta_down.saturating_add(down.saturating_sub(prev_down));
        } else {
            delta_up = delta_up.saturating_add(*up);
            delta_down = delta_down.saturating_add(*down);
        }
    }

    let state = if delta_down > 0 {
        ActiveTrafficState::Flowing
    } else if delta_up > 0 {
        ActiveTrafficState::Stalled
    } else {
        ActiveTrafficState::Idle
    };
    (state, next_map)
}

fn update_active_traffic_state(
    tracker_slot: &mut Option<ActiveTrafficTracker>,
    active_node: &str,
    current_conns: &[(String, u64, u64)],
) -> (ActiveTrafficState, bool) {
    let tracker = tracker_slot.get_or_insert_with(ActiveTrafficTracker::default);
    let node_changed = tracker.active_node.as_deref() != Some(active_node);
    if node_changed {
        tracker.last_stall_probe_at = None;
    }
    let stall_cooldown_active = tracker
        .last_stall_probe_at
        .is_some_and(|t| t.elapsed() < Duration::from_secs(STALL_REPROBE_COOLDOWN_SECS));

    let (state, next_map) = classify_active_traffic_delta(
        tracker.active_node.as_deref(),
        active_node,
        &tracker.conn_bytes,
        current_conns,
    );
    tracker.active_node = Some(active_node.to_owned());
    tracker.conn_bytes = next_map;
    (state, stall_cooldown_active)
}

async fn sample_active_node_traffic(
    mihomo: &tauri_plugin_mihomo::Mihomo,
    active_node: &str,
) -> (ActiveTrafficState, bool) {
    let Ok(conns) = mihomo.get_connections().await else {
        return (ActiveTrafficState::Idle, false);
    };

    let current_conns: Vec<(String, u64, u64)> = conns
        .connections
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.chains.iter().any(|ch| ch == active_node))
        .map(|c| (c.id, c.upload, c.download))
        .collect();

    let (state, stall_cooldown_active) =
        update_active_traffic_state(&mut ACTIVE_TRAFFIC_TRACKER.lock(), active_node, &current_conns);
    if state == ActiveTrafficState::Flowing {
        *LAST_FLOWING_TRAFFIC_AT.lock() = Some(Instant::now());
    }
    (state, stall_cooldown_active)
}

/// 判定本次体检是否必须发起一轮 `PROXY__METRICS` 物理实拨：
/// 非 `Alive` 或时间戳缺失强制实拨；`Alive` 结合活跃出口流量态分流免测（Flowing 30m / Stalled 破缓存 / Idle 15m）。
fn should_probe_active_node(
    status: NodeStatus,
    age: Option<Duration>,
    traffic: ActiveTrafficState,
    stall_cooldown_active: bool,
) -> bool {
    let NodeStatus::Alive(_) = status else {
        return true;
    };
    let Some(age) = age else {
        return true;
    };

    match traffic {
        ActiveTrafficState::Flowing => age >= Duration::from_secs(PASSIVE_TRAFFIC_MAX_SKIP_SECS),
        ActiveTrafficState::Stalled => !stall_cooldown_active || age >= Duration::from_secs(IDLE_HEALTH_FRESH_SECS),
        ActiveTrafficState::Idle => age >= Duration::from_secs(IDLE_HEALTH_FRESH_SECS),
    }
}

/// 纯判定：当前活跃出口是否在有效 `filter_text` 下已被排除，且组内至少存在 1 个符合过滤条件的真实候选（BUG-405）。
fn should_migrate_filtered_out_active<F>(active_node: &str, members: &[String], filter_text: &str, is_fake: F) -> bool
where
    F: Fn(&str) -> bool,
{
    let effective = crate::utils::node::effective_filter_text(filter_text);
    if effective.is_empty() || match_node_filter(active_node, effective) {
        return false;
    }
    members
        .iter()
        .any(|m| m != active_node && !is_dummy_node(m) && !is_fake(m) && match_node_filter(m, effective))
}

/// 评估当前活跃节点是否需要故障转移；返回 `Err` 表示 Mihomo API 不可达。
/// 判死仅采信本轮 `probe_metrics_group` 主/备测速 URL 实拨结果，不直接用免拨测异源历史判死。
async fn evaluate_failover(profile_uid: &str, consecutive_fails: u32) -> anyhow::Result<FailoverVerdict> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();

    let group_info = mihomo
        .get_group_by_name("PROXY")
        .await
        .map_err(|e| anyhow::anyhow!("获取 PROXY 组信息失败: {e}"))?;

    let active_node = match &group_info.now {
        Some(n) if !n.is_empty() && n != "DIRECT" && n != "REJECT" => n.clone(),
        _ => return Ok(FailoverVerdict::NoAction), // 直连/拒绝节点或无选中，不动作
    };

    // 假/广告节点或 HTTPS 验真软隔离节点 → 视为需要重选（无需等 url-test）。
    // S4：这是「不可信节点迁移」而非「节点已死」，不能套用原节点确已死门检。
    if is_dummy_node(&active_node) || crate::utils::node::is_confirmed_fake_node(&active_node) {
        return Ok(FailoverVerdict::ShouldMigrate);
    }

    let (filter_config, _) = get_filter_and_sort_config(profile_uid).await;
    let filter_text = filter_config.filter_text.trim();
    let members = group_info.all.clone().unwrap_or_default();

    // 活跃出口已被过滤词排除（如内核重构/provider 重载瞬间落入 -香港 -台湾 之外节点）且存在合规候选，立即触发迁移
    if should_migrate_filtered_out_active(
        &active_node,
        &members,
        filter_text,
        crate::utils::node::is_confirmed_fake_node,
    ) {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 当前活跃节点 {} 不在用户过滤范围「{}」内且存在合规候选，触发迁移",
            active_node,
            filter_text
        );
        return Ok(FailoverVerdict::ShouldMigrate);
    }

    let (traffic_state, stall_cooldown_active) = sample_active_node_traffic(&mihomo, &active_node).await;

    let mut proxies = mihomo
        .get_proxies()
        .await
        .map_err(|e| anyhow::anyhow!("获取代理列表失败: {e}"))?;

    // 【BUG-377 + BUG-379】只有「健康且数据在对应流量态保鲜窗口内」才允许免拨测；
    // 任何非 Alive 状态或疑似黑洞停滞态都必须先取得本轮主 URL 实拨证据，再重读 alive / history 裁决。
    let need_probe = match proxies.proxies.get(&active_node) {
        None => false, // 活跃节点不在代理列表（异常），不动作
        Some(p) => should_probe_active_node(node_status(p), history_age(p), traffic_state, stall_cooldown_active),
    };
    if need_probe && traffic_state == ActiveTrafficState::Stalled {
        if let Some(tracker) = ACTIVE_TRAFFIC_TRACKER.lock().as_mut() {
            tracker.last_stall_probe_at = Some(Instant::now());
        }
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 检测到活跃节点 {} 存在上行请求但零下行回包，立即触发体检复核",
            active_node
        );
    }

    // 本轮体检是否得出「主测速地址失真、节点侧正常」的结论。该结论必须进入健康裁决，
    // 不能只停留在日志里（见 MetricsProbeOutcome 注释）。
    let mut primary_url_distorted = false;

    if need_probe {
        // 经测量组并发直拨全员（TUN 下无自指回环），禁用短窗合并；初检第 1 次失利支持单目标免打备用 URL
        let allow_single_target_skip = consecutive_fails == 0;
        let probed_opt = probe_metrics_group(
            &mihomo,
            "体检",
            false,
            std::slice::from_ref(&active_node),
            allow_single_target_skip,
        )
        .await;

        // 主/备测速地址全灭代表测速链路本身不可用，本轮判 NoAction，避免切点抖动
        let Some(probed) = probed_opt else {
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] 测速链路不可用，本轮不判定活跃节点死活（避免测速失真引发切点抖动）"
            );
            // 若当前存在上行请求但零下行回包，短保鲜标红活跃节点消除假绿展示
            if traffic_state == ActiveTrafficState::Stalled && !crate::module::lightweight::is_in_lightweight_mode() {
                crate::core::handle::Handle::notify_delay_results(
                    "PROXY".into(),
                    vec![(active_node.clone(), 0)],
                    false,
                );
                crate::core::handle::Handle::refresh_proxies();
                *LAST_UI_ACTIVE_SYNC.lock() = Some(UiActiveSyncState {
                    node_name: active_node,
                    history_time: None,
                    reported_alive: false,
                });
            }
            return Ok(FailoverVerdict::NoAction);
        };
        // 先取出裁决结论，再取延迟表；结论不能被后面的重读 get_proxies() 冲掉。
        primary_url_distorted = probed.primary_url_distorted;
        let probed_delays = probed.delays;
        if probed_delays.is_empty() && !primary_url_distorted {
            *LAST_METRICS_PROBE.lock() = None;
        }

        // 【BUG-369 / BUG-387 修复】后台定期体检刚刚完成了一次真实的整组延迟探测：
        // - `!primary_url_distorted` 时即便 `probed_delays` 为空（全员超时 `{}`）也向前端推送全组 `0 (Timeout)`；
        // - `primary_url_distorted` 时仅同步真正测得有效延迟的节点，避免把未响应节点漆成 30 分钟 `0`。
        let mut ui_synced_display: Option<Vec<(String, u32)>> = None;
        if !crate::module::lightweight::is_in_lightweight_mode() {
            let already_reported = !probed_delays.is_empty()
                && LAST_METRICS_PROBE
                    .lock()
                    .as_ref()
                    .is_some_and(|s| s.reported_live_to_ui);
            if !already_reported {
                let display = build_probe_ui_display(
                    group_info.all.as_deref().unwrap_or(&[]),
                    &probed_delays,
                    primary_url_distorted,
                );
                if !display.is_empty() {
                    crate::core::handle::Handle::notify_delay_results("PROXY".into(), display.clone(), true);
                    crate::core::handle::Handle::refresh_proxies();
                    ui_synced_display = Some(display);
                    if !probed_delays.is_empty()
                        && !primary_url_distorted
                        && let Some(snap) = LAST_METRICS_PROBE.lock().as_mut()
                    {
                        snap.reported_live_to_ui = true;
                    }
                }
            }
        }

        // 拨测后重读，确保 proxy.alive 与 proxy.history.last() 反映最新真实状态，
        // 而非 300 秒前测量组自然周期的陈旧缓存。
        proxies = mihomo
            .get_proxies()
            .await
            .map_err(|e| anyhow::anyhow!("获取代理列表失败: {e}"))?;

        if let Some(display) = ui_synced_display {
            let active_hist_time = proxies
                .proxies
                .get(&active_node)
                .and_then(|p| p.history.last())
                .map(|h| h.time.clone());
            let mut guard = LAST_UI_ACTIVE_SYNC.lock();
            *guard = compute_next_ui_active_sync(&active_node, &display, active_hist_time, guard.as_ref());
        }
    } else if !crate::module::lightweight::is_in_lightweight_mode()
        && matches!(
            proxies.proxies.get(&active_node).map(node_status),
            Some(NodeStatus::Alive(_))
        )
    {
        // 【BUG-370 / BUG-387 修复】免拨测确认恢复（如重试解除、离线恢复）、此前曾向 UI 推送 `Timeout`
        // （Mihomo 超时不写 history，若仅比时间戳会静默漏推）、或内核 PROXY__METRICS 自然周期（300s）
        // 刚刚刷新出比上次上屏更新的可用结果时，同步向前端推送可用延迟与 refresh_proxies()，
        // 彻底打通“断流报 Timeout、恢复却静默”的同步断层。
        let active_hist_time = proxies
            .proxies
            .get(&active_node)
            .and_then(|p| p.history.last())
            .map(|h| h.time.clone());
        let health_not_ok = *LAST_NOTIFIED_HEALTH.lock() != "network_health::ok";
        let should_sync = {
            let guard = LAST_UI_ACTIVE_SYNC.lock();
            should_sync_alive_ui(health_not_ok, guard.as_ref(), &active_node, active_hist_time.as_deref())
        };
        if should_sync {
            let fresh_alive_display: Vec<(String, u32)> = group_info
                .all
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .filter(|n| !is_dummy_node(n) && !crate::utils::node::is_confirmed_fake_node(n))
                .filter_map(|n| {
                    let p = proxies.proxies.get(n)?;
                    match (node_status(p), history_age(p)) {
                        (NodeStatus::Alive(d), Some(age))
                            if age < Duration::from_secs(PASSIVE_TRAFFIC_MAX_SKIP_SECS) =>
                        {
                            Some((n.clone(), d))
                        }
                        _ => None,
                    }
                })
                .collect();
            if !fresh_alive_display.is_empty() {
                crate::core::handle::Handle::notify_delay_results("PROXY".into(), fresh_alive_display, true);
                crate::core::handle::Handle::refresh_proxies();
                *LAST_UI_ACTIVE_SYNC.lock() = Some(UiActiveSyncState {
                    node_name: active_node.clone(),
                    history_time: active_hist_time,
                    reported_alive: true,
                });
            }
        }
    }

    // 连续性优先：主 URL 整体失败而备用 URL 正常即属测速地址失真，本轮直接判 NoAction
    if primary_url_distorted {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 备用测速地址已证明节点侧正常 → 本轮返回 NoAction（不累计失败、不自愈、不切点）"
        );
        return Ok(FailoverVerdict::NoAction);
    }

    // 仅由本轮主/备测速 URL 实拨结果裁决节点死活；Alive 节点仅在非 Flowing 且冷却允许时执行真 HTTPS 复核
    match proxies.proxies.get(&active_node).map(node_status) {
        Some(NodeStatus::Alive(_)) => {
            if should_allow_https_migration(traffic_state) && !crate::utils::node::is_verified_true_node(&active_node) {
                let can_verify_now = LAST_ACTIVE_VERIFY_CHECK
                    .lock()
                    .is_none_or(|t| t.elapsed() >= Duration::from_secs(ACTIVE_VERIFY_COOLDOWN_SECS));
                if can_verify_now {
                    *LAST_ACTIVE_VERIFY_CHECK.lock() = Some(Instant::now());
                    let verify_members: Vec<String> = members
                        .iter()
                        .filter(|n| !is_dummy_node(n) && !crate::utils::node::is_confirmed_fake_node(n))
                        .cloned()
                        .collect();
                    run_https_verification_pass(&mihomo, &verify_members, "活跃节点即时验真").await;
                }
                if crate::utils::node::has_verify_failure_node(&active_node) {
                    let has_verified_healthy = members.iter().any(|name| {
                        name != &active_node
                            && !is_dummy_node(name)
                            && !crate::utils::node::is_confirmed_fake_node(name)
                            && crate::utils::node::is_verified_true_node(name)
                            && match_node_filter(name, filter_text)
                            && matches!(proxies.proxies.get(name).map(node_status), Some(NodeStatus::Alive(_)))
                    });
                    if has_verified_healthy {
                        logging!(
                            info,
                            Type::Lightweight,
                            "[后台监测] 当前节点 {} 真 HTTPS 验真失败且子集内存在已验真健康节点，触发切往已验真节点",
                            active_node
                        );
                        // S4：原节点仍健康，属「不可信节点迁移」，不是故障转移
                        return Ok(FailoverVerdict::ShouldMigrate);
                    }
                }
            }
            return Ok(FailoverVerdict::NoAction);
        }
        // 【S4 · 三态迟滞】一次拨测失败但近期曾成功 ⇒ 疑似抖动，不是节点已死。
        // 本轮不累计失败、不进自愈链；由下一周期按新鲜度重新拨测确认。
        Some(NodeStatus::Suspect) => {
            logging!(
                info,
                Type::Lightweight,
                "[后台监测] 活跃节点 {} 仅为疑似抖动（alive=false 但近期曾成功），本轮不判死、不累计失败",
                active_node
            );
            return Ok(FailoverVerdict::NoAction);
        }
        Some(NodeStatus::Unknown) | None => {
            return Ok(FailoverVerdict::NoAction);
        }
        Some(NodeStatus::Dead) => {}
    }

    // 活跃节点已死：在子集（filterText）内寻找健康候选（剔除假/已拉黑节点）
    let mut found_healthy = false;
    for name in &members {
        if name == &active_node || is_dummy_node(name) || crate::utils::node::is_confirmed_fake_node(name) {
            continue;
        }
        if !match_node_filter(name, filter_text) {
            continue;
        }
        if let Some(NodeStatus::Alive(_)) = proxies.proxies.get(name).map(node_status) {
            found_healthy = true;
            break;
        }
    }

    let verdict = if found_healthy {
        FailoverVerdict::ShouldFailover
    } else {
        FailoverVerdict::AllDead
    };
    Ok(verdict)
}

/// 纯判定部分（便于单测锁定「什么才算结构性有效」）：只做结构性校验，
/// 不读 `alive` / `history`，也不发起任何网络调用。
fn persisted_exit_is_usable(
    now: Option<&str>,
    all: Option<&[String]>,
    filter_text: &str,
    confirmed_fake: bool,
) -> bool {
    let Some(now) = now.filter(|n| !n.is_empty()) else {
        return false;
    };
    if now == "DIRECT" || now == "REJECT" || is_dummy_node(now) || confirmed_fake {
        return false;
    }
    if !match_node_filter(now, crate::utils::node::effective_filter_text(filter_text)) {
        return false;
    }
    all.is_some_and(|nodes| nodes.iter().any(|n| n == now))
}

/// 最高方针「连续性优先」（agreements §5 / Sync-22）：判断当前 `PROXY.now` 是否仍是
/// 结构性有效的用户持久出口，供**纯托盘启动**的初始化自动选点决定是否允许切点。
///
/// 仅用于初始化场景，F1/F4 手动选点与后台故障自愈不受此判断约束。刻意不检查
/// `alive` / `history`：冷启动时内核尚未拨测，这些数据不可靠，「没有健康延迟」
/// 不等于「节点已死」；真死节点由后台监测经连续 2 次确认 + 死节点门检自愈，
/// 不在此抢占。任何读取失败一律返回 `false`（保守允许选点），保证仍能建立可用出口。
async fn has_usable_persisted_exit(profile_uid: &str) -> bool {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let Ok(group) = mihomo.get_group_by_name("PROXY").await else {
        return false;
    };
    let (filter_config, _) = get_filter_and_sort_config(profile_uid).await;
    let now = group.now.as_deref();
    let confirmed_fake = now.is_some_and(crate::utils::node::is_confirmed_fake_node);
    persisted_exit_is_usable(now, group.all.as_deref(), &filter_config.filter_text, confirmed_fake)
}

/// 自动并发测速并优选切换到符合过滤条件的最快节点
/// - `sort_type`: 0=从配置文件读取, 1=按延迟排序, 2=按名称排序（仅影响展示顺序）
/// - `select`: true=测速完成后将 PROXY 切换至最快节点；false=仅测速填充展示，不切换
/// - `skip_wait_ready`: true=跳过 wait_for_clash_ready（调用方已确保就绪）
pub async fn trigger_backend_auto_select(
    profile_uid: &str,
    node_names: Option<Vec<String>>,
    sort_type: i32,
    select: bool,
    skip_wait_ready: bool,
) -> anyhow::Result<AutoSelectOutcome> {
    trigger_backend_auto_select_with_options(profile_uid, node_names, sort_type, select, skip_wait_ready, false).await
}

async fn trigger_backend_auto_select_with_options(
    profile_uid: &str,
    node_names: Option<Vec<String>>,
    sort_type: i32,
    select: bool,
    skip_wait_ready: bool,
    allow_self_heal_reuse: bool,
) -> anyhow::Result<AutoSelectOutcome> {
    // 互斥锁防止并发调用
    if AUTO_SELECT_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        logging!(info, Type::Lightweight, "[后台监测] 自动选点已在运行中，返回繁忙");
        return Err(anyhow::anyhow!("AUTO_SELECT_BUSY"));
    }

    // 修复 BUG-MAJOR-001：使用 Drop Guard 确保锁一定释放（即使发生 panic）
    struct LockGuard;
    impl Drop for LockGuard {
        fn drop(&mut self) {
            AUTO_SELECT_RUNNING.store(false, Ordering::Release);
            logging!(debug, Type::Lightweight, "[后台监测] 自动选点锁已释放（Drop Guard）");
        }
    }
    let _guard = LockGuard;

    // 确保内核就绪后再进行选点操作（调用方已确保就绪时跳过）
    if !skip_wait_ready && !wait_for_clash_ready().await {
        logging!(warn, Type::Lightweight, "[后台监测] 自动选点失败：内核尚未就绪");
        return Ok(AutoSelectOutcome {
            display: vec![],
            selected: false,
            selected_node: None,
            source: DelayResultSource::NotExecuted,
            covers_all_group_nodes: false,
        });
    }

    // 直接调用内部函数
    // 如果发生 panic，_guard 会在栈展开时自动释放锁
    let result =
        trigger_backend_auto_select_inner(profile_uid, node_names, sort_type, select, allow_self_heal_reuse).await;

    // 锁已通过 Drop Guard 释放，在此之后执行副作用操作
    // （refresh_clash 和 notify_delay_results 不影响选点逻辑，放在锁外可减少锁持有时间）
    // 🛡️ 防线二：轻量模式下直接跳过副作用分发，避免无用的跨进程通信和撞车风险
    // 【治本】不再调用 refresh_clash()：整份配置重载 + auto_close_connection 是真实断流推手；
    // 选点本身已是干净热切换（select_node_for_group），无需重载。仅把真实延迟结果回写前端。
    if !crate::module::lightweight::is_in_lightweight_mode()
        && let Ok(outcome) = &result
        && !outcome.display.is_empty()
        && outcome.source != DelayResultSource::NotExecuted
    {
        let from_probe = matches!(outcome.source, DelayResultSource::LiveProbe);
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 准备发送 {} 条延迟结果到前端 (source={:?}, from_probe={})...",
            outcome.display.len(),
            outcome.source,
            from_probe
        );
        let t0 = std::time::Instant::now();
        crate::core::handle::Handle::notify_delay_results("PROXY".into(), outcome.display.clone(), from_probe);
        if from_probe
            && outcome.covers_all_group_nodes
            && let Some(snap) = LAST_METRICS_PROBE.lock().as_mut()
        {
            snap.reported_live_to_ui = true;
        }
        let active_target = if let Some(selected) = outcome.selected_node.clone() {
            Some(selected)
        } else {
            get_active_node_name().await
        };
        if let Some(cur_active) = active_target {
            let mut guard = LAST_UI_ACTIVE_SYNC.lock();
            *guard = compute_next_ui_active_sync(&cur_active, &outcome.display, None, guard.as_ref());
        }
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] notify_delay_results 完成，耗时 {:?}",
            t0.elapsed()
        );
    }

    result
}

async fn trigger_backend_auto_select_inner(
    profile_uid: &str,
    node_names: Option<Vec<String>>,
    sort_type: i32,
    select: bool,
    allow_self_heal_reuse: bool,
) -> anyhow::Result<AutoSelectOutcome> {
    let start_manual_epoch = MANUAL_SWITCH_EPOCH.load(Ordering::Acquire);
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();

    // 解析展示排序方式（sort_type==0 时从 head state 读取）
    let (filter_config, saved_sort_type) = get_filter_and_sort_config(profile_uid).await;
    let sort_type = if sort_type == 0 {
        // 未传入 sort_type 时，从 head state 配置文件读取
        saved_sort_type.unwrap_or(1)
    } else {
        sort_type
    };

    // 确定待测节点子集：
    // - 传入 node_names（非空）：使用调用方指定的子集（如前端当前可见/所选节点），
    //   仅剔除 dummy，不再套用后端 filter_text（子集已由前端按当前筛选条件给出）
    // - 未传入：后端自取 PROXY 全量节点，套用 dummy + 保存的 filter_text 过滤（F1/自动选点场景）
    // is_frontend_subset 用于下方「测量组数据新鲜时跳过全组拨测」的判定
    let is_frontend_subset = matches!(&node_names, Some(names) if !names.is_empty());
    let valid_nodes: Vec<String> = match node_names {
        Some(names) if !names.is_empty() => names
            .into_iter()
            .filter(|n| !is_dummy_node(n) && !crate::utils::node::is_confirmed_fake_node(n))
            .collect(),
        _ => {
            let group_info = mihomo
                .get_group_by_name("PROXY")
                .await
                .map_err(|e| anyhow::anyhow!("获取 PROXY 组信息失败: {e}"))?;
            let nodes = match group_info.all {
                Some(n) => n,
                None => {
                    return Ok(AutoSelectOutcome {
                        display: vec![],
                        selected: false,
                        selected_node: None,
                        source: DelayResultSource::NotExecuted,
                        covers_all_group_nodes: false,
                    });
                }
            };
            let filter_text = filter_config.filter_text.trim();
            nodes
                .into_iter()
                .filter(|n| {
                    !is_dummy_node(n)
                        && !crate::utils::node::is_confirmed_fake_node(n)
                        && match_node_filter(n, filter_text)
                })
                .collect()
        }
    };

    if valid_nodes.is_empty() {
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] 自动选点失败: 没有可用的非广告且符合过滤条件的节点"
        );
        return Ok(AutoSelectOutcome {
            display: vec![],
            selected: false,
            selected_node: None,
            source: DelayResultSource::NotExecuted,
            covers_all_group_nodes: false,
        });
    }

    logging!(
        info,
        Type::Lightweight,
        "[后台监测] 开始自动优选最快节点，过滤词: \"{}\"，待测节点数: {}",
        filter_config.filter_text,
        valid_nodes.len()
    );

    // 【拨测复用与并发风暴抑制】：
    // 1. 优先查近期完整 `probe_metrics_group` 快照（支持自愈复用 <10s 二次确认结果、
    //    以及前端批量子集复用 <30s 结果，避开 Mihomo 死节点不更新 history 的退化）；
    // 2. 若无进程内近期快照且属于前端批量子集（len > 1），再查 Mihomo 测量组自然周期 history；
    // 3. 显式单点测速（valid_nodes.len() == 1）绝不复用任何缓存，必须发起实时探测。
    let delays: std::collections::HashMap<String, u32>;
    let source: DelayResultSource;
    let mut skip_unresponded_in_display = false;
    let reused_snapshot = try_reuse_probe_snapshot(
        &mut LAST_METRICS_PROBE.lock(),
        Instant::now(),
        is_frontend_subset,
        valid_nodes.len(),
        allow_self_heal_reuse,
        &valid_nodes,
    );
    if let Some((snap_delays, snap_source)) = reused_snapshot {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 复用近期测量组实时拨测快照（待测 {} 个节点，source={:?}），跳过重复全组拨测",
            valid_nodes.len(),
            snap_source
        );
        delays = snap_delays;
        source = snap_source;
    } else {
        let mut reused_cache = false;
        let mut cached_delays: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        if is_frontend_subset && valid_nodes.len() > 1 {
            if let Ok(proxies) = mihomo.get_proxies().await {
                let threshold = Duration::from_secs(METRICS_REUSE_MAX_AGE_SECS);
                let mut cached: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
                let mut all_fresh = true;
                for name in &valid_nodes {
                    match proxies.proxies.get(name) {
                        Some(p) if history_age(p).is_some_and(|age| age < threshold) => {
                            if p.alive {
                                if let Some(h) = p.history.last() {
                                    cached.insert(name.clone(), h.delay as u32);
                                }
                            }
                        }
                        // 存在无数据/数据陈旧的节点 → 需全组拨测刷新
                        _ => {
                            all_fresh = false;
                            break;
                        }
                    }
                }
                if all_fresh && cached.values().any(|&d| is_valid_probe_delay(d)) {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 子集测速复用测量组新鲜缓存（{} 个节点，{} 个存活），跳过全组拨测",
                        valid_nodes.len(),
                        cached.len()
                    );
                    cached_delays = cached;
                    reused_cache = true;
                }
            }
        }

        if reused_cache {
            delays = cached_delays;
            source = DelayResultSource::FreshMetricsCache;
        } else {
            let is_single_node_probe = is_frontend_subset && valid_nodes.len() == 1;
            let allow_coalesce = !is_single_node_probe;
            match probe_metrics_group(&mihomo, "选点", allow_coalesce, &valid_nodes, is_single_node_probe).await {
                Some(outcome)
                    if !outcome.delays.is_empty()
                        && (!outcome.primary_url_distorted
                            || valid_nodes
                                .iter()
                                .any(|n| outcome.delays.get(n).copied().is_some_and(is_valid_probe_delay))) =>
                {
                    skip_unresponded_in_display = outcome.primary_url_distorted;
                    delays = outcome.delays;
                    source = DelayResultSource::LiveProbe;
                }
                Some(_) | None => {
                    logging!(
                        warn,
                        Type::Lightweight,
                        "[后台监测] 测量组拨测未取得有效实时数据（待测节点数={}），判定为「未执行」，不回退陈旧 history，返回空结果",
                        valid_nodes.len()
                    );
                    if select && !crate::module::lightweight::is_in_lightweight_mode() {
                        crate::core::handle::Handle::refresh_proxies();
                    }
                    return Ok(AutoSelectOutcome {
                        display: vec![],
                        selected: false,
                        selected_node: None,
                        source: DelayResultSource::NotExecuted,
                        covers_all_group_nodes: false,
                    });
                }
            }
        }
    }

    let covers_all_group_nodes =
        !is_frontend_subset && filter_config.filter_text.trim().is_empty() && !skip_unresponded_in_display;

    // 分类收集：display 用于展示（全部有效节点；主URL失真时仅回写有有效延迟的节点），candidates 用于选点（仅有效延迟节点）
    let mut display: Vec<(String, u32)> = Vec::new();
    let mut candidates: Vec<(String, u32)> = Vec::new();
    for name in &valid_nodes {
        let delay = match delays.get(name) {
            Some(&d) => d,
            // 未测得（超时/失败/测量组未就绪）→ 0（timeout 语义）
            // 与 delay_proxy_by_name 失败时的返回一致，前端显示 "Timeout" 而非 "Error"
            None => 0,
        };
        if !skip_unresponded_in_display || is_valid_probe_delay(delay) {
            display.push((name.clone(), delay));
        }
        if (NODE_DELAY_MIN_MS..NODE_DELAY_MAX_MS).contains(&delay) {
            candidates.push((name.clone(), delay));
        }
    }

    // 失败服务器避让：剔除所在服务器最近被判死的候选（若全数位于冷却服务器则回退全量）
    if !candidates.is_empty() {
        let failed = snapshot_failed_servers();
        if !failed.is_empty() {
            let server_map = load_node_server_map().await;
            if !server_map.is_empty() {
                let kept: Vec<(String, u32)> = candidates
                    .iter()
                    .filter(|(n, _)| !server_map.get(n).is_some_and(|s| failed.contains(s)))
                    .cloned()
                    .collect();
                if kept.is_empty() {
                    logging!(
                        warn,
                        Type::Lightweight,
                        "[后台监测] 全部候选都位于失败服务器 {failed:?}，放弃避让以保证有节点可选"
                    );
                } else if kept.len() < candidates.len() {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 失败服务器避让：剔除 {} 个候选（服务器 {failed:?}），保留 {} 个",
                        candidates.len() - kept.len(),
                        kept.len()
                    );
                    candidates = kept;
                }
            }
        }
    }

    // 展示结果按 sort_type 排序（仅影响返回顺序，不影响选点；BUG-322/BUG-349：0ms 超时节点置底）
    sort_display_delays(&mut display, sort_type);

    let mut selected = false;
    let mut selected_node: Option<String> = None;

    if select {
        // 冷启动/全员未验真兜底：若当前有效延迟候选中尚无任何 verified_true 节点，
        // 触发一轮并发真 HTTPS 验真（仅当存在待观察节点且物理在线时生效）
        if !candidates.is_empty()
            && !candidates
                .iter()
                .any(|(n, _)| crate::utils::node::is_verified_true_node(n))
        {
            run_https_verification_pass(&mihomo, &valid_nodes, "选点前置验真").await;
            candidates.retain(|(n, _)| !crate::utils::node::is_confirmed_fake_node(n));
        }

        // 【BUG-392 · 切换前实时过滤门检】测速耗时数秒期间用户可能刚敲入/改完过滤词（如从 "-" 提交为 "-香港"）；
        // 必须在 select_node_for_group 切换前读取最新 filter_text 并淘汰越界候选，严禁先切成越界节点再事后跳过回写。
        let had_reachable_candidates = !candidates.is_empty();
        let latest_filter_text = get_filter_and_sort_config(profile_uid)
            .await
            .0
            .filter_text
            .trim()
            .to_string();

        // 优先从匹配最新过滤词且 verified_true 的节点中按延迟择优；若全员未验真则保底回退到全部匹配候选
        // 选点固定按延迟升序取全局最快节点，不受 sort_type 影响
        if let Some((fastest_node, delay)) = select_fastest_candidate(candidates, &latest_filter_text) {
            logging!(
                info,
                Type::Lightweight,
                "[后台监测] 测速完成，最优节点: {} ({}ms)",
                fastest_node,
                delay
            );

            // 双重校验：确保当前配置 UID 未被篡改
            let current_uid = get_current_profile_uid().await;
            if current_uid.as_deref() != Some(profile_uid) {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 活动配置已在选定期间更改，舍弃本次切换结果"
                );
                return Ok(AutoSelectOutcome {
                    display,
                    selected: false,
                    selected_node: None,
                    source,
                    covers_all_group_nodes,
                });
            }

            // 【BUG-388 · 手动切点抢占护栏】测速/验真期间用户手动切换过节点（epoch 递增），严禁覆盖用户最新显式选择
            let current_manual_epoch = MANUAL_SWITCH_EPOCH.load(Ordering::Acquire);
            if is_manual_switch_preempted(start_manual_epoch, current_manual_epoch) {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 选点期间检测到用户手动切点（epoch {} -> {}），放弃覆盖用户当前选择",
                    start_manual_epoch,
                    current_manual_epoch
                );
                return Err(anyhow::anyhow!("AUTO_SELECT_MANUAL_PREEMPTED"));
            }

            // 最高方针「连续性优先」（Sync-22）：最优节点若本就是当前出口，跳过一次
            // 无意义的 select_node_for_group 写入。读取组信息失败时按原逻辑执行，
            // 不改变任何既有语义；回写 profile.selected 与前端刷新仍照常进行（幂等）。
            let already_current = mihomo
                .get_group_by_name("PROXY")
                .await
                .ok()
                .and_then(|g| g.now)
                .is_some_and(|n| n == fastest_node.as_str());
            if already_current {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 最优节点 {} 已是当前出口，保持现状（不发起切换写入）",
                    fastest_node
                );
            }
            let switch_result = if already_current {
                Ok(())
            } else {
                mihomo.select_node_for_group("PROXY", &fastest_node).await.map(|_| ())
            };

            match switch_result {
                Ok(_) => {
                    if !already_current {
                        logging!(
                            info,
                            Type::Lightweight,
                            "[后台监测] 成功将 PROXY 策略组切换为: {}",
                            fastest_node
                        );
                    }
                    selected = true;
                    selected_node = Some(fastest_node.clone());
                    emit_network_health_notice("network_health::ok");
                    // 【关键修复】选点成功后同步回写 profile.selected（若未与最新过滤条件冲突）
                    persist_selected_node_if_matching(profile_uid, &fastest_node).await;
                    if !crate::module::lightweight::is_in_lightweight_mode() {
                        crate::core::handle::Handle::refresh_proxies();
                    }
                }
                Err(e) => {
                    logging!(warn, Type::Lightweight, "[后台监测] 切换节点失败: {e}");
                }
            }
        } else {
            if had_reachable_candidates {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 测速期间过滤条件已变更为「{}」，本批候选无匹配节点，跳过本次过时切换",
                    latest_filter_text
                );
            } else {
                logging!(warn, Type::Lightweight, "[后台监测] 自动选点失败: 所有测速节点均不可达");
            }
            // 【治本】失败/跳过路径同样刷新前端代理缓存，保持前端状态权威。
            if !crate::module::lightweight::is_in_lightweight_mode() {
                crate::core::handle::Handle::refresh_proxies();
            }
        }
    }

    Ok(AutoSelectOutcome {
        display,
        selected,
        selected_node,
        source,
        covers_all_group_nodes,
    })
}

async fn get_active_node_name() -> Option<String> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let group_info = mihomo.get_group_by_name("PROXY").await.ok()?;
    group_info.now.filter(|node| !node.is_empty())
}

/// 计算下一次健康检测的等待间隔（秒）：重试模式 3 秒、离线 5 秒、正常 15 秒，与唤醒周期对齐。
const fn check_interval_secs(is_retry_mode: bool, was_online: bool) -> u64 {
    if is_retry_mode {
        RETRY_CHECK_INTERVAL_SECS
    } else if !was_online {
        OFFLINE_CHECK_INTERVAL_SECS
    } else {
        NORMAL_CHECK_INTERVAL_SECS
    }
}

fn sort_display_delays(display: &mut [(String, u32)], sort_type: i32) {
    match sort_type {
        2 => display.sort_by(|a, b| a.0.cmp(&b.0)),
        _ => display.sort_by_key(|r| if r.1 == 0 { u32::MAX } else { r.1 }),
    }
}

/// 最近一次向前端同步的活跃节点状态（BUG-370 / BUG-387）
///
/// Mihomo 在拨测超时时不向 `proxy.history` 追加失败记录，若在推送 `0 (Timeout)` 时记录了
/// 断流前的旧成功 `history.last().time`，随后节点恢复健康时就会误判为「无需回传」。
#[derive(Debug, Clone, PartialEq, Eq)]
struct UiActiveSyncState {
    node_name: String,
    history_time: Option<String>,
    reported_alive: bool,
}

static LAST_NOTIFIED_HEALTH: parking_lot::Mutex<&'static str> = parking_lot::Mutex::new("network_health::ok");
static LAST_MANUAL_NODE_SWITCH: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
static MANUAL_SWITCH_EPOCH: AtomicU64 = AtomicU64::new(0);
static LAST_UI_ACTIVE_SYNC: parking_lot::Mutex<Option<UiActiveSyncState>> = parking_lot::Mutex::new(None);
static ALERT_DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// 记录前端手动切换产生的新节点切换时间戳并立即唤醒监测循环吸收（BUG-336），不触发额外全组拨测；
/// 同时递增全局手动切换代际号，使在途自动选点不会在测速结束后覆盖用户最新选择（BUG-388）。
pub fn notify_manual_node_switched() {
    MANUAL_SWITCH_EPOCH.fetch_add(1, Ordering::AcqRel);
    *LAST_MANUAL_NODE_SWITCH.lock() = Some(Instant::now());
    NODE_SWITCH_NOTIFY.notify_one();
}

fn reset_monitor_static_state() {
    *LAST_NOTIFIED_HEALTH.lock() = "network_health::ok";
    *LAST_MANUAL_NODE_SWITCH.lock() = None;
    MANUAL_SWITCH_EPOCH.store(0, Ordering::Release);
    *LAST_UI_ACTIVE_SYNC.lock() = None;
    *LAST_METRICS_PROBE.lock() = None;
    *LAST_ACTIVE_VERIFY_CHECK.lock() = None;
    *LAST_HTTPS_VERIFY_ALL_FAILED.lock() = None;
    *LAST_FLOWING_TRAFFIC_AT.lock() = None;
    *ACTIVE_TRAFFIC_TRACKER.lock() = None;
}

fn emit_network_health_notice(next_status: &'static str) {
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

/// 统一处理后台监测循环中的 API 不可达异常：累计次数、达阈值后复核存活并按裁决自愈。
#[allow(clippy::too_many_arguments)]
async fn handle_api_unreachable(
    current_profile: &str,
    log_context: &str,
    api_error_count: &mut u32,
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
    last_auto_select_time: &mut Option<Instant>,
    last_check_time: &mut Instant,
    recent_auto_switches: &mut Vec<Instant>,
) {
    *api_error_count += 1;
    logging!(
        warn,
        Type::Lightweight,
        "[后台监测] {log_context}，连续异常次数: {}",
        *api_error_count
    );
    if *api_error_count < API_ERROR_HEAL_THRESHOLD {
        return;
    }

    let api_alive_on_recheck = is_clash_api_alive().await;
    if decide_self_heal(SelfHealTrigger::ApiUnreachable {
        api_error_count: *api_error_count,
        api_alive_on_recheck,
    }) == SelfHealDecision::Allow
    {
        *api_error_count = 0;
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] 连续 {} 次 API 异常且复核仍不可达，触发自愈选点",
            API_ERROR_HEAL_THRESHOLD
        );
        self_heal_with_accounting(
            current_profile,
            None,
            SelfHealOrigin::ApiChannelDown,
            auto_select_fail_count,
            api_dead_heal_fail_count,
            last_auto_select_time,
            last_check_time,
            recent_auto_switches,
        )
        .await;
    } else {
        *api_error_count = 0;
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] API 通道复核已恢复，跳过自愈选点，交由下一周期统一重新测量"
        );
    }
}

/// 启动全局后台节点监测常驻线程（运行于独立守护运行时 `DAEMON_RUNTIME`，带 panic 自愈监督）
pub fn start_background_monitor() {
    let handle = AsyncHandler::spawn_daemon_supervised("background_monitor", Type::Lightweight, move || async move {
        reset_monitor_static_state();
        logging!(info, Type::Lightweight, "[后台监测] 自动监测及故障自愈守护线程启动成功");
        let thread_start_time = Instant::now();
        let mut last_check_time = Instant::now();
        let mut consecutive_fails: u32 = 0;
        let mut is_retry_mode = false;
        let mut last_active_node: Option<String> = None;
        // 节点切换时间：切换后 SWITCH_GRACE_PERIOD_SECS 内为宽限期，
        // 期间失败判定不计入连续失败次数（缓存数据未稳，防误判震荡）
        let mut last_node_change_time: Option<Instant> = None;
        let mut was_online = true;
        let mut is_first_run = true;
        let mut last_online_check_time: Option<Instant> = None;
        let mut last_https_verify_time: Option<Instant> = None;
        let mut last_auto_select_time: Option<Instant> = None;
        // S4：最近发生的「自愈实际切换」时间戳，用于最小间隔 + 滑动窗口速率上限
        let mut recent_auto_switches: Vec<Instant> = Vec::new();
        let mut auto_select_fail_count = 0u32;
        let mut api_dead_heal_fail_count = 0u32;
        // 连续 API 异常计数：mihomo API 不可达时（get_group_by_name 超时/连接失败）累计，
        // 达到阈值后触发自愈，避免把内核卡死误判为"内核重载中"而放弃自愈。
        let mut api_error_count = 0u32;
        // 【v2.6.5 隐患 C 修复】周期性心跳计数器
        // 用于在正常运行无事件时仍能定期输出 debug 心跳日志，与"线程已死"区分
        let mut cycle_count: u64 = 0;

        loop {
            if crate::core::handle::Handle::global().is_exiting() {
                logging!(info, Type::Lightweight, "[后台监测] 检测到应用退出，监测线程终止");
                break;
            }

            cycle_count += 1;

            // 【v2.6.5 隐患 C 修复】周期性心跳日志
            // 每 MONITOR_HEARTBEAT_CYCLE_COUNT 个循环周期打一条 debug 心跳，
            // 让"正常运行无事件"也能留下日志痕迹，与"线程已死"可区分。
            // debug 级别，生产默认不可见；排障时用 RUST_LOG=debug 启用。
            if cycle_count.is_multiple_of(MONITOR_HEARTBEAT_CYCLE_COUNT) {
                let uptime = thread_start_time.elapsed().as_secs();
                logging!(
                    debug,
                    Type::Lightweight,
                    "[后台监测] 心跳: active={:?}, fails={}, online={}, retry_mode={}, api_errors={}, uptime={}s, cycles={}",
                    last_active_node,
                    consecutive_fails,
                    was_online,
                    is_retry_mode,
                    api_error_count,
                    uptime,
                    cycle_count,
                );
            }

            if is_first_run {
                is_first_run = false;
                // 首次启动时等待内核就绪，确保后续 API 调用不会失败
                if !wait_for_clash_ready().await {
                    logging!(warn, Type::Lightweight, "[后台监测] 内核未就绪，跳过首次检测周期");
                    last_check_time = Instant::now();
                    continue;
                }
                // 窗口不可见（轻量/纯托盘启动）时，由后端在启动时执行一次初始化自动选点；
                // 窗口存在则交给前端，此处不动作。进入轻量模式（lightweight.rs B4）也会选点，
                // 二者由 trigger_backend_auto_select 内部的 AUTO_SELECT_RUNNING 互斥，不会重复执行。
                let window_state = crate::utils::window_manager::WindowManager::get_main_window_state();
                if matches!(window_state, crate::utils::window_manager::WindowState::NotExist) {
                    if let Some(uid) = get_current_profile_uid().await {
                        let _ = restore_profile_selected_nodes(&uid).await;
                        // 最高方针「连续性优先」（Sync-22）：持久出口仍有效时只做全组测速刷新，
                        // 不按最快节点覆盖用户选择；仅当出口结构性失效才允许重选。
                        // 前序 wait_for_clash_ready 已通过，跳过冗余等待；结果经事件回写前端 UI。
                        let select = !has_usable_persisted_exit(&uid).await;
                        let _ = trigger_backend_auto_select(&uid, None, 0, select, true).await;
                    }
                }
            } else {
                // 定期健康检测的间隔：重试模式 3 秒，离线 5 秒（对齐快速探测），正常 15 秒
                let check_interval = check_interval_secs(is_retry_mode, was_online);

                // 【注意】tokio::select! 中两个 Notify 的 notified() 分支：
                // - MONITOR_WAKEUP_NOTIFY 用 notify_one()，会存储许可，不存在完全丢失；
                // - PROFILE_SWITCH_NOTIFY 自 v2.8.8 起改用 notify_waiters()，不存储许可，
                //   唤醒信号可能在两次等待之间丢失。因此此处仅把它当作加速手段：即使丢失，
                //   下一个周期也会重新读取当前 Profile，语义仍然正确（等待阶段则由
                //   is_profile_switched 的状态对比兜底）。
                tokio::select! {
                    _ = sleep(Duration::from_secs(check_interval)) => {}
                    _ = MONITOR_WAKEUP_NOTIFY.notified() => {
                        logging!(debug, Type::Lightweight, "[后台监测] 收到唤醒信号，立即唤醒监测");
                        // 唤醒即放行一次体检（探活+必要时自愈），不再等下一自然周期
                        last_check_time = Instant::now() - Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS + 1);
                    }
                    _ = PROFILE_SWITCH_NOTIFY.notified() => {
                        logging!(debug, Type::Lightweight, "[后台监测] 收到配置切换通知信号，立即唤醒监测");
                        last_check_time = Instant::now() - Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS + 1);
                    }
                    _ = NODE_SWITCH_NOTIFY.notified() => {
                        logging!(debug, Type::Lightweight, "[后台监测] 收到手动切点通知，立即刷新切换宽限期");
                    }
                }
            }

            // 在任意 continue 早退分支（无活动配置 / 离线 / API 异常）之前优先吸收前端手动切点时间戳（BUG-336），
            // 确保 20s 切换宽限期第 0ms 即时重置且绝不被早退分支漏消费。
            let pending_manual_ts = LAST_MANUAL_NODE_SWITCH.lock().take();
            if let Some(manual_ts) = pending_manual_ts {
                last_node_change_time = Some(manual_ts);
                consecutive_fails = 0;
                is_retry_mode = false;
            }

            let current_profile = match get_current_profile_uid().await {
                Some(uid) => uid,
                None => {
                    continue;
                }
            };

            // 检测物理网络连通性状态（有节流门控，避免每次循环都发起 DNS 查询）
            let probe_interval = if was_online {
                Duration::from_secs(60) // 在线时：60 秒探测一次
            } else {
                Duration::from_secs(5) // 离线时：5 秒探测一次，快速发现网络恢复
            };
            let need_probe = last_online_check_time
                .map(|t| t.elapsed() >= probe_interval)
                .unwrap_or(true); // 首次循环一定探测

            let is_online = if need_probe {
                last_online_check_time = Some(Instant::now());
                if was_online && is_recent_flowing_traffic_active() {
                    true
                } else {
                    // 【性能优化与动态探测】：复用 is_physical_network_online 探测物理网络连通性
                    is_physical_network_online(2).await
                }
            } else {
                // 尚未到探测间隔，复用上次结果
                was_online
            };

            if !was_online && is_online {
                logging!(info, Type::Lightweight, "[后台监测] 检测到网络连接已恢复，重置失败计数");
                consecutive_fails = 0;
                is_retry_mode = false;

                // 【S3 统一裁决】物理网络恢复只说明链路可用，不构成节点故障证据。
                // 旧实现在此凭一次 `evaluate_failover != NoAction` 就立即自愈（绕过连续 2 次判定），
                // 且本周期主路径还会再拨测一次，形成双拨测；现在统一裁决器恒判 Wait，
                // 只放行一次体检，由主路径按连续失败计数裁决。
                debug_assert_eq!(
                    decide_self_heal(SelfHealTrigger::NetworkRecovered),
                    SelfHealDecision::Wait
                );
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 网络恢复：不立即自愈，交由统一裁决器重新测量并按连续 2 次失败计数"
                );
                // 与唤醒分支同一手法：把上次检测时刻前拨，强制本周期立即执行统一体检；
                // 若体检判死也只累计第 1 次失败，3 秒后第二次独立拨测确认才可能自愈。
                last_check_time = Instant::now() - Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS + 1);
                emit_network_health_notice("network_health::ok");
            } else if was_online && !is_online {
                logging!(
                    warn,
                    Type::Lightweight,
                    "[后台监测] 检测到网络已断开，暂停健康检测与自愈"
                );
                emit_network_health_notice("network_health::offline");
            }
            was_online = is_online;

            // 2. 定期检测与快速重试自愈：重试 3 秒，离线 5 秒，正常 15 秒，对齐唤醒周期
            let check_interval = check_interval_secs(is_retry_mode, was_online);
            if last_check_time.elapsed().as_secs() >= check_interval {
                last_check_time = Instant::now();

                // 如果当前处于离线状态，跳过本次检测
                if !is_online {
                    continue;
                }

                // 定期真 HTTPS 验真（BUG-400）：
                // - 排除普通离线/超时死节点（非 NodeStatus::Alive），避免全组健康节点陪跑；
                // - 仅当过滤子集内存在 `Alive` 但未验真节点，且活跃节点未验真或已验真 Alive 备用节点 < 3 时触发。
                if last_https_verify_time.is_none_or(|t| t.elapsed() >= Duration::from_secs(HTTPS_VERIFY_INTERVAL_SECS))
                {
                    last_https_verify_time = Some(Instant::now());
                    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
                    if let (Ok(proxy_group), Ok(proxies)) =
                        (mihomo.get_group_by_name("PROXY").await, mihomo.get_proxies().await)
                    {
                        let members: Vec<String> = proxy_group
                            .all
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|n| !is_dummy_node(n) && !crate::utils::node::is_confirmed_fake_node(n))
                            .collect();
                        let (filter_config, _) = get_filter_and_sort_config(&current_profile).await;
                        let filter_text = filter_config.filter_text.trim();
                        if should_run_periodic_https_verify_with(
                            &members,
                            proxy_group.now.as_deref(),
                            filter_text,
                            crate::utils::node::is_confirmed_fake_node,
                            |n| matches!(proxies.proxies.get(n).map(node_status), Some(NodeStatus::Alive(_))),
                            crate::utils::node::is_verified_true_node,
                        ) {
                            run_https_verification_pass(&mihomo, &members, "后台定期验真").await;
                        }
                    }
                }

                // 检测活动节点是否发生变化（包括自动选点和手动切换）
                let active_node_name = match get_active_node_name().await {
                    Some(name) => name,
                    None => {
                        handle_api_unreachable(
                            &current_profile,
                            "读取当前节点失败 (API不可达)",
                            &mut api_error_count,
                            &mut auto_select_fail_count,
                            &mut api_dead_heal_fail_count,
                            &mut last_auto_select_time,
                            &mut last_check_time,
                            &mut recent_auto_switches,
                        )
                        .await;
                        continue;
                    }
                };
                let mid_manual_ts = LAST_MANUAL_NODE_SWITCH.lock().take();
                if let Some(manual_ts) = mid_manual_ts {
                    last_node_change_time = Some(manual_ts);
                    consecutive_fails = 0;
                    is_retry_mode = false;
                }
                if Some(&active_node_name) != last_active_node.as_ref() {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 检测到活动节点发生变化: {:?} -> {}",
                        last_active_node,
                        active_node_name
                    );
                    last_active_node = Some(active_node_name.clone());
                    // 若手动切点通知已在当前宽限期内设置过精确点击时刻，保留该时间戳避免重复延展宽限期；
                    // 否则（如自动选点切换或外部 REST 切换）以当前检测时刻开启宽限期。
                    if last_node_change_time
                        .is_none_or(|t| t.elapsed() >= Duration::from_secs(SWITCH_GRACE_PERIOD_SECS))
                    {
                        last_node_change_time = Some(Instant::now());
                    }
                    consecutive_fails = 0;
                    is_retry_mode = false;
                }

                match evaluate_failover(&current_profile, consecutive_fails).await {
                    Ok(verdict) => {
                        // API 调用成功，重置 API 异常计数
                        api_error_count = 0;
                        match verdict {
                            FailoverVerdict::NoAction => {
                                if consecutive_fails > 0 {
                                    logging!(
                                        info,
                                        Type::Lightweight,
                                        "[后台监测] 活跃节点复检恢复正常（此前连续失败 {} 次），解除重试状态",
                                        consecutive_fails
                                    );
                                }
                                consecutive_fails = 0;
                                is_retry_mode = false;
                                emit_network_health_notice("network_health::ok");
                            }
                            FailoverVerdict::ShouldFailover
                            | FailoverVerdict::AllDead
                            | FailoverVerdict::ShouldMigrate => {
                                // S4：区分「节点已死故障转移」与「不可信节点迁移」，
                                // 只有前者需要过「原节点确已死」门检。
                                let self_heal_origin = if matches!(verdict, FailoverVerdict::ShouldMigrate) {
                                    SelfHealOrigin::NodeMigration
                                } else {
                                    SelfHealOrigin::ConfirmedDead
                                };
                                // 若在 evaluate_failover 拨测等待期间前端刚发生了手动节点切换（BUG-336），
                                // 立即吸收最新切换时间戳并进入切换宽限期，消除拨测期切点竞态。
                                let post_probe_manual_ts = LAST_MANUAL_NODE_SWITCH.lock().take();
                                if let Some(manual_ts) = post_probe_manual_ts {
                                    last_node_change_time = Some(manual_ts);
                                }
                                // 切换宽限期：切到新节点后 SWITCH_GRACE_PERIOD_SECS 内缓存未稳，失败判定可接受，
                                // 不累计连续失败次数，避免「切到死缓存 → 误判 → 再切走」的震荡。
                                let in_grace = last_node_change_time
                                    .map(|t| t.elapsed() < Duration::from_secs(SWITCH_GRACE_PERIOD_SECS))
                                    .unwrap_or(false);
                                if in_grace {
                                    logging!(
                                        info,
                                        Type::Lightweight,
                                        "[后台监测] 活跃节点检测异常 ({:?})，当前处于切换宽限期，不计入连续失败",
                                        verdict
                                    );
                                    consecutive_fails = 0;
                                    is_retry_mode = false;
                                } else {
                                    consecutive_fails += 1;
                                    is_retry_mode = true;
                                    logging!(
                                        info,
                                        Type::Lightweight,
                                        "[后台监测] 活跃节点检测异常 ({:?})，连续失败次数: {}",
                                        verdict,
                                        consecutive_fails
                                    );

                                    if decide_self_heal(SelfHealTrigger::NodeConfirmedDead { consecutive_fails })
                                        == SelfHealDecision::Allow
                                    {
                                        consecutive_fails = 0;
                                        is_retry_mode = false;

                                        logging!(
                                            info,
                                            Type::Lightweight,
                                            "[后台监测] 连续 2 次检测失败，启动后台自愈选点"
                                        );

                                        // 【失败服务器登记】把刚被判死的活跃节点**所在服务器**记入冷却表，
                                        // 供本次自愈选点避让（详见 FAILED_SERVER_COOLDOWN_SECS 的注释）。
                                        {
                                            let mut server_map = load_node_server_map().await;
                                            if let Some(server) = server_map.remove(&active_node_name) {
                                                record_server_failure(&server);
                                            }
                                        }

                                        // 统一走冷却/计数/警报记账（与网络恢复、API 异常路径一致，🟡6）
                                        self_heal_with_accounting(
                                            &current_profile,
                                            Some(&active_node_name),
                                            self_heal_origin,
                                            &mut auto_select_fail_count,
                                            &mut api_dead_heal_fail_count,
                                            &mut last_auto_select_time,
                                            &mut last_check_time,
                                            &mut recent_auto_switches,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        handle_api_unreachable(
                            &current_profile,
                            &format!("节点健康检查异常 (API不可达): {e}"),
                            &mut api_error_count,
                            &mut auto_select_fail_count,
                            &mut api_dead_heal_fail_count,
                            &mut last_auto_select_time,
                            &mut last_check_time,
                            &mut recent_auto_switches,
                        )
                        .await;
                    }
                }
            }
        }
    });

    let mut guard = MONITOR_TASK_HANDLE.lock().unwrap_or_else(|e| {
        logging!(
            warn,
            Type::Lightweight,
            "MONITOR_TASK_HANDLE 锁被中毒线程污染，恢复继续"
        );
        e.into_inner()
    });
    if let Some(old) = guard.take() {
        old.abort();
    }
    *guard = Some(handle);
}

/// 连续 5 次自愈失败 / 内核 API 不可达后，弹出 Windows 系统警报框。
///
/// 内部使用 `tokio::task::spawn_blocking` 包住 `show_error_dialog`
/// （其底层为 MessageBoxW 同步阻塞调用，会阻塞当前 tokio worker 线程直到用户点击"确定"）。
/// 这是有意为之：连续 5 次失败说明网络环境已严重恶化，继续健康检测与自愈已无意义，
/// 弹窗期间停下来等用户处理是正确行为。
fn fire_self_heal_alert_dialog(log_reason: &'static str, body: &'static str) {
    logging!(warn, Type::Lightweight, "{log_reason}");
    if ALERT_DIALOG_OPEN
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 系统网络警报弹窗已在显示中，跳过重复弹窗"
        );
        return;
    }
    tokio::task::spawn_blocking(move || {
        crate::show_error_dialog("Clash Mini - 网络警报", body);
        ALERT_DIALOG_OPEN.store(false, Ordering::Release);
    });
}

fn fire_self_heal_alert() {
    fire_self_heal_alert_dialog(
        "[后台监测] 连续 5 次自愈选点失败，弹出 Windows 提示框",
        "后台自动优选节点连续 5 次失败，当前所有代理节点均已失效，无法正常连接网络。\n\n请检查您的网络连接或节点订阅状态。",
    );
}

fn fire_core_restart_failed_alert() {
    fire_self_heal_alert_dialog(
        "[后台监测] 内核重启恢复失败，立即升级触发终态 Windows 提示框",
        "后台检测到代理内核通信异常且自动重启内核失败，当前无法正常恢复代理连接。\n\n请检查系统服务权限或手动重启 Clash Mini。",
    );
}

/// 执行一次自愈选点并统一记账：25 秒冷却保护 + auto_select_fail_count 累计 +
/// 连续 5 次弹 Windows 警报。
///
/// **调用前置条件（S3，§5.1 / §5.3）**：所有触发入口必须先经 `decide_self_heal`
/// 统一裁决且结果为 `SelfHealDecision::Allow`；网络恢复恒判 `Wait`，不得进入本函数。
/// 本函数只负责冷却与计数记账，不再承担「是否应当自愈」的判定，
/// 避免防抖契约分散在多个分支（🟡6 / Sync-19）。
///
/// - 冷却：auto_select 失败后至少等 25 秒再重试；处于冷却中直接返回 false 跳过。
/// - 计数：选点成功归零；失败或非 BUSY 报错加 1；AUTO_SELECT_BUSY 冲突不计失败、
///   回退冷却时间戳允许下次重试。
/// - 警报：连续 5 次失败弹 Windows 提示框（fire_self_heal_alert，内部 spawn_blocking）。
///
/// 返回 true 表示实际发起了自愈（不在冷却中），false 表示处于冷却中被跳过。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelfHealAction {
    None,
    RestartCore,
    FireAlert,
}

/// 纯函数状态机（BUG-318）：将「API 假死 3 次重启内核计数」与「全节点失联 5 次系统告警计数」解耦。
/// 1. 告警计数满 5 次优先触发终态 `FireAlert` 并重置双计数；
/// 2. 第 3 次失败时若内核 API 不可达则触发 `RestartCore`（此时仅重置 `api_dead_heal_fail_count`）；
///    后续由 `complete_restart_core_attempt` 根据重启结果裁决：重启成功则保留 `auto_select_fail_count` 继续累计至第 5 次告警，
///    若内核重启自身报错（`Err`）则视为不可恢复终态，立即升级触发 `FireAlert` 并重置本轮双计数；
/// 3. 若内核 API 可达则跳过重启并保留告警计数。
const fn evaluate_self_heal_counters(
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
    api_alive: bool,
) -> SelfHealAction {
    if *auto_select_fail_count >= 5 {
        *auto_select_fail_count = 0;
        *api_dead_heal_fail_count = 0;
        return SelfHealAction::FireAlert;
    }
    if *api_dead_heal_fail_count >= 3 {
        *api_dead_heal_fail_count = 0;
        if !api_alive {
            return SelfHealAction::RestartCore;
        }
    }
    SelfHealAction::None
}

/// 处理 `SelfHealAction::RestartCore` 的内核重启结果（纯函数，BUG-318）：
/// - `restart_ok == true`：保留 `auto_select_fail_count` 不清零，使重启后若仍断流可在第 5 次可靠触发 `FireAlert`；
/// - `restart_ok == false`：内核重启自身失败属终态故障，立即升级返回 `SelfHealAction::FireAlert` 并重置本轮双计数。
const fn complete_restart_core_attempt(
    restart_ok: bool,
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
) -> SelfHealAction {
    if restart_ok {
        SelfHealAction::None
    } else {
        *auto_select_fail_count = 0;
        *api_dead_heal_fail_count = 0;
        SelfHealAction::FireAlert
    }
}

// 统一记账函数历史上就承载多条自愈入口的计数/冷却/告警状态，S3 起新增裁决来源、
// S4 起新增切换速率窗口，参数数量随之增长；项目既有惯例允许此处保留显式参数列表。
#[allow(clippy::too_many_arguments)]
async fn self_heal_with_accounting(
    profile_uid: &str,
    dead_node: Option<&str>,
    origin: SelfHealOrigin,
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
    last_auto_select_time: &mut Option<Instant>,
    last_check_time: &mut Instant,
    recent_auto_switches: &mut Vec<Instant>,
) -> bool {
    // 25 秒冷却保护：auto_select 失败后至少等 25 秒再重试
    let now = Instant::now();
    if let Some(last_time) = *last_auto_select_time {
        let elapsed = now.duration_since(last_time).as_secs();
        if elapsed < 25 {
            logging!(
                info,
                Type::Lightweight,
                "[后台监测] 自愈选点在 25 秒冷却中（已过 {} 秒），跳过本次",
                elapsed
            );
            return false;
        }
    }

    // 【S4 · 原节点确已死门检（BUG-375）】裁决器放行只说明「连续 2 次体检失败」。
    // 切点前必须再读一次**同一权威来源**（get_proxies + node_status）确认原节点此刻仍确证已死；
    // 若它在拨测等待期间已恢复（Alive / Suspect / Unknown），说明只是一次抖动，放弃切点。
    // 自愈是兜底而非择优——「别的节点更快」永远不构成切点理由（那属于 F1 手动选点）。
    // 本门检只适用于 ConfirmedDead：不可信节点迁移（原节点本就健康，见 ShouldMigrate）与
    // API 通道故障（无原节点信息，沿用 BUG-318 重启/告警链路）均跳过。
    if self_heal_requires_dead_gate(origin)
        && let Some(dead) = dead_node.filter(|name| !name.is_empty())
    {
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        match mihomo.get_proxies().await {
            Ok(proxies) => {
                let status = proxies
                    .proxies
                    .get(dead)
                    .map(node_status)
                    .unwrap_or(NodeStatus::Unknown);
                if !should_proceed_with_self_heal(status) {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 自愈门检：原节点 {} 当前状态为 {:?}，未确证已死 → 放弃本次切点",
                        dead,
                        status
                    );
                    return false;
                }
            }
            Err(e) => {
                logging!(
                    warn,
                    Type::Lightweight,
                    "[后台监测] 自愈门检读取节点状态失败 ({e})，保守放弃本次切点"
                );
                return false;
            }
        }
    }

    // 【S4 · 切换速率上限】先裁剪滑动窗口，再按「实际切换」次数而非选点尝试次数裁决；
    // 对已通过确死门检的 `ConfirmedDead` 豁免速率截断，避免用户被强行锁死在已死节点上（BUG-388）。
    recent_auto_switches.retain(|t| now.duration_since(*t).as_secs() < AUTO_SWITCH_WINDOW_SECS);
    let last_switch_age = recent_auto_switches.last().map(|t| now.duration_since(*t).as_secs());
    if self_heal_subject_to_switch_rate_limit(origin)
        && !auto_switch_permitted(last_switch_age, recent_auto_switches.len())
    {
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] 自愈切换速率保护：距上次切换 {:?}s / 窗口内已切 {} 次（下限 {}s，{}s 内上限 {} 次），跳过本次切点",
            last_switch_age,
            recent_auto_switches.len(),
            MIN_AUTO_SWITCH_INTERVAL_SECS,
            AUTO_SWITCH_WINDOW_SECS,
            MAX_AUTO_SWITCHES_PER_WINDOW
        );
        return false;
    }

    // 【BUG-388 · API 通道恢复保活门检】若由 ApiChannelDown 进入自愈但此刻 API 已可读且当前出口健康，
    // 遵循连续性优先直接保活现状并复位双失败计数，避免 API 短暂恢复即全组切点。
    if matches!(origin, SelfHealOrigin::ApiChannelDown) {
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        if let (Ok(group_info), Ok(proxies)) = (mihomo.get_group_by_name("PROXY").await, mihomo.get_proxies().await)
            && let Some(active) = group_info.now.as_deref()
        {
            let is_valid_real = !active.is_empty()
                && active != "DIRECT"
                && active != "REJECT"
                && !is_dummy_node(active)
                && !crate::utils::node::is_confirmed_fake_node(active);
            let status = proxies
                .proxies
                .get(active)
                .map(node_status)
                .unwrap_or(NodeStatus::Unknown);
            if should_keep_alive_on_api_recovery(is_valid_real, status) {
                *auto_select_fail_count = 0;
                *api_dead_heal_fail_count = 0;
                emit_network_health_notice("network_health::ok");
                *last_check_time = Instant::now();
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] API 通道自愈前复核已恢复且当前节点 {} 健康（{:?}），保活现状并跳过切点",
                    active,
                    status
                );
                return false;
            }
        }
    }

    let prev_auto_select_time = *last_auto_select_time;
    *last_auto_select_time = Some(now);

    let mut failed_this_round = false;
    match trigger_backend_auto_select_with_options(profile_uid, None, 0, true, false, true).await {
        Ok(outcome) => {
            if outcome.selected {
                *auto_select_fail_count = 0;
                *api_dead_heal_fail_count = 0;
                emit_network_health_notice("network_health::ok");
                // S4：仅「真正换到不同节点」才计入速率窗口；选回同一节点不算一次切换。
                // dead_node 为 None 的 API 通道故障路径无基准，按「执行了切点」计入。
                let changed = match (dead_node, outcome.selected_node.as_deref()) {
                    (Some(d), Some(n)) => d != n,
                    _ => true,
                };
                if changed {
                    recent_auto_switches.push(Instant::now());
                }
                if matches!(origin, SelfHealOrigin::NodeMigration) {
                    let filter_text = get_filter_and_sort_config(profile_uid).await.0.filter_text;
                    close_excluded_proxy_connections(&filter_text).await;
                }
                if let (Some(dead), Some(new_node)) = (dead_node, outcome.selected_node.as_deref())
                    && !dead.is_empty()
                    && dead != new_node
                    && !matches!(origin, SelfHealOrigin::NodeMigration)
                {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 自愈已切换 {} → {}，不清理原节点存量连接（长连接由应用层自行重连）",
                        dead,
                        new_node
                    );
                }
            } else {
                *auto_select_fail_count += 1;
                *api_dead_heal_fail_count += 1;
                failed_this_round = true;
                emit_network_health_notice("network_health::all_dead");
                logging!(
                    warn,
                    Type::Lightweight,
                    "[后台监测] 自愈选点未选出可用节点（所有节点不可达），连续失败次数: {}",
                    *auto_select_fail_count
                );
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            if is_benign_auto_select_abort(&err_str) {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 自愈选点让路中断 ({err_str})，跳过本次切点且不施加冷却或故障惩罚"
                );
                // 并发繁忙 / 用户手动抢占均不算故障，回退冷却时间戳，允许下次重试
                *last_auto_select_time = prev_auto_select_time;
            } else {
                *auto_select_fail_count += 1;
                *api_dead_heal_fail_count += 1;
                failed_this_round = true;
                emit_network_health_notice("network_health::all_dead");
                logging!(
                    warn,
                    Type::Lightweight,
                    "[后台监测] 自愈选点失败 ({}), 连续失败次数: {}",
                    err_str,
                    *auto_select_fail_count
                );
            }
        }
    }
    *last_check_time = Instant::now();

    if failed_this_round {
        let need_api_check = *api_dead_heal_fail_count >= 3;
        let api_alive = if need_api_check {
            is_clash_api_alive().await
        } else {
            true
        };
        if need_api_check && api_alive {
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] 连续 3 次自愈选点失败，但内核 API 可达，判定为节点侧不可达（保留告警计数={}），跳过内核重启",
                *auto_select_fail_count
            );
        }
        match evaluate_self_heal_counters(auto_select_fail_count, api_dead_heal_fail_count, api_alive) {
            SelfHealAction::RestartCore => {
                logging!(
                    warn,
                    Type::Lightweight,
                    "[后台监测] 连续 3 次自愈选点失败且内核 API 不可达，尝试重启内核以恢复 API 通道（当前告警计数={}）",
                    *auto_select_fail_count
                );
                match crate::core::manager::CoreManager::global().restart_core().await {
                    Ok(()) => {
                        let _ = complete_restart_core_attempt(true, auto_select_fail_count, api_dead_heal_fail_count);
                        logging!(
                            info,
                            Type::Lightweight,
                            "[后台监测] 重启内核成功，API 通道应已恢复（保留告警计数={}）",
                            *auto_select_fail_count
                        );
                    }
                    Err(e) => {
                        logging!(
                            error,
                            Type::Lightweight,
                            "[后台监测] 重启内核失败: {e}，立即升级为终态告警并重置本轮计数"
                        );
                        if complete_restart_core_attempt(false, auto_select_fail_count, api_dead_heal_fail_count)
                            == SelfHealAction::FireAlert
                        {
                            fire_core_restart_failed_alert();
                        }
                    }
                }
            }
            SelfHealAction::FireAlert => {
                fire_self_heal_alert();
            }
            SelfHealAction::None => {}
        }
    }
    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{
        ACTIVE_TRAFFIC_TRACKER, API_ERROR_HEAL_THRESHOLD, ActiveTrafficState, FAILOVER_CONFIRM_FAILS,
        IDLE_HEALTH_FRESH_SECS, MAX_AUTO_SWITCHES_PER_WINDOW, MIN_AUTO_SWITCH_INTERVAL_SECS,
        PASSIVE_TRAFFIC_MAX_SKIP_SECS, SUSPECT_WINDOW_SECS, SelfHealDecision, SelfHealOrigin, SelfHealTrigger,
        auto_switch_permitted, classify_active_traffic_delta, classify_node_status, decide_self_heal,
        is_valid_probe_delay, persisted_exit_is_usable, self_heal_requires_dead_gate, should_proceed_with_self_heal,
    };
    use super::{
        DEFAULT_TEST_URL, DelayResultSource, Duration, FALLBACK_TEST_URL, HashMap, Instant, LAST_MANUAL_NODE_SWITCH,
        LAST_METRICS_PROBE, LAST_NOTIFIED_HEALTH, LAST_UI_ACTIVE_SYNC, METRICS_REUSE_MAX_AGE_SECS,
        MetricsProbeSnapshot, NodeStatus, SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS, SelfHealAction,
        build_probe_ui_display_with, complete_restart_core_attempt, compute_next_ui_active_sync,
        evaluate_self_heal_counters, merge_probe_delays, notify_manual_node_switched,
        prioritize_verified_candidates_with, reset_monitor_static_state, resolve_probe_target, secondary_test_url,
        should_probe_active_node, should_recheck_with_fallback, should_sync_alive_ui, sort_display_delays,
        try_reuse_probe_snapshot,
    };

    #[test]
    fn test_classify_node_status_suspect_hysteresis() {
        // 健康节点
        assert_eq!(classify_node_status(true, Some(80), Some(5)), NodeStatus::Alive(80));
        // 一次拨测失败但上一次成功拨测仍在疑似窗口内 → 抖动，不判死
        assert_eq!(classify_node_status(false, Some(80), Some(5)), NodeStatus::Suspect);
        assert_eq!(
            classify_node_status(false, Some(80), Some(SUSPECT_WINDOW_SECS - 1)),
            NodeStatus::Suspect
        );
        // 超过疑似窗口、或时间戳不可解析（保守）→ 确证已死
        assert_eq!(
            classify_node_status(false, Some(80), Some(SUSPECT_WINDOW_SECS)),
            NodeStatus::Dead
        );
        assert_eq!(classify_node_status(false, Some(80), None), NodeStatus::Dead);
        // 延迟越界（假节点 / 超时）即便 alive 也判死；无 history 判 Unknown
        assert_eq!(classify_node_status(true, Some(0), Some(5)), NodeStatus::Dead);
        assert_eq!(classify_node_status(true, Some(5000), Some(5)), NodeStatus::Dead);
        assert_eq!(classify_node_status(true, None, None), NodeStatus::Unknown);
    }

    #[test]
    fn test_self_heal_requires_confirmed_dead_origin() {
        // 自愈是兜底不是择优：只有原节点确证已死才允许切点
        assert!(should_proceed_with_self_heal(NodeStatus::Dead));
        assert!(!should_proceed_with_self_heal(NodeStatus::Suspect));
        assert!(!should_proceed_with_self_heal(NodeStatus::Alive(80)));
        assert!(!should_proceed_with_self_heal(NodeStatus::Unknown));
    }

    #[test]
    fn test_dead_gate_applies_only_to_confirmed_dead_failover() {
        // 只有节点侧故障转移需要「原节点确已死」复核；
        // 不可信节点迁移（原节点本就健康）与 API 通道故障（无原节点信息）不得被门检挡住。
        assert!(self_heal_requires_dead_gate(SelfHealOrigin::ConfirmedDead));
        assert!(!self_heal_requires_dead_gate(SelfHealOrigin::NodeMigration));
        assert!(!self_heal_requires_dead_gate(SelfHealOrigin::ApiChannelDown));
    }

    #[test]
    fn test_auto_switch_permitted_rate_limits() {
        // 从未切换过 → 放行
        assert!(auto_switch_permitted(None, 0));
        // 距上次实际切换不足最小间隔 → 拒绝
        assert!(!auto_switch_permitted(Some(0), 0));
        assert!(!auto_switch_permitted(Some(MIN_AUTO_SWITCH_INTERVAL_SECS - 1), 0));
        // 达到最小间隔且窗口内次数未达上限 → 放行
        assert!(auto_switch_permitted(Some(MIN_AUTO_SWITCH_INTERVAL_SECS), 0));
        assert!(auto_switch_permitted(
            Some(MIN_AUTO_SWITCH_INTERVAL_SECS),
            MAX_AUTO_SWITCHES_PER_WINDOW - 1
        ));
        // 窗口内次数达上限 → 拒绝
        assert!(!auto_switch_permitted(
            Some(MIN_AUTO_SWITCH_INTERVAL_SECS),
            MAX_AUTO_SWITCHES_PER_WINDOW
        ));
    }

    #[test]
    fn test_persisted_exit_is_usable_gates_init_auto_select() {
        let all = vec!["日本-01".to_string(), "香港-01".to_string()];

        // 用户持久出口结构性有效 → 保持现状，初始化选点不得切点
        assert!(persisted_exit_is_usable(Some("日本-01"), Some(&all), "", false));
        assert!(persisted_exit_is_usable(Some("日本-01"), Some(&all), "日本", false));

        // 无选中 / 空出口 → 允许重选，保证仍能建立可用出口
        assert!(!persisted_exit_is_usable(None, Some(&all), "", false));
        assert!(!persisted_exit_is_usable(Some(""), Some(&all), "", false));

        // 节点已从新配置中消失（换订阅 / 被污染的持久选择）→ 允许重选
        assert!(!persisted_exit_is_usable(Some("新加坡-01"), Some(&all), "", false));

        // DIRECT / REJECT / 广告节点 / 验真永久拉黑 → 允许重选
        assert!(!persisted_exit_is_usable(Some("DIRECT"), Some(&all), "", false));
        assert!(!persisted_exit_is_usable(Some("REJECT"), Some(&all), "", false));
        assert!(!persisted_exit_is_usable(
            Some("https://promo.example.com"),
            Some(&all),
            "",
            false
        ));
        assert!(!persisted_exit_is_usable(Some("日本-01"), Some(&all), "", true));

        // 不在当前过滤子集内 / 成员表读不到 → 保守允许重选
        assert!(!persisted_exit_is_usable(Some("日本-01"), Some(&all), "香港", false));
        assert!(!persisted_exit_is_usable(Some("日本-01"), None, "", false));
    }

    #[test]
    fn test_decide_self_heal_unifies_all_three_triggers() {
        // 1. 节点侧故障：第 1 次失败必须等待，达到连续 2 次才放行
        assert_eq!(
            decide_self_heal(SelfHealTrigger::NodeConfirmedDead { consecutive_fails: 1 }),
            SelfHealDecision::Wait
        );
        assert_eq!(
            decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
                consecutive_fails: FAILOVER_CONFIRM_FAILS
            }),
            SelfHealDecision::Allow
        );
        assert_eq!(
            decide_self_heal(SelfHealTrigger::NodeConfirmedDead { consecutive_fails: 5 }),
            SelfHealDecision::Allow
        );

        // 2. 网络恢复恒判 Wait：物理链路恢复不构成节点故障证据，禁止一次判定即自愈
        assert_eq!(
            decide_self_heal(SelfHealTrigger::NetworkRecovered),
            SelfHealDecision::Wait
        );

        // 3. 内核 API 通道故障：未达阈值等待；达阈值但复核时 API 已恢复仍等待；
        //    仅「达阈值且复核仍不可达」放行，避免用陈旧故障结论切点
        assert_eq!(
            decide_self_heal(SelfHealTrigger::ApiUnreachable {
                api_error_count: API_ERROR_HEAL_THRESHOLD - 1,
                api_alive_on_recheck: false,
            }),
            SelfHealDecision::Wait
        );
        assert_eq!(
            decide_self_heal(SelfHealTrigger::ApiUnreachable {
                api_error_count: API_ERROR_HEAL_THRESHOLD,
                api_alive_on_recheck: true,
            }),
            SelfHealDecision::Wait
        );
        assert_eq!(
            decide_self_heal(SelfHealTrigger::ApiUnreachable {
                api_error_count: API_ERROR_HEAL_THRESHOLD,
                api_alive_on_recheck: false,
            }),
            SelfHealDecision::Allow
        );
    }

    #[test]
    fn test_resolve_probe_target_ipv4() {
        assert_eq!(resolve_probe_target("http://1.2.3.4/generate_204"), "1.2.3.4:80");
        assert_eq!(resolve_probe_target("https://1.2.3.4:8443/test"), "1.2.3.4:8443");
    }

    #[test]
    fn test_resolve_probe_target_ipv6() {
        // IPv6 必须被包装为 [addr]:port，否则 lookup_host 解析失败
        assert_eq!(
            resolve_probe_target("http://[2001:db8::1]/generate_204"),
            "[2001:db8::1]:80"
        );
        assert_eq!(resolve_probe_target("https://[::1]:8443/test"), "[::1]:8443");
    }

    #[test]
    fn test_resolve_probe_target_domain() {
        assert_eq!(
            resolve_probe_target("http://cp.cloudflare.com/generate_204"),
            "cp.cloudflare.com:80"
        );
        assert_eq!(
            resolve_probe_target("https://example.com:8443/test"),
            "example.com:8443"
        );
    }

    #[test]
    fn test_resolve_probe_target_malformed() {
        // 非法 URL fallback 到默认目标（HTTPS 默认端口 443）
        assert_eq!(resolve_probe_target("not-a-valid-url"), "cp.cloudflare.com:443");
    }

    #[test]
    fn test_sort_display_delays_places_timeout_zero_at_bottom() {
        // BUG-322 / BUG-349: 0ms 代表测速超时，按延迟升序排列时必须排在所有真实延迟之后
        let mut items = vec![
            ("Dead-A".to_string(), 0),
            ("Slow-Node".to_string(), 220),
            ("Fast-Node".to_string(), 45),
            ("Dead-B".to_string(), 0),
        ];
        sort_display_delays(&mut items, 1);
        assert_eq!(items[0], ("Fast-Node".to_string(), 45));
        assert_eq!(items[1], ("Slow-Node".to_string(), 220));
        assert_eq!(items[2].1, 0);
        assert_eq!(items[3].1, 0);
    }

    #[test]
    fn test_self_heal_counters_reach_five_when_api_alive() {
        // BUG-318: 当内核 API 存活但所有节点不可达时，第 3 次失败不得清零告警计数，第 5 次必须触发告警
        let mut alert_count = 0;
        let mut api_dead_count = 0;
        for round in 1..=4 {
            alert_count += 1;
            api_dead_count += 1;
            let action = evaluate_self_heal_counters(&mut alert_count, &mut api_dead_count, true);
            assert_eq!(action, SelfHealAction::None, "round {round} should not alert yet");
        }
        assert_eq!(alert_count, 4);
        assert_eq!(api_dead_count, 1);

        alert_count += 1;
        api_dead_count += 1;
        let fifth_action = evaluate_self_heal_counters(&mut alert_count, &mut api_dead_count, true);
        assert_eq!(fifth_action, SelfHealAction::FireAlert);
        assert_eq!(alert_count, 0);
        assert_eq!(api_dead_count, 0);
    }

    #[test]
    fn test_self_heal_counters_reach_five_when_api_dead_across_restart() {
        // BUG-318: 当内核 API 不可达触发第 3 次 RestartCore 且重启后仍持续失败时，
        // 告警计数不得被 RestartCore 清零，第 5 次必须可靠触发 FireAlert！
        let mut alert_count = 0;
        let mut api_dead_count = 0;

        for round in 1..=2 {
            alert_count += 1;
            api_dead_count += 1;
            assert_eq!(
                evaluate_self_heal_counters(&mut alert_count, &mut api_dead_count, false),
                SelfHealAction::None,
                "round {round}"
            );
        }

        // 第 3 次：触发 RestartCore，api_dead_count 归零，重启成功后 complete_restart_core_attempt 必须保留 alert_count=3
        alert_count += 1;
        api_dead_count += 1;
        assert_eq!(
            evaluate_self_heal_counters(&mut alert_count, &mut api_dead_count, false),
            SelfHealAction::RestartCore
        );
        assert_eq!(
            complete_restart_core_attempt(true, &mut alert_count, &mut api_dead_count),
            SelfHealAction::None
        );
        assert_eq!(alert_count, 3);
        assert_eq!(api_dead_count, 0);

        // 第 4 次：重启后仍失败
        alert_count += 1;
        api_dead_count += 1;
        assert_eq!(
            evaluate_self_heal_counters(&mut alert_count, &mut api_dead_count, false),
            SelfHealAction::None
        );
        assert_eq!(alert_count, 4);

        // 第 5 次：必须触发 FireAlert 并清零双计数
        alert_count += 1;
        api_dead_count += 1;
        assert_eq!(
            evaluate_self_heal_counters(&mut alert_count, &mut api_dead_count, false),
            SelfHealAction::FireAlert
        );
        assert_eq!(alert_count, 0);
        assert_eq!(api_dead_count, 0);
    }

    #[test]
    fn test_complete_restart_core_attempt_escalates_on_restart_error() {
        // BUG-318: 当第 3 次触发 RestartCore 且内核重启自身失败（Err）时，立即升级为终态 FireAlert 并归零本轮双计数
        let mut alert_count = 3;
        let mut api_dead_count = 3;
        assert_eq!(
            evaluate_self_heal_counters(&mut alert_count, &mut api_dead_count, false),
            SelfHealAction::RestartCore
        );
        assert_eq!(
            complete_restart_core_attempt(false, &mut alert_count, &mut api_dead_count),
            SelfHealAction::FireAlert
        );
        assert_eq!(alert_count, 0);
        assert_eq!(api_dead_count, 0);
    }

    #[tokio::test]
    async fn test_manual_node_switch_wakes_notify_and_reset_clears_statics() {
        reset_monitor_static_state();
        *LAST_NOTIFIED_HEALTH.lock() = "network_health::all_dead";

        notify_manual_node_switched();
        assert!(LAST_MANUAL_NODE_SWITCH.lock().is_some());
        // 确认 NODE_SWITCH_NOTIFY 已存入许可，select! 可第 0ms 唤醒
        let notified_res = tokio::time::timeout(
            std::time::Duration::from_millis(20),
            super::NODE_SWITCH_NOTIFY.notified(),
        )
        .await;
        assert!(
            notified_res.is_ok(),
            "NODE_SWITCH_NOTIFY should store permit immediately"
        );

        // 守护线程重启时必须复位进程级 static 状态
        reset_monitor_static_state();
        assert_eq!(*LAST_NOTIFIED_HEALTH.lock(), "network_health::ok");
        assert!(LAST_MANUAL_NODE_SWITCH.lock().is_none());
        assert!(LAST_UI_ACTIVE_SYNC.lock().is_none());
        assert!(LAST_METRICS_PROBE.lock().is_none());
        assert!(ACTIVE_TRAFFIC_TRACKER.lock().is_none());
    }

    #[test]
    fn test_classify_active_traffic_delta() {
        let mut prev = HashMap::new();
        prev.insert("c1".to_string(), (100u64, 500u64));

        // 1. 节点切换首轮：必须返回 Idle 并重建新基线，绝不把旧节点连接的存量下载误判为 Flowing
        let curr_switch = vec![("c1".to_string(), 200u64, 900u64)];
        let (st_switch, map_switch) = classify_active_traffic_delta(Some("HK-1"), "JP-1", &prev, &curr_switch);
        assert_eq!(st_switch, ActiveTrafficState::Idle);
        assert_eq!(map_switch.get("c1"), Some(&(200, 900)));

        // 2. 下行字节增长 -> Flowing（真实回包证实节点存活）
        let curr_flow = vec![("c1".to_string(), 200u64, 1500u64)];
        let (st_flow, _) = classify_active_traffic_delta(Some("HK-1"), "HK-1", &prev, &curr_flow);
        assert_eq!(st_flow, ActiveTrafficState::Flowing);

        // 3. 已有或新建连接有上行发包但零下行回包 -> Stalled（疑似黑洞停滞）
        let curr_stalled = vec![("c1".to_string(), 300u64, 500u64), ("c2".to_string(), 120u64, 0u64)];
        let (st_stall, _) = classify_active_traffic_delta(Some("HK-1"), "HK-1", &prev, &curr_stalled);
        assert_eq!(st_stall, ActiveTrafficState::Stalled);

        // 4. 字节完全静止或全部连接已关闭 -> Idle
        let curr_idle = vec![("c1".to_string(), 100u64, 500u64)];
        let (st_idle, _) = classify_active_traffic_delta(Some("HK-1"), "HK-1", &prev, &curr_idle);
        assert_eq!(st_idle, ActiveTrafficState::Idle);
        let (st_empty, _) = classify_active_traffic_delta(Some("HK-1"), "HK-1", &prev, &[]);
        assert_eq!(st_empty, ActiveTrafficState::Idle);
    }

    #[test]
    fn test_should_probe_active_node_only_skips_fresh_healthy() {
        // 1. 空闲态放宽至 IDLE_HEALTH_FRESH_SECS (900s)
        assert!(!should_probe_active_node(
            NodeStatus::Alive(85),
            Some(Duration::from_secs(IDLE_HEALTH_FRESH_SECS - 1)),
            ActiveTrafficState::Idle,
            false,
        ));
        assert!(should_probe_active_node(
            NodeStatus::Alive(85),
            Some(Duration::from_secs(IDLE_HEALTH_FRESH_SECS)),
            ActiveTrafficState::Idle,
            false,
        ));

        // 2. 被动下行流量证活：1799s 内免除全组拨测风暴，满 PASSIVE_TRAFFIC_MAX_SKIP_SECS (1800s) 才复检一次
        assert!(!should_probe_active_node(
            NodeStatus::Alive(85),
            Some(Duration::from_secs(PASSIVE_TRAFFIC_MAX_SKIP_SECS - 1)),
            ActiveTrafficState::Flowing,
            false,
        ));
        assert!(should_probe_active_node(
            NodeStatus::Alive(85),
            Some(Duration::from_secs(PASSIVE_TRAFFIC_MAX_SKIP_SECS)),
            ActiveTrafficState::Flowing,
            false,
        ));

        // 3. 黑洞停滞态（有上行无下行）：冷却未激活时立即突破 15s~300s 缓存实拨复核；冷却期内不反复轰炸
        assert!(should_probe_active_node(
            NodeStatus::Alive(85),
            Some(Duration::from_secs(15)),
            ActiveTrafficState::Stalled,
            false,
        ));
        assert!(!should_probe_active_node(
            NodeStatus::Alive(85),
            Some(Duration::from_secs(15)),
            ActiveTrafficState::Stalled,
            true,
        ));

        // 4. 数据缺失、未知、疑似抖动与已判死一律实拨，连续 2 次失败因此来自两轮独立物理拨测
        assert!(should_probe_active_node(
            NodeStatus::Alive(85),
            None,
            ActiveTrafficState::Flowing,
            false,
        ));
        assert!(should_probe_active_node(
            NodeStatus::Unknown,
            None,
            ActiveTrafficState::Idle,
            false,
        ));
        assert!(should_probe_active_node(
            NodeStatus::Suspect,
            Some(Duration::from_secs(1)),
            ActiveTrafficState::Flowing,
            false,
        ));
        assert!(should_probe_active_node(
            NodeStatus::Dead,
            Some(Duration::from_secs(3)),
            ActiveTrafficState::Flowing,
            false,
        ));
        assert!(should_probe_active_node(
            NodeStatus::Dead,
            None,
            ActiveTrafficState::Idle,
            false,
        ));
    }

    #[test]
    fn test_https_verify_tainted_history_can_never_skip_probe() {
        // 【BUG-377 / Sync-23 根因复现】真 HTTPS 验真以 6000ms 超时对同一测量组拨测，
        // 成功时可能把 `[5000, 6000)` 的延迟写进 history.last()，时间戳又极新：
        // classify_node_status 判 Dead，旧逻辑的「Dead 且 history < 10s 免拨测」直接
        // 跳过主 URL 复拨，把这次异源测量当成节点已死的证据。
        let tainted = classify_node_status(true, Some(5500), Some(2));
        assert_eq!(tainted, NodeStatus::Dead);
        assert!(
            should_probe_active_node(
                tainted,
                Some(Duration::from_secs(2)),
                ActiveTrafficState::Flowing,
                false
            ),
            "验真污染的 Dead + 新鲜时间戳必须强制主 URL 实拨复核"
        );
        // 统一超时口径：验真改用 NODE_TEST_TIMEOUT_MS（其值等于 NODE_DELAY_MAX_MS），
        // 结果不会再落入 `[5000, 6000)` 这一「健康裁决判死」的越界区间。
        assert!(!is_valid_probe_delay(5500));
        assert!(is_valid_probe_delay(4999));
    }

    #[test]
    fn test_try_reuse_probe_snapshot_respects_single_node_self_heal_and_ttl() {
        let base_now = Instant::now();
        let mut delays = HashMap::new();
        delays.insert("日本-01".to_string(), 75u32);

        let make_snap = |age_secs: u64| {
            Some(MetricsProbeSnapshot {
                completed_at: base_now - Duration::from_secs(age_secs),
                delays: delays.clone(),
                reported_live_to_ui: false,
            })
        };
        let empty_expected: Vec<String> = Vec::new();

        // 1. 显式单点测速（subset_len == 1）绝不复用快照，即使刚结束 1s（§5.2 契约 4）
        let mut snap1 = make_snap(1);
        assert!(try_reuse_probe_snapshot(&mut snap1, base_now, true, 1, false, &empty_expected).is_none());
        assert!(try_reuse_probe_snapshot(&mut snap1, base_now, true, 1, true, &empty_expected).is_none());

        // 2. 非自愈全量 F1（allow_self_heal_reuse=false, is_frontend_subset=false）不走缓存复用
        let mut snap2 = make_snap(2);
        assert!(try_reuse_probe_snapshot(&mut snap2, base_now, false, 10, false, &empty_expected).is_none());

        // 3. 自愈在 < SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS 内复用二次确认快照：
        // 首次消费获得 LiveProbe 语义，随后再次复用降级为 FreshMetricsCache
        let mut snap3 = make_snap(SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS - 1);
        let first = try_reuse_probe_snapshot(&mut snap3, base_now, false, 10, true, &empty_expected);
        assert_eq!(
            first.map(|(m, s)| (m.get("日本-01").copied(), s)),
            Some((Some(75), DelayResultSource::LiveProbe))
        );
        let second = try_reuse_probe_snapshot(&mut snap3, base_now, false, 10, true, &empty_expected);
        assert_eq!(second.map(|(_, s)| s), Some(DelayResultSource::FreshMetricsCache));

        // 4. 超过自愈 TTL（>=10s）但未超子集 TTL（<30s）：自愈不复用，批量子集复用且标记 FreshMetricsCache
        let mut snap4 = make_snap(SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS);
        assert!(try_reuse_probe_snapshot(&mut snap4, base_now, false, 10, true, &empty_expected).is_none());
        let subset_hit = try_reuse_probe_snapshot(&mut snap4, base_now, true, 5, false, &empty_expected);
        assert_eq!(subset_hit.map(|(_, s)| s), Some(DelayResultSource::FreshMetricsCache));

        // 5. 超过子集 TTL（>=30s）不再复用
        let mut snap5 = make_snap(METRICS_REUSE_MAX_AGE_SECS);
        assert!(try_reuse_probe_snapshot(&mut snap5, base_now, true, 5, false, &empty_expected).is_none());

        // 6. 【门控方向修正（Sync-18）】只要快照对期望目标节点全灭，一律禁止复用；
        // 原实现给「经过主/备双地址复核」的全灭快照开了后门，会在 10s 内污染自愈选点。
        let expected_other = vec!["香港-01".to_string(), "新加坡-01".to_string()];
        let mut snap_all_dark = make_snap(1);
        assert!(try_reuse_probe_snapshot(&mut snap_all_dark, base_now, true, 2, true, &expected_other).is_none());
        // 目标节点命中有效延迟时仍可正常复用
        let expected_hit = vec!["日本-01".to_string()];
        let mut snap_hit = make_snap(1);
        assert!(try_reuse_probe_snapshot(&mut snap_hit, base_now, true, 2, true, &expected_hit).is_some());
    }

    #[test]
    fn test_prioritize_verified_candidates_prefers_verified_nodes() {
        let candidates = vec![
            ("unverified-fast".to_owned(), 20),
            ("verified-mid".to_owned(), 80),
            ("verified-fast".to_owned(), 45),
        ];
        // 有 verified_true 节点时只保留已验真池（哪怕未验真节点延迟数字更低，绝不误选）
        let pool = prioritize_verified_candidates_with(candidates.clone(), |name| name.starts_with("verified-"));
        assert_eq!(pool.len(), 2);
        let fastest = pool.iter().min_by_key(|r| r.1).unwrap();
        assert_eq!(fastest.0, "verified-fast");

        // 全员均未验真时保底回退全部有效延迟候选，避免无点可选
        let fallback_pool = prioritize_verified_candidates_with(candidates, |_| false);
        assert_eq!(fallback_pool.len(), 3);
        assert_eq!(fallback_pool.iter().min_by_key(|r| r.1).unwrap().0, "unverified-fast");
    }

    #[test]
    fn test_partial_distortion_fallback_and_merge() {
        // 1. 主/备测速 URL 互换（含首尾空白与大小写兼容）
        assert_eq!(secondary_test_url(DEFAULT_TEST_URL), FALLBACK_TEST_URL);
        assert_eq!(secondary_test_url(FALLBACK_TEST_URL), DEFAULT_TEST_URL);
        assert_eq!(
            secondary_test_url(&format!("  {}  ", FALLBACK_TEST_URL.to_ascii_uppercase())),
            DEFAULT_TEST_URL
        );

        // 2. 局部测速失真判定：仅期望外节点的延全都无效（即便其它节点有值）也触发备用 URL 复测
        let mut partial_distortion = HashMap::new();
        partial_distortion.insert("其它节点".to_string(), 90u32);
        partial_distortion.insert("香港-01".to_string(), 0u32);
        partial_distortion.insert("日本-01".to_string(), 5000u32);
        let expected = vec!["香港-01".to_string(), "日本-01".to_string()];
        assert!(should_recheck_with_fallback(&partial_distortion, &expected));

        // 期望节点中只要存在有效延迟即无需复测
        partial_distortion.insert("日本-01".to_string(), 85u32);
        assert!(!should_recheck_with_fallback(&partial_distortion, &expected));

        // expected_nodes 为空时，仅当存在非虚拟有效延迟节点才跳过复测
        let mut all_dead = HashMap::new();
        all_dead.insert("DIRECT".to_string(), 10u32);
        all_dead.insert("日本-01".to_string(), 0u32);
        assert!(should_recheck_with_fallback(&all_dead, &[]));
        all_dead.insert("日本-01".to_string(), 75u32);
        assert!(!should_recheck_with_fallback(&all_dead, &[]));

        // 3. merge_probe_delays：有效备用延迟补全无效主测速结果，但不覆盖已有效的主延迟
        let mut primary = HashMap::new();
        primary.insert("节点A".to_string(), 0u32);
        primary.insert("节点B".to_string(), 95u32);
        primary.insert("节点C".to_string(), 0u32);

        let mut fallback = HashMap::new();
        fallback.insert("节点A".to_string(), 110u32);
        fallback.insert("节点B".to_string(), 0u32);
        fallback.insert("节点D".to_string(), 150u32);

        let merged = merge_probe_delays(primary, fallback);
        assert_eq!(merged.get("节点A").copied(), Some(110));
        assert_eq!(merged.get("节点B").copied(), Some(95));
        assert_eq!(merged.get("节点C").copied(), Some(0));
        assert_eq!(merged.get("节点D").copied(), Some(150));
    }

    #[test]
    fn test_probe_ui_display_and_active_sync_state() {
        let nodes = vec![
            "日本-01".to_string(),
            "官网: https://example.com".to_string(),
            "日本-02".to_string(),
        ];
        let mut delays = HashMap::new();

        // 1. !primary_url_distorted 且全员超时（probed_delays 为空）：依然生成全组 0 (Timeout) 以清掉顶栏假绿
        let all_timeout_display = build_probe_ui_display_with(&nodes, &delays, false, |_| false);
        assert_eq!(
            all_timeout_display,
            vec![("日本-01".to_string(), 0), ("日本-02".to_string(), 0)]
        );

        // 2. primary_url_distorted = true：只推真正取得有效延迟的节点，绝不把未响应节点漆成 0 (Timeout)
        delays.insert("日本-01".to_string(), 118);
        delays.insert("日本-02".to_string(), 0);
        let distorted_display = build_probe_ui_display_with(&nodes, &delays, true, |_| false);
        assert_eq!(distorted_display, vec![("日本-01".to_string(), 118)]);

        // 3. compute_next_ui_active_sync + should_sync_alive_ui：
        // 推送 0 (Timeout) 时即便 Mihomo history.last().time 仍停留在旧成功时刻，也绝不能写入该时间并须置 reported_alive=false；
        // 数秒后免拨测复检恢复 Alive（哪怕 history.time 与 LAST_NOTIFIED_HEALTH 都没变）必须触发回传！
        let timeout_display = vec![("日本-01".to_string(), 0), ("日本-02".to_string(), 92)];
        let after_timeout = compute_next_ui_active_sync(
            "日本-01",
            &timeout_display,
            Some("2026-11-16T12:00:00Z".to_string()),
            None,
        )
        .unwrap();
        assert!(!after_timeout.reported_alive);
        assert_eq!(after_timeout.history_time, None);
        assert!(should_sync_alive_ui(
            false,
            Some(&after_timeout),
            "日本-01",
            Some("2026-11-16T12:00:00Z")
        ));

        // 同步恢复后记为 reported_alive = true，同一时间戳与节点在 health_ok 时幂等跳过，换节点或新时间戳时再次同步
        let ok_display = vec![("日本-01".to_string(), 108)];
        let after_recover = compute_next_ui_active_sync(
            "日本-01",
            &ok_display,
            Some("2026-11-16T12:00:00Z".to_string()),
            Some(&after_timeout),
        )
        .unwrap();
        assert!(after_recover.reported_alive);
        assert_eq!(after_recover.history_time.as_deref(), Some("2026-11-16T12:00:00Z"));
        assert!(!should_sync_alive_ui(
            false,
            Some(&after_recover),
            "日本-01",
            Some("2026-11-16T12:00:00Z")
        ));
        assert!(should_sync_alive_ui(
            false,
            Some(&after_recover),
            "日本-01",
            Some("2026-11-16T12:05:00Z")
        ));
        assert!(should_sync_alive_ui(
            false,
            Some(&after_recover),
            "日本-02",
            Some("2026-11-16T12:00:00Z")
        ));
    }

    #[test]
    fn test_bug388_continuity_and_self_heal_invariants() {
        use super::{
            HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS, LAST_ACTIVE_VERIFY_CHECK, LAST_HTTPS_VERIFY_ALL_FAILED,
            MANUAL_SWITCH_EPOCH, Ordering, is_benign_auto_select_abort, is_https_verify_backoff_active,
            is_manual_switch_preempted, notify_manual_node_switched, reset_monitor_static_state,
            self_heal_subject_to_switch_rate_limit, should_allow_https_migration, should_keep_alive_on_api_recovery,
        };

        // 1. HTTPS 验真全败退避窗口
        assert!(!is_https_verify_backoff_active(None));
        assert!(is_https_verify_backoff_active(Some(0)));
        assert!(is_https_verify_backoff_active(Some(
            HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS - 1
        )));
        assert!(!is_https_verify_backoff_active(Some(
            HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS
        )));

        // 2. Flowing 状态严禁执行活跃节点 HTTPS 验真/迁移
        assert!(!should_allow_https_migration(ActiveTrafficState::Flowing));
        assert!(should_allow_https_migration(ActiveTrafficState::Idle));
        assert!(should_allow_https_migration(ActiveTrafficState::Stalled));

        // 3. ConfirmedDead 豁免切换速率上限，NodeMigration / ApiChannelDown 仍受限
        assert!(!self_heal_subject_to_switch_rate_limit(SelfHealOrigin::ConfirmedDead));
        assert!(self_heal_subject_to_switch_rate_limit(SelfHealOrigin::NodeMigration));
        assert!(self_heal_subject_to_switch_rate_limit(SelfHealOrigin::ApiChannelDown));

        // 4. ApiChannelDown 恢复保活门检
        assert!(should_keep_alive_on_api_recovery(true, NodeStatus::Alive(120)));
        assert!(!should_keep_alive_on_api_recovery(false, NodeStatus::Alive(120)));
        assert!(!should_keep_alive_on_api_recovery(true, NodeStatus::Dead));
        assert!(!should_keep_alive_on_api_recovery(true, NodeStatus::Suspect));
        assert!(!should_keep_alive_on_api_recovery(true, NodeStatus::Unknown));

        // 5. 手动切点代际抢占与良性中断识别、静态状态复位
        assert!(!is_manual_switch_preempted(3, 3));
        assert!(is_manual_switch_preempted(3, 4));
        assert!(is_benign_auto_select_abort("AUTO_SELECT_BUSY"));
        assert!(is_benign_auto_select_abort("AUTO_SELECT_MANUAL_PREEMPTED"));
        assert!(!is_benign_auto_select_abort("OTHER_ERROR"));

        let prev_epoch = MANUAL_SWITCH_EPOCH.load(Ordering::Acquire);
        notify_manual_node_switched();
        assert_eq!(MANUAL_SWITCH_EPOCH.load(Ordering::Acquire), prev_epoch + 1);
        *LAST_ACTIVE_VERIFY_CHECK.lock() = Some(Instant::now());
        *LAST_HTTPS_VERIFY_ALL_FAILED.lock() = Some(Instant::now());
        reset_monitor_static_state();
        assert_eq!(MANUAL_SWITCH_EPOCH.load(Ordering::Acquire), 0);
        assert!(LAST_ACTIVE_VERIFY_CHECK.lock().is_none());
        assert!(LAST_HTTPS_VERIFY_ALL_FAILED.lock().is_none());
    }

    #[test]
    fn test_bug392_select_fastest_candidate_revalidates_latest_filter() {
        use super::select_fastest_candidate_with_filter;

        // 1. 复现现场：陈旧的全量探测结果（源于过滤词 "-" / "-x"）返回时，
        //    最新 filter_text 已提交为 "-香港"：必须当场淘汰更快且已验真的 "香港3 (043311)"，改选 "台湾2 (043311)"
        let stale_probe_candidates = vec![
            ("香港3 (043311)".to_string(), 72),
            ("香港6 (043311)".to_string(), 74),
            ("台湾2 (043311)".to_string(), 99),
        ];
        let picked =
            select_fastest_candidate_with_filter(stale_probe_candidates, "-香港", |name| name.starts_with("香港"));
        assert_eq!(picked, Some(("台湾2 (043311)".to_string(), 99)));

        // 2. 在剩余满足 "-香港" 的候选中，仍优先选择 verified_true 节点
        let mixed_matching = vec![
            ("台湾2 (043311)".to_string(), 99),
            ("台湾1 (043311)".to_string(), 110),
            ("香港3 (043311)".to_string(), 72),
        ];
        let picked_verified =
            select_fastest_candidate_with_filter(mixed_matching, "-香港", |name| name == "台湾1 (043311)");
        assert_eq!(picked_verified, Some(("台湾1 (043311)".to_string(), 110)));

        // 3. 若本批过期测速结果全部不符合最新过滤词，则返回 None 放弃过时切换，避免切入被排除地区
        let only_hk = vec![("香港3 (043311)".to_string(), 72), ("香港6 (043311)".to_string(), 74)];
        let picked_none = select_fastest_candidate_with_filter(only_hk, "-香港", |_| true);
        assert_eq!(picked_none, None);
    }

    #[test]
    fn test_should_close_stale_proxy_connection_bug396() {
        use super::should_close_stale_proxy_connection;

        let hk_chain = vec!["香港1 (181201)".to_string(), "PROXY".to_string()];
        let sg_chain = vec!["新加坡4 (181201)".to_string(), "PROXY".to_string()];
        let jp_chain = vec!["日本3 (181201)".to_string(), "PROXY".to_string()];
        let direct_chain = vec!["DIRECT".to_string()];
        let reject_chain = vec!["REJECT".to_string()];

        // 粘滞在旧香港或多跳前新加坡节点上的代理连接必须立即清理
        assert!(should_close_stale_proxy_connection(
            &hk_chain,
            "PROXY",
            "日本3 (181201)"
        ));
        assert!(should_close_stale_proxy_connection(
            &sg_chain,
            "PROXY",
            "日本3 (181201)"
        ));

        // 已位于当前目标节点上的连接保持不断流
        assert!(!should_close_stale_proxy_connection(
            &jp_chain,
            "PROXY",
            "日本3 (181201)"
        ));

        // 直连与封锁流量绝不触碰
        assert!(!should_close_stale_proxy_connection(
            &direct_chain,
            "PROXY",
            "日本3 (181201)"
        ));
        assert!(!should_close_stale_proxy_connection(
            &reject_chain,
            "PROXY",
            "日本3 (181201)"
        ));

        // 非法或内置目标保护
        assert!(!should_close_stale_proxy_connection(&hk_chain, "PROXY", ""));
        assert!(!should_close_stale_proxy_connection(&hk_chain, "PROXY", "DIRECT"));
        assert!(!should_close_stale_proxy_connection(&hk_chain, "PROXY", "REJECT"));
    }

    #[test]
    fn test_bug400_periodic_https_verify_skips_dead_and_sufficient_standbys() {
        use super::{
            HTTPS_VERIFY_INTERVAL_SECS, PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS,
            should_run_periodic_https_verify_with,
        };

        assert_eq!(HTTPS_VERIFY_INTERVAL_SECS, 600);
        assert_eq!(PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS, 3);

        let members = vec![
            "日本1".to_string(),
            "日本2".to_string(),
            "日本3".to_string(),
            "日本4".to_string(),
            "日本5".to_string(),
            "香港-离线".to_string(),
        ];

        // 1. 仅离线/普通测速未通过节点未验真（is_alive == false）：绝不拖累全组每周期陪跑
        assert!(!should_run_periodic_https_verify_with(
            &members,
            Some("日本1"),
            "",
            |_| false,
            |n| n != "香港-离线",
            |n| n != "香港-离线",
        ));

        // 2. 活跃节点自己 Alive 但尚未验真 → 允许触发
        assert!(should_run_periodic_https_verify_with(
            &members,
            Some("日本1"),
            "",
            |_| false,
            |n| n != "香港-离线",
            |n| n != "日本1" && n != "香港-离线",
        ));

        // 3. 活跃节点已验真，但备用 Alive 已验真仅 2 个（< 3）且有 Alive 未验真节点 → 允许补充验真
        assert!(should_run_periodic_https_verify_with(
            &members,
            Some("日本1"),
            "",
            |_| false,
            |n| n != "香港-离线",
            |n| matches!(n, "日本1" | "日本2" | "日本3"),
        ));

        // 4. 够用即止：活跃节点已验真且匹配子集内已有 >= 3 个已验真 Alive 备用节点（日本2/3/4）→ 跳过全组探测
        assert!(!should_run_periodic_https_verify_with(
            &members,
            Some("日本1"),
            "",
            |_| false,
            |_| true,
            |n| matches!(n, "日本1" | "日本2" | "日本3" | "日本4"),
        ));

        // 5. filter_text 排除范围外的未验真 Alive 节点不得触发当前子集的定期探测
        assert!(!should_run_periodic_https_verify_with(
            &members,
            Some("日本1"),
            "-香港",
            |_| false,
            |_| true,
            |n| !n.contains("香港"),
        ));
    }

    #[test]
    fn test_bug401_single_target_skips_fallback_when_primary_url_healthy() {
        use super::{SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS, should_recheck_primary_with_fallback_with};

        assert_eq!(SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS, 3);

        // 1. 单节点目标超时，但同组已有 >= 3 个真实节点在主 URL 响应 → 证明主 URL 健康，跳过第二轮备用 URL
        let mut primary_healthy = HashMap::new();
        primary_healthy.insert("目标-坏节点".to_string(), 0u32);
        primary_healthy.insert("日本1".to_string(), 80u32);
        primary_healthy.insert("日本2".to_string(), 95u32);
        primary_healthy.insert("日本3".to_string(), 110u32);
        let single_expected = vec!["目标-坏节点".to_string()];
        assert!(!should_recheck_primary_with_fallback_with(
            &primary_healthy,
            &single_expected,
            true,
            |_| false,
        ));

        // 2. 不计 DIRECT / 假节点 / 软隔离节点，若有效真实节点不足 3 个（仅 2 个）仍走备用 URL 复核
        let mut primary_few_peers = HashMap::new();
        primary_few_peers.insert("目标-坏节点".to_string(), 0u32);
        primary_few_peers.insert("DIRECT".to_string(), 50u32);
        primary_few_peers.insert("剩余流量：10GB".to_string(), 60u32);
        primary_few_peers.insert("日本1".to_string(), 80u32);
        primary_few_peers.insert("日本2".to_string(), 95u32);
        assert!(should_recheck_primary_with_fallback_with(
            &primary_few_peers,
            &single_expected,
            true,
            |_| false,
        ));

        // 3. allow_single_target_skip = false（如二次确认判死前复核）仍必须打备用 URL 防失真
        assert!(should_recheck_primary_with_fallback_with(
            &primary_healthy,
            &single_expected,
            false,
            |_| false,
        ));

        // 4. 多节点期望子集全灭时即便其他节点响应仍走备用 URL 复核（保留局部失真防护）
        let multi_expected = vec!["目标A".to_string(), "目标B".to_string()];
        assert!(should_recheck_primary_with_fallback_with(
            &primary_healthy,
            &multi_expected,
            true,
            |_| false,
        ));
    }

    #[test]
    fn test_bug403_recent_flowing_traffic_proves_physical_online() {
        use super::{
            LAST_FLOWING_TRAFFIC_AT, ONLINE_CHECK_FLOWING_SKIP_SECS, has_recent_flowing_traffic,
            reset_monitor_static_state,
        };

        assert!(!has_recent_flowing_traffic(None));
        assert!(has_recent_flowing_traffic(Some(Duration::from_secs(0))));
        assert!(has_recent_flowing_traffic(Some(Duration::from_secs(
            ONLINE_CHECK_FLOWING_SKIP_SECS - 1
        ))));
        assert!(!has_recent_flowing_traffic(Some(Duration::from_secs(
            ONLINE_CHECK_FLOWING_SKIP_SECS
        ))));

        *LAST_FLOWING_TRAFFIC_AT.lock() = Some(Instant::now());
        assert!(LAST_FLOWING_TRAFFIC_AT.lock().is_some());
        reset_monitor_static_state();
        assert!(LAST_FLOWING_TRAFFIC_AT.lock().is_none());
    }

    #[test]
    fn test_bug405_close_excluded_connection_and_migrate_filtered_active() {
        use super::{should_close_excluded_proxy_connection, should_migrate_filtered_out_active};

        let chains_hk = vec!["香港1 (184629)".to_string(), "PROXY".to_string()];
        let chains_sg = vec!["新加坡3 (184629)".to_string(), "PROXY".to_string()];
        let chains_direct = vec!["DIRECT".to_string()];

        // 1. 过滤框排除香港/台湾时：仅断开落在香港节点上的 PROXY 连接，保护新加坡与 DIRECT 连接
        assert!(should_close_excluded_proxy_connection(
            &chains_hk,
            "PROXY",
            "-香港 -台湾"
        ));
        assert!(!should_close_excluded_proxy_connection(
            &chains_sg,
            "PROXY",
            "-香港 -台湾"
        ));
        assert!(!should_close_excluded_proxy_connection(
            &chains_direct,
            "PROXY",
            "-香港 -台湾"
        ));
        // 过滤词为空或半输入态不触发清理
        assert!(!should_close_excluded_proxy_connection(&chains_hk, "PROXY", ""));
        assert!(!should_close_excluded_proxy_connection(&chains_hk, "PROXY", "日本 -"));

        // 2. 活跃节点被过滤词排除且存在合规候选时立即触发迁移；半输入态/无合规候选时不迁移
        let members = vec![
            "香港1 (184629)".to_string(),
            "台湾1 (184629)".to_string(),
            "新加坡3 (184629)".to_string(),
        ];
        assert!(should_migrate_filtered_out_active(
            "香港1 (184629)",
            &members,
            "-香港 -台湾",
            |_| false
        ));
        assert!(!should_migrate_filtered_out_active(
            "新加坡3 (184629)",
            &members,
            "-香港 -台湾",
            |_| false
        ));
        assert!(!should_migrate_filtered_out_active(
            "香港1 (184629)",
            &members,
            "日本 -",
            |_| false
        ));
        let only_blocked = vec!["香港1 (184629)".to_string(), "台湾1 (184629)".to_string()];
        assert!(!should_migrate_filtered_out_active(
            "香港1 (184629)",
            &only_blocked,
            "-香港 -台湾",
            |_| false
        ));
    }
}
