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

        let stop_timeout = Duration::from_millis(3500);

        logging!(info, Type::System, "stop core");
        let stopped = match timeout(stop_timeout, CoreManager::global().stop_core()).await {
            Ok(_) => {
                logging!(info, Type::Window, "core已停止");
                true
            }
            Err(_) => {
                logging!(
                    warn,
                    Type::Window,
                    "Warning: 停止core超时（可能系统正在关机或管道繁忙），尝试服务级强制停止兜底"
                );
                false
            }
        };
        // 服务模式下 mini-mihomo.exe 运行在 Session 0 SYSTEM 权限下，普通权限 Process::kill() 会因 Error 5 失败；
        // 当 stop_core 超时且服务 IPC 存在时，额外通过提权服务通道发送停止指令兜底（BUG-331）。
        if !stopped && (tun_enabled || crate::core::service::is_service_ipc_path_exists()) {
            let _ = timeout(
                Duration::from_millis(1500),
                crate::core::service::stop_core_by_service(),
            )
            .await;
        }
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
