use super::connections::persist_selected_node_if_matching;
use super::probe::{
    compute_next_ui_active_sync, get_current_profile_uid, get_filter_and_sort_config, is_valid_probe_delay,
    load_node_server_map, probe_metrics_group, run_https_verification_pass, snapshot_failed_servers,
    try_reuse_probe_snapshot, wait_for_clash_ready,
};
use super::verdict::{history_age, is_manual_switch_preempted};
use super::{
    AUTO_SELECT_RUNNING, AutoSelectOutcome, DelayResultSource, LAST_METRICS_PROBE, LAST_UI_ACTIVE_SYNC,
    MANUAL_SWITCH_EPOCH, METRICS_REUSE_MAX_AGE_SECS, NODE_DELAY_MAX_MS, NODE_DELAY_MIN_MS, NORMAL_CHECK_INTERVAL_SECS,
    OFFLINE_CHECK_INTERVAL_SECS, RETRY_CHECK_INTERVAL_SECS, emit_network_health_notice, is_dummy_node,
};
use crate::utils::node::match_node_filter;
use clash_verge_logging::{Type, logging};
use std::sync::atomic::Ordering;
use tokio::time::{Duration, Instant};

pub(super) fn prioritize_verified_candidates_with<F>(
    candidates: Vec<(String, u32)>,
    is_verified: F,
) -> Vec<(String, u32)>
where
    F: Fn(&str) -> bool,
{
    if candidates.iter().any(|(name, _)| is_verified(name)) {
        candidates.into_iter().filter(|(name, _)| is_verified(name)).collect()
    } else {
        candidates
    }
}

pub(super) fn select_fastest_candidate_with_filter<F>(
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

pub(super) fn select_fastest_candidate(
    candidates: Vec<(String, u32)>,
    latest_filter_text: &str,
) -> Option<(String, u32)> {
    select_fastest_candidate_with_filter(
        candidates,
        latest_filter_text,
        crate::utils::node::is_verified_true_node,
    )
}

/// 纯判定部分（便于单测锁定「什么才算结构性有效」）：只做结构性校验，
/// 不读 `alive` / `history`，也不发起任何网络调用。
pub(super) fn persisted_exit_is_usable(
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
pub(super) async fn has_usable_persisted_exit(profile_uid: &str) -> bool {
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

pub(super) async fn trigger_backend_auto_select_with_options(
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
    // 2. 若无进程内近期快照且属于前端批量子集（len > 1），再查 Mihomo 测量组近期 history；
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
    let total_candidates = candidates.len();

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
    } else {
        let active_node = get_active_node_name().await;
        let active_delay = active_node.as_deref().and_then(|n| delays.get(n).copied());
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 纯测速刷新完成（不切换节点）：待测 {} 个，可达 {} 个，source={:?}，当前节点={:?}（延迟={:?}ms）",
            valid_nodes.len(),
            total_candidates,
            source,
            active_node,
            active_delay,
        );
    }

    Ok(AutoSelectOutcome {
        display,
        selected,
        selected_node,
        source,
        covers_all_group_nodes,
    })
}

pub(super) async fn get_active_node_name() -> Option<String> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let group_info = mihomo.get_group_by_name("PROXY").await.ok()?;
    group_info.now.filter(|node| !node.is_empty())
}

/// 计算下一次健康检测的等待间隔（秒）：重试模式 3 秒、离线 5 秒、正常 15 秒，与唤醒周期对齐。
pub(super) const fn check_interval_secs(is_retry_mode: bool, was_online: bool) -> u64 {
    if is_retry_mode {
        RETRY_CHECK_INTERVAL_SECS
    } else if !was_online {
        OFFLINE_CHECK_INTERVAL_SECS
    } else {
        NORMAL_CHECK_INTERVAL_SECS
    }
}

pub(super) fn sort_display_delays(display: &mut [(String, u32)], sort_type: i32) {
    match sort_type {
        2 => display.sort_by(|a, b| a.0.cmp(&b.0)),
        _ => display.sort_by_key(|r| if r.1 == 0 { u32::MAX } else { r.1 }),
    }
}
