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
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{
    AppHandle, Manager as _, Wry,
    menu::{IsMenuItem, MenuEvent, MenuItem},
};

mod menu_def;
use menu_def::MenuIds;

const TRAY_CLICK_DEBOUNCE_MS: u64 = 300;

#[allow(dead_code)]
pub struct TrayIconState {
    pub tray: std::sync::Arc<tokio::sync::Mutex<Option<TrayIcon>>>,
}

/// 轻量模式菜单项引用，用于在进入/退出轻量模式时动态设置 enabled 状态
static LITE_MODE_MENU_ITEM: OnceLock<MenuItem<Wry>> = OnceLock::new();

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
            let lite_mode = match MenuItem::with_id(
                &app_handle_clone,
                MenuIds::LITE_MODE,
                "轻量模式 / Lite mode",
                true,
                None::<&str>,
            ) {
                Ok(item) => {
                    let _ = LITE_MODE_MENU_ITEM.set(item);
                    // 修复编译错误：使用 if let Some(item) 代替 unwrap/expect
                    if let Some(item) = LITE_MODE_MENU_ITEM.get() {
                        item.clone()
                    } else {
                        unreachable!("LITE_MODE_MENU_ITEM 刚刚设置，不可能为 None")
                    }
                }
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to create lite mode menu item: {}", e);
                    return;
                }
            };

            // 根据当前轻量模式状态初始化菜单项的勾选和可用状态
            if let Some(item) = LITE_MODE_MENU_ITEM.get() {
                let _ = item.set_checked(lightweight::is_in_lightweight_mode());
                // 始终保持可点击，以便用户可以切换状态
                let _ = item.set_enabled(true);
            }

            let quit = match MenuItem::with_id(&app_handle_clone, MenuIds::EXIT, "退出 (Exit)", true, None::<&str>) {
                Ok(item) => item,
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to create exit menu item: {}", e);
                    return;
                }
            };

            let menu = match tauri::menu::MenuBuilder::new(&app_handle_clone)
                .items(&[&lite_mode as &dyn IsMenuItem<Wry>, &quit as &dyn IsMenuItem<Wry>])
                .build()
            {
                Ok(m) => m,
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to build static menu: {}", e);
                    return;
                }
            };

            #[cfg(target_os = "linux")]
            let builder = TrayIconBuilder::with_id("clash-mini-dev-tray")
                .icon(image)
                .menu(&menu)
                .icon_as_template(false);

            #[cfg(not(target_os = "linux"))]
            let mut builder = TrayIconBuilder::with_id("clash-mini-dev-tray")
                .icon(image)
                .menu(&menu)
                .icon_as_template(false);

            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                builder = builder.show_menu_on_left_click(false);
            }

            match builder.build(&app_handle_clone) {
                Ok(tray) => {
                    let _ = tray.set_tooltip(Some("Clash Mini"));
                    tray.on_tray_icon_event(on_tray_icon_event);
                    tray.on_menu_event(on_menu_event);
                    app_handle_clone.manage(TrayIconState {
                        tray: std::sync::Arc::new(tokio::sync::Mutex::new(Some(tray))),
                    });
                    log::info!(target: "app", "[Tray] System tray created and managed successfully");
                }
                Err(e) => {
                    log::error!(target: "app", "[Tray] Failed to build tray icon on main thread: {}", e);
                }
            }
        })?;

        Ok(())
    }

    // 以下方法在 Clash Mini 的简化托盘中均不执行实际操作，
    // 保留接口以兼容上游调用，避免大规模重构。
    #[allow(clippy::unused_async)]
    pub async fn update_click_behavior(&self) -> Result<()> {
        Ok(())
    }

    #[allow(clippy::unused_async)]
    pub async fn update_menu(&self) -> Result<()> {
        Ok(())
    }

    #[allow(clippy::unused_async)]
    pub async fn update_icon(&self, _verge: &IVerge) -> Result<()> {
        Ok(())
    }

    #[allow(clippy::unused_async)]
    pub async fn update_tooltip(&self) -> Result<()> {
        Ok(())
    }

    #[allow(clippy::unused_async)]
    pub async fn update_part(&self) -> Result<()> {
        Ok(())
    }

    #[allow(clippy::unused_async)]
    pub async fn update_menu_and_icon(&self) {}

    fn should_handle_tray_click(&self) -> bool {
        let allow = self.limiter.check();
        if !allow {
            logging!(debug, Type::Tray, "tray click rate limited");
        }
        allow
    }

    #[allow(dead_code)]
    pub const fn update_speed_task(&self, _enable_tray_speed: bool) {}
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

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    if !Tray::global().should_handle_tray_click() {
        return;
    }
    if event.id.as_ref().is_empty() {
        return;
    }
    let _app_clone = app.clone(); // 修复编译警告：添加下划线前缀
    AsyncHandler::spawn(|| async move {
        match event.id.as_ref() {
            MenuIds::LITE_MODE => {
                // 切换轻量模式，根据当前状态决定进入还是退出
                if lightweight::is_in_lightweight_mode() {
                    // 当前已在轻量模式，尝试退出
                    if lightweight::exit_lightweight_mode().await {
                        logging!(info, Type::Tray, "已退出轻量模式");
                    } else {
                        logging!(error, Type::Tray, "退出轻量模式失败");
                    }
                } else {
                    // 当前不在轻量模式，尝试进入
                    if lightweight::entry_lightweight_mode().await {
                        logging!(info, Type::Tray, "已进入轻量模式");
                    } else {
                        logging!(error, Type::Tray, "进入轻量模式失败");
                    }
                }
                // 同步更新托盘 UI（勾选状态）
                crate::core::tray::update_lite_mode_menu(lightweight::is_in_lightweight_mode());
            }
            MenuIds::EXIT => {
                feat::quit().await;
            }
            _ => {
                logging!(debug, Type::Tray, "Unhandled tray menu event: {:?}", event.id);
            }
        }
    });
}

/// 更新托盘轻量模式菜单的勾选状态并保持可点击
pub fn update_lite_mode_menu(is_in: bool) {
    if let Some(item) = LITE_MODE_MENU_ITEM.get() {
        let _ = item.set_checked(is_in);
        // 始终保持可点击，以便用户可以切换状态
        let _ = item.set_enabled(true);
    }
}

/// 进入轻量模式后禁用「轻量模式」菜单项
/// 供 lib.rs 的事件处理句柄（窗口关闭时）和 tray 自身菜单点击时调用
/// 已废弃的函数，保留兼容性调用（不再使用）
+pub fn _disable_lite_mode_menu_item() {
+    // 过去的实现已被新的 update_lite_mode_menu 替代，保持空实现避免编译错误
+}

