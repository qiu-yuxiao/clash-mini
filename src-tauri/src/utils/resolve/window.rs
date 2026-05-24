#[cfg(not(target_os = "windows"))]
use dark_light::{Mode as SystemTheme, detect as detect_system_theme};
#[cfg(not(target_os = "windows"))]
use tauri::utils::config::Color;
use tauri::webview::PageLoadEvent;
use tauri::{Theme, WebviewWindow};

use crate::{config::Config, core::handle, utils::resolve::window_script::build_window_initial_script};
use clash_verge_logging::{Type, logging_error};

#[cfg(not(target_os = "windows"))]
const DARK_BACKGROUND_COLOR: Color = Color(46, 48, 61, 255); // #2E303D
#[cfg(not(target_os = "windows"))]
const LIGHT_BACKGROUND_COLOR: Color = Color(245, 245, 245, 255); // #F5F5F5
const DARK_BACKGROUND_HEX: &str = "#2E303D";
const LIGHT_BACKGROUND_HEX: &str = "#F5F5F5";

// 定义默认窗口尺寸常量
const DEFAULT_WIDTH: f64 = 940.0;
const DEFAULT_HEIGHT: f64 = 700.0;

const MINIMAL_WIDTH: f64 = 520.0;
const MINIMAL_HEIGHT: f64 = 520.0;

#[cfg(target_os = "linux")]
const DEFAULT_DECORATIONS: bool = false;
#[cfg(not(target_os = "linux"))]
const DEFAULT_DECORATIONS: bool = true;

/// 构建新的 WebView 窗口
pub async fn build_new_window() -> Result<WebviewWindow, String> {
    let app_handle = handle::Handle::app_handle();

    let config = Config::verge().await;
    let latest = config.latest_arc();
    let start_page = latest.start_page.as_deref().unwrap_or("/");
    let initial_theme_mode = match latest.theme_mode.as_deref() {
        Some("dark") => "dark",
        Some("light") => "light",
        _ => "system",
    };

    let resolved_theme = match initial_theme_mode {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    };

    #[cfg(not(target_os = "windows"))]
    let prefers_dark_background = match resolved_theme {
        Some(Theme::Dark) => true,
        Some(Theme::Light) => false,
        _ => !matches!(detect_system_theme().ok(), Some(SystemTheme::Light)),
    };

    #[cfg(not(target_os = "windows"))]
    let background_color = if prefers_dark_background {
        DARK_BACKGROUND_COLOR
    } else {
        LIGHT_BACKGROUND_COLOR
    };

    let initial_script = build_window_initial_script(initial_theme_mode, DARK_BACKGROUND_HEX, LIGHT_BACKGROUND_HEX);

    #[cfg(target_os = "windows")]
    let mut builder = tauri::WebviewWindowBuilder::new(
        app_handle,
        "main", /* the unique window label */
        tauri::WebviewUrl::App(start_page.into()),
    )
    .title("Clash WinAero")
    .center()
    .decorations(DEFAULT_DECORATIONS)
    .fullscreen(false)
    .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    .min_inner_size(MINIMAL_WIDTH, MINIMAL_HEIGHT)
    .visible(false)
    .transparent(true) // 必须设为 true 以支持透明磨砂玻璃
    .initialization_script(&initial_script)
    .general_autofill_enabled(false)
    .on_page_load(move |window, payload| {
        if payload.event() != PageLoadEvent::Finished {
            return;
        }

        logging_error!(Type::Window, window.show());
        logging_error!(Type::Window, window.set_focus());
    });

    #[cfg(not(target_os = "windows"))]
    let mut builder = tauri::WebviewWindowBuilder::new(
        app_handle,
        "main", /* the unique window label */
        tauri::WebviewUrl::App(start_page.into()),
    )
    .title("Clash WinAero")
    .center()
    .decorations(DEFAULT_DECORATIONS)
    .fullscreen(false)
    .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    .min_inner_size(MINIMAL_WIDTH, MINIMAL_HEIGHT)
    .visible(false)
    .initialization_script(&initial_script)
    .general_autofill_enabled(false)
    .on_page_load(move |window, payload| {
        if payload.event() != PageLoadEvent::Finished {
            return;
        }

        logging_error!(Type::Window, window.show());
        logging_error!(Type::Window, window.set_focus());
    });

    if let Some(theme) = resolved_theme {
        builder = builder.theme(Some(theme));
    }

    #[cfg(not(target_os = "windows"))]
    {
        builder = builder.background_color(background_color);
    }

    match builder.build() {
        Ok(window) => {
            #[cfg(not(target_os = "windows"))]
            {
                logging_error!(Type::Window, window.set_background_color(Some(background_color)));
            }

            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::{apply_acrylic, apply_blur, apply_mica};
                // 尝试应用毛玻璃/亚克力/Mica效果
                if apply_mica(&window, None).is_err()
                    && apply_acrylic(&window, Some((0, 0, 0, 0))).is_err()
                {
                    let _ = apply_blur(&window, Some((0, 0, 0, 0)));
                }
            }

            Ok(window)
        }
        Err(e) => Err(e.to_string()),
    }
}
