use crate::config::Config;
use crate::utils::dirs;
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Value};
use std::collections::HashSet;

/// 收敛所有远端 `proxy-providers.*.health-check` 为 `lazy: true`（BUG-399），
/// 并清理因策略组重整而悬空的 `proxy-providers.*.proxy` 引用（BUG-425）。
fn normalize_proxy_providers(config: &mut Mapping, valid_proxy_names: &HashSet<std::string::String>) {
    let Some(Value::Mapping(providers)) = config.get_mut("proxy-providers") else {
        return;
    };
    let mut keys_to_clean = Vec::new();
    for (k, provider_val) in providers.iter_mut() {
        let Some(provider_map) = provider_val.as_mapping_mut() else {
            continue;
        };
        if let Some(Value::Mapping(hc)) = provider_map.get_mut("health-check") {
            hc.insert(Value::from("lazy"), Value::from(true));
        }
        if let Some(proxy_target) = provider_map.get("proxy").and_then(Value::as_str).map(str::trim)
            && !proxy_target.is_empty()
            && !matches!(proxy_target, "DIRECT" | "REJECT" | "PROXY")
            && !valid_proxy_names.contains(proxy_target)
        {
            keys_to_clean.push((
                k.clone(),
                k.as_str().unwrap_or("<unknown>").to_owned(),
                proxy_target.to_owned(),
            ));
        }
    }

    for (key_val, provider_label, dangling_proxy) in keys_to_clean {
        if let Some(provider_map) = providers.get_mut(&key_val).and_then(Value::as_mapping_mut) {
            logging!(
                warn,
                Type::Config,
                "移除 proxy-provider '{provider_label}' 中指向已裁剪策略组的悬空 proxy 字段: {dangling_proxy}"
            );
            provider_map.remove("proxy");
        }
    }
}

pub(super) async fn enforce_mini_agreements(mut config: Mapping) -> Mapping {
    // 1. Extract all raw proxies from `proxies` sequence
    //    先收集全部有效 server 的原始条目再做拓扑分组：既拦截单条广告/结构横幅，
    //    也能对「共享横幅端点但混入未知新文案」的组整组连坐清剿，并将寄生真地区节点折叠归真。
    let mut raw_entries: Vec<crate::utils::node::RawProxyEntry> = Vec::new();
    let mut raw_fp_map: std::collections::HashMap<std::string::String, std::string::String> =
        std::collections::HashMap::new();
    if let Some(Value::Sequence(proxies)) = config.get("proxies") {
        for p in proxies {
            let Some(m) = p.as_mapping() else {
                // 纯字符串形式（"- 节点名"）没有 server 字段，沿用名字过滤
                if let Some(name) = p.as_str() {
                    raw_entries.push(crate::utils::node::RawProxyEntry {
                        name: name.to_owned(),
                        fingerprint: None,
                        is_dummy: crate::utils::node::is_dummy_node(name),
                    });
                }
                continue;
            };
            let Some(name) = m.get("name").and_then(Value::as_str) else {
                continue;
            };
            // 【BUG-301】占位假节点的名字可伪装成 URL 绕过名字过滤（现场：
            // "v0-网址:xxx" + server='1'），但 server 字段藏不住。按 server 有效性
            // 拦截入组；节点定义保留在 proxies: 中（与 dummy 过滤同策略），只是
            // 不允许它成为 PROXY 组成员——否则内核重启后 Selector 落回首位死节点，
            // TUN 下全系统流量打进死地址触发 connect error 风暴。
            if let Some(server) = m.get("server").and_then(Value::as_str) {
                if !crate::utils::node::is_valid_node_server(server) {
                    logging!(
                        warn,
                        Type::Config,
                        "跳过 server 字段无效的节点: {name} (server={server:?})"
                    );
                    continue;
                }
            }
            let fp = crate::utils::node::proxy_endpoint_fingerprint(m);
            if let Some(ref fp_str) = fp {
                raw_fp_map.insert(name.to_owned(), fp_str.clone());
                // 仅按持久化 HTTPS 验真状态预过滤，拓扑连坐由本次订阅独立重算（防端口复用误伤）
                if crate::utils::node::is_https_quarantined_fp(fp_str) {
                    logging!(info, Type::Config, "跳过真 HTTPS 验真已确认拉黑的假节点: {name}");
                    continue;
                }
            }
            raw_entries.push(crate::utils::node::RawProxyEntry {
                name: name.to_owned(),
                fingerprint: fp,
                is_dummy: crate::utils::node::is_dummy_node(name),
            });
        }
    }

    let topology = crate::utils::node::resolve_proxy_clone_topology(raw_entries);
    crate::utils::node::set_runtime_node_fingerprints(raw_fp_map);
    crate::utils::node::set_runtime_tainted_banner_fps(topology.tainted_fps);
    if topology.tainted_banner_removed > 0 {
        logging!(
            info,
            Type::Config,
            "订阅拓扑净化：同指纹横幅连坐剔除寄生节点 {} 个",
            topology.tainted_banner_removed
        );
    }
    if topology.clone_removed > 0 {
        logging!(
            info,
            Type::Config,
            "订阅入组去重：折叠同 server:port 同配置克隆节点 {} 个",
            topology.clone_removed
        );
    }
    let proxy_names: Vec<Value> = topology.names.into_iter().map(Value::from).collect();

    // 2. Normalize all upstream proxy-provider health checks to lazy: true (BUG-399)
    //    and strip dangling `proxy:` group references (BUG-425),
    //    then extract provider names from `proxy-providers` mapping
    let valid_proxy_names: HashSet<std::string::String> = config
        .get("proxies")
        .and_then(Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(|p| p.get("name").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    normalize_proxy_providers(&mut config, &valid_proxy_names);
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

    // 3b. 构造测量专用组 PROXY__METRICS（url-test + lazy: true，避免无流量自治盲测）
    let test_url = Config::verge()
        .await
        .latest_arc()
        .default_latency_test
        .as_deref()
        .unwrap_or("https://cp.cloudflare.com/generate_204")
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
    metrics_group.insert(Value::from("interval"), Value::from(1800));
    metrics_group.insert(Value::from("lazy"), Value::from(true));

    // Replace the entire proxy-groups sequence with [PROXY (control), PROXY__METRICS (measurement)]
    config.insert(
        Value::from("proxy-groups"),
        Value::from(vec![Value::from(single_group), Value::from(metrics_group)]),
    );
    sync_runtime_proxy_group_head(&mut config).await;

    // 4. 配置 GFWList 规则集（构建时预置随包释放，运行时按 24h 间隔经 PROXY 定时更新）
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

    let raw_rule_providers = match config.remove("rule-providers") {
        Some(Value::Mapping(m)) => m,
        _ => Mapping::new(),
    };

    // 5. Construct the rules list
    let mut final_rules = Vec::new();

    // Add manual rules from prepend-rules
    if let Some(Value::Sequence(prepend)) = config.remove("prepend-rules") {
        for rule in prepend {
            final_rules.push(rule);
        }
    }

    // 注：境外域名走代理由 GFWList 规则集判断（规则集从 GEOIP/GEOSITE 更换而来，
    // 省去 geosite.dat 的常驻内存开销；GEOIP 数据与 gfwlist.yaml 均随包携带，
    // 见 utils::init::init_resources）。
    // MATCH 兜底策略由 rule_fallback 控制；IPv6 境内外分流依赖 Country.mmdb
    // （随包携带、离线可用），见下方 IPv6 兜底规则。

    // Add GFWList rule:
    final_rules.push(Value::from("RULE-SET,gfwlist,PROXY"));

    // DNS fallback 上游走代理的 PROXY 规则已统一由 `inject_dns_proxy_rules` 在
    // `sanitize_dns_upstreams` 末端生成（B-1/F-2 单点注入），此处不再重复注入。

    // Add local LAN bypass rules (DIRECT) to avoid DNS loopback & LAN routing deadlock
    // 【IPv4 私网 / 环回 / 链路本地】防止「代理所有流量」（MATCH,PROXY）下本地流量被误转远端代理
    final_rules.push(Value::from("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,172.16.0.0/12,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,192.168.0.0/16,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,100.64.0.0/10,DIRECT,no-resolve"));
    // IPv4 链路本地（APIPA，169.254.0.0/16）：与下方 IPv6 `fe80::/10` 对称，此前缺失
    final_rules.push(Value::from("IP-CIDR,169.254.0.0/16,DIRECT,no-resolve"));

    // 【IPv6 环回 / 未指定 / ULA / 链路本地 / 组播】
    // ULA（fc00::/7）覆盖 TUN 自身地址段（fdfe:dcba:9876::1/126），必须直连。
    final_rules.push(Value::from("IP-CIDR6,::1/128,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::/128,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,fc00::/7,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,fe80::/10,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,ff00::/8,DIRECT,no-resolve"));

    // 【IPv4-mapped IPv6（::ffff:a.b.c.d）的私网/环回等价段】
    // 注意：**不可**整体把 `::ffff:0:0/96` 设为 DIRECT —— 那会把国外目标也直连、绕过代理；
    // 只精确匹配与上方 IPv4 私网规则一一对应的映射段（96 位映射前缀 + IPv4 前缀长度）。
    final_rules.push(Value::from("IP-CIDR6,::ffff:127.0.0.0/104,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:10.0.0.0/104,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:172.16.0.0/108,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:192.168.0.0/112,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:100.64.0.0/106,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:169.254.0.0/112,DIRECT,no-resolve"));

    // Add manual rules from append-rules
    if let Some(Value::Sequence(append)) = config.remove("append-rules") {
        for rule in append {
            final_rules.push(rule);
        }
    }

    // 【国内 GEOIP 兜底】
    // 1) 【必须用 GEOIP 而非 GEOIP6】mihomo（Clash Meta）不支持 `GEOIP6` 规则类型
    //    （老 Clash Premium 语法），写入即 `unsupported rule type: GEOIP6`，会导致配置校验失败、
    //    内核拒绝启动并全部流量回落直连。mihomo 的 `GEOIP` 自动覆盖 IPv4/IPv6（查 mmdb 对应段）。
    // 2) 【刻意不加 no-resolve】保持域名目标先由内核 DNS 解析后再比对 CN IP 段直连。
    // 3) 【不再注入 IP-CIDR6,::/0,PROXY】原位于 GEOIP,CN,DIRECT 与 MATCH,DIRECT 之间的
    //    `IP-CIDR6,::/0,PROXY` 会将境内 CDN/下载/通讯 IPv6 流量及非 GFWList 域名 AAAA 会话
    //    全部送入付费代理节点偷跑流量，故默认 GFWList 模式保持对称直连兜底 `MATCH,DIRECT`。
    // Read verge config to get the rule fallback type
    let verge = Config::verge().await.latest_arc();
    let rule_fallback = verge.rule_fallback.as_deref().unwrap_or("direct");

    // Add MATCH final rule according to the fallback type
    // 注意：默认 GFWList 模式（rule_fallback != "proxy"）仅注入 GEOIP,CN,DIRECT + MATCH,DIRECT 对称兜底；
    // 已移除原先位于 MATCH,DIRECT 之前的 IP-CIDR6,::/0,PROXY，避免境内未命中 GeoIP 的 CDN/下载/通讯 IPv6 流量
    // 及非 GFWList 域名 AAAA 会话偷跑进机场付费节点计费。
    // 仅当用户选择「代理全部流量」（rule_fallback == "proxy"）时，直接落入 MATCH,PROXY。
    if rule_fallback == "proxy" {
        final_rules.push(Value::from("MATCH,PROXY"));
    } else {
        final_rules.push(Value::from("GEOIP,CN,DIRECT"));
        final_rules.push(Value::from("MATCH,DIRECT"));
    }

    // 仅保留被最终 rules（含用户 prepend-rules / append-rules）实际引用的 rule-providers。
    // 机场订阅常自带 China-Site / China-IP / Local-IP 等未被 Clash Mini 规则链使用的
    // rule-providers（3.1.9 现场实证：订阅自带 3 个 .mrs 规则集共 ~700KB，虽未被 rules
    // 引用却仍触发内核首次启动联网拉取并常驻内存；若机场规则源被墙更会拖慢甚至卡死首次校验）。
    // 按引用关系裁剪后，默认仅保留已随包预置的 gfwlist，实现首次启动路由规则 100% 离线零拉取。
    let mut referenced_providers: HashSet<std::string::String> = HashSet::new();
    referenced_providers.insert("gfwlist".to_string());
    for rule in &final_rules {
        if let Some(s) = rule.as_str() {
            let mut parts = s.split(',');
            if let (Some(kind), Some(name)) = (parts.next(), parts.next())
                && kind.trim().eq_ignore_ascii_case("RULE-SET")
            {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    referenced_providers.insert(trimmed.to_string());
                }
            }
        }
    }

    let mut rule_providers = Mapping::new();
    for (k, v) in raw_rule_providers {
        if let Some(name) = k.as_str()
            && referenced_providers.contains(name)
        {
            rule_providers.insert(k, v);
        }
    }
    rule_providers.insert(Value::from("gfwlist"), Value::from(gfwlist_provider));
    config.insert(Value::from("rule-providers"), Value::from(rule_providers));

    config.insert(Value::from("rules"), Value::from(final_rules));

    config
}

fn profile_timestamp_suffix(updated: Option<i64>) -> std::string::String {
    use chrono::{Local, TimeZone as _};
    let ts = updated.unwrap_or(0);
    if let Some(dt) = Local.timestamp_opt(ts, 0).single() {
        dt.format("%H%M%S").to_string()
    } else {
        "000000".to_string()
    }
}

pub(super) struct MergedProfileNodes {
    pub proxies: Vec<Value>,
    pub proxy_providers: Mapping,
}

type PerItemNameMap = std::collections::HashMap<(usize, std::string::String), std::string::String>;
type GlobalNameMap = std::collections::HashMap<std::string::String, std::string::String>;

fn merge_proxies_with_maps(
    profiles: &crate::config::profiles::IProfiles,
) -> (Vec<Value>, PerItemNameMap, GlobalNameMap) {
    let mut raw_proxies = Vec::new();

    if let Some(items) = profiles.get_items() {
        for (item_idx, item) in items.iter().enumerate() {
            if let Some(itype) = &item.itype
                && (itype == "remote" || itype == "local")
                && let Some(file) = &item.file
                && let Ok(file_path) = dirs::app_profiles_dir().map(|d| d.join(file.as_str()))
                && let Ok(mut mapping) = crate::utils::help::read_mapping(&file_path)
                && let Some(Value::Sequence(proxies)) = mapping.remove("proxies")
            {
                let suffix = profile_timestamp_suffix(item.updated);
                for proxy in proxies {
                    let original_name = proxy
                        .as_mapping()
                        .and_then(|map| map.get("name").and_then(Value::as_str))
                        .unwrap_or("Proxy")
                        .to_string();
                    raw_proxies.push((proxy, item_idx, original_name, suffix.clone()));
                }
            }
        }
    }

    let mut name_suffix_counts = std::collections::HashMap::new();
    let mut global_name_counts = std::collections::HashMap::new();
    for (_, _, name, suffix) in &raw_proxies {
        *name_suffix_counts
            .entry((name.clone(), suffix.clone()))
            .or_insert(0usize) += 1;
        *global_name_counts.entry(name.clone()).or_insert(0usize) += 1;
    }

    let mut name_suffix_counters = std::collections::HashMap::new();
    let mut per_item_name_map = PerItemNameMap::new();
    let mut global_name_map = GlobalNameMap::new();
    let mut renamed_entries = Vec::with_capacity(raw_proxies.len());

    for (proxy, item_idx, name, suffix) in raw_proxies {
        let key = (name.clone(), suffix.clone());
        let count = name_suffix_counts.get(&key).copied().unwrap_or(0);
        let new_name = if count > 1 {
            let idx = name_suffix_counters.entry(key).or_insert(0usize);
            *idx += 1;
            format!("{name} #{idx} ({suffix})")
        } else {
            format!("{name} ({suffix})")
        };
        per_item_name_map
            .entry((item_idx, name.clone()))
            .or_insert_with(|| new_name.clone());
        if global_name_counts.get(&name).copied().unwrap_or(0) == 1 {
            global_name_map.insert(name, new_name.clone());
        }
        renamed_entries.push((proxy, item_idx, new_name));
    }

    let mut all_proxies = Vec::with_capacity(renamed_entries.len());
    for (mut proxy, item_idx, new_name) in renamed_entries {
        if let Some(map) = proxy.as_mapping_mut() {
            map.insert(Value::from("name"), Value::from(new_name));
            if let Some(dp) = map
                .get("dialer-proxy")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                && !matches!(dp, "DIRECT" | "REJECT" | "PROXY")
                && let Some(renamed_dp) = per_item_name_map
                    .get(&(item_idx, dp.to_string()))
                    .or_else(|| global_name_map.get(dp))
            {
                map.insert(Value::from("dialer-proxy"), Value::from(renamed_dp.as_str()));
            }
        }
        all_proxies.push(proxy);
    }

    (all_proxies, per_item_name_map, global_name_map)
}

fn alloc_provider_key(merged: &Mapping, k: &Value, suffix: &str) -> Value {
    if !merged.contains_key(k) {
        return k.clone();
    }
    let base = k.as_str().unwrap_or("provider");
    let mut candidate = format!("{base}_{suffix}");
    let mut idx = 2usize;
    while merged.contains_key(Value::from(candidate.as_str())) {
        candidate = format!("{base}_{suffix}_{idx}");
        idx += 1;
    }
    Value::from(candidate)
}

fn merge_proxy_providers_with_maps(
    profiles: &crate::config::profiles::IProfiles,
    per_item_name_map: &PerItemNameMap,
    global_name_map: &GlobalNameMap,
) -> Mapping {
    let mut merged = Mapping::new();
    let Some(items) = profiles.get_items() else {
        return merged;
    };
    let current_uid = profiles.get_current();

    let ordered_items = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.uid.as_ref() == current_uid)
        .chain(
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.uid.as_ref() != current_uid),
        );

    let mut used_paths: HashSet<std::string::String> = HashSet::new();

    for (item_idx, item) in ordered_items {
        let Some(itype) = &item.itype else {
            continue;
        };
        if itype != "remote" && itype != "local" {
            continue;
        }
        let Some(file) = &item.file else {
            continue;
        };
        let Ok(file_path) = dirs::app_profiles_dir().map(|d| d.join(file.as_str())) else {
            continue;
        };
        let Ok(mut mapping) = crate::utils::help::read_mapping(&file_path) else {
            continue;
        };
        let Some(Value::Mapping(providers)) = mapping.remove("proxy-providers") else {
            continue;
        };

        let suffix = profile_timestamp_suffix(item.updated);
        for (k, mut v) in providers {
            let final_key = alloc_provider_key(&merged, &k, &suffix);
            let final_key_str = final_key.as_str().unwrap_or("provider");

            if let Some(provider_map) = v.as_mapping_mut() {
                if let Some(proxy_target) = provider_map.get("proxy").and_then(Value::as_str).map(str::trim)
                    && !proxy_target.is_empty()
                    && !matches!(proxy_target, "DIRECT" | "REJECT" | "PROXY")
                    && let Some(renamed_proxy) = per_item_name_map
                        .get(&(item_idx, proxy_target.to_string()))
                        .or_else(|| global_name_map.get(proxy_target))
                {
                    provider_map.insert(Value::from("proxy"), Value::from(renamed_proxy.as_str()));
                }

                let existing_path = provider_map
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                if let Some(p) = existing_path {
                    let norm = p.to_ascii_lowercase();
                    if !used_paths.insert(norm) {
                        let isolated_path = format!("./proxy_providers/{final_key_str}.yaml");
                        used_paths.insert(isolated_path.to_ascii_lowercase());
                        provider_map.insert(Value::from("path"), Value::from(isolated_path));
                    }
                }
            }

            merged.insert(final_key, v);
        }
    }

    merged
}

/// 跨订阅聚合全部 `remote` / `local` 配置中的节点与 `proxy-providers`，
/// 并同步改写链式 `dialer-proxy`、`proxy-providers.*.proxy` 引用与重名缓存 `path`。
pub(super) fn merge_profiles_proxies_and_providers(
    profiles: &crate::config::profiles::IProfiles,
) -> MergedProfileNodes {
    let (proxies, per_item_name_map, global_name_map) = merge_proxies_with_maps(profiles);
    let proxy_providers = merge_proxy_providers_with_maps(profiles, &per_item_name_map, &global_name_map);
    MergedProfileNodes {
        proxies,
        proxy_providers,
    }
}

/// 读取当前激活 Profile 的持久化 PROXY 选点与过滤词（`proxy_head_state.json`）。
pub async fn read_persisted_proxy_and_filter() -> (Option<std::string::String>, std::string::String) {
    let profiles = Config::profiles().await.latest_arc();
    let Some(current_uid) = profiles.current.as_deref() else {
        return (None, std::string::String::new());
    };
    let persisted = profiles
        .items
        .as_ref()
        .and_then(|items| items.iter().find(|it| it.uid.as_deref() == Some(current_uid)))
        .and_then(|item| item.selected.as_ref())
        .and_then(|sel| sel.iter().find(|s| s.name.as_deref() == Some("PROXY")))
        .and_then(|s| s.now.as_deref())
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned);
    let filter_text = crate::module::monitor::get_filter_and_sort_config(current_uid)
        .await
        .0
        .filter_text
        .trim()
        .to_string();
    (persisted, filter_text)
}

/// 纯函数：强制运行时关闭 `profile.store-selected`，并按过滤条件将合规节点提前至 `PROXY.proxies[0]`。
pub(super) fn apply_proxy_group_head_and_store_selected(
    config: &mut Mapping,
    preferred: Option<&str>,
    filter_text: &str,
) {
    match config.get_mut("profile") {
        Some(Value::Mapping(profile_map)) => {
            profile_map.insert(Value::from("store-selected"), Value::Bool(false));
        }
        _ => {
            let mut profile_map = Mapping::new();
            profile_map.insert(Value::from("store-selected"), Value::Bool(false));
            config.insert(Value::from("profile"), Value::Mapping(profile_map));
        }
    }

    let Some(Value::Sequence(groups)) = config.get_mut("proxy-groups") else {
        return;
    };
    for group in groups.iter_mut() {
        let Some(group_map) = group.as_mapping_mut() else {
            continue;
        };
        if group_map.get("name").and_then(Value::as_str) != Some("PROXY") {
            continue;
        }
        let Some(Value::Sequence(proxies_val)) = group_map.get("proxies") else {
            break;
        };
        let mut names: Vec<std::string::String> = proxies_val
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
        let promoted = crate::utils::node::promote_preferred_proxy_first(
            &mut names,
            preferred,
            filter_text,
            |n| {
                n != "DIRECT"
                    && n != "REJECT"
                    && !crate::utils::node::is_dummy_node(n)
                    && !crate::utils::node::is_confirmed_fake_node(n)
            },
            crate::utils::node::is_verified_true_node,
        );
        if promoted.is_some() {
            group_map.insert(
                Value::from("proxies"),
                Value::Sequence(names.into_iter().map(Value::from).collect()),
            );
        }
        break;
    }
}

/// 同步运行时配置 `PROXY.proxies[0]` 与 `profile.store-selected: false`（BUG-405）。
pub async fn sync_runtime_proxy_group_head(config: &mut Mapping) {
    let (preferred, filter_text) = read_persisted_proxy_and_filter().await;
    apply_proxy_group_head_and_store_selected(config, preferred.as_deref(), &filter_text);
}
