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
use crate::config::IVerge;
use crate::utils::dirs;
use crate::{config::Config, utils::tmpl};
use anyhow::{Context as _, Result};
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};

type ResultLog = Vec<(String, String)>;
#[derive(Debug)]
struct ConfigValues {
    clash_config: Mapping,
    clash_core: Option<String>,
    enable_tun: bool,
    enable_builtin: bool,
    socks_enabled: bool,
    http_enabled: bool,
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
        ..
    } = *verge_arc;

    let (clash_core, enable_tun, enable_builtin, socks_enabled, http_enabled) = (
        Some(verge_arc.get_valid_clash_core()),
        enable_tun_mode.unwrap_or(false),
        enable_builtin_enhanced.unwrap_or(true),
        verge_socks_enabled.unwrap_or(false),
        verge_http_enabled.unwrap_or(false),
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
            // 若 stack 依然为旧默认值 "system"，自动平滑升级为 DEFAULT_STACK（gvisor）
            if tun.get("stack").and_then(|v| v.as_str()) == Some("system") {
                tun.insert("stack".into(), crate::constants::tun::DEFAULT_STACK.into());
            }
            config.insert("tun".into(), tun.into());
        } else if matches!(key.as_str(), Some("dns") | Some("profile")) {
            // dns / profile 是"订阅优先"的复合节点：仅补缺，不覆盖订阅已声明的字段。
            // 与 tun 不同，二者归订阅所有（Mini 无对应覆写入口），模板只提供兜底默认值。
            let section_name = key.as_str().unwrap_or_default().to_string();
            let mut section = config
                .get_mut(section_name.as_str())
                .map_or_else(Mapping::new, |val| val.as_mapping().cloned().unwrap_or_else(Mapping::new));
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

fn apply_mandatory_dns_settings(mut config: Mapping) -> Mapping {
    use serde_yaml_ng::Value;

    // 【修复】改为「仅补全缺失字段」，绝不整体替换 dns 节点。
    // 原实现 config.insert("dns", ...) 只保留 enable / enhanced-mode=redir-host /
    // cache-* / nameserver=[8.8.8.8,114.114.114.114]，会抹掉订阅
    // 提供的 default-nameserver、proxy-server-nameserver、fallback、fallback-filter、
    // nameserver-policy 等字段，并在流水线末端强制 redir-host 覆盖 use_tun 写入的
    // fake-ip。叠加 8.8.8.8 在中国大陆直连不可达，导致 TUN 下 redir-host 需要把域名
    // 实时解析成真实 IP 时出现间歇性解析失败（切换代理模式触发 reload 清 DNS 缓存后恢复）。
    let mut dns_config = match config.remove("dns") {
        Some(Value::Mapping(m)) => m,
        _ => Mapping::new(),
    };

    // 逐项补全：已存在的一律保留，不覆盖
    if !dns_config.contains_key(Value::from("enable")) {
        dns_config.insert("enable".into(), Value::Bool(true));
    }
    // 注：不设置 enhanced-mode / fake-ip-range —— 由 use_tun 在 TUN 场景写入 fake-ip，
    // 非 TUN 场景交由内核默认值（redir-host），避免覆盖用户显式配置。
    if !dns_config.contains_key(Value::from("nameserver")) {
        dns_config.insert(
            "nameserver".into(),
            Value::Sequence(vec![
                Value::String("https://doh.pub/dns-query".into()),
                Value::String("https://dns.alidns.com/dns-query".into()),
                Value::String("114.114.114.114".into()),
            ]),
        );
    }
    if !dns_config.contains_key(Value::from("default-nameserver")) {
        dns_config.insert(
            "default-nameserver".into(),
            Value::Sequence(vec![
                Value::String("223.6.6.6".into()),
                Value::String("223.5.5.5".into()),
                Value::String("114.114.114.114".into()),
            ]),
        );
    }
    if !dns_config.contains_key(Value::from("proxy-server-nameserver")) {
        dns_config.insert(
            "proxy-server-nameserver".into(),
            Value::Sequence(vec![
                Value::String("https://doh.pub/dns-query".into()),
                Value::String("https://dns.alidns.com/dns-query".into()),
                Value::String("tls://223.5.5.5".into()),
            ]),
        );
    }

    // 【冷启动保底】default-nameserver / proxy-server-nameserver 确保含明文 UDP 上游
    // 223.5.5.5（仅追加，不覆盖已有条目）。订阅的加密 DNS（DoH/DoT）服务器域名需先经
    // default-nameserver 解析，冷启动窗口内（TUN 接口绑定延迟、系统 DNS 被 dns-hijack
    // any:53 劫持回环、8.8.8.8 大陆直连不可达）该链可能全断 → 节点域名解析不出 →
    // 首轮测速全 timeout。明文 IP 上游无需任何前置解析，接口绑定后即可用，消除鸡生蛋。
    // 注：default-nameserver 补缺默认值已含明文上游，此处统一兜住"订阅已提供但缺明文"的情况。
    for key in ["default-nameserver", "proxy-server-nameserver"] {
        let fallback = Value::String("223.5.5.5".into());
        if let Some(val) = dns_config.get_mut(key) {
            match val {
                Value::Sequence(seq) => {
                    if !seq.contains(&fallback) {
                        seq.push(fallback);
                        logging!(
                            info,
                            Type::Core,
                            "DNS {key} 追加保底明文上游 223.5.5.5（冷启动防鸡生蛋）"
                        );
                    }
                }
                Value::String(s) => {
                    if s != "223.5.5.5" {
                        *val = Value::Sequence(vec![Value::String(s.clone()), fallback]);
                        logging!(
                            info,
                            Type::Core,
                            "DNS {key} 标量提升为列表并追加保底明文上游 223.5.5.5（冷启动防鸡生蛋）"
                        );
                    }
                }
                _ => {}
            }
        }
    }

    config.insert("dns".into(), Value::Mapping(dns_config));
    logging!(
        info,
        Type::Core,
        "补全缺失的 DNS 字段（非覆盖式，保留订阅/用户已有设置）"
    );
    config
}

async fn enforce_mini_agreements(mut config: Mapping) -> Mapping {
    // 1. Extract all raw proxies from `proxies` sequence
    //    过滤广告/假节点（"网址"、"官网"、"剩余流量" 等），避免它们进入 PROXY 组后
    //    被内核 reload_config 选中触发自愈死循环
    let mut proxy_names = Vec::new();
    if let Some(Value::Sequence(proxies)) = config.get("proxies") {
        for p in proxies {
            let name_opt = p
                .as_mapping()
                .and_then(|m| m.get("name"))
                .and_then(Value::as_str)
                .or_else(|| p.as_str());
            if let Some(name) = name_opt {
                if !crate::utils::node::is_dummy_node(name) {
                    proxy_names.push(Value::from(name.to_owned()));
                }
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

    // 3. Construct the control group "PROXY" with type "select"
    //    —— 用户/app 控制，选择稳定、手动选点持久（不被内核自动翻回）
    let proxy_names_value = Value::from(proxy_names.clone());
    let provider_names_value = Value::from(provider_names.clone());

    let mut single_group = Mapping::new();
    single_group.insert(Value::from("name"), Value::from("PROXY"));
    single_group.insert(Value::from("type"), Value::from("select"));
    if !proxy_names.is_empty() {
        single_group.insert(Value::from("proxies"), proxy_names_value.clone());
    }
    if !provider_names.is_empty() {
        single_group.insert(Value::from("use"), provider_names_value.clone());
    }
    // If both are empty, fallback to DIRECT
    if single_group.get("proxies").is_none() && single_group.get("use").is_none() {
        single_group.insert(Value::from("proxies"), Value::from(vec![Value::from("DIRECT")]));
    }

    // 3b. Construct the measurement group "PROXY__METRICS" with type "url-test", same membership.
    //     Mihomo 内核按 interval 周期性直接拨测每个成员，维护其 history/alive（TUN 下可靠）。
    //     这是节点延迟的唯一权威来源；app 不再用 delay_proxy_by_name 自测活跃节点
    //     （旧设计在 TUN 下对活跃节点自测会回环返回 timeout，是断流+全 timeout 的根因）。
    let test_url = Config::verge()
        .await
        .latest_arc()
        .default_latency_test
        .as_deref()
        .unwrap_or("http://cp.cloudflare.com/generate_204")
        .to_string();

    let mut metrics_group = Mapping::new();
    metrics_group.insert(Value::from("name"), Value::from("PROXY__METRICS"));
    metrics_group.insert(Value::from("type"), Value::from("url-test"));
    if !proxy_names.is_empty() {
        metrics_group.insert(Value::from("proxies"), proxy_names_value.clone());
    }
    if !provider_names.is_empty() {
        metrics_group.insert(Value::from("use"), provider_names_value.clone());
    }
    if metrics_group.get("proxies").is_none() && metrics_group.get("use").is_none() {
        metrics_group.insert(Value::from("proxies"), Value::from(vec![Value::from("DIRECT")]));
    }
    metrics_group.insert(Value::from("url"), Value::from(test_url));
    metrics_group.insert(Value::from("interval"), Value::from(300));

    // Replace the entire proxy-groups sequence with [PROXY (control), PROXY__METRICS (measurement)]
    config.insert(
        Value::from("proxy-groups"),
        Value::from(vec![Value::from(single_group), Value::from(metrics_group)]),
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

    // 注：规则集从 GEOIP/GEOSITE 更换为 GFWList。
    // GFWList 负责判断被墙域名走代理，GEOIP/GEOSITE 数据库不再需要，
    // 省去 geoip.dat(~8MB) + geosite.dat(~30MB) 文件的常驻内存开销。
    // MATCH 兜底策略仍由 rule_fallback 控制，与原逻辑一致。

    // Add GFWList rule: only explicitly blocked domains go through proxy
    final_rules.push(Value::from("RULE-SET,gfwlist,PROXY"));

    // Add local LAN bypass rules (DIRECT) to avoid DNS loopback & LAN routing deadlock
    final_rules.push(Value::from("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,172.16.0.0/12,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,192.168.0.0/16,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,100.64.0.0/10,DIRECT,no-resolve"));

    // Add manual rules from append-rules
    if let Some(Value::Sequence(append)) = config.remove("append-rules") {
        for rule in append {
            final_rules.push(rule);
        }
    }

    // Read verge config to get the rule fallback type
    let verge = Config::verge().await.latest_arc();
    let rule_fallback = verge.rule_fallback.as_deref().unwrap_or("direct");

    // Add MATCH final rule according to the fallback type
    if rule_fallback == "proxy" {
        final_rules.push(Value::from("MATCH,PROXY"));
    } else {
        final_rules.push(Value::from("MATCH,DIRECT"));
    }

    config.insert(Value::from("rules"), Value::from(final_rules));

    config
}

#[allow(clippy::collapsible_if, clippy::needless_borrows_for_generic_args)]
fn get_merged_proxies(profiles: &crate::config::profiles::IProfiles) -> Vec<Value> {
    use chrono::{Local, TimeZone as _};
    let mut all_proxies = Vec::new();
    let mut raw_proxies = Vec::new();

    if let Some(items) = profiles.get_items() {
        for item in items {
            // Only process remote and local types
            if let Some(itype) = &item.itype {
                if itype == "remote" || itype == "local" {
                    // Read file
                    if let Some(file) = &item.file {
                        if let Ok(file_path) = dirs::app_profiles_dir().map(|d| d.join(file.as_str())) {
                            if let Ok(mut mapping) = crate::utils::help::read_mapping(&file_path) {
                                if let Some(Value::Sequence(proxies)) = mapping.remove("proxies") {
                                    // Generate suffix from item.updated timestamp
                                    let ts = item.updated.unwrap_or(0);
                                    let suffix = if let Some(dt) = Local.timestamp_opt(ts, 0).single() {
                                        dt.format("%H%M%S").to_string()
                                    } else {
                                        "000000".to_string()
                                    };

                                    for proxy in proxies {
                                        let original_name = if let Some(map) = proxy.as_mapping() {
                                            map.get(&Value::from("name"))
                                                .and_then(Value::as_str)
                                                .unwrap_or("Proxy")
                                                .to_string()
                                        } else {
                                            "Proxy".to_string()
                                        };
                                        raw_proxies.push((proxy, original_name, suffix.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Identify duplicate (name, suffix) keys
    let mut name_suffix_counts = std::collections::HashMap::new();
    for (_, name, suffix) in &raw_proxies {
        let key = (name.clone(), suffix.clone());
        *name_suffix_counts.entry(key).or_insert(0) += 1;
    }

    // Format names and collect proxies
    let mut name_suffix_counters = std::collections::HashMap::new();
    for (mut proxy, name, suffix) in raw_proxies {
        let key = (name.clone(), suffix.clone());
        let count = name_suffix_counts.get(&key).copied().unwrap_or(0);

        let new_name = if count > 1 {
            let idx = name_suffix_counters.entry(key).or_insert(0);
            *idx += 1;
            format!("{} #{} ({})", name, idx, suffix)
        } else {
            format!("{} ({})", name, suffix)
        };

        if let Some(map) = proxy.as_mapping_mut() {
            map.insert(Value::from("name"), Value::from(new_name));
        }
        all_proxies.push(proxy);
    }

    all_proxies
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
        #[cfg(not(target_os = "windows"))]
        redir_enabled,
        #[cfg(target_os = "linux")]
        tproxy_enabled,
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
    let script_item = profile.script_item;
    let rules_item = profile.rules_item;
    let proxies_item = profile.proxies_item;
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

    config = enforce_mini_agreements(config).await;

    config = use_tun(config, enable_tun);
    config = use_sort(config);

    // dns settings
    config = apply_mandatory_dns_settings(config);

    let mut exists_keys_set = HashSet::new();
    exists_keys_set.extend(exists_keys);

    Ok((config, exists_keys_set, result_map))
}

#[allow(clippy::expect_used, clippy::unwrap_used)]
#[cfg(test)]
mod tests {

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

        // 1. Verify PROXY (control) + PROXY__METRICS (measurement) groups
        let groups = config.get("proxy-groups").and_then(Value::as_sequence).unwrap();
        assert_eq!(groups.len(), 2);

        let proxy_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("PROXY"))
            .and_then(Value::as_mapping)
            .unwrap();
        assert_eq!(proxy_group.get("type").unwrap().as_str(), Some("select"));

        let metrics_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("PROXY__METRICS"))
            .and_then(Value::as_mapping)
            .unwrap();
        assert_eq!(metrics_group.get("type").unwrap().as_str(), Some("url-test"));
        assert!(metrics_group.get("url").is_some());
        assert!(metrics_group.get("interval").is_some());

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
        // prepend-rules(1) + GFWList(1) + IP-CIDR(5) + append-rules(1) + MATCH(1) = 9
        // MATCH=DIRECT because default rule_fallback is "direct" (GFWList mode)
        assert_eq!(rules.len(), 9);
        assert_eq!(rules[0].as_str(), Some("PROCESS-NAME,custom-process,DIRECT"));
        assert_eq!(rules[1].as_str(), Some("RULE-SET,gfwlist,PROXY"));
        assert_eq!(rules[2].as_str(), Some("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
        assert_eq!(rules[3].as_str(), Some("IP-CIDR,172.16.0.0/12,DIRECT,no-resolve"));
        assert_eq!(rules[4].as_str(), Some("IP-CIDR,192.168.0.0/16,DIRECT,no-resolve"));
        assert_eq!(rules[5].as_str(), Some("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve"));
        assert_eq!(rules[6].as_str(), Some("IP-CIDR,100.64.0.0/10,DIRECT,no-resolve"));
        assert_eq!(rules[7].as_str(), Some("DOMAIN,custom-domain,REJECT"));
        assert_eq!(rules[8].as_str(), Some("MATCH,DIRECT"));

        // Verify prepend-rules and append-rules keys are removed
        assert!(config.get("prepend-rules").is_none());
        assert!(config.get("append-rules").is_none());
    }
}
