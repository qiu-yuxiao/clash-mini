use super::observer::{
    ACTIVE_TRAFFIC_TRACKER, ActiveHealthObservation, ActiveTrafficState, KernelReprobeReason, LAST_HEALTH_OBSERVATION,
    PROXY_DIAL_FAILURE_TRACKER, poll_active_node_dial_failures, sample_active_node_traffic, should_probe_active_node,
};
use super::probe::{
    build_probe_ui_display, compute_next_ui_active_sync, get_filter_and_sort_config, probe_metrics_group,
    run_https_verification_pass, should_sync_alive_ui,
};
use super::{
    ACTIVE_VERIFY_COOLDOWN_SECS, API_ERROR_HEAL_THRESHOLD, BROWNOUT_CONFIRM_FAILS, BROWNOUT_WINDOW_SECS,
    FAILOVER_CONFIRM_FAILS, LAST_ACTIVE_VERIFY_CHECK, LAST_METRICS_PROBE, LAST_NOTIFIED_HEALTH, LAST_UI_ACTIVE_SYNC,
    MAX_AUTO_SWITCHES_PER_WINDOW, MIN_AUTO_SWITCH_INTERVAL_SECS, NODE_DELAY_MAX_MS, NODE_DELAY_MIN_MS,
    PASSIVE_TRAFFIC_MAX_SKIP_SECS, SELF_HEAL_COOLDOWN_SECS, SUSPECT_WINDOW_SECS, UiActiveSyncState, is_dummy_node,
};
use crate::utils::node::match_node_filter;
use clash_verge_logging::{Type, logging};
use tokio::time::{Duration, Instant};

/// 单节点健康状态（由测量组维护的 history/alive 判定，TUN 下可靠）。
/// S4 起由二值扩为三态：`Suspect` 吸收单次波动，仅 `Dead` 构成自愈证据。
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

/// 单节点健康三态分类（时间戳不可解析时保守判 `Dead`，防长期坏节点卡在门外）。
pub(super) fn classify_node_status(alive: bool, last_delay: Option<u32>, age_secs: Option<u64>) -> NodeStatus {
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
pub(super) const fn should_proceed_with_self_heal(dead_status: NodeStatus) -> bool {
    matches!(dead_status, NodeStatus::Dead)
}

/// 自愈切换速率上限（S4，纯函数）。
/// `last_switch_age_secs` 为距上次实际切换的秒数；`recent_switch_count` 为
/// `AUTO_SWITCH_WINDOW_SECS` 滑动窗口内已发生的实际切换次数（由调用方裁剪）。
pub(super) fn auto_switch_permitted(last_switch_age_secs: Option<u64>, recent_switch_count: usize) -> bool {
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
pub(super) enum SelfHealOrigin {
    /// 节点侧故障转移：原节点经连续 2 次独立体检确认已死，必须过门检
    ConfirmedDead,
    /// 不可信节点迁移：原节点仍可用但假/验真失败，门检不适用
    NodeMigration,
    /// 内核 API 通道故障：无原节点信息，沿用 BUG-318 重启/告警链路
    ApiChannelDown,
}

/// 门检适用范围（纯函数，便于单测锁定）：仅节点侧故障转移需要「原节点确已死」复核。
pub(super) const fn self_heal_requires_dead_gate(origin: SelfHealOrigin) -> bool {
    matches!(origin, SelfHealOrigin::ConfirmedDead)
}

/// 切换速率限制适用范围（BUG-388，纯函数便于单测锁定）：
/// 已通过连续体检与「原节点确已死」复核证实的 `ConfirmedDead` 死链必须允许立即抢修，不能被 180s 速率上限锁死在死节点；
/// `NodeMigration`（原节点仍可用时的不可信迁移）与 `ApiChannelDown` 仍受速率上限约束。
pub(super) const fn self_heal_subject_to_switch_rate_limit(origin: SelfHealOrigin) -> bool {
    !matches!(origin, SelfHealOrigin::ConfirmedDead)
}

/// 连续性优先（BUG-388，纯函数便于单测锁定）：活跃节点存在真实下行流量（`Flowing`）时，
/// 禁止以“未通过真 HTTPS 验真”为由执行后台切换或即时全组 HTTPS 验真探测。
pub(super) const fn should_allow_https_migration(traffic_state: ActiveTrafficState) -> bool {
    !matches!(traffic_state, ActiveTrafficState::Flowing)
}

/// `ApiChannelDown` 恢复保活门检（BUG-388，纯函数便于单测锁定）：
/// 当 API 已可读且当前活跃真实节点健康（`Alive(_)`）时，遵循连续性优先直接保活现有出口，跳过全组择优切点。
pub(super) const fn should_keep_alive_on_api_recovery(is_valid_real_node: bool, status: NodeStatus) -> bool {
    is_valid_real_node && matches!(status, NodeStatus::Alive(_))
}

/// 手动切点抢占判定（BUG-388，纯函数便于单测锁定）：比较选点开始与切换落盘前代际号。
pub(super) const fn is_manual_switch_preempted(start_epoch: u64, current_epoch: u64) -> bool {
    current_epoch != start_epoch
}

/// 识别属于并发/抢占让路而非真实故障的选点中断错误（BUG-388）。
pub(super) fn is_benign_auto_select_abort(err_str: &str) -> bool {
    matches!(err_str, "AUTO_SELECT_BUSY" | "AUTO_SELECT_MANUAL_PREEMPTED")
}

/// 后台监测所有自愈请求的统一裁决输入：仅 `SelfHealDecision::Allow` 允许触发自愈选点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelfHealTrigger {
    /// 主路径：连续独立体检确认活跃节点失效，或滑动窗口内反复触发间歇黑洞（BUG-412）
    NodeConfirmedDead {
        consecutive_fails: u32,
        brownout_episodes: usize,
    },
    /// 物理网络离线转在线：只代表链路恢复，不构成节点故障证据
    NetworkRecovered,
    /// 内核 API 通道连续异常；`api_alive_on_recheck` 为阈值触发后的 API 存活复核结果
    ApiUnreachable {
        api_error_count: u32,
        api_alive_on_recheck: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelfHealDecision {
    Allow,
    Wait,
}

/// 判断主测速地址失真轮次是否真正证明了当前活跃节点本身可用（BUG-417）：
/// 仅当主地址整体失败但当前活跃节点自身在备用测速地址下取得有效延迟时，才免于判死；
/// 若仅其他节点通过备用地址而当前活跃节点仍超时无响应，禁止由其它节点“替考”豁免。
pub(super) fn is_active_excused_by_primary_distortion(
    primary_url_distorted: bool,
    active_node: &str,
    probed_delays: &std::collections::HashMap<String, u32>,
) -> bool {
    primary_url_distorted
        && probed_delays
            .get(active_node)
            .copied()
            .is_some_and(super::probe::is_valid_probe_delay)
}

/// 统一裁决器：节点侧需连续 `FAILOVER_CONFIRM_FAILS` 次失败或 600s 窗口内 `BROWNOUT_CONFIRM_FAILS` 次间歇故障；
/// 网络恢复恒 `Wait`；API 异常须经存活复核仍失联。
pub(super) const fn decide_self_heal(trigger: SelfHealTrigger) -> SelfHealDecision {
    match trigger {
        SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails,
            brownout_episodes,
        } => {
            if consecutive_fails >= FAILOVER_CONFIRM_FAILS || brownout_episodes >= BROWNOUT_CONFIRM_FAILS {
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

/// 纯判定（BUG-411）：满足「管道不存在 + 期望内核存活 + 超出冷却窗」才允许跳过 3 次慢累计立即重启内核。
pub(super) const fn can_fast_restart_missing_pipe(
    pipe_missing: bool,
    expect_alive: bool,
    last_attempt_elapsed_secs: Option<u64>,
) -> bool {
    pipe_missing && expect_alive && !matches!(last_attempt_elapsed_secs, Some(secs) if secs < SELF_HEAL_COOLDOWN_SECS)
}

pub(super) fn prune_brownout_episodes(episodes: &mut Vec<Instant>, now: Instant) {
    episodes.retain(|t| now.saturating_duration_since(*t).as_secs() < BROWNOUT_WINDOW_SECS);
}

pub(super) fn record_brownout_episode(episodes: &mut Vec<Instant>, now: Instant) -> usize {
    prune_brownout_episodes(episodes, now);
    episodes.push(now);
    episodes.len()
}

/// 依据测量组（PROXY__METRICS，url-test）维护的 history/alive 判定节点状态。
/// 这是唯一权威健康来源：由 Mihomo 内核周期性直接拨测，TUN 下可靠，
/// 彻底取代旧设计「后端用 delay_proxy_by_name 自测活跃节点」的不可靠路径。
pub(super) fn node_status(proxy: &tauri_plugin_mihomo::models::Proxy) -> NodeStatus {
    // 测量组尚未测过该节点 → 数据缺失，不误判
    let last_delay = proxy.history.last().map(|h| h.delay as u32);
    // 用同一份 history 时间戳判断「刚失败」还是「长期不可用」（S4 三态）
    let age_secs = history_age(proxy).map(|age| age.as_secs());
    classify_node_status(proxy.alive, last_delay, age_secs)
}

/// 节点上次拨测距现在的时长（基于 history 中 RFC3339 时间戳）。
/// 无论拨测由谁触发（后台体检 / 选点 / 显式 delay_group）都会刷新该时间，
/// 因此它是「缓存数据是否新鲜」的权威依据。无 history 或时间解析失败返回 None。
pub(super) fn history_age(proxy: &tauri_plugin_mihomo::models::Proxy) -> Option<Duration> {
    let time_str = &proxy.history.last()?.time;
    let measured_at = chrono::DateTime::parse_from_rfc3339(time_str).ok()?;
    let age_secs = chrono::Utc::now()
        .signed_duration_since(measured_at.with_timezone(&chrono::Utc))
        .num_seconds();
    // 时钟偏差导致时间在未来时按 0（刚测过）处理
    Some(Duration::from_secs(age_secs.max(0) as u64))
}

/// 纯判定：当前活跃出口是否在有效 `filter_text` 下已被排除，且组内至少存在 1 个符合过滤条件的真实候选（BUG-405）。
pub(super) fn should_migrate_filtered_out_active<F>(
    active_node: &str,
    members: &[String],
    filter_text: &str,
    is_fake: F,
) -> bool
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
pub(super) async fn evaluate_failover(profile_uid: &str) -> anyhow::Result<FailoverVerdict> {
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
    let reprobe_signal = poll_active_node_dial_failures(&active_node).await;

    let mut proxies = mihomo
        .get_proxies()
        .await
        .map_err(|e| anyhow::anyhow!("获取代理列表失败: {e}"))?;

    // 【BUG-377 + BUG-379 + BUG-415 + BUG-416】只有「健康、无建连失败突发/网卡切换待复核且数据在对应流量态保鲜窗口内」才允许免拨测；
    // 任何非 Alive 状态、单向停滞态、代理建连失败突发或默认网卡切换都必须先取得本轮主 URL 实拨证据，再重读 alive / history 裁决。
    let need_probe = match proxies.proxies.get(&active_node) {
        None => false, // 活跃节点不在代理列表（异常），不动作
        Some(p) => should_probe_active_node(
            node_status(p),
            history_age(p),
            traffic_state,
            stall_cooldown_active,
            reprobe_signal.is_some(),
        ),
    };
    if need_probe && let Some(signal) = reprobe_signal {
        if let Some(tracker) = PROXY_DIAL_FAILURE_TRACKER.lock().as_mut() {
            tracker.consume_burst_for_probe(Instant::now());
        }
        match signal {
            KernelReprobeReason::DialFailureBurst(dial_fail_count) => logging!(
                info,
                Type::Lightweight,
                "[后台监测] 检测到活跃节点 {} 近期出现 {} 次代理建连失败告警，突破免测缓存立即触发体检复核",
                active_node,
                dial_fail_count
            ),
            KernelReprobeReason::TunInterfaceChanged => logging!(
                info,
                Type::Lightweight,
                "[后台监测] 检测到物理默认网卡切换，突破免测缓存立即复核当前节点 {}",
                active_node
            ),
        }
    }
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

    // 本轮体检是否得出「主测速地址失真、活跃节点自身正常」及原始失真标记。
    let mut primary_url_distorted = false;
    let mut active_excused_by_distortion = false;

    if need_probe {
        // 经测量组并发直拨全员（TUN 下无自指回环），禁用短窗合并；单目标初检与复核在同组已有 >= 3 个真实节点
        // 通过主 URL 时跳过备用 URL，防止间歇性黑洞节点借第 2 次备用地址压线逃生（BUG-401 / BUG-412）
        let probed_opt = probe_metrics_group(&mihomo, "体检", false, std::slice::from_ref(&active_node), true).await;

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
        active_excused_by_distortion =
            is_active_excused_by_primary_distortion(primary_url_distorted, &active_node, &probed_delays);
        if probed_delays.is_empty() && !primary_url_distorted {
            *LAST_METRICS_PROBE.lock() = None;
        }

        // 【BUG-369 / BUG-387 / BUG-417 修复】后台定期体检刚刚完成了一次真实的整组延迟探测：
        // - `!primary_url_distorted` 时即便 `probed_delays` 为空（全员超时 `{}`）也向前端推送全组 `0 (Timeout)`；
        // - `primary_url_distorted` 时仅同步真正测得有效延迟的节点，避免把未响应普通节点漆成 30 分钟 `0`；
        //   但若当前活跃节点本身主/备双超时（`!active_excused_by_distortion`），需显式推送 `(active_node, 0)` 清除顶部假绿。
        let mut ui_synced_display: Option<Vec<(String, u32)>> = None;
        if !crate::module::lightweight::is_in_lightweight_mode() {
            let already_reported = !probed_delays.is_empty()
                && LAST_METRICS_PROBE
                    .lock()
                    .as_ref()
                    .is_some_and(|s| s.reported_live_to_ui);
            if !already_reported {
                let mut display = build_probe_ui_display(
                    group_info.all.as_deref().unwrap_or(&[]),
                    &probed_delays,
                    primary_url_distorted,
                );
                if primary_url_distorted
                    && !active_excused_by_distortion
                    && !display.iter().any(|(n, _)| n == &active_node)
                    && !is_dummy_node(&active_node)
                    && !crate::utils::node::is_confirmed_fake_node(&active_node)
                {
                    display.push((active_node.clone(), 0));
                }
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
        // 而非此前遗留的陈旧缓存（PROXY__METRICS 已配置为 interval: 1800 + lazy: true，不会自行周期拨测）。
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
        // （Mihomo 超时不写 history，若仅比时间戳会静默漏推）、或内核 PROXY__METRICS 被动刷新
        // （注：已配置为 interval: 1800 + lazy: true，主要由本监测器按需拨测触发）更新出可用结果时，
        // 同步向前端推送可用延迟与 refresh_proxies()，彻底打通“断流报 Timeout、恢复却静默”的同步断层。
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

    if let Some(p) = proxies.proxies.get(&active_node) {
        *LAST_HEALTH_OBSERVATION.lock() = Some(ActiveHealthObservation {
            active_node: active_node.clone(),
            status: node_status(p),
            traffic_state,
            history_age_secs: history_age(p).map(|d| d.as_secs()),
            probed_this_round: need_probe,
        });
    }

    // 连续性优先：仅当活跃节点自身也通过备用测速地址时，才免于判死并返回 NoAction；
    // 若仅同组其他节点通过备用地址而活跃节点仍超时，则继续按正常状态裁决（BUG-417）。
    if active_excused_by_distortion {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 备用测速地址已证明当前活跃节点 {} 正常 → 本轮返回 NoAction（不累计失败、不自愈、不切点）",
            active_node
        );
        return Ok(FailoverVerdict::NoAction);
    }
    if primary_url_distorted {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 主测速地址失真，但当前活跃节点 {} 在备用测速地址下仍超时无响应，继续执行故障裁决",
            active_node
        );
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
