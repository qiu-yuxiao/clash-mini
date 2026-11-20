mod agreements;
mod chain;
mod dns;
pub mod field;
mod merge;
pub mod seq;
mod tun;

#[cfg(test)]
mod tests;

pub use self::agreements::{read_persisted_proxy_and_filter, sync_runtime_proxy_group_head};

use self::agreements::{enforce_mini_agreements, get_merged_proxies};
use self::dns::{apply_mandatory_dns_settings, sanitize_dns_upstreams};
use self::{
    chain::{AsyncChainItemFrom as _, ChainItem, ChainType},
    field::{use_keys, use_lowercase, use_sort},
    merge::use_merge,
    seq::{SeqMap, use_seq},
    tun::use_tun_with_sniffer,
};
use crate::config::Config;
use crate::config::IVerge;
use anyhow::{Context as _, Result};
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};

type ResultLog = Vec<(String, String)>;
#[derive(Debug)]
struct ConfigValues {
    clash_config: Mapping,
    clash_core: Option<String>,
    enable_tun: bool,
    enable_tun_sniffer: Option<bool>,
    enable_builtin: bool,
    socks_enabled: bool,
    http_enabled: bool,
}

#[derive(Debug)]
struct ProfileItems {
    config: Mapping,
    merge_item: ChainItem,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    global_merge: ChainItem,
}

impl Default for ProfileItems {
    fn default() -> Self {
        Self {
            config: Default::default(),
            merge_item: ChainItem {
                data: ChainType::Merge(Mapping::new()),
            },
            rules_item: ChainItem {
                data: ChainType::Rules(SeqMap::default()),
            },
            proxies_item: ChainItem {
                data: ChainType::Proxies(SeqMap::default()),
            },
            global_merge: ChainItem {
                data: ChainType::Merge(Mapping::new()),
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
        ref enable_tun_sniffer,
        ref enable_builtin_enhanced,
        ref verge_socks_enabled,
        ref verge_http_enabled,
        ..
    } = *verge_arc;

    let (clash_core, enable_tun, enable_tun_sniffer, enable_builtin, socks_enabled, http_enabled) = (
        Some(verge_arc.get_valid_clash_core()),
        enable_tun_mode.unwrap_or(false),
        *enable_tun_sniffer,
        enable_builtin_enhanced.unwrap_or(true),
        verge_socks_enabled.unwrap_or(false),
        verge_http_enabled.unwrap_or(false),
    );

    drop(verge_arc);
    drop(verge);

    ConfigValues {
        clash_config,
        clash_core,
        enable_tun,
        enable_tun_sniffer,
        enable_builtin,
        socks_enabled,
        http_enabled,
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
        .with_context(|| format!("failed to read current profile \"{current_profile_uid}\""))?;

    let current_item = match profiles_arc.get_item(&current_profile_uid) {
        Ok(item) => item,
        Err(err) => {
            return Err(err).with_context(|| format!("failed to get current profile \"{current_profile_uid}\""));
        }
    };

    // global_merge 已无条件加载 "Merge"，因此配置级 merge 仅在显式指向其它专属 Merge 时加载，避免 Merge.yaml 重复合并两次（BUG-409）
    let merge_uid = current_item
        .current_merge()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "Merge");
    let rules_uid = current_item.current_rules().cloned().unwrap_or_else(|| "Rules".into());
    let proxies_uid = current_item
        .current_proxies()
        .cloned()
        .unwrap_or_else(|| "Proxies".into());

    let merge_item = {
        let item = merge_uid.and_then(|uid| profiles_arc.get_item(uid).ok().cloned());
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        data: ChainType::Merge(Mapping::new()),
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
        data: ChainType::Proxies(SeqMap::default()),
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
        data: ChainType::Merge(Mapping::new()),
    });

    drop(profiles_arc);

    Ok(ProfileItems {
        config: current,
        merge_item,
        rules_item,
        proxies_item,
        global_merge,
    })
}

fn process_global_items(mut config: Mapping, global_merge: ChainItem) -> (Mapping, Vec<String>) {
    let mut exists_keys = use_keys(&config).collect::<Vec<_>>();

    if let ChainType::Merge(merge) = global_merge.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    (config, exists_keys)
}

fn process_profile_items(
    mut config: Mapping,
    mut exists_keys: Vec<String>,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    merge_item: ChainItem,
) -> (Mapping, Vec<String>) {
    if let ChainType::Rules(rules) = rules_item.data {
        config = use_seq(rules, config.to_owned(), "rules");
    }

    if let ChainType::Proxies(proxies) = proxies_item.data {
        config = use_seq(proxies, config.to_owned(), "proxies");
    }

    if let ChainType::Merge(merge) = merge_item.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    (config, exists_keys)
}

async fn merge_default_config(
    mut config: Mapping,
    clash_config: Mapping,
    socks_enabled: bool,
    http_enabled: bool,
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
            // 注：旧默认值 stack=system → gvisor 的归一化统一在 IClashTemp::new() 完成
            //（clash.yaml 加载时改写），此处不再重复判断，避免同一语义两处维护。
            config.insert("tun".into(), tun.into());
        } else if matches!(key.as_str(), Some("dns") | Some("profile")) {
            // dns / profile 是"订阅优先"的复合节点：仅补缺，不覆盖订阅已声明的字段。
            // 与 tun 不同，二者归订阅所有（Mini 无对应覆写入口），模板只提供兜底默认值。
            let section_name = key.as_str().unwrap_or_default().to_string();
            let mut section = config.get_mut(section_name.as_str()).map_or_else(Mapping::new, |val| {
                val.as_mapping().cloned().unwrap_or_else(Mapping::new)
            });
            let defaults = value.as_mapping().cloned().unwrap_or_else(Mapping::new);
            for (default_key, default_value) in defaults.into_iter() {
                let absent = match default_key.as_str() {
                    Some(name) => !section.contains_key(Value::from(name)),
                    None => true,
                };
                if absent {
                    section.insert(default_key, default_value);
                }
            }
            config.insert(key, section.into());
        } else if key.as_str() == Some("tcp-concurrent") {
            // tcp-concurrent 是 Mini 为省资源自加的标量默认值（上游模板无此键），
            // 但订阅可能显式声明。与 dns/profile 同理：订阅优先，仅当订阅未声明时补
            // false，避免强制覆盖订阅的并发连接语义。
            if !config.contains_key(Value::from("tcp-concurrent")) {
                config.insert(key, value);
            }
        } else {
            if key.as_str() == Some("socks-port") && !socks_enabled {
                config.remove("socks-port");
                continue;
            }
            if key.as_str() == Some("port") && !http_enabled {
                config.remove("port");
                continue;
            }
            // Windows 下的 redir-port / tproxy-port 为其他平台专属键，直接跳过不写入
            if matches!(key.as_str(), Some("redir-port") | Some("tproxy-port")) {
                continue;
            }
            // config_version 是 Clash Mini 自己的**配置格式版本号**（随 clash.yaml 落盘），
            // 不是 mihomo 配置项，绝不能混进运行时配置。实测内核会忽略未知顶层键（-t 校验通过），
            // 但运行时配置理应只含内核认识的键，避免将来内核改为严格解析时校验失败。
            if key.as_str() == Some("config_version") {
                continue;
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

/// 原生 Rust 内建规则卫兵（替代原内置 `meta_guard.js` / `meta_hy_alpn.js`，零 JS 引擎开销）
fn apply_builtin_guards(mut config: Mapping, clash_core: Option<&str>, enable_builtin: bool) -> Mapping {
    if !enable_builtin || clash_core.is_some_and(|core| core != "mini-mihomo") {
        return config;
    }

    if config.get("mode").and_then(Value::as_str) == Some("script") {
        config.insert(Value::from("mode"), Value::from("rule"));
    }

    if let Some(proxies) = config.get_mut("proxies").and_then(Value::as_sequence_mut) {
        for proxy_val in proxies {
            if let Some(map) = proxy_val.as_mapping_mut()
                && map.get("type").and_then(Value::as_str) == Some("hysteria")
                && let Some(Value::String(alpn)) = map.get("alpn").cloned()
            {
                map.insert(Value::from("alpn"), Value::Sequence(vec![Value::String(alpn)]));
            }
        }
    }

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
        enable_tun_sniffer,
        enable_builtin,
        socks_enabled,
        http_enabled,
    } = cfg_vals;

    // collect profile items
    let profile = collect_profile_items().await?;
    let mut config = profile.config;

    // Always merge proxies from all remote and local subscription files
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();
    drop(profiles);
    let merged = get_merged_proxies(&profiles_arc);
    config.insert(Value::from("proxies"), Value::from(merged));

    let merge_item = profile.merge_item;
    let rules_item = profile.rules_item;
    let proxies_item = profile.proxies_item;
    let global_merge = profile.global_merge;

    // process globals
    let (config, exists_keys) = process_global_items(config, global_merge);

    // process profile-specific items
    let (config, exists_keys) = process_profile_items(config, exists_keys, rules_item, proxies_item, merge_item);

    // merge default clash config
    let config = merge_default_config(config, clash_config, socks_enabled, http_enabled).await;

    // builtin guards
    let mut config = apply_builtin_guards(config, clash_core.as_deref(), enable_builtin);

    config = enforce_mini_agreements(config).await;

    config = use_tun_with_sniffer(config, enable_tun, enable_tun_sniffer);
    config = use_sort(config);

    // dns settings
    config = apply_mandatory_dns_settings(config);
    config = sanitize_dns_upstreams(config);

    let mut exists_keys_set = HashSet::new();
    exists_keys_set.extend(exists_keys);

    Ok((config, exists_keys_set, HashMap::new()))
}
