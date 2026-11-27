use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_yaml_ng::{Mapping, Value};

mod containers;
mod uri_classic;
mod uri_modern;

#[cfg(test)]
mod tests;

pub use containers::parse_subscription_payload;

/// 大小写不敏感地剥离固定 ASCII 前缀，同时保留后续载荷的原始大小写（Base64 / 密码 / 公钥均区分大小写）。
pub(super) fn strip_prefix_ascii_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix).then(|| &s[prefix.len()..])
}

pub(super) fn percent_decode(v: &str) -> String {
    percent_encoding::percent_decode_str(v).decode_utf8_lossy().to_string()
}

pub(super) fn is_truthy(v: &str) -> bool {
    v == "1" || v.eq_ignore_ascii_case("true")
}

pub(super) fn parse_csv_list(v: &str) -> Vec<String> {
    v.split(',')
        .map(|s| percent_decode(s).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn extract_host_port(host_port: &str, default_port: u16) -> Option<(String, u16)> {
    let host_port = host_port.trim();
    if host_port.starts_with('[') {
        let close = host_port.find(']')?;
        let host = host_port[1..close].trim();
        if host.is_empty() {
            return None;
        }
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
        let host = parts.next()?.trim();
        if host.is_empty() {
            return None;
        }
        let port_str = parts.next().unwrap_or("").trim();
        let port = if port_str.is_empty() {
            default_port
        } else {
            port_str.parse::<u16>().ok().unwrap_or(default_port)
        };
        Some((host.to_string(), port))
    }
}

/// 统一 URI 预处理管道（BUG-320 / BUG-348）：
/// 严格遵循「先剥离 `#fragment` -> 再剥离 `?query` -> 最后交由各协议按最后一个 `@` 拆解 userinfo」顺序，
/// 彻底杜绝查询参数或密码中含有 `@` 时主机端口被错误截断。
pub(super) fn strip_fragment_and_query(payload: &str) -> (&str, Option<&str>, Option<String>) {
    let (before_frag, frag_opt) = match payload.split_once('#') {
        Some((b, f)) => (b, Some(percent_decode(f))),
        None => (payload, None),
    };
    let (auth_host, query_opt) = match before_frag.split_once('?') {
        Some((ah, q)) => (ah, Some(q)),
        None => (before_frag, None),
    };
    (auth_host, query_opt, frag_opt)
}

pub fn decode_base64_robust(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let s_clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s_clean.is_empty() {
        return None;
    }

    if let Ok(data) = STANDARD.decode(&s_clean) {
        return Some(data);
    }
    if let Ok(data) = base64::engine::general_purpose::URL_SAFE.decode(&s_clean) {
        return Some(data);
    }

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

/// 解析 URI query 串为 (小写键, 原值) 列表，跳过键或值为空的项。
pub(super) fn parse_query_pairs(query: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        let key = kv.next().unwrap_or("").trim().to_ascii_lowercase();
        let value = kv.next().unwrap_or("");
        if key.is_empty() || value.is_empty() {
            continue;
        }
        pairs.push((key, value.to_string()));
    }
    pairs
}

/// 把分享链接里的带宽值规范成 mihomo 可识别的写法：
/// 纯数字补 " Mbps"（内核默认单位），已带单位的值原样保留。
pub(super) fn normalize_bandwidth(value: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
        format!("{trimmed} Mbps")
    } else {
        trimmed.to_string()
    }
}

/// 按逗号拆分为非空字符串列表，并包成 YAML `Value::Sequence`。
pub(super) fn insert_alpn_list(map: &mut Mapping, raw: &str) {
    let list: Vec<String> = parse_csv_list(raw);
    if !list.is_empty() {
        map.insert(Value::from("alpn"), Value::from(list));
    }
}

/// 统一传输层选项构建参数，消除 vmess / vless / trojan / sing-box 之间的重复逻辑。
#[derive(Default)]
pub(super) struct TransportOpts {
    pub network: String,
    pub path: String,
    pub host: String,
    pub service_name: String,
    pub header_type: String,
    pub mode: String,
    pub extra: String,
    pub ws_max_early_data: Option<u64>,
    pub ws_early_data_header_name: String,
}

fn build_ws_opts(opts: &TransportOpts, http_upgrade: bool) -> Mapping {
    let mut ws_opts = Mapping::new();
    if !opts.path.is_empty() {
        ws_opts.insert(Value::from("path"), Value::from(opts.path.as_str()));
    }
    if !opts.host.is_empty() {
        let mut headers = Mapping::new();
        headers.insert(Value::from("Host"), Value::from(opts.host.as_str()));
        ws_opts.insert(Value::from("headers"), Value::from(headers));
    }
    if let Some(max_early) = opts.ws_max_early_data {
        ws_opts.insert(Value::from("max-early-data"), Value::from(max_early));
    }
    if !opts.ws_early_data_header_name.is_empty() {
        ws_opts.insert(
            Value::from("early-data-header-name"),
            Value::from(opts.ws_early_data_header_name.as_str()),
        );
    }
    if http_upgrade {
        ws_opts.insert(Value::from("v2ray-http-upgrade"), Value::from(true));
    }
    ws_opts
}

fn build_http_opts(path: &str, host: &str) -> Mapping {
    let mut http_opts = Mapping::new();
    if !path.is_empty() {
        let paths: Vec<String> = path
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !paths.is_empty() {
            http_opts.insert(Value::from("path"), Value::from(paths));
        }
    }
    if !host.is_empty() {
        let hosts: Vec<String> = host
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !hosts.is_empty() {
            let mut headers = Mapping::new();
            headers.insert(Value::from("Host"), Value::from(hosts));
            http_opts.insert(Value::from("headers"), Value::from(headers));
        }
    }
    http_opts
}

fn build_h2_opts(path: &str, host: &str) -> Mapping {
    let mut h2_opts = Mapping::new();
    if !path.is_empty() {
        h2_opts.insert(Value::from("path"), Value::from(path));
    }
    if !host.is_empty() {
        let hosts: Vec<String> = host
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !hosts.is_empty() {
            h2_opts.insert(Value::from("host"), Value::from(hosts));
        }
    }
    h2_opts
}

fn build_xhttp_opts(opts: &TransportOpts) -> Mapping {
    let mut xhttp_opts = Mapping::new();
    if !opts.path.is_empty() {
        xhttp_opts.insert(Value::from("path"), Value::from(opts.path.as_str()));
    }
    if !opts.host.is_empty() {
        xhttp_opts.insert(Value::from("host"), Value::from(opts.host.as_str()));
    }
    if !opts.mode.is_empty() {
        xhttp_opts.insert(Value::from("mode"), Value::from(opts.mode.as_str()));
    }
    if !opts.extra.is_empty()
        && let Ok(serde_json::Value::Object(extra_obj)) = serde_json::from_str::<serde_json::Value>(&opts.extra)
    {
        if let Some(no_grpc) = extra_obj.get("noGRPCHeader").and_then(serde_json::Value::as_bool) {
            xhttp_opts.insert(Value::from("no-grpc-header"), Value::from(no_grpc));
        }
        if let Some(padding) = extra_obj.get("xPaddingBytes").and_then(serde_json::Value::as_str)
            && !padding.is_empty()
        {
            xhttp_opts.insert(Value::from("x-padding-bytes"), Value::from(padding));
        }
    }
    xhttp_opts
}

/// 将归一化传输层参数写入 Mihomo 代理节点 `Mapping`。
pub(super) fn apply_transport_opts(map: &mut Mapping, opts: &TransportOpts, omit_plain_tcp: bool) {
    let raw_net = opts.network.trim().to_ascii_lowercase();
    let header_is_http = opts.header_type.trim().eq_ignore_ascii_case("http");
    let effective = match raw_net.as_str() {
        "" => {
            if header_is_http {
                "http"
            } else {
                ""
            }
        }
        "raw" | "tcp" => {
            if header_is_http {
                "http"
            } else if omit_plain_tcp {
                ""
            } else {
                "tcp"
            }
        }
        "websocket" => "ws",
        "splithttp" => "xhttp",
        other => other,
    };

    if effective.is_empty() {
        return;
    }

    match effective {
        "ws" => {
            map.insert(Value::from("network"), Value::from("ws"));
            map.insert(Value::from("ws-opts"), Value::from(build_ws_opts(opts, false)));
        }
        "httpupgrade" => {
            map.insert(Value::from("network"), Value::from("ws"));
            map.insert(Value::from("ws-opts"), Value::from(build_ws_opts(opts, true)));
        }
        "grpc" => {
            map.insert(Value::from("network"), Value::from("grpc"));
            let mut grpc_opts = Mapping::new();
            let service_name = if !opts.service_name.is_empty() {
                opts.service_name.as_str()
            } else {
                opts.path.as_str()
            };
            if !service_name.is_empty() {
                grpc_opts.insert(Value::from("grpc-service-name"), Value::from(service_name));
            }
            map.insert(Value::from("grpc-opts"), Value::from(grpc_opts));
        }
        "h2" => {
            map.insert(Value::from("network"), Value::from("h2"));
            map.insert(
                Value::from("h2-opts"),
                Value::from(build_h2_opts(&opts.path, &opts.host)),
            );
        }
        "http" => {
            map.insert(Value::from("network"), Value::from("http"));
            map.insert(
                Value::from("http-opts"),
                Value::from(build_http_opts(&opts.path, &opts.host)),
            );
        }
        "xhttp" => {
            map.insert(Value::from("network"), Value::from("xhttp"));
            let xhttp_opts = build_xhttp_opts(opts);
            if !xhttp_opts.is_empty() {
                map.insert(Value::from("xhttp-opts"), Value::from(xhttp_opts));
            }
        }
        other => {
            map.insert(Value::from("network"), Value::from(other));
        }
    }
}

/// 解析 SIP002 / SIP008 / sing-box 的 Shadowsocks 插件定义并写入 `plugin` 与 `plugin-opts`。
pub(super) fn apply_ss_plugin_parts(map: &mut Mapping, raw_name: &str, opts_str: &str) {
    let trimmed_name = raw_name.trim();
    if trimmed_name.is_empty() {
        return;
    }

    let plugin_name = match trimmed_name.to_ascii_lowercase().as_str() {
        "obfs-local" | "simple-obfs" | "obfs" => "obfs",
        "v2ray-plugin" => "v2ray-plugin",
        "shadow-tls" => "shadow-tls",
        "restls" => "restls",
        "gost-plugin" => "gost-plugin",
        _ => trimmed_name,
    };

    map.insert(Value::from("plugin"), Value::from(plugin_name));

    let mut plugin_opts = Mapping::new();
    for part in opts_str.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (k_raw, v_opt) = match part.split_once('=') {
            Some((k, v)) => (k.trim().to_ascii_lowercase(), Some(v.trim())),
            None => (part.to_ascii_lowercase(), None),
        };
        insert_ss_plugin_opt(&mut plugin_opts, plugin_name, &k_raw, v_opt);
    }

    if plugin_name == "v2ray-plugin" && !plugin_opts.contains_key("mode") {
        plugin_opts.insert(Value::from("mode"), Value::from("websocket"));
    }

    if !plugin_opts.is_empty() {
        map.insert(Value::from("plugin-opts"), Value::from(plugin_opts));
    }
}

fn insert_ss_plugin_opt(plugin_opts: &mut Mapping, plugin_name: &str, k: &str, v_opt: Option<&str>) {
    match (plugin_name, k, v_opt) {
        ("obfs", "obfs" | "mode", Some(v)) if !v.is_empty() => {
            plugin_opts.insert(Value::from("mode"), Value::from(v));
        }
        ("obfs", "obfs-host" | "host", Some(v)) if !v.is_empty() => {
            plugin_opts.insert(Value::from("host"), Value::from(v));
        }
        ("v2ray-plugin" | "gost-plugin", "tls", None) => {
            plugin_opts.insert(Value::from("tls"), Value::from(true));
        }
        ("v2ray-plugin" | "gost-plugin", "tls", Some(v)) => {
            plugin_opts.insert(Value::from("tls"), Value::from(is_truthy(v)));
        }
        ("v2ray-plugin" | "gost-plugin", "mux", Some(v)) => {
            let enabled = !(v == "0" || v.eq_ignore_ascii_case("false"));
            plugin_opts.insert(Value::from("mux"), Value::from(enabled));
        }
        (_, "skip-cert-verify" | "insecure" | "allowinsecure", None) => {
            plugin_opts.insert(Value::from("skip-cert-verify"), Value::from(true));
        }
        (_, "skip-cert-verify" | "insecure" | "allowinsecure", Some(v)) => {
            plugin_opts.insert(Value::from("skip-cert-verify"), Value::from(is_truthy(v)));
        }
        ("shadow-tls", "version", Some(v)) => {
            if let Ok(ver) = v.parse::<u64>() {
                plugin_opts.insert(Value::from("version"), Value::from(ver));
            }
        }
        (_, key, Some(v)) if !v.is_empty() => {
            plugin_opts.insert(Value::from(key), Value::from(v));
        }
        (_, key, None) => {
            plugin_opts.insert(Value::from(key), Value::from(true));
        }
        _ => {}
    }
}

/// 解析完整的 SIP002 `plugin=name;opt1=val1;opt2=val2` 参数值。
pub(super) fn apply_ss_plugin(map: &mut Mapping, plugin_raw: &str) {
    let trimmed = plugin_raw.trim();
    if trimmed.is_empty() {
        return;
    }
    let (name, opts) = trimmed.split_once(';').unwrap_or((trimmed, ""));
    apply_ss_plugin_parts(map, name, opts);
}

fn dispatch_uri_line(line: &str) -> Option<Mapping> {
    if strip_prefix_ascii_case(line, "vmess://").is_some() {
        uri_classic::parse_vmess(line)
    } else if strip_prefix_ascii_case(line, "ss://").is_some() {
        uri_classic::parse_ss(line)
    } else if strip_prefix_ascii_case(line, "ssr://").is_some() {
        uri_classic::parse_ssr(line)
    } else if strip_prefix_ascii_case(line, "trojan://").is_some() {
        uri_classic::parse_trojan(line)
    } else if strip_prefix_ascii_case(line, "vless://").is_some() {
        uri_modern::parse_vless(line)
    } else if strip_prefix_ascii_case(line, "hysteria2://").is_some()
        || strip_prefix_ascii_case(line, "hy2://").is_some()
    {
        uri_modern::parse_hysteria2(line)
    } else if strip_prefix_ascii_case(line, "hysteria://").is_some() {
        uri_modern::parse_hysteria(line)
    } else if strip_prefix_ascii_case(line, "anytls://").is_some() {
        uri_modern::parse_anytls(line)
    } else if strip_prefix_ascii_case(line, "tuic://").is_some() {
        uri_modern::parse_tuic(line)
    } else if strip_prefix_ascii_case(line, "wireguard://").is_some()
        || strip_prefix_ascii_case(line, "wg://").is_some()
    {
        uri_modern::parse_wireguard(line)
    } else if strip_prefix_ascii_case(line, "socks5://").is_some()
        || strip_prefix_ascii_case(line, "socks://").is_some()
    {
        uri_classic::parse_socks5(line)
    } else {
        None
    }
}

pub fn parse_uri_list(content: &str) -> Option<Mapping> {
    let mut proxies = Vec::new();

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

        if let Some(map) = uri_classic::parse_http_fallback(line) {
            proxies.push(Value::from(map));
            continue;
        }

        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        if let Some(map) = dispatch_uri_line(line) {
            proxies.push(Value::from(map));
        }
    }

    if proxies.is_empty() {
        return None;
    }

    let mut result = Mapping::new();
    result.insert(Value::from("proxies"), Value::from(proxies));
    Some(result)
}
