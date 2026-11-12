use serde_yaml_ng::{Mapping, Value};

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

// if key not exists then append value
#[allow(unused_macros)]
macro_rules! append {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        if !$map.contains_key(&ret_key) {
            $map.insert(ret_key, Value::from($val));
        }
    };
}

pub fn use_tun(mut config: Mapping, enable: bool) -> Mapping {
    let tun_key = Value::from("tun");
    let tun_val = config.get(&tun_key);
    let mut tun_val = tun_val.map_or_else(Mapping::new, |val| {
        val.as_mapping().cloned().unwrap_or_else(Mapping::new)
    });

    // 用户「Allow IPv6」开关值（顶层 ipv6）。
    // 必须在下方 TUN 分支强制 ipv6=true 之前读取，否则会被覆盖成 true。
    // 【两种模式共用】该开关统一同步到 dns.ipv6，保证「是否使用 IPv6」的语义
    // 在 TUN / 系统代理 / 手动三种模式下完全一致。
    let user_ipv6 = config
        .get(Value::from("ipv6"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if enable {
        // 【上游语义】TUN 开启时接管 DNS：仅当 enhanced-mode 为 fake-ip（或未设置）时，
        // 确保 enable=true 及 ipv6 同步，并补齐缺失的 enhanced-mode=fake-ip、fake-ip-range、
        // 默认 fake-ip-filter 等键，绝不整体覆盖订阅或用户已提供的 nameserver 等自定义字段。
        // fake-ip 模式下客户端立即拿到假 IP，内核按「域名→规则→转发」处理，
        // 不依赖 redir-host 那样把域名实时解析成真实 IP，TUN 下更健壮。
        let dns_key = Value::from("dns");
        let dns_val = config.get(&dns_key);
        let mut dns_val = dns_val.map_or_else(Mapping::new, |val| {
            val.as_mapping().cloned().unwrap_or_else(Mapping::new)
        });
        let current_mode = dns_val
            .get(Value::from("enhanced-mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("fake-ip");

        if current_mode == "fake-ip" || !dns_val.contains_key(Value::from("enhanced-mode")) {
            revise!(dns_val, "enable", true);
            if !dns_val.contains_key(Value::from("enhanced-mode")) {
                revise!(dns_val, "enhanced-mode", "fake-ip");
            }
            if !dns_val.contains_key(Value::from("fake-ip-range")) {
                revise!(dns_val, "fake-ip-range", "198.18.0.1/16");
            }
            if user_ipv6 && !dns_val.contains_key(Value::from("fake-ip-range6")) {
                // 【约束·勿改】该值必须落在 `fc00::/7` 之外！
                // enhance::enforce_mini_agreements 会注入 `IP-CIDR6,fc00::/7,DIRECT,no-resolve`
                // 用于让 ULA/局域网 IPv6 直连；若把 fake-ip-range6 改成社区常见的
                // `fdfe:dcba:9877::/64`（属 ULA），fake IPv6 就会撞上该直连规则。
                // 当前取值 2001:2::/64（RFC 5180 基准测试段）不在 fc00::/7 内，安全。
                revise!(dns_val, "fake-ip-range6", "2001:2::0/64");
            }
            // fake-ip-filter：按条目 merge，而非「键缺失才整体写入」。
            // 真实订阅几乎都会自带该键，若整体跳过，NCSI/NTP 兜底条目会一条都进不去。
            if !dns_val.contains_key(Value::from("fake-ip-filter")) {
                let filters: Vec<Value> = crate::constants::tun::DEFAULT_FAKE_IP_FILTER
                    .iter()
                    .map(|&s| Value::String(s.into()))
                    .collect();
                dns_val.insert(Value::from("fake-ip-filter"), Value::Sequence(filters));
            } else if let Some(Value::Sequence(seq)) = dns_val.get_mut("fake-ip-filter") {
                // 仅补缺条目，保留订阅已有条目与顺序；非列表类型保持原样不改写
                for &item in crate::constants::tun::DEFAULT_FAKE_IP_FILTER {
                    let entry = Value::String(item.into());
                    if !seq.contains(&entry) {
                        seq.push(entry);
                    }
                }
            }
        }
        // dns.ipv6 跟随「Allow IPv6」开关，对所有 enhanced-mode（fake-ip / redir-host /
        // 未设置）一律生效，保证 TUN / 系统代理 / 手动三模式下「开关即是否使用 IPv6」语义一致。
        // 放在 fake-ip 补齐块之外：redir-host 不注入 TUN 专属字段，但开关同步不能缺席。
        revise!(dns_val, "ipv6", user_ipv6);
        revise!(config, "dns", dns_val);

        // 【IPv6 防泄漏】TUN 启用时，必须让内核具备 IPv6 收包能力，否则 IPv6 会整体绕过代理。
        //
        // 问题：mihomo 官方文档明确 `tun.inet6-address`「需要同时将顶层配置中的 ipv6 设置为 true
        // 才会生效」。顶层 ipv6=false 时 TUN 不具备 IPv6 能力 → 不把 IPv6 默认路由导入 TUN，
        // 系统 ::/0 仍指向物理网关 → 所有 IPv6 流量从物理网卡直连泄漏（实测到 Google 的 IPv6
        // 请求立即失败，且完全不经过代理）。这是「内核禁 IPv6 + 系统开 IPv6」的最差组合。
        //
        // 修复：TUN 启用时强制顶层 ipv6=true，并显式指定 TUN 的 IPv6 地址，使 TUN 同时接管
        // IPv4 与 IPv6 默认路由，IPv6 流量进入内核按规则处理，不再泄漏。
        //
        // 注意：用户「Allow IPv6」开关的语义仍然保留 —— 它决定 `dns.ipv6`（见上方 revise!），
        // 即关闭时内核 DNS 不返回 AAAA 记录，应用拿不到 IPv6 地址、实际走 IPv4。
        // 二者不冲突：顶层 ipv6 管「TUN 是否接管 IPv6 路由（防泄漏）」，dns.ipv6 管「是否使用 IPv6」。
        revise!(config, "ipv6", true);
        // 【格式修正】内核要求 inet6-address 为前缀数组（[]netip.Prefix），
        // 写单个字符串会被拒绝：yaml unmarshal 报错、配置验证失败、内核无法启动。
        tun_val.insert(
            Value::from("inet6-address"),
            Value::Sequence(vec![Value::String(crate::constants::tun::TUN_INET6_ADDRESS.into())]),
        );

        // TUN 模式下默认开启并补全 sniffer 域名嗅探：
        // 避免常驻后台进程（如 Edge）持有切换前直连/Router IPv6 DNS 缓存的真实/投毒 IP
        // 时因无域名而绕过 `RULE-SET,gfwlist,PROXY` 落入 `MATCH,DIRECT`。
        merge_tun_sniffer_defaults(&mut config);
    } else {
        // 非 TUN（系统代理 / 手动）：不注入 fake-ip 等 TUN 专属 DNS 字段，交由内核默认
        // （redir-host）；也不做 IPv6 路由接管 —— 非全局接管场景本就不存在「绕过代理泄漏」。
        //
        // dns.ipv6 一律跟随「Allow IPv6」开关（`user_ipv6`），**无论订阅是否自带 dns 段**：
        // 否则订阅若自带 `dns.ipv6: true`，用户关闭开关后 DNS 仍会返回 AAAA（拿到 IPv6 地址
        // 却连不通，多一次失败尝试），造成两模式行为不一致。
        //
        // 【单一真源】本函数是 dns.ipv6 的唯一写入点（两个分支都写）。
        // 下游 `apply_mandatory_dns_settings` 只补 enable/nameserver 等，不再兜 dns.ipv6 ——
        // 若在那里读顶层 ipv6 兜底，TUN 模式下读到的会是本函数刚强制过的 `true`（防泄漏接管），
        // 从而把用户关掉的 IPv6 静默写回开启。此处显式无条件同步即可根除该类风险。
        let dns_key = Value::from("dns");
        let mut dns_val = config
            .get(&dns_key)
            .and_then(|v| v.as_mapping().cloned())
            .unwrap_or_default();
        revise!(dns_val, "ipv6", user_ipv6);
        revise!(config, "dns", dns_val);
    }

    // 更新TUN配置
    revise!(tun_val, "enable", enable);
    revise!(config, "tun", tun_val);

    config
}

const DEFAULT_SNIFF_SKIP_DOMAINS: &[&str] = &["Mijia Cloud", "+.push.apple.com"];
const DEFAULT_SNIFF_SKIP_DST_CIDRS: &[&str] = &[
    "127.0.0.0/8",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "100.64.0.0/10",
    "169.254.0.0/16",
    "::1/128",
    "fc00::/7",
    "fe80::/10",
];

fn insert_if_missing(map: &mut Mapping, key: &str, value: serde_yaml_ng::Value) {
    let k = serde_yaml_ng::Value::from(key);
    if !map.contains_key(&k) {
        map.insert(k, value);
    }
}

fn append_missing_str_items(map: &mut Mapping, key: &str, items: &[&str]) {
    use serde_yaml_ng::Value;

    let k = Value::from(key);
    let mut seq = match map.get(&k) {
        Some(Value::Sequence(existing)) => existing.clone(),
        _ => Vec::with_capacity(items.len()),
    };
    for &item in items {
        let already = seq.iter().any(|v| v.as_str() == Some(item));
        if !already {
            seq.push(Value::String(item.into()));
        }
    }
    map.insert(k, Value::Sequence(seq));
}

fn merge_tun_sniffer_defaults(config: &mut Mapping) {
    use serde_yaml_ng::Value;

    let mut sniffer = config
        .get("sniffer")
        .and_then(|v| v.as_mapping())
        .cloned()
        .unwrap_or_default();

    // TUN 模式必须开启 sniffer，确保非 Fake-IP / 缓存 IP / 污染 IP 流量可通过 TLS SNI / HTTP Host / QUIC SNI 还原域名
    revise!(sniffer, "enable", true);
    insert_if_missing(&mut sniffer, "force-dns-mapping", Value::Bool(true));
    insert_if_missing(&mut sniffer, "parse-pure-ip", Value::Bool(true));
    insert_if_missing(&mut sniffer, "override-destination", Value::Bool(true));

    let mut sniff_map = sniffer
        .get("sniff")
        .and_then(|v| v.as_mapping())
        .cloned()
        .unwrap_or_default();

    let mut http_proto = Mapping::new();
    http_proto.insert(
        Value::from("ports"),
        Value::Sequence(vec![Value::Number(80.into()), Value::String("8080-8880".into())]),
    );
    http_proto.insert(Value::from("override-destination"), Value::Bool(true));
    insert_if_missing(&mut sniff_map, "HTTP", Value::Mapping(http_proto));

    for proto in ["TLS", "QUIC"] {
        let mut proto_map = Mapping::new();
        proto_map.insert(
            Value::from("ports"),
            Value::Sequence(vec![Value::Number(443.into()), Value::Number(8443.into())]),
        );
        insert_if_missing(&mut sniff_map, proto, Value::Mapping(proto_map));
    }
    sniffer.insert(Value::from("sniff"), Value::Mapping(sniff_map));

    append_missing_str_items(&mut sniffer, "skip-domain", DEFAULT_SNIFF_SKIP_DOMAINS);
    append_missing_str_items(&mut sniffer, "skip-dst-address", DEFAULT_SNIFF_SKIP_DST_CIDRS);

    revise!(config, "sniffer", sniffer);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_yaml_ng::Value;

    #[test]
    fn use_tun_enabled_injects_sniffer_defaults_and_preserves_custom_fields() {
        let cfg: Mapping = serde_yaml_ng::from_str(
            r"
ipv6: false
dns:
  enable: true
tun:
  enable: false
",
        )
        .unwrap();

        let out = use_tun(cfg, true);
        let sniffer = out.get("sniffer").and_then(Value::as_mapping).unwrap();
        assert_eq!(sniffer.get("enable").and_then(Value::as_bool), Some(true));
        assert_eq!(sniffer.get("force-dns-mapping").and_then(Value::as_bool), Some(true));
        assert_eq!(sniffer.get("parse-pure-ip").and_then(Value::as_bool), Some(true));
        assert_eq!(sniffer.get("override-destination").and_then(Value::as_bool), Some(true));

        let sniff = sniffer.get("sniff").and_then(Value::as_mapping).unwrap();
        let http = sniff.get("HTTP").and_then(Value::as_mapping).unwrap();
        assert_eq!(http.get("override-destination").and_then(Value::as_bool), Some(true));
        assert!(sniff.get("TLS").is_some());
        assert!(sniff.get("QUIC").is_some());

        let skip_domain = sniffer.get("skip-domain").and_then(Value::as_sequence).unwrap();
        assert!(skip_domain.iter().any(|v| v.as_str() == Some("Mijia Cloud")));
        assert!(skip_domain.iter().any(|v| v.as_str() == Some("+.push.apple.com")));

        let skip_dst = sniffer.get("skip-dst-address").and_then(Value::as_sequence).unwrap();
        assert!(skip_dst.iter().any(|v| v.as_str() == Some("192.168.0.0/16")));
        assert!(skip_dst.iter().any(|v| v.as_str() == Some("fe80::/10")));
        assert!(!skip_dst.iter().any(|v| v.as_str() == Some("198.18.0.0/16")));
    }

    #[test]
    fn use_tun_enabled_merges_partial_subscription_sniffer_without_duplicates() {
        let cfg: Mapping = serde_yaml_ng::from_str(
            r#"
ipv6: false
sniffer:
  enable: false
  override-destination: false
  skip-domain:
    - Mijia Cloud
    - "+.internal.example"
dns:
  enable: true
"#,
        )
        .unwrap();

        let out = use_tun(cfg, true);
        let sniffer = out.get("sniffer").and_then(Value::as_mapping).unwrap();
        assert_eq!(sniffer.get("enable").and_then(Value::as_bool), Some(true));
        assert_eq!(
            sniffer.get("override-destination").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(sniffer.get("force-dns-mapping").and_then(Value::as_bool), Some(true));
        assert_eq!(sniffer.get("parse-pure-ip").and_then(Value::as_bool), Some(true));

        let skip_domain = sniffer.get("skip-domain").and_then(Value::as_sequence).unwrap();
        assert_eq!(
            skip_domain.iter().filter(|v| v.as_str() == Some("Mijia Cloud")).count(),
            1
        );
        assert!(skip_domain.iter().any(|v| v.as_str() == Some("+.internal.example")));
        assert!(skip_domain.iter().any(|v| v.as_str() == Some("+.push.apple.com")));
    }

    #[test]
    fn use_tun_disabled_does_not_inject_sniffer() {
        let cfg: Mapping = serde_yaml_ng::from_str(
            r"
ipv6: false
dns:
  enable: true
",
        )
        .unwrap();

        let out = use_tun(cfg, false);
        assert!(out.get("sniffer").is_none());
    }
}
