use super::{CoreManager, RunningMode};
use crate::config::Config;
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;

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

        // 【核心架构约定】Mini 只有 PROXY 一个有效代理组。
        // 内核重启会重置 Selector 组的 now 字段为列表第一个节点（通常是广告假节点）。
        // 这里在 stop 之前保存 PROXY 组的 now，在 start 之后恢复，避免前端显示假节点。
        let saved_proxy_now = self.snapshot_proxy_group_now().await;

        self.stop_core_inner().await?;
        self.start_core_inner().await?;

        // 恢复 PROXY 组的节点选择（等内核就绪后）
        if let Some(node) = saved_proxy_now {
            self.restore_proxy_group_now(&node).await;
        }

        Ok(())
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
        logging!(
            warn,
            Type::Service,
            "检测到系统服务未就绪，执行配置回退：关闭 TUN 模式，开启系统代理"
        );
        let patch = crate::config::IVerge {
            enable_tun_mode: Some(false),
            enable_system_proxy: Some(true),
            ..Default::default()
        };
        if let Err(e) = crate::feat::patch_verge(&patch, false).await {
            logging!(error, Type::Service, "应用回退配置失败: {}", e);
            // 兜底设置运行模式，防止状态卡在 Service 导致后续无法启动
            self.set_running_mode(RunningMode::Sidecar);
        }
    }

    /// 快照当前 PROXY 组的 now 字段（用于内核重启后恢复）
    /// 【核心架构约定】Mini 只有 PROXY 一个有效代理组，所以只快照 PROXY 组。
    /// 返回 None 表示无需恢复（读取失败、now 为空、或应用退出中）。
    pub(super) async fn snapshot_proxy_group_now(&self) -> Option<std::string::String> {
        if Handle::global().is_exiting() {
            return None;
        }
        let mihomo = Handle::mihomo().await.clone();
        match mihomo.get_group_by_name("PROXY").await {
            Ok(group) => match group.now {
                Some(node) if !node.is_empty() => {
                    logging!(info, Type::Core, "快照 PROXY 组当前节点: {}", node);
                    Some(node)
                }
                _ => None,
            },
            Err(e) => {
                logging!(warn, Type::Core, "快照 PROXY 组失败（忽略，重启后不恢复）: {}", e);
                None
            }
        }
    }

    /// 恢复 PROXY 组的节点选择（内核重启后调用）
    /// 等待内核 API 就绪后调用 mihomo.select_node_for_group("PROXY", node)
    /// 失败不抛错（兜底失败时前端可能显示假节点，但不影响代理功能本身）
    pub(super) async fn restore_proxy_group_now(&self, node: &str) {
        if Handle::global().is_exiting() {
            return;
        }

        let mihomo = Handle::mihomo().await.clone();

        // 等待内核 API 就绪（最多 30 秒）
        let start = std::time::Instant::now();
        let mut api_ready = false;
        while start.elapsed().as_secs() < 30 {
            if Handle::global().is_exiting() {
                return;
            }
            if mihomo.get_base_config().await.is_ok() {
                api_ready = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        if !api_ready {
            logging!(warn, Type::Core, "等待内核 API 就绪超时，跳过 PROXY 组节点恢复");
            return;
        }

        // 等待 PROXY 组节点列表填充（最多 20 秒）
        let start2 = std::time::Instant::now();
        while start2.elapsed().as_secs() < 20 {
            if Handle::global().is_exiting() {
                return;
            }
            if let Ok(group_info) = mihomo.get_group_by_name("PROXY").await {
                if let Some(all) = group_info.all {
                    if !all.is_empty() {
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // 恢复节点选择：优先恢复 snapshot 保存的节点
        // 若该节点已不存在（订阅更新导致），回退到 PROXY 组子集中首个可用节点
        match mihomo.select_node_for_group("PROXY", node).await {
            Ok(()) => logging!(info, Type::Core, "已恢复 PROXY 组节点选择: {}", node),
            Err(e) => {
                logging!(
                    warn,
                    Type::Core,
                    "恢复 PROXY 组节点选择失败 ({}): {}，尝试回退到子集首个可用节点",
                    node,
                    e
                );
                // 读取用户保存的 filterText，在子集范围内选点兜底
                let filter_lower = match Config::profiles().await.latest_arc().current.as_ref() {
                    Some(current_uid) => {
                        let path = crate::utils::dirs::app_home_dir()
                            .map(|d| d.join("proxy_head_state.json"))
                            .ok();
                        match path {
                            Some(p) => tokio::fs::read_to_string(&p)
                                .await
                                .ok()
                                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                                .and_then(|v| {
                                    v[current_uid.as_str()]["PROXY"]["filterText"]
                                        .as_str()
                                        .map(|s| s.trim().to_lowercase())
                                })
                                .unwrap_or_default(),
                            None => String::new(),
                        }
                    }
                    None => String::new(),
                };
                if let Ok(group_info) = mihomo.get_group_by_name("PROXY").await {
                    if let Some(all) = group_info.all {
                        // 过滤广告/假节点，避免回退到伪节点触发自愈死循环
                        let fallback_node = all
                            .iter()
                            .filter(|n| !crate::utils::node::is_dummy_node(n))
                            .find(|n| filter_lower.is_empty() || n.to_lowercase().contains(&filter_lower))
                            .or_else(|| all.iter().find(|n| !crate::utils::node::is_dummy_node(n)));
                        if let Some(fb) = fallback_node {
                            match mihomo.select_node_for_group("PROXY", fb).await {
                                Ok(()) => logging!(info, Type::Core, "已回退 PROXY 组节点选择到: {}", fb),
                                Err(e2) => logging!(warn, Type::Core, "PROXY 组回退节点选择也失败: {}", e2),
                            }
                        }
                    }
                }
            }
        }
    }
}
