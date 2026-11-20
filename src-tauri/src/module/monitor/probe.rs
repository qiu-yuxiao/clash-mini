use super::{
    DEFAULT_TEST_URL, DelayResultSource, FAILED_SERVER_COOLDOWN_SECS, FALLBACK_TEST_URL, FilterConfig,
    HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS, LAST_FLOWING_TRAFFIC_AT, LAST_HTTPS_VERIFY_ALL_FAILED, LAST_METRICS_PROBE,
    METRICS_PROBE_LOCK, METRICS_REUSE_MAX_AGE_SECS, MetricsProbeOutcome, MetricsProbeSnapshot, NODE_DELAY_MAX_MS,
    NODE_DELAY_MIN_MS, NODE_TEST_TIMEOUT_MS, ONLINE_CHECK_FLOWING_SKIP_SECS, PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS,
    PRIMARY_NET_PROBE_TIMEOUT_MS, PROBE_COALESCE_WINDOW_MS, PROFILE_SWITCH_NOTIFY, SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS,
    SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS, UiActiveSyncState, VERIFY_HTTPS_URL_FALLBACK,
    VERIFY_HTTPS_URL_PRIMARY, is_dummy_node,
};
use crate::{config::Config, utils::node::match_node_filter};
use clash_verge_logging::{Type, logging};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use tokio::time::{Duration, Instant, sleep};

/// 纯函数（便于单测锁定，BUG-403）：判断最近一次观测到活跃节点真实下行回包（`Flowing`）是否仍在在线证活窗内
pub(super) const fn has_recent_flowing_traffic(elapsed: Option<Duration>) -> bool {
    matches!(elapsed, Some(d) if d.as_secs() < ONLINE_CHECK_FLOWING_SKIP_SECS)
}

pub(super) fn is_recent_flowing_traffic_active() -> bool {
    has_recent_flowing_traffic(LAST_FLOWING_TRAFFIC_AT.lock().map(|t| t.elapsed()))
}

/// 纯函数（便于单测锁定，BUG-388）：判断当前是否处于真 HTTPS 验真全败退避冷却窗口内
pub(super) const fn is_https_verify_backoff_active(last_failed_elapsed_secs: Option<u64>) -> bool {
    matches!(last_failed_elapsed_secs, Some(secs) if secs < HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS)
}

/// 判断是否需要触发「后台定期真 HTTPS 验真」（BUG-400）：
/// 1. 仅考察匹配当前 `filter_text` 的非伪、非软隔离真实候选节点；
/// 2. 仅当存在至少 1 个当前 `NodeStatus::Alive(_)`（已通过普通测速证实在线）但尚未 `verified_true`
///    的可疑节点时才可能触发，彻底排除长期离线/超时的普通死节点拖累全组反复陪跑 HTTPS 验真；
/// 3. 够用即止：若当前活跃节点已是 `verified_true`，且匹配子集内已有
///    `>= PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS` (3) 个 `Alive` + `verified_true` 备用节点，
///    跳过后台定期全组探测（切点缺候选时仍由「选点前置验真」兜底）。
pub(super) fn should_run_periodic_https_verify_with<FA, FV>(
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
pub(super) async fn run_https_verification_pass(
    mihomo: &tauri_plugin_mihomo::Mihomo,
    candidate_nodes: &[String],
    reason: &str,
) {
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
pub(super) fn record_server_failure(server: &str) {
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
pub(super) fn snapshot_failed_servers() -> HashSet<String> {
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
pub(super) async fn load_node_server_map() -> HashMap<String, String> {
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

pub(super) fn secondary_test_url(primary_url: &str) -> &'static str {
    if primary_url.trim().eq_ignore_ascii_case(FALLBACK_TEST_URL) {
        DEFAULT_TEST_URL
    } else {
        FALLBACK_TEST_URL
    }
}

pub(super) const fn is_valid_probe_delay(delay: u32) -> bool {
    delay >= NODE_DELAY_MIN_MS && delay < NODE_DELAY_MAX_MS
}

/// 构造后台体检 `probe_metrics_group` 同步到前端展示的 `(节点名, 延迟)` 列表（BUG-387）。
///
/// - `!primary_url_distorted`：即便 `probed_delays` 为空（全员超时 `Ok({})`）也为全部可见真实节点生成 `0 (Timeout)`，
///   避免活跃节点已断流时前端顶栏与列表残留断流前的假绿延迟；
/// - `primary_url_distorted == true`：主测速 URL 全局失真时仅输出备用 URL 真正测得的有效延迟，
///   严禁把未响应节点批量漆成 `0 (Timeout)` 写入前端 30 分钟长缓存。
pub(super) fn build_probe_ui_display_with<F>(
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

pub(super) fn build_probe_ui_display(
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
pub(super) fn compute_next_ui_active_sync(
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
pub(super) fn should_sync_alive_ui(
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

pub(super) fn should_recheck_with_fallback(primary_delays: &HashMap<String, u32>, expected_nodes: &[String]) -> bool {
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
pub(super) fn should_recheck_primary_with_fallback_with<F>(
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

pub(super) fn should_recheck_primary_with_fallback(
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

pub(super) fn merge_probe_delays(
    mut primary: HashMap<String, u32>,
    fallback: HashMap<String, u32>,
) -> HashMap<String, u32> {
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
pub(super) fn try_reuse_probe_snapshot(
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
pub(super) async fn probe_metrics_group(
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
pub(super) fn resolve_probe_target(test_url: &str) -> String {
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
pub(super) async fn is_physical_network_online(timeout_secs: u64) -> bool {
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
pub(super) async fn is_clash_api_alive() -> bool {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    tokio::time::timeout(Duration::from_secs(3), mihomo.get_base_config())
        .await
        .map(|res| res.is_ok())
        .unwrap_or(false)
}

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
