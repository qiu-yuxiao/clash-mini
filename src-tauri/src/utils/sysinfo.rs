use parking_lot::RwLock;
use std::{
    fmt::{Debug, Display},
    time::Instant,
};
use sysinfo::{Networks, System};
use tauri::{AppHandle, Manager as _, Runtime, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[cfg(windows)]
use deelevate::{PrivilegeLevel, Token};
#[cfg(unix)]
use libc;

pub struct SysInfo {
    system_name: String,
    system_version: String,
    system_kernel_version: String,
    system_arch: String,
}

impl Default for SysInfo {
    #[inline]
    fn default() -> Self {
        let system_name = System::name().unwrap_or_else(|| "Null".into());
        let system_version = System::long_os_version().unwrap_or_else(|| "Null".into());
        let system_kernel_version = System::kernel_version().unwrap_or_else(|| "Null".into());
        let system_arch = System::cpu_arch();
        Self {
            system_name,
            system_version,
            system_kernel_version,
            system_arch,
        }
    }
}

pub struct AppInfo {
    app_version: String,
    app_core_mode: String,
    pub app_startup_time: Instant,
    pub app_is_admin: bool,
}

impl Default for AppInfo {
    #[inline]
    fn default() -> Self {
        Self {
            app_version: "0.0.0".into(),
            app_core_mode: "NotRunning".into(),
            app_is_admin: false,
            app_startup_time: Instant::now(),
        }
    }
}

#[derive(Default)]
pub struct Platform {
    pub sysinfo: SysInfo,
    pub appinfo: AppInfo,
}

impl Debug for Platform {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Platform")
            .field("system_name", &self.sysinfo.system_name)
            .field("system_version", &self.sysinfo.system_version)
            .field("system_kernel_version", &self.sysinfo.system_kernel_version)
            .field("system_arch", &self.sysinfo.system_arch)
            .field("app_version", &self.appinfo.app_version)
            .field("app_core_mode", &self.appinfo.app_core_mode)
            .field("app_is_admin", &self.appinfo.app_is_admin)
            .finish()
    }
}

impl Display for Platform {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "System Name: {}\nSystem Version: {}\nSystem kernel Version: {}\nSystem Arch: {}\nVerge Version: {}\nRunning Mode: {}\nIs Admin: {}",
            self.sysinfo.system_name,
            self.sysinfo.system_version,
            self.sysinfo.system_kernel_version,
            self.sysinfo.system_arch,
            self.appinfo.app_version,
            self.appinfo.app_core_mode,
            self.appinfo.app_is_admin
        )
    }
}

#[inline]
fn is_binary_admin() -> bool {
    #[cfg(not(windows))]
    unsafe {
        libc::geteuid() == 0
    }
    #[cfg(windows)]
    Token::with_current_process()
        .and_then(|token| token.privilege_level())
        .map(|level| level != PrivilegeLevel::NotPrivileged)
        .unwrap_or(false)
}

#[inline]
#[cfg(unix)]
pub fn current_gid() -> u32 {
    unsafe { libc::getgid() }
}

#[inline]
pub fn list_network_interfaces() -> Vec<String> {
    let mut networks = Networks::new();
    networks.refresh(false);
    networks.keys().map(|name| name.to_owned()).collect()
}

#[inline]
pub fn set_app_core_mode<R: Runtime>(app: &AppHandle<R>, mode: impl Into<String>) {
    let platform_spec = app.state::<RwLock<Platform>>();
    let mut spec = platform_spec.write();
    spec.appinfo.app_core_mode = mode.into();
}

#[inline]
pub fn get_app_uptime<R: Runtime>(app: &AppHandle<R>) -> Instant {
    let platform_spec = app.state::<RwLock<Platform>>();
    let spec = platform_spec.read();
    spec.appinfo.app_startup_time
}

#[inline]
pub fn is_current_app_handle_admin<R: Runtime>(app: &AppHandle<R>) -> bool {
    let platform_spec = app.state::<RwLock<Platform>>();
    let spec = platform_spec.read();
    spec.appinfo.app_is_admin
}

/// 初始化 Platform 状态并注册到 AppHandle
#[inline]
pub fn init_platform<R: Runtime>(app: &tauri::App<R>) {
    let app_version = app.package_info().version.to_string();
    let is_admin = is_binary_admin();

    let mut platform = Platform::default();
    platform.appinfo.app_version = app_version;
    platform.appinfo.app_is_admin = is_admin;

    app.manage(RwLock::new(platform));
}

// ===== Tauri 命令 =====

#[tauri::command]
pub fn get_system_info(state: State<'_, RwLock<Platform>>) -> Result<String, String> {
    Ok(state.inner().read().to_string())
}

#[tauri::command]
pub fn get_app_uptime_cmd(state: State<'_, RwLock<Platform>>) -> Result<u128, String> {
    Ok(state.inner().read().appinfo.app_startup_time.elapsed().as_millis())
}

#[tauri::command]
pub fn app_is_admin(state: State<'_, RwLock<Platform>>) -> Result<bool, String> {
    Ok(state.inner().read().appinfo.app_is_admin)
}

#[tauri::command]
pub fn export_diagnostic_info<R: Runtime>(
    app_handle: AppHandle<R>,
    state: State<'_, RwLock<Platform>>,
) -> Result<(), String> {
    let info = state.inner().read().to_string();
    app_handle
        .clipboard()
        .write_text(info)
        .map_err(|e| e.to_string())
}
