use super::{CoreManager, RunningMode};
use crate::config::Config;
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;

/// 保护 fallback_to_system_proxy 中的配置修改，避免与前端 patch_verge 交错写入
static FALLBACK_CONFIG_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

impl CoreManager {
    pub async fn start_core(&self) -> Result<()> {
        // 先在锁外等待服务就绪（可能阻塞很久，如UAC弹窗、服务启动等）
        // 这样 stop/restart 不会被长时间阻塞
        self.await_service_ready_if_needed().await;

        let _life = self.lifecycle_lock.lock().await;
        self.start_core_inner().await
    }

    async fn start_core_inner(&self) -> Result<()> {
        // 退出中不再启动新内核
        if Handle::global().is_exiting() {
            return Ok(());
        }

        // 已有内核运行时保持幂等
        if !matches!(*self.get_running_mode(), RunningMode::NotRunning) {
            logging!(
                info,
                Type::Core,
                "start_core called while a core is running; treated as no-op"
            );
            return Ok(());
        }

        self.prepare_startup_mode().await?;
        defer! {
            self.after_core_process();
        }

        // 等待服务期间可能进入退出
        if Handle::global().is_exiting() {
            self.set_running_mode(RunningMode::NotRunning);
            return Ok(());
        }

        let result = match *self.get_running_mode() {
            RunningMode::Service => self.start_core_by_service().await,
            RunningMode::NotRunning | RunningMode::Sidecar => self.start_core_by_sidecar().await,
        };

        // 启动失败时回滚 mode
        if result.is_err() {
            self.set_running_mode(RunningMode::NotRunning);
        }

        result
    }

    pub async fn stop_core(&self) -> Result<()> {
        let _life = self.lifecycle_lock.lock().await;
        self.stop_core_inner().await
    }

    async fn stop_core_inner(&self) -> Result<()> {
        CLASH_LOGGER.clear_logs().await;

        // WARNING: DO NOT remove or bypass clearing the IPC connection pool here!
        // When Clash core restarts, previous connection streams become stale and dead.
        // Failing to clear the pool will result in backend connection errors and speed test failures.
        // Refer to BUG-171/BUG-172 agreements.
        match tauri_plugin_mihomo::IpcConnectionPool::global() {
            Ok(pool) => {
                pool.clear_pool();
            }
            Err(e) => {
                logging!(warn, Type::Core, "获取 IPC 连接池失败，跳过清理: {e}");
            }
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
        let _life = self.lifecycle_lock.lock().await;
        logging!(info, Type::Core, "Restarting core");
        self.stop_core_inner().await?;
        self.start_core_inner().await
    }

    /// 锁外等待服务就绪（可能耗时很久，如UAC弹窗、服务启动等）
    /// 避免长时间持有 lifecycle_lock 阻塞 stop/restart
    #[cfg(target_os = "windows")]
    async fn await_service_ready_if_needed(&self) {
        let needs_service = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);
        if needs_service {
            let is_admin = crate::utils::sysinfo::is_current_app_handle_admin(Handle::app_handle());
            if !is_admin {
                self.wait_for_service_ready().await;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    async fn await_service_ready_if_needed(&self) {}

    /// 锁内设置运行模式（不包含耗时等待）
    async fn prepare_startup_mode(&self) -> Result<()> {
        let needs_service = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);

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
        let _guard = FALLBACK_CONFIG_LOCK.lock().await;
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
