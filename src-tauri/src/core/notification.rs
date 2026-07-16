use crate::utils::window_manager::WindowManager;
use clash_verge_logging::{Type, logging};
use serde_json::json;
use smartstring::alias::String;

use tauri::Emitter as _;

#[derive(Debug)]
pub enum FrontendEvent<'a> {
    RefreshClash,
    RefreshVerge,
    RefreshProxies,
    NoticeMessage { status: &'a str, message: String },
    ProfileChanged { current_profile_id: &'a String },
    TimerUpdated { profile_index: &'a String },
    ProfileUpdateStarted { uid: &'a String },
    ProfileUpdateCompleted { uid: &'a String },
    DelayResults { group: String, results: Vec<(String, u32)> },
}

#[derive(Debug)]
pub struct NotificationSystem {}

impl NotificationSystem {
    fn serialize_event(event: FrontendEvent) -> (&'static str, Result<serde_json::Value, serde_json::Error>) {
        match event {
            FrontendEvent::RefreshClash => ("verge://refresh-clash-config", Ok(json!("yes"))),
            FrontendEvent::RefreshVerge => ("verge://refresh-verge-config", Ok(json!("yes"))),
            FrontendEvent::RefreshProxies => ("verge://refresh-proxy-config", Ok(json!("yes"))),
            FrontendEvent::NoticeMessage { status, message } => {
                ("verge://notice-message", serde_json::to_value((status, message)))
            }
            FrontendEvent::ProfileChanged { current_profile_id } => ("profile-changed", Ok(json!(current_profile_id))),
            FrontendEvent::TimerUpdated { profile_index } => ("verge://timer-updated", Ok(json!(profile_index))),
            FrontendEvent::ProfileUpdateStarted { uid } => ("profile-update-started", Ok(json!({ "uid": uid }))),
            FrontendEvent::ProfileUpdateCompleted { uid } => ("profile-update-completed", Ok(json!({ "uid": uid }))),
            FrontendEvent::DelayResults { group, results } => {
                let data = json!({
                    "group": group,
                    "results": results
                });
                ("verge://backend-delay-results", Ok(data))
            }
        }
    }

    pub(crate) fn send_event(event: FrontendEvent) {
        // 🛡️ 防线一：轻量模式下直接过滤，绝不对已销毁的 Webview 窗口调用 window.emit
        if crate::module::lightweight::is_in_lightweight_mode() {
            return;
        }

        let Some(window) = WindowManager::get_main_window() else {
            return;
        };

        let (event_name, Ok(payload)) = Self::serialize_event(event) else {
            return;
        };

        tauri::async_runtime::spawn(async move {
            if let Err(e) = window.emit(event_name, payload) {
                logging!(warn, Type::Frontend, "Event emit failed: {}", e);
            }
        });
    }
}
