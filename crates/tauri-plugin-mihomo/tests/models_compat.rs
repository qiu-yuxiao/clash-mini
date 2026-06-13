use serde::de::DeserializeOwned;
use serde_json::json;
use tauri_plugin_mihomo::models::{
    Connections, DNSMode, ErrorResponse, FindProcessMode, LogLevel, Memory, MihomoVersion, Network,
    Connection, Proxy, ProxyType, Rule, RuleType, Traffic, TuicServer, TunConfig,
};

#[allow(clippy::expect_used)]
fn parse_json<T: DeserializeOwned>(raw: &str) -> T {
    serde_json::from_str(raw).expect("valid test payload")
}

#[allow(clippy::expect_used)]
fn parse_value<T: DeserializeOwned>(value: serde_json::Value) -> T {
    serde_json::from_value(value).expect("valid test payload")
}

#[test]
fn deserialize_connections_with_minimal_fields() {
    let raw = r#"{"downloadTotal":10,"uploadTotal":20,"memory":0}"#;
    let conns: Connections = parse_json(raw);
    assert_eq!(conns.download_total, 10);
    assert_eq!(conns.upload_total, 20);
    assert_eq!(conns.memory, 0);
    assert!(conns.connections.is_none());
}

#[test]
fn deserialize_connection_supports_provider_chains() {
    let raw = serde_json::json!({
        "id": "abc",
        "metadata": {
            "network": "tcp",
            "type": "HTTP",
            "sourceIP": "127.0.0.1",
            "destinationIP": "10.0.0.1",
            "sourceGeoIP": [],
            "destinationGeoIP": [],
            "sourceIPASN": "AS1",
            "destinationIPASN": "AS2",
            "sourcePort": "1000",
            "destinationPort": "2000",
            "inboundIP": "127.0.0.1",
            "inboundPort": "3000",
            "inboundName": "in",
            "inboundUser": "",
            "host": "",
            "dnsMode": "normal",
            "uid": 0,
            "process": "",
            "processPath": "",
            "specialProxy": "",
            "specialRules": "",
            "remoteDestination": "",
            "dscp": 0,
            "sniffHost": ""
        },
        "upload": 12,
        "download": 34,
        "start": "2026-01-01T00:00:00Z",
        "chains": ["proxyA"],
        "providerChains": ["providerX", "providerY"],
        "rule": "MATCH",
        "rulePayload": "some"
    });

    let conn: Connection = parse_value(raw);
    assert_eq!(conn.id, "abc");
    assert_eq!(conn.chains, vec!["proxyA".to_string()]);
    assert_eq!(
        conn.provider_chains,
        Some(vec!["providerX".to_string(), "providerY".to_string()])
    );
}

#[test]
fn deserialize_mihomo_version_ignores_unknown_fields() {
    let raw = r#"{"meta":true,"version":"1.2.3","future_field":"x","legacy_field":123}"#;
    let version: MihomoVersion = parse_json(raw);
    assert!(version.meta);
    assert_eq!(version.version, "1.2.3");
}

#[test]
fn deserialize_error_response_with_extra_fields() {
    let raw = r#"{"message":"oops","detail":{"foo":"bar"}}"#;
    let resp: ErrorResponse = parse_json(raw);
    assert_eq!(resp.message, "oops");
}

#[test]
fn deserialize_unknown_proxy_type() {
    let p: ProxyType = parse_json("\"LegacyType\"");
    assert!(matches!(p, ProxyType::Unknown(v) if v == "LegacyType"));
}

#[test]
fn deserialize_known_proxy_type_variants() {
    let value = parse_json::<ProxyType>("\"PassRule\"");
    assert_eq!(value, ProxyType::PassRule);
}

#[test]
fn deserialize_mihomo_version_captures_flatten_extra_fields() {
    let raw = r#"{"meta":true,"version":"1.2.3","future_field":"x","legacy_field":123}"#;
    let version: MihomoVersion = parse_json(raw);
    assert!(version.meta);
    assert_eq!(version.version, "1.2.3");
    assert_eq!(version.extra.get("future_field"), Some(&json!("x")));
    assert_eq!(version.extra.get("legacy_field"), Some(&json!(123)));
}

#[test]
fn deserialize_connections_captures_flatten_extra_fields() {
    let conns = parse_value::<Connections>(json!({
        "downloadTotal": 10,
        "uploadTotal": 20,
        "memory": 0,
        "newField": true,
    }));

    assert_eq!(conns.download_total, 10);
    assert_eq!(conns.upload_total, 20);
    assert_eq!(conns.extra.get("newField"), Some(&json!(true)));
}

#[test]
fn deserialize_traffic_captures_up_down_totals_and_unknown_fields() {
    let raw = r#"{"up":10,"down":20,"upTotal":100,"downTotal":200,"throughput":55}"#;
    let traffic: Traffic = parse_json(raw);
    assert_eq!(traffic.up, 10);
    assert_eq!(traffic.down, 20);
    assert_eq!(traffic.up_total, 100);
    assert_eq!(traffic.down_total, 200);
    assert_eq!(traffic.extra.get("throughput"), Some(&json!(55)));
}

#[test]
fn deserialize_unknown_log_level_should_fail() {
    let level = serde_json::from_str::<LogLevel>("\"trace\"");
    assert!(level.is_err());
}

#[test]
fn deserialize_unknown_find_process_mode_should_fail() {
    let value = serde_json::from_str::<FindProcessMode>("\"legacy\"");
    assert!(value.is_err());
}

#[test]
fn deserialize_unknown_dns_mode_should_parse_as_unknown_variant() {
    let value = serde_json::from_str::<DNSMode>("\"legacy\"");
    assert!(matches!(value, Ok(DNSMode::Unknown(v)) if v == "legacy"));
}

#[test]
fn deserialize_unknown_network_should_parse_as_unknown_variant() {
    let value = serde_json::from_str::<Network>("\"bluetooth\"");
    assert!(matches!(value, Ok(Network::Unknown(v)) if v == "bluetooth"));
}

#[test]
fn deserialize_unknown_rule_type_should_parse_as_unknown_variant() {
    let value = parse_json::<RuleType>("\"LegacyRule\"");
    assert!(matches!(value, RuleType::Unknown(v) if v == "LegacyRule"));
}

#[test]
fn deserialize_rule_supports_index_and_extra_fields() {
    let raw = r#"{
        "type": "Domain",
        "index": 4,
        "payload": "example.com",
        "proxy": "GLOBAL",
        "size": 12,
        "extra_field": "v"
    }"#;

    let rule: Rule = parse_json(raw);
    assert_eq!(rule.rule_type, RuleType::Domain);
    assert_eq!(rule.index, 4);
    assert_eq!(rule.payload, "example.com");
    assert_eq!(rule.proxy, "GLOBAL");
    assert_eq!(rule.size, 12);
    assert_eq!(rule.extra.get("extra_field"), Some(&json!("v")));
}

#[test]
fn deserialize_known_rule_type_variants() {
    let value = parse_json::<RuleType>("\"DomainWildcard\"");
    assert_eq!(value, RuleType::DomainWildcard);

    let value = parse_json::<RuleType>("\"ProcessNameWildcard\"");
    assert_eq!(value, RuleType::ProcessNameWildcard);

    let value = parse_json::<RuleType>("\"ProcessPathWildcard\"");
    assert_eq!(value, RuleType::ProcessPathWildcard);
}

#[test]
fn deserialize_tuic_server_with_new_fields_and_unknown() {
    let raw = r#"{
        "enable": true,
        "listen": "127.0.0.1:1234",
        "certificate": "cert",
        "private-key": "key",
        "ech-key": "",
        "client-auth-type": "SelfSigned",
        "client-auth-cert": "client.pem",
        "bbr-profile": "BBRv2",
        "future-field": "v"
    }"#;
    let tuic: TuicServer = parse_json(raw);
    assert!(tuic.enable);
    assert_eq!(tuic.client_auth_type.as_deref(), Some("SelfSigned"));
    assert_eq!(tuic.client_auth_cert.as_deref(), Some("client.pem"));
    assert_eq!(tuic.bbr_profile.as_deref(), Some("BBRv2"));
    assert_eq!(tuic.extra.get("future-field"), Some(&json!("v")));
}

#[test]
fn deserialize_tun_config_supports_new_fields() {
    let raw = r#"{
        "enable": true,
        "device": "utun0",
        "stack": "System",
        "auto-detect-interface": true,
        "auto-route": true,
        "dns-hijack": ["1.1.1.1/32"],
        "file-descriptor": 0,
        "auto-redirect-input-mark": 10,
        "auto-redirect-output-mark": 20,
        "auto-redirect-iproute2-fallback-rule-index": 30,
        "loopback-address": ["127.0.0.1/8"],
        "include-mac-address": ["aa:bb"],
        "exclude-mac-address": ["cc:dd"],
        "disable-icmp-forwarding": true
    }"#;
    let tun: TunConfig = parse_json(raw);
    assert_eq!(tun.auto_redirect_input_mark, Some(10));
    assert_eq!(tun.auto_redirect_output_mark, Some(20));
    assert_eq!(tun.auto_redirect_iproute2_fallback_rule_index, Some(30));
    assert_eq!(tun.loopback_address, Some(vec!["127.0.0.1/8".to_string()]));
    assert_eq!(tun.include_mac_address, Some(vec!["aa:bb".to_string()]));
    assert_eq!(tun.exclude_mac_address, Some(vec!["cc:dd".to_string()]));
    assert_eq!(tun.disable_icmp_forwarding, Some(true));
}

#[test]
fn deserialize_proxy_routing_mark_and_provider_name_extends_model() {
    let raw = r#"{
        "alive": true,
        "history": [],
        "extra": {},
        "name": "test",
        "udp": true,
        "uot": false,
        "type": "Direct",
        "xudp": false,
        "tfo": false,
        "mptcp": false,
        "smux": false,
        "interface": "eth0",
        "dialer-proxy": "",
        "routing-mark": 214,
        "provider-name": "group"
    }"#;
    let proxy: Proxy = parse_json(raw);
    assert_eq!(proxy.routing_mark, 214);
    assert_eq!(proxy.provider_name, "group");
}

#[test]
fn deserialize_memory_supports_u64_fields() {
    let raw = r#"{"inuse": 4096,"oslimit": 8192}"#;
    let mem: Memory = parse_json(raw);
    assert_eq!(mem.inuse, 4096);
    assert_eq!(mem.oslimit, 8192);
}
