use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use tauri::async_runtime::JoinHandle;

use crate::{
    config::Config,
    core::{
        CoreManager, Timer,
        handle::Handle,
        logger::Logger,
        service::{SERVICE_MANAGER, ServiceManager, is_service_ipc_path_exists},
        sysopt,
        tray::Tray,
    },
    feat,
    module::lightweight::auto_lightweight_boot,
    process::AsyncHandler,
    utils::{init, server, window_manager::WindowManager},
};
use clash_verge_logging::{Type, logging, logging_error};
use clash_verge_signal;

pub mod dns;
pub mod scheme;
pub mod universal_parser;
pub mod window;
pub mod window_script;

static RESOLVE_DONE: AtomicBool = AtomicBool::new(false);
static RESOLVE_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// M2-01: 追踪启动阶段的 detached 任务，退出时统一 abort
static STARTUP_TASKS: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

/// 将启动任务注册到全局追踪列表
fn track_startup_task(handle: JoinHandle<()>) {
    if let Ok(mut tasks) = STARTUP_TASKS.lock() {
        tasks.push(handle);
    }
}

/// M2-01: 退出时中止所有启动阶段任务
pub fn abort_startup_tasks() {
    if let Ok(mut tasks) = STARTUP_TASKS.lock() {
        for handle in tasks.drain(..) {
            handle.abort();
        }
    }
}

pub fn init_work_dir_and_logger() -> anyhow::Result<()> {
    AsyncHandler::block_on(async {
        init_work_config().await;
        init_resources().await;
        logging!(info, Type::Setup, "Initializing logger");
        // #[cfg(not(feature = "tokio-trace"))]
        Logger::global().init().await?;
        Ok(())
    })
}

pub fn resolve_setup_sync() {
    // M2-01: 追踪启动任务，退出时统一 abort
    let handle = AsyncHandler::spawn(|| async {
        AsyncHandler::spawn_blocking(init_scheme);
        AsyncHandler::spawn_blocking(init_embed_server);
    });
    track_startup_task(handle);
}

pub fn resolve_setup_async() {
    let app_handle = Handle::app_handle().clone();
    // M2-01: 追踪启动任务，退出时统一 abort
    let handle = AsyncHandler::spawn(move || async move {
        if let Err(e) = Tray::global().init(&app_handle).await {
            log::error!(target: "app", "[Setup] Failed to initialize tray: {}", e);
        }

        logging!(info, Type::ClashVergeRev, "Version: {}", env!("CARGO_PKG_VERSION"));

        // M2-01: 追踪 startup_script 任务
        let script_handle = AsyncHandler::spawn(|| async {
            init_startup_script().await;
        });
        track_startup_task(script_handle);
        init_verge_config().await;
        Config::verify_config_initialization().await;

        // 优化启动顺序：先判断是否需要进入轻量模式，避免"创建窗口再销毁"的资源浪费
        // - 静默启动且启用了自动轻量模式：直接进入轻量模式，不创建窗口
        // - 其他情况：正常创建窗口
        let is_silent_start = Config::verge().await.data_arc().enable_silent_start.unwrap_or(false);
        let enable_auto_lightweight = Config::verge()
            .await
            .data_arc()
            .enable_auto_light_weight_mode
            .unwrap_or(false);

        if is_silent_start && enable_auto_lightweight {
            // 静默启动 + 自动轻量模式：直接进入轻量模式，跳过窗口创建
        } else {
            // 正常启动：先创建窗口
            init_window().await;
        }

        init_auto_lightweight_boot().await;

        let core_init = AsyncHandler::spawn(|| async {
            init_service_manager().await;
            init_core_manager().await;

            // 防护：只有 Core 成功启动后才设置系统代理
            // start_core 失败时 running_mode 会被回滚为 NotRunning（lifecycle.rs:49-52）
            // 若此时仍调用 init_system_proxy，会将 OS 代理指向不存在的端口→断网
            if !matches!(
                *CoreManager::global().get_running_mode(),
                crate::core::manager::RunningMode::NotRunning
            ) {
                init_system_proxy().await;
                init_system_proxy_guard().await;
            } else {
                logging!(warn, Type::Setup, "Core 启动失败，跳过系统代理设置以保护用户网络连接");
            }
        });

        let _ = futures::join!(core_init, init_timer());

        crate::module::monitor::start_background_monitor();
        Handle::refresh_clash();
        refresh_tray_menu().await;
        resolve_done();
    });
    track_startup_task(handle);
}

pub(super) fn init_scheme() {
    logging_error!(Type::Setup, init::init_scheme());
}

pub async fn resolve_scheme(param: &str) -> Result<()> {
    logging_error!(Type::Setup, scheme::resolve_scheme(param).await);
    Ok(())
}

pub(super) fn init_embed_server() {
    server::embed_server();
}

pub(super) async fn init_resources() {
    logging_error!(Type::Setup, init::init_resources().await);
}

pub(super) async fn init_startup_script() {
    logging_error!(Type::Setup, init::startup_script().await);
}

pub(super) async fn init_timer() {
    logging_error!(Type::Setup, Timer::global().init().await);
}

pub(super) async fn init_auto_lightweight_boot() {
    logging_error!(Type::Setup, auto_lightweight_boot().await);
}

pub fn init_signal() {
    logging!(info, Type::Setup, "Initializing signal handlers...");
    clash_verge_signal::register(feat::quit);
}

pub async fn init_work_config() {
    logging_error!(Type::Setup, init::init_config().await);
}

pub(super) async fn init_verge_config() {
    logging_error!(Type::Setup, Config::init_config().await);
}

pub(super) async fn init_service_manager() {
    clash_verge_service_ipc::set_config(Some(ServiceManager::config())).await;
    if !is_service_ipc_path_exists() {
        return;
    }
    let is_ok = SERVICE_MANAGER.init().await.is_ok();
    if is_ok {
        logging_error!(Type::Setup, SERVICE_MANAGER.refresh().await);
    }
}

pub(super) async fn init_core_manager() {
    logging_error!(Type::Setup, CoreManager::global().init().await);
}

pub(super) async fn init_system_proxy() {
    logging_error!(Type::Setup, sysopt::Sysopt::global().update_sysproxy().await);
}

pub(super) async fn init_system_proxy_guard() {
    sysopt::Sysopt::global().refresh_guard().await;
}

pub(super) async fn refresh_tray_menu() {
    logging_error!(Type::Setup, Tray::global().update_part().await);
}

pub(super) async fn init_window() {
    let is_silent_start = Config::verge().await.data_arc().enable_silent_start.unwrap_or(false);

    let args: Vec<String> = std::env::args().collect();
    let has_silent_arg = args.iter().any(|arg| arg == "--silent");

    #[cfg(target_os = "windows")]
    let should_silent = is_silent_start && has_silent_arg;
    #[cfg(not(target_os = "windows"))]
    let should_silent = is_silent_start;

    #[cfg(target_os = "macos")]
    if should_silent {
        use crate::core::handle::Handle;
        Handle::global().set_activation_policy_accessory();
    }
    WindowManager::create_window(!should_silent).await;
}

pub fn resolve_done() {
    RESOLVE_DONE.store(true, Ordering::Release);
    // 使用 notify_waiters() 唤醒所有等待初始化的协程，避免并发调用时部分协程被饿死
    RESOLVE_NOTIFY.notify_waiters();
}

pub async fn wait_for_resolve_done() {
    // 先注册 notified() 再检查标志，防止错过通知（与 ServiceManager::current() 同模式）
    loop {
        let notified = RESOLVE_NOTIFY.notified();
        if RESOLVE_DONE.load(Ordering::Acquire) {
            return;
        }
        notified.await;
    }
}
