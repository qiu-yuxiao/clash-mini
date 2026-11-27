use super::{
    TransportOpts, apply_ss_plugin, apply_transport_opts, decode_base64_robust, extract_host_port, insert_alpn_list,
    is_truthy, parse_query_pairs, percent_decode, strip_fragment_and_query, strip_prefix_ascii_case,
};
use serde_yaml_ng::{Mapping, Value};
use std::str;

#[derive(serde::Deserialize, Debug)]
struct VMessJson {
    ps: Option<String>,
    add: Option<String>,
    port: Option<serde_json::Value>,
    id: Option<String>,
    aid: Option<serde_json::Value>,
    scy: Option<String>,
    net: Option<String>,
    #[serde(rename = "type")]
    header_type: Option<String>,
    host: Option<String>,
    path: Option<String>,
    tls: Option<String>,
    sni: Option<String>,
    alpn: Option<String>,
    fp: Option<String>,
    #[serde(alias = "skip-cert-verify", alias = "allowInsecure", alias = "insecure")]
    skip_cert_verify: Option<serde_json::Value>,
}

fn json_value_to_u16(val: Option<serde_json::Value>) -> Option<u16> {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_u64().and_then(|v| u16::try_from(v).ok()),
        Some(serde_json::Value::String(s)) => s.trim().parse::<u16>().ok(),
        _ => None,
    }
}

fn json_value_to_u32(val: Option<serde_json::Value>) -> Option<u32> {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_u64().and_then(|v| u32::try_from(v).ok()),
        Some(serde_json::Value::String(s)) => s.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn json_value_is_truthy(val: Option<&serde_json::Value>) -> bool {
    match val {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Number(n)) => n.as_u64() == Some(1),
        Some(serde_json::Value::String(s)) => is_truthy(s.trim()),
        _ => false,
    }
}

fn parse_vmess_base64_json(payload: &str) -> Option<Mapping> {
    if payload.contains('@') || payload.contains('?') {
        return None;
    }
    let decoded_bytes = decode_base64_robust(payload)?;
    let decoded_str = str::from_utf8(&decoded_bytes).ok()?;
    let json: VMessJson = serde_json::from_str(decoded_str).ok()?;

    let server = json.add.unwrap_or_default();
    let uuid = json.id.unwrap_or_default();
    if server.trim().is_empty() && uuid.trim().is_empty() {
        return None;
    }

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("vmess"));
    map.insert(
        Value::from("name"),
        Value::from(json.ps.unwrap_or_else(|| "VMess Node".to_string())),
    );
    map.insert(Value::from("server"), Value::from(server));
    map.insert(
        Value::from("port"),
        Value::from(json_value_to_u16(json.port).unwrap_or(443)),
    );
    map.insert(Value::from("uuid"), Value::from(uuid));
    map.insert(
        Value::from("alterId"),
        Value::from(json_value_to_u32(json.aid).unwrap_or(0)),
    );

    let cipher = json
        .scy
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("auto");
    map.insert(Value::from("cipher"), Value::from(cipher));
    map.insert(Value::from("udp"), Value::from(true));

    let sni_val = json
        .sni
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let host_trim = json
        .host
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(str::to_string);

    if json.tls.as_deref().is_some_and(|t| t.eq_ignore_ascii_case("tls")) {
        map.insert(Value::from("tls"), Value::from(true));
        if let Some(sni) = sni_val.or_else(|| host_trim.clone()) {
            map.insert(Value::from("servername"), Value::from(sni));
        }
    } else if let Some(sni) = sni_val {
        map.insert(Value::from("servername"), Value::from(sni));
    }

    if json_value_is_truthy(json.skip_cert_verify.as_ref()) {
        map.insert(Value::from("skip-cert-verify"), Value::from(true));
    }
    if let Some(alpn) = json.alpn.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        insert_alpn_list(&mut map, alpn);
    }
    if let Some(fp) = json.fp.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        map.insert(Value::from("client-fingerprint"), Value::from(fp));
    }

    let transport = TransportOpts {
        network: json.net.unwrap_or_default(),
        path: json.path.unwrap_or_default(),
        host: json.host.unwrap_or_default(),
        header_type: json.header_type.unwrap_or_default(),
        ..TransportOpts::default()
    };
    apply_transport_opts(&mut map, &transport, true);

    Some(map)
}

fn decode_vmess_userinfo(user_raw: &str) -> Option<(Option<String>, String)> {
    let decoded = if !user_raw.contains(':')
        && let Some(bytes) = decode_base64_robust(user_raw)
        && let Ok(s) = String::from_utf8(bytes)
        && s.contains(':')
    {
        s
    } else {
        percent_decode(user_raw)
    };

    if let Some((cipher, uuid)) = decoded.split_once(':') {
        let c = cipher.trim();
        let u = uuid.trim().to_string();
        if u.is_empty() {
            return None;
        }
        let cipher_opt = (!c.is_empty()).then(|| c.to_string());
        Some((cipher_opt, u))
    } else {
        let u = decoded.trim().to_string();
        (!u.is_empty()).then_some((None, u))
    }
}

fn extract_shadowrocket_obfs_host(raw: &str) -> String {
    let decoded = percent_decode(raw);
    let trimmed = decoded.trim();
    if trimmed.starts_with('{')
        && let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(trimmed)
        && let Some(host) = obj.get("Host").and_then(serde_json::Value::as_str)
    {
        return host.trim().to_string();
    }
    trimmed.to_string()
}

fn parse_vmess_uri(payload: &str) -> Option<Mapping> {
    let (auth_host, query, frag) = strip_fragment_and_query(payload);

    let (user_raw, host_port_raw) = if let Some((u, hp)) = auth_host.rsplit_once('@') {
        (u.to_string(), hp.to_string())
    } else {
        let bytes = decode_base64_robust(auth_host.trim_end_matches('/'))?;
        let decoded = String::from_utf8(bytes).ok()?;
        let (u, hp) = decoded.rsplit_once('@')?;
        (u.to_string(), hp.to_string())
    };

    let (cipher_from_user, uuid) = decode_vmess_userinfo(&user_raw)?;
    let (server, port) = extract_host_port(host_port_raw.trim_end_matches('/'), 443)?;
    if server.is_empty() {
        return None;
    }

    let mut remarks_param: Option<String> = None;
    let mut alter_id: u32 = 0;
    let mut cipher = cipher_from_user.unwrap_or_else(|| "auto".to_string());
    let mut transport = TransportOpts::default();

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("vmess"));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("uuid"), Value::from(uuid));
    map.insert(Value::from("udp"), Value::from(true));

    if let Some(q) = query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "remarks" => remarks_param = Some(percent_decode(&v)),
                "aid" | "alterid" => {
                    if let Ok(aid) = v.trim().parse::<u32>() {
                        alter_id = aid;
                    }
                }
                "scy" | "encryption" | "cipher" => {
                    cipher = percent_decode(&v);
                }
                "tls" if is_truthy(&v) || v.eq_ignore_ascii_case("tls") => {
                    map.insert(Value::from("tls"), Value::from(true));
                }
                "security" if v.eq_ignore_ascii_case("tls") => {
                    map.insert(Value::from("tls"), Value::from(true));
                }
                "sni" | "peer" => {
                    map.insert(Value::from("servername"), Value::from(percent_decode(&v)));
                }
                "allowinsecure" | "insecure" | "skip-cert-verify" | "allow_insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                "alpn" => insert_alpn_list(&mut map, &v),
                "fp" | "fingerprint" => {
                    map.insert(Value::from("client-fingerprint"), Value::from(percent_decode(&v)));
                }
                "type" | "net" | "obfs" if !v.eq_ignore_ascii_case("none") => {
                    transport.network = percent_decode(&v);
                }
                "headertype" => transport.header_type = percent_decode(&v),
                "path" => transport.path = percent_decode(&v),
                "host" => transport.host = percent_decode(&v),
                "obfsparam" if transport.host.is_empty() => {
                    transport.host = extract_shadowrocket_obfs_host(&v);
                }
                "servicename" | "service-name" | "service_name" => transport.service_name = percent_decode(&v),
                _ => {}
            }
        }
    }

    let name = frag
        .filter(|s| !s.is_empty())
        .or(remarks_param)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "VMess Node".to_string());
    map.insert(Value::from("name"), Value::from(name));
    map.insert(Value::from("alterId"), Value::from(alter_id));
    map.insert(Value::from("cipher"), Value::from(cipher));

    if !map.contains_key("servername") && map.contains_key("tls") && !transport.host.is_empty() {
        map.insert(Value::from("servername"), Value::from(transport.host.as_str()));
    }
    apply_transport_opts(&mut map, &transport, true);

    Some(map)
}

pub(super) fn parse_vmess(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "vmess://")?.trim();
    parse_vmess_base64_json(payload).or_else(|| parse_vmess_uri(payload))
}

pub(super) fn parse_ss(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "ss://")?;
    let (auth_host, outer_query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "SS Node".to_string());

    let mut inner_query: Option<String> = None;
    let (method_pw, host_port) = if auth_host.contains('@') {
        let (m_p, h_p) = auth_host.rsplit_once('@')?;
        (m_p.to_string(), h_p.trim_end_matches('/').to_string())
    } else {
        let decoded = decode_base64_robust(auth_host.trim_end_matches('/'))?;
        let decoded_str = String::from_utf8(decoded).ok()?;
        let (decoded_clean, q_part) = match decoded_str.split_once('?') {
            Some((head, q)) => {
                if !q.is_empty() {
                    inner_query = Some(q.to_string());
                }
                (head, q)
            }
            None => (decoded_str.as_str(), ""),
        };
        let _ = q_part;
        let (m_p, h_p) = decoded_clean.rsplit_once('@')?;
        (m_p.to_string(), h_p.trim_end_matches('/').to_string())
    };

    let decoded_method_pw = if !method_pw.contains(':') {
        let decoded = decode_base64_robust(&method_pw)?;
        String::from_utf8(decoded).ok()?
    } else {
        percent_decode(&method_pw)
    };

    let mut mp_parts = decoded_method_pw.splitn(2, ':');
    let cipher = mp_parts.next()?.trim().to_string();
    let password = mp_parts.next()?.trim().to_string();

    let (server, port) = extract_host_port(&host_port, 8388)?;

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("ss"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("cipher"), Value::from(cipher));
    map.insert(Value::from("password"), Value::from(password));
    map.insert(Value::from("udp"), Value::from(true));

    let effective_query = outer_query.or(inner_query.as_deref());
    if let Some(q) = effective_query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "plugin" => {
                    let decoded_plugin = percent_decode(&v);
                    apply_ss_plugin(&mut map, &decoded_plugin);
                }
                "uot" if is_truthy(&v) => {
                    map.insert(Value::from("udp-over-tcp"), Value::from(true));
                }
                _ => {}
            }
        }
    }

    Some(map)
}

fn decode_base64_utf8_trimmed(raw: &str) -> Option<String> {
    let bytes = decode_base64_robust(raw)?;
    let s = String::from_utf8(bytes).ok()?;
    let trimmed = s.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(super) fn parse_ssr(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "ssr://")?.trim();
    let decoded_bytes = decode_base64_robust(payload)?;
    let decoded = String::from_utf8(decoded_bytes).ok()?;

    let (main_part, query_opt) = match decoded.split_once("/?") {
        Some((m, q)) => (m, Some(q)),
        None => match decoded.split_once('?') {
            Some((m, q)) => (m, Some(q)),
            None => (decoded.as_str(), None),
        },
    };

    // 从右往左拆 6 段：pass_b64 : obfs : method : protocol : port : server（支持 IPv6 冒号）
    let mut rparts = main_part.rsplitn(6, ':');
    let pass_b64 = rparts.next()?.trim();
    let obfs = rparts.next()?.trim();
    let method = rparts.next()?.trim();
    let protocol = rparts.next()?.trim();
    let port_str = rparts.next()?.trim();
    let server_raw = rparts.next()?.trim();

    let server = server_raw
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(server_raw)
        .trim();
    if server.is_empty() || method.is_empty() || protocol.is_empty() || obfs.is_empty() {
        return None;
    }

    let port = port_str.parse::<u16>().ok()?;
    let password = String::from_utf8(decode_base64_robust(pass_b64)?).ok()?;

    let mut remarks: Option<String> = None;
    let mut obfs_param: Option<String> = None;
    let mut proto_param: Option<String> = None;

    if let Some(q) = query_opt {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "remarks" => remarks = decode_base64_utf8_trimmed(&v),
                "obfsparam" => obfs_param = decode_base64_utf8_trimmed(&v),
                "protoparam" => proto_param = decode_base64_utf8_trimmed(&v),
                _ => {}
            }
        }
    }

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("ssr"));
    map.insert(
        Value::from("name"),
        Value::from(remarks.unwrap_or_else(|| "SSR Node".to_string())),
    );
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("cipher"), Value::from(method));
    map.insert(Value::from("password"), Value::from(password));
    map.insert(Value::from("obfs"), Value::from(obfs));
    map.insert(Value::from("protocol"), Value::from(protocol));
    if let Some(op) = obfs_param {
        map.insert(Value::from("obfs-param"), Value::from(op));
    }
    if let Some(pp) = proto_param {
        map.insert(Value::from("protocol-param"), Value::from(pp));
    }
    map.insert(Value::from("udp"), Value::from(true));

    Some(map)
}

pub(super) fn parse_trojan(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "trojan://")?.trim();
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "Trojan Node".to_string());

    let (password_raw, host_port) = auth_host.rsplit_once('@')?;
    let host_port = host_port.trim_end_matches('/');
    let password = percent_decode(password_raw);

    let (server, port) = extract_host_port(host_port, 443)?;

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("trojan"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("password"), Value::from(password));
    map.insert(Value::from("udp"), Value::from(true));

    if let Some(q) = query {
        let mut reality_opts = Mapping::new();
        let mut transport = TransportOpts::default();

        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "sni" | "peer" => {
                    map.insert(Value::from("sni"), Value::from(percent_decode(&v)));
                }
                "allowinsecure" | "insecure" | "allow_insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                "alpn" => insert_alpn_list(&mut map, &v),
                "fp" | "fingerprint" => {
                    map.insert(Value::from("client-fingerprint"), Value::from(percent_decode(&v)));
                }
                "pbk" => {
                    reality_opts.insert(Value::from("public-key"), Value::from(percent_decode(&v)));
                }
                "sid" => {
                    reality_opts.insert(Value::from("short-id"), Value::from(percent_decode(&v)));
                }
                "type" => transport.network = percent_decode(&v),
                "headertype" => transport.header_type = percent_decode(&v),
                "path" => transport.path = percent_decode(&v),
                "host" => transport.host = percent_decode(&v),
                "servicename" | "service-name" | "service_name" => transport.service_name = percent_decode(&v),
                _ => {}
            }
        }

        if !reality_opts.is_empty() {
            map.insert(Value::from("reality-opts"), Value::from(reality_opts));
        }
        apply_transport_opts(&mut map, &transport, true);
    }

    Some(map)
}

fn split_socks_userinfo(ui: &str) -> Option<(String, String)> {
    let decoded = if !ui.contains(':')
        && let Some(bytes) = decode_base64_robust(ui)
        && let Ok(s) = String::from_utf8(bytes)
    {
        s
    } else {
        ui.to_string()
    };
    let (username, password) = decoded.split_once(':')?;
    Some((percent_decode(username), percent_decode(password)))
}

pub(super) fn parse_socks5(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "socks5://").or_else(|| strip_prefix_ascii_case(link, "socks://"))?;
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "SOCKS5 Node".to_string());

    let (user_info, host_port) = if let Some((ui, hp)) = auth_host.rsplit_once('@') {
        (Some(ui.to_string()), hp.trim_end_matches('/').to_string())
    } else if !auth_host.contains(':')
        && let Some(bytes) = decode_base64_robust(auth_host.trim_end_matches('/'))
        && let Ok(decoded) = String::from_utf8(bytes)
        && let Some((ui, hp)) = decoded.rsplit_once('@')
    {
        (Some(ui.to_string()), hp.trim_end_matches('/').to_string())
    } else {
        (None, auth_host.trim_end_matches('/').to_string())
    };

    let (server, port) = extract_host_port(&host_port, 1080)?;

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("socks5"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("udp"), Value::from(true));

    if let Some(ui) = user_info
        && let Some((username, password)) = split_socks_userinfo(&ui)
    {
        map.insert(Value::from("username"), Value::from(username));
        map.insert(Value::from("password"), Value::from(password));
    }

    if let Some(q) = query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "tls" if is_truthy(&v) => {
                    map.insert(Value::from("tls"), Value::from(true));
                }
                "insecure" | "allowinsecure" | "allow_insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                _ => {}
            }
        }
    }

    Some(map)
}

pub(super) fn parse_http_fallback(line: &str) -> Option<Mapping> {
    let line = line.trim().strip_prefix('#').unwrap_or(line).trim();
    let payload = line.strip_prefix("[HTTP]")?.trim();

    let mut parts = payload.splitn(2, " (");
    let name = parts.next()?.trim().to_string();
    let addr_port_part = parts.next()?;
    let addr_port = addr_port_part.strip_suffix(')')?.trim();

    let (server, port_str) = addr_port.split_once(':')?;
    let port = port_str.parse::<u16>().ok().unwrap_or(80);

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("http"));
    map.insert(Value::from("name"), Value::from(name));
    map.insert(Value::from("server"), Value::from(server.trim()));
    map.insert(Value::from("port"), Value::from(port));

    Some(map)
}
