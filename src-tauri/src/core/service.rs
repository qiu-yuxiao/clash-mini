use crate::{
    config::{Config, IClashTemp},
    core::logger::Logger,
    utils::dirs,
};
use anyhow::{Context as _, Result, bail};
use backon::{ConstantBuilder, Retryable as _};
use clash_verge_logging::{Type, logging};
use clash_verge_service_ipc::CoreConfig;
use compact_str::CompactString;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use scopeguard::defer;
use std::{
    borrow::Cow,
    env::current_exe,
    future::Future,
    path::{Path, PathBuf},
    process::Command as StdCommand,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Ready,
    StartRequired,
    InstallRequired,
    UninstallRequired,
    ReinstallRequired,
    ForceReinstallRequired,
    Unavailable(String),
}

pub struct ServiceManager {
    // 【注意】此处使用 parking_lot::Mutex，而 monitor.rs 中使用 std::sync::Mutex
    // 两种 Mutex 都是正确的，仅风格不一致。若后续统一，建议全部使用 parking_lot::Mutex
    // （性能更优且无需 unwrap 处理 poison error）
    status: Mutex<ServiceStatus>,
    operation_running: AtomicBool,
}

fn uninstall_service() -> Result<()> {
    logging!(info, Type::Service, "uninstall service");

    use deelevate::{PrivilegeLevel, Token};
    use runas::Command as RunasCommand;
    use std::os::windows::process::CommandExt as _;

    let binary_path = dirs::service_path()?;
    let uninstall_path = binary_path.with_file_name("clash-verge-service-uninstall.exe");

    if !uninstall_path.exists() {
        bail!(format!("uninstaller not found: {uninstall_path:?}"));
    }

    let token = Token::with_current_process()?;
    let level = token.privilege_level()?;
    let status = match level {
        PrivilegeLevel::NotPrivileged => RunasCommand::new(uninstall_path).show(false).status()?,
        _ => StdCommand::new(uninstall_path).creation_flags(0x08000000).status()?,
    };

    if !status.success() {
        bail!(
            "failed to uninstall service with status {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

fn install_service() -> Result<()> {
    use std::process::Output;
    logging!(info, Type::Service, "install service");

    use deelevate::{PrivilegeLevel, Token};
    use runas::Command as RunasCommand;
    use std::os::windows::process::CommandExt as _;

    let binary_path = dirs::service_path()?;
    let install_path = binary_path.with_file_name("clash-verge-service-install.exe");

    if !install_path.exists() {
        bail!(format!("installer not found: {install_path:?}"));
    }

    let token = Token::with_current_process()?;
    let level = token.privilege_level()?;
    let output = match level {
        PrivilegeLevel::NotPrivileged => {
            let status = RunasCommand::new(&install_path).show(false).status()?;
            Output {
                status,
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
        }
        _ => {
            // StdCommand returns Output directly
            StdCommand::new(&install_path).creation_flags(0x08000000).output()?
        }
    };

    if let Some((code, err)) = check_output_error(&output) {
        logging!(
            error,
            Type::Service,
            "failed to install service code: {}, details: {}",
            code,
            err
        );
        bail!("failed to install service code: {}, details: {}", code, err);
    }

    Ok(())
}

fn check_output_error(output: &std::process::Output) -> Option<(i32, Cow<'_, str>)> {
    if output.status.success() {
        return None;
    }
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        return Some((code, stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        return Some((code, stdout));
    }
    Some((code, Cow::Borrowed("Unknown error")))
}

pub fn is_service_installed() -> bool {
    use std::os::windows::process::CommandExt as _;
    let output = std::process::Command::new("sc.exe")
        .arg("query")
        .arg("clash_verge_service")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();

    if let Ok(out) = output {
        let stdout = std::string::String::from_utf8_lossy(&out.stdout);
        out.status.success() && !stdout.contains("does not exist")
    } else {
        false
    }
}

fn start_service() -> Result<()> {
    logging!(info, Type::Service, "start service");

    use deelevate::{PrivilegeLevel, Token};
    use runas::Command as RunasCommand;
    use std::os::windows::process::CommandExt as _;

    let token = Token::with_current_process()?;
    let level = token.privilege_level()?;
    let status = match level {
        PrivilegeLevel::NotPrivileged => RunasCommand::new("sc.exe")
            .arg("start")
            .arg("clash_verge_service")
            .show(false)
            .status()?,
        _ => std::process::Command::new("sc.exe")
            .arg("start")
            .arg("clash_verge_service")
            .creation_flags(0x08000000)
            .status()?,
    };

    if !status.success() {
        bail!("failed to start service with status {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

fn reinstall_service() -> Result<()> {
    logging!(info, Type::Service, "reinstall service");

    // 先卸载服务
    if let Err(err) = uninstall_service() {
        logging!(warn, Type::Service, "failed to uninstall service: {}", err);
    }

    // 再安装服务
    match install_service() {
        Ok(_) => Ok(()),
        Err(err) => {
            bail!(format!("failed to install service: {err}"))
        }
    }
}

/// 强制重装服务（UI修复按钮）
fn force_reinstall_service() -> Result<()> {
    logging!(info, Type::Service, "用户请求强制重装服务");
    reinstall_service().map_err(|err| {
        logging!(error, Type::Service, "强制重装服务失败: {}", err);
        err
    })
}

/// 尝试使用服务启动core
pub(super) async fn start_with_existing_service(config_file: &PathBuf) -> Result<()> {
    logging!(info, Type::Service, "尝试使用现有服务启动核心");

    let verge_config = Config::verge().await;
    let clash_core = verge_config.latest_arc().get_valid_clash_core();
    drop(verge_config);

    let app_dir = dirs::app_home_dir()?;
    let cores_dir = app_dir.join("cores");
    let custom_core_path = cores_dir.join("mini-mihomo.exe");

    let bin_path = if custom_core_path.exists() {
        logging!(
            info,
            Type::Service,
            "Service using custom core: {}",
            custom_core_path.display()
        );
        custom_core_path
    } else {
        current_exe()?.with_file_name(format!("{clash_core}.exe"))
    };

    let payload = clash_verge_service_ipc::ClashConfig {
        core_config: CoreConfig {
            config_path: dirs::path_to_str(config_file)?.into(),
            core_path: dirs::path_to_str(&bin_path)?.into(),
            core_ipc_path: IClashTemp::guard_external_controller_ipc(),
            config_dir: dirs::path_to_str(&dirs::app_home_dir()?)?.into(),
        },
        log_config: Logger::global().service_writer_config()?,
    };

    // 先清理前次会话可能残留的旧内核（崩溃退出时 clean_async 未执行）
    let _ = clash_verge_service_ipc::stop_clash().await;

    // L2-02: stop 后轮询端口是否可用，替代固定 500ms 等待
    let mixed_port = Config::clash().await.data_arc().get_mixed_port();
    let mut waited = 0u64;
    let poll_interval = 50u64;
    let max_wait = 3000u64;
    loop {
        if tokio::net::TcpListener::bind(("127.0.0.1", mixed_port)).await.is_ok() {
            break;
        }
        if waited >= max_wait {
            logging!(warn, Type::Service, "等待端口释放超时({}ms)，继续启动", max_wait);
            break;
        }
        tokio::time::sleep(Duration::from_millis(poll_interval)).await;
        waited += poll_interval;
    }

    let response = clash_verge_service_ipc::start_clash(&payload)
        .await
        .context("无法连接到Clash Verge Service")?;

    if response.code > 0 {
        let err_msg = response.message;
        logging!(error, Type::Service, "启动核心失败: {}", err_msg);
        bail!(err_msg);
    }

    logging!(info, Type::Service, "服务成功启动核心");
    Ok(())
}

// 以服务启动core
pub(super) async fn run_core_by_service(config_file: &PathBuf) -> Result<()> {
    logging!(info, Type::Service, "正在尝试通过服务启动核心");

    SERVICE_MANAGER.refresh().await?;

    logging!(info, Type::Service, "服务已运行且版本匹配，直接使用");
    start_with_existing_service(config_file).await
}

pub(super) async fn get_clash_logs_by_service() -> Result<Vec<CompactString>> {
    logging!(info, Type::Service, "正在获取服务模式下的 Clash 日志");

    let response = clash_verge_service_ipc::get_clash_logs()
        .await
        .context("无法连接到Clash Verge Service")?;

    if response.code > 0 {
        let err_msg = response.message;
        logging!(error, Type::Service, "获取服务模式下的 Clash 日志失败: {}", err_msg);
        bail!(err_msg);
    }

    logging!(info, Type::Service, "成功获取服务模式下的 Clash 日志");
    Ok(response.data.unwrap_or_default())
}

/// 通过服务停止core
pub(super) async fn stop_core_by_service() -> Result<()> {
    logging!(info, Type::Service, "通过服务停止核心 (IPC)");

    let response = clash_verge_service_ipc::stop_clash()
        .await
        .context("无法连接到Clash Verge Service")?;

    if response.code > 0 {
        let err_msg = response.message;
        logging!(error, Type::Service, "停止核心失败: {}", err_msg);
        bail!(err_msg);
    }

    logging!(info, Type::Service, "服务成功停止核心");
    Ok(())
}

/// 检查服务是否正在运行
pub async fn is_service_available() -> Result<()> {
    if let Err(e) = Path::metadata(clash_verge_service_ipc::IPC_PATH.as_ref()) {
        let verge = Config::verge().await;
        let verge_last = verge.latest_arc();
        let is_enable = verge_last.enable_tun_mode.unwrap_or(false);
        if is_enable {
            logging!(warn, Type::Service, "Some issue with service IPC Path: {}", e);
        }
        return Err(e.into());
    }
    clash_verge_service_ipc::connect().await?;
    Ok(())
}

async fn wait_for_service_ipc(manager: &ServiceManager) -> Result<()> {
    let config = ServiceManager::config();

    let backoff = ConstantBuilder::default()
        .with_delay(config.retry_delay)
        .with_max_times(config.max_retries);

    let result = (|| async {
        if !is_service_ipc_path_exists() {
            bail!("IPC path not ready");
        }
        clash_verge_service_ipc::connect().await.map(drop)
    })
    .retry(backoff)
    .await;

    if result.is_ok() {
        manager.set_status(ServiceStatus::Ready);
    } else {
        manager.set_status(ServiceStatus::Unavailable("Waiting for service to be available".into()));
    }

    result
}

pub fn is_service_ipc_path_exists() -> bool {
    Path::new(clash_verge_service_ipc::IPC_PATH).exists()
}

impl ServiceManager {
    pub const fn config() -> clash_verge_service_ipc::IpcConfig {
        clash_verge_service_ipc::IpcConfig {
            default_timeout: Duration::from_millis(150),
            retry_delay: Duration::from_millis(250),
            max_retries: 20,
        }
    }

    pub async fn init(&self) -> Result<()> {
        if let Err(e) = clash_verge_service_ipc::connect().await {
            self.set_status(ServiceStatus::Unavailable(format!("服务连接失败: {e}")));
            return Err(e);
        }
        Ok(())
    }

    /// 以快速无锁方式获取当前服务状态，避免高并发场景下操作执行期间阻塞状态读取者。
    #[allow(clippy::unused_async)]
    pub async fn current(&self) -> ServiceStatus {
        self.status.lock().clone()
    }

    fn set_status(&self, status: ServiceStatus) {
        *self.status.lock() = status;
    }

    async fn run_operation(&self, operation: impl Future<Output = Result<()>>) -> Result<()> {
        if self.operation_running.swap(true, Ordering::AcqRel) {
            bail!("service operation already running");
        }
        defer! {
            self.operation_running.store(false, Ordering::Release);
        }

        operation.await?;

        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        if crate::utils::sysinfo::is_current_app_handle_admin(crate::core::handle::Handle::app_handle()) {
            if is_service_available().await.is_ok() {
                self.set_status(ServiceStatus::Ready);
            } else {
                self.set_status(ServiceStatus::Unavailable("Admin mode, no service needed".into()));
            }
            return Ok(());
        }

        self.run_operation(async {
            if is_service_available().await.is_ok() {
                self.set_status(ServiceStatus::Ready);
                return Ok(());
            }

            if is_service_installed() {
                logging!(info, Type::Service, "服务已安装但未运行，尝试启动服务");
                self.apply_service_status(ServiceStatus::StartRequired).await?;
                return Ok(());
            }

            self.set_status(ServiceStatus::Unavailable("Service not installed".into()));
            Ok(())
        })
        .await
    }

    pub async fn handle_service_status(&self, status: ServiceStatus) -> Result<()> {
        self.run_operation(self.apply_service_status(status)).await
    }

    async fn apply_service_status(&self, status: ServiceStatus) -> Result<()> {
        self.set_status(status.clone());
        match status {
            ServiceStatus::Ready => logging!(info, Type::Service, "服务就绪，直接启动"),
            ServiceStatus::StartRequired => {
                logging!(info, Type::Service, "执行启动服务流程");
                run_service_command(start_service, "start service").await?;
                wait_for_service_ipc(self).await?;
            }
            ServiceStatus::ReinstallRequired => {
                logging!(info, Type::Service, "服务需要重装，执行重装流程");
                run_service_command(reinstall_service, "reinstall service").await?;
                wait_for_service_ipc(self).await?;
            }
            ServiceStatus::ForceReinstallRequired => {
                logging!(info, Type::Service, "服务需要强制重装，执行强制重装流程");
                run_service_command(force_reinstall_service, "force reinstall service").await?;
                wait_for_service_ipc(self).await?;
            }
            ServiceStatus::InstallRequired => {
                if is_service_installed() {
                    logging!(info, Type::Service, "服务已安装但未运行，转换为启动服务");
                    self.set_status(ServiceStatus::StartRequired);
                    run_service_command(start_service, "start service").await?;
                    return wait_for_service_ipc(self).await;
                }
                logging!(info, Type::Service, "需要安装服务，执行安装流程");
                run_service_command(install_service, "install service").await?;
                wait_for_service_ipc(self).await?;
            }
            ServiceStatus::UninstallRequired => {
                logging!(info, Type::Service, "服务需要卸载，执行卸载流程");
                run_service_command(uninstall_service, "uninstall service").await?;
                self.set_status(ServiceStatus::Unavailable("Service Uninstalled".into()));
            }
            ServiceStatus::Unavailable(reason) => {
                logging!(info, Type::Service, "服务不可用: {}，将使用Sidecar模式", reason);
                bail!("服务不可用: {}", reason);
            }
        }

        Ok(())
    }
}

async fn run_service_command(
    operation: impl FnOnce() -> Result<()> + Send + 'static,
    label: &'static str,
) -> Result<()> {
    tokio::task::spawn_blocking(operation)
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking join error: {e}")))
        .with_context(|| format!("{label} failed"))
}

pub static SERVICE_MANAGER: Lazy<ServiceManager> = Lazy::new(|| ServiceManager {
    status: Mutex::new(ServiceStatus::Unavailable("Need Checks".into())),
    operation_running: AtomicBool::new(false),
});
