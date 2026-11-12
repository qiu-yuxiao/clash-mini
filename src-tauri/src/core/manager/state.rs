use super::{CoreManager, RunningMode};
use crate::{
    AsyncHandler,
    config::{Config, IClashTemp},
    core::{handle, logger::Logger, manager::CLASH_LOGGER, service},
    logging,
    utils::dirs,
};
use anyhow::Result;
use clash_verge_logging::Type;
use compact_str::CompactString;
use log::Level;
use scopeguard::defer;
use tauri_plugin_shell::ShellExt as _;

impl CoreManager {
    pub async fn get_clash_logs(&self) -> Result<Vec<CompactString>> {
        match *self.get_running_mode() {
            RunningMode::Service => service::get_clash_logs_by_service().await,
            RunningMode::Sidecar => Ok(CLASH_LOGGER.get_logs().await),
            RunningMode::NotRunning => Ok(Vec::new()),
        }
    }

    pub(super) async fn start_core_by_sidecar(&self) -> Result<()> {
        logging!(info, Type::Core, "Starting core in sidecar mode");

        // 启动前先强制清理后台残留的 mini-mihomo 进程，避免孤儿进程占用端口导致新实例无法绑定
        Self::kill_all_mini_cores().await;

        let config_file = Config::generate_file(crate::config::ConfigType::Run).await?;
        let app_handle = handle::Handle::app_handle();
        let clash_core = Config::verge().await.latest_arc().get_valid_clash_core();
        let config_dir = dirs::app_home_dir()?;

        let cores_dir = config_dir.join("cores");

        let gomemlimit = std::env::var("GOMEMLIMIT").unwrap_or_else(|_| "128MiB".to_string());
        let gogc = std::env::var("GOGC").unwrap_or_else(|_| "100".to_string());
        let gomaxprocs = std::env::var("GOMAXPROCS").unwrap_or_else(|_| "2".to_string());

        // 【损坏内核防崩溃守卫 BUG-319 / BUG-346】统一校验托管热更内核体积与 PE32+/COFF 头：
        // 确系结构损坏时清理并回退内置 sidecar；瞬时 I/O 占用时保留原文件并仅本次回退内置 sidecar
        let (mut rx, child) = if let Some(custom_core_path) =
            crate::core::core_updater::CoreUpdater::resolve_usable_custom_core(&cores_dir)
        {
            logging!(
                info,
                Type::Core,
                "Starting custom hot-upgraded core: {}",
                custom_core_path.display()
            );
            app_handle
                .shell()
                .command(custom_core_path)
                .env("GOMEMLIMIT", gomemlimit)
                .env("GOGC", gogc)
                .env("GOMAXPROCS", gomaxprocs)
                .args([
                    "-d",
                    dirs::path_to_str(&config_dir)?,
                    "-f",
                    dirs::path_to_str(&config_file)?,
                    "-ext-ctl-pipe",
                    &IClashTemp::guard_external_controller_ipc(),
                ])
                .spawn()?
        } else {
            app_handle
                .shell()
                .sidecar(clash_core.as_str())?
                .env("GOMEMLIMIT", gomemlimit)
                .env("GOGC", gogc)
                .env("GOMAXPROCS", gomaxprocs)
                .args([
                    "-d",
                    dirs::path_to_str(&config_dir)?,
                    "-f",
                    dirs::path_to_str(&config_file)?,
                    "-ext-ctl-pipe",
                    &IClashTemp::guard_external_controller_ipc(),
                ])
                .spawn()?
        };

        let pid = child.pid();
        logging!(trace, Type::Core, "Sidecar started with PID: {}", pid);

        self.set_running_child_sidecar(child);
        self.set_running_mode(RunningMode::Sidecar);

        let handle = AsyncHandler::spawn(|| async move {
            while let Some(event) = rx.recv().await {
                match event {
                    tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                        let message = CompactString::from(&*String::from_utf8_lossy(&line));
                        Logger::global().writer_sidecar_log(Level::Info, &message);
                        CLASH_LOGGER.append_log(message).await;
                    }
                    tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                        let message = CompactString::from(&*String::from_utf8_lossy(&line));
                        Logger::global().writer_sidecar_log(Level::Warn, &message);
                        CLASH_LOGGER.append_log(message).await;
                    }
                    tauri_plugin_shell::process::CommandEvent::Terminated(term) => {
                        let message = if let Some(code) = term.code {
                            CompactString::from(format!("Process terminated with code: {}", code))
                        } else if let Some(signal) = term.signal {
                            CompactString::from(format!("Process terminated by signal: {}", signal))
                        } else {
                            CompactString::from("Process terminated")
                        };
                        Logger::global().writer_sidecar_log(Level::Info, &message);
                        if term.code == Some(0) {
                            CLASH_LOGGER.clear_logs().await;
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });

        self.set_log_consumer_handle(handle);

        Ok(())
    }

    pub(super) async fn stop_core_by_sidecar(&self) {
        logging!(info, Type::Core, "Stopping sidecar");
        defer! {
            self.set_running_mode(RunningMode::NotRunning);
        }

        // 中止日志消费任务
        if let Some(handle) = self.take_log_consumer_handle() {
            handle.abort();
        }

        if let Some(child) = self.take_child_sidecar() {
            let pid = child.pid();
            let result = child.kill();
            logging!(
                trace,
                Type::Core,
                "Sidecar stopped (PID: {:?}, Result: {:?})",
                pid,
                result
            );
            // 轮询等待旧子进程真正退出以释放监听端口与 TUN 句柄（上限 2 秒，步进 50ms，BUG-326；join 异常保守视为仍存活）
            let wait_pid = sysinfo::Pid::from_u32(pid);
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), async move {
                loop {
                    let alive = tokio::task::spawn_blocking(move || {
                        let system = sysinfo::System::new_with_specifics(
                            sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
                        );
                        system.process(wait_pid).is_some()
                    })
                    .await
                    .unwrap_or(true);
                    if !alive {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            })
            .await;
        }
        Self::kill_all_mini_cores().await;
    }

    /// 检测系统中是否仍有 `mini-mihomo` 进程存活。
    /// 若 `spawn_blocking` 调度异常，保守返回 `true`（视为仍存活，绝不提前误判为已退出）。
    pub async fn has_running_mini_cores() -> bool {
        tokio::task::spawn_blocking(|| {
            let system = sysinfo::System::new_with_specifics(
                sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
            );
            system
                .processes()
                .values()
                .any(|p| p.name().to_string_lossy().contains("mini-mihomo"))
        })
        .await
        .unwrap_or(true)
    }

    /// 轮询等待全部 `mini-mihomo` 进程彻底退出（供退出清理与热升级共用，BUG-331 / BUG-346）。
    /// 返回 `true` 表示已确认全部退出，`false` 表示超时仍有存活或未能确认。
    pub async fn wait_for_mini_cores_exit(max_wait: std::time::Duration) -> bool {
        tokio::time::timeout(max_wait, async {
            loop {
                if !Self::has_running_mini_cores().await {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .is_ok()
    }

    /// 当退出流程已优先通过提权服务通道停止内核后，同步清理本地日志缓冲、IPC 连接池与残留句柄状态。
    pub async fn cleanup_local_state_after_service_stop(&self) {
        CLASH_LOGGER.clear_logs().await;
        if let Ok(pool) = tauri_plugin_mihomo::IpcConnectionPool::global() {
            pool.clear_pool();
        }
        if let Some(handle) = self.take_log_consumer_handle() {
            handle.abort();
        }
        if let Some(child) = self.take_child_sidecar() {
            let _ = child.kill();
        }
        self.set_running_mode(RunningMode::NotRunning);
        self.after_core_process();
    }

    pub async fn kill_all_mini_cores() {
        let _ = tokio::task::spawn_blocking(|| {
            logging!(
                info,
                Type::Core,
                "Scanning and killing leftover mini-mihomo processes..."
            );
            // 仅扫描进程列表，避免 System::new_all() 额外枚举磁盘、网卡、传感器与全量硬件信息带来的 50~150ms 阻塞
            let system = sysinfo::System::new_with_specifics(
                sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
            );
            for (pid, process) in system.processes() {
                let name = process.name();
                let name_str = name.to_string_lossy();
                if name_str.contains("mini-mihomo") {
                    if process.kill() {
                        logging!(warn, Type::Core, "Killed leftover process: {} (PID: {})", name_str, pid);
                    } else {
                        logging!(
                            warn,
                            Type::Core,
                            "Failed to kill leftover process (insufficient privilege or already exited): {} (PID: {})",
                            name_str,
                            pid
                        );
                    }
                }
            }
        })
        .await;
    }

    pub(super) async fn start_core_by_service(&self) -> Result<()> {
        logging!(info, Type::Core, "Starting core in service mode");
        let config_file = Config::generate_file(crate::config::ConfigType::Run).await?;
        service::run_core_by_service(&config_file).await?;
        self.set_running_mode(RunningMode::Service);
        Ok(())
    }

    #[allow(dead_code)]
    pub(super) async fn stop_core_by_service(&self) -> Result<()> {
        logging!(info, Type::Core, "Stopping service");
        defer! {
            self.set_running_mode(RunningMode::NotRunning);
        }
        service::stop_core_by_service().await?;
        Ok(())
    }
}
