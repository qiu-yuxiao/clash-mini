use super::{CoreManager, RunningMode};
use crate::cmd::StringifyErr as _;
use crate::config::{Config, IVerge};
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::service::{SERVICE_MANAGER, ServiceManager, ServiceStatus};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;
use smartstring::alias::String;

impl CoreManager {
    pub async fn start_core(&self) -> Result<()> {
        self.prepare_startup().await?;
        defer! {
            self.after_core_process();
        }

        match *self.get_running_mode() {
            RunningMode::Service => self.start_core_by_service().await,
            RunningMode::NotRunning | RunningMode::Sidecar => self.start_core_by_sidecar().await,
        }
    }

    pub async fn stop_core(&self) -> Result<()> {
        CLASH_LOGGER.clear_logs().await;

        // WARNING: DO NOT remove or bypass clearing the IPC connection pool here!
        // When Clash core restarts, previous connection streams become stale and dead.
        // Failing to clear the pool will result in backend connection errors and speed test failures.
        // Refer to BUG-171/BUG-172 agreements.
        if let Ok(pool) = tauri_plugin_mihomo::IpcConnectionPool::global() {
            pool.clear_pool();
        }

        defer! {
            self.after_core_process();
        }

        match *self.get_running_mode() {
            RunningMode::Service => self.stop_core_by_service().await,
            RunningMode::Sidecar => {
                self.stop_core_by_sidecar().await;
                Ok(())
            }
            RunningMode::NotRunning => Ok(()),
        }
    }

    pub async fn restart_core(&self) -> Result<()> {
        logging!(info, Type::Core, "Restarting core");
        self.stop_core().await?;
        self.start_core().await
    }

    pub async fn change_core(&self, clash_core: &String) -> Result<(), String> {
        if !IVerge::VALID_CLASH_CORES.contains(&clash_core.as_str()) {
            return Err(format!("Invalid clash core: {}", clash_core).into());
        }

        Config::verge().await.edit_draft(|d| {
            d.clash_core = Some(clash_core.to_owned());
        });
        Config::verge().await.apply();

        let verge_data = Config::verge().await.latest_arc();
        verge_data.save_file().await.map_err(|e| e.to_string())?;

        self.update_config_checked().await.stringify_err()?;
        Ok(())
    }

    async fn prepare_startup(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        self.wait_for_service_if_needed().await;

        let value = SERVICE_MANAGER.lock().await.current();
        let mode = match value {
            ServiceStatus::Ready => RunningMode::Service,
            _ => RunningMode::Sidecar,
        };

        self.set_running_mode(mode);
        Ok(())
    }

    fn after_core_process(&self) {
        let app_handle = Handle::app_handle();
        crate::utils::sysinfo::set_app_core_mode(app_handle, self.get_running_mode().to_string());
    }

    #[cfg(target_os = "windows")]
    async fn wait_for_service_if_needed(&self) {
        use crate::{config::Config, constants::timing, core::service};
        use backon::{ConstantBuilder, Retryable as _};

        let needs_service = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);

        if !needs_service {
            return;
        }

        let is_admin = crate::utils::sysinfo::is_current_app_handle_admin(Handle::app_handle());
        if is_admin {
            return;
        }

        let is_service_installed = tokio::task::spawn_blocking(service::is_service_installed)
            .await
            .unwrap_or(false);
        if !is_service_installed {
            logging!(
                warn,
                Type::Service,
                "Clash Verge Service is not installed, skipping wait."
            );
            return;
        }

        // Wait up to 3 minutes (180,000 milliseconds) for service to boot on startup
        let max_times = 180000 / timing::SERVICE_WAIT_INTERVAL.as_millis();
        let backoff = ConstantBuilder::default()
            .with_delay(timing::SERVICE_WAIT_INTERVAL)
            .with_max_times(max_times as usize);

        let result = (|| async {
            let mut manager = SERVICE_MANAGER.lock().await;

            if matches!(manager.current(), ServiceStatus::Ready) {
                return Ok(());
            }

            // If the service IPC path is not ready yet, treat it as transient and retry.
            // Running init/refresh too early can mark service state unavailable and break later config reloads.
            if !service::is_service_ipc_path_exists() {
                return Err(anyhow::anyhow!("Service IPC not ready"));
            }

            manager.init().await?;
            drop(manager);
            let _ = ServiceManager::refresh().await;

            let manager = SERVICE_MANAGER.lock().await;
            if matches!(manager.current(), ServiceStatus::Ready) {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Service not ready"))
            }
        })
        .retry(backoff)
        .await;

        if result.is_err() {
            logging!(
                error,
                Type::Service,
                "Clash Verge Service startup timed out after 3 minutes."
            );
            std::thread::spawn(|| {
                crate::show_error_dialog(
                    "Clash Mini Service Error",
                    "无法连接到 Clash Verge Service。TUN 模式可能无法正常工作。\n请尝试在系统托盘右键菜单中重新安装或修复服务。",
                );
            });
        }
    }
}
