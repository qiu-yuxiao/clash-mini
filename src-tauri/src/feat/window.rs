use crate::config::Config;
use crate::core::{CoreManager, handle, sysopt};
use crate::module::lightweight;
use crate::utils;
use crate::utils::window_manager::WindowManager;
use clash_verge_logging::{Type, logging};
use tokio::time::{Duration, timeout};

/// 退出前统一准备工作：设置退出标志、中止后台任务、停止定时器等
pub async fn prepare_exit() {
    logging!(debug, Type::System, "准备退出，中止所有后台任务");
    // 第一时间置位正常退出内核事件，让伴生看门狗 0ms 同步退场（BUG-316）
    crate::process::guardian::notify_intentional_exit();
    // 设置退出标志
    handle::Handle::global().set_is_exiting();

    // 中止后台 monitor 常驻线程
    crate::module::monitor::abort_monitor();

    // 中止轻量模式 cleanup 任务
    crate::module::lightweight::abort_lightweight_cleanup();

    // M2-01: 中止启动阶段残留的 detached 任务
    crate::utils::resolve::abort_startup_tasks();

    // 唤醒 monitor 线程，让它检测到退出标志并尽快终止
    crate::module::monitor::MONITOR_WAKEUP_NOTIFY.notify_one();
    crate::module::monitor::PROFILE_SWITCH_NOTIFY.notify_waiters();

    // 停止 Timer 调度器
    crate::core::timer::Timer::global().shutdown();

    // 关闭所有 WebSocket 订阅
    let mihomo = handle::Handle::mihomo().await.clone();
    if let Err(e) = mihomo.clear_all_ws_connections().await {
        logging!(warn, Type::System, "退出时清理WebSocket订阅失败: {e}");
    }

    // 刷新日志缓冲区
    crate::core::logger::Logger::global().flush();

    // 短暂等待后台任务退出
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    utils::server::shutdown_embedded_server();
    Config::apply_all_and_save_file().await;
}

pub async fn open_or_close_dashboard() {
    if lightweight::is_in_lightweight_mode() {
        let _ = lightweight::exit_lightweight_mode().await;
        return;
    }

    let result = WindowManager::toggle_main_window().await;
    logging!(info, Type::Window, "Window toggle result: {result:?}");
}

pub async fn quit() {
    logging!(debug, Type::System, "启动退出流程");
    prepare_exit().await;

    logging!(info, Type::System, "开始异步清理资源");
    let cleanup_result = clean_async().await;

    logging!(
        info,
        Type::System,
        "资源清理完成，退出代码: {}",
        if cleanup_result { 0 } else { 1 }
    );

    // 最终刷新日志
    crate::core::logger::Logger::global().shutdown();

    let app_handle = handle::Handle::app_handle();
    app_handle.exit(if cleanup_result { 0 } else { 1 });
}

pub async fn clean_async() -> bool {
    logging!(info, Type::System, "开始执行异步清理操作...");

    // 重置系统代理
    let proxy_task = tokio::task::spawn(async {
        let sys_proxy_enabled = Config::verge().await.data_arc().enable_system_proxy.unwrap_or(false);
        if !sys_proxy_enabled {
            logging!(info, Type::Window, "系统代理未启用，跳过重置");
            return true;
        }

        logging!(info, Type::Window, "开始重置系统代理...");
        match timeout(Duration::from_millis(1500), sysopt::Sysopt::global().reset_sysproxy()).await {
            Ok(Ok(_)) => {
                logging!(info, Type::Window, "系统代理已重置");
                true
            }
            Ok(Err(e)) => {
                logging!(warn, Type::Window, "Warning: 重置系统代理失败: {e}");
                false
            }
            Err(_) => {
                logging!(warn, Type::Window, "Warning: 重置系统代理超时，继续退出");
                false
            }
        }
    });

    // 关闭 Tun 模式 + 停止核心服务
    let core_task = tokio::task::spawn(async {
        logging!(info, Type::System, "disable tun");
        let tun_enabled = Config::verge().await.data_arc().enable_tun_mode.unwrap_or(false);
        if tun_enabled {
            let disable_tun = serde_json::json!({ "tun": { "enable": false } });

            logging!(info, Type::System, "send disable tun request to mihomo");
            let mihomo = handle::Handle::mihomo().await.clone();
            match timeout(Duration::from_millis(1000), mihomo.patch_base_config(&disable_tun)).await {
                Ok(Ok(_)) => {
                    logging!(info, Type::Window, "TUN模式已禁用");
                }
                Ok(Err(e)) => {
                    logging!(warn, Type::Window, "Warning: 禁用TUN模式失败: {e}");
                }
                Err(_) => {
                    logging!(
                        warn,
                        Type::Window,
                        "Warning: 禁用TUN模式超时（可能系统正在关机），继续退出流程"
                    );
                }
            }
        }

        // 在停止前预先采样运行模式（stop_core 内部 defer 会将模式置为 NotRunning）
        let was_service_mode = matches!(
            *CoreManager::global().get_running_mode(),
            crate::core::manager::RunningMode::Service
        );

        let mut stopped = false;

        // 【服务模式优先提权停止 BUG-331】
        // 服务态或开启 TUN 时，mini-mihomo.exe 运行在 Session 0 SYSTEM 权限下，普通用户态无法终止，
        // 且若先排队等待 lifecycle_lock 可能白耗 3.5s；因此优先走提权服务通道直发停止指令并确认进程退出，
        // 随后同步清理本地连接池、日志缓冲与可能存在的残留子进程状态。
        if was_service_mode || tun_enabled {
            logging!(info, Type::System, "stop core via service channel first");
            match timeout(
                Duration::from_millis(2000),
                crate::core::service::stop_core_by_service(),
            )
            .await
            {
                Ok(Ok(())) => {
                    logging!(info, Type::Window, "已通过提权服务通道停止内核");
                    if !CoreManager::wait_for_mini_cores_exit(Duration::from_millis(1000)).await {
                        logging!(
                            warn,
                            Type::Window,
                            "Warning: 服务停止指令已返回，但等待 mini-mihomo 进程彻底退出满 1s 仍未结束"
                        );
                    }
                    CoreManager::global().cleanup_local_state_after_service_stop().await;
                    stopped = true;
                }
                Ok(Err(e)) => {
                    logging!(
                        warn,
                        Type::Window,
                        "Warning: 提权服务通道停止内核返回错误 ({e})，回退执行 CoreManager::stop_core"
                    );
                }
                Err(_) => {
                    logging!(
                        warn,
                        Type::Window,
                        "Warning: 提权服务通道停止内核超时，回退执行 CoreManager::stop_core"
                    );
                }
            }
        }

        if !stopped {
            let stop_timeout = if was_service_mode || tun_enabled {
                Duration::from_millis(2000)
            } else {
                Duration::from_millis(3500)
            };
            logging!(info, Type::System, "stop core");
            stopped = match timeout(stop_timeout, CoreManager::global().stop_core()).await {
                Ok(Ok(())) => {
                    logging!(info, Type::Window, "core已停止");
                    true
                }
                Ok(Err(e)) => {
                    logging!(warn, Type::Window, "Warning: 停止core返回错误 ({e})");
                    false
                }
                Err(_) => {
                    logging!(
                        warn,
                        Type::Window,
                        "Warning: 停止core超时（可能系统正在关机或管道繁忙）"
                    );
                    false
                }
            };

            // 若初始模式误记为 Sidecar/NotRunning（状态漂移），但停止后仍有 mini-mihomo 残留且系统服务已安装，
            // 再补一次服务级停止兜底，防止 Session 0 孤儿内核漏网。
            if !(was_service_mode || tun_enabled)
                && (!stopped || CoreManager::has_running_mini_cores().await)
                && crate::core::service::is_service_installed_async().await
            {
                match timeout(
                    Duration::from_millis(1500),
                    crate::core::service::stop_core_by_service(),
                )
                .await
                {
                    Ok(Ok(())) => {
                        logging!(info, Type::Window, "已通过系统服务兜底停止残留内核");
                        let _ = CoreManager::wait_for_mini_cores_exit(Duration::from_millis(1000)).await;
                        stopped = true;
                    }
                    Ok(Err(e)) => {
                        logging!(warn, Type::Window, "Warning: 系统服务兜底停止残留内核失败: {e}");
                    }
                    Err(_) => {
                        logging!(warn, Type::Window, "Warning: 系统服务兜底停止残留内核超时");
                    }
                }
            }
        }

        // 始终执行本地进程级兜底清理（kill_all_mini_cores 内部已对无权限 Session 0 进程优雅告警放行），
        // 确保无论提权通道成败、或同会话内存在历史残留用户态 mini-mihomo 进程，均不会拆掉最后一道防线（BUG-331）。
        CoreManager::kill_all_mini_cores().await;
        stopped
    });

    // 并行执行清理任务
    let (proxy_result, core_result) = tokio::join!(proxy_task, core_task);

    let proxy_success = proxy_result.unwrap_or_default();
    let core_success = core_result.unwrap_or_default();

    let all_success = proxy_success && core_success;

    logging!(
        info,
        Type::System,
        "异步关闭操作完成 - 代理: {}, 核心: {}, 总体: {}",
        proxy_success,
        core_success,
        all_success
    );

    all_success
}
