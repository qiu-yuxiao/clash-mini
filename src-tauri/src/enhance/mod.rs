mod chain;
pub mod field;
mod merge;
mod script;
pub mod seq;
mod tun;

use self::{
    chain::{AsyncChainItemFrom as _, ChainItem, ChainType},
    field::{use_keys, use_lowercase, use_sort},
    merge::use_merge,
    script::use_script,
    seq::{SeqMap, use_seq},
    tun::use_tun,
};
use crate::utils::dirs;
use crate::{config::Config, utils::tmpl};
use crate::{config::IVerge, constants};
use anyhow::{Context as _, Result};
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};
use tokio::fs;

type ResultLog = Vec<(String, String)>;
#[derive(Debug)]
struct ConfigValues {
    clash_config: Mapping,
    clash_core: Option<String>,
    enable_tun: bool,
    enable_builtin: bool,
    socks_enabled: bool,
    http_enabled: bool,
    enable_dns_settings: bool,
    #[cfg(not(target_os = "windows"))]
    redir_enabled: bool,
    #[cfg(target_os = "linux")]
    tproxy_enabled: bool,
}

#[derive(Debug)]
struct ProfileItems {
    config: Mapping,
    merge_item: ChainItem,
    script_item: ChainItem,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    groups_item: ChainItem,
    global_merge: ChainItem,
    global_script: ChainItem,
    profile_name: String,
}

impl Default for ProfileItems {
    fn default() -> Self {
        Self {
            config: Default::default(),
            profile_name: Default::default(),
            merge_item: ChainItem {
                uid: "".into(),
                data: ChainType::Merge(Mapping::new()),
            },
            script_item: ChainItem {
                uid: "".into(),
                data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
            },
            rules_item: ChainItem {
                uid: "".into(),
                data: ChainType::Rules(SeqMap::default()),
            },
            proxies_item: ChainItem {
                uid: "".into(),
                data: ChainType::Proxies(SeqMap::default()),
            },
            groups_item: ChainItem {
                uid: "".into(),
                data: ChainType::Groups(SeqMap::default()),
            },
            global_merge: ChainItem {
                uid: "Merge".into(),
                data: ChainType::Merge(Mapping::new()),
            },
            global_script: ChainItem {
                uid: "Script".into(),
                data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
            },
        }
    }
}

async fn get_config_values() -> ConfigValues {
    let clash = Config::clash().await;
    let clash_arc = clash.latest_arc();
    let clash_config = clash_arc.0.clone();
    drop(clash_arc);
    drop(clash);

    let verge = Config::verge().await;

    let verge_arc = verge.latest_arc();
    let IVerge {
        ref enable_tun_mode,
        ref enable_builtin_enhanced,
        ref verge_socks_enabled,
        ref verge_http_enabled,
        ref enable_dns_settings,
        ..
    } = *verge_arc;

    let (clash_core, enable_tun, enable_builtin, socks_enabled, http_enabled, enable_dns_settings) = (
        Some(verge_arc.get_valid_clash_core()),
        enable_tun_mode.unwrap_or(false),
        enable_builtin_enhanced.unwrap_or(true),
        verge_socks_enabled.unwrap_or(false),
        verge_http_enabled.unwrap_or(false),
        enable_dns_settings.unwrap_or(false),
    );

    #[cfg(not(target_os = "windows"))]
    let redir_enabled = verge_arc.verge_redir_enabled.unwrap_or(false);

    #[cfg(target_os = "linux")]
    let tproxy_enabled = verge_arc.verge_tproxy_enabled.unwrap_or(false);

    drop(verge_arc);
    drop(verge);

    ConfigValues {
        clash_config,
        clash_core,
        enable_tun,
        enable_builtin,
        socks_enabled,
        http_enabled,
        enable_dns_settings,
        #[cfg(not(target_os = "windows"))]
        redir_enabled,
        #[cfg(target_os = "linux")]
        tproxy_enabled,
    }
}

#[allow(clippy::cognitive_complexity)]
async fn collect_profile_items() -> Result<ProfileItems> {
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();
    drop(profiles);

    let current_profile_uid = match profiles_arc.get_current().cloned() {
        Some(uid) => uid,
        None => {
            drop(profiles_arc);
            return Ok(ProfileItems::default());
        }
    };

    let current = profiles_arc
        .current_mapping()
        .await
        .with_context(|| format!("failed to read current profile \"{current_profile_uid}\""))?;

    let current_item = match profiles_arc.get_item(&current_profile_uid) {
        Ok(item) => item,
        Err(err) => {
            return Err(err).with_context(|| format!("failed to get current profile \"{current_profile_uid}\""));
        }
    };

    let merge_uid = current_item.current_merge().cloned().unwrap_or_else(|| "Merge".into());
    let script_uid = current_item
        .current_script()
        .cloned()
        .unwrap_or_else(|| "Script".into());
    let rules_uid = current_item.current_rules().cloned().unwrap_or_else(|| "Rules".into());
    let proxies_uid = current_item
        .current_proxies()
        .cloned()
        .unwrap_or_else(|| "Proxies".into());
    let groups_uid = current_item
        .current_groups()
        .cloned()
        .unwrap_or_else(|| "Groups".into());

    let name = current_item.name.clone().unwrap_or_default();

    let merge_item = {
        let item = profiles_arc.get_item(&merge_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Merge(Mapping::new()),
    });

    let script_item = {
        let item = profiles_arc.get_item(&script_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
    });

    let rules_item = {
        let item = profiles_arc.get_item(&rules_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Rules(SeqMap::default()),
    });

    let proxies_item = {
        let item = profiles_arc.get_item(&proxies_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Proxies(SeqMap::default()),
    });

    let groups_item = {
        let item = profiles_arc.get_item(&groups_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Groups(SeqMap::default()),
    });

    let global_merge = {
        let item = profiles_arc.get_item("Merge").ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "Merge".into(),
        data: ChainType::Merge(Mapping::new()),
    });

    let global_script = {
        let item = profiles_arc.get_item("Script").ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "Script".into(),
        data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
    });

    drop(profiles_arc);

    Ok(ProfileItems {
        config: current,
        merge_item,
        script_item,
        rules_item,
        proxies_item,
        groups_item,
        global_merge,
        global_script,
        profile_name: name,
    })
}

async fn process_global_items(
    mut config: Mapping,
    global_merge: ChainItem,
    global_script: ChainItem,
    profile_name: &String,
) -> (Mapping, Vec<String>, HashMap<String, ResultLog>) {
    let mut result_map = HashMap::new();
    let mut exists_keys = use_keys(&config).collect::<Vec<_>>();

    if let ChainType::Merge(merge) = global_merge.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    if let ChainType::Script(script) = global_script.data {
        let mut logs = vec![];
        match use_script(script, config.clone(), profile_name.clone()).await {
            Ok((res_config, res_logs)) => {
                exists_keys.extend(use_keys(&res_config));
                config = res_config;
                logs.extend(res_logs);
            }
            Err(err) => logs.push(("exception".into(), err.to_string().into())),
        }
        result_map.insert(global_script.uid, logs);
    }

    (config, exists_keys, result_map)
}

#[allow(clippy::too_many_arguments)]
async fn process_profile_items(
    mut config: Mapping,
    mut exists_keys: Vec<String>,
    mut result_map: HashMap<String, ResultLog>,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    groups_item: ChainItem,
    merge_item: ChainItem,
    script_item: ChainItem,
    profile_name: &String,
) -> (Mapping, Vec<String>, HashMap<String, ResultLog>) {
    if let ChainType::Rules(rules) = rules_item.data {
        config = use_seq(rules, config.to_owned(), "rules");
    }

    if let ChainType::Proxies(proxies) = proxies_item.data {
        config = use_seq(proxies, config.to_owned(), "proxies");
    }

    if let ChainType::Groups(groups) = groups_item.data {
        config = use_seq(groups, config.to_owned(), "proxy-groups");
    }

    if let ChainType::Merge(merge) = merge_item.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    if let ChainType::Script(script) = script_item.data {
        let mut logs = vec![];
        match use_script(script, config.clone(), profile_name.clone()).await {
            Ok((res_config, res_logs)) => {
                exists_keys.extend(use_keys(&res_config));
                config = res_config;
                logs.extend(res_logs);
            }
            Err(err) => logs.push(("exception".into(), err.to_string().into())),
        }
        result_map.insert(script_item.uid, logs);
    }

    (config, exists_keys, result_map)
}

async fn merge_default_config(
    mut config: Mapping,
    clash_config: Mapping,
    socks_enabled: bool,
    http_enabled: bool,
    #[cfg(not(target_os = "windows"))] redir_enabled: bool,
    #[cfg(target_os = "linux")] tproxy_enabled: bool,
) -> Mapping {
    for (key, value) in clash_config.into_iter() {
        if key.as_str() == Some("tun") {
            let mut tun = config.get_mut("tun").map_or_else(Mapping::new, |val| {
                val.as_mapping().cloned().unwrap_or_else(Mapping::new)
            });
            let patch_tun = value.as_mapping().cloned().unwrap_or_else(Mapping::new);
            for (key, value) in patch_tun.into_iter() {
                tun.insert(key, value);
            }
            config.insert("tun".into(), tun.into());
        } else {
            if key.as_str() == Some("socks-port") && !socks_enabled {
                config.remove("socks-port");
                continue;
            }
            if key.as_str() == Some("port") && !http_enabled {
                config.remove("port");
                continue;
            }
            #[cfg(target_os = "windows")]
            {
                if key.as_str() == Some("redir-port") {
                    continue;
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if key.as_str() == Some("redir-port") && !redir_enabled {
                    config.remove("redir-port");
                    continue;
                }
            }
            #[cfg(target_os = "linux")]
            {
                if key.as_str() == Some("tproxy-port") && !tproxy_enabled {
                    config.remove("tproxy-port");
                    continue;
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                if key.as_str() == Some("tproxy-port") {
                    config.remove("tproxy-port");
                    continue;
                }
            }
            // 处理 external-controller 键的开关逻辑
            if key.as_str() == Some("external-controller") {
                let enable_external_controller = Config::verge()
                    .await
                    .latest_arc()
                    .enable_external_controller
                    .unwrap_or(false);

                if enable_external_controller {
                    config.insert(key, value);
                } else {
                    // 如果禁用了外部控制器，设置为空字符串
                    config.insert(key, "".into());
                }
            } else {
                config.insert(key, value);
            }
        }
    }

    config
}

async fn apply_builtin_scripts(mut config: Mapping, clash_core: Option<String>, enable_builtin: bool) -> Mapping {
    if enable_builtin {
        let items: Vec<_> = ChainItem::builtin()
            .into_iter()
            .filter(|(s, _)| s.is_support(clash_core.as_ref()))
            .map(|(_, c)| c)
            .collect();
        for item in items {
            logging!(debug, Type::Core, "run builtin script {}", item.uid);
            if let ChainType::Script(script) = item.data {
                match use_script(script, config.clone(), String::from("")).await {
                    Ok((res_config, _)) => {
                        config = res_config;
                    }
                    Err(err) => {
                        logging!(error, Type::Core, "builtin script error `{err}`");
                    }
                }
            }
        }
    }

    config
}

fn cleanup_proxy_groups(mut config: Mapping) -> Mapping {
    const BUILTIN_POLICIES: &[&str] = &["DIRECT", "REJECT", "REJECT-DROP", "PASS"];

    let proxy_names = config
        .get("proxies")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| match item {
                    Value::Mapping(map) => map
                        .get("name")
                        .and_then(Value::as_str)
                        .map(|name| name.to_owned().into()),
                    Value::String(name) => Some(name.to_owned().into()),
                    _ => None,
                })
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let group_names = config
        .get("proxy-groups")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| {
                    item.as_mapping()
                        .and_then(|map| map.get("name"))
                        .and_then(Value::as_str)
                        .map(std::convert::Into::into)
                })
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let provider_names = config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .map(|map| {
            map.keys()
                .filter_map(Value::as_str)
                .map(std::convert::Into::into)
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let mut allowed_names = proxy_names;
    allowed_names.extend(group_names);
    allowed_names.extend(provider_names.iter().cloned());
    allowed_names.extend(BUILTIN_POLICIES.iter().map(|p| (*p).into()));

    if let Some(Value::Sequence(groups)) = config.get_mut("proxy-groups") {
        for group in groups {
            if let Some(group_map) = group.as_mapping_mut() {
                let mut has_valid_provider = false;

                if let Some(Value::Sequence(uses)) = group_map.get_mut("use") {
                    uses.retain(|provider| match provider {
                        Value::String(name) => {
                            let exists = provider_names.contains(name.as_str());
                            has_valid_provider = has_valid_provider || exists;
                            exists
                        }
                        _ => false,
                    });
                }

                if let Some(Value::Sequence(proxies)) = group_map.get_mut("proxies") {
                    proxies.retain(|proxy| match proxy {
                        Value::String(name) => allowed_names.contains(name.as_str()) || has_valid_provider,
                        _ => true,
                    });
                }
            }
        }
    }

    let config = rewrite_rules(config, &allowed_names);

    config
}

fn rewrite_rule_target(rule_str: &str, allowed_names: &HashSet<String>, default_group: &str) -> std::string::String {
    let parts: Vec<&str> = rule_str.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return rule_str.to_owned();
    }

    let rule_type = parts[0].to_ascii_uppercase();

    // Determine the index of the target group in the parts array
    let target_idx = if rule_type == "MATCH" {
        if parts.len() >= 2 { Some(1) } else { None }
    } else if parts.len() == 3 {
        Some(2)
    } else if parts.len() >= 4 {
        // e.g. GEOIP,CN,DIRECT,no-resolve -> target is at index 2
        // Check if the last part is a known option
        let last_upper = parts[parts.len() - 1].to_ascii_uppercase();
        if last_upper == "NO-RESOLVE" || last_upper == "FORCE-DNS" {
            Some(2)
        } else {
            Some(parts.len() - 1)
        }
    } else {
        None
    };

    if let Some(idx) = target_idx {
        if idx < parts.len() {
            let current_target = parts[idx];

            // Check case-insensitive match first
            let matched_allowed = allowed_names
                .iter()
                .find(|name| name.as_str().eq_ignore_ascii_case(current_target));

            if let Some(allowed_name) = matched_allowed {
                if allowed_name.as_str() != current_target {
                    let mut new_parts = parts;
                    new_parts[idx] = allowed_name.as_str();
                    return new_parts.join(",");
                }
            } else {
                // Target not found in allowed_names, fallback to default_group
                let mut new_parts = parts;
                new_parts[idx] = default_group;
                return new_parts.join(",");
            }
        }
    }

    rule_str.to_owned()
}

fn rewrite_rules(mut config: Mapping, allowed_names: &HashSet<String>) -> Mapping {
    let default_group = config
        .get("proxy-groups")
        .and_then(|v| v.as_sequence())
        .and_then(|seq| seq.first())
        .and_then(|item| item.as_mapping())
        .and_then(|map| map.get("name"))
        .and_then(Value::as_str)
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "PROXY".to_owned());

    let rule_fields = ["rules", "prepend-rules", "append-rules"];
    for field in rule_fields {
        if let Some(Value::Sequence(rules)) = config.get_mut(field) {
            for rule_val in rules.iter_mut() {
                if let Value::String(rule_str) = rule_val {
                    let rewritten = rewrite_rule_target(rule_str.as_str(), allowed_names, &default_group);
                    *rule_val = Value::String(rewritten.into());
                }
            }
        }
    }
    config
}

async fn apply_dns_settings(mut config: Mapping, enable_dns_settings: bool) -> Mapping {
    if enable_dns_settings && let Ok(app_dir) = dirs::app_home_dir() {
        let dns_path = app_dir.join(constants::files::DNS_CONFIG);

        if dns_path.exists()
            && let Ok(dns_yaml) = fs::read_to_string(&dns_path).await
            && let Ok(dns_config) = serde_yaml_ng::from_str::<serde_yaml_ng::Mapping>(&dns_yaml)
        {
            if let Some(hosts_value) = dns_config.get("hosts")
                && hosts_value.is_mapping()
            {
                config.insert("hosts".into(), hosts_value.clone());
                logging!(info, Type::Core, "apply hosts configuration");
            }

            if let Some(dns_value) = dns_config.get("dns") {
                if let Some(dns_mapping) = dns_value.as_mapping() {
                    config.insert("dns".into(), dns_mapping.clone().into());
                    logging!(info, Type::Core, "apply dns_config.yaml (dns section)");
                }
            } else {
                config.insert("dns".into(), dns_config.into());
                logging!(info, Type::Core, "apply dns_config.yaml");
            }
        }
    }

    config
}

async fn enforce_mini_agreements(mut config: Mapping) -> Mapping {
    // 1. Extract all raw proxies from `proxies` sequence
    let mut proxy_names = Vec::new();
    if let Some(Value::Sequence(proxies)) = config.get("proxies") {
        for p in proxies {
            if let Some(name) = p.as_mapping().and_then(|m| m.get("name")).and_then(Value::as_str) {
                proxy_names.push(Value::from(name.to_owned()));
            } else if let Some(name) = p.as_str() {
                proxy_names.push(Value::from(name.to_owned()));
            }
        }
    }

    // 2. Extract all proxy-provider names from `proxy-providers` mapping
    let mut provider_names = Vec::new();
    if let Some(Value::Mapping(providers)) = config.get("proxy-providers") {
        for (k, _) in providers {
            if let Some(name) = k.as_str() {
                provider_names.push(Value::from(name.to_owned()));
            }
        }
    }

    // 3. Construct a single group named "PROXY" with type "select"
    let mut single_group = Mapping::new();
    single_group.insert(Value::from("name"), Value::from("PROXY"));
    single_group.insert(Value::from("type"), Value::from("select"));
    if !proxy_names.is_empty() {
        single_group.insert(Value::from("proxies"), Value::from(proxy_names));
    }
    if !provider_names.is_empty() {
        single_group.insert(Value::from("use"), Value::from(provider_names));
    }
    // If both are empty, fallback to DIRECT
    if single_group.get("proxies").is_none() && single_group.get("use").is_none() {
        single_group.insert(Value::from("proxies"), Value::from(vec![Value::from("DIRECT")]));
    }

    // Replace the entire proxy-groups sequence
    config.insert(
        Value::from("proxy-groups"),
        Value::from(vec![Value::from(single_group)]),
    );

    // 4. Inject rule-provider for "gfwlist"
    let mut gfwlist_provider = Mapping::new();
    gfwlist_provider.insert(Value::from("type"), Value::from("http"));
    gfwlist_provider.insert(Value::from("behavior"), Value::from("domain"));
    gfwlist_provider.insert(
        Value::from("url"),
        Value::from("https://cdn.jsdelivr.net/gh/Loyalsoldier/clash-rules@release/gfw.txt"),
    );
    gfwlist_provider.insert(Value::from("path"), Value::from("./ruleset/gfwlist.yaml"));
    gfwlist_provider.insert(Value::from("interval"), Value::from(86400));
    gfwlist_provider.insert(Value::from("proxy"), Value::from("PROXY"));

    let mut rule_providers = match config.remove("rule-providers") {
        Some(Value::Mapping(m)) => m,
        _ => Mapping::new(),
    };
    rule_providers.insert(Value::from("gfwlist"), Value::from(gfwlist_provider));
    config.insert(Value::from("rule-providers"), Value::from(rule_providers));

    // 5. Construct the rules list
    let mut final_rules = Vec::new();

    // Add manual rules from prepend-rules
    if let Some(Value::Sequence(prepend)) = config.remove("prepend-rules") {
        for rule in prepend {
            final_rules.push(rule);
        }
    }

    // Add GEOSITE strong rules
    final_rules.push(Value::from("GEOSITE,google,PROXY"));
    final_rules.push(Value::from("GEOSITE,github,PROXY"));
    final_rules.push(Value::from("GEOSITE,telegram,PROXY"));

    // Add GFWList fallback rule
    final_rules.push(Value::from("RULE-SET,gfwlist,PROXY"));

    // Add manual rules from append-rules
    if let Some(Value::Sequence(append)) = config.remove("append-rules") {
        for rule in append {
            final_rules.push(rule);
        }
    }

    // Read verge config to get the rule fallback type
    let verge = Config::verge().await.latest_arc();
    let rule_fallback = verge.rule_fallback.as_deref().unwrap_or("proxy");

    // Add MATCH final rule according to the fallback type
    if rule_fallback == "proxy" {
        final_rules.push(Value::from("MATCH,PROXY"));
    } else {
        final_rules.push(Value::from("MATCH,DIRECT"));
    }

    config.insert(Value::from("rules"), Value::from(final_rules));

    config
}

/// Enhance mode
/// 返回最终订阅、该订阅包含的键、和script执行的结果
pub async fn enhance() -> Result<(Mapping, HashSet<String>, HashMap<String, ResultLog>)> {
    // gather config values
    let cfg_vals = get_config_values().await;
    let ConfigValues {
        clash_config,
        clash_core,
        enable_tun,
        enable_builtin,
        socks_enabled,
        http_enabled,
        enable_dns_settings,
        #[cfg(not(target_os = "windows"))]
        redir_enabled,
        #[cfg(target_os = "linux")]
        tproxy_enabled,
    } = cfg_vals;

    // collect profile items
    let profile = collect_profile_items().await?;
    let config = profile.config;
    let merge_item = profile.merge_item;
    let script_item = profile.script_item;
    let rules_item = profile.rules_item;
    let proxies_item = profile.proxies_item;
    let groups_item = profile.groups_item;
    let global_merge = profile.global_merge;
    let global_script = profile.global_script;
    let profile_name = profile.profile_name;

    // process globals
    let (config, exists_keys, result_map) =
        process_global_items(config, global_merge, global_script, &profile_name).await;

    // process profile-specific items
    let (config, exists_keys, result_map) = process_profile_items(
        config,
        exists_keys,
        result_map,
        rules_item,
        proxies_item,
        groups_item,
        merge_item,
        script_item,
        &profile_name,
    )
    .await;

    // merge default clash config
    let config = merge_default_config(
        config,
        clash_config,
        socks_enabled,
        http_enabled,
        #[cfg(not(target_os = "windows"))]
        redir_enabled,
        #[cfg(target_os = "linux")]
        tproxy_enabled,
    )
    .await;

    // builtin scripts
    let mut config = apply_builtin_scripts(config, clash_core, enable_builtin).await;

    config = cleanup_proxy_groups(config);
    config = enforce_mini_agreements(config).await;

    config = use_tun(config, enable_tun);
    config = use_sort(config);

    // dns settings
    config = apply_dns_settings(config, enable_dns_settings).await;

    let mut exists_keys_set = HashSet::new();
    exists_keys_set.extend(exists_keys);

    Ok((config, exists_keys_set, result_map))
}

#[allow(clippy::expect_used, clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::cleanup_proxy_groups;

    #[test]
    fn remove_missing_proxies_from_groups() {
        let config_str = r#"
proxies:
  - name: "alive-node"
    type: ss
proxy-groups:
  - name: "manual"
    type: select
    proxies:
      - "alive-node"
      - "missing-node"
      - "DIRECT"
  - name: "nested"
    type: select
    proxies:
      - "manual"
      - "ghost"
"#;

        let mut config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("manual"))
            .and_then(|group| group.as_mapping())
            .expect("manual group should exist");

        let manual_proxies = manual_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("manual proxies should be a sequence");

        assert_eq!(manual_proxies.len(), 2);
        assert!(manual_proxies.iter().any(|p| p.as_str() == Some("alive-node")));
        assert!(manual_proxies.iter().any(|p| p.as_str() == Some("DIRECT")));

        let nested_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("nested"))
            .and_then(|group| group.as_mapping())
            .expect("nested group should exist");

        let nested_proxies = nested_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("nested proxies should be a sequence");

        assert_eq!(nested_proxies.len(), 1);
        assert_eq!(nested_proxies[0].as_str(), Some("manual"));
    }

    #[test]
    fn keep_provider_backed_groups_intact() {
        let config_str = r#"
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
proxies: []
proxy-groups:
  - name: "manual"
    type: select
    use:
      - "providerA"
      - "ghostProvider"
    proxies:
      - "dynamic-node"
      - "DIRECT"
"#;

        let mut config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("manual"))
            .and_then(|group| group.as_mapping())
            .expect("manual group should exist");

        let uses = manual_group
            .get("use")
            .and_then(|v| v.as_sequence())
            .expect("use should be a sequence");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].as_str(), Some("providerA"));

        let proxies = manual_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("proxies should be a sequence");
        assert_eq!(proxies.len(), 2);
        assert!(proxies.iter().any(|p| p.as_str() == Some("dynamic-node")));
        assert!(proxies.iter().any(|p| p.as_str() == Some("DIRECT")));
    }

    #[test]
    fn prune_invalid_provider_and_proxies_without_provider() {
        let config_str = r#"
proxy-groups:
  - name: "manual"
    type: select
    use:
      - "ghost-provider"
    proxies:
      - "ghost-node"
      - "DIRECT"
"#;

        let mut config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("manual"))
            .and_then(|group| group.as_mapping())
            .expect("manual group should exist");

        let uses = manual_group
            .get("use")
            .and_then(|v| v.as_sequence())
            .expect("use should be a sequence");
        assert_eq!(uses.len(), 0);

        let proxies = manual_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("proxies should be a sequence");
        assert_eq!(proxies.len(), 1);
        assert_eq!(proxies[0].as_str(), Some("DIRECT"));
    }

    #[tokio::test]
    async fn test_enforce_mini_agreements_logic() {
        use super::enforce_mini_agreements;
        use serde_yaml_ng::{Mapping, Value};

        let config_str = r#"
proxies:
  - name: "node-A"
    type: ss
  - name: "node-B"
    type: vmess
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
proxy-groups:
  - name: "Group-A"
    type: select
    proxies:
      - "node-A"
  - name: "Group-B"
    type: select
    proxies:
      - "node-B"
prepend-rules:
  - PROCESS-NAME,custom-process,DIRECT
append-rules:
  - DOMAIN,custom-domain,REJECT
"#;

        let mut config: Mapping = serde_yaml_ng::from_str(config_str).unwrap();
        config = enforce_mini_agreements(config).await;

        // 1. Verify single PROXY group
        let groups = config.get("proxy-groups").and_then(Value::as_sequence).unwrap();
        assert_eq!(groups.len(), 1);
        let proxy_group = groups[0].as_mapping().unwrap();
        assert_eq!(proxy_group.get("name").unwrap().as_str(), Some("PROXY"));
        assert_eq!(proxy_group.get("type").unwrap().as_str(), Some("select"));

        let group_proxies = proxy_group.get("proxies").and_then(Value::as_sequence).unwrap();
        assert_eq!(group_proxies.len(), 2);
        assert_eq!(group_proxies[0].as_str(), Some("node-A"));
        assert_eq!(group_proxies[1].as_str(), Some("node-B"));

        let group_uses = proxy_group.get("use").and_then(Value::as_sequence).unwrap();
        assert_eq!(group_uses.len(), 1);
        assert_eq!(group_uses[0].as_str(), Some("providerA"));

        // 2. Verify gfwlist provider
        let providers = config.get("rule-providers").and_then(Value::as_mapping).unwrap();
        let gfwlist = providers.get("gfwlist").and_then(Value::as_mapping).unwrap();
        assert_eq!(gfwlist.get("type").unwrap().as_str(), Some("http"));
        assert_eq!(gfwlist.get("behavior").unwrap().as_str(), Some("domain"));
        assert_eq!(
            gfwlist.get("url").unwrap().as_str(),
            Some("https://cdn.jsdelivr.net/gh/Loyalsoldier/clash-rules@release/gfw.txt")
        );
        assert_eq!(gfwlist.get("proxy").unwrap().as_str(), Some("PROXY"));

        let rules = config.get("rules").and_then(Value::as_sequence).unwrap();
        assert_eq!(rules.len(), 7);
        assert_eq!(rules[0].as_str(), Some("PROCESS-NAME,custom-process,DIRECT"));
        assert_eq!(rules[1].as_str(), Some("GEOSITE,google,PROXY"));
        assert_eq!(rules[2].as_str(), Some("GEOSITE,github,PROXY"));
        assert_eq!(rules[3].as_str(), Some("GEOSITE,telegram,PROXY"));
        assert_eq!(rules[4].as_str(), Some("RULE-SET,gfwlist,PROXY"));
        assert_eq!(rules[5].as_str(), Some("DOMAIN,custom-domain,REJECT"));
        assert_eq!(rules[6].as_str(), Some("MATCH,DIRECT"));

        // Verify prepend-rules and append-rules keys are removed
        assert!(config.get("prepend-rules").is_none());
        assert!(config.get("append-rules").is_none());
    }
}
