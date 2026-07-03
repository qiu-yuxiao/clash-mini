use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_yaml_ng::Mapping;
use std::str;

fn extract_host_port(host_port: &str, default_port: u16) -> Option<(String, u16)> {
    let host_port = host_port.trim();
    if host_port.starts_with('[') {
        let close = host_port.find(']')?;
        let host = &host_port[1..close];
        let rest = &host_port[close + 1..];
        let port = rest
            .strip_prefix(':')
            .unwrap_or("")
            .parse::<u16>()
            .ok()
            .unwrap_or(default_port);
        Some((host.to_string(), port))
    } else {
        let mut parts = host_port.splitn(2, ':');
        let host = parts.next()?.trim().to_string();
        let port_str = parts.next().unwrap_or("").trim();
        let port = if port_str.is_empty() {
            default_port
        } else {
            port_str.parse::<u16>().ok().unwrap_or(default_port)
        };
        Some((host, port))
    }
}

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
    // Remove all whitespace characters including newlines
    let s_clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s_clean.is_empty() {
        return None;
    }

    // Try standard base64 first
    if let Ok(data) = STANDARD.decode(&s_clean) {
        return Some(data);
    }

    // Try URL safe base64
    if let Ok(data) = base64::engine::general_purpose::URL_SAFE.decode(&s_clean) {
        return Some(data);
    }

    // Try padding if length is not multiple of 4
    let mut padded = s_clean;
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

    let (server, port) = extract_host_port(server_port, 8388)?;

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

    let (server, port) = extract_host_port(host_port, 443)?;

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

    let (server, port) = extract_host_port(host_port, 443)?;

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
            } else if k == "alpn" {
                let alpn_list: Vec<String> = v
                    .split(',')
                    .map(|s| percent_encoding::percent_decode_str(s).decode_utf8_lossy().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !alpn_list.is_empty() {
                    map.insert(
                        serde_yaml_ng::Value::from("alpn"),
                        serde_yaml_ng::Value::from(alpn_list),
                    );
                }
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

    let (server, port) = extract_host_port(host_port, 443)?;

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

fn parse_tuic(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("tuic://")?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts
        .next()
        .map(|r| percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "TUIC Node".to_string());

    // tuic://uuid:password@host:port?...
    // or tuic://token@host:port?... (v4)
    let (user_info, host_port_query) = base_part.split_once('@')?;

    let (host_port, query) = host_port_query
        .split_once('?')
        .map_or((host_port_query, None as Option<&str>), |(hp, q)| (hp, Some(q)));

    let (server, port) = extract_host_port(host_port, 443)?;

    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(serde_yaml_ng::Value::from("type"), serde_yaml_ng::Value::from("tuic"));
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(remarks));
    map.insert(serde_yaml_ng::Value::from("server"), serde_yaml_ng::Value::from(server));
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

    // Percent-decode user_info (uuid:password or token may be encoded)
    let user_info_decoded = percent_encoding::percent_decode_str(user_info)
        .decode_utf8_lossy()
        .to_string();

    // Distinguish v4 (token) and v5 (uuid:password)
    if let Some((uuid, password)) = user_info_decoded.split_once(':') {
        // v5: uuid:password
        map.insert(
            serde_yaml_ng::Value::from("uuid"),
            serde_yaml_ng::Value::from(uuid.to_string()),
        );
        map.insert(
            serde_yaml_ng::Value::from("password"),
            serde_yaml_ng::Value::from(password.to_string()),
        );
    } else {
        // v4: token
        map.insert(
            serde_yaml_ng::Value::from("token"),
            serde_yaml_ng::Value::from(user_info_decoded),
        );
    }

    if let Some(q) = query {
        for pair in q.split('&') {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("").to_lowercase();
            let v = kv.next().unwrap_or("");
            if v.is_empty() {
                continue;
            }
            match k.as_str() {
                "congestion_control" | "congestion-controller" => {
                    map.insert(
                        serde_yaml_ng::Value::from("congestion-controller"),
                        serde_yaml_ng::Value::from(v.to_string()),
                    );
                }
                "alpn" => {
                    let alpn_list: Vec<String> = v
                        .split(',')
                        .map(|s| percent_encoding::percent_decode_str(s).decode_utf8_lossy().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !alpn_list.is_empty() {
                        map.insert(
                            serde_yaml_ng::Value::from("alpn"),
                            serde_yaml_ng::Value::from(alpn_list),
                        );
                    }
                }
                "sni" => {
                    map.insert(
                        serde_yaml_ng::Value::from("sni"),
                        serde_yaml_ng::Value::from(v.to_string()),
                    );
                }
                "allow_insecure" if v == "1" || v.to_lowercase() == "true" => {
                    map.insert(
                        serde_yaml_ng::Value::from("skip-cert-verify"),
                        serde_yaml_ng::Value::from(true),
                    );
                }
                "disable_sni" => {
                    map.insert(
                        serde_yaml_ng::Value::from("disable-sni"),
                        serde_yaml_ng::Value::from(v == "1" || v.to_lowercase() == "true"),
                    );
                }
                "udp_relay_mode" => {
                    map.insert(
                        serde_yaml_ng::Value::from("udp-relay-mode"),
                        serde_yaml_ng::Value::from(v.to_string()),
                    );
                }
                "reduce_rtt" => {
                    map.insert(
                        serde_yaml_ng::Value::from("reduce-rtt"),
                        serde_yaml_ng::Value::from(v == "1" || v.to_lowercase() == "true"),
                    );
                }
                "fast_open" => {
                    map.insert(
                        serde_yaml_ng::Value::from("fast-open"),
                        serde_yaml_ng::Value::from(v == "1" || v.to_lowercase() == "true"),
                    );
                }
                _ => {}
            }
        }
    }

    Some(map)
}

fn parse_wireguard(link: &str) -> Option<serde_yaml_ng::Mapping> {
    let payload = link.strip_prefix("wireguard://")?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts
        .next()
        .map(|r| percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "WireGuard Node".to_string());

    // wireguard://private-key@host:port?public-key=xxx&...
    // private-key may be base64 encoded
    let (private_key_raw, host_port_query) = base_part.split_once('@')?;

    let (host_port, query) = host_port_query
        .split_once('?')
        .map_or((host_port_query, None as Option<&str>), |(hp, q)| (hp, Some(q)));

    let (server, port) = extract_host_port(host_port, 51820)?;

    // Try base64 decode for private key, fallback to raw
    let private_key = if let Some(decoded) = decode_base64_robust(private_key_raw) {
        String::from_utf8(decoded).unwrap_or_else(|_| private_key_raw.to_string())
    } else {
        private_key_raw.to_string()
    };

    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::from("type"),
        serde_yaml_ng::Value::from("wireguard"),
    );
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(remarks));
    map.insert(serde_yaml_ng::Value::from("server"), serde_yaml_ng::Value::from(server));
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));
    map.insert(
        serde_yaml_ng::Value::from("private-key"),
        serde_yaml_ng::Value::from(private_key),
    );
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

    let mut public_key = String::new();
    let mut reserved_str = String::new();
    let mut address = String::new();
    let mut mtu_str = String::new();
    let mut psk = String::new();

    if let Some(q) = query {
        for pair in q.split('&') {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("").to_lowercase();
            let v = kv.next().unwrap_or("");
            if v.is_empty() {
                continue;
            }
            match k.as_str() {
                "public-key" | "publickey" | "pk" => {
                    public_key = percent_encoding::percent_decode_str(v).decode_utf8_lossy().to_string();
                }
                "reserved" => {
                    reserved_str = v.to_string();
                }
                "address" | "ip" => {
                    address = percent_encoding::percent_decode_str(v).decode_utf8_lossy().to_string();
                }
                "mtu" => {
                    mtu_str = v.to_string();
                }
                "preshared-key" | "pre-shared-key" | "psk" => {
                    psk = percent_encoding::percent_decode_str(v).decode_utf8_lossy().to_string();
                }
                _ => {}
            }
        }
    }

    if !public_key.is_empty() {
        map.insert(
            serde_yaml_ng::Value::from("public-key"),
            serde_yaml_ng::Value::from(public_key),
        );
    }

    if !reserved_str.is_empty() {
        // reserved can be "209,98,59" or base64 like "U4An"
        if reserved_str.contains(',') {
            let reserved: Vec<u16> = reserved_str
                .split(',')
                .filter_map(|s| s.trim().parse::<u16>().ok())
                .collect();
            if !reserved.is_empty() {
                map.insert(
                    serde_yaml_ng::Value::from("reserved"),
                    serde_yaml_ng::Value::from(reserved),
                );
            }
        } else if let Some(decoded) = decode_base64_robust(&reserved_str) {
            let reserved: Vec<u16> = decoded.iter().map(|&b| b as u16).collect();
            if !reserved.is_empty() {
                map.insert(
                    serde_yaml_ng::Value::from("reserved"),
                    serde_yaml_ng::Value::from(reserved),
                );
            }
        }
    }

    if !address.is_empty() {
        // address can be comma-separated ipv4,ipv6 with CIDR (e.g., "10.0.0.2/32,fd00::2/128")
        let addr_parts: Vec<&str> = address.split(',').map(|s| s.trim()).collect();

        // Strip CIDR and classify by address family
        let mut ipv4_addr: Option<String> = None;
        let mut ipv6_addrs: Vec<String> = Vec::new();

        for part in addr_parts {
            // Strip CIDR suffix if present (e.g., /32, /128)
            let ip_only = part.split_once('/').map_or(part, |(ip, _)| ip);

            if ip_only.contains(':') {
                // IPv6
                ipv6_addrs.push(ip_only.to_string());
            } else if ipv4_addr.is_none() {
                // IPv4 (take the first one only)
                ipv4_addr = Some(ip_only.to_string());
            }
        }

        if let Some(ipv4) = ipv4_addr {
            map.insert(serde_yaml_ng::Value::from("ip"), serde_yaml_ng::Value::from(ipv4));
        }

        if !ipv6_addrs.is_empty() {
            // Clash schema: ipv6 can be a single string or an array
            if ipv6_addrs.len() == 1 {
                map.insert(
                    serde_yaml_ng::Value::from("ipv6"),
                    serde_yaml_ng::Value::from(ipv6_addrs[0].clone()),
                );
            } else {
                map.insert(
                    serde_yaml_ng::Value::from("ipv6"),
                    serde_yaml_ng::Value::from(ipv6_addrs),
                );
            }
        }
    }

    if !mtu_str.is_empty() {
        if let Ok(mtu) = mtu_str.parse::<u16>() {
            map.insert(serde_yaml_ng::Value::from("mtu"), serde_yaml_ng::Value::from(mtu));
        }
    }

    if !psk.is_empty() {
        map.insert(
            serde_yaml_ng::Value::from("pre-shared-key"),
            serde_yaml_ng::Value::from(psk),
        );
    }

    // Default allowed-ips
    let allowed_ips = vec![serde_yaml_ng::Value::from("0.0.0.0/0")];
    map.insert(
        serde_yaml_ng::Value::from("allowed-ips"),
        serde_yaml_ng::Value::from(allowed_ips),
    );

    Some(map)
}

fn parse_socks5(link: &str) -> Option<serde_yaml_ng::Mapping> {
    // Support both socks5:// and socks://
    let payload = link
        .strip_prefix("socks5://")
        .or_else(|| link.strip_prefix("socks://"))?;
    let mut parts = payload.splitn(2, '#');
    let base_part = parts.next()?;
    let remarks = parts
        .next()
        .map(|r| percent_encoding::percent_decode_str(r).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "SOCKS5 Node".to_string());

    // socks5://[username:password@]host:port
    let (user_info, host_port) = if let Some(at_pos) = base_part.rfind('@') {
        (Some(&base_part[..at_pos]), &base_part[at_pos + 1..])
    } else {
        (None, base_part)
    };

    let (server, port) = extract_host_port(host_port, 1080)?;

    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(serde_yaml_ng::Value::from("type"), serde_yaml_ng::Value::from("socks5"));
    map.insert(serde_yaml_ng::Value::from("name"), serde_yaml_ng::Value::from(remarks));
    map.insert(serde_yaml_ng::Value::from("server"), serde_yaml_ng::Value::from(server));
    map.insert(serde_yaml_ng::Value::from("port"), serde_yaml_ng::Value::from(port));
    map.insert(serde_yaml_ng::Value::from("udp"), serde_yaml_ng::Value::from(true));

    if let Some(ui) = user_info {
        if let Some((username, password)) = ui.split_once(':') {
            map.insert(
                serde_yaml_ng::Value::from("username"),
                serde_yaml_ng::Value::from(
                    percent_encoding::percent_decode_str(username)
                        .decode_utf8_lossy()
                        .to_string(),
                ),
            );
            map.insert(
                serde_yaml_ng::Value::from("password"),
                serde_yaml_ng::Value::from(
                    percent_encoding::percent_decode_str(password)
                        .decode_utf8_lossy()
                        .to_string(),
                ),
            );
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
        } else if line.starts_with("tuic://") {
            parse_tuic(line)
        } else if line.starts_with("wireguard://") {
            parse_wireguard(line)
        } else if line.starts_with("socks5://") || line.starts_with("socks://") {
            parse_socks5(line)
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

    #[test]
    fn test_decode_base64_robust_newlines() {
        let s = "aGVsbG8gd29ybGQKaGVs\nbG8gd29ybGQ=";
        let decoded = decode_base64_robust(s).unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        assert_eq!(decoded_str, "hello world\nhello world");
    }

    #[test]
    fn test_extract_host_port_ipv4() {
        let (host, port) = extract_host_port("example.com:8443", 443).unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 8443);
    }

    #[test]
    fn test_extract_host_port_ipv6() {
        let (host, port) = extract_host_port("[2401:c080:1000:29ac:5400:6ff:fe43:9d48]:443", 443).unwrap();
        assert_eq!(host, "2401:c080:1000:29ac:5400:6ff:fe43:9d48");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_extract_host_port_ipv6_custom_port() {
        let (host, port) = extract_host_port("[::1]:8443", 443).unwrap();
        assert_eq!(host, "::1");
        assert_eq!(port, 8443);
    }

    #[test]
    fn test_parse_vless_ipv6() {
        let link = "vless://28beee5e-40ab-3c36-b797-1e4ecf27d0a0@[2401:c080:1000:29ac:5400:6ff:fe43:9d48]:443?encryption=none&type=tcp&fp=ios&host=s3611.wagahaha.xyz&flow=xtls-rprx-vision&security=tls&sni=u729792us3611.wagahaha.xyz&alpn=h2,http/1.1#%E6%9C%AA%E7%9F%A5%20VLESS-160";
        let map = parse_vless(link).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("type")).unwrap().as_str().unwrap(),
            "vless"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("name")).unwrap().as_str().unwrap(),
            "未知 VLESS-160"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "2401:c080:1000:29ac:5400:6ff:fe43:9d48"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            443
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("uuid")).unwrap().as_str().unwrap(),
            "28beee5e-40ab-3c36-b797-1e4ecf27d0a0"
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
            "u729792us3611.wagahaha.xyz"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("client-fingerprint"))
                .unwrap()
                .as_str()
                .unwrap(),
            "ios"
        );
        let alpn = map
            .get(serde_yaml_ng::Value::from("alpn"))
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(alpn.len(), 2);
        assert_eq!(alpn[0].as_str().unwrap(), "h2");
        assert_eq!(alpn[1].as_str().unwrap(), "http/1.1");
    }

    #[test]
    fn test_parse_trojan_ipv6() {
        let link = "trojan://password@[2001:db8::1]:443?sni=example.com#test";
        let map = parse_trojan(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "2001:db8::1"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            443
        );
    }

    #[test]
    fn test_parse_hysteria2_ipv6() {
        let link = "hysteria2://password@[2001:db8::1]:8443?sni=example.com#test";
        let map = parse_hysteria2(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "2001:db8::1"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            8443
        );
    }

    #[test]
    fn test_parse_tuic_v5() {
        let link = "tuic://00000000-0000-0000-0000-000000000001:password@example.com:10443?congestion_control=cubic&alpn=h3&sni=example.com&allow_insecure=0&udp_relay_mode=native#test-tuic";
        let map = parse_tuic(link).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("type")).unwrap().as_str().unwrap(),
            "tuic"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("name")).unwrap().as_str().unwrap(),
            "test-tuic"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "example.com"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            10443
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("uuid")).unwrap().as_str().unwrap(),
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("password"))
                .unwrap()
                .as_str()
                .unwrap(),
            "password"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("congestion-controller"))
                .unwrap()
                .as_str()
                .unwrap(),
            "cubic"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("udp-relay-mode"))
                .unwrap()
                .as_str()
                .unwrap(),
            "native"
        );
        let alpn = map
            .get(serde_yaml_ng::Value::from("alpn"))
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(alpn.len(), 1);
        assert_eq!(alpn[0].as_str().unwrap(), "h3");
    }

    #[test]
    fn test_parse_tuic_v4() {
        let link = "tuic://TOKEN@example.com:443?alpn=h3&udp_relay_mode=quic#v4-node";
        let map = parse_tuic(link).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("type")).unwrap().as_str().unwrap(),
            "tuic"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("token")).unwrap().as_str().unwrap(),
            "TOKEN"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("udp-relay-mode"))
                .unwrap()
                .as_str()
                .unwrap(),
            "quic"
        );
    }

    #[test]
    fn test_parse_tuic_ipv6() {
        let link = "tuic://uuid:password@[2001:db8::1]:443?sni=example.com#tuic-ipv6";
        let map = parse_tuic(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "2001:db8::1"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            443
        );
    }

    #[test]
    fn test_parse_wireguard() {
        let link = "wireguard://eCtXsJZ27+4PbhDkHnB923tkUn2Gj59wZw5wFA75MnU=@162.159.192.1:2480?public-key=Cr8hWlKvtDt7nrvf+f0brNQQzabAqrjfBvas9pmowjo=&address=172.16.0.2&mtu=1408#wg-test";
        let map = parse_wireguard(link).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("type")).unwrap().as_str().unwrap(),
            "wireguard"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("name")).unwrap().as_str().unwrap(),
            "wg-test"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "162.159.192.1"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            2480
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("public-key"))
                .unwrap()
                .as_str()
                .unwrap(),
            "Cr8hWlKvtDt7nrvf+f0brNQQzabAqrjfBvas9pmowjo="
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("ip")).unwrap().as_str().unwrap(),
            "172.16.0.2"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("mtu")).unwrap().as_u64().unwrap(),
            1408
        );
        let allowed = map
            .get(serde_yaml_ng::Value::from("allowed-ips"))
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].as_str().unwrap(), "0.0.0.0/0");
    }

    #[test]
    fn test_parse_wireguard_ipv6() {
        let link = "wireguard://eCtXsJZ27+4PbhDkHnB923tkUn2Gj59wZw5wFA75MnU=@[2001:db8::1]:51820?public-key=Cr8hWlKvtDt7nrvf+f0brNQQzabAqrjfBvas9pmowjo=&address=172.16.0.2#wg-ipv6";
        let map = parse_wireguard(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "2001:db8::1"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            51820
        );
    }

    #[test]
    fn test_parse_socks5_no_auth() {
        let link = "socks5://example.com:1080#socks-test";
        let map = parse_socks5(link).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("type")).unwrap().as_str().unwrap(),
            "socks5"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("name")).unwrap().as_str().unwrap(),
            "socks-test"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "example.com"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            1080
        );
    }

    #[test]
    fn test_parse_socks5_with_auth() {
        let link = "socks5://user:pass@example.com:1080#auth-socks";
        let map = parse_socks5(link).unwrap();

        assert_eq!(
            map.get(serde_yaml_ng::Value::from("username"))
                .unwrap()
                .as_str()
                .unwrap(),
            "user"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("password"))
                .unwrap()
                .as_str()
                .unwrap(),
            "pass"
        );
    }

    #[test]
    fn test_parse_socks5_ipv6() {
        let link = "socks5://[2001:db8::1]:1080#ipv6-socks";
        let map = parse_socks5(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("server")).unwrap().as_str().unwrap(),
            "2001:db8::1"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("port")).unwrap().as_u64().unwrap(),
            1080
        );
    }

    // New tests for bug fixes

    #[test]
    fn test_parse_tuic_percent_encoded() {
        // UUID with percent-encoded characters: %3D should decode to '='
        let link = "tuic://uuid%3Dtest:pass%40word@example.com:443#encoded-tuic";
        let map = parse_tuic(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("uuid")).unwrap().as_str().unwrap(),
            "uuid=test"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("password"))
                .unwrap()
                .as_str()
                .unwrap(),
            "pass@word"
        );
    }

    #[test]
    fn test_parse_wireguard_address_with_cidr() {
        // Address with CIDR suffix should be stripped
        let link = "wireguard://key@server:51820?address=10.0.0.2/32,fd00::2/128#wg-cidr";
        let map = parse_wireguard(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("ip")).unwrap().as_str().unwrap(),
            "10.0.0.2"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("ipv6")).unwrap().as_str().unwrap(),
            "fd00::2"
        );
    }

    #[test]
    fn test_parse_wireguard_ipv6_first() {
        // IPv6 before IPv4 in address list - should still classify correctly
        let link = "wireguard://key@server:51820?address=fd00::1/64,10.0.0.1/24#wg-ipv6-first";
        let map = parse_wireguard(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("ip")).unwrap().as_str().unwrap(),
            "10.0.0.1"
        );
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("ipv6")).unwrap().as_str().unwrap(),
            "fd00::1"
        );
    }

    #[test]
    fn test_parse_wireguard_multiple_ipv6() {
        // Multiple IPv6 addresses - should be stored as array
        let link = "wireguard://key@server:51820?address=10.0.0.1/32,fd00::1/64,fd01::2/64#wg-multi-ipv6";
        let map = parse_wireguard(link).unwrap();
        assert_eq!(
            map.get(serde_yaml_ng::Value::from("ip")).unwrap().as_str().unwrap(),
            "10.0.0.1"
        );
        let ipv6_list = map
            .get(serde_yaml_ng::Value::from("ipv6"))
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(ipv6_list.len(), 2);
        assert_eq!(ipv6_list[0].as_str().unwrap(), "fd00::1");
        assert_eq!(ipv6_list[1].as_str().unwrap(), "fd01::2");
    }
}
