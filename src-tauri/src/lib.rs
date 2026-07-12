// Copyright (c) 2026 秋雨潇潇 <qiuyuxiao@gmail.com> (Portions relating to modifications)
// SPDX-License-Identifier: GPL-3.0-only

#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::collapsible_if, clippy::cognitive_complexity, clippy::bool_assert_comparison)]
#![recursion_limit = "512"]

mod cmd;
pub mod config;
mod constants;
mod core;
mod enhance;
mod feat;
mod module;
mod process;
pub mod utils;

use crate::constants::files;
use crate::{
    core::handle,
    process::AsyncHandler,
    utils::{resolve, server},
};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use once_cell::sync::OnceCell;
use tauri::{AppHandle, Manager as _};
#[cfg(target_os = "macos")]
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_deep_link::DeepLinkExt as _;
use tauri_plugin_mihomo::RejectPolicy;

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
/// Application initialization helper functions
mod app_init {
    use super::*;

    /// Initialize singleton monitoring for other instances
    pub fn init_singleton_check() -> Result<()> {
        AsyncHandler::block_on(async move {
            logging!(info, Type::Setup, "开始检查单例实例...");
            server::check_singleton().await?;
            Ok(())
        })
    }

    /// Setup plugins for the Tauri builder
    pub fn setup_plugins(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
        #[allow(unused_mut)]
        let mut builder = builder
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_clipboard_manager::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_deep_link::init())
            .plugin(tauri_plugin_http::init())
            .plugin(
                tauri_plugin_mihomo::Builder::new()
                    .protocol(tauri_plugin_mihomo::models::Protocol::LocalSocket)
                    .socket_path(crate::config::IClashTemp::guard_external_controller_ipc())
                    .pool_config(
                        tauri_plugin_mihomo::IpcPoolConfigBuilder::new()
                            .min_connections(3)
                            .max_connections(16)
                            .idle_timeout(std::time::Duration::from_secs(30))
                            .health_check_interval(std::time::Duration::from_secs(60))
                            .reject_policy(RejectPolicy::Wait)
                            .build(),
                    )
                    .build(),
            );

        // Devtools plugin only in debug mode with feature tauri-dev
        // to avoid duplicated registering of logger since the devtools plugin also registers a logger
        #[cfg(all(debug_assertions, not(feature = "tokio-trace"), feature = "tauri-dev"))]
        {
            builder = builder.plugin(tauri_plugin_devtools::init());
        }
        builder
    }

    /// Setup deep link handling
    pub fn setup_deep_links(app: &tauri::App) {
        #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
        {
            logging!(info, Type::Setup, "注册深层链接...");
            let _ = app.deep_link().register_all();
        }

        app.deep_link().on_open_url(|event| {
            let urls = event.urls();
            AsyncHandler::spawn(move || async move {
                if let Some(url) = urls.first()
                    && let Err(e) = resolve::resolve_scheme(url.as_ref()).await
                {
                    logging!(error, Type::Setup, "Failed to resolve scheme: {}", e);
                }
            });
        });
    }

    /// Setup autostart plugin
    pub fn setup_autostart(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        let mut auto_start_plugin_builder = tauri_plugin_autostart::Builder::new();
        #[cfg(not(target_os = "macos"))]
        let auto_start_plugin_builder = tauri_plugin_autostart::Builder::new();

        #[cfg(target_os = "macos")]
        {
            auto_start_plugin_builder = auto_start_plugin_builder
                .macos_launcher(MacosLauncher::LaunchAgent)
                .app_name(&app.config().identifier);
        }
        app.handle().plugin(auto_start_plugin_builder.build())?;
        Ok(())
    }

    /// Setup window state management
    pub fn setup_window_state(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
        logging!(info, Type::Setup, "初始化窗口状态管理...");
        let window_state_plugin = tauri_plugin_window_state::Builder::new()
            .with_filename(files::WINDOW_STATE)
            .with_state_flags(tauri_plugin_window_state::StateFlags::default())
            .build();
        app.handle().plugin(window_state_plugin)?;
        Ok(())
    }

    pub fn generate_handlers() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
        tauri::generate_handler![
            crate::utils::sysinfo::get_app_uptime,
            crate::utils::sysinfo::app_is_admin,
            cmd::is_port_in_use,
            cmd::get_sys_proxy,
            cmd::get_auto_proxy,
            cmd::open_logs_dir,
            cmd::open_core_dir,
            cmd::restart_core,
            cmd::get_running_mode,
            cmd::install_service,
            cmd::uninstall_service,
            cmd::is_service_available,
            cmd::get_clash_info,
            cmd::patch_clash_config,
            cmd::patch_clash_mode,
            cmd::get_runtime_config,
            cmd::update_proxy_chain_config_in_runtime,
            cmd::get_proxy_addr,
            cmd::save_proxy_head_state,
            cmd::get_proxy_head_state,
            cmd::get_clash_logs,
            cmd::close_all_connections,
            cmd::get_verge_config,
            cmd::patch_verge_config,
            cmd::trigger_auto_select,
            cmd::download_icon_cache,
            cmd::open_devtools,
            cmd::get_profiles,
            cmd::enhance_profiles,
            cmd::patch_profiles_config,
            cmd::view_profile,
            cmd::patch_profile,
            cmd::import_profile,
            cmd::delete_profile,
            cmd::read_profile_file,
            cmd::save_profile_file,
            cmd::update_profile,
            cmd::get_unlock_items,
            cmd::check_media_unlock,
            cmd::check_core_update,
            cmd::start_core_upgrade,
        ]
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn show_error_dialog(title: &str, message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt as _;
    unsafe extern "system" {
        fn MessageBoxW(hwnd: *mut std::ffi::c_void, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    }
    let wide_message: Vec<u16> = OsStr::new(message).encode_wide().chain(Some(0)).collect();
    let wide_title: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_message.as_ptr(),
            wide_title.as_ptr(),
            0x00000010, // MB_ICONERROR
        );
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn show_error_dialog(title: &str, message: &str) {
    eprintln!("[{}] {}", title, message);
    let script = format!(
        "display dialog {:?} with title {:?} buttons {{\"OK\"}} default button \"OK\" with icon stop",
        message, title
    );
    let _ = std::process::Command::new("osascript").args(["-e", &script]).spawn();
}

#[cfg(target_os = "linux")]
pub(crate) fn show_error_dialog(title: &str, message: &str) {
    eprintln!("[{}] {}", title, message);
    let _ = std::process::Command::new("zenity")
        .args(["--error", &format!("--title={}", title), &format!("--text={}", message)])
        .spawn();
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub(crate) fn show_error_dialog(title: &str, message: &str) {
    eprintln!("[{}] {}", title, message);
}

pub fn run() {
    if let Err(err) = app_init::init_singleton_check() {
        let msg = format!(
            "Clash Mini is already running or another process is using its port.\nError details: {}",
            err
        );
        show_error_dialog("Clash Mini Startup Error", &msg);
        return;
    }

    #[cfg(target_os = "linux")]
    utils::linux::workarounds::apply_nvidia_dmabuf_renderer_workaround();
    #[cfg(target_os = "linux")]
    utils::linux::workarounds::apply_wayland_webkit_fix();

    let _ = utils::dirs::init_portable_flag();

    let builder = app_init::setup_plugins(tauri::Builder::default())
        .setup(|app| {
            // SAFETY: APP_HANDLE 是应用启动时初始化的全局单例，
            // 仅在 setup 阶段设置一次，此时必定未被初始化，因此 expect 不会触发。
            // 若设置失败说明存在严重的初始化顺序问题，应尽早暴露而非静默继续。
            #[allow(clippy::expect_used)]
            APP_HANDLE
                .set(app.app_handle().clone())
                .expect("failed to set global app handle");

            resolve::init_work_dir_and_logger()?;

            let app_handle = app.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let is_updating = crate::core::updater::SilentUpdater::global()
                    .try_install_on_startup(&app_handle)
                    .await;
                if is_updating {
                    std::process::exit(0);
                }
            });

            let app_handle_bg = app.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                crate::core::updater::SilentUpdater::global()
                    .start_background_check(app_handle_bg)
                    .await;
            });

            logging!(info, Type::Setup, "开始应用初始化...");
            if let Err(e) = app_init::setup_autostart(app) {
                logging!(error, Type::Setup, "Failed to setup autostart: {}", e);
            }

            app_init::setup_deep_links(app);

            if let Err(e) = app_init::setup_window_state(app) {
                logging!(error, Type::Setup, "Failed to setup window state: {}", e);
            }

            resolve::resolve_setup_async();
            resolve::resolve_setup_sync();
            resolve::init_signal();

            // 初始化平台系统信息
            crate::utils::sysinfo::init_platform(app);

            logging!(info, Type::Setup, "初始化已启动");
            Ok(())
        })
        .invoke_handler(app_init::generate_handlers())
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let Some(webview_window) = window.get_webview_window("main") {
                    match event {
                        tauri::WindowEvent::CloseRequested { .. } => {
                            event_handlers::handle_window_close(&webview_window, event);
                        }
                        tauri::WindowEvent::Focused(focused) => {
                            event_handlers::handle_window_focus(*focused);
                        }
                        tauri::WindowEvent::Resized(new_size) => {
                            event_handlers::handle_window_resized(&webview_window, *new_size);
                        }
                        #[cfg(target_os = "macos")]
                        tauri::WindowEvent::Destroyed => {
                            event_handlers::handle_window_destroyed();
                        }
                        _ => {}
                    }
                }
            }
        });

    mod event_handlers {
        use crate::module::lightweight;
        use crate::{
            config::Config,
            core::{self, handle, hotkey},
            process::AsyncHandler,
        };
        use clash_verge_logging::{Type, logging};
        use tauri::AppHandle;
        #[cfg(target_os = "macos")]
        use tauri::Manager as _;

        pub fn handle_ready_resumed(_app_handle: &AppHandle) {
            use tauri::Manager as _;

            if handle::Handle::global().is_exiting() {
                logging!(debug, Type::System, "应用正在退出，跳过处理");
                return;
            }

            logging!(info, Type::System, "应用就绪");

            #[cfg(target_os = "windows")]
            if let Some(window) = _app_handle.get_webview_window("main") {
                setup_wm_sizing_hook(&window);
            }

            #[cfg(target_os = "macos")]
            if let Some(window) = _app_handle.get_webview_window("main") {
                let _ = window.set_title("Clash Mini");
            }
        }

        #[cfg(target_os = "macos")]
        pub async fn handle_reopen(has_visible_windows: bool) {
            if lightweight::is_in_lightweight_mode() {
                lightweight::exit_lightweight_mode().await;
                return;
            }

            if !has_visible_windows {
                handle::Handle::global().set_activation_policy_regular();
                let _ = crate::utils::window_manager::WindowManager::show_main_window().await;
            }
        }

        pub fn handle_window_close(window: &tauri::WebviewWindow, event: &tauri::WindowEvent) {
            #[cfg(target_os = "macos")]
            handle::Handle::global().set_activation_policy_accessory();

            if core::handle::Handle::global().is_exiting() {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                AsyncHandler::spawn(|| async {
                    let entered = lightweight::entry_lightweight_mode().await;
                    if !entered {
                        logging!(
                            error,
                            Type::Lightweight,
                            "[窗口关闭] 轻量模式进入失败，尝试恢复窗口显示"
                        );
                        let _ = crate::utils::window_manager::WindowManager::show_main_window().await;
                    }
                });
            }
        }

        pub const fn handle_window_resized(_window: &tauri::WebviewWindow, _new_size: tauri::PhysicalSize<u32>) {
            // 窗口最小尺寸由 WM_SIZING 子类化处理器在 Rust 层面拦截并钳制位置和尺寸，
            // 无需在此事后调用 set_size/set_position 与原生缩放循环竞争。
        }

        pub fn handle_window_focus(focused: bool) {
            AsyncHandler::spawn(move || async move {
                let is_enable_global_hotkey = Config::verge().await.data_arc().enable_global_hotkey.unwrap_or(true);

                if focused {
                    #[cfg(target_os = "macos")]
                    {
                        use crate::core::hotkey::SystemHotkey;
                        let _ = hotkey::Hotkey::global()
                            .register_system_hotkey(SystemHotkey::CmdQ)
                            .await;
                        let _ = hotkey::Hotkey::global()
                            .register_system_hotkey(SystemHotkey::CmdW)
                            .await;
                    }
                    if !is_enable_global_hotkey {
                        let _ = hotkey::Hotkey::global().init(false).await;
                    }
                    return;
                }

                #[cfg(target_os = "macos")]
                {
                    use crate::core::hotkey::SystemHotkey;
                    let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdQ);
                    let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdW);
                }

                if !is_enable_global_hotkey {
                    let _ = hotkey::Hotkey::global().reset();
                }
            });
        }

        #[cfg(target_os = "windows")]
        use std::sync::atomic::{AtomicPtr, Ordering};

        #[cfg(target_os = "windows")]
        static OLD_WNDPROC: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

        /// 安装 WM_SIZING 消息处理器，防止上边缘缩放到最小高度后窗口 Y 坐标继续下移。
        ///
        /// 标准 WM_GETMINMAXINFO 只钳制尺寸不钳制位置，而上边缘缩放时 Windows 会持续增加 Y 坐标，
        /// 导致窗口缩到最小时整体向下平移。WM_SIZING 在系统应用矩形之前给出提议矩形，在此处
        /// 同时钳制尺寸和 Y 位置可根除该问题。
        #[cfg(target_os = "windows")]
        fn setup_wm_sizing_hook(window: &tauri::WebviewWindow) {
            use raw_window_handle::HasWindowHandle as _;
            use windows::Win32::UI::WindowsAndMessaging::{GWLP_WNDPROC, SetWindowLongPtrW};

            let handle = match window.window_handle() {
                Ok(h) => h,
                Err(_) => return,
            };

            let hwnd = match handle.as_raw() {
                raw_window_handle::RawWindowHandle::Win32(h) => {
                    windows::Win32::Foundation::HWND(h.hwnd.get() as *mut std::ffi::c_void)
                }
                _ => return,
            };

            unsafe {
                let new_proc = Some(sizing_wndproc as unsafe extern "system" fn(_, _, _, _) -> _);
                let old_proc_val = SetWindowLongPtrW(
                    hwnd,
                    GWLP_WNDPROC,
                    std::mem::transmute::<
                        Option<
                            unsafe extern "system" fn(
                                windows::Win32::Foundation::HWND,
                                u32,
                                windows::Win32::Foundation::WPARAM,
                                windows::Win32::Foundation::LPARAM,
                            )
                                -> windows::Win32::Foundation::LRESULT,
                        >,
                        isize,
                    >(new_proc),
                );
                if old_proc_val != 0 {
                    OLD_WNDPROC.store(old_proc_val as *mut _, Ordering::Release);
                }
            }
        }

        /// WM_SIZING 子类化窗口过程。
        /// 拦截上边缘缩放（WMSZ_TOP / WMSZ_TOPLEFT / WMSZ_TOPRIGHT），
        /// 当提议高度小于最小尺寸时，修正 rect.top，使窗口位置和高度同时被钳制。
        #[cfg(target_os = "windows")]
        unsafe extern "system" fn sizing_wndproc(
            hwnd: windows::Win32::Foundation::HWND,
            msg: u32,
            wparam: windows::Win32::Foundation::WPARAM,
            lparam: windows::Win32::Foundation::LPARAM,
        ) -> windows::Win32::Foundation::LRESULT {
            use crate::utils::resolve::window::MINIMAL_HEIGHT;
            use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
            use windows::Win32::UI::HiDpi::GetDpiForWindow;
            use windows::Win32::UI::WindowsAndMessaging::{
                CallWindowProcW, DefWindowProcW, WM_SIZING, WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
            };

            if msg == WM_SIZING {
                let side = wparam.0 as u32;
                if side == WMSZ_TOP || side == WMSZ_TOPLEFT || side == WMSZ_TOPRIGHT {
                    // SAFETY: lparam 在 WM_SIZING 消息中指向有效的 RECT 结构
                    unsafe {
                        let rect = &mut *(lparam.0 as *mut windows::Win32::Foundation::RECT);
                        let dpi = GetDpiForWindow(hwnd);
                        let scale = (dpi as f64) / 96.0;
                        let min_height_px = (MINIMAL_HEIGHT * scale).round() as i32;
                        let height = rect.bottom - rect.top;
                        if height < min_height_px {
                            rect.top = rect.bottom - min_height_px;
                            return windows::Win32::Foundation::LRESULT(1);
                        }
                    }
                }
            }

            let old_proc = OLD_WNDPROC.load(Ordering::Acquire);
            if !old_proc.is_null() {
                // SAFETY: old_proc 是之前 SetWindowLongPtrW 返回的有效窗口过程
                unsafe {
                    CallWindowProcW(
                        Some(std::mem::transmute::<
                            *mut std::ffi::c_void,
                            unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> windows::Win32::Foundation::LRESULT,
                        >(old_proc)),
                        hwnd,
                        msg,
                        wparam,
                        lparam,
                    )
                }
            } else {
                // SAFETY: DefWindowProcW 是默认窗口过程，对所有参数安全
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }

        #[cfg(target_os = "macos")]
        pub fn handle_window_destroyed() {
            use crate::core::hotkey::SystemHotkey;
            AsyncHandler::spawn(move || async move {
                let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdQ);
                let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdW);
                let is_enable_global_hotkey = Config::verge().await.data_arc().enable_global_hotkey.unwrap_or(true);
                if !is_enable_global_hotkey {
                    let _ = hotkey::Hotkey::global().reset();
                }
            });
        }
    }

    #[cfg(feature = "clippy")]
    let context = tauri::test::mock_context(tauri::test::noop_assets());
    #[cfg(feature = "clippy")]
    let app = builder.build(context).unwrap_or_else(|e| {
        logging!(error, Type::Setup, "Failed to build Tauri application: {}", e);
        std::process::exit(1);
    });

    #[cfg(not(feature = "clippy"))]
    let app = builder.build(tauri::generate_context!()).unwrap_or_else(|e| {
        logging!(error, Type::Setup, "Failed to build Tauri application: {}", e);
        std::process::exit(1);
    });

    app.run(|app_handle, e| match e {
        tauri::RunEvent::Ready | tauri::RunEvent::Resumed => {
            if core::handle::Handle::global().is_exiting() {
                return;
            }
            event_handlers::handle_ready_resumed(app_handle);
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen {
            has_visible_windows, ..
        } => {
            if core::handle::Handle::global().is_exiting() {
                return;
            }
            AsyncHandler::spawn(move || async move {
                event_handlers::handle_reopen(has_visible_windows).await;
            });
        }
        tauri::RunEvent::Exit => {
            logging!(info, Type::System, "Application exited");
        }
        #[allow(unused_variables)]
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            // 处于轻量模式且未在退出流程中时阻止退出，以便先退出轻量模式；
            // 但仅当无退出码（code.is_none()）时才阻止——若带退出码（如系统关机/注销），
            // 应允许退出，避免"阻止退出但无动作"的死锁路径。
            if module::lightweight::is_in_lightweight_mode() && !handle::Handle::global().is_exiting() && code.is_none()
            {
                api.prevent_exit();
            } else if code.is_none() {
                api.prevent_exit();
                if !handle::Handle::global().is_exiting() {
                    AsyncHandler::spawn(|| async {
                        feat::quit().await;
                    });
                }
            }
        }
        _ => {}
    });
}
