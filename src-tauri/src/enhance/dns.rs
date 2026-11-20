use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Value};
use std::collections::HashSet;

pub(super) fn apply_mandatory_dns_settings(mut config: Mapping) -> Mapping {
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
    // 注：此处**不**补 dns.ipv6 —— 该字段由 `enhance::tun::use_tun` 作为唯一真源同步
    // （TUN 与非 TUN 两分支均无条件写入，值恒为用户「Allow IPv6」开关）。
    // 若在这里读顶层 ipv6 兜底，TUN 模式下读到的是 use_tun 刚强制过的 `true`（IPv6 防泄漏
    // 接管所需），会把用户关掉的 IPv6 静默写回开启 —— 这是靠「另一分支必然已写入」侥幸不出错的
    // 脆弱耦合，故直接移除。
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
                Value::Sequence(seq) if !seq.contains(&fallback) => {
                    seq.push(fallback);
                    logging!(
                        info,
                        Type::Core,
                        "DNS {key} 追加保底明文上游 223.5.5.5（冷启动防鸡生蛋）"
                    );
                }
                Value::String(s) if s != "223.5.5.5" => {
                    *val = Value::Sequence(vec![Value::String(s.clone()), fallback]);
                    logging!(
                        info,
                        Type::Core,
                        "DNS {key} 标量提升为列表并追加保底明文上游 223.5.5.5（冷启动防鸡生蛋）"
                    );
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

/// 将 `dns.nameserver` 中的境外公共 DNS 明文 UDP 上游（Google / Cloudflare）升级为 DoH，
/// 避免 `respect-rules: true` 下经由 TCP 承载节点发起 UDP 查询时并发竞速超时取消。
/// `default-nameserver`（需明文引导）、境内明文 DNS 与已加密上游保持不变。
struct ParsedDnsUpstream {
    host: std::string::String,
    is_plain: bool,
    force_direct: bool,
    is_special: bool,
    clean_upstream: std::string::String,
}

fn parse_dns_upstream(raw: &str) -> ParsedDnsUpstream {
    let trimmed = raw.trim();
    let (without_frag, frag_opt) = match trimmed.split_once('#') {
        Some((before, frag)) => (before.trim(), Some(frag.trim())),
        None => (trimmed, None),
    };
    let mut force_direct = false;
    let mut kept_frags: Vec<&str> = Vec::new();
    if let Some(frag) = frag_opt {
        for part in frag.split('&').map(str::trim).filter(|p| !p.is_empty()) {
            // C12：mihomo 原生并不识别 `#DIRECT` / `#PROXY` 政策后缀；
            // `#DIRECT` 在直连不可达语义下供清理器降级使用，且与 `#PROXY` 一并剥离，
            // 仅保留 `#h3=true` 等合法 mihomo 参数。
            if part.eq_ignore_ascii_case("direct") {
                force_direct = true;
            } else if !part.eq_ignore_ascii_case("proxy") {
                kept_frags.push(part);
            }
        }
    }
    let clean_upstream = if kept_frags.is_empty() {
        without_frag.to_string()
    } else {
        format!("{without_frag}#{}", kept_frags.join("&"))
    };

    let (scheme, rest) = match without_frag.split_once("://") {
        Some((s, r)) => (s.to_ascii_lowercase(), r),
        None => (std::string::String::new(), without_frag),
    };
    let is_special = scheme == "dhcp" || scheme == "rcode" || rest.eq_ignore_ascii_case("system");
    let is_plain = scheme.is_empty() || scheme == "udp" || scheme == "tcp";
    let host = extract_upstream_host(without_frag)
        .unwrap_or_default()
        .to_ascii_lowercase();

    ParsedDnsUpstream {
        host,
        is_plain,
        force_direct,
        is_special,
        clean_upstream,
    }
}

const fn is_shared_cgnat_v4(ip: std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (octets[1] & 0b1100_0000) == 64
}

const fn is_ula_or_link_local_v6(ip: std::net::Ipv6Addr) -> bool {
    let seg0 = ip.segments()[0];
    (seg0 & 0xfe00) == 0xfc00 || (seg0 & 0xffc0) == 0xfe80
}

fn is_local_or_domestic_dns_host(host: &str) -> bool {
    const CN_DNS_IPV4: &[&str] = &[
        "223.5.5.5",
        "223.6.6.6",
        "119.29.29.29",
        "119.28.28.28",
        "114.114.114.114",
        "114.114.115.115",
        "180.76.76.76",
        "180.184.1.1",
        "180.184.2.2",
        "1.2.4.8",
        "210.2.4.8",
    ];
    const CN_DNS_IPV6: &[&str] = &["2400:3200::1", "2400:3200:baba::1", "2402:4e00::", "2402:4e00:1::"];
    const CN_OR_LOCAL_DOMAINS: &[&str] = &[
        "doh.pub",
        "dot.pub",
        "dns.pub",
        "alidns.com",
        "dns.alidns.com",
        "doh.360.cn",
        "dot.360.cn",
        "360.cn",
        "dnspod.cn",
        "dns.dnspod.cn",
        "bytedns.com",
        "cn",
        "lan",
        "local",
        "internal",
        "home",
        "home.arpa",
        "localhost",
        "corp",
        "intranet",
    ];

    if host.is_empty() {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        return ip.is_private()
            || ip.is_loopback()
            || ip.is_link_local()
            || is_shared_cgnat_v4(ip)
            || CN_DNS_IPV4.contains(&host);
    }
    if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
        return ip.is_loopback() || ip.is_unspecified() || is_ula_or_link_local_v6(ip) || CN_DNS_IPV6.contains(&host);
    }
    if !host.contains('.') {
        return true;
    }
    CN_OR_LOCAL_DOMAINS.iter().any(|suffix| {
        host == *suffix
            || (host.len() > suffix.len()
                && host.ends_with(suffix)
                && host.as_bytes()[host.len() - suffix.len() - 1] == b'.')
    })
}

pub(super) fn collect_dns_field_strings(dns: &Mapping, field: &str) -> Vec<std::string::String> {
    let Some(val) = dns.get(field) else {
        return Vec::new();
    };
    match val {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(seq) => seq.iter().filter_map(Value::as_str).map(ToString::to_string).collect(),
        _ => Vec::new(),
    }
}

fn derive_domestic_doh_upstream(dns: &Mapping) -> std::string::String {
    for raw in collect_dns_field_strings(dns, "nameserver") {
        let parsed = parse_dns_upstream(&raw);
        if !parsed.is_special
            && !parsed.is_plain
            && !parsed.clean_upstream.is_empty()
            && is_local_or_domestic_dns_host(&parsed.host)
        {
            // 降级保通模板必须去掉 fragment（如 `#h3=true`）：fragment 只用于「原样保留
            // 订阅条目」，而 domestic_fallback 是派生的通用境内 DoH，携带他家/他字段的
            // 参数会导致降级后仍解析失败（D-1）。
            return std::string::String::from(
                parsed
                    .clean_upstream
                    .split_once('#')
                    .map_or(parsed.clean_upstream.as_str(), |(base, _)| base),
            );
        }
    }
    "https://doh.pub/dns-query".to_string()
}

fn foreign_plain_doh_url(host: &str) -> Option<&'static str> {
    match host {
        "8.8.8.8" => Some("https://8.8.8.8/dns-query"),
        "8.8.4.4" => Some("https://8.8.4.4/dns-query"),
        "1.1.1.1" => Some("https://1.1.1.1/dns-query"),
        "1.0.0.1" => Some("https://1.0.0.1/dns-query"),
        "9.9.9.9" => Some("https://9.9.9.9/dns-query"),
        "149.112.112.112" => Some("https://149.112.112.112/dns-query"),
        "208.67.222.222" | "208.67.220.220" => Some("https://doh.opendns.com/dns-query"),
        _ => None,
    }
}

fn dual_secondary_v4(ip: &str) -> Option<&'static str> {
    match ip {
        "8.8.8.8" => Some("8.8.4.4"),
        "8.8.4.4" => Some("8.8.8.8"),
        "1.1.1.1" => Some("1.0.0.1"),
        "1.0.0.1" => Some("1.1.1.1"),
        "9.9.9.9" => Some("149.112.112.112"),
        "149.112.112.112" => Some("9.9.9.9"),
        "208.67.222.222" => Some("208.67.220.220"),
        "208.67.220.220" => Some("208.67.222.222"),
        _ => None,
    }
}

fn dual_secondary_v6(ip: &str) -> Option<&'static str> {
    match ip {
        "2001:4860:4860::8888" => Some("2001:4860:4860::8844"),
        "2001:4860:4860::8844" => Some("2001:4860:4860::8888"),
        "2606:4700:4700::1111" => Some("2606:4700:4700::1001"),
        "2606:4700:4700::1001" => Some("2606:4700:4700::1111"),
        "2620:fe::fe" => Some("2620:fe::9"),
        "2620:fe::9" => Some("2620:fe::fe"),
        "2620:119:35::35" => Some("2620:119:53::53"),
        "2620:119:53::53" => Some("2620:119:35::35"),
        _ => None,
    }
}

fn known_domain_landing_v4_ips(domain: &str) -> &'static [&'static str] {
    match domain {
        "dns.google" | "dns.google.com" | "8888.google" => &["8.8.8.8", "8.8.4.4"],
        "cloudflare-dns.com" | "dns.cloudflare.com" => &["1.1.1.1", "1.0.0.1"],
        "dns.quad9.net" => &["9.9.9.9", "149.112.112.112"],
        "doh.opendns.com" => &["208.67.222.222", "208.67.220.220"],
        _ => &[],
    }
}

/// 统一从一组 DNS 上游主机生成前置 `PROXY` 保通规则（A9：`dns.fallback` 与 `nameserver-policy` 共用真源）。
///
/// - 私网/环回/链路本地/境内 DNS IP 自动跳过；
/// - IP 字面量补全双活辅 IP（Google / Cloudflare / Quad9 / OpenDNS 双向对偶）；
/// - 域名型上游生成 `DOMAIN,<host>,PROXY`，并对已知域名补齐固定落地 IPv4 规则。
pub(super) fn dns_upstream_proxy_rules_from_hosts<'a, I>(hosts: I) -> Vec<std::string::String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut rules = Vec::new();
    let mut seen = HashSet::new();

    for raw_host in hosts {
        let host = raw_host.trim().to_ascii_lowercase();
        if host.is_empty() || is_local_or_domestic_dns_host(&host) {
            continue;
        }
        let mut candidates: Vec<std::string::String> = Vec::new();

        if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
            if ip.is_private() || ip.is_loopback() || ip.is_link_local() || is_shared_cgnat_v4(ip) {
                continue;
            }
            candidates.push(format!("IP-CIDR,{host}/32,PROXY,no-resolve"));
            if let Some(peer) = dual_secondary_v4(&host) {
                candidates.push(format!("IP-CIDR,{peer}/32,PROXY,no-resolve"));
            }
        } else if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
            if ip.is_loopback() || ip.is_unspecified() || is_ula_or_link_local_v6(ip) {
                continue;
            }
            candidates.push(format!("IP-CIDR6,{host}/128,PROXY,no-resolve"));
            if let Some(peer) = dual_secondary_v6(&host) {
                candidates.push(format!("IP-CIDR6,{peer}/128,PROXY,no-resolve"));
            }
        } else {
            candidates.push(format!("DOMAIN,{host},PROXY"));
            for landing_ip in known_domain_landing_v4_ips(&host) {
                candidates.push(format!("IP-CIDR,{landing_ip}/32,PROXY,no-resolve"));
            }
        }

        for rule in candidates {
            if seen.insert(rule.clone()) {
                rules.push(rule);
            }
        }
    }

    rules
}

fn find_dns_proxy_rule_insert_pos(rules: &[Value]) -> usize {
    if let Some(gfw_idx) = rules.iter().position(|r| r.as_str() == Some("RULE-SET,gfwlist,PROXY")) {
        let mut pos = gfw_idx + 1;
        while let Some(next_str) = rules.get(pos).and_then(Value::as_str) {
            if next_str.ends_with(",PROXY") || next_str.ends_with(",PROXY,no-resolve") {
                pos += 1;
            } else {
                break;
            }
        }
        return pos;
    }

    rules
        .iter()
        .position(|r| {
            r.as_str().is_some_and(|s| {
                s.starts_with("IP-CIDR,127.0.0.0/8,")
                    || s.starts_with("GEOIP,")
                    || s.starts_with("IP-CIDR6,::/0,")
                    || s.starts_with("MATCH,")
            })
        })
        .unwrap_or(rules.len())
}

/// 机场订阅 DNS 统一卫生治理（BUG-308 / BUG-364 / A9 / C10-C12 / §3.7）：
/// - 统一覆盖 `dns.nameserver`、`dns.fallback` 与 `dns.nameserver-policy`（绝不触碰
///   必须保持明文的 `default-nameserver`，也不删除任何 `nameserver-policy` 策略键）；
/// - 仅以 `is_local_or_domestic_dns_host` 判定是否境内/内网可达（不再因条目出现在
///   `fallback` 而误判，也不为境内 DNS 生成 `PROXY` 规则）；
/// - 剥离 Mihomo 不识别的 `#DIRECT` / `#PROXY` 伪后缀，保留 `#h3=true` 等原生参数；
/// - `respect-rules` 未开启（或携带 `#DIRECT`）时，将不可达海外上游就地改写为由 `nameserver`
///   推导的可达境内 DoH（兜底 `https://doh.pub/dns-query`）并去重；
/// - `respect-rules: true` 时，将境外明文 UDP 公共 DNS 升级为 DoH（消除 TCP 承载 UDP
///   竞速取消），并与 `dns_fallback_proxy_rules` 共用真源向 `rules` 补齐 `PROXY` 规则。
#[derive(Default)]
struct SanitizeDnsCounters {
    rewritten_foreign: usize,
    rewritten_plain: usize,
}

fn push_unique_string(target: &mut Vec<std::string::String>, value: std::string::String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn sanitize_upstream_items<'a, I>(
    raw_items: I,
    respect_rules: bool,
    domestic_fallback: &str,
    proxy_hosts: &mut Vec<std::string::String>,
    counters: &mut SanitizeDnsCounters,
) -> Vec<std::string::String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut cleaned: Vec<std::string::String> = Vec::new();
    for raw in raw_items {
        let parsed = parse_dns_upstream(raw);
        if parsed.clean_upstream.is_empty() {
            continue;
        }
        if parsed.is_special || is_local_or_domestic_dns_host(&parsed.host) {
            push_unique_string(&mut cleaned, parsed.clean_upstream);
            continue;
        }

        if parsed.force_direct || !respect_rules {
            counters.rewritten_foreign += 1;
            push_unique_string(&mut cleaned, domestic_fallback.to_string());
            continue;
        }

        let target_upstream = if parsed.is_plain {
            if let Some(doh) = foreign_plain_doh_url(&parsed.host) {
                counters.rewritten_plain += 1;
                doh.to_string()
            } else {
                parsed.clean_upstream
            }
        } else {
            parsed.clean_upstream
        };

        let target_host = extract_upstream_host(&target_upstream)
            .unwrap_or(parsed.host)
            .to_ascii_lowercase();
        if !target_host.is_empty() {
            push_unique_string(proxy_hosts, target_host);
        }
        push_unique_string(&mut cleaned, target_upstream);
    }
    cleaned
}

fn sanitize_dns_list_field(
    dns: &mut Mapping,
    field: &str,
    respect_rules: bool,
    domestic_fallback: &str,
    proxy_hosts: &mut Vec<std::string::String>,
    counters: &mut SanitizeDnsCounters,
) {
    let Some(val) = dns.get_mut(field) else {
        return;
    };
    let (was_scalar, raw_items): (bool, Vec<std::string::String>) = match val {
        Value::String(s) => (true, vec![s.clone()]),
        Value::Sequence(seq) => (
            false,
            seq.iter().filter_map(Value::as_str).map(ToString::to_string).collect(),
        ),
        _ => return,
    };
    if raw_items.is_empty() {
        return;
    }

    let cleaned = sanitize_upstream_items(
        raw_items.iter().map(std::string::String::as_str),
        respect_rules,
        domestic_fallback,
        proxy_hosts,
        counters,
    );

    if cleaned.is_empty() {
        *val = if was_scalar {
            Value::String(domestic_fallback.to_string())
        } else {
            Value::Sequence(vec![Value::String(domestic_fallback.to_string())])
        };
    } else if was_scalar && cleaned.len() == 1 {
        if let Some(single) = cleaned.into_iter().next() {
            *val = Value::String(single);
        }
    } else {
        *val = Value::Sequence(cleaned.into_iter().map(Value::String).collect());
    }
}

fn sanitize_nameserver_policy_map(
    dns: &mut Mapping,
    respect_rules: bool,
    domestic_fallback: &str,
    proxy_hosts: &mut Vec<std::string::String>,
    counters: &mut SanitizeDnsCounters,
) {
    let Some(Value::Mapping(policy)) = dns.get_mut("nameserver-policy") else {
        return;
    };
    let mut next_policy = Mapping::with_capacity(policy.len());

    for (key, val) in policy.iter() {
        let (was_scalar, raw_items): (bool, Vec<&str>) = match val {
            Value::String(s) => (true, vec![s.as_str()]),
            Value::Sequence(seq) => (false, seq.iter().filter_map(Value::as_str).collect()),
            other => {
                next_policy.insert(key.clone(), other.clone());
                continue;
            }
        };

        let cleaned = sanitize_upstream_items(raw_items, respect_rules, domestic_fallback, proxy_hosts, counters);

        if cleaned.is_empty() {
            next_policy.insert(key.clone(), Value::String(domestic_fallback.to_string()));
        } else if was_scalar && cleaned.len() == 1 {
            if let Some(single) = cleaned.into_iter().next() {
                next_policy.insert(key.clone(), Value::String(single));
            }
        } else {
            next_policy.insert(
                key.clone(),
                Value::Sequence(cleaned.into_iter().map(Value::String).collect()),
            );
        }
    }

    *policy = next_policy;
}

fn inject_dns_proxy_rules(config: &mut Mapping, proxy_hosts: &[std::string::String]) {
    if proxy_hosts.is_empty() {
        return;
    }
    let generated = dns_upstream_proxy_rules_from_hosts(proxy_hosts.iter().map(std::string::String::as_str));
    let existing_rules = config.get("rules").and_then(Value::as_sequence);
    let to_insert: Vec<Value> = generated
        .into_iter()
        .filter(|rule| {
            !existing_rules
                .map(|seq| seq.iter().any(|r| r.as_str() == Some(rule.as_str())))
                .unwrap_or(false)
        })
        .map(Value::String)
        .collect();
    if !to_insert.is_empty()
        && let Some(Value::Sequence(rules)) = config.get_mut("rules")
    {
        let insert_pos = find_dns_proxy_rule_insert_pos(rules);
        rules.splice(insert_pos..insert_pos, to_insert);
    }
}

pub(super) fn sanitize_dns_upstreams(mut config: Mapping) -> Mapping {
    let mut proxy_hosts: Vec<std::string::String> = Vec::new();
    let mut counters = SanitizeDnsCounters::default();

    if let Some(Value::Mapping(dns)) = config.get_mut("dns") {
        let respect_rules = dns.get("respect-rules").and_then(Value::as_bool).unwrap_or(false);
        let domestic_fallback = derive_domestic_doh_upstream(dns);

        for field in ["nameserver", "fallback"] {
            sanitize_dns_list_field(
                dns,
                field,
                respect_rules,
                &domestic_fallback,
                &mut proxy_hosts,
                &mut counters,
            );
        }
        sanitize_nameserver_policy_map(dns, respect_rules, &domestic_fallback, &mut proxy_hosts, &mut counters);
    }

    inject_dns_proxy_rules(&mut config, &proxy_hosts);

    if counters.rewritten_foreign > 0 || counters.rewritten_plain > 0 {
        logging!(
            info,
            Type::Config,
            "DNS 上游完成统一卫生处理：海外不可达改写境内 DoH {} 条，明文 UDP 升级 DoH {} 条",
            counters.rewritten_foreign,
            counters.rewritten_plain
        );
    }
    config
}

/// 从 DNS 上游字符串中取出主机名部分。
///
/// 支持 mihomo 的全部常见写法：`1.1.1.1`、`8.8.8.8:53`、
/// `https://1.1.1.1/dns-query`、`tls://8.8.8.8`、
/// `https://[2606:4700:4700::1111]/dns-query#h3=true`。
pub(super) fn extract_upstream_host(raw: &str) -> Option<std::string::String> {
    let s = raw.trim();
    // 去掉 `#h3=true` 之类的策略参数
    let s = s.split('#').next()?;
    // 去掉 scheme
    let s = match s.find("://") {
        Some(idx) => s.get(idx + 3..)?,
        None => s,
    };
    // 去掉 path 与 query
    let host_port = s.split(['/', '?']).next()?;

    let host = if let Some(rest) = host_port.strip_prefix('[') {
        // `[IPv6]:port`
        rest.split(']').next()?
    } else if host_port.matches(':').count() == 1 {
        // `host:port`；单冒号一定不是 IPv6 字面量
        host_port.split(':').next()?
    } else {
        // 裸 IPv6 或纯主机名
        host_port
    };

    let host = host.trim();
    (!host.is_empty()).then(|| std::string::String::from(host))
}

// 原由 `dns.fallback` 独立生成强制走代理规则（已重构并由下方注释说明单一路径替代）：
// `dns_fallback_proxy_rules` 已删除（B-1/F-2）：PROXY 规则统一由
// `sanitize_dns_upstreams` → `inject_dns_proxy_rules` 单一路径生成，
// 覆盖 fallback + nameserver + nameserver-policy 三域，不再有第二注入点。
