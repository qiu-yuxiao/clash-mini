use super::{
    TransportOpts, apply_transport_opts, decode_base64_robust, extract_host_port, insert_alpn_list, is_truthy,
    normalize_bandwidth, parse_query_pairs, percent_decode, strip_fragment_and_query, strip_prefix_ascii_case,
};
use serde_yaml_ng::{Mapping, Value};

pub(super) fn parse_vless(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "vless://")?.trim();
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "VLESS Node".to_string());

    let (uuid_raw, host_port) = auth_host.rsplit_once('@')?;
    let host_port = host_port.trim_end_matches('/');
    let uuid = percent_decode(uuid_raw);

    let (server, port) = extract_host_port(host_port, 443)?;

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("vless"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("uuid"), Value::from(uuid));
    map.insert(Value::from("cipher"), Value::from("auto"));
    map.insert(Value::from("udp"), Value::from(true));

    if let Some(q) = query {
        let mut reality_opts = Mapping::new();
        let mut transport = TransportOpts::default();

        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "flow" => {
                    map.insert(Value::from("flow"), Value::from(percent_decode(&v)));
                }
                "security" if v.eq_ignore_ascii_case("tls") || v.eq_ignore_ascii_case("reality") => {
                    map.insert(Value::from("tls"), Value::from(true));
                }
                "sni" | "peer" => {
                    map.insert(Value::from("servername"), Value::from(percent_decode(&v)));
                }
                "allowinsecure" | "insecure" | "allow_insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                "encryption" if !v.eq_ignore_ascii_case("none") => {
                    map.insert(Value::from("encryption"), Value::from(percent_decode(&v)));
                }
                "pbk" => {
                    reality_opts.insert(Value::from("public-key"), Value::from(percent_decode(&v)));
                }
                "sid" => {
                    reality_opts.insert(Value::from("short-id"), Value::from(percent_decode(&v)));
                }
                "fp" | "fingerprint" => {
                    map.insert(Value::from("client-fingerprint"), Value::from(percent_decode(&v)));
                }
                "alpn" => insert_alpn_list(&mut map, &v),
                "type" => transport.network = percent_decode(&v),
                "headertype" => transport.header_type = percent_decode(&v),
                "path" => transport.path = percent_decode(&v),
                "host" => transport.host = percent_decode(&v),
                "servicename" | "service-name" | "service_name" => transport.service_name = percent_decode(&v),
                "mode" => transport.mode = percent_decode(&v),
                "extra" => transport.extra = percent_decode(&v),
                _ => {}
            }
        }

        if !reality_opts.is_empty() {
            map.insert(Value::from("reality-opts"), Value::from(reality_opts));
        }
        apply_transport_opts(&mut map, &transport, false);
    }

    Some(map)
}

fn first_port_from_spec(port_spec: &str) -> Option<u16> {
    let first_seg = port_spec.split([',', '/']).next()?.trim();
    let first_num = first_seg.split('-').next()?.trim();
    first_num.parse::<u16>().ok()
}

fn extract_hy2_host_port(host_port: &str) -> Option<(String, u16, Option<String>)> {
    let trimmed = host_port.trim();
    let (host, port_part) = if trimmed.starts_with('[') {
        let close = trimmed.find(']')?;
        let h = trimmed[1..close].trim();
        let rest = trimmed[close + 1..].trim();
        (h, rest.strip_prefix(':').unwrap_or("").trim())
    } else if let Some((h, p)) = trimmed.split_once(':') {
        (h.trim(), p.trim())
    } else {
        (trimmed, "")
    };
    if host.is_empty() {
        return None;
    }
    if port_part.contains('-') || port_part.contains(',') || port_part.contains('/') {
        let port = first_port_from_spec(port_part).unwrap_or(443);
        let normalized_ports = port_part.replace('/', ",");
        Some((host.to_string(), port, Some(normalized_ports)))
    } else {
        let port = if port_part.is_empty() {
            443
        } else {
            port_part.parse::<u16>().ok().unwrap_or(443)
        };
        Some((host.to_string(), port, None))
    }
}

pub(super) fn parse_hysteria2(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "hysteria2://")
        .or_else(|| strip_prefix_ascii_case(link, "hy2://"))?
        .trim();
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "Hysteria2 Node".to_string());

    let (password_raw, host_port) = auth_host.rsplit_once('@')?;
    let host_port = host_port.trim_end_matches('/');
    let password = percent_decode(password_raw);

    let (server, port, authority_ports) = extract_hy2_host_port(host_port)?;

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("hysteria2"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    if let Some(ports_str) = authority_ports {
        map.insert(Value::from("ports"), Value::from(ports_str));
    }
    map.insert(Value::from("password"), Value::from(password));
    map.insert(Value::from("udp"), Value::from(true));

    if let Some(q) = query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "insecure" | "allowinsecure" | "allow_insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                "sni" | "peer" => {
                    map.insert(Value::from("sni"), Value::from(percent_decode(&v)));
                }
                "obfs" if !v.eq_ignore_ascii_case("none") => {
                    map.insert(Value::from("obfs"), Value::from(percent_decode(&v)));
                }
                "obfs-password" | "obfs_password" | "obfsparam" => {
                    map.insert(Value::from("obfs-password"), Value::from(percent_decode(&v)));
                }
                "mport" | "ports" => {
                    let ports = percent_decode(&v).replace('/', ",");
                    let ports = ports.trim();
                    if !ports.is_empty() {
                        map.insert(Value::from("ports"), Value::from(ports));
                    }
                }
                "hop-interval" | "hop_interval" => {
                    let trimmed = v.trim();
                    let secs_str = trimmed.strip_suffix('s').unwrap_or(trimmed);
                    if let Ok(secs) = secs_str.parse::<u64>() {
                        map.insert(Value::from("hop-interval"), Value::from(secs));
                    }
                }
                "pinsha256" | "pin-sha256" | "fingerprint" => {
                    map.insert(Value::from("fingerprint"), Value::from(percent_decode(&v)));
                }
                "alpn" => insert_alpn_list(&mut map, &v),
                "up" | "upmbps" => {
                    map.insert(Value::from("up"), Value::from(normalize_bandwidth(&percent_decode(&v))));
                }
                "down" | "downmbps" => {
                    map.insert(
                        Value::from("down"),
                        Value::from(normalize_bandwidth(&percent_decode(&v))),
                    );
                }
                _ => {}
            }
        }
    }

    Some(map)
}

pub(super) fn parse_hysteria(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "hysteria://")?.trim();
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "Hysteria Node".to_string());

    let (host_port, userinfo) = match auth_host.rsplit_once('@') {
        Some((user, hp)) => (hp.trim_end_matches('/'), Some(user)),
        None => (auth_host.trim_end_matches('/'), None),
    };

    let (server, port) = extract_host_port(host_port, 443)?;
    if server.is_empty() {
        return None;
    }

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("hysteria"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));

    let mut auth_str: Option<String> = userinfo.map(percent_decode).filter(|s| !s.is_empty());
    let mut obfs_param: Option<String> = None;
    let mut obfs_plain: Option<String> = None;

    if let Some(q) = query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "auth" | "auth_str" | "auth-str" => auth_str = Some(percent_decode(&v)),
                "peer" | "sni" => {
                    map.insert(Value::from("sni"), Value::from(percent_decode(&v)));
                }
                "up" | "upmbps" => {
                    map.insert(Value::from("up"), Value::from(normalize_bandwidth(&percent_decode(&v))));
                }
                "down" | "downmbps" => {
                    map.insert(
                        Value::from("down"),
                        Value::from(normalize_bandwidth(&percent_decode(&v))),
                    );
                }
                "protocol" => {
                    let proto = percent_decode(&v);
                    if matches!(proto.as_str(), "udp" | "wechat-video" | "faketcp") {
                        map.insert(Value::from("protocol"), Value::from(proto));
                    }
                }
                "insecure" | "allowinsecure" | "allow_insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                "alpn" => insert_alpn_list(&mut map, &v),
                "obfsparam" | "obfs-password" | "obfs_password" => obfs_param = Some(percent_decode(&v)),
                "obfs" => obfs_plain = Some(percent_decode(&v)),
                _ => {}
            }
        }
    }

    if let Some(v) = auth_str {
        map.insert(Value::from("auth-str"), Value::from(v));
    }
    if let Some(v) = obfs_param.or(obfs_plain) {
        map.insert(Value::from("obfs"), Value::from(v));
    }

    Some(map)
}

pub(super) fn parse_anytls(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "anytls://")?.trim();
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "AnyTLS Node".to_string());

    let (password_raw, host_port) = auth_host.rsplit_once('@')?;
    let host_port = host_port.trim_end_matches('/');
    let password = percent_decode(password_raw);
    if password.is_empty() {
        return None;
    }

    let (server, port) = extract_host_port(host_port, 443)?;
    if server.is_empty() {
        return None;
    }

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("anytls"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("password"), Value::from(password));
    map.insert(Value::from("udp"), Value::from(true));

    if let Some(q) = query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "sni" | "peer" => {
                    map.insert(Value::from("sni"), Value::from(percent_decode(&v)));
                }
                "insecure" | "allowinsecure" | "allow_insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                "alpn" => insert_alpn_list(&mut map, &v),
                "fp" | "fingerprint" => {
                    map.insert(Value::from("client-fingerprint"), Value::from(percent_decode(&v)));
                }
                _ => {}
            }
        }
    }

    Some(map)
}

pub(super) fn parse_tuic(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "tuic://")?.trim();
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "TUIC Node".to_string());

    let (user_info, host_port) = auth_host.rsplit_once('@')?;
    let host_port = host_port.trim_end_matches('/');
    let (server, port) = extract_host_port(host_port, 443)?;

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("tuic"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("udp"), Value::from(true));

    let user_info_decoded = percent_decode(user_info);
    if let Some((uuid, password)) = user_info_decoded.split_once(':') {
        map.insert(Value::from("uuid"), Value::from(uuid));
        map.insert(Value::from("password"), Value::from(password));
    } else {
        map.insert(Value::from("token"), Value::from(user_info_decoded));
    }

    if let Some(q) = query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "congestion_control" | "congestion-controller" => {
                    map.insert(Value::from("congestion-controller"), Value::from(percent_decode(&v)));
                }
                "alpn" => insert_alpn_list(&mut map, &v),
                "sni" | "peer" => {
                    map.insert(Value::from("sni"), Value::from(percent_decode(&v)));
                }
                "allow_insecure" | "allowinsecure" | "insecure" if is_truthy(&v) => {
                    map.insert(Value::from("skip-cert-verify"), Value::from(true));
                }
                "disable_sni" => {
                    map.insert(Value::from("disable-sni"), Value::from(is_truthy(&v)));
                }
                "udp_relay_mode" => {
                    map.insert(Value::from("udp-relay-mode"), Value::from(percent_decode(&v)));
                }
                "reduce_rtt" => {
                    map.insert(Value::from("reduce-rtt"), Value::from(is_truthy(&v)));
                }
                "fast_open" => {
                    map.insert(Value::from("fast-open"), Value::from(is_truthy(&v)));
                }
                _ => {}
            }
        }
    }

    Some(map)
}

pub(super) fn apply_wireguard_addresses(map: &mut Mapping, addrs: &[String]) {
    let mut ipv4_addr: Option<String> = None;
    let mut ipv6_addrs: Vec<String> = Vec::new();

    for part in addrs {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let ip_only = trimmed.split_once('/').map_or(trimmed, |(ip, _)| ip).trim();
        if ip_only.contains(':') {
            ipv6_addrs.push(ip_only.to_string());
        } else if ipv4_addr.is_none() {
            ipv4_addr = Some(ip_only.to_string());
        }
    }

    if let Some(ipv4) = ipv4_addr {
        map.insert(Value::from("ip"), Value::from(ipv4));
    }
    let has_ipv6 = !ipv6_addrs.is_empty();
    if ipv6_addrs.len() == 1 {
        map.insert(Value::from("ipv6"), Value::from(ipv6_addrs[0].as_str()));
    } else if has_ipv6 {
        map.insert(Value::from("ipv6"), Value::from(ipv6_addrs));
    }

    let mut allowed_ips = vec![Value::from("0.0.0.0/0")];
    if has_ipv6 {
        allowed_ips.push(Value::from("::/0"));
    }
    map.insert(Value::from("allowed-ips"), Value::from(allowed_ips));
}

fn parse_wireguard_reserved(map: &mut Mapping, reserved_str: &str) {
    if reserved_str.contains(',') {
        let reserved: Vec<u16> = reserved_str
            .split(',')
            .filter_map(|s| s.trim().parse::<u16>().ok())
            .collect();
        if !reserved.is_empty() {
            map.insert(Value::from("reserved"), Value::from(reserved));
        }
    } else if let Some(decoded) = decode_base64_robust(reserved_str) {
        let reserved: Vec<u16> = decoded.iter().map(|&b| u16::from(b)).collect();
        if !reserved.is_empty() {
            map.insert(Value::from("reserved"), Value::from(reserved));
        }
    }
}

pub(super) fn parse_wireguard(link: &str) -> Option<Mapping> {
    let payload = strip_prefix_ascii_case(link, "wireguard://").or_else(|| strip_prefix_ascii_case(link, "wg://"))?;
    let (auth_host, query, frag) = strip_fragment_and_query(payload);
    let remarks = frag.unwrap_or_else(|| "WireGuard Node".to_string());

    let (private_key_raw, host_port) = auth_host.rsplit_once('@')?;
    let host_port = host_port.trim_end_matches('/');
    let (server, port) = extract_host_port(host_port, 51820)?;

    let private_key = if let Some(decoded) = decode_base64_robust(private_key_raw) {
        String::from_utf8(decoded).unwrap_or_else(|_| private_key_raw.to_string())
    } else {
        private_key_raw.to_string()
    };

    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("wireguard"));
    map.insert(Value::from("name"), Value::from(remarks));
    map.insert(Value::from("server"), Value::from(server));
    map.insert(Value::from("port"), Value::from(port));
    map.insert(Value::from("private-key"), Value::from(private_key));
    map.insert(Value::from("udp"), Value::from(true));

    let mut address = String::new();
    if let Some(q) = query {
        for (k, v) in parse_query_pairs(q) {
            match k.as_str() {
                "public-key" | "publickey" | "pk" => {
                    map.insert(Value::from("public-key"), Value::from(percent_decode(&v)));
                }
                "reserved" => parse_wireguard_reserved(&mut map, &v),
                "address" | "ip" => address = percent_decode(&v),
                "mtu" => {
                    if let Ok(mtu) = v.parse::<u16>() {
                        map.insert(Value::from("mtu"), Value::from(mtu));
                    }
                }
                "preshared-key" | "pre-shared-key" | "psk" => {
                    map.insert(Value::from("pre-shared-key"), Value::from(percent_decode(&v)));
                }
                _ => {}
            }
        }
    }

    if address.is_empty() {
        map.insert(Value::from("allowed-ips"), Value::from(vec![Value::from("0.0.0.0/0")]));
    } else {
        let addr_parts: Vec<String> = address.split(',').map(|s| s.trim().to_string()).collect();
        apply_wireguard_addresses(&mut map, &addr_parts);
    }

    Some(map)
}
