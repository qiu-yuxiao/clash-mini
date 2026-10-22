use super::{CoreManager, RunningMode};
use crate::config::Config;
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;
use tauri_plugin_mihomo::Mihomo;

/// 内置出站（非真实代理节点）。
///
/// `PROXY` 组只有在「一个真实节点都没有」的冷启动窗口里才会用 `DIRECT` 临时兜底
/// （见 `enhance::enforce_mini_agreements` 的空组兜底分支）。此时 `group.now` 会等于
/// `DIRECT`，若照抄进快照、再在真实节点加载后还原，必然得到
/// `Selector update error: proxy not exist` —— v3.0.0 现场日志实证：
/// `WARN 恢复 PROXY 组节点选择失败 (DIRECT): ... proxy not exist`。
/// 这类名字不属于「用户的选择」，快照与还原都应跳过，交给正常的自动选点流程。
fn is_builtin_outbound(name: &str) -> bool {
    matches!(name, "DIRECT" | "REJECT" | "REJECT-DROP" | "PASS" | "COMPATIBLE")
}

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
        // 【锁外等服务，与 start_core 同源】restart 路径此前不经过
        // await_service_ready_if_needed，TUN 开启时 prepare_startup_mode 只能读到
        // SERVICE_MANAGER 的缓存状态——若缓存陈旧（如安装命令超时后停在 Unavailable，
        // 而服务实际随后已就绪），内核会被静默降级成 Sidecar，TUN 完全不生效
        // 且没有任何提示（BUG-298 现场：TUN 开关显示成功但流量不被接管）。
        self.await_service_ready_if_needed().await;

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

        // 【强咬合防线】重启完成后通知前端拉取最新配置和活跃节点
        Handle::refresh_clash();

        Ok(())
    }

    /// 锁外等待服务就绪（可能耗时很久，如UAC弹窗、服务启动等）
    /// 避免长时间持有 lifecycle_lock 阻塞 stop/restart
    async fn await_service_ready_if_needed(&self) {
        let needs_service = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);
        if needs_service {
            let is_admin = crate::utils::sysinfo::is_current_app_handle_admin(Handle::app_handle());
            if !is_admin {
                self.wait_for_service_ready().await;
            }
        }
    }

    /// 锁内设置运行模式（不包含耗时等待）
    async fn prepare_startup_mode(&self) -> Result<()> {
        let needs_service = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);

        let mode = if needs_service {
            let value = SERVICE_MANAGER.current().await;
            match value {
                ServiceStatus::Ready => RunningMode::Service,
                // 锁内只能接受现状不能等待（UAC 可能很久）。走到这里说明
                // await_service_ready_if_needed 未能把服务带到 Ready——
                // 必须留痕，禁止静默降级（BUG-298 教训）。
                other => {
                    logging!(
                        warn,
                        Type::Service,
                        "TUN 已开启但服务状态为 {other:?}，内核将以 Sidecar 模式启动（TUN 不会生效）"
                    );
                    RunningMode::Sidecar
                }
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
            // 【前端通知】静默回退曾让用户误以为 TUN 已生效（BUG-298：开关显示成功
            // 但内核实为 Sidecar，流量不被接管）。回退必须明确告知。
            Handle::notice_message("warning", "系统服务未就绪，TUN 模式无法生效，已自动回退为系统代理模式");
            self.fallback_to_system_proxy().await;
        }
    }

    async fn fallback_to_system_proxy(&self) {
        logging!(
            warn,
            Type::Service,
            "检测到系统服务未就绪，执行配置回退：关闭 TUN 模式，开启系统代理"
        );
        // 【禁止走 feat::patch_verge】enable_tun_mode 变更会联动 RESTART_CORE 分支，
        // 与本函数的调用方（start_core / restart_core 的锁外前奏）构成 async fn
        // 静态递归（E0733），运行期还会经 VERGE_PATCH_LOCK 互相等待造成死锁。
        // 这里只落定配置并应用系统代理：随后的启动/重启流程读到 enable_tun_mode=false
        // 自行走 Sidecar，系统代理由 update_sysproxy 立即接管。
        {
            let verge = Config::verge().await;
            verge.edit_draft(|d| {
                d.enable_tun_mode = Some(false);
                d.enable_system_proxy = Some(true);
            });
            verge.apply();
            let data = verge.data_arc();
            if let Err(e) = data.save_file() {
                logging!(error, Type::Service, "应用回退配置失败: {}", e);
            }
        }
        if let Err(e) = crate::core::sysopt::Sysopt::global().update_sysproxy().await {
            logging!(error, Type::Service, "回退应用系统代理失败: {}", e);
        }
        crate::core::sysopt::Sysopt::global().refresh_guard().await;
        // 【必须通知前端】配置是被后端「背着前端」改掉的（绕过了 patch_verge），
        // 而 verge://refresh-verge-config 这个事件原本由 patch_verge 的
        // process_terminated_flags 发出，此路径不补发的话前端 TUN / 系统代理开关
        // 会一直停在旧状态（显示「TUN 开、代理关」，实际正好相反）。
        // 这属于 BUG-298 那类「界面与现实不一致」问题的镜像，不能只靠 warning 提示兜住。
        Handle::refresh_verge();
    }

    /// 快照当前 PROXY 组的 now 字段（用于内核重启后恢复）
    /// 【核心架构约定】Mini 只有 PROXY 一个有效代理组，所以只快照 PROXY 组。
    /// 返回 None 表示无需恢复（读取失败、now 为空、或应用退出中）。
    ///
    /// 可见性说明：原本为 `pub(super)` 仅供 manager 子模块内部使用；
    /// v2.6.5 隐患 B 修复后 `core_updater.rs`（不在 manager 子模块下）的内核升级路径
    /// 也需要调用 snapshot/restore 来保护 PROXY.now，故提升为 `pub`。
    pub async fn snapshot_proxy_group_now(&self) -> Option<std::string::String> {
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
    pub async fn restore_proxy_group_now(&self, node: &str) {
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

        // 等待 PROXY 组节点列表填充（最多 20 秒），顺带确认目标节点是否已入组
        // （决定后续 select 是否值得重试：不在组内属持久性错误，重试无意义）
        let start2 = std::time::Instant::now();
        let mut node_in_group = false;
        while start2.elapsed().as_secs() < 20 {
            if Handle::global().is_exiting() {
                return;
            }
            if let Ok(group_info) = mihomo.get_group_by_name("PROXY").await {
                if let Some(all) = group_info.all {
                    if !all.is_empty() {
                        node_in_group = all.iter().any(|n| n == node);
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // 防御：内置出站（DIRECT/REJECT…）不是可选择的节点，拿它去 select 必然报
        // `proxy not exist`。放到「API 就绪 + 组列表已填充」之后处理，跳过这次注定
        // 失败的 IPC，直接走与「还原失败」相同的子集兜底，恢复语义不变。
        if is_builtin_outbound(node) {
            logging!(
                info,
                Type::Core,
                "跳过 PROXY 组节点恢复：{} 属内置出站，直接按子集兜底",
                node
            );
            let filter_lower = read_filter_text().await;
            select_fallback_in_subset(&mihomo, &filter_lower).await;
            return;
        }

        // 【BUG-301】目标节点不在组成员里（订阅更新移除、或空组兜底成 DIRECT），
        // select 是持久性错误，重试没有意义——直接走与「还原失败」相同的子集兜底。
        if !node_in_group {
            logging!(
                info,
                Type::Core,
                "跳过 PROXY 组节点恢复：目标节点 {node} 不在组成员中，直接按子集兜底"
            );
            let filter_lower = read_filter_text().await;
            select_fallback_in_subset(&mihomo, &filter_lower).await;
            return;
        }

        // 恢复节点选择：优先恢复 snapshot 保存的节点。
        // 【BUG-301】服务核心冷启动的头几十秒，TUN 风暴/流量洪峰会把核心 CPU 打满，
        // IPC 请求可能接连 4 秒超时（现场实证：恢复一次超时即放弃，节点留在死节点上，
        // 全系统断网 80 秒直到自愈监控救场）。节点已前置校验在组内，不存在持久性
        // 错误，任何失败都值得重试——给 90 秒总预算，预算耗尽再走兜底。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
        loop {
            match mihomo.select_node_for_group("PROXY", node).await {
                Ok(()) => {
                    logging!(info, Type::Core, "已恢复 PROXY 组节点选择: {}", node);
                    break;
                }
                Err(e) => {
                    if Handle::global().is_exiting() || std::time::Instant::now() >= deadline {
                        logging!(
                            warn,
                            Type::Core,
                            "恢复 PROXY 组节点选择失败 ({}): {}，重试预算耗尽，回退到子集首个可用节点",
                            node,
                            e
                        );
                        // 读取用户保存的 filterText，在子集范围内选点兜底
                        let filter_lower = read_filter_text().await;
                        select_fallback_in_subset(&mihomo, &filter_lower).await;
                        break;
                    }
                    logging!(
                        warn,
                        Type::Core,
                        "恢复 PROXY 组节点选择失败 ({}): {}，3 秒后重试（总预算 90 秒）",
                        node,
                        e
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    }
}

/// 从 proxy_head_state.json 读取当前 profile 的 PROXY 组 filterText
async fn read_filter_text() -> String {
    let current_uid = match Config::profiles().await.latest_arc().current.as_ref() {
        Some(uid) => uid.clone(),
        None => return String::new(),
    };
    let Some(path) = crate::utils::dirs::app_home_dir()
        .ok()
        .map(|d| d.join("proxy_head_state.json"))
    else {
        return String::new();
    };
    let Some(content) = tokio::fs::read_to_string(&path).await.ok() else {
        return String::new();
    };
    let Some(value) = serde_json::from_str::<serde_json::Value>(&content).ok() else {
        return String::new();
    };
    value[current_uid.as_str()]["PROXY"]["filterText"]
        .as_str()
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default()
}

/// 在 filterText 子集内选择首个可用节点作为 PROXY 兜底
async fn select_fallback_in_subset(mihomo: &Mihomo, filter_lower: &str) {
    let Ok(group_info) = mihomo.get_group_by_name("PROXY").await else {
        return;
    };
    let Some(all) = group_info.all else { return };
    let fallback = all
        .iter()
        .filter(|n| !crate::utils::node::is_dummy_node(n))
        .find(|n| filter_lower.is_empty() || n.to_lowercase().contains(filter_lower))
        .or_else(|| all.iter().find(|n| !crate::utils::node::is_dummy_node(n)));
    if let Some(fb) = fallback {
        match mihomo.select_node_for_group("PROXY", fb).await {
            Ok(()) => logging!(info, Type::Core, "已回退 PROXY 组节点选择到: {}", fb),
            Err(e) => logging!(warn, Type::Core, "PROXY 组回退节点选择也失败: {}", e),
        }
    }
}
