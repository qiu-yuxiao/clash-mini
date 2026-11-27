#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::uri_classic::{parse_http_fallback, parse_socks5, parse_ss, parse_ssr, parse_trojan, parse_vmess};
use super::uri_modern::{parse_anytls, parse_hysteria, parse_hysteria2, parse_tuic, parse_vless, parse_wireguard};
use super::{decode_base64_robust, extract_host_port, parse_subscription_payload, parse_uri_list};
use base64::Engine as _;
use serde_yaml_ng::Value;

#[test]
fn test_parse_vless_reality() {
    let link = "vless://2513e61a-6982-39cd-9f0a-8f6fbc3fc5f0@awsjfde1.fatdns.net:443?encryption=none&flow=xtls-rprx-vision&security=reality&type=tcp&sni=s0.awsstatic.com&pbk=QzUWMFvn-J14NCZGsn5pz30inE7VT9auYKRwaRr5T20&sid=00fd4e9ac051c144&fp=chrome#aws%E5%BE%B7%E5%9B%BD";
    let map = parse_vless(link).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "vless");
    assert_eq!(map.get(Value::from("name")).unwrap().as_str().unwrap(), "aws德国");
    assert_eq!(
        map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "awsjfde1.fatdns.net"
    );
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
    assert_eq!(
        map.get(Value::from("uuid")).unwrap().as_str().unwrap(),
        "2513e61a-6982-39cd-9f0a-8f6fbc3fc5f0"
    );
    assert_eq!(
        map.get(Value::from("flow")).unwrap().as_str().unwrap(),
        "xtls-rprx-vision"
    );
    assert!(map.get(Value::from("tls")).unwrap().as_bool().unwrap());
    assert!(map.get(Value::from("encryption")).is_none());
    assert_eq!(
        map.get(Value::from("servername")).unwrap().as_str().unwrap(),
        "s0.awsstatic.com"
    );
    assert_eq!(
        map.get(Value::from("client-fingerprint")).unwrap().as_str().unwrap(),
        "chrome"
    );

    let reality_opts = map.get(Value::from("reality-opts")).unwrap().as_mapping().unwrap();
    assert_eq!(
        reality_opts.get(Value::from("public-key")).unwrap().as_str().unwrap(),
        "QzUWMFvn-J14NCZGsn5pz30inE7VT9auYKRwaRr5T20"
    );
    assert_eq!(
        reality_opts.get(Value::from("short-id")).unwrap().as_str().unwrap(),
        "00fd4e9ac051c144"
    );
}

#[test]
fn test_parse_http_fallback() {
    let line = "# [HTTP] v5-香港03|1x|v (v5.cdn.ljz8s7lhbv.fatdns.net:6712)";
    let map = parse_http_fallback(line).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "http");
    assert_eq!(
        map.get(Value::from("name")).unwrap().as_str().unwrap(),
        "v5-香港03|1x|v"
    );
    assert_eq!(
        map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "v5.cdn.ljz8s7lhbv.fatdns.net"
    );
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 6712);
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

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "vless");
    assert_eq!(
        map.get(Value::from("name")).unwrap().as_str().unwrap(),
        "未知 VLESS-160"
    );
    assert_eq!(
        map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "2401:c080:1000:29ac:5400:6ff:fe43:9d48"
    );
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
    assert_eq!(
        map.get(Value::from("uuid")).unwrap().as_str().unwrap(),
        "28beee5e-40ab-3c36-b797-1e4ecf27d0a0"
    );
    assert_eq!(
        map.get(Value::from("flow")).unwrap().as_str().unwrap(),
        "xtls-rprx-vision"
    );
    assert!(map.get(Value::from("tls")).unwrap().as_bool().unwrap());
    assert_eq!(
        map.get(Value::from("servername")).unwrap().as_str().unwrap(),
        "u729792us3611.wagahaha.xyz"
    );
    assert_eq!(
        map.get(Value::from("client-fingerprint")).unwrap().as_str().unwrap(),
        "ios"
    );
    let alpn = map.get(Value::from("alpn")).unwrap().as_sequence().unwrap();
    assert_eq!(alpn.len(), 2);
    assert_eq!(alpn[0].as_str().unwrap(), "h2");
    assert_eq!(alpn[1].as_str().unwrap(), "http/1.1");
}

#[test]
fn test_parse_trojan_ipv6() {
    let link = "trojan://password@[2001:db8::1]:443?sni=example.com#test";
    let map = parse_trojan(link).unwrap();
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "2001:db8::1");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
}

#[test]
fn test_parse_hysteria2_ipv6() {
    let link = "hysteria2://password@[2001:db8::1]:8443?sni=example.com#test";
    let map = parse_hysteria2(link).unwrap();
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "2001:db8::1");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);
}

#[test]
fn test_parse_tuic_v5() {
    let link = "tuic://00000000-0000-0000-0000-000000000001:password@example.com:10443?congestion_control=cubic&alpn=h3&sni=example.com&allow_insecure=0&udp_relay_mode=native#test-tuic";
    let map = parse_tuic(link).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "tuic");
    assert_eq!(map.get(Value::from("name")).unwrap().as_str().unwrap(), "test-tuic");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 10443);
    assert_eq!(
        map.get(Value::from("uuid")).unwrap().as_str().unwrap(),
        "00000000-0000-0000-0000-000000000001"
    );
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "password");
    assert_eq!(
        map.get(Value::from("congestion-controller")).unwrap().as_str().unwrap(),
        "cubic"
    );
    assert_eq!(
        map.get(Value::from("udp-relay-mode")).unwrap().as_str().unwrap(),
        "native"
    );
    let alpn = map.get(Value::from("alpn")).unwrap().as_sequence().unwrap();
    assert_eq!(alpn.len(), 1);
    assert_eq!(alpn[0].as_str().unwrap(), "h3");
}

#[test]
fn test_parse_tuic_v4() {
    let link = "tuic://TOKEN@example.com:443?alpn=h3&udp_relay_mode=quic#v4-node";
    let map = parse_tuic(link).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "tuic");
    assert_eq!(map.get(Value::from("token")).unwrap().as_str().unwrap(), "TOKEN");
    assert_eq!(
        map.get(Value::from("udp-relay-mode")).unwrap().as_str().unwrap(),
        "quic"
    );
}

#[test]
fn test_parse_tuic_ipv6() {
    let link = "tuic://uuid:password@[2001:db8::1]:443?sni=example.com#tuic-ipv6";
    let map = parse_tuic(link).unwrap();
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "2001:db8::1");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
}

#[test]
fn test_parse_wireguard() {
    let link = "wireguard://eCtXsJZ27+4PbhDkHnB923tkUn2Gj59wZw5wFA75MnU=@162.159.192.1:2480?public-key=Cr8hWlKvtDt7nrvf+f0brNQQzabAqrjfBvas9pmowjo=&address=172.16.0.2&mtu=1408#wg-test";
    let map = parse_wireguard(link).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "wireguard");
    assert_eq!(map.get(Value::from("name")).unwrap().as_str().unwrap(), "wg-test");
    assert_eq!(
        map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "162.159.192.1"
    );
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 2480);
    assert_eq!(
        map.get(Value::from("public-key")).unwrap().as_str().unwrap(),
        "Cr8hWlKvtDt7nrvf+f0brNQQzabAqrjfBvas9pmowjo="
    );
    assert_eq!(map.get(Value::from("ip")).unwrap().as_str().unwrap(), "172.16.0.2");
    assert_eq!(map.get(Value::from("mtu")).unwrap().as_u64().unwrap(), 1408);
    let allowed = map.get(Value::from("allowed-ips")).unwrap().as_sequence().unwrap();
    assert_eq!(allowed.len(), 1);
    assert_eq!(allowed[0].as_str().unwrap(), "0.0.0.0/0");
}

#[test]
fn test_parse_wireguard_ipv6() {
    let link = "wireguard://eCtXsJZ27+4PbhDkHnB923tkUn2Gj59wZw5wFA75MnU=@[2001:db8::1]:51820?public-key=Cr8hWlKvtDt7nrvf+f0brNQQzabAqrjfBvas9pmowjo=&address=172.16.0.2#wg-ipv6";
    let map = parse_wireguard(link).unwrap();
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "2001:db8::1");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 51820);
}

#[test]
fn test_parse_socks5_no_auth() {
    let link = "socks5://example.com:1080#socks-test";
    let map = parse_socks5(link).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "socks5");
    assert_eq!(map.get(Value::from("name")).unwrap().as_str().unwrap(), "socks-test");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 1080);
}

#[test]
fn test_parse_socks5_with_auth() {
    let link = "socks5://user:pass@example.com:1080#auth-socks";
    let map = parse_socks5(link).unwrap();

    assert_eq!(map.get(Value::from("username")).unwrap().as_str().unwrap(), "user");
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "pass");
}

#[test]
fn test_parse_socks5_ipv6() {
    let link = "socks5://[2001:db8::1]:1080#ipv6-socks";
    let map = parse_socks5(link).unwrap();
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "2001:db8::1");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 1080);
}

#[test]
fn test_parse_tuic_percent_encoded() {
    let link = "tuic://uuid%3Dtest:pass%40word@example.com:443#encoded-tuic";
    let map = parse_tuic(link).unwrap();
    assert_eq!(map.get(Value::from("uuid")).unwrap().as_str().unwrap(), "uuid=test");
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "pass@word");
}

#[test]
fn test_parse_wireguard_address_with_cidr() {
    let link = "wireguard://key@server:51820?address=10.0.0.2/32,fd00::2/128#wg-cidr";
    let map = parse_wireguard(link).unwrap();
    assert_eq!(map.get(Value::from("ip")).unwrap().as_str().unwrap(), "10.0.0.2");
    assert_eq!(map.get(Value::from("ipv6")).unwrap().as_str().unwrap(), "fd00::2");
    let allowed = map.get(Value::from("allowed-ips")).unwrap().as_sequence().unwrap();
    assert_eq!(allowed, &[Value::from("0.0.0.0/0"), Value::from("::/0")]);
}

#[test]
fn test_parse_wireguard_ipv6_first() {
    let link = "wireguard://key@server:51820?address=fd00::1/64,10.0.0.1/24#wg-ipv6-first";
    let map = parse_wireguard(link).unwrap();
    assert_eq!(map.get(Value::from("ip")).unwrap().as_str().unwrap(), "10.0.0.1");
    assert_eq!(map.get(Value::from("ipv6")).unwrap().as_str().unwrap(), "fd00::1");
}

#[test]
fn test_parse_wireguard_multiple_ipv6() {
    let link = "wireguard://key@server:51820?address=10.0.0.1/32,fd00::1/64,fd01::2/64#wg-multi-ipv6";
    let map = parse_wireguard(link).unwrap();
    assert_eq!(map.get(Value::from("ip")).unwrap().as_str().unwrap(), "10.0.0.1");
    let ipv6_list = map.get(Value::from("ipv6")).unwrap().as_sequence().unwrap();
    assert_eq!(ipv6_list.len(), 2);
    assert_eq!(ipv6_list[0].as_str().unwrap(), "fd00::1");
    assert_eq!(ipv6_list[1].as_str().unwrap(), "fd01::2");
}

#[test]
fn test_parse_hysteria() {
    let link = "hysteria://208.87.243.187:15823?upmbps=100&downmbps=100&auth=dongtaiwang.com&insecure=1&peer=apple.com&alpn=h3#hy-test";
    let map = parse_hysteria(link).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "hysteria");
    assert_eq!(map.get(Value::from("name")).unwrap().as_str().unwrap(), "hy-test");
    assert_eq!(
        map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "208.87.243.187"
    );
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 15823);
    assert_eq!(
        map.get(Value::from("auth-str")).unwrap().as_str().unwrap(),
        "dongtaiwang.com"
    );
    assert_eq!(map.get(Value::from("sni")).unwrap().as_str().unwrap(), "apple.com");
    assert_eq!(map.get(Value::from("up")).unwrap().as_str().unwrap(), "100 Mbps");
    assert_eq!(map.get(Value::from("down")).unwrap().as_str().unwrap(), "100 Mbps");
    assert!(map.get(Value::from("skip-cert-verify")).unwrap().as_bool().unwrap());
    let alpn = map.get(Value::from("alpn")).unwrap().as_sequence().unwrap();
    assert_eq!(alpn.len(), 1);
    assert_eq!(alpn[0].as_str().unwrap(), "h3");
}

#[test]
fn test_parse_hysteria_obfs_param() {
    let link = "hysteria://51.159.226.1:49003?auth=x&obfs=xplus&obfsParam=secret#hy-obfs";
    let map = parse_hysteria(link).unwrap();
    assert_eq!(map.get(Value::from("obfs")).unwrap().as_str().unwrap(), "secret");
}

#[test]
fn test_parse_hysteria_defaults() {
    let link = "hysteria://example.com?auth=pw#hy-min";
    let map = parse_hysteria(link).unwrap();
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
    assert!(map.get(Value::from("up")).is_none());
    assert!(map.get(Value::from("sni")).is_none());
    assert!(map.get(Value::from("skip-cert-verify")).is_none());
}

#[test]
fn test_parse_hysteria_ipv6() {
    let link = "hysteria://[2001:db8::1]:8443?auth=pw&peer=example.com#hy-ipv6";
    let map = parse_hysteria(link).unwrap();
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "2001:db8::1");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);
}

#[test]
fn test_parse_hysteria_invalid() {
    assert!(parse_hysteria("hysteria://?auth=pw").is_none());
    let map = parse_hysteria("hysteria://example.com?auth=pw&protocol=tcp#hy-proto").unwrap();
    assert!(map.get(Value::from("protocol")).is_none());
}

#[test]
fn test_parse_anytls() {
    let link = "anytls://mypassword@example.com:8443?sni=example.com&insecure=1&alpn=h2,http/1.1&fp=chrome#anytls-test";
    let map = parse_anytls(link).unwrap();

    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "anytls");
    assert_eq!(map.get(Value::from("name")).unwrap().as_str().unwrap(), "anytls-test");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);
    assert_eq!(
        map.get(Value::from("password")).unwrap().as_str().unwrap(),
        "mypassword"
    );
    assert_eq!(map.get(Value::from("sni")).unwrap().as_str().unwrap(), "example.com");
    assert!(map.get(Value::from("skip-cert-verify")).unwrap().as_bool().unwrap());
    assert_eq!(
        map.get(Value::from("client-fingerprint")).unwrap().as_str().unwrap(),
        "chrome"
    );
    let alpn = map.get(Value::from("alpn")).unwrap().as_sequence().unwrap();
    assert_eq!(alpn.len(), 2);
    assert_eq!(alpn[0].as_str().unwrap(), "h2");
    assert_eq!(alpn[1].as_str().unwrap(), "http/1.1");
}

#[test]
fn test_parse_anytls_defaults() {
    let link = "anytls://pw@example.com#anytls-min";
    let map = parse_anytls(link).unwrap();
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
    assert!(map.get(Value::from("sni")).is_none());
    assert!(map.get(Value::from("skip-cert-verify")).is_none());
}

#[test]
fn test_parse_anytls_ipv6() {
    let link = "anytls://pw@[2001:db8::1]:443?sni=example.com#anytls-ipv6";
    let map = parse_anytls(link).unwrap();
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "2001:db8::1");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
}

#[test]
fn test_parse_anytls_invalid() {
    assert!(parse_anytls("anytls://example.com:443").is_none());
    assert!(parse_anytls("anytls://@example.com:443").is_none());
}

#[test]
fn test_parse_anytls_password_with_at() {
    let map = parse_anytls("anytls://user%40pass@example.com:8443?sni=example.com#anytls-enc").unwrap();
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "user@pass");

    let map = parse_anytls("anytls://user@pass@example.com:8443?sni=example.com#anytls-at").unwrap();
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "user@pass");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);
}

#[test]
fn test_parse_hysteria_percent_encoded_query() {
    let link = "hysteria://example.com:443?auth=pass%2Bword%26123&peer=sub.example.com&obfsParam=sec%20ret&protocol=wechat%2Dvideo#hy-decode";
    let map = parse_hysteria(link).unwrap();
    assert_eq!(
        map.get(Value::from("auth-str")).unwrap().as_str().unwrap(),
        "pass+word&123"
    );
    assert_eq!(
        map.get(Value::from("sni")).unwrap().as_str().unwrap(),
        "sub.example.com"
    );
    assert_eq!(map.get(Value::from("obfs")).unwrap().as_str().unwrap(), "sec ret");
    assert_eq!(
        map.get(Value::from("protocol")).unwrap().as_str().unwrap(),
        "wechat-video"
    );
}

#[test]
fn test_parse_trojan_password_with_at_and_sni_decode() {
    let link = "trojan://a@b@example.com:443?sni=cdn%20edge.example.com#t-at";
    let map = parse_trojan(link).unwrap();
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "a@b");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(
        map.get(Value::from("sni")).unwrap().as_str().unwrap(),
        "cdn edge.example.com"
    );
}

#[test]
fn test_parse_hysteria2_password_and_query_decode() {
    let link = "hysteria2://p@ss@example.com:8443?sni=example.com&obfs=salamander&obfs-password=my%20secret#h2-decode";
    let map = parse_hysteria2(link).unwrap();
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "p@ss");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(
        map.get(Value::from("obfs-password")).unwrap().as_str().unwrap(),
        "my secret"
    );
}

#[test]
fn test_parse_tuic_password_with_at() {
    let link = "tuic://00000000-0000-0000-0000-000000000001:p@w@example.com:443?sni=x.com#tuic-at";
    let map = parse_tuic(link).unwrap();
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "p@w");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
}

#[test]
fn test_parse_hysteria_query_value_contains_at() {
    let link = "hysteria://auth_str@example.com:443?peer=user@backend.example.com&insecure=1#hy-query-at";
    let map = parse_hysteria(link).unwrap();
    assert_eq!(map.get(Value::from("auth-str")).unwrap().as_str().unwrap(), "auth_str");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(
        map.get(Value::from("sni")).unwrap().as_str().unwrap(),
        "user@backend.example.com"
    );
    assert!(map.get(Value::from("skip-cert-verify")).unwrap().as_bool().unwrap());
}

#[test]
fn test_parse_anytls_query_value_contains_at() {
    let link = "anytls://mypass@example.com:443?sni=user@internal.example.com&insecure=true#anytls-query-at";
    let map = parse_anytls(link).unwrap();
    assert_eq!(map.get(Value::from("password")).unwrap().as_str().unwrap(), "mypass");
    assert_eq!(map.get(Value::from("server")).unwrap().as_str().unwrap(), "example.com");
    assert_eq!(
        map.get(Value::from("sni")).unwrap().as_str().unwrap(),
        "user@internal.example.com"
    );
    assert!(map.get(Value::from("skip-cert-verify")).unwrap().as_bool().unwrap());
}

#[test]
fn test_parse_vmess_sni_field_preserved() {
    let raw_json = r#"{"v":"2","ps":"VMess-SNI","add":"1.2.3.4","port":"443","id":"11111111-2222-3333-4444-555555555555","aid":"0","net":"ws","type":"none","host":"ws.cdn.com","sni":"sni.origin.com","path":"/ws","tls":"tls"}"#;
    let encoded = base64::engine::general_purpose::STANDARD.encode(raw_json);
    let link = format!("vmess://{encoded}");
    let map = parse_vmess(&link).unwrap();
    assert_eq!(
        map.get(Value::from("servername")).unwrap().as_str().unwrap(),
        "sni.origin.com"
    );
}

#[test]
fn test_parse_ss_wireguard_socks5_query_with_at() {
    let user_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("aes-256-gcm:secret");
    let ss_link = format!("ss://{user_b64}@ss.example.com:8388/?plugin=v2ray-plugin%3Bhost%3Dcdn@edge.com#SS-At");
    let ss_map = parse_ss(&ss_link).unwrap();
    assert_eq!(
        ss_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "ss.example.com"
    );
    assert_eq!(ss_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8388);
    assert_eq!(
        ss_map.get(Value::from("plugin")).unwrap().as_str().unwrap(),
        "v2ray-plugin"
    );
    let p_opts = ss_map.get(Value::from("plugin-opts")).unwrap().as_mapping().unwrap();
    assert_eq!(
        p_opts.get(Value::from("host")).unwrap().as_str().unwrap(),
        "cdn@edge.com"
    );

    let wg_link =
        "wireguard://cHJpdmF0ZWtleQ==@wg.example.com:51820/?publickey=cHVibGlja2V5&reserved=1,2,3&peer=test@wg#WG-At";
    let wg_map = parse_wireguard(wg_link).unwrap();
    assert_eq!(
        wg_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "wg.example.com"
    );
    assert_eq!(wg_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 51820);

    let socks_link = "socks5://u@name:pass@198.51.100.10:1080/?tag=a@b#Socks-At";
    let socks_map = parse_socks5(socks_link).unwrap();
    assert_eq!(
        socks_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "198.51.100.10"
    );
    assert_eq!(
        socks_map.get(Value::from("username")).unwrap().as_str().unwrap(),
        "u@name"
    );

    let vless_link = "vless://2513e61a-6982-39cd-9f0a-8f6fbc3fc5f0@vless.example.com:8443/?type=ws&path=%2Fws@v2&host=cdn@edge.com#VLESS-At";
    let vless_map = parse_vless(vless_link).unwrap();
    assert_eq!(
        vless_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "vless.example.com"
    );
    assert_eq!(vless_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);

    let trojan_link = "trojan://p@ss@trojan.example.com:8443/?sni=sni.example.com&extra=a@b#Trojan-At";
    let trojan_map = parse_trojan(trojan_link).unwrap();
    assert_eq!(
        trojan_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "trojan.example.com"
    );
    assert_eq!(trojan_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);

    let hy2_link = "hysteria2://p@ss@hy2.example.com:8443/?sni=sni.example.com&obfs-password=k@y#Hy2-At";
    let hy2_map = parse_hysteria2(hy2_link).unwrap();
    assert_eq!(
        hy2_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "hy2.example.com"
    );
    assert_eq!(hy2_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);

    let hy1_link = "hysteria://hy1.example.com:8443/?auth=tok@en&upmbps=50&downmbps=100#Hy1-At";
    let hy1_map = parse_hysteria(hy1_link).unwrap();
    assert_eq!(
        hy1_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "hy1.example.com"
    );
    assert_eq!(hy1_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);

    let anytls_link = "anytls://p@ss@anytls.example.com:8443/?sni=sni.example.com&tag=x@y#AnyTLS-At";
    let anytls_map = parse_anytls(anytls_link).unwrap();
    assert_eq!(
        anytls_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "anytls.example.com"
    );
    assert_eq!(anytls_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);

    let tuic_link = "tuic://uuid:p@ss@tuic.example.com:8443/?sni=sni.example.com&tag=x@y#TUIC-At";
    let tuic_map = parse_tuic(tuic_link).unwrap();
    assert_eq!(
        tuic_map.get(Value::from("server")).unwrap().as_str().unwrap(),
        "tuic.example.com"
    );
    assert_eq!(tuic_map.get(Value::from("port")).unwrap().as_u64().unwrap(), 8443);
}

#[test]
fn test_parse_hy2_extended_params_and_case_insensitive_scheme() {
    let link = "HY2://pass%23word@hy2.example.com:443?mport=20000-30000,40000/40500&hop-interval=30s&pinSHA256=ab:cd:ef&alpn=h3,h2&upmbps=100&downmbps=500&allowInsecure=1&sni=cdn.hy2.example#Hy2-Hop";
    let map = parse_hysteria2(link).unwrap();
    assert_eq!(map.get(Value::from("type")).unwrap().as_str().unwrap(), "hysteria2");
    assert_eq!(
        map.get(Value::from("ports")).unwrap().as_str().unwrap(),
        "20000-30000,40000,40500"
    );
    assert_eq!(map.get(Value::from("hop-interval")).unwrap().as_u64().unwrap(), 30);
    assert_eq!(
        map.get(Value::from("fingerprint")).unwrap().as_str().unwrap(),
        "ab:cd:ef"
    );
    assert_eq!(map.get(Value::from("up")).unwrap().as_str().unwrap(), "100 Mbps");
    assert_eq!(map.get(Value::from("down")).unwrap().as_str().unwrap(), "500 Mbps");
    assert!(map.get(Value::from("skip-cert-verify")).unwrap().as_bool().unwrap());
    assert_eq!(map.get(Value::from("alpn")).unwrap().as_sequence().unwrap().len(), 2);
}

#[test]
fn test_parse_vless_grpc_xhttp_httpupgrade_and_insecure() {
    let grpc_link = "VLESS://11111111-1111-1111-1111-111111111111@grpc.example.com:443?type=grpc&serviceName=myGrpcSvc&security=tls&allowInsecure=1#VLESS-gRPC";
    let grpc_map = parse_vless(grpc_link).unwrap();
    assert_eq!(grpc_map.get(Value::from("network")).unwrap().as_str().unwrap(), "grpc");
    assert!(
        grpc_map
            .get(Value::from("skip-cert-verify"))
            .unwrap()
            .as_bool()
            .unwrap()
    );
    let grpc_opts = grpc_map.get(Value::from("grpc-opts")).unwrap().as_mapping().unwrap();
    assert_eq!(
        grpc_opts
            .get(Value::from("grpc-service-name"))
            .unwrap()
            .as_str()
            .unwrap(),
        "myGrpcSvc"
    );

    let xhttp_link = "vless://11111111-1111-1111-1111-111111111111@xhttp.example.com:443?type=xhttp&mode=packet-up&host=x.cdn.com&path=%2Fxhttp&extra=%7B%22noGRPCHeader%22%3Atrue%2C%22xPaddingBytes%22%3A%22100-500%22%7D&security=tls&encryption=mlkem768x25519plus.native.0rtt#VLESS-XHTTP";
    let xhttp_map = parse_vless(xhttp_link).unwrap();
    assert_eq!(
        xhttp_map.get(Value::from("network")).unwrap().as_str().unwrap(),
        "xhttp"
    );
    assert_eq!(
        xhttp_map.get(Value::from("encryption")).unwrap().as_str().unwrap(),
        "mlkem768x25519plus.native.0rtt"
    );
    let xhttp_opts = xhttp_map.get(Value::from("xhttp-opts")).unwrap().as_mapping().unwrap();
    assert_eq!(
        xhttp_opts.get(Value::from("mode")).unwrap().as_str().unwrap(),
        "packet-up"
    );
    assert_eq!(xhttp_opts.get(Value::from("path")).unwrap().as_str().unwrap(), "/xhttp");
    assert!(
        xhttp_opts
            .get(Value::from("no-grpc-header"))
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        xhttp_opts
            .get(Value::from("x-padding-bytes"))
            .unwrap()
            .as_str()
            .unwrap(),
        "100-500"
    );

    let hu_link = "vless://11111111-1111-1111-1111-111111111111@hu.example.com:443?type=httpupgrade&host=hu.cdn.com&path=%2Fhu&security=tls#VLESS-HU";
    let hu_map = parse_vless(hu_link).unwrap();
    assert_eq!(hu_map.get(Value::from("network")).unwrap().as_str().unwrap(), "ws");
    let ws_opts = hu_map.get(Value::from("ws-opts")).unwrap().as_mapping().unwrap();
    assert!(
        ws_opts
            .get(Value::from("v2ray-http-upgrade"))
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

#[test]
fn test_parse_trojan_transports_and_reality() {
    let ws_link = "trojan://secret@ws.trojan.com:443?type=ws&path=%2Ftrws&host=cdn.trojan.com&sni=cdn.trojan.com&alpn=http/1.1&fp=firefox&insecure=1#Trojan-WS";
    let ws_map = parse_trojan(ws_link).unwrap();
    assert_eq!(ws_map.get(Value::from("network")).unwrap().as_str().unwrap(), "ws");
    assert!(ws_map.get(Value::from("skip-cert-verify")).unwrap().as_bool().unwrap());
    assert_eq!(
        ws_map.get(Value::from("client-fingerprint")).unwrap().as_str().unwrap(),
        "firefox"
    );
    let ws_opts = ws_map.get(Value::from("ws-opts")).unwrap().as_mapping().unwrap();
    assert_eq!(ws_opts.get(Value::from("path")).unwrap().as_str().unwrap(), "/trws");

    let grpc_link = "trojan://secret@grpc.trojan.com:443?type=grpc&serviceName=trGrpc&pbk=PUBKEY&sid=01ab#Trojan-gRPC";
    let grpc_map = parse_trojan(grpc_link).unwrap();
    assert_eq!(grpc_map.get(Value::from("network")).unwrap().as_str().unwrap(), "grpc");
    assert_eq!(
        grpc_map
            .get(Value::from("grpc-opts"))
            .unwrap()
            .as_mapping()
            .unwrap()
            .get(Value::from("grpc-service-name"))
            .unwrap()
            .as_str()
            .unwrap(),
        "trGrpc"
    );
    assert_eq!(
        grpc_map
            .get(Value::from("reality-opts"))
            .unwrap()
            .as_mapping()
            .unwrap()
            .get(Value::from("public-key"))
            .unwrap()
            .as_str()
            .unwrap(),
        "PUBKEY"
    );
}

#[test]
fn test_parse_ss_plugins_and_ssr_and_shadowrocket_vmess() {
    let user_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("2022-blake3-aes-128-gcm:serverkey:userkey");
    let ss_obfs = format!(
        "SS://{user_b64}@ss.example.com:8388?plugin=obfs-local%3Bobfs%3Dtls%3Bobfs-host%3Dbing.com&uot=1#SS-Obfs"
    );
    let ss_map = parse_ss(&ss_obfs).unwrap();
    assert_eq!(
        ss_map.get(Value::from("password")).unwrap().as_str().unwrap(),
        "serverkey:userkey"
    );
    assert_eq!(ss_map.get(Value::from("plugin")).unwrap().as_str().unwrap(), "obfs");
    assert!(ss_map.get(Value::from("udp-over-tcp")).unwrap().as_bool().unwrap());
    let obfs_opts = ss_map.get(Value::from("plugin-opts")).unwrap().as_mapping().unwrap();
    assert_eq!(obfs_opts.get(Value::from("mode")).unwrap().as_str().unwrap(), "tls");
    assert_eq!(
        obfs_opts.get(Value::from("host")).unwrap().as_str().unwrap(),
        "bing.com"
    );

    // SSR: server:port:protocol:method:obfs:base64pass/?obfsparam=...&protoparam=...&remarks=...
    let b64 = |s: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s);
    let ssr_raw = format!(
        "ssr.example.com:8388:auth_aes128_md5:aes-256-cfb:tls1.2_ticket_auth:{}/?obfsparam={}&protoparam={}&remarks={}",
        b64("pass123"),
        b64("cdn.example.com"),
        b64("1001:user"),
        b64("香港 SSR")
    );
    let ssr_link = format!("ssr://{}", b64(&ssr_raw));
    let ssr_map = parse_ssr(&ssr_link).unwrap();
    assert_eq!(ssr_map.get(Value::from("type")).unwrap().as_str().unwrap(), "ssr");
    assert_eq!(ssr_map.get(Value::from("name")).unwrap().as_str().unwrap(), "香港 SSR");
    assert_eq!(
        ssr_map.get(Value::from("password")).unwrap().as_str().unwrap(),
        "pass123"
    );
    assert_eq!(
        ssr_map.get(Value::from("obfs-param")).unwrap().as_str().unwrap(),
        "cdn.example.com"
    );
    assert_eq!(
        ssr_map.get(Value::from("protocol-param")).unwrap().as_str().unwrap(),
        "1001:user"
    );

    // Shadowrocket VMess URI
    let sr_vmess = "vmess://chacha20-poly1305:11111111-1111-1111-1111-111111111111@vm.example.com:443?remarks=HK-SR&obfs=websocket&path=/ws&obfsParam=%7B%22Host%22%3A%22cdn.vm.com%22%7D&tls=1&allowInsecure=1&fp=safari";
    let vm_map = parse_vmess(sr_vmess).unwrap();
    assert_eq!(vm_map.get(Value::from("name")).unwrap().as_str().unwrap(), "HK-SR");
    assert_eq!(
        vm_map.get(Value::from("cipher")).unwrap().as_str().unwrap(),
        "chacha20-poly1305"
    );
    assert_eq!(vm_map.get(Value::from("network")).unwrap().as_str().unwrap(), "ws");
    assert_eq!(
        vm_map.get(Value::from("servername")).unwrap().as_str().unwrap(),
        "cdn.vm.com"
    );
    assert!(vm_map.get(Value::from("skip-cert-verify")).unwrap().as_bool().unwrap());
    assert_eq!(
        vm_map.get(Value::from("client-fingerprint")).unwrap().as_str().unwrap(),
        "safari"
    );
}

#[test]
fn test_parse_uri_list_case_insensitive_all_new_schemes() {
    let b64 = |s: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s);
    let wg_key = "WG://key123@wg.example.com:51820?address=10.0.0.2/32#WG-Upper";
    let socks_b64 = format!(
        "SOCKS://{}@socks.example.com:1080?tls=1&insecure=1#Socks-TLS",
        b64("alice:bob")
    );
    let hy2_link = "HY2://pw@hy2.example.com:443#Hy2-Upper";
    let input = format!("{wg_key}\n{socks_b64}\n{hy2_link}");

    let mapped = parse_uri_list(&input).unwrap();
    let proxies = mapped.get(Value::from("proxies")).unwrap().as_sequence().unwrap();
    assert_eq!(proxies.len(), 3);
    assert_eq!(proxies[0].get("type").unwrap().as_str().unwrap(), "wireguard");
    assert_eq!(proxies[1].get("username").unwrap().as_str().unwrap(), "alice");
    assert!(proxies[1].get("tls").unwrap().as_bool().unwrap());
    assert_eq!(proxies[2].get("type").unwrap().as_str().unwrap(), "hysteria2");
}

#[test]
fn test_parse_subscription_payload_singbox_and_sip008_json() {
    let singbox = r#"{
      "outbounds": [
        { "type": "direct", "tag": "direct" },
        { "type": "selector", "tag": "select", "outbounds": ["relay", "hy2-node"] },
        {
          "type": "vless",
          "tag": "relay",
          "server": "relay.example.com",
          "server_port": 443,
          "uuid": "11111111-1111-1111-1111-111111111111",
          "flow": "xtls-rprx-vision",
          "tls": {
            "enabled": true,
            "server_name": "www.microsoft.com",
            "utls": { "enabled": true, "fingerprint": "chrome" },
            "reality": { "enabled": true, "public_key": "PUB123", "short_id": "0102" }
          }
        },
        {
          "type": "hysteria2",
          "tag": "hy2-node",
          "server": "hy2.example.com",
          "server_port": 443,
          "server_ports": ["20000:30000"],
          "hop_interval": "30s",
          "password": "secret",
          "up_mbps": 50,
          "down_mbps": 200,
          "detour": "relay",
          "tls": { "enabled": true, "server_name": "hy2.example.com", "insecure": true }
        },
        {
          "type": "wireguard",
          "tag": "wg-exit",
          "private_key": "PRIV",
          "address": ["10.0.0.2/32", "fd00::2/128"],
          "peers": [{ "address": "1.2.3.4", "port": 51820, "public_key": "PUB", "reserved": [1, 2, 3] }]
        }
      ]
    }"#;

    let (sb_map, _) = parse_subscription_payload(singbox).unwrap();
    let sb_proxies = sb_map.get("proxies").unwrap().as_sequence().unwrap();
    assert_eq!(sb_proxies.len(), 3);
    assert_eq!(sb_proxies[0].get("type").unwrap().as_str().unwrap(), "vless");
    assert_eq!(sb_proxies[1].get("ports").unwrap().as_str().unwrap(), "20000-30000");
    assert_eq!(sb_proxies[1].get("hop-interval").unwrap().as_u64().unwrap(), 30);
    assert_eq!(sb_proxies[1].get("dialer-proxy").unwrap().as_str().unwrap(), "relay");
    assert_eq!(sb_proxies[2].get("server").unwrap().as_str().unwrap(), "1.2.3.4");
    assert_eq!(sb_proxies[2].get("ipv6").unwrap().as_str().unwrap(), "fd00::2");

    // Base64-encoded SIP008 JSON
    let sip008 = r#"{
      "version": 1,
      "servers": [
        {
          "server": "ss.example.com",
          "server_port": 8388,
          "password": "pw",
          "method": "aes-256-gcm",
          "remarks": "SIP008-HK",
          "plugin": "v2ray-plugin",
          "plugin_opts": "mode=websocket;tls;host=cdn.ss.example;path=/ws"
        }
      ]
    }"#;
    let sip_b64 = base64::engine::general_purpose::STANDARD.encode(sip008);
    let (sip_map, _) = parse_subscription_payload(&sip_b64).unwrap();
    let sip_proxies = sip_map.get("proxies").unwrap().as_sequence().unwrap();
    assert_eq!(sip_proxies.len(), 1);
    assert_eq!(sip_proxies[0].get("name").unwrap().as_str().unwrap(), "SIP008-HK");
    assert_eq!(sip_proxies[0].get("plugin").unwrap().as_str().unwrap(), "v2ray-plugin");
    assert!(
        sip_proxies[0]
            .get("plugin-opts")
            .unwrap()
            .as_mapping()
            .unwrap()
            .get("tls")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

#[test]
fn test_double_check_hardening_cases() {
    // 1. 空主机名拦截
    assert!(parse_hysteria2("hy2://pass@:443#bad").is_none());
    assert!(parse_vless("vless://uuid@[]:443#bad").is_none());

    // 2. Hysteria2 authority 多端口与端口跳跃写法
    let hy2_auth = parse_hysteria2("hy2://pw@example.com:20000-30000?sni=example.com#Hy2-AuthHop").unwrap();
    assert_eq!(hy2_auth.get(Value::from("port")).unwrap().as_u64().unwrap(), 20000);
    assert_eq!(
        hy2_auth.get(Value::from("ports")).unwrap().as_str().unwrap(),
        "20000-30000"
    );

    let hy2_v6_list = parse_hysteria2("hy2://pw@[2001:db8::1]:443,20000-30000#Hy2-V6Hop").unwrap();
    assert_eq!(
        hy2_v6_list.get(Value::from("server")).unwrap().as_str().unwrap(),
        "2001:db8::1"
    );
    assert_eq!(hy2_v6_list.get(Value::from("port")).unwrap().as_u64().unwrap(), 443);
    assert_eq!(
        hy2_v6_list.get(Value::from("ports")).unwrap().as_str().unwrap(),
        "443,20000-30000"
    );

    // 3. hysteria up/down 与 vless pbk/fp 百分号解码
    let hy1 = parse_hysteria("hysteria://1.2.3.4:443?auth=a&up=100%20Mbps&down=200%20Mbps#hy").unwrap();
    assert_eq!(hy1.get(Value::from("up")).unwrap().as_str().unwrap(), "100 Mbps");
    assert_eq!(hy1.get(Value::from("down")).unwrap().as_str().unwrap(), "200 Mbps");
    let vl = parse_vless(
        "vless://11111111-1111-1111-1111-111111111111@1.2.3.4:443?security=reality&pbk=AB%2BCD&fp=chrome%20ios#vl",
    )
    .unwrap();
    assert_eq!(
        vl.get(Value::from("reality-opts"))
            .unwrap()
            .as_mapping()
            .unwrap()
            .get(Value::from("public-key"))
            .unwrap()
            .as_str()
            .unwrap(),
        "AB+CD"
    );
    assert_eq!(
        vl.get(Value::from("client-fingerprint")).unwrap().as_str().unwrap(),
        "chrome ios"
    );

    // 4. sing-box detour 过滤（保留已转换节点、剥离 direct/selector），utls.enabled=false 尊重，
    //    server_ports 推导 port，WS early-data 与小写 headers.host，1.11 endpoints WG
    let sb_json = r#"{
      "outbounds": [
        { "type": "direct", "tag": "direct" },
        { "type": "selector", "tag": "select", "outbounds": ["relay"] },
        {
          "type": "vless",
          "tag": "relay",
          "server": "relay.example.com",
          "server_port": 443,
          "uuid": "11111111-1111-1111-1111-111111111111",
          "detour": "direct",
          "tls": { "enabled": true, "utls": { "enabled": false, "fingerprint": "chrome" } },
          "transport": { "type": "ws", "path": "/ws", "headers": { "host": "ws.cdn.example" }, "max_early_data": 2048, "early_data_header_name": "Sec-WebSocket-Protocol" }
        },
        {
          "type": "hysteria2",
          "tag": "hy2-range-only",
          "server": "hy2.example.com",
          "server_ports": ["30000:40000"],
          "password": "pw",
          "detour": "relay",
          "tls": { "server_name": "hy2.example.com" }
        }
      ],
      "endpoints": [
        {
          "type": "wireguard",
          "tag": "wg-endpoint",
          "private_key": "PRIV",
          "address": ["10.0.0.2/32"],
          "peers": [{ "address": "5.6.7.8", "port": 51820, "public_key": "PUB" }]
        }
      ]
    }"#;
    let (sb_map, _) = parse_subscription_payload(sb_json).unwrap();
    let proxies = sb_map.get("proxies").unwrap().as_sequence().unwrap();
    assert_eq!(proxies.len(), 3);
    assert!(proxies[0].get("dialer-proxy").is_none(), "detour=direct 必须剥离");
    assert!(
        proxies[0].get("client-fingerprint").is_none(),
        "utls.enabled=false 不应输出指纹"
    );
    let ws_opts = proxies[0].get("ws-opts").unwrap().as_mapping().unwrap();
    assert_eq!(ws_opts.get("max-early-data").unwrap().as_u64().unwrap(), 2048);
    assert_eq!(
        ws_opts
            .get("headers")
            .unwrap()
            .as_mapping()
            .unwrap()
            .get("Host")
            .unwrap()
            .as_str()
            .unwrap(),
        "ws.cdn.example"
    );
    assert_eq!(proxies[1].get("port").unwrap().as_u64().unwrap(), 30000);
    assert_eq!(proxies[1].get("dialer-proxy").unwrap().as_str().unwrap(), "relay");
    assert_eq!(proxies[2].get("name").unwrap().as_str().unwrap(), "wg-endpoint");
}
