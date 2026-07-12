use crate::{
    config::{Config, IVerge},
    core::{CoreManager, autostart, handle, hotkey, logger::Logger, sysopt, tray},
};
use anyhow::{Result, bail};
use bitflags::bitflags;
use clash_verge_draft::SharedDraft;
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::Mapping;

/// 互斥保护 patch_verge 的并发调用，防止 draft edit/apply/save_file 写入竞态
static VERGE_PATCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Patch Clash configuration
pub async fn patch_clash(patch: &Mapping) -> Result<()> {
    // 在修改前保存旧配置（必须在 edit_draft 之前获取，否则 latest_arc 会返回 draft 新值）
    let old_config = Config::clash().await.latest_arc().0.clone();

    Config::clash().await.edit_draft(|d| d.patch_config(patch));

    // 注意：此处不提前 apply clash 配置，确保在 enhance() 执行期间，
    // 已提交的配置（data_arc）仍然是旧值，仅 draft 是新值。
    // enhance() 通过 latest_arc() 读取（会包含 draft），因此能正确处理新配置。
    // 若 enhance 失败，我们直接 discard draft 即可，不会影响已提交的配置。

    // 检测 allow-lan 和 ipv6 的实质变动，决定是否需要重启内核。
    // 这两个底层网络和协议栈属性在热重载时容易导致端口冲突、TUN网卡死锁，因此强制通过重启解决，保证稳定性。
    let old_allow_lan = old_config.get("allow-lan").and_then(|v| v.as_bool());
    let old_ipv6 = old_config.get("ipv6").and_then(|v| v.as_bool());

    let new_allow_lan = patch.get("allow-lan").and_then(|v| v.as_bool());
    let new_ipv6 = patch.get("ipv6").and_then(|v| v.as_bool());

    let allow_lan_changed = new_allow_lan.is_some() && new_allow_lan != old_allow_lan;
    let ipv6_changed = new_ipv6.is_some() && new_ipv6 != old_ipv6;

    let res = async {
        let need_restart = patch.get("secret").is_some()
            || patch.get("external-controller").is_some()
            || allow_lan_changed
            || ipv6_changed;

        if need_restart {
            Config::generate().await?;
            CoreManager::global().restart_core().await?;
        } else {
            CoreManager::global().update_config_checked().await?;
        }
        handle::Handle::refresh_clash();
        <Result<()>>::Ok(())
    }
    .await;
    match res {
        Ok(()) => {
            // 成功：提交 clash 配置的 draft，并保存到文件
            Config::clash().await.apply();
            let clash_data = Config::clash().await.data_arc();
            clash_data.save_config().await?;
            Ok(())
        }
        Err(err) => {
            // 失败：丢弃 clash 配置的 draft，恢复到修改前的状态
            // 由于我们没有提前 apply，已提交的配置仍然是旧值，只需 discard draft 即可
            Config::clash().await.discard();
            // 保险起见：如果修改前就有 draft，恢复到旧的 draft 状态
            // （这里简化处理：直接 discard，因为大多数情况下修改前 draft 是 None）
            let _ = old_config; // 避免未使用警告，如需恢复旧 draft 可在此处添加逻辑
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
    let rule_fallback = &patch.rule_fallback;
    let enable_dns_settings = patch.enable_dns_settings;
    let enable_builtin_enhanced = patch.enable_builtin_enhanced;

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
    if tun_mode.is_some()
        || rule_fallback.is_some()
        || enable_dns_settings.is_some()
        || enable_builtin_enhanced.is_some()
    {
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
    if tray_icon.is_some() || enable_tray_speed.is_some() {
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
    // SYS_PROXY 必须在 CLASH_CONFIG（TUN 重载）之前执行，原因：
    // 1. 退场顺序：切换到 TUN 模式时，必须先清除 OS 系统代理，再启动 TUN 网卡，
    //    否则存在双重流量接管的冲突时间窗口。
    // 2. 异常隔离：若 CLASH_CONFIG（内核重载/TUN 适配器提权）报错并经 `?` 向上传播，
    //    将导致此处之后的所有步骤被跳过。将 SYS_PROXY 提前可确保代理清理在任何
    //    内核错误之前完成，不会因其他步骤失败而被遗漏残留在 OS 注册表中。
    // 注：对于端口变更场景（RESTART_CORE + SYS_PROXY），RESTART_CORE 仍在本块之前，
    //    顺序依然正确（先重启内核监听新端口，再更新 OS 代理指向新端口）。
    if update_flags.contains(UpdateFlags::SYS_PROXY) {
        sysopt::Sysopt::global().update_sysproxy().await?;
        sysopt::Sysopt::global().refresh_guard().await;
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
    // SYS_PROXY 已提前至 CLASH_CONFIG 之前执行（见上方注释），此处不再重复。
    if update_flags.contains(UpdateFlags::HOTKEY)
        && let Some(hotkeys) = &patch.hotkeys
    {
        hotkey::Hotkey::global().update(hotkeys.to_owned()).await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_MENU) {
        tray::Tray::global().update_menu().await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_ICON) {
        tray::Tray::global().update_icon(&Config::verge().await.latest_arc())?;
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
    let _guard = VERGE_PATCH_LOCK.lock().await;

    // 验证 css_injection 安全性
    if let Some(theme) = &patch.theme_setting {
        if let Some(css) = &theme.css_injection {
            validate_css_injection(css)?;
        }
    }

    let old_config = (*Config::verge().await.latest_arc()).clone();

    Config::verge().await.edit_draft(|d| d.patch_config(patch));

    let update_flags = determine_update_flags(patch);
    logging!(debug, Type::Setup, "Determined update flags: {:?}", update_flags);
    let process_flag_result = process_terminated_flags(update_flags, patch).await;

    if let Err(err) = process_flag_result {
        Config::verge().await.discard();
        return Err(err);
    }

    Config::verge().await.apply();

    if !not_save_file {
        // 分离数据获取和异步调用
        let verge_data = Config::verge().await.data_arc();
        logging!(debug, Type::Setup, "Saving Verge configuration to file...");
        if let Err(e) = verge_data.save_file().await {
            // save_file 失败时回滚内存配置，保持内存与磁盘一致
            Config::verge().await.edit_draft(|d| *d = old_config);
            Config::verge().await.apply();
            return Err(e);
        }
    }
    Ok(())
}

pub async fn fetch_verge_config() -> Result<SharedDraft<IVerge>> {
    let draft = Config::verge().await;
    let data = draft.data_arc();
    Ok(data)
}
