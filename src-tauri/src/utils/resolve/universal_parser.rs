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
    map.insert(serde_yaml_ng::Value::from("type"), serde_yaml_ng::Value::from("vmess"));
    map.insert(
        serde_yaml_ng::Value::from("name"),
        serde_yaml_ng::Value::from(json.ps.unwrap_or_else(|| "VMess Node".to_string())),
    );
    map.insert(
        serde_yaml_ng::Value::from("server"),
        serde_yaml_ng::Value::from(json.add.unwrap_or_default()),
    );

    let port = json
        .port
        .and_then(|p| match p {
            serde_json::Value::Number(n) => n.as_u64().map(|v| v as u16),
            serde_json::Value::String(s) => s.parse::<u16>().ok(),
            _ => None,
        })
        .unwrap_or(443);
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));

    map.insert(
        serde_yaml_ng::Value::from("uuid"),
        serde_yaml_ng::Value::from(json.id.unwrap_or_default()),
    );

    let alter_id = json
        .aid
        .and_then(|a| match a {
            serde_json::Value::Number(n) => n.as_u64().map(|v| v as u32),
            serde_json::Value::String(s) => s.parse::<u32>().ok(),
            _ => None,
        })
        .unwrap_or(0);
    map.insert(
        serde_yaml_ng::Value::from("alterId"),
        serde_yaml_ng::Value::from(alter_id),
    );

    map.insert(serde_yaml_ng::Value::from("cipher"), serde_yaml_ng::Value::from("auto"));
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

    if json.tls.as_deref() == Some("tls") {
        map.insert(serde_yaml_ng::Value::from("tls"), serde_yaml_ng::Value::from(true));
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
                ws_opts.insert(serde_yaml_ng::Value::from("path"), serde_yaml_ng::Value::from(p));
            }
            if let Some(h) = json.host.filter(|h| !h.is_empty()) {
                let mut headers = serde_yaml_ng::Mapping::new();
                headers.insert(serde_yaml_ng::Value::from("Host"), serde_yaml_ng::Value::from(h));
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
    let remarks = parts
        .next()
        .map(|r| percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "SS Node".to_string());

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
    map.insert(serde_yaml_ng::Value::from("type"), serde_yaml_ng::Value::from("ss"));
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(remarks));
    map.insert(serde_yaml_ng::Value::from("server"), serde_yaml_ng::Value::from(server));
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));
    map.insert(serde_yaml_ng::Value::from("cipher"), serde_yaml_ng::Value::from(cipher));
    map.insert(
        serde_yaml_ng::Value::from("password"),
        serde_yaml_ng::Value::from(password),
    );
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

    Some(map)
}

fn parse_trojan(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("trojan://")?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts
        .next()
        .map(|r| percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "Trojan Node".to_string());

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
    map.insert(serde_yaml_ng::Value::from("type"), serde_yaml_ng::Value::from("trojan"));
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(remarks));
    map.insert(serde_yaml_ng::Value::from("server"), serde_yaml_ng::Value::from(server));
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));
    map.insert(
        serde_yaml_ng::Value::from("password"),
        serde_yaml_ng::Value::from(password),
    );
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

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

fn parse_vless(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("vless://")?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts
        .next()
        .map(|r| percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "VLESS Node".to_string());

    let mut subparts = base_part.splitn(2, '@');
    let uuid = subparts.next()?.to_string();
    let host_port_query = subparts.next()?;

    let mut hpq_parts = host_port_query.splitn(2, '?');
    let host_port = hpq_parts.next()?;
    let query = hpq_parts.next();

    let mut hp_parts = host_port.splitn(2, ':');
    let server = hp_parts.next()?.trim().to_string();
    let port_str = hp_parts.next()?.trim();
    let port = port_str.parse::<u16>().ok().unwrap_or(443);

    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(serde_yaml_ng::Value::from("type"), serde_yaml_ng::Value::from("vless"));
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(remarks));
    map.insert(serde_yaml_ng::Value::from("server"), serde_yaml_ng::Value::from(server));
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));
    map.insert(serde_yaml_ng::Value::from("uuid"), serde_yaml_ng::Value::from(uuid));
    map.insert(serde_yaml_ng::Value::from("cipher"), serde_yaml_ng::Value::from("auto"));
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

    // Parse query parameters
    if let Some(q) = query {
        let mut reality_opts = serde_yaml_ng::Mapping::new();
        let mut has_reality = false;
        let mut network_type = String::new();
        let mut path_str = String::new();
        let mut host_str = String::new();

        for pair in q.split('&') {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("").to_lowercase();
            let v = kv.next().unwrap_or("");
            if v.is_empty() {
                continue;
            }
            if k == "flow" {
                map.insert(
                    serde_yaml_ng::Value::from("flow"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
            } else if k == "security" {
                if v == "tls" || v == "reality" {
                    map.insert(serde_yaml_ng::Value::from("tls"), serde_yaml_ng::Value::from(true));
                }
            } else if k == "sni" {
                map.insert(
                    serde_yaml_ng::Value::from("servername"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
            } else if k == "type" {
                network_type = v.to_string();
            } else if k == "pbk" {
                reality_opts.insert(
                    serde_yaml_ng::Value::from("public-key"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
                has_reality = true;
            } else if k == "sid" {
                reality_opts.insert(
                    serde_yaml_ng::Value::from("short-id"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
                has_reality = true;
            } else if k == "fp" {
                map.insert(
                    serde_yaml_ng::Value::from("client-fingerprint"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
            } else if k == "path" {
                path_str = percent_encoding::percent_decode_str(v).decode_utf8_lossy().to_string();
            } else if k == "host" {
                host_str = percent_encoding::percent_decode_str(v).decode_utf8_lossy().to_string();
            }
        }

        if has_reality {
            map.insert(
                serde_yaml_ng::Value::from("reality-opts"),
                serde_yaml_ng::Value::from(reality_opts),
            );
        }

        if !network_type.is_empty() {
            let net_val = if network_type == "raw" {
                "tcp".to_string()
            } else {
                network_type
            };
            map.insert(
                serde_yaml_ng::Value::from("network"),
                serde_yaml_ng::Value::from(net_val.clone()),
            );
            if net_val == "ws" {
                let mut ws_opts = serde_yaml_ng::Mapping::new();
                if !path_str.is_empty() {
                    ws_opts.insert(serde_yaml_ng::Value::from("path"), serde_yaml_ng::Value::from(path_str));
                }
                if !host_str.is_empty() {
                    let mut headers = serde_yaml_ng::Mapping::new();
                    headers.insert(serde_yaml_ng::Value::from("Host"), serde_yaml_ng::Value::from(host_str));
                    ws_opts.insert(
                        serde_yaml_ng::Value::from("headers"),
                        serde_yaml_ng::Value::from(headers),
                    );
                }
                map.insert(
                    serde_yaml_ng::Value::from("ws-opts"),
                    serde_yaml_ng::Value::from(ws_opts),
                );
            } else if net_val == "grpc" {
                let mut grpc_opts = serde_yaml_ng::Mapping::new();
                if !path_str.is_empty() {
                    grpc_opts.insert(
                        serde_yaml_ng::Value::from("grpc-service-name"),
                        serde_yaml_ng::Value::from(path_str),
                    );
                }
                map.insert(
                    serde_yaml_ng::Value::from("grpc-opts"),
                    serde_yaml_ng::Value::from(grpc_opts),
                );
            }
        }
    }

    Some(map)
}

fn parse_hysteria2(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("hysteria2://")?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts
        .next()
        .map(|r| percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "Hysteria2 Node".to_string());

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
        serde_yaml_ng::Value::from("hysteria2"),
    );
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(remarks));
    map.insert(serde_yaml_ng::Value::from("server"), serde_yaml_ng::Value::from(server));
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));
    map.insert(
        serde_yaml_ng::Value::from("password"),
        serde_yaml_ng::Value::from(password),
    );
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

    // Parse query parameters
    if let Some(q) = query {
        for pair in q.split('&') {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("").to_lowercase();
            let v = kv.next().unwrap_or("");
            if v.is_empty() {
                continue;
            }
            if k == "insecure" && (v == "1" || v.to_lowercase() == "true") {
                map.insert(
                    serde_yaml_ng::Value::from("skip-cert-verify"),
                    serde_yaml_ng::Value::from(true),
                );
            } else if k == "sni" {
                map.insert(
                    serde_yaml_ng::Value::from("sni"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
            } else if k == "obfs" {
                map.insert(
                    serde_yaml_ng::Value::from("obfs"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
            } else if k == "obfs-password" {
                map.insert(
                    serde_yaml_ng::Value::from("obfs-password"),
                    serde_yaml_ng::Value::from(v.to_string()),
                );
            }
        }
    }

    Some(map)
}

fn parse_http_fallback(line: &str) -> Option<serde_yaml_ng::Mapping> {
    let line = line.trim().strip_prefix('#').unwrap_or(line).trim();
    let payload = line.strip_prefix("[HTTP]")?.trim();

    let mut parts = payload.splitn(2, " (");
    let name = parts.next()?.trim().to_string();
    let addr_port_part = parts.next()?;
    let addr_port = addr_port_part.strip_suffix(')')?.trim();

    let (server, port_str) = addr_port.split_once(':')?;
    let port = port_str.parse::<u16>().ok().unwrap_or(80);

    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(serde_yaml_ng::Value::from("type"), serde_yaml_ng::Value::from("http"));
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(name));
    map.insert(
        serde_yaml_ng::Value::from("server"),
        serde_yaml_ng::Value::from(server.trim().to_string()),
    );
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));

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
        if line.is_empty() {
            continue;
        }

        // Check for smart http fallback even if it starts with #
        if let Some(map) = parse_http_fallback(line) {
            proxies.push(serde_yaml_ng::Value::from(map));
            continue;
        }

        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        let proxy_map = if line.starts_with("vmess://") {
            parse_vmess(line)
        } else if line.starts_with("ss://") {
            parse_ss(line)
        } else if line.starts_with("trojan://") {
            parse_trojan(line)
        } else if line.starts_with("vless://") {
            parse_vless(line)
        } else if line.starts_with("hysteria2://") {
            parse_hysteria2(line)
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vless_reality() {
        let link = "vless://2513e61a-6982-39cd-9f0a-8f6fbc3fc5f0@awsjfde1.fatdns.net:443?encryption=none&flow=xtls-rprx-vision&security=reality&type=tcp&sni=s0.awsstatic.com&pbk=QzUWMFvn-J14NCZGsn5pz30inE7VT9auYKRwaRr5T20&sid=00fd4e9ac051c144&fp=chrome#aws%E5%BE%B7%E5%9B%BD";
        let map = parse_vless(link).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("type")).unwrap().as_str().unwrap(),
            "vless"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("name")).unwrap().as_str().unwrap(),
            "aws德国"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "awsjfde1.fatdns.net"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            443
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("uuid")).unwrap().as_str().unwrap(),
            "2513e61a-6982-39cd-9f0a-8f6fbc3fc5f0"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("flow")).unwrap().as_str().unwrap(),
            "xtls-rprx-vision"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("tls")).unwrap().as_bool().unwrap(),
            true
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("servername"))
                .unwrap()
                .as_str()
                .unwrap(),
            "s0.awsstatic.com"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("client-fingerprint"))
                .unwrap()
                .as_str()
                .unwrap(),
            "chrome"
        );

        let reality_opts = map
            .get(serde_yaml_ng::Value::from("reality-opts"))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(
            reality_opts
                .get(serde_yaml_ng::Value::from("public-key"))
                .unwrap()
                .as_str()
                .unwrap(),
            "QzUWMFvn-J14NCZGsn5pz30inE7VT9auYKRwaRr5T20"
        );
        assert_eq!(
            reality_opts
                .get(serde_yaml_ng::Value::from("short-id"))
                .unwrap()
                .as_str()
                .unwrap(),
            "00fd4e9ac051c144"
        );
    }

    #[test]
    fn test_parse_http_fallback() {
        let line = "# [HTTP] v5-香港03|1x|v (v5.cdn.ljz8s7lhbv.fatdns.net:6712)";
        let map = parse_http_fallback(line).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("type")).unwrap().as_str().unwrap(),
            "http"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("name")).unwrap().as_str().unwrap(),
            "v5-香港03|1x|v"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "v5.cdn.ljz8s7lhbv.fatdns.net"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            6712
        );
    }
}
