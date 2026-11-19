use super::{CoreManager, RunningMode};
use crate::config::Config;
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;
use tauri_plugin_mihomo::Mihomo;

/// 内置出站（非真实代理节点）；快照与持久化恢复均跳过它们。
fn is_builtin_outbound(name: &str) -> bool {
    matches!(name, "DIRECT" | "REJECT" | "REJECT-DROP" | "PASS" | "COMPATIBLE")
}

impl CoreManager {
    pub async fn start_core(&self) -> Result<()> {
        // 先在锁外等待服务就绪（可能阻塞很久，如UAC弹窗、服务启动等）
        // 这样 stop/restart 不会被长时间阻塞
        self.await_service_ready_if_needed().await;

        let _life = self.lifecycle_lock.lock().await;
        self.start_core_inner(true).await
    }

    async fn start_core_inner(&self, ensure_selection_async: bool) -> Result<()> {
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

        // 启动失败时回滚 mode；冷启动成功时异步兜底校验 PROXY 选点与清理越界连接（BUG-405）
        if result.is_err() {
            self.set_running_mode(RunningMode::NotRunning);
        } else if ensure_selection_async {
            crate::process::AsyncHandler::spawn(|| async move {
                Self::global().ensure_proxy_group_selection(None).await;
            });
        }

        result
    }

    pub async fn stop_core(&self) -> Result<()> {
        let _life = self.lifecycle_lock.lock().await;
        self.stop_core_inner().await
    }

    async fn stop_core_inner(&self) -> Result<()> {
        CLASH_LOGGER.clear_logs().await;

        // 必须清空 IPC 连接池：内核重启后旧命名管道连接句柄已失效
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
        // 锁外等待系统服务就绪（与 start_core 同源），防止 TUN 模式下读取陈旧服务状态
        self.await_service_ready_if_needed().await;

        let _life = self.lifecycle_lock.lock().await;
        logging!(info, Type::Core, "Restarting core");

        // 重启前记录过滤合规的目标选点（快照优先，缺失/越界回退持久化）
        let saved_proxy_now = self.resolve_desired_proxy_selection().await;

        self.stop_core_inner().await?;
        self.start_core_inner(false).await?;

        self.ensure_proxy_group_selection(saved_proxy_now.as_deref()).await;

        Handle::refresh_clash();
        crate::module::monitor::MONITOR_WAKEUP_NOTIFY.notify_one();

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

    pub(super) fn after_core_process(&self) {
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
            match service::inspect_service_registration_async().await {
                service::ScmServiceRegistration::NotInstalled => {
                    // 1. 完全未安装，尝试进行安装（触发 UAC）
                    logging!(info, Type::Service, "检测到系统服务未安装，启动安装提权");
                    SERVICE_MANAGER
                        .handle_service_status(ServiceStatus::InstallRequired)
                        .await?;
                }
                service::ScmServiceRegistration::DisabledOrBroken => {
                    // 2. 服务已在 SCM 被手动禁用（Start=4）或可执行文件缺失，属确定性不可用，不再白等 30s 轮询窗口（BUG-328）
                    anyhow::bail!("系统服务已被禁用或服务程序缺失，无法启动");
                }
                service::ScmServiceRegistration::InstalledAndEnabled => {
                    // 3. 服务已正常注册，若冷启动初次刷新暂未就绪（如 SCM 仍在拉起 AutoStart 服务），
                    // 记录警告并继续进入下方 30s 轮询等待，禁止直接短路退出（BUG-328）。
                    if let Err(e) = SERVICE_MANAGER.refresh().await {
                        logging!(warn, Type::Service, "初次刷新已安装服务状态未就绪，继续轮询等待: {e}");
                    }
                }
            }

            // 4. 轮询等待服务就绪
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
            let reg = service::inspect_service_registration_async().await;
            let is_transient_timeout = matches!(reg, service::ScmServiceRegistration::InstalledAndEnabled);
            if is_transient_timeout {
                logging!(
                    error,
                    Type::Service,
                    "已启用系统服务在超时窗口内未就绪，本次会话临时回退至系统代理（锁定磁盘 TUN 配置不被后续落盘覆写）"
                );
                Handle::notice_message(
                    "warning",
                    "系统服务响应超时，本次启动已临时切换为系统代理模式（下次启动将自动重试 TUN）",
                );
            } else {
                logging!(
                    error,
                    Type::Service,
                    "系统服务不可用 ({reg:?})，执行持久化回退至系统代理"
                );
                // 【前端通知】静默回退曾让用户误以为 TUN 已生效（BUG-298：开关显示成功
                // 但内核实为 Sidecar，流量不被接管）。回退必须明确告知。
                Handle::notice_message("warning", "系统服务未就绪，TUN 模式无法生效，已自动回退为系统代理模式");
            }
            self.fallback_to_system_proxy(!is_transient_timeout).await;
        }
    }

    async fn fallback_to_system_proxy(&self, persist_to_disk: bool) {
        logging!(
            warn,
            Type::Service,
            "检测到系统服务未就绪，执行配置回退（persist_to_disk={}）：关闭 TUN 模式，开启系统代理",
            persist_to_disk
        );
        // 不经 feat::patch_verge_with_resets（避免与 start/restart 锁外等待构成递归死锁）；
        // 会话级瞬时超时回退不写盘以保留用户原始 TUN 意图，并手动发 Handle::refresh_verge 同步 UI。
        {
            let verge = Config::verge().await;
            if persist_to_disk {
                crate::config::IVerge::clear_temp_tun_fallback();
            } else {
                let current = verge.latest_arc();
                crate::config::IVerge::enter_temp_tun_fallback(current.enable_tun_mode, current.enable_system_proxy);
            }
            verge.edit_draft(|d| {
                d.enable_tun_mode = Some(false);
                d.enable_system_proxy = Some(true);
            });
            verge.apply();
            if persist_to_disk {
                let data = verge.data_arc();
                if let Err(e) = data.save_file() {
                    logging!(error, Type::Service, "应用回退配置失败: {}", e);
                }
            }
        }
        if let Err(e) = crate::core::sysopt::Sysopt::global().update_sysproxy().await {
            logging!(error, Type::Service, "回退应用系统代理失败: {}", e);
        }
        crate::core::sysopt::Sysopt::global().refresh_guard().await;
        Handle::refresh_verge();
    }

    /// 快照当前 PROXY 组的选中节点（用于重启/升级内核后恢复）
    pub async fn snapshot_proxy_group_now(&self) -> Option<std::string::String> {
        if Handle::global().is_exiting() {
            return None;
        }
        let mihomo = Handle::mihomo().await.clone();
        match mihomo.get_group_by_name("PROXY").await {
            Ok(group) => match group.now {
                Some(node)
                    if !node.is_empty()
                        && !is_builtin_outbound(&node)
                        && !crate::utils::node::is_dummy_node(&node)
                        && !crate::utils::node::is_confirmed_fake_node(&node) =>
                {
                    logging!(info, Type::Core, "快照 PROXY 组当前节点: {}", node);
                    Some(node)
                }
                _ => None,
            },
            Err(e) => {
                logging!(warn, Type::Core, "快照 PROXY 组失败（将尝试读取持久化选点）: {}", e);
                None
            }
        }
    }

    /// 综合实时快照、持久化 `profiles.yaml::selected` 与用户 `filterText`，解析重启/重载前合规的目标节点（BUG-405）。
    pub async fn resolve_desired_proxy_selection(&self) -> Option<std::string::String> {
        let (persisted, raw_filter) = crate::enhance::read_persisted_proxy_and_filter().await;
        let effective_filter = crate::utils::node::effective_filter_text(&raw_filter);
        let is_compliant = |node: &str| {
            !node.is_empty()
                && !is_builtin_outbound(node)
                && !crate::utils::node::is_dummy_node(node)
                && !crate::utils::node::is_confirmed_fake_node(node)
                && crate::utils::node::match_node_filter(node, effective_filter)
        };

        if let Some(snap) = self.snapshot_proxy_group_now().await {
            if is_compliant(&snap) {
                return Some(snap);
            }
            logging!(
                warn,
                Type::Core,
                "快照节点 {} 不符合当前过滤范围「{}」，尝试回退持久化合规选点",
                snap,
                effective_filter
            );
        }

        persisted.filter(|node| is_compliant(node))
    }

    /// 统一保障内核启动 / 重启 / 重载后的 PROXY 组选点合规性，并在收尾清理滞留于已排除节点的代理连接（BUG-405）。
    pub async fn ensure_proxy_group_selection(&self, preferred: Option<&str>) {
        if Handle::global().is_exiting() {
            return;
        }

        let mihomo = Handle::mihomo().await.clone();

        // 等待内核 API 就绪（总上限 30s，前 2s 使用 50ms 快速轮询压缩窗口期）
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
            let step_ms = if start.elapsed().as_secs() < 2 { 50 } else { 200 };
            tokio::time::sleep(std::time::Duration::from_millis(step_ms)).await;
        }
        if !api_ready {
            logging!(warn, Type::Core, "等待内核 API 就绪超时，跳过 PROXY 组节点恢复");
            return;
        }

        // 等待 PROXY 组节点列表填充（总上限 20s，前 2s 使用 50ms 快速轮询）
        let start2 = std::time::Instant::now();
        let mut loaded_group: Option<(Vec<String>, Option<String>)> = None;
        while start2.elapsed().as_secs() < 20 {
            if Handle::global().is_exiting() {
                return;
            }
            if let Ok(group_info) = mihomo.get_group_by_name("PROXY").await
                && let Some(all) = group_info.all
                && !all.is_empty()
            {
                loaded_group = Some((all, group_info.now));
                break;
            }
            let step_ms = if start2.elapsed().as_secs() < 2 { 50 } else { 200 };
            tokio::time::sleep(std::time::Duration::from_millis(step_ms)).await;
        }
        let Some((all, current_now)) = loaded_group else {
            logging!(warn, Type::Core, "等待 PROXY 组节点列表填充超时，跳过选点恢复");
            return;
        };

        let (persisted, raw_filter) = crate::enhance::read_persisted_proxy_and_filter().await;
        let effective_filter = crate::utils::node::effective_filter_text(&raw_filter);

        let is_compliant_member = |n: &str| {
            !n.is_empty()
                && !is_builtin_outbound(n)
                && !crate::utils::node::is_dummy_node(n)
                && !crate::utils::node::is_confirmed_fake_node(n)
                && crate::utils::node::match_node_filter(n, effective_filter)
                && all.iter().any(|m| m == n)
        };

        let desired_target: Option<String> = if let Some(pref) = preferred {
            if is_compliant_member(pref) {
                Some(pref.to_string())
            } else {
                logging!(
                    info,
                    Type::Core,
                    "跳过 PROXY 组节点恢复：{} 属内置/假节点、不在成员中或已被过滤词「{}」排除，改按子集兜底",
                    pref,
                    effective_filter
                );
                None
            }
        } else if current_now.as_deref().is_some_and(is_compliant_member) {
            current_now.clone()
        } else {
            persisted.filter(|p| is_compliant_member(p))
        };

        if let Some(target) = desired_target {
            if current_now.as_deref() == Some(target.as_str()) {
                logging!(debug, Type::Core, "PROXY 组已落位合规出口节点: {}", target);
            } else {
                // 90s 总预算重试（TUN 流量洪峰下 IPC 可能短暂超时）
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
                loop {
                    match mihomo.select_node_for_group("PROXY", &target).await {
                        Ok(()) => {
                            logging!(info, Type::Core, "已恢复 PROXY 组节点选择: {}", target);
                            break;
                        }
                        Err(e) => {
                            if Handle::global().is_exiting() || std::time::Instant::now() >= deadline {
                                logging!(
                                    warn,
                                    Type::Core,
                                    "恢复 PROXY 组节点选择失败 ({}): {}，重试预算耗尽，回退到子集首个可用节点",
                                    target,
                                    e
                                );
                                select_fallback_in_subset(&mihomo, &all, effective_filter).await;
                                break;
                            }
                            logging!(
                                warn,
                                Type::Core,
                                "恢复 PROXY 组节点选择失败 ({}): {}，3 秒后重试（总预算 90 秒）",
                                target,
                                e
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        }
                    }
                }
            }
        } else {
            select_fallback_in_subset(&mihomo, &all, effective_filter).await;
        }

        // 收尾清理：清除启动/重载窗口内落入已排除节点上的代理连接
        crate::module::monitor::close_excluded_proxy_connections(&raw_filter).await;
    }
}

/// 在 filterText 子集内选择首个可用节点作为 PROXY 兜底，并同步持久化 `profile.selected`。
async fn select_fallback_in_subset(mihomo: &Mihomo, all: &[String], filter_text: &str) {
    let fallback = crate::utils::node::pick_subset_fallback_node(
        all,
        filter_text,
        |n| {
            !is_builtin_outbound(n)
                && !crate::utils::node::is_dummy_node(n)
                && !crate::utils::node::is_confirmed_fake_node(n)
        },
        crate::utils::node::is_verified_true_node,
    );
    if let Some(fb) = fallback {
        match mihomo.select_node_for_group("PROXY", fb).await {
            Ok(()) => {
                logging!(info, Type::Core, "已回退 PROXY 组节点选择到: {}", fb);
                if let Some(uid) = crate::module::monitor::get_current_profile_uid().await {
                    crate::module::monitor::persist_selected_node_if_matching(&uid, fb).await;
                }
            }
            Err(e) => logging!(warn, Type::Core, "PROXY 组回退节点选择也失败: {}", e),
        }
    } else if !filter_text.trim().is_empty() {
        logging!(
            info,
            Type::Core,
            "PROXY 组在过滤范围「{}」内无可用兜底节点，遵循边界约束跳过越界切点",
            filter_text.trim()
        );
    }
}
