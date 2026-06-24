use std::collections::HashMap;

use tauri::{
    AppHandle, Emitter, Runtime, State,
    async_runtime::RwLock,
    command,
    ipc::{Channel, InvokeResponseBody},
};

use crate::{Result, mihomo::Mihomo, models::*};

#[command]
pub(crate) async fn update_controller(
    state: State<'_, RwLock<Mihomo>>,
    host: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    let mut mihomo = state.write().await;
    mihomo.update_external_host(host);
    mihomo.update_external_port(port);
    drop(mihomo);
    Ok(())
}

#[command]
pub(crate) async fn update_secret(state: State<'_, RwLock<Mihomo>>, secret: Option<String>) -> Result<()> {
    state.write().await.update_secret(secret);
    Ok(())
}

#[command]
pub(crate) async fn get_version(state: State<'_, RwLock<Mihomo>>) -> Result<MihomoVersion> {
    let mihomo = state.read().await.clone();
    mihomo.get_version().await
}

#[command]
pub(crate) async fn flush_fakeip(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.flush_fakeip().await
}

#[command]
pub(crate) async fn flush_dns(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.flush_dns().await
}

// connections
#[command]
pub(crate) async fn get_connections(state: State<'_, RwLock<Mihomo>>) -> Result<Connections> {
    let mihomo = state.read().await.clone();
    mihomo.get_connections().await
}

#[command]
pub(crate) async fn close_all_connections(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.close_all_connections().await
}

#[command]
pub(crate) async fn close_connection(state: State<'_, RwLock<Mihomo>>, connection_id: String) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.close_connection(&connection_id).await
}

// groups
#[command]
pub(crate) async fn get_groups(state: State<'_, RwLock<Mihomo>>) -> Result<Groups> {
    let mihomo = state.read().await.clone();
    mihomo.get_groups().await
}

#[command]
pub(crate) async fn get_group_by_name(state: State<'_, RwLock<Mihomo>>, group_name: String) -> Result<Proxy> {
    let mihomo = state.read().await.clone();
    mihomo.get_group_by_name(&group_name).await
}

// [Clash Mini 备注]: 此命令为 Clash Verge 遗留接口。
// 在 Clash Mini 中，前台批量测速已统一收拢到后端的 trigger_auto_select 触发并返回结果，
// 此处在 Clash Mini 内属于闲置死代码，保留仅为了维持插件 API 接口的完整性。
#[command]
pub(crate) async fn delay_group(
    state: State<'_, RwLock<Mihomo>>,
    group_name: String,
    test_url: String,
    timeout: u32,
    keep_fixed: bool,
) -> Result<HashMap<String, u32>> {
    let mihomo = state.read().await.clone();
    let fixed = if keep_fixed {
        mihomo.get_group_by_name(&group_name).await?.fixed
    } else {
        None
    };
    log::debug!("delay group, fixed: {fixed:?}");
    let res = mihomo.delay_group(&group_name, &test_url, timeout).await?;
    if keep_fixed
        && let Some(fixed) = fixed
        && !fixed.is_empty()
    {
        mihomo.select_node_for_group(&group_name, &fixed).await?;
    }
    Ok(res)
}

// providers
#[command]
pub(crate) async fn get_proxy_providers(state: State<'_, RwLock<Mihomo>>) -> Result<ProxyProviders> {
    let mihomo = state.read().await.clone();
    mihomo.get_proxy_providers().await
}

#[command]
pub(crate) async fn get_proxy_provider_by_name(
    state: State<'_, RwLock<Mihomo>>,
    provider_name: String,
) -> Result<ProxyProvider> {
    let mihomo = state.read().await.clone();
    mihomo.get_proxy_provider_by_name(&provider_name).await
}

#[command]
pub(crate) async fn update_proxy_provider<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    provider_name: String,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.update_proxy_provider(&provider_name).await?;
    let _ = app.emit("verge://refresh-proxy-config", "yes");
    Ok(())
}

#[command]
pub(crate) async fn healthcheck_proxy_provider<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    provider_name: String,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.healthcheck_proxy_provider(&provider_name).await?;
    let _ = app.emit("verge://refresh-proxy-config", "yes");
    Ok(())
}

#[command]
pub(crate) async fn healthcheck_node_in_provider<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    provider_name: String,
    proxy_name: String,
    test_url: String,
    timeout: u32,
) -> Result<ProxyDelay> {
    let mihomo = state.read().await.clone();
    let res = mihomo
        .healthcheck_node_in_provider(&provider_name, &proxy_name, &test_url, timeout)
        .await;
    if res.is_ok() {
        let _ = app.emit("verge://refresh-proxy-config", "yes");
    }
    res
}

// proxies
#[command]
pub(crate) async fn get_proxies(state: State<'_, RwLock<Mihomo>>) -> Result<Proxies> {
    let mihomo = state.read().await.clone();
    mihomo.get_proxies().await
}

#[command]
pub(crate) async fn get_proxy_by_name(state: State<'_, RwLock<Mihomo>>, proxy_name: String) -> Result<Proxy> {
    let mihomo = state.read().await.clone();
    mihomo.get_proxy_by_name(&proxy_name).await
}

#[command]
pub(crate) async fn select_node_for_group<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    group_name: String,
    node: String,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.select_node_for_group(&group_name, &node).await?;
    let _ = app.emit("verge://refresh-proxy-config", "yes");
    Ok(())
}

#[command]
pub(crate) async fn unfixed_proxy<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    group_name: String,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.unfixed_proxy(&group_name).await?;
    let _ = app.emit("verge://refresh-proxy-config", "yes");
    Ok(())
}

#[command]
pub(crate) async fn delay_proxy_by_name<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    proxy_name: String,
    test_url: String,
    timeout: u32,
) -> Result<ProxyDelay> {
    let mihomo = state.read().await.clone();
    let res = mihomo.delay_proxy_by_name(&proxy_name, &test_url, timeout).await;
    if res.is_ok() {
        let _ = app.emit("verge://refresh-proxy-config", "yes");
    }
    res
}

// rules
#[command]
pub(crate) async fn get_rules(state: State<'_, RwLock<Mihomo>>) -> Result<Rules> {
    let mihomo = state.read().await.clone();
    mihomo.get_rules().await
}

#[command]
pub(crate) async fn get_rule_providers(state: State<'_, RwLock<Mihomo>>) -> Result<RuleProviders> {
    let mihomo = state.read().await.clone();
    mihomo.get_rule_providers().await
}

#[command]
pub(crate) async fn update_rule_provider<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    provider_name: String,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    let res = mihomo.update_rule_provider(&provider_name).await;
    if res.is_ok() {
        let _ = app.emit("verge://refresh-clash-config", "yes");
    }
    res
}

// runtime config
#[command]
pub(crate) async fn get_base_config(state: State<'_, RwLock<Mihomo>>) -> Result<BaseConfig> {
    let mihomo = state.read().await.clone();
    mihomo.get_base_config().await
}

#[command]
pub(crate) async fn reload_config<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    force: bool,
    config_path: String,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    let res = mihomo.reload_config(force, &config_path).await;
    if res.is_ok() {
        let _ = app.emit("verge://refresh-clash-config", "yes");
    }
    res
}

#[command]
pub(crate) async fn patch_base_config(state: State<'_, RwLock<Mihomo>>, data: serde_json::Value) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.patch_base_config(&data).await
}

#[command]
pub(crate) async fn update_geo(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.update_geo().await
}

#[command]
pub(crate) async fn restart(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.restart().await
}

// upgrade
#[command]
pub(crate) async fn upgrade_core(
    state: State<'_, RwLock<Mihomo>>,
    channel: CoreUpdaterChannel,
    force: bool,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.upgrade_core(channel, force).await
}

#[command]
pub(crate) async fn upgrade_ui(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.upgrade_ui().await
}

#[command]
pub(crate) async fn upgrade_geo(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.upgrade_geo().await
}

// mihomo websocket
#[command]
pub(crate) async fn ws_traffic(
    state: State<'_, RwLock<Mihomo>>,
    on_message: Channel<InvokeResponseBody>,
) -> Result<ConnectionId> {
    let mihomo = state.read().await.clone();
    mihomo
        .ws_traffic_checked(move |data| on_message.send(data).is_ok())
        .await
}

#[command]
pub(crate) async fn ws_memory(
    state: State<'_, RwLock<Mihomo>>,
    on_message: Channel<InvokeResponseBody>,
) -> Result<ConnectionId> {
    let mihomo = state.read().await.clone();
    mihomo
        .ws_memory_checked(move |data| on_message.send(data).is_ok())
        .await
}

#[command]
pub(crate) async fn ws_connections(
    state: State<'_, RwLock<Mihomo>>,
    on_message: Channel<InvokeResponseBody>,
) -> Result<ConnectionId> {
    let mihomo = state.read().await.clone();
    mihomo
        .ws_connections_checked(move |data| on_message.send(data).is_ok())
        .await
}

#[command]
pub(crate) async fn ws_logs(
    state: State<'_, RwLock<Mihomo>>,
    level: LogLevel,
    on_message: Channel<InvokeResponseBody>,
) -> Result<ConnectionId> {
    let mihomo = state.read().await.clone();
    mihomo
        .ws_logs_checked(level, move |data| on_message.send(data).is_ok())
        .await
}

#[command]
pub(crate) async fn ws_disconnect(
    state: State<'_, RwLock<Mihomo>>,
    id: ConnectionId,
    force_timeout: Option<u64>,
) -> Result<()> {
    let mihomo = state.read().await.clone();
    mihomo.disconnect(id, force_timeout).await
}

#[command]
pub(crate) async fn clear_all_ws_connections(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.write().await.clear_all_ws_connections().await
}
