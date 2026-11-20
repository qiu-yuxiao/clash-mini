use super::auto_select::{
    check_interval_secs, get_active_node_name, has_usable_persisted_exit, trigger_backend_auto_select,
    trigger_backend_auto_select_with_options,
};
use super::connections::{close_dead_node_proxy_connections, close_excluded_proxy_connections};
use super::observer::{LAST_HEALTH_OBSERVATION, sync_service_kernel_logs};
use super::probe::{
    get_current_profile_uid, get_filter_and_sort_config, is_clash_api_alive, is_physical_network_online,
    is_recent_flowing_traffic_active, load_node_server_map, record_server_failure, restore_profile_selected_nodes,
    run_https_verification_pass, should_run_periodic_https_verify_with, wait_for_clash_ready,
};
use super::verdict::{
    FailoverVerdict, NodeStatus, SelfHealDecision, SelfHealOrigin, SelfHealTrigger, auto_switch_permitted,
    can_fast_restart_missing_pipe, decide_self_heal, evaluate_failover, is_benign_auto_select_abort, node_status,
    prune_brownout_episodes, record_brownout_episode, self_heal_requires_dead_gate,
    self_heal_subject_to_switch_rate_limit, should_keep_alive_on_api_recovery, should_proceed_with_self_heal,
};
use super::{
    ALERT_DIALOG_OPEN, API_ERROR_HEAL_THRESHOLD, AUTO_SWITCH_WINDOW_SECS, AutoSelectOutcome, FORCE_ONLINE_RECHECK,
    HTTPS_VERIFY_INTERVAL_SECS, LAST_MANUAL_NODE_SWITCH, MAX_AUTO_SWITCHES_PER_WINDOW, MIN_AUTO_SWITCH_INTERVAL_SECS,
    MONITOR_HEARTBEAT_CYCLE_COUNT, MONITOR_SUMMARY_INTERVAL_SECS, MONITOR_TASK_HANDLE, MONITOR_WAKEUP_NOTIFY,
    NODE_SWITCH_NOTIFY, NORMAL_CHECK_INTERVAL_SECS, PIPE_MISSING_CONFIRM_DELAY_MS, PROFILE_SWITCH_NOTIFY,
    SELF_HEAL_COOLDOWN_SECS, SWITCH_GRACE_PERIOD_SECS, emit_network_health_notice, is_dummy_node,
    reset_monitor_static_state,
};
use crate::process::AsyncHandler;
use clash_verge_logging::{Type, logging};
use std::sync::atomic::Ordering;
use tokio::time::{Duration, Instant, sleep};

/// 二次复核当前内核命名管道确实不存在且非主动停启/热更窗口（BUG-411）。
async fn is_mihomo_pipe_missing_confirmed() -> bool {
    let first_expect = crate::core::manager::CoreManager::global().should_expect_core_pipe_alive();
    let first_missing = crate::core::handle::Handle::mihomo().await.is_local_socket_missing();
    if !first_expect || !first_missing {
        return false;
    }
    sleep(Duration::from_millis(PIPE_MISSING_CONFIRM_DELAY_MS)).await;
    let second_expect = crate::core::manager::CoreManager::global().should_expect_core_pipe_alive();
    let second_missing = crate::core::handle::Handle::mihomo().await.is_local_socket_missing();
    second_expect && second_missing
}

/// 当内核进程异常退出（`os error 2` 管道不存在）时，跳过 3 轮慢轮询与 30s 空等立即重启内核（BUG-411）。
async fn try_fast_restart_missing_pipe(
    api_error_count: &mut u32,
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
    last_auto_select_time: &mut Option<Instant>,
    last_check_time: &mut Instant,
) -> bool {
    let pipe_missing = crate::core::handle::Handle::mihomo().await.is_local_socket_missing();
    let expect_alive = crate::core::manager::CoreManager::global().should_expect_core_pipe_alive();
    let elapsed = last_auto_select_time.map(|t| t.elapsed().as_secs());
    if !can_fast_restart_missing_pipe(pipe_missing, expect_alive, elapsed) {
        return false;
    }
    if !is_mihomo_pipe_missing_confirmed().await {
        return false;
    }

    *api_error_count = 0;
    *last_auto_select_time = Some(Instant::now());
    *last_check_time = Instant::now();
    *auto_select_fail_count += 1;
    if *auto_select_fail_count >= 5 {
        *auto_select_fail_count = 0;
        *api_dead_heal_fail_count = 0;
        fire_self_heal_alert();
    } else {
        execute_core_restart_with_accounting(
            "[后台监测] 检测到内核命名管道已不存在（进程退出），跳过轮询空等立即重启内核",
            auto_select_fail_count,
            api_dead_heal_fail_count,
        )
        .await;
    }
    true
}

/// 统一处理后台监测循环中的 API 不可达异常：缺管快速重启、累计次数、达阈值后复核存活并按裁决自愈。
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
    if try_fast_restart_missing_pipe(
        api_error_count,
        auto_select_fail_count,
        api_dead_heal_fail_count,
        last_auto_select_time,
        last_check_time,
    )
    .await
    {
        return;
    }

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
        let mut last_summary_log_time = Instant::now();
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
        // BUG-412：同一活跃节点在 600s 滑动窗口内触发独立 `ShouldFailover` 故障周期的时间戳
        let mut recent_brownout_episodes: Vec<Instant> = Vec::new();
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
                recent_brownout_episodes.clear();
            }

            let current_profile = match get_current_profile_uid().await {
                Some(uid) => uid,
                None => {
                    continue;
                }
            };

            // 同步增量读取内核日志中的网卡事件（BUG-416），使 `[TUN] default interface lost/changed` 立即跳过 60s 在线免测
            sync_service_kernel_logs().await;
            let force_online_recheck = FORCE_ONLINE_RECHECK.swap(false, Ordering::AcqRel);

            // 检测物理网络连通性状态（有节流门控，避免每次循环都发起 DNS 查询）
            let probe_interval = if was_online {
                Duration::from_secs(60) // 在线时：60 秒探测一次
            } else {
                Duration::from_secs(5) // 离线时：5 秒探测一次，快速发现网络恢复
            };
            let need_probe = force_online_recheck
                || last_online_check_time
                    .map(|t| t.elapsed() >= probe_interval)
                    .unwrap_or(true); // 首次循环一定探测

            let is_online = if need_probe {
                last_online_check_time = Some(Instant::now());
                if !force_online_recheck && was_online && is_recent_flowing_traffic_active() {
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
                recent_brownout_episodes.clear();

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
                    if last_summary_log_time.elapsed() >= Duration::from_secs(MONITOR_SUMMARY_INTERVAL_SECS) {
                        last_summary_log_time = Instant::now();
                        logging!(
                            info,
                            Type::Lightweight,
                            "[后台监测] 周期状态摘要: online=false, active={:?}, uptime={}s",
                            last_active_node,
                            thread_start_time.elapsed().as_secs()
                        );
                    }
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
                    recent_brownout_episodes.clear();
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
                    recent_brownout_episodes.clear();
                }

                match evaluate_failover(&current_profile).await {
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
                                auto_select_fail_count = 0;
                                api_dead_heal_fail_count = 0;
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
                                    recent_brownout_episodes.clear();
                                } else {
                                    consecutive_fails += 1;
                                    is_retry_mode = true;
                                    let now = Instant::now();
                                    let brownout_episodes = if consecutive_fails == 1
                                        && matches!(verdict, FailoverVerdict::ShouldFailover)
                                    {
                                        record_brownout_episode(&mut recent_brownout_episodes, now)
                                    } else {
                                        prune_brownout_episodes(&mut recent_brownout_episodes, now);
                                        recent_brownout_episodes.len()
                                    };
                                    logging!(
                                        info,
                                        Type::Lightweight,
                                        "[后台监测] 活跃节点检测异常 ({:?})，连续失败次数: {}，窗口内间歇故障次数: {}",
                                        verdict,
                                        consecutive_fails,
                                        brownout_episodes
                                    );

                                    if decide_self_heal(SelfHealTrigger::NodeConfirmedDead {
                                        consecutive_fails,
                                        brownout_episodes,
                                    }) == SelfHealDecision::Allow
                                    {
                                        consecutive_fails = 0;
                                        is_retry_mode = false;
                                        recent_brownout_episodes.clear();

                                        logging!(
                                            info,
                                            Type::Lightweight,
                                            "[后台监测] 满足故障转移条件（连续/间歇黑洞阈值已达），启动后台自愈选点"
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

                if last_summary_log_time.elapsed() >= Duration::from_secs(MONITOR_SUMMARY_INTERVAL_SECS) {
                    last_summary_log_time = Instant::now();
                    let uptime = thread_start_time.elapsed().as_secs();
                    let obs_snapshot = LAST_HEALTH_OBSERVATION.lock().clone();
                    if let Some(obs) = obs_snapshot {
                        logging!(
                            info,
                            Type::Lightweight,
                            "[后台监测] 周期状态摘要: node={}, status={:?}, traffic={:?}, last_probe_age={:?}s, probed={}, fails={}, brownouts={}, uptime={}s",
                            obs.active_node,
                            obs.status,
                            obs.traffic_state,
                            obs.history_age_secs,
                            obs.probed_this_round,
                            consecutive_fails,
                            recent_brownout_episodes.len(),
                            uptime
                        );
                    } else {
                        logging!(
                            info,
                            Type::Lightweight,
                            "[后台监测] 周期状态摘要: active={:?}, fails={}, api_errors={}, online={}, uptime={}s",
                            last_active_node,
                            consecutive_fails,
                            api_error_count,
                            was_online,
                            uptime
                        );
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
        scopeguard::defer! {
            ALERT_DIALOG_OPEN.store(false, Ordering::Release);
        }
        crate::show_error_dialog("Clash Mini - 网络警报", body);
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
pub(super) enum SelfHealAction {
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
pub(super) const fn evaluate_self_heal_counters(
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
pub(super) const fn complete_restart_core_attempt(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiChannelPrecheck {
    KeepAlive,
    ProceedAutoSelect,
    ConfirmedDown,
}

async fn precheck_api_channel_down(
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
    last_check_time: &mut Instant,
) -> ApiChannelPrecheck {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    match mihomo.get_group_by_name("PROXY").await {
        Ok(group_info) => {
            if let Ok(proxies) = mihomo.get_proxies().await
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
                    return ApiChannelPrecheck::KeepAlive;
                }
            }
            ApiChannelPrecheck::ProceedAutoSelect
        }
        Err(e) => {
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] API 通道自愈前复核仍不可达 ({e})，跳过 30 秒空等直接计入失败"
            );
            ApiChannelPrecheck::ConfirmedDown
        }
    }
}

async fn execute_core_restart_with_accounting(
    reason_log: &str,
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
) {
    logging!(warn, Type::Lightweight, "{reason_log}");
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

#[allow(clippy::too_many_arguments)]
async fn apply_self_heal_auto_select_result(
    result: anyhow::Result<AutoSelectOutcome>,
    profile_uid: &str,
    dead_node: Option<&str>,
    origin: SelfHealOrigin,
    prev_auto_select_time: Option<Instant>,
    auto_select_fail_count: &mut u32,
    api_dead_heal_fail_count: &mut u32,
    last_auto_select_time: &mut Option<Instant>,
    recent_auto_switches: &mut Vec<Instant>,
) -> bool {
    match result {
        Ok(outcome) if outcome.selected => {
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
            } else if let (Some(dead), Some(new_node)) = (dead_node, outcome.selected_node.as_deref())
                && !dead.is_empty()
                && dead != new_node
            {
                close_dead_node_proxy_connections(dead).await;
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 自愈已切换 {} → {}（已同步清理原故障节点存量代理连接）",
                    dead,
                    new_node
                );
            }
            false
        }
        Ok(_) => {
            *auto_select_fail_count += 1;
            *api_dead_heal_fail_count += 1;
            emit_network_health_notice("network_health::all_dead");
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] 自愈选点未选出可用节点（所有节点不可达），连续失败次数: {}",
                *auto_select_fail_count
            );
            true
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
                return false;
            }
            *auto_select_fail_count += 1;
            *api_dead_heal_fail_count += 1;
            emit_network_health_notice("network_health::all_dead");
            logging!(
                warn,
                Type::Lightweight,
                "[后台监测] 自愈选点失败 ({}), 连续失败次数: {}",
                err_str,
                *auto_select_fail_count
            );
            true
        }
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
        if elapsed < SELF_HEAL_COOLDOWN_SECS {
            logging!(
                info,
                Type::Lightweight,
                "[后台监测] 自愈选点在 {} 秒冷却中（已过 {} 秒），跳过本次",
                SELF_HEAL_COOLDOWN_SECS,
                elapsed
            );
            return false;
        }
    }

    // 【S4 · 原节点确已死门检（BUG-375）】裁决器放行只说明「连续 2 次体检失败」或「窗口内反复黑洞」。
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

    // 【BUG-388 · API 通道恢复保活门检 / BUG-411 · 通道仍死直接免等】
    let mut api_confirmed_down = false;
    if matches!(origin, SelfHealOrigin::ApiChannelDown) {
        match precheck_api_channel_down(auto_select_fail_count, api_dead_heal_fail_count, last_check_time).await {
            ApiChannelPrecheck::KeepAlive => return false,
            ApiChannelPrecheck::ProceedAutoSelect => {}
            ApiChannelPrecheck::ConfirmedDown => api_confirmed_down = true,
        }
    }

    let prev_auto_select_time = *last_auto_select_time;
    *last_auto_select_time = Some(now);

    let failed_this_round = if api_confirmed_down {
        *auto_select_fail_count += 1;
        *api_dead_heal_fail_count += 1;
        emit_network_health_notice("network_health::all_dead");
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] API 通道仍不可达，直接计入自愈失败次数: {}",
            *auto_select_fail_count
        );
        true
    } else {
        let skip_wait_ready = matches!(origin, SelfHealOrigin::ApiChannelDown);
        let result = trigger_backend_auto_select_with_options(profile_uid, None, 0, true, skip_wait_ready, true).await;
        apply_self_heal_auto_select_result(
            result,
            profile_uid,
            dead_node,
            origin,
            prev_auto_select_time,
            auto_select_fail_count,
            api_dead_heal_fail_count,
            last_auto_select_time,
            recent_auto_switches,
        )
        .await
    };
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
                let reason = format!(
                    "[后台监测] 连续 3 次自愈选点失败且内核 API 不可达，尝试重启内核以恢复 API 通道（当前告警计数={}）",
                    *auto_select_fail_count
                );
                execute_core_restart_with_accounting(&reason, auto_select_fail_count, api_dead_heal_fail_count).await;
            }
            SelfHealAction::FireAlert => {
                fire_self_heal_alert();
            }
            SelfHealAction::None => {}
        }
    }
    true
}
