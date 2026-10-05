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
        let ipv6_val = config
            .get(Value::from("ipv6"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let current_mode = dns_val
            .get(Value::from("enhanced-mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("fake-ip");

        if current_mode == "fake-ip" || !dns_val.contains_key(Value::from("enhanced-mode")) {
            revise!(dns_val, "enable", true);
            revise!(dns_val, "ipv6", ipv6_val);
            if !dns_val.contains_key(Value::from("enhanced-mode")) {
                revise!(dns_val, "enhanced-mode", "fake-ip");
            }
            if !dns_val.contains_key(Value::from("fake-ip-range")) {
                revise!(dns_val, "fake-ip-range", "198.18.0.1/16");
            }
            if ipv6_val && !dns_val.contains_key(Value::from("fake-ip-range6")) {
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
        revise!(config, "dns", dns_val);
    } else {
        // TUN未启用时，不修改配置文件中的DNS设置
    }

    // 更新TUN配置
    revise!(tun_val, "enable", enable);
    revise!(config, "tun", tun_val);

    config
}
