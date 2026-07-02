use tauri::webview::PageLoadEvent;
use tauri::{Theme, WebviewWindow};

use crate::{config::Config, core::handle, utils::resolve::window_script::build_window_initial_script};
use clash_verge_logging::{Type, logging_error};

#[cfg(not(target_os = "windows"))]
use dark_light::{Mode as SystemTheme, detect as detect_system_theme};
#[cfg(not(target_os = "windows"))]
use tauri::utils::config::Color;

#[cfg(not(target_os = "windows"))]
const DARK_BACKGROUND_COLOR: Color = Color(46, 48, 61, 255);
#[cfg(not(target_os = "windows"))]
const LIGHT_BACKGROUND_COLOR: Color = Color(245, 245, 245, 255);
const DARK_BACKGROUND_HEX: &str = "#2E303D";
const LIGHT_BACKGROUND_HEX: &str = "#F5F5F5";

const DEFAULT_WIDTH: f64 = 270.0;
const DEFAULT_HEIGHT: f64 = 680.0;
const MAX_WIDTH: f64 = 640.0;
const MAX_HEIGHT: f64 = 860.0;
const MINIMAL_WIDTH: f64 = 270.0;
const MINIMAL_HEIGHT: f64 = 99.0;
const DEFAULT_DECORATIONS: bool = false;

pub async fn build_new_window() -> Result<WebviewWindow, String> {
    let app_handle = handle::Handle::app_handle();
    let app_handle_clone = app_handle.clone();
    let config = Config::verge().await;
    let latest = config.latest_arc();
    let always_on_top = latest.enable_always_on_top.unwrap_or(false);
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

    let mut builder = tauri::WebviewWindowBuilder::new(app_handle, "main", tauri::WebviewUrl::App(start_page.into()))
        .center()
        .decorations(DEFAULT_DECORATIONS)
        .fullscreen(false)
        .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
        .max_inner_size(MAX_WIDTH, MAX_HEIGHT)
        .min_inner_size(MINIMAL_WIDTH, MINIMAL_HEIGHT)
        .visible(false)
        .initialization_script(&initial_script)
        .general_autofill_enabled(false)
        .on_page_load(move |window, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }

            logging_error!(Type::Window, window.set_title(&get_bold_window_title()));
            logging_error!(Type::Window, window.show());
            logging_error!(Type::Window, window.set_focus());
            let label = window.label().to_string();
            let ah = app_handle_clone.clone();
            let ah2 = ah.clone();
            let _ = ah.run_on_main_thread(move || {
                use tauri::Manager as _;
                if let Some(w) = ah2.get_webview_window(&label) {
                    let _ = w.set_always_on_top(always_on_top);
                }
            });
        });

    #[cfg(target_os = "windows")]
    {
        builder = builder.transparent(false).additional_browser_args(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disk-cache-size=31457280",
        );
    }

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

            Ok(window)
        }
        Err(e) => Err(e.to_string()),
    }
}

fn get_bold_window_title() -> String {
    format!("Clash Mini Ver.{}", env!("CARGO_PKG_VERSION"))
}
