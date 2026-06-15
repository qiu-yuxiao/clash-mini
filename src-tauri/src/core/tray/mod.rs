use crate::config::IVerge;
use crate::module::lightweight;
use crate::process::AsyncHandler;
use crate::singleton;
use crate::utils::window_manager::WindowManager;
use crate::{Type, feat, logging};
use clash_verge_limiter::{Limiter, SystemClock, SystemLimiter};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::handle;
use anyhow::Result;
use std::time::Duration;
use tauri::{
    AppHandle, Wry, Manager,
    menu::{IsMenuItem, MenuEvent, MenuItem},
};

mod menu_def;
use menu_def::MenuIds;

const TRAY_CLICK_DEBOUNCE_MS: u64 = 300;

#[allow(dead_code)]
pub struct TrayIconState {
    pub tray: std::sync::Arc<tokio::sync::Mutex<Option<TrayIcon>>>,
}

pub struct Tray {
    limiter: SystemLimiter,
}

impl Default for Tray {
    fn default() -> Self {
        Self {
            limiter: Limiter::new(Duration::from_millis(TRAY_CLICK_DEBOUNCE_MS), SystemClock),
        }
    }
}

singleton!(Tray, TRAY);

impl Tray {
    fn new() -> Self {
        Self::default()
    }

    pub fn init(&self, app_handle: &AppHandle) -> Result<()> {
        if handle::Handle::global().is_exiting() {
            logging!(debug, Type::Tray, "应用正在退出，跳过托盘初始化");
            return Ok(());
        }

        logging!(info, Type::Tray, "正在从AppHandle创建静态系统托盘");

        // Load default icon bytes synchronously depending on OS
        let icon_bytes = include_bytes!("../../../icons/tray-icon.png").to_vec();
        let image = tauri::image::Image::from_bytes(&icon_bytes)?;

        let app_handle_clone = app_handle.clone();
        app_handle.run_on_main_thread(move || {
            let quit = match MenuItem::with_id(&app_handle_clone, MenuIds::EXIT, "退出 (Exit)", true, None::<&str>) {
                Ok(item) => item,
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to create exit menu item: {}", e);
                    return;
                }
            };

            let menu = match tauri::menu::MenuBuilder::new(&app_handle_clone).items(&[&quit as &dyn IsMenuItem<Wry>]).build() {
                Ok(m) => m,
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to build static menu: {}", e);
                    return;
                }
            };

            #[cfg(target_os = "linux")]
            let builder = TrayIconBuilder::with_id("clash-mini-dev-tray").icon(image).menu(&menu).icon_as_template(false);

            #[cfg(not(target_os = "linux"))]
            let mut builder = TrayIconBuilder::with_id("clash-mini-dev-tray").icon(image).menu(&menu).icon_as_template(false);

            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                builder = builder.show_menu_on_left_click(false);
            }

            match builder.build(&app_handle_clone) {
                Ok(tray) => {
                    let _ = tray.set_tooltip(Some("Clash Mini"));
                    tray.on_tray_icon_event(on_tray_icon_event);
                    tray.on_menu_event(on_menu_event);
                    app_handle_clone.manage(TrayIconState { tray: std::sync::Arc::new(tokio::sync::Mutex::new(Some(tray))) });
                    log::info!(target: "app", "[Tray] System tray created and managed successfully");
                }
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to build tray icon on main thread: {}", e);
                }
            }
        })?;

        Ok(())
    }

    pub async fn update_click_behavior(&self) -> Result<()> {
        Ok(())
    }

    pub async fn update_menu(&self) -> Result<()> {
        Ok(())
    }

    pub async fn update_icon(&self, _verge: &IVerge) -> Result<()> {
        Ok(())
    }

    pub async fn update_tooltip(&self) -> Result<()> {
        Ok(())
    }

    pub async fn update_part(&self) -> Result<()> {
        Ok(())
    }

    pub async fn update_menu_and_icon(&self) {}

    fn should_handle_tray_click(&self) -> bool {
        let allow = self.limiter.check();
        if !allow {
            logging!(debug, Type::Tray, "tray click rate limited");
        }
        allow
    }

    #[allow(dead_code)]
    pub fn update_speed_task(&self, _enable_tray_speed: bool) {}
}

fn on_tray_icon_event(_tray_icon: &TrayIcon, tray_event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Down,
        ..
    } = tray_event
    {
        if !Tray::global().should_handle_tray_click() {
            return;
        }

        AsyncHandler::spawn(|| async move {
            if !lightweight::exit_lightweight_mode().await {
                WindowManager::show_main_window().await;
            };
        });
    }
}

fn on_menu_event(_: &AppHandle, event: MenuEvent) {
    if !Tray::global().should_handle_tray_click() {
        return;
    }
    if event.id.as_ref().is_empty() {
        return;
    }
    AsyncHandler::spawn(|| async move {
        match event.id.as_ref() {
            MenuIds::EXIT => {
                feat::quit().await;
            }
            _ => {
                logging!(debug, Type::Tray, "Unhandled tray menu event: {:?}", event.id);
            }
        }
    });
}
