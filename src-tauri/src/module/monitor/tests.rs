#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::auto_select::{
    persisted_exit_is_usable, prioritize_verified_candidates_with, select_fastest_candidate_with_filter,
    sort_display_delays,
};
use super::connections::{
    should_close_dead_node_proxy_connection, should_close_excluded_proxy_connection,
    should_close_stale_proxy_connection,
};
use super::daemon::{SelfHealAction, complete_restart_core_attempt, evaluate_self_heal_counters};
use super::observer::{
    ACTIVE_TRAFFIC_TRACKER, ActiveHealthObservation, ActiveTrafficState, KernelLogEvent, KernelReprobeReason,
    LAST_HEALTH_OBSERVATION, PROXY_DIAL_FAILURE_TRACKER, ProxyDialFailureTracker, classify_active_traffic_delta,
    classify_kernel_log_event, is_proxy_dial_failure_log_line, read_incremental_log_lines, record_kernel_log_line,
    should_probe_active_node,
};
use super::probe::{
    build_probe_ui_display_with, compute_next_ui_active_sync, has_recent_flowing_traffic,
    is_https_verify_backoff_active, is_valid_probe_delay, merge_probe_delays, resolve_probe_target, secondary_test_url,
    should_recheck_primary_with_fallback_with, should_recheck_with_fallback, should_run_periodic_https_verify_with,
    should_sync_alive_ui, try_reuse_probe_snapshot,
};
use super::verdict::{
    NodeStatus, SelfHealDecision, SelfHealOrigin, SelfHealTrigger, auto_switch_permitted,
    can_fast_restart_missing_pipe, classify_node_status, decide_self_heal, is_benign_auto_select_abort,
    is_manual_switch_preempted, record_brownout_episode, self_heal_requires_dead_gate,
    self_heal_subject_to_switch_rate_limit, should_allow_https_migration, should_keep_alive_on_api_recovery,
    should_migrate_filtered_out_active, should_proceed_with_self_heal,
};
use super::{
    API_ERROR_HEAL_THRESHOLD, BROWNOUT_WINDOW_SECS, DEFAULT_TEST_URL, DIAL_FAIL_REPROBE_COOLDOWN_SECS,
    DIAL_FAIL_WINDOW_SECS, DelayResultSource, FAILOVER_CONFIRM_FAILS, FALLBACK_TEST_URL, FORCE_ONLINE_RECHECK,
    HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS, HTTPS_VERIFY_INTERVAL_SECS, IDLE_HEALTH_FRESH_SECS, LAST_ACTIVE_VERIFY_CHECK,
    LAST_FLOWING_TRAFFIC_AT, LAST_HTTPS_VERIFY_ALL_FAILED, LAST_MANUAL_NODE_SWITCH, LAST_METRICS_PROBE,
    LAST_NOTIFIED_HEALTH, LAST_UI_ACTIVE_SYNC, MANUAL_SWITCH_EPOCH, MAX_AUTO_SWITCHES_PER_WINDOW,
    METRICS_REUSE_MAX_AGE_SECS, MIN_AUTO_SWITCH_INTERVAL_SECS, MetricsProbeSnapshot, NODE_SWITCH_NOTIFY,
    ONLINE_CHECK_FLOWING_SKIP_SECS, PASSIVE_TRAFFIC_MAX_SKIP_SECS, PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS,
    SELF_HEAL_COOLDOWN_SECS, SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS, SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS,
    SUSPECT_WINDOW_SECS, notify_manual_node_switched, reset_monitor_static_state,
};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tokio::time::{Duration, Instant};

static STATIC_STATE_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
    // 1. 节点侧故障：第 1 次失败且未达间歇黑洞次数必须等待，达到连续 2 次或窗口内 2 次才放行
    assert_eq!(
        decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails: 1,
            brownout_episodes: 1,
        }),
        SelfHealDecision::Wait
    );
    assert_eq!(
        decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails: FAILOVER_CONFIRM_FAILS,
            brownout_episodes: 1,
        }),
        SelfHealDecision::Allow
    );
    assert_eq!(
        decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails: 5,
            brownout_episodes: 1,
        }),
        SelfHealDecision::Allow
    );
    assert_eq!(
        decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails: 1,
            brownout_episodes: super::BROWNOUT_CONFIRM_FAILS,
        }),
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
    let _serial = STATIC_STATE_TEST_LOCK.lock().await;
    reset_monitor_static_state();
    *LAST_NOTIFIED_HEALTH.lock() = "network_health::all_dead";
    record_kernel_log_line(
        "[2026-11-20 21:28:56.383] time=\"2026-11-20T21:28:56.383087000+08:00\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) 198.18.0.1:8245 --> collector.github.com:443 error: failed to create session: context deadline exceeded\"",
    );
    *LAST_HEALTH_OBSERVATION.lock() = Some(ActiveHealthObservation {
        active_node: "JP-1".to_string(),
        status: NodeStatus::Alive(80),
        traffic_state: ActiveTrafficState::Flowing,
        history_age_secs: Some(10),
        probed_this_round: false,
    });
    FORCE_ONLINE_RECHECK.store(true, Ordering::Release);

    notify_manual_node_switched();
    assert!(LAST_MANUAL_NODE_SWITCH.lock().is_some());
    // 确认 NODE_SWITCH_NOTIFY 已存入许可，select! 可第 0ms 唤醒
    let notified_res = tokio::time::timeout(std::time::Duration::from_millis(20), NODE_SWITCH_NOTIFY.notified()).await;
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
    assert!(PROXY_DIAL_FAILURE_TRACKER.lock().is_none());
    assert!(LAST_HEALTH_OBSERVATION.lock().is_none());
    assert!(!FORCE_ONLINE_RECHECK.load(Ordering::Acquire));
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
        false,
    ));
    assert!(should_probe_active_node(
        NodeStatus::Alive(85),
        Some(Duration::from_secs(IDLE_HEALTH_FRESH_SECS)),
        ActiveTrafficState::Idle,
        false,
        false,
    ));

    // 2. 被动下行流量证活：1799s 内免除全组拨测风暴，满 PASSIVE_TRAFFIC_MAX_SKIP_SECS (1800s) 才复检一次
    assert!(!should_probe_active_node(
        NodeStatus::Alive(85),
        Some(Duration::from_secs(PASSIVE_TRAFFIC_MAX_SKIP_SECS - 1)),
        ActiveTrafficState::Flowing,
        false,
        false,
    ));
    assert!(should_probe_active_node(
        NodeStatus::Alive(85),
        Some(Duration::from_secs(PASSIVE_TRAFFIC_MAX_SKIP_SECS)),
        ActiveTrafficState::Flowing,
        false,
        false,
    ));

    // 3. 黑洞停滞态（有上行无下行）：冷却未激活时立即突破 900s/1800s 免测缓存实拨复核；冷却期内不反复轰炸
    assert!(should_probe_active_node(
        NodeStatus::Alive(85),
        Some(Duration::from_secs(15)),
        ActiveTrafficState::Stalled,
        false,
        false,
    ));
    assert!(!should_probe_active_node(
        NodeStatus::Alive(85),
        Some(Duration::from_secs(15)),
        ActiveTrafficState::Stalled,
        true,
        false,
    ));

    // 3b. 代理建连失败突发（BUG-415）：即使 Flowing 或 Idle 在免测窗内，也必须立即破缓存触发实拨复核
    assert!(should_probe_active_node(
        NodeStatus::Alive(85),
        Some(Duration::from_secs(10)),
        ActiveTrafficState::Flowing,
        false,
        true,
    ));
    assert!(should_probe_active_node(
        NodeStatus::Alive(85),
        Some(Duration::from_secs(10)),
        ActiveTrafficState::Idle,
        false,
        true,
    ));

    // 4. 数据缺失、未知、疑似抖动与已判死一律实拨，连续 2 次失败因此来自两轮独立物理拨测
    assert!(should_probe_active_node(
        NodeStatus::Alive(85),
        None,
        ActiveTrafficState::Flowing,
        false,
        false,
    ));
    assert!(should_probe_active_node(
        NodeStatus::Unknown,
        None,
        ActiveTrafficState::Idle,
        false,
        false,
    ));
    assert!(should_probe_active_node(
        NodeStatus::Suspect,
        Some(Duration::from_secs(1)),
        ActiveTrafficState::Flowing,
        false,
        false,
    ));
    assert!(should_probe_active_node(
        NodeStatus::Dead,
        Some(Duration::from_secs(3)),
        ActiveTrafficState::Flowing,
        false,
        false,
    ));
    assert!(should_probe_active_node(
        NodeStatus::Dead,
        None,
        ActiveTrafficState::Idle,
        false,
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
            false,
            false,
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

    {
        let _serial = STATIC_STATE_TEST_LOCK.blocking_lock();
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
}

#[test]
fn test_bug392_select_fastest_candidate_revalidates_latest_filter() {
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
    use should_close_stale_proxy_connection;

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
    assert!(!has_recent_flowing_traffic(None));
    assert!(has_recent_flowing_traffic(Some(Duration::from_secs(0))));
    assert!(has_recent_flowing_traffic(Some(Duration::from_secs(
        ONLINE_CHECK_FLOWING_SKIP_SECS - 1
    ))));
    assert!(!has_recent_flowing_traffic(Some(Duration::from_secs(
        ONLINE_CHECK_FLOWING_SKIP_SECS
    ))));

    {
        let _serial = STATIC_STATE_TEST_LOCK.blocking_lock();
        *LAST_FLOWING_TRAFFIC_AT.lock() = Some(Instant::now());
        assert!(LAST_FLOWING_TRAFFIC_AT.lock().is_some());
        reset_monitor_static_state();
        assert!(LAST_FLOWING_TRAFFIC_AT.lock().is_none());
    }
}

#[test]
fn test_bug405_close_excluded_connection_and_migrate_filtered_active() {
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

#[test]
fn test_bug411_and_bug412_pipe_missing_brownout_and_dead_conn_cleanup() {
    // 1. BUG-411：缺管快重启门控
    assert!(!can_fast_restart_missing_pipe(false, true, None));
    assert!(!can_fast_restart_missing_pipe(true, false, None));
    assert!(can_fast_restart_missing_pipe(true, true, None));
    assert!(!can_fast_restart_missing_pipe(
        true,
        true,
        Some(SELF_HEAL_COOLDOWN_SECS - 1)
    ));
    assert!(can_fast_restart_missing_pipe(true, true, Some(SELF_HEAL_COOLDOWN_SECS)));

    // 2. BUG-412：间歇性黑洞滑动窗口累计判死
    let t0 = Instant::now();
    let mut episodes = Vec::new();
    assert_eq!(record_brownout_episode(&mut episodes, t0), 1);
    assert_eq!(
        decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails: 1,
            brownout_episodes: episodes.len(),
        }),
        SelfHealDecision::Wait
    );
    // 600s 窗口内第二次独立 ShouldFailover -> 即使第 1 次后曾侥幸恢复，也立即放行切点
    let t1 = t0 + Duration::from_secs(BROWNOUT_WINDOW_SECS - 1);
    assert_eq!(record_brownout_episode(&mut episodes, t1), 2);
    assert_eq!(
        decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails: 1,
            brownout_episodes: episodes.len(),
        }),
        SelfHealDecision::Allow
    );
    // 超过 600s 窗口后旧故障淘汰，仅剩当前 1 次 -> 重新 Wait
    let t2 = t1 + Duration::from_secs(BROWNOUT_WINDOW_SECS);
    assert_eq!(record_brownout_episode(&mut episodes, t2), 1);
    assert_eq!(
        decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
            consecutive_fails: 1,
            brownout_episodes: episodes.len(),
        }),
        SelfHealDecision::Wait
    );

    // 3. BUG-412：自愈切离故障节点时仅精准断开挂在死节点上的 PROXY 连接
    let dead_chain = vec!["日本5 (031306)".to_string(), "PROXY".to_string()];
    let healthy_chain = vec!["日本1 (031306)".to_string(), "PROXY".to_string()];
    let direct_chain = vec!["DIRECT".to_string()];
    let reject_chain = vec!["REJECT".to_string()];
    assert!(should_close_dead_node_proxy_connection(
        &dead_chain,
        "PROXY",
        "日本5 (031306)"
    ));
    assert!(!should_close_dead_node_proxy_connection(
        &healthy_chain,
        "PROXY",
        "日本5 (031306)"
    ));
    assert!(!should_close_dead_node_proxy_connection(
        &direct_chain,
        "PROXY",
        "日本5 (031306)"
    ));
    assert!(!should_close_dead_node_proxy_connection(
        &reject_chain,
        "PROXY",
        "日本5 (031306)"
    ));
    assert!(!should_close_dead_node_proxy_connection(&dead_chain, "PROXY", ""));
    assert!(!should_close_dead_node_proxy_connection(&dead_chain, "PROXY", "DIRECT"));
}

#[test]
fn test_is_proxy_dial_failure_log_line_filters_accurately() {
    use is_proxy_dial_failure_log_line;

    // 命中样本：建连 / 会话 / 握手 / 超时 / 连接重置故障
    assert!(is_proxy_dial_failure_log_line(
        "[2026-11-20 21:28:56.383] time=\"2026-11-20T21:28:56.383087000+08:00\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) 198.18.0.1:8245 --> collector.github.com:443 error: failed to create session: context deadline exceeded\""
    ));
    assert!(is_proxy_dial_failure_log_line(
        "[2026-11-20 03:53:33.882] time=\"2026-11-20T03:53:33.881089000+08:00\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) [fdfe:dcba:9876::1]:3237 --> avatars.githubusercontent.com:443 error: failed to create session: dial tcp 155.254.101.37:40235: i/o timeout\""
    ));
    assert!(is_proxy_dial_failure_log_line(
        "time=\"2026-11-20T03:13:04.213633000+08:00\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) 127.0.0.1:3536 --> www.youtube.com:443 error: dial tcp 174.132.167.252:443: i/o timeout\""
    ));
    assert!(is_proxy_dial_failure_log_line(
        "time=\"2026-11-20T03:13:04Z\" level=warning msg=\"[TCP] dial PROXY (match Match/) a.com:443 error: remote error: tls: handshake failure\""
    ));
    assert!(is_proxy_dial_failure_log_line(
        "time=\"2026-11-20T03:13:04Z\" level=warning msg=\"[UDP] dial PROXY (match RuleSet/gfwlist) a.com:443 error: read: connection reset by peer\""
    ));

    // 排除噪声：DIRECT、PROXY__METRICS、主动取消、DNS 无解析记录、正常 info 日志
    assert!(!is_proxy_dial_failure_log_line(
        "[2026-11-20 03:14:10.408] time=\"2026-11-20T03:14:10.408528400+08:00\" level=warning msg=\"[TCP] dial DIRECT (match Match/) 198.18.0.1:3332 --> 172.217.117.4:443 error: dial tcp 172.217.117.4:443: i/o timeout\""
    ));
    assert!(!is_proxy_dial_failure_log_line(
        "time=\"2026-11-20T03:14:10Z\" level=warning msg=\"[TCP] dial PROXY__METRICS (match Match/) cp.cloudflare.com:443 error: i/o timeout\""
    ));
    assert!(!is_proxy_dial_failure_log_line(
        "time=\"2026-11-20T03:14:10Z\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) a.com:443 error: operation was canceled\""
    ));
    assert!(!is_proxy_dial_failure_log_line(
        "time=\"2026-11-20T03:14:10Z\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) a.com:443 error: dns resolve failed: couldn't find ip\""
    ));
    assert!(!is_proxy_dial_failure_log_line(
        "time=\"2026-11-20T20:59:06Z\" level=info msg=\"Initial configuration complete, total time: 116ms\""
    ));
}

#[test]
fn test_proxy_dial_failure_tracker_window_cooldown_and_node_switch() {
    let mut tracker = ProxyDialFailureTracker::default();
    let base_utc = chrono::Utc::now();
    let base_inst = Instant::now();
    tracker.on_node_checked("日本1", base_utc);
    assert!(tracker.reanchor_to_end);

    let make_line = |ts: chrono::DateTime<chrono::Utc>| {
        format!(
            "time=\"{}\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) 198.18.0.1:8245 --> collector.github.com:443 error: failed to create session: context deadline exceeded\"",
            ts.to_rfc3339()
        )
    };

    // 1. 切换前的旧日志（ts < active_since_utc）与超过 30s 滑窗的老日志必须被忽略
    let before_switch_line = make_line(base_utc - chrono::Duration::seconds(1));
    tracker.ingest_log_line(&before_switch_line, base_inst, base_utc);
    assert_eq!(tracker.evaluate_burst(base_inst), (0, false));

    let expired_line = make_line(base_utc);
    tracker.ingest_log_line(
        &expired_line,
        base_inst + Duration::from_secs(DIAL_FAIL_WINDOW_SECS),
        base_utc + chrono::Duration::seconds(DIAL_FAIL_WINDOW_SECS as i64),
    );
    assert_eq!(
        tracker.evaluate_burst(base_inst + Duration::from_secs(DIAL_FAIL_WINDOW_SECS)),
        (0, false)
    );

    // 2. 滑窗内 1~2 次失败不突破缓存，第 3 次立刻触发 burst_should_reprobe
    let t1_inst = base_inst + Duration::from_secs(40);
    let t1_utc = base_utc + chrono::Duration::seconds(40);
    tracker.ingest_log_line(&make_line(t1_utc), t1_inst, t1_utc);
    tracker.ingest_log_line(&make_line(t1_utc), t1_inst, t1_utc);
    assert_eq!(tracker.evaluate_burst(t1_inst), (2, false));

    tracker.ingest_log_line(&make_line(t1_utc), t1_inst, t1_utc);
    assert_eq!(tracker.evaluate_burst(t1_inst), (3, true));

    // 3. 消费后进入 60s 冷却：冷却期内再来 3 次也不触发，旧事件超过 30s 自动衰减，满 60s 后可重新触发
    tracker.consume_burst_for_probe(t1_inst);
    let mid_inst = t1_inst + Duration::from_secs(20);
    let mid_utc = t1_utc + chrono::Duration::seconds(20);
    for _ in 0..3 {
        tracker.ingest_log_line(&make_line(mid_utc), mid_inst, mid_utc);
    }
    assert_eq!(tracker.evaluate_burst(mid_inst), (3, false));

    let pre_cd_inst = t1_inst + Duration::from_secs(DIAL_FAIL_REPROBE_COOLDOWN_SECS - 1);
    assert_eq!(tracker.evaluate_burst(pre_cd_inst), (0, false));

    let after_cd_inst = t1_inst + Duration::from_secs(DIAL_FAIL_REPROBE_COOLDOWN_SECS);
    let after_cd_utc = t1_utc + chrono::Duration::seconds(DIAL_FAIL_REPROBE_COOLDOWN_SECS as i64);
    for _ in 0..3 {
        tracker.ingest_log_line(&make_line(after_cd_utc), after_cd_inst, after_cd_utc);
    }
    assert_eq!(tracker.evaluate_burst(after_cd_inst), (3, true));

    // 4. 切换活跃节点清空历史计数、冷却并重设 reanchor_to_end
    tracker.reanchor_to_end = false;
    tracker.on_node_checked("新加坡1", after_cd_utc + chrono::Duration::seconds(1));
    assert!(tracker.reanchor_to_end);
    assert_eq!(
        tracker.evaluate_burst(after_cd_inst + Duration::from_secs(1)),
        (0, false)
    );
}

#[test]
fn test_read_incremental_log_lines_handles_anchor_append_partial_and_rotation() {
    use read_incremental_log_lines;
    use std::io::Write as _;

    let unique_name = format!(
        "clash_mini_dial_log_test_{}_{}.log",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let path = std::env::temp_dir().join(unique_name);
    let _ = std::fs::remove_file(&path);

    // 先写入历史旧日志
    std::fs::write(&path, "old-line-1\nold-line-2\n").unwrap();

    let mut cursor_path = None;
    let mut cursor_offset = 0u64;
    let mut reanchor = false;

    // 1. 首次观察锚定到文件末尾，绝不回放历史旧行
    assert!(read_incremental_log_lines(&path, &mut cursor_path, &mut cursor_offset, &mut reanchor).is_empty());
    assert_eq!(cursor_offset, "old-line-1\nold-line-2\n".len() as u64);

    // 2. 追加完整行 + 半行（无换行符）：只返回完整行，半行保留到下次补齐
    {
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"new-line-1\nhalf-").unwrap();
    }
    let batch1 = read_incremental_log_lines(&path, &mut cursor_path, &mut cursor_offset, &mut reanchor);
    assert_eq!(batch1, vec!["new-line-1".to_string()]);

    {
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"line-2\n").unwrap();
    }
    let batch2 = read_incremental_log_lines(&path, &mut cursor_path, &mut cursor_offset, &mut reanchor);
    assert_eq!(batch2, vec!["half-line-2".to_string()]);

    // 3. 日志轮转截断（文件变小）：自动从 0 偏移读取新文件内容
    std::fs::write(&path, "rotated-line-1\n").unwrap();
    let batch3 = read_incremental_log_lines(&path, &mut cursor_path, &mut cursor_offset, &mut reanchor);
    assert_eq!(batch3, vec!["rotated-line-1".to_string()]);

    // 4. 切节点重锚定：reanchor = true 时跳过中间堆积日志直抵末尾
    {
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"pre-switch-line\n").unwrap();
    }
    reanchor = true;
    assert!(read_incremental_log_lines(&path, &mut cursor_path, &mut cursor_offset, &mut reanchor).is_empty());
    assert!(!reanchor);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_bug416_tun_interface_signals_invalidate_online_skip_and_reprobe() {
    assert_eq!(
        classify_kernel_log_event(
            "[2026-11-20 06:51:43.873] time=\"2026-11-20T06:51:43.873+08:00\" level=warning msg=\"[TUN] default interface lost by monitor\""
        ),
        Some(KernelLogEvent::TunInterfaceLost)
    );
    assert_eq!(
        classify_kernel_log_event(
            "[2026-11-20 06:51:47.120] time=\"2026-11-20T06:51:47.120+08:00\" level=info msg=\"[TUN] default interface changed by monitor, Wi-Fi => Ethernet\""
        ),
        Some(KernelLogEvent::TunInterfaceChanged)
    );
    assert_eq!(
        classify_kernel_log_event(
            "time=\"2026-11-20T03:13:04.213633000+08:00\" level=warning msg=\"[TCP] dial PROXY (match RuleSet/gfwlist) 127.0.0.1:3536 --> www.youtube.com:443 error: dial tcp 174.132.167.252:443: i/o timeout\""
        ),
        Some(KernelLogEvent::ProxyDialFailure)
    );
    assert_eq!(
        classify_kernel_log_event("time=\"2026-11-20T20:59:06Z\" level=info msg=\"Initial configuration complete\""),
        None
    );

    let serial = STATIC_STATE_TEST_LOCK.blocking_lock();
    reset_monitor_static_state();
    let base_utc = chrono::Utc::now();
    let base_inst = Instant::now();
    *LAST_FLOWING_TRAFFIC_AT.lock() = Some(base_inst);
    FORCE_ONLINE_RECHECK.store(false, Ordering::Release);

    let mut tracker = ProxyDialFailureTracker::default();
    tracker.on_node_checked("日本1", base_utc);

    let make_lost = |ts: chrono::DateTime<chrono::Utc>| {
        format!(
            "time=\"{}\" level=warning msg=\"[TUN] default interface lost by monitor\"",
            ts.to_rfc3339()
        )
    };
    let make_changed = |ts: chrono::DateTime<chrono::Utc>| {
        format!(
            "time=\"{}\" level=info msg=\"[TUN] default interface changed by monitor, Wi-Fi => Ethernet\"",
            ts.to_rfc3339()
        )
    };

    // 1. 网卡丢失：清零最近下行免测时间戳并置位 FORCE_ONLINE_RECHECK，但不误触发节点切换实拨
    tracker.ingest_log_line(&make_lost(base_utc), base_inst, base_utc);
    assert!(LAST_FLOWING_TRAFFIC_AT.lock().is_none());
    assert!(FORCE_ONLINE_RECHECK.load(Ordering::Acquire));
    assert_eq!(tracker.evaluate_reprobe_signal(base_inst), None);

    // 2. 网卡切换：同时失效物理免测窗口并触发 TunInterfaceChanged 节点复核；消费后受 60s 冷却门控
    *LAST_FLOWING_TRAFFIC_AT.lock() = Some(base_inst);
    FORCE_ONLINE_RECHECK.store(false, Ordering::Release);
    let t1_inst = base_inst + Duration::from_secs(5);
    let t1_utc = base_utc + chrono::Duration::seconds(5);
    tracker.ingest_log_line(&make_changed(t1_utc), t1_inst, t1_utc);
    assert!(LAST_FLOWING_TRAFFIC_AT.lock().is_none());
    assert!(FORCE_ONLINE_RECHECK.load(Ordering::Acquire));
    assert_eq!(
        tracker.evaluate_reprobe_signal(t1_inst),
        Some(KernelReprobeReason::TunInterfaceChanged)
    );

    tracker.consume_burst_for_probe(t1_inst);
    assert_eq!(tracker.evaluate_reprobe_signal(t1_inst), None);

    let mid_inst = t1_inst + Duration::from_secs(20);
    let mid_utc = t1_utc + chrono::Duration::seconds(20);
    tracker.ingest_log_line(&make_changed(mid_utc), mid_inst, mid_utc);
    assert_eq!(tracker.evaluate_reprobe_signal(mid_inst), None);

    let after_cd_inst = t1_inst + Duration::from_secs(DIAL_FAIL_REPROBE_COOLDOWN_SECS);
    let after_cd_utc = t1_utc + chrono::Duration::seconds(DIAL_FAIL_REPROBE_COOLDOWN_SECS as i64);
    tracker.ingest_log_line(&make_changed(after_cd_utc), after_cd_inst, after_cd_utc);
    assert_eq!(
        tracker.evaluate_reprobe_signal(after_cd_inst),
        Some(KernelReprobeReason::TunInterfaceChanged)
    );

    reset_monitor_static_state();
    drop(serial);
}
