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
            .unwrap_or_else(Mapping::new);
        revise!(dns_val, "ipv6", user_ipv6);
        revise!(config, "dns", dns_val);
    }

    // 更新TUN配置
    revise!(tun_val, "enable", enable);
    revise!(config, "tun", tun_val);

    config
}
