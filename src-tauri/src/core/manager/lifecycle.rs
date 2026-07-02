use super::{CoreManager, RunningMode};
use crate::cmd::StringifyErr as _;
use crate::config::{Config, IVerge};
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
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
        let needs_service = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);

        #[cfg(target_os = "windows")]
        if needs_service {
            self.wait_for_service_ready().await;
        }

        let mode = if needs_service {
            let value = SERVICE_MANAGER.current().await;
            match value {
                ServiceStatus::Ready => RunningMode::Service,
                _ => RunningMode::Sidecar,
            }
        } else {
            RunningMode::Sidecar
        };

        self.set_running_mode(mode);
        Ok(())
    }

    fn after_core_process(&self) {
        let app_handle = Handle::app_handle();
        crate::utils::sysinfo::set_app_core_mode(app_handle, self.get_running_mode().to_string());
    }

    #[cfg(target_os = "windows")]
    async fn wait_for_service_ready(&self) {
        use crate::{constants::timing, core::service};
        use backon::{ConstantBuilder, Retryable as _};

        let is_admin = crate::utils::sysinfo::is_current_app_handle_admin(Handle::app_handle());
        if is_admin {
            return;
        }

        let res = async {
            // 1. 如果完全未安装，则尝试进行安装（触发 UAC）
            if !service::is_service_installed() {
                logging!(info, Type::Service, "检测到系统服务未安装，启动安装提权");
                SERVICE_MANAGER
                    .handle_service_status(ServiceStatus::InstallRequired)
                    .await?;
            } else {
                // 2. 如果已安装，刷新并启动（若停止）
                SERVICE_MANAGER.refresh().await?;
            }

            // 3. 轮询等待服务就绪
            let max_times = timing::SERVICE_WAIT_MAX.as_millis() / timing::SERVICE_WAIT_INTERVAL.as_millis();
            let backoff = ConstantBuilder::default()
                .with_delay(timing::SERVICE_WAIT_INTERVAL)
                .with_max_times(max_times as usize);

            (|| async {
                if matches!(SERVICE_MANAGER.current().await, ServiceStatus::Ready) {
                    return Ok(());
                }

                if !service::is_service_ipc_path_exists() {
                    return Err(anyhow::anyhow!("Service IPC not ready"));
                }

                SERVICE_MANAGER.init().await?;
                let _ = SERVICE_MANAGER.refresh().await;

                if matches!(SERVICE_MANAGER.current().await, ServiceStatus::Ready) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Service not ready"))
                }
            })
            .retry(backoff)
            .await
        }
        .await;

        if res.is_err() {
            logging!(error, Type::Service, "系统服务启动或安装失败，执行回退至系统代理");
            self.fallback_to_system_proxy().await;
        }
    }

    async fn fallback_to_system_proxy(&self) {
        Config::verge().await.edit_draft(|d| {
            d.enable_tun_mode = Some(false);
            d.enable_system_proxy = Some(true);
        });
        Config::verge().await.apply();
        if let Err(e) = Config::verge().await.latest_arc().save_file().await {
            logging!(error, Type::Service, "保存回退配置失败: {}", e);
        }
        self.set_running_mode(RunningMode::Sidecar);
    }
}
