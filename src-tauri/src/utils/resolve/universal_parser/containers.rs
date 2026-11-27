use super::{
    TransportOpts, apply_ss_plugin_parts, apply_transport_opts, decode_base64_robust, parse_uri_list,
    uri_modern::apply_wireguard_addresses,
};
use anyhow::Result;
use serde_json::{Map as JsonMap, Value as JsonValue};
use serde_yaml_ng::{Mapping, Value};

fn json_str<'a>(obj: &'a JsonMap<String, JsonValue>, key: &str) -> Option<&'a str> {
    obj.get(key)
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn json_port(obj: &JsonMap<String, JsonValue>, key: &str) -> Option<u16> {
    obj.get(key)
        .and_then(JsonValue::as_u64)
        .and_then(|p| u16::try_from(p).ok())
}

fn json_str_list(obj: &JsonMap<String, JsonValue>, key: &str) -> Vec<String> {
    match obj.get(key) {
        Some(JsonValue::Array(arr)) => arr
            .iter()
            .filter_map(JsonValue::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        Some(JsonValue::String(s)) => {
            let t = s.trim();
            if t.is_empty() { Vec::new() } else { vec![t.to_string()] }
        }
        _ => Vec::new(),
    }
}

fn apply_singbox_tls(map: &mut Mapping, obj: &JsonMap<String, JsonValue>, sni_key: &str, emit_tls_bool: bool) {
    let Some(JsonValue::Object(tls)) = obj.get("tls") else {
        return;
    };
    let enabled = tls.get("enabled").and_then(JsonValue::as_bool).unwrap_or(false);
    if !enabled {
        return;
    }
    if emit_tls_bool {
        map.insert(Value::from("tls"), Value::from(true));
    }
    if let Some(sni) = json_str(tls, "server_name") {
        map.insert(Value::from(sni_key), Value::from(sni));
    }
    if tls.get("insecure").and_then(JsonValue::as_bool) == Some(true) {
        map.insert(Value::from("skip-cert-verify"), Value::from(true));
    }
    if tls.get("disable_sni").and_then(JsonValue::as_bool) == Some(true) {
        map.insert(Value::from("disable-sni"), Value::from(true));
    }
    let alpn = json_str_list(tls, "alpn");
    if !alpn.is_empty() {
        map.insert(Value::from("alpn"), Value::from(alpn));
    }
    if let Some(JsonValue::Object(utls)) = tls.get("utls")
        && let Some(fp) = json_str(utls, "fingerprint")
    {
        map.insert(Value::from("client-fingerprint"), Value::from(fp));
    }
    if let Some(JsonValue::Object(reality)) = tls.get("reality")
        && reality.get("enabled").and_then(JsonValue::as_bool).unwrap_or(true)
        && let Some(pbk) = json_str(reality, "public_key")
    {
        let mut r_opts = Mapping::new();
        r_opts.insert(Value::from("public-key"), Value::from(pbk));
        if let Some(sid) = json_str(reality, "short_id") {
            r_opts.insert(Value::from("short-id"), Value::from(sid));
        }
        map.insert(Value::from("reality-opts"), Value::from(r_opts));
    }
}

fn apply_singbox_transport(map: &mut Mapping, obj: &JsonMap<String, JsonValue>, has_tls: bool) {
    let Some(JsonValue::Object(tr)) = obj.get("transport") else {
        return;
    };
    let Some(tr_type) = json_str(tr, "type") else {
        return;
    };

    let mut opts = TransportOpts {
        network: match tr_type {
            "http" if has_tls => "h2".to_string(),
            other => other.to_string(),
        },
        ..TransportOpts::default()
    };

    if let Some(path) = json_str(tr, "path") {
        opts.path = path.to_string();
    }
    let hosts = json_str_list(tr, "host");
    if !hosts.is_empty() {
        opts.host = hosts.join(",");
    } else if let Some(JsonValue::Object(headers)) = tr.get("headers") {
        let hdr_hosts = json_str_list(headers, "Host");
        if !hdr_hosts.is_empty() {
            opts.host = hdr_hosts.join(",");
        }
    }
    if let Some(svc) = json_str(tr, "service_name") {
        opts.service_name = svc.to_string();
    }
    if let Some(mode) = json_str(tr, "mode") {
        opts.mode = mode.to_string();
    }
    apply_transport_opts(map, &opts, true);
}

fn init_singbox_base(
    obj: &JsonMap<String, JsonValue>,
    mihomo_type: &str,
    name: &str,
    default_port: u16,
) -> Option<Mapping> {
    let server = json_str(obj, "server")?;
    let port = json_port(obj, "server_port").unwrap_or(default_port);

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from(mihomo_type));
    map.insert(Value::from("name"), Value::from(name));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("udp"), Value::from(true));
    if let Some(detour) = json_str(obj, "detour") {
        map.insert(Value::from("dialer-proxy"), Value::from(detour));
    }
    Some(map)
}

fn convert_singbox_ss(obj: &JsonMap<String, JsonValue>, name: &str) -> Option<Mapping> {
    let mut map = init_singbox_base(obj, "ss", name, 8388)?;
    map.insert(Value::from("cipher"), Value::from(json_str(obj, "method")?));
    map.insert(
        Value::from("password"),
        Value::from(json_str(obj, "password").unwrap_or("")),
    );

    if let Some(plugin) = json_str(obj, "plugin") {
        let opts = json_str(obj, "plugin_opts").unwrap_or("");
        apply_ss_plugin_parts(&mut map, plugin, opts);
    }
    let uot = match obj.get("udp_over_tcp") {
        Some(JsonValue::Bool(b)) => *b,
        Some(JsonValue::Object(uot_obj)) => uot_obj.get("enabled").and_then(JsonValue::as_bool).unwrap_or(false),
        _ => false,
    };
    if uot {
        map.insert(Value::from("udp-over-tcp"), Value::from(true));
    }
    Some(map)
}

fn convert_singbox_hy2(obj: &JsonMap<String, JsonValue>, name: &str) -> Option<Mapping> {
    let mut map = init_singbox_base(obj, "hysteria2", name, 443)?;
    if let Some(pw) = json_str(obj, "password") {
        map.insert(Value::from("password"), Value::from(pw));
    }
    if let Some(up) = obj.get("up_mbps").and_then(JsonValue::as_u64) {
        map.insert(Value::from("up"), Value::from(format!("{up} Mbps")));
    }
    if let Some(down) = obj.get("down_mbps").and_then(JsonValue::as_u64) {
        map.insert(Value::from("down"), Value::from(format!("{down} Mbps")));
    }
    let ports: Vec<String> = json_str_list(obj, "server_ports")
        .into_iter()
        .map(|p| p.replace(':', "-"))
        .collect();
    if !ports.is_empty() {
        map.insert(Value::from("ports"), Value::from(ports.join(",")));
    }
    if let Some(hop) = obj.get("hop_interval") {
        let secs_opt = match hop {
            JsonValue::Number(n) => n.as_u64(),
            JsonValue::String(s) => {
                let trimmed = s.trim();
                trimmed.strip_suffix('s').unwrap_or(trimmed).parse::<u64>().ok()
            }
            _ => None,
        };
        if let Some(secs) = secs_opt {
            map.insert(Value::from("hop-interval"), Value::from(secs));
        }
    }
    if let Some(JsonValue::Object(obfs)) = obj.get("obfs") {
        if let Some(obfs_type) = json_str(obfs, "type") {
            map.insert(Value::from("obfs"), Value::from(obfs_type));
        }
        if let Some(obfs_pw) = json_str(obfs, "password") {
            map.insert(Value::from("obfs-password"), Value::from(obfs_pw));
        }
    }
    apply_singbox_tls(&mut map, obj, "sni", false);
    Some(map)
}

fn convert_singbox_hysteria(obj: &JsonMap<String, JsonValue>, name: &str) -> Option<Mapping> {
    let mut map = init_singbox_base(obj, "hysteria", name, 443)?;
    if let Some(auth) = json_str(obj, "auth_str").or_else(|| json_str(obj, "auth")) {
        map.insert(Value::from("auth-str"), Value::from(auth));
    }
    if let Some(up) = json_str(obj, "up") {
        map.insert(Value::from("up"), Value::from(up));
    } else if let Some(up) = obj.get("up_mbps").and_then(JsonValue::as_u64) {
        map.insert(Value::from("up"), Value::from(format!("{up} Mbps")));
    }
    if let Some(down) = json_str(obj, "down") {
        map.insert(Value::from("down"), Value::from(down));
    } else if let Some(down) = obj.get("down_mbps").and_then(JsonValue::as_u64) {
        map.insert(Value::from("down"), Value::from(format!("{down} Mbps")));
    }
    if let Some(obfs) = json_str(obj, "obfs") {
        map.insert(Value::from("obfs"), Value::from(obfs));
    }
    apply_singbox_tls(&mut map, obj, "sni", false);
    Some(map)
}

fn convert_singbox_wireguard(obj: &JsonMap<String, JsonValue>, name: &str) -> Option<Mapping> {
    let first_peer = obj
        .get("peers")
        .and_then(JsonValue::as_array)
        .and_then(|a| a.first())
        .and_then(JsonValue::as_object);

    let server = json_str(obj, "server")
        .or_else(|| first_peer.and_then(|p| json_str(p, "address").or_else(|| json_str(p, "server"))))?;
    let port = json_port(obj, "server_port")
        .or_else(|| first_peer.and_then(|p| json_port(p, "port").or_else(|| json_port(p, "server_port"))))
        .unwrap_or(51820);
    let private_key = json_str(obj, "private_key")?;
    let public_key = json_str(obj, "peer_public_key").or_else(|| first_peer.and_then(|p| json_str(p, "public_key")));

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("wireguard"));
    map.insert(Value::from("name"), Value::from(name));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("private-key"), Value::from(private_key));
    map.insert(Value::from("udp"), Value::from(true));

    if let Some(pk) = public_key {
        map.insert(Value::from("public-key"), Value::from(pk));
    }
    if let Some(psk) =
        json_str(obj, "pre_shared_key").or_else(|| first_peer.and_then(|p| json_str(p, "pre_shared_key")))
    {
        map.insert(Value::from("pre-shared-key"), Value::from(psk));
    }
    let reserved_src = obj
        .get("reserved")
        .or_else(|| first_peer.and_then(|p| p.get("reserved")));
    if let Some(JsonValue::Array(arr)) = reserved_src {
        let reserved: Vec<u16> = arr
            .iter()
            .filter_map(JsonValue::as_u64)
            .filter_map(|v| u16::try_from(v).ok())
            .collect();
        if !reserved.is_empty() {
            map.insert(Value::from("reserved"), Value::from(reserved));
        }
    }
    if let Some(mtu) = json_port(obj, "mtu") {
        map.insert(Value::from("mtu"), Value::from(mtu));
    }
    if let Some(detour) = json_str(obj, "detour") {
        map.insert(Value::from("dialer-proxy"), Value::from(detour));
    }

    let addrs = {
        let mut list = json_str_list(obj, "local_address");
        if list.is_empty() {
            list = json_str_list(obj, "address");
        }
        list
    };
    if addrs.is_empty() {
        map.insert(Value::from("allowed-ips"), Value::from(vec![Value::from("0.0.0.0/0")]));
    } else {
        apply_wireguard_addresses(&mut map, &addrs);
    }
    Some(map)
}

fn convert_singbox_outbound(obj: &JsonMap<String, JsonValue>, index: usize) -> Option<Mapping> {
    let ob_type = json_str(obj, "type")?;
    if matches!(ob_type, "direct" | "block" | "dns" | "selector" | "urltest") {
        return None;
    }

    let fallback_name = format!("{ob_type}-{index}");
    let name = json_str(obj, "tag").unwrap_or(&fallback_name);
    let has_tls = obj
        .get("tls")
        .and_then(JsonValue::as_object)
        .and_then(|t| t.get("enabled"))
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);

    match ob_type {
        "shadowsocks" => convert_singbox_ss(obj, name),
        "vmess" => {
            let mut map = init_singbox_base(obj, "vmess", name, 443)?;
            map.insert(Value::from("uuid"), Value::from(json_str(obj, "uuid")?));
            map.insert(
                Value::from("alterId"),
                Value::from(obj.get("alter_id").and_then(JsonValue::as_u64).unwrap_or(0)),
            );
            map.insert(
                Value::from("cipher"),
                Value::from(json_str(obj, "security").unwrap_or("auto")),
            );
            apply_singbox_tls(&mut map, obj, "servername", true);
            apply_singbox_transport(&mut map, obj, has_tls);
            Some(map)
        }
        "vless" => {
            let mut map = init_singbox_base(obj, "vless", name, 443)?;
            map.insert(Value::from("uuid"), Value::from(json_str(obj, "uuid")?));
            map.insert(Value::from("cipher"), Value::from("auto"));
            if let Some(flow) = json_str(obj, "flow") {
                map.insert(Value::from("flow"), Value::from(flow));
            }
            apply_singbox_tls(&mut map, obj, "servername", true);
            apply_singbox_transport(&mut map, obj, has_tls);
            Some(map)
        }
        "trojan" => {
            let mut map = init_singbox_base(obj, "trojan", name, 443)?;
            map.insert(Value::from("password"), Value::from(json_str(obj, "password")?));
            apply_singbox_tls(&mut map, obj, "sni", false);
            apply_singbox_transport(&mut map, obj, has_tls);
            Some(map)
        }
        "hysteria2" => convert_singbox_hy2(obj, name),
        "hysteria" => convert_singbox_hysteria(obj, name),
        "tuic" => {
            let mut map = init_singbox_base(obj, "tuic", name, 443)?;
            map.insert(Value::from("uuid"), Value::from(json_str(obj, "uuid")?));
            map.insert(
                Value::from("password"),
                Value::from(json_str(obj, "password").unwrap_or("")),
            );
            if let Some(cc) = json_str(obj, "congestion_control") {
                map.insert(Value::from("congestion-controller"), Value::from(cc));
            }
            if let Some(mode) = json_str(obj, "udp_relay_mode") {
                map.insert(Value::from("udp-relay-mode"), Value::from(mode));
            }
            if obj.get("zero_rtt_handshake").and_then(JsonValue::as_bool) == Some(true) {
                map.insert(Value::from("reduce-rtt"), Value::from(true));
            }
            apply_singbox_tls(&mut map, obj, "sni", false);
            Some(map)
        }
        "anytls" => {
            let mut map = init_singbox_base(obj, "anytls", name, 443)?;
            map.insert(Value::from("password"), Value::from(json_str(obj, "password")?));
            apply_singbox_tls(&mut map, obj, "sni", false);
            Some(map)
        }
        "wireguard" => convert_singbox_wireguard(obj, name),
        "socks" => {
            let mut map = init_singbox_base(obj, "socks5", name, 1080)?;
            if let Some(u) = json_str(obj, "username") {
                map.insert(Value::from("username"), Value::from(u));
            }
            if let Some(p) = json_str(obj, "password") {
                map.insert(Value::from("password"), Value::from(p));
            }
            Some(map)
        }
        "http" => {
            let mut map = init_singbox_base(obj, "http", name, 443)?;
            if let Some(u) = json_str(obj, "username") {
                map.insert(Value::from("username"), Value::from(u));
            }
            if let Some(p) = json_str(obj, "password") {
                map.insert(Value::from("password"), Value::from(p));
            }
            apply_singbox_tls(&mut map, obj, "sni", true);
            Some(map)
        }
        "ssh" => {
            let mut map = init_singbox_base(obj, "ssh", name, 22)?;
            if let Some(u) = json_str(obj, "user") {
                map.insert(Value::from("username"), Value::from(u));
            }
            if let Some(p) = json_str(obj, "password") {
                map.insert(Value::from("password"), Value::from(p));
            }
            if let Some(pk) = json_str(obj, "private_key") {
                map.insert(Value::from("private-key"), Value::from(pk));
            }
            if let Some(pkp) = json_str(obj, "private_key_passphrase") {
                map.insert(Value::from("private-key-passphrase"), Value::from(pkp));
            }
            let host_keys = json_str_list(obj, "host_key");
            if !host_keys.is_empty() {
                map.insert(Value::from("host-key"), Value::from(host_keys));
            }
            let algos = json_str_list(obj, "host_key_algorithms");
            if !algos.is_empty() {
                map.insert(Value::from("host-key-algorithms"), Value::from(algos));
            }
            Some(map)
        }
        _ => None,
    }
}

pub(super) fn parse_singbox_json(text: &str) -> Option<Mapping> {
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let JsonValue::Object(root) = serde_json::from_str::<JsonValue>(text).ok()? else {
        return None;
    };
    let JsonValue::Array(outbounds) = root.get("outbounds")? else {
        return None;
    };

    let mut proxies = Vec::new();
    for (idx, item) in outbounds.iter().enumerate() {
        if let Some(obj) = item.as_object()
            && let Some(proxy_map) = convert_singbox_outbound(obj, idx + 1)
        {
            proxies.push(Value::from(proxy_map));
        }
    }

    if proxies.is_empty() {
        return None;
    }
    let mut result = Mapping::new();
    result.insert(Value::from("proxies"), Value::from(proxies));
    Some(result)
}

pub(super) fn parse_sip008_json(text: &str) -> Option<Mapping> {
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let JsonValue::Object(root) = serde_json::from_str::<JsonValue>(text).ok()? else {
        return None;
    };
    let JsonValue::Array(servers) = root.get("servers")? else {
        return None;
    };

    let mut proxies = Vec::new();
    for item in servers {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let Some(server) = json_str(obj, "server") else {
            continue;
        };
        let Some(port) = json_port(obj, "server_port") else {
            continue;
        };
        let Some(method) = json_str(obj, "method") else {
            continue;
        };
        let Some(password) = json_str(obj, "password") else {
            continue;
        };
        let fallback_name = format!("{server}:{port}");
        let name = json_str(obj, "remarks").unwrap_or(&fallback_name);

        let mut map = Mapping::new();
        map.insert(Value::from("type"), Value::from("ss"));
        map.insert(Value::from("name"), Value::from(name));
        map.insert(Value::from("server"), Value::from(server));
        map.insert(Value::from("port"), Value::from(port));
        map.insert(Value::from("cipher"), Value::from(method));
        map.insert(Value::from("password"), Value::from(password));
        map.insert(Value::from("udp"), Value::from(true));

        if let Some(plugin) = json_str(obj, "plugin") {
            let opts = json_str(obj, "plugin_opts").unwrap_or("");
            apply_ss_plugin_parts(&mut map, plugin, opts);
        }

        proxies.push(Value::from(map));
    }

    if proxies.is_empty() {
        return None;
    }
    let mut result = Mapping::new();
    result.insert(Value::from("proxies"), Value::from(proxies));
    Some(result)
}

fn try_parse_structured_subscription(text: &str) -> Option<(Mapping, smartstring::alias::String)> {
    if let Ok(y) = serde_yaml_ng::from_str::<Mapping>(text)
        && (y.contains_key("proxies") || y.contains_key("proxy-providers"))
    {
        return Some((y, text.to_string().into()));
    }
    if let Some(parsed) = parse_singbox_json(text).or_else(|| parse_sip008_json(text)) {
        let serialized = serde_yaml_ng::to_string(&parsed).unwrap_or_default().into();
        return Some((parsed, serialized));
    }
    None
}

/// 统一订阅载荷解析管道：Clash YAML → sing-box / SIP008 JSON → Base64 解码再探 → URI 列表回退。
pub fn parse_subscription_payload(data: &str) -> Result<(Mapping, smartstring::alias::String)> {
    let data = data.trim_start_matches('\u{feff}');
    if let Some(res) = try_parse_structured_subscription(data) {
        return Ok(res);
    }

    let decoded_str_opt = decode_base64_robust(data).and_then(|bytes| String::from_utf8(bytes).ok());
    if let Some(ref decoded_str) = decoded_str_opt
        && let Some(res) = try_parse_structured_subscription(decoded_str.trim_start_matches('\u{feff}'))
    {
        return Ok(res);
    }

    let uri_input = decoded_str_opt.as_deref().unwrap_or(data);
    if let Some(parsed) = parse_uri_list(uri_input) {
        let serialized = serde_yaml_ng::to_string(&parsed).unwrap_or_default().into();
        return Ok((parsed, serialized));
    }

    anyhow::bail!("订阅内容格式无法识别，支持 Clash YAML、sing-box JSON、SIP008 JSON 及常见节点分享链接/Base64 订阅")
}
