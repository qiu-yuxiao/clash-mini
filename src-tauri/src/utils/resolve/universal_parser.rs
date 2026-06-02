use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_yaml_ng::Mapping;
use std::str;

#[derive(serde::Deserialize, Debug)]
struct VMessJson {
    ps: Option<String>,
    add: Option<String>,
    port: Option<serde_json::Value>,
    id: Option<String>,
    aid: Option<serde_json::Value>,
    net: Option<String>,
    host: Option<String>,
    path: Option<String>,
    tls: Option<String>,
}

pub fn decode_base64_robust(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Try standard base64 first
    if let Ok(data) = STANDARD.decode(s) {
        return Some(data);
    }
    
    // Try URL safe base64
    if let Ok(data) = base64::engine::general_purpose::URL_SAFE.decode(s) {
        return Some(data);
    }

    // Try padding if length is not multiple of 4
    let mut padded = s.to_string();
    while !padded.len().is_multiple_of(4) {
        padded.push('=');
    }
    if let Ok(data) = STANDARD.decode(&padded) {
        return Some(data);
    }
    if let Ok(data) = base64::engine::general_purpose::URL_SAFE.decode(&padded) {
        return Some(data);
    }

    None
}

fn parse_vmess(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("vmess://")?.trim();
    let decoded_bytes = decode_base64_robust(payload)?;
    let decoded_str = str::from_utf8(&decoded_bytes).ok()?;
    let json: VMessJson = serde_json::from_str(decoded_str).ok()?;
    
    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::from("type"),
        serde_yaml_ng::Value::from("vmess"),
    );
    map.insert(
        serde_yaml_ng::Value::from("name"),
        serde_yaml_ng::Value::from(json.ps.unwrap_or_else(|| "VMess Node".to_string())),
    );
    map.insert(
        serde_yaml_ng::Value::from("server"),
        serde_yaml_ng::Value::from(json.add.unwrap_or_default()),
    );
    
    let port = json.port.and_then(|p| match p {
        serde_json::Value::Number(n) => n.as_u64().map(|v| v as u16),
        serde_json::Value::String(s) => s.parse::<u16>().ok(),
        _ => None,
    }).unwrap_or(443);
    map.insert(
        serde_yaml_ng::Value::from("port"),
        serde_yaml_ng::Value::from(port),
    );
    
    map.insert(
        serde_yaml_ng::Value::from("uuid"),
        serde_yaml_ng::Value::from(json.id.unwrap_or_default()),
    );
    
    let alter_id = json.aid.and_then(|a| match a {
        serde_json::Value::Number(n) => n.as_u64().map(|v| v as u32),
        serde_json::Value::String(s) => s.parse::<u32>().ok(),
        _ => None,
    }).unwrap_or(0);
    map.insert(
        serde_yaml_ng::Value::from("alterId"),
        serde_yaml_ng::Value::from(alter_id),
    );
    
    map.insert(
        serde_yaml_ng::Value::from("cipher"),
        serde_yaml_ng::Value::from("auto"),
    );
    map.insert(
        serde_yaml_ng::Value::from("udp"),
        serde_yaml_ng::Value::from(true),
    );
    
    if json.tls.as_deref() == Some("tls") {
        map.insert(
            serde_yaml_ng::Value::from("tls"),
            serde_yaml_ng::Value::from(true),
        );
    }
    
    let network = json.net.unwrap_or_default();
    if !network.is_empty() && network != "tcp" {
        map.insert(
            serde_yaml_ng::Value::from("network"),
            serde_yaml_ng::Value::from(network.clone()),
        );
        if network == "ws" {
            let mut ws_opts = serde_yaml_ng::Mapping::new();
            if let Some(p) = json.path {
                ws_opts.insert(
                    serde_yaml_ng::Value::from("path"),
                    serde_yaml_ng::Value::from(p),
                );
            }
            if let Some(h) = json.host.filter(|h| !h.is_empty()) {
                let mut headers = serde_yaml_ng::Mapping::new();
                headers.insert(
                    serde_yaml_ng::Value::from("Host"),
                    serde_yaml_ng::Value::from(h),
                );
                ws_opts.insert(
                    serde_yaml_ng::Value::from("headers"),
                    serde_yaml_ng::Value::from(headers),
                );
            }
            map.insert(
                serde_yaml_ng::Value::from("ws-opts"),
                serde_yaml_ng::Value::from(ws_opts),
            );
        } else if network == "grpc" {
            let mut grpc_opts = serde_yaml_ng::Mapping::new();
            if let Some(p) = json.path {
                grpc_opts.insert(
                    serde_yaml_ng::Value::from("grpc-service-name"),
                    serde_yaml_ng::Value::from(p),
                );
            }
            map.insert(
                serde_yaml_ng::Value::from("grpc-opts"),
                serde_yaml_ng::Value::from(grpc_opts),
            );
        }
    }
    
    Some(map)
}

fn parse_ss(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("ss://")?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts.next().map(|r| {
        percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string()
    }).unwrap_or_else(|| "SS Node".to_string());
    
    let (method_pw, host_port) = if base_part.contains('@') {
        let (m_p, h_p) = base_part.split_once('@')?;
        (m_p.to_string(), h_p.to_string())
    } else {
        let decoded = decode_base64_robust(base_part)?;
        let decoded_str = String::from_utf8(decoded).ok()?;
        let (m_p, h_p) = decoded_str.split_once('@')?;
        (m_p.to_string(), h_p.to_string())
    };
    
    let decoded_method_pw = if !method_pw.contains(':') {
        let decoded = decode_base64_robust(&method_pw)?;
        String::from_utf8(decoded).ok()?
    } else {
        method_pw
    };
    
    let mut mp_parts = decoded_method_pw.splitn(2, ':');
    let cipher = mp_parts.next()?.trim().to_string();
    let password = mp_parts.next()?.trim().to_string();
    
    let mut hp_parts = host_port.splitn(2, '?');
    let server_port = hp_parts.next()?;
    
    let mut sp_parts = server_port.splitn(2, ':');
    let server = sp_parts.next()?.trim().to_string();
    let port_str = sp_parts.next()?.trim();
    let port = port_str.parse::<u16>().ok().unwrap_or(8388);
    
    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::from("type"),
        serde_yaml_ng::Value::from("ss"),
    );
    map.insert(
        serde_yaml_ng::Value::from("name"),
        serde_yaml_ng::Value::from(remarks),
    );
    map.insert(
        serde_yaml_ng::Value::from("server"),
        serde_yaml_ng::Value::from(server),
    );
    map.insert(
        serde_yaml_ng::Value::from("port"),
        serde_yaml_ng::Value::from(port),
    );
    map.insert(
        serde_yaml_ng::Value::from("cipher"),
        serde_yaml_ng::Value::from(cipher),
    );
    map.insert(
        serde_yaml_ng::Value::from("password"),
        serde_yaml_ng::Value::from(password),
    );
    map.insert(
        serde_yaml_ng::Value::from("udp"),
        serde_yaml_ng::Value::from(true),
    );
    
    Some(map)
}

fn parse_trojan(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("trojan://")?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts.next().map(|r| {
        percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string()
    }).unwrap_or_else(|| "Trojan Node".to_string());
    
    let mut subparts = base_part.splitn(2, '@');
    let password = subparts.next()?.to_string();
    let host_port_query = subparts.next()?;
    
    let mut hpq_parts = host_port_query.splitn(2, '?');
    let host_port = hpq_parts.next()?;
    let query = hpq_parts.next();
    
    let mut hp_parts = host_port.splitn(2, ':');
    let server = hp_parts.next()?.trim().to_string();
    let port_str = hp_parts.next()?.trim();
    let port = port_str.parse::<u16>().ok().unwrap_or(443);
    
    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::from("type"),
        serde_yaml_ng::Value::from("trojan"),
    );
    map.insert(
        serde_yaml_ng::Value::from("name"),
        serde_yaml_ng::Value::from(remarks),
    );
    map.insert(
        serde_yaml_ng::Value::from("server"),
        serde_yaml_ng::Value::from(server),
    );
    map.insert(
        serde_yaml_ng::Value::from("port"),
        serde_yaml_ng::Value::from(port),
    );
    map.insert(
        serde_yaml_ng::Value::from("password"),
        serde_yaml_ng::Value::from(password),
    );
    map.insert(
        serde_yaml_ng::Value::from("udp"),
        serde_yaml_ng::Value::from(true),
    );
    
    if let Some(q) = query {
        for pair in q.split('&') {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("").to_lowercase();
            let v = kv.next().unwrap_or("");
            if k == "sni" && !v.is_empty() {
                map.insert(
                    serde_yaml_ng::Value::from("sni"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
            } else if k == "allowinsecure" && (v == "1" || v.to_lowercase() == "true") {
                map.insert(
                    serde_yaml_ng::Value::from("skip-cert-verify"),
                    serde_yaml_ng::Value::from(true),
                );
            }
        }
    }
    
    Some(map)
}

pub fn parse_uri_list(content: &str) -> Option<Mapping> {
    let mut proxies = Vec::new();
    
    // Check if whole content is base64 encoded
    let decoded_content = if let Some(decoded) = decode_base64_robust(content) {
        String::from_utf8(decoded).unwrap_or_else(|_| content.to_string())
    } else {
        content.to_string()
    };
    
    for line in decoded_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        
        let proxy_map = if line.starts_with("vmess://") {
            parse_vmess(line)
        } else if line.starts_with("ss://") {
            parse_ss(line)
        } else if line.starts_with("trojan://") {
            parse_trojan(line)
        } else {
            None
        };
        
        if let Some(map) = proxy_map {
            proxies.push(serde_yaml_ng::Value::from(map));
        }
    }
    
    if proxies.is_empty() {
        return None;
    }
    
    let mut result = Mapping::new();
    result.insert(
        serde_yaml_ng::Value::from("proxies"),
        serde_yaml_ng::Value::from(proxies),
    );
    Some(result)
}
