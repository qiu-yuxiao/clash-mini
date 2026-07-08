use crate::{
    config::{Config, IVerge},
    core::{CoreManager, autostart, handle, hotkey, logger::Logger, sysopt, tray},
    module::auto_backup::AutoBackupManager,
};
use anyhow::{Result, bail};
use bitflags::bitflags;
use clash_verge_draft::SharedDraft;
use clash_verge_logging::{Type, logging, logging_error};
use serde_yaml_ng::Mapping;

/// Patch Clash configuration
pub async fn patch_clash(patch: &Mapping) -> Result<()> {
    Config::clash().await.edit_draft(|d| d.patch_config(patch));

    // 保存旧配置用于回滚
    let old_config = Config::clash().await.latest_arc().0.clone();

    // 将 Draft 提前提交，确保 enhance() 中 get_config_values() 能读取到最新值
    // （旧流程在 update_config_checked() 之后才 apply，导致生成的运行时配置丢失 draft 修改）
    //
    // ⚠️ 注意：apply() 提前意味着在 enhance() 执行期间，任何并发读取 live config
    // 的操作都会看到新值。若 enhance() 失败，会通过回滚逻辑恢复旧配置。由于 patch_clash
    // 的调用路径是同步的 Tauri command（用户操作触发），实际不存在并发读者，风险可控。
    Config::clash().await.apply();

    let res = async {
        // 激活订阅
        if patch.get("secret").is_some() || patch.get("external-controller").is_some() {
            Config::generate().await?;
            CoreManager::global().restart_core().await?;
        } else {
            if patch.get("mode").is_some() {
                tray::Tray::global().update_menu_and_icon().await;
            }
            Config::runtime().await.edit_draft(|d| d.patch_config(patch));
            CoreManager::global().update_config_checked().await?;
        }
        handle::Handle::refresh_clash();
        <Result<()>>::Ok(())
    }
    .await;
    match res {
        Ok(()) => {
            // 分离数据获取和异步调用
            let clash_data = Config::clash().await.data_arc();
            clash_data.save_config().await?;
            Ok(())
        }
        Err(err) => {
            // 恢复旧配置
            Config::clash().await.edit_draft(|d| d.0 = old_config);
            Config::clash().await.apply();
            Err(err)
        }
    }
}

// Define update flags as bitflags for better performance
bitflags! {
     #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
     struct UpdateFlags: u16 {
        const RESTART_CORE = 1 << 0;
        const CLASH_CONFIG = 1 << 1;
        const VERGE_CONFIG = 1 << 2;
        const LAUNCH = 1 << 3;
        const SYS_PROXY = 1 << 4;
        const SYSTRAY_ICON = 1 << 5;
        const HOTKEY = 1 << 6;
        const SYSTRAY_MENU = 1 << 7;
        const SYSTRAY_TOOLTIP = 1 << 8;
        const SYSTRAY_CLICK_BEHAVIOR = 1 << 9;
        const LANGUAGE = 1 << 11;
        const LOG_LEVEL = 1 << 12;
        const LOG_FILE = 1 << 13;
        const ALWAYS_ON_TOP = 1 << 14;

        const GROUP_SYS_TRAY = Self::SYSTRAY_MENU.bits()
                             | Self::SYSTRAY_TOOLTIP.bits()
                             | Self::SYSTRAY_ICON.bits();
     }
}

fn determine_update_flags(patch: &IVerge) -> UpdateFlags {
    let tun_mode = patch.enable_tun_mode;
    let auto_launch = patch.enable_auto_launch;
    let system_proxy = patch.enable_system_proxy;
    let pac = patch.proxy_auto_config;
    let pac_content = &patch.pac_file_content;
    let proxy_bypass = &patch.system_proxy_bypass;
    let language = &patch.language;
    let mixed_port = patch.verge_mixed_port;
    #[cfg(target_os = "macos")]
    let tray_icon = &patch.tray_icon;
    #[cfg(not(target_os = "macos"))]
    let tray_icon: Option<String> = None;
    let common_tray_icon = patch.common_tray_icon;
    let sysproxy_tray_icon = patch.sysproxy_tray_icon;
    let tun_tray_icon = patch.tun_tray_icon;
    #[cfg(not(target_os = "windows"))]
    let redir_enabled = patch.verge_redir_enabled;
    #[cfg(not(target_os = "windows"))]
    let redir_port = patch.verge_redir_port;
    #[cfg(target_os = "linux")]
    let tproxy_enabled = patch.verge_tproxy_enabled;
    #[cfg(target_os = "linux")]
    let tproxy_port = patch.verge_tproxy_port;
    let socks_enabled = patch.verge_socks_enabled;
    let socks_port = patch.verge_socks_port;
    let http_enabled = patch.verge_http_enabled;
    let http_port = patch.verge_port;
    #[cfg(target_os = "macos")]
    let enable_tray_speed = patch.enable_tray_speed;
    #[cfg(not(target_os = "macos"))]
    let enable_tray_speed: Option<bool> = None;
    // let enable_tray_icon = patch.enable_tray_icon;
    let enable_global_hotkey = patch.enable_global_hotkey;
    let tray_event = &patch.tray_event;
    let home_cards = patch.home_cards.as_ref();
    // enable_auto_light_weight_mode 现由 entry_lightweight_mode 直接读取配置判断，
    // 不再需要空壳函数 enable/disable_auto_light_weight_mode，相关 UpdateFlags 已移除。
    let enable_external_controller = patch.enable_external_controller;
    let tray_proxy_groups_display_mode = &patch.tray_proxy_groups_display_mode;
    let tray_inline_outbound_modes = patch.tray_inline_outbound_modes;
    let enable_proxy_guard = patch.enable_proxy_guard;
    let proxy_guard_duration = patch.proxy_guard_duration;
    let log_level = &patch.app_log_level;
    let log_max_size = patch.app_log_max_size;
    let log_max_count = patch.app_log_max_count;
    let enable_always_on_top = patch.enable_always_on_top;

    #[cfg(target_os = "windows")]
    let restart_core_needed = socks_enabled.is_some()
        || http_enabled.is_some()
        || socks_port.is_some()
        || http_port.is_some()
        || mixed_port.is_some()
        || enable_external_controller.is_some();
    #[cfg(not(target_os = "windows"))]
    let mut restart_core_needed = socks_enabled.is_some()
        || http_enabled.is_some()
        || socks_port.is_some()
        || http_port.is_some()
        || mixed_port.is_some()
        || enable_external_controller.is_some();
    #[cfg(not(target_os = "windows"))]
    {
        restart_core_needed |= redir_enabled.is_some() || redir_port.is_some();
    }
    #[cfg(target_os = "linux")]
    {
        restart_core_needed |= tproxy_enabled.is_some() || tproxy_port.is_some();
        restart_core_needed |= tun_mode == Some(true);
    }

    let mut update_flags = UpdateFlags::empty();
    if restart_core_needed {
        update_flags.insert(UpdateFlags::RESTART_CORE);
    }
    if tun_mode.is_some() {
        update_flags.insert(UpdateFlags::CLASH_CONFIG | UpdateFlags::GROUP_SYS_TRAY | UpdateFlags::SYSTRAY_ICON);
    }
    if enable_global_hotkey.is_some()
        || home_cards.is_some()
        || patch.theme_mode.is_some()
        || patch.theme_setting.is_some()
    {
        update_flags.insert(UpdateFlags::VERGE_CONFIG);
    }
    if auto_launch.is_some() {
        update_flags.insert(UpdateFlags::LAUNCH);
    }
    if system_proxy.is_some() || mixed_port.is_some() {
        update_flags.insert(UpdateFlags::SYS_PROXY | UpdateFlags::GROUP_SYS_TRAY | UpdateFlags::SYSTRAY_ICON);
    }
    if proxy_bypass.is_some()
        || pac_content.is_some()
        || pac.is_some()
        || enable_proxy_guard.is_some()
        || proxy_guard_duration.is_some()
    {
        update_flags.insert(UpdateFlags::SYS_PROXY);
    }
    if language.is_some() {
        update_flags.insert(UpdateFlags::LANGUAGE | UpdateFlags::SYSTRAY_MENU | UpdateFlags::SYSTRAY_TOOLTIP);
    }
    if common_tray_icon.is_some()
        || sysproxy_tray_icon.is_some()
        || tun_tray_icon.is_some()
        || tray_icon.is_some()
        || enable_tray_speed.is_some()
    {
        update_flags.insert(UpdateFlags::SYSTRAY_ICON);
    }
    if patch.hotkeys.is_some() {
        update_flags.insert(UpdateFlags::HOTKEY | UpdateFlags::SYSTRAY_MENU);
    }
    if tray_event.is_some() {
        update_flags.insert(UpdateFlags::SYSTRAY_CLICK_BEHAVIOR);
    }
    if tray_proxy_groups_display_mode.is_some() {
        update_flags.insert(UpdateFlags::SYSTRAY_MENU);
    }
    if log_level.is_some() {
        update_flags.insert(UpdateFlags::LOG_LEVEL);
    }
    if log_max_size.is_some() || log_max_count.is_some() {
        update_flags.insert(UpdateFlags::LOG_FILE);
    }
    if enable_always_on_top.is_some() {
        update_flags.insert(UpdateFlags::ALWAYS_ON_TOP);
    }
    if tray_inline_outbound_modes.is_some() {
        update_flags.insert(UpdateFlags::SYSTRAY_MENU);
    }

    update_flags
}

#[allow(clippy::cognitive_complexity)]
async fn process_terminated_flags(update_flags: UpdateFlags, patch: &IVerge) -> Result<()> {
    // Process updates based on flags
    if update_flags.contains(UpdateFlags::RESTART_CORE) {
        Config::generate().await?;
        CoreManager::global().restart_core().await?;
    }
    if update_flags.contains(UpdateFlags::CLASH_CONFIG) {
        CoreManager::global().update_config_checked().await?;
        handle::Handle::refresh_clash();
    }
    if update_flags.contains(UpdateFlags::VERGE_CONFIG) {
        Config::verge()
            .await
            .edit_draft(|d| d.enable_global_hotkey = patch.enable_global_hotkey);
        handle::Handle::refresh_verge();
    }
    if update_flags.contains(UpdateFlags::LAUNCH) {
        // WARNING: ALWAYS pass patch.enable_auto_launch directly to update_launch.
        // DO NOT rely on update_launch querying latest_arc() during configuration patching because the draft config has not been applied yet.
        autostart::update_launch(patch.enable_auto_launch).await?;
    }
    if update_flags.contains(UpdateFlags::LANGUAGE)
        && let Some(language) = &patch.language
    {
        clash_verge_i18n::set_locale(language.as_str());
    }
    if update_flags.contains(UpdateFlags::SYS_PROXY) {
        sysopt::Sysopt::global().update_sysproxy().await?;
        sysopt::Sysopt::global().refresh_guard().await;
    }
    if update_flags.contains(UpdateFlags::HOTKEY)
        && let Some(hotkeys) = &patch.hotkeys
    {
        hotkey::Hotkey::global().update(hotkeys.to_owned()).await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_MENU) {
        tray::Tray::global().update_menu().await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_ICON) {
        tray::Tray::global()
            .update_icon(&Config::verge().await.latest_arc())?;
        #[cfg(target_os = "macos")]
        if patch.enable_tray_speed.is_some() {
            tray::Tray::global().update_speed_task(patch.enable_tray_speed.unwrap_or(false));
        }
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_TOOLTIP) {
        tray::Tray::global().update_tooltip().await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_CLICK_BEHAVIOR) {
        tray::Tray::global().update_click_behavior().await?;
    }
    if update_flags.contains(UpdateFlags::LOG_LEVEL) {
        Logger::global().update_log_level(patch.get_log_level())?;
    }
    if update_flags.contains(UpdateFlags::LOG_FILE) {
        let log_max_size = patch.app_log_max_size.unwrap_or(128);
        let log_max_count = patch.app_log_max_count.unwrap_or(8);
        Logger::global().update_log_config(log_max_size, log_max_count).await?;
    }
    if update_flags.contains(UpdateFlags::ALWAYS_ON_TOP)
        && let Some(always_on_top) = patch.enable_always_on_top
        && let Some(window) = crate::utils::window_manager::WindowManager::get_main_window()
    {
        let app_handle = crate::core::handle::Handle::app_handle();
        let label = window.label().to_string();
        let app_handle_clone = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            use tauri::Manager as _;
            if let Some(w) = app_handle_clone.get_webview_window(&label) {
                let _ = w.set_always_on_top(always_on_top);
            }
        });
    }
    Ok(())
}

/// 验证 CSS injection 字符串的安全性
fn validate_css_injection(css: &str) -> Result<()> {
    if css.len() > 100_000 {
        bail!("CSS injection exceeds maximum length of 100KB");
    }
    // 禁止 @import（可能加载外部资源）
    if css.contains("@import") {
        bail!("CSS injection cannot contain @import rules");
    }
    // 禁止 javascript: URL（XSS 向量）
    if css.to_ascii_lowercase().contains("javascript:") {
        bail!("CSS injection cannot contain javascript: URLs");
    }
    // 禁止 IE expression()（旧式 XSS 向量）
    if css.to_ascii_lowercase().contains("expression(") {
        bail!("CSS injection cannot contain expression()");
    }
    Ok(())
}

pub async fn patch_verge(patch: &IVerge, not_save_file: bool) -> Result<()> {
    // 验证 css_injection 安全性
    if let Some(theme) = &patch.theme_setting {
        if let Some(css) = &theme.css_injection {
            validate_css_injection(css)?;
        }
    }

    Config::verge().await.edit_draft(|d| d.patch_config(patch));

    let update_flags = determine_update_flags(patch);
    logging!(debug, Type::Setup, "Determined update flags: {:?}", update_flags);
    let process_flag_result = process_terminated_flags(update_flags, patch).await;

    if let Err(err) = process_flag_result {
        Config::verge().await.discard();
        return Err(err);
    }
    Config::verge().await.apply();
    logging_error!(Type::Backup, AutoBackupManager::global().refresh_settings().await);
    if !not_save_file {
        // 分离数据获取和异步调用
        let verge_data = Config::verge().await.data_arc();
        logging!(debug, Type::Setup, "Saving Verge configuration to file...");
        verge_data.save_file().await?;
    }
    Ok(())
}

pub async fn fetch_verge_config() -> Result<SharedDraft<IVerge>> {
    let draft = Config::verge().await;
    let data = draft.data_arc();
    Ok(data)
}
