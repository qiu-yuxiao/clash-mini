#![allow(clippy::expect_used, clippy::unwrap_used)]

#[tokio::test]
async fn test_enforce_mini_agreements_logic() {
    use super::agreements::enforce_mini_agreements;
    use serde_yaml_ng::{Mapping, Value};

    // `enforce_mini_agreements` 经 `Config::verge()` → `Config::global()` →
    // `IClashTemp::new()` → `dirs::clash_path()` → `app_home_dir()` 触达全局
    // `APP_HANDLE`。单测环境没有 Tauri AppHandle，若走非 portable 分支会
    // panic（App handle not initialized）。将 PORTABLE_FLAG 置为 true 使
    // `app_home_dir()` 走 portable 分支（仅依赖 current_exe，不碰 app_handle），
    // 从而让本测试自包含、可单独运行——与本仓库其它单测（save_profile / prfitem）一致。
    let _ = crate::utils::dirs::PORTABLE_FLAG.get_or_init(|| true);

    let config_str = r#"
proxies:
  - name: "node-A"
    type: ss
  - name: "node-B"
    type: vmess
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
    health-check:
      enable: true
      url: https://www.gstatic.com/generate_204
      interval: 60
      lazy: false
proxy-groups:
  - name: "Group-A"
    type: select
    proxies:
      - "node-A"
  - name: "Group-B"
    type: select
    proxies:
      - "node-B"
rule-providers:
  China-Site:
    type: http
    behavior: domain
    format: mrs
    url: https://example.com/China-Site.mrs
    path: ./ruleset/China-Site.mrs
  Custom-RuleSet:
    type: http
    behavior: domain
    url: https://example.com/custom.yaml
    path: ./ruleset/custom.yaml
prepend-rules:
  - PROCESS-NAME,custom-process,DIRECT
append-rules:
  - DOMAIN,custom-domain,REJECT
"#;

    let mut config: Mapping = serde_yaml_ng::from_str(config_str).unwrap();
    config = enforce_mini_agreements(config).await;

    // 1. Verify PROXY (control) + PROXY__METRICS (measurement) groups
    let groups = config.get("proxy-groups").and_then(Value::as_sequence).unwrap();
    assert_eq!(groups.len(), 2);

    let proxy_group = groups
        .iter()
        .find(|g| g.get("name").and_then(Value::as_str) == Some("PROXY"))
        .and_then(Value::as_mapping)
        .unwrap();
    assert_eq!(proxy_group.get("type").unwrap().as_str(), Some("select"));

    let metrics_group = groups
        .iter()
        .find(|g| g.get("name").and_then(Value::as_str) == Some("PROXY__METRICS"))
        .and_then(Value::as_mapping)
        .unwrap();
    assert_eq!(metrics_group.get("type").unwrap().as_str(), Some("url-test"));
    assert!(metrics_group.get("url").is_some());
    assert_eq!(metrics_group.get("interval").unwrap().as_u64(), Some(1800));
    assert_eq!(metrics_group.get("lazy").unwrap().as_bool(), Some(true));

    let provider_hc_lazy = config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .and_then(|m| m.get("providerA"))
        .and_then(Value::as_mapping)
        .and_then(|m| m.get("health-check"))
        .and_then(Value::as_mapping)
        .and_then(|m| m.get("lazy"))
        .and_then(Value::as_bool);
    assert_eq!(provider_hc_lazy, Some(true));

    let group_proxies = proxy_group.get("proxies").and_then(Value::as_sequence).unwrap();
    assert_eq!(group_proxies.len(), 2);
    assert_eq!(group_proxies[0].as_str(), Some("node-A"));
    assert_eq!(group_proxies[1].as_str(), Some("node-B"));

    let group_uses = proxy_group.get("use").and_then(Value::as_sequence).unwrap();
    assert_eq!(group_uses.len(), 1);
    assert_eq!(group_uses[0].as_str(), Some("providerA"));

    // 2. Verify gfwlist provider & rules (pre-bundled + 24h periodic update via PROXY),
    //    and verify unreferenced airport rule-providers (China-Site, Custom-RuleSet) are stripped
    let providers = config.get("rule-providers").and_then(Value::as_mapping).unwrap();
    assert_eq!(providers.len(), 1);
    assert!(providers.get("China-Site").is_none());
    assert!(providers.get("Custom-RuleSet").is_none());
    let gfwlist = providers.get("gfwlist").and_then(Value::as_mapping).unwrap();
    assert_eq!(gfwlist.get("type").unwrap().as_str(), Some("http"));
    assert_eq!(gfwlist.get("behavior").unwrap().as_str(), Some("domain"));
    assert_eq!(
        gfwlist.get("url").unwrap().as_str(),
        Some("https://cdn.jsdelivr.net/gh/Loyalsoldier/clash-rules@release/gfw.txt")
    );
    assert_eq!(gfwlist.get("path").unwrap().as_str(), Some("./ruleset/gfwlist.yaml"));
    assert_eq!(gfwlist.get("interval").unwrap().as_u64(), Some(86400));
    assert_eq!(gfwlist.get("proxy").unwrap().as_str(), Some("PROXY"));

    let rules = config.get("rules").and_then(Value::as_sequence).unwrap();
    // prepend(1) + GFWList(1) + IP-CIDR(6) + IP-CIDR6(5) + IPv4-mapped(6)
    // + append(1) + GEOIP,CN(1) + MATCH(1) = 22
    // MATCH=DIRECT because default rule_fallback is "direct" (GFWList mode); IP-CIDR6,::/0,PROXY 已移除
    assert_eq!(rules.len(), 22);
    assert_eq!(rules[0].as_str(), Some("PROCESS-NAME,custom-process,DIRECT"));
    assert_eq!(rules[1].as_str(), Some("RULE-SET,gfwlist,PROXY"));
    assert_eq!(rules[2].as_str(), Some("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
    assert_eq!(rules[3].as_str(), Some("IP-CIDR,172.16.0.0/12,DIRECT,no-resolve"));
    assert_eq!(rules[4].as_str(), Some("IP-CIDR,192.168.0.0/16,DIRECT,no-resolve"));
    assert_eq!(rules[5].as_str(), Some("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve"));
    assert_eq!(rules[6].as_str(), Some("IP-CIDR,100.64.0.0/10,DIRECT,no-resolve"));
    assert_eq!(rules[7].as_str(), Some("IP-CIDR,169.254.0.0/16,DIRECT,no-resolve"));
    assert_eq!(rules[8].as_str(), Some("IP-CIDR6,::1/128,DIRECT,no-resolve"));
    assert_eq!(rules[9].as_str(), Some("IP-CIDR6,::/128,DIRECT,no-resolve"));
    assert_eq!(rules[10].as_str(), Some("IP-CIDR6,fc00::/7,DIRECT,no-resolve"));
    assert_eq!(rules[11].as_str(), Some("IP-CIDR6,fe80::/10,DIRECT,no-resolve"));
    assert_eq!(rules[12].as_str(), Some("IP-CIDR6,ff00::/8,DIRECT,no-resolve"));
    assert_eq!(
        rules[13].as_str(),
        Some("IP-CIDR6,::ffff:127.0.0.0/104,DIRECT,no-resolve")
    );
    assert_eq!(
        rules[14].as_str(),
        Some("IP-CIDR6,::ffff:10.0.0.0/104,DIRECT,no-resolve")
    );
    assert_eq!(
        rules[15].as_str(),
        Some("IP-CIDR6,::ffff:172.16.0.0/108,DIRECT,no-resolve")
    );
    assert_eq!(
        rules[16].as_str(),
        Some("IP-CIDR6,::ffff:192.168.0.0/112,DIRECT,no-resolve")
    );
    assert_eq!(
        rules[17].as_str(),
        Some("IP-CIDR6,::ffff:100.64.0.0/106,DIRECT,no-resolve")
    );
    assert_eq!(
        rules[18].as_str(),
        Some("IP-CIDR6,::ffff:169.254.0.0/112,DIRECT,no-resolve")
    );
    assert_eq!(rules[19].as_str(), Some("DOMAIN,custom-domain,REJECT"));
    // IPv6 兜底（在 MATCH 之前，确保境外/投毒 IPv6 不直连超时）
    // 刻意不带 no-resolve：否则域名目标会跳过这两条规则、仍漏到 MATCH,DIRECT
    // GEOIP 而非 GEOIP6：mihomo 不支持 GEOIP6 规则类型（配置校验直接失败）
    assert_eq!(rules[20].as_str(), Some("GEOIP,CN,DIRECT"));
    assert_eq!(rules[21].as_str(), Some("MATCH,DIRECT"));

    // Verify prepend-rules and append-rules keys are removed
    assert!(config.get("prepend-rules").is_none());
    assert!(config.get("append-rules").is_none());
}

/// 回归：`dns.respect-rules: true` + `rule_fallback = direct` 时，fallback 的
/// 境外 DoH 会被 `MATCH,DIRECT` 兜走并直连失败，导致防投毒链路整体失效
/// （v3.0.0 现场：`dial DIRECT (match Match/) mihomo --> 8.8.8.8:443 error`）。
/// 本用例锁定「按 fallback 里的 IP 字面量生成走代理规则」这一约定。
#[test]
fn dns_fallback_upstreams_are_forced_through_proxy() {
    use super::dns::{collect_dns_field_strings, dns_upstream_proxy_rules_from_hosts, extract_upstream_host};
    use serde_yaml_ng::Mapping;

    // 标量与列表混用 + 各种 scheme / 端口 / h3 参数 / 去重
    let dns: Mapping = serde_yaml_ng::from_str(
        r"
fallback:
  - https://1.1.1.1/dns-query
  - https://8.8.8.8/dns-query#h3=true
  - tls://8.8.4.4:853
  - https://dns.google/dns-query
  - https://[2606:4700:4700::1111]/dns-query
  - 1.1.1.1
  - https://192.168.1.1/dns-query
  - 127.0.0.1
  - 10.0.0.1
  - https://[::1]/dns-query
  - https://[fe80::1]/dns-query
",
    )
    .unwrap();

    let hosts: Vec<std::string::String> = collect_dns_field_strings(&dns, "fallback")
        .iter()
        .filter_map(|raw| extract_upstream_host(raw))
        .collect();
    let rules: Vec<std::string::String> = dns_upstream_proxy_rules_from_hosts(hosts.iter().map(|s| s.as_str()));

    assert_eq!(
        rules,
        vec![
            std::string::String::from("IP-CIDR,1.1.1.1/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,1.0.0.1/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,8.8.8.8/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,8.8.4.4/32,PROXY,no-resolve"),
            std::string::String::from("DOMAIN,dns.google,PROXY"),
            std::string::String::from("IP-CIDR6,2606:4700:4700::1111/128,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR6,2606:4700:4700::1001/128,PROXY,no-resolve"),
        ],
        "域名型上游与双活对偶辅IP（含IPv4/IPv6）生成走代理规则，按出现顺序去重"
    );

    // Quad9 与 OpenDNS 对偶辅 IP 自动补全（A9）
    let quad9_opendns: Mapping =
        serde_yaml_ng::from_str("fallback: ['https://9.9.9.9/dns-query', '208.67.222.222']").unwrap();
    let quad9_hosts: Vec<std::string::String> = collect_dns_field_strings(&quad9_opendns, "fallback")
        .iter()
        .filter_map(|raw| extract_upstream_host(raw))
        .collect();
    let quad9_rules: Vec<std::string::String> =
        dns_upstream_proxy_rules_from_hosts(quad9_hosts.iter().map(|s| s.as_str()));
    assert_eq!(
        quad9_rules,
        vec![
            std::string::String::from("IP-CIDR,9.9.9.9/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,149.112.112.112/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,208.67.222.222/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,208.67.220.220/32,PROXY,no-resolve"),
        ]
    );

    // 单独配置 8.8.8.8 时，应自动补齐对偶辅 IP 8.8.4.4
    let single_google: Mapping = serde_yaml_ng::from_str("fallback: ['https://8.8.8.8/dns-query']").unwrap();
    let single_hosts: Vec<std::string::String> = collect_dns_field_strings(&single_google, "fallback")
        .iter()
        .filter_map(|raw| extract_upstream_host(raw))
        .collect();
    let single_rules: Vec<std::string::String> =
        dns_upstream_proxy_rules_from_hosts(single_hosts.iter().map(|s| s.as_str()));
    assert_eq!(
        single_rules,
        vec![
            std::string::String::from("IP-CIDR,8.8.8.8/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,8.8.4.4/32,PROXY,no-resolve"),
        ]
    );

    // 单独配置 dns.google 时，应同时生成 DOMAIN 与 8.8.8.8/8.8.4.4 落地规则
    let domain_google: Mapping = serde_yaml_ng::from_str("fallback: ['https://dns.google/dns-query']").unwrap();
    let domain_hosts: Vec<std::string::String> = collect_dns_field_strings(&domain_google, "fallback")
        .iter()
        .filter_map(|raw| extract_upstream_host(raw))
        .collect();
    let domain_rules: Vec<std::string::String> =
        dns_upstream_proxy_rules_from_hosts(domain_hosts.iter().map(|s| s.as_str()));
    assert_eq!(
        domain_rules,
        vec![
            std::string::String::from("DOMAIN,dns.google,PROXY"),
            std::string::String::from("IP-CIDR,8.8.8.8/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,8.8.4.4/32,PROXY,no-resolve"),
        ]
    );

    // 没有上游时不得凭空造规则
    assert!(dns_upstream_proxy_rules_from_hosts(std::iter::empty::<&str>()).is_empty());
}

/// 回归（BUG-308 / C10）：订阅自带 `dns.nameserver` / `dns.fallback` 含境外明文 UDP 上游（如 8.8.8.8）时，
/// `respect-rules: true` 下该查询穿代理，TCP 承载节点（anytls）上 UDP-over-TCP 会话
/// 建立慢于境内 DoH、在 DNS 并发赛跑中必被取消（3.1.7 现场实证 7 条
/// `failed to create session` 全落在 anytls 窗口内）；`respect-rules: false` 下直连必超时的
/// 境外加密 DoH/DoT 与明文上游统一就地降级为境内 DoH。锁定「明文境外公共 DNS → DoH
/// 改写并去重、境内明文与加密上游保留、default-nameserver 不动」的卫生改写约定。
#[test]
fn foreign_plain_dns_upstreams_in_nameserver_are_sanitized_to_doh() {
    use super::dns::sanitize_dns_upstreams;
    use serde_yaml_ng::Mapping;

    let config: Mapping = serde_yaml_ng::from_str(
        r"
dns:
  respect-rules: true
  nameserver:
    - https://doh.pub/dns-query
    - https://dns.alidns.com/dns-query
    - 8.8.8.8
    - 8.8.8.8:53
    - 1.1.1.1
    - 223.5.5.5
    - 114.114.114.114
    - tls://8.8.4.4:853
    - https://8.8.4.4/dns-query
  default-nameserver:
    - 223.6.6.6
    - 8.8.8.8
rules:
  - RULE-SET,gfwlist,PROXY
  - MATCH,DIRECT
",
    )
    .unwrap();

    let sanitized = sanitize_dns_upstreams(config);
    let dns = sanitized.get("dns").unwrap();
    let nameservers: Vec<std::string::String> = dns
        .get("nameserver")
        .unwrap()
        .as_sequence()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();

    assert_eq!(
        nameservers,
        vec![
            std::string::String::from("https://doh.pub/dns-query"),
            std::string::String::from("https://dns.alidns.com/dns-query"),
            // 境外明文 UDP → DoH 改写并去重（裸 IP 与 :53 端口合并）
            std::string::String::from("https://8.8.8.8/dns-query"),
            std::string::String::from("https://1.1.1.1/dns-query"),
            // 境内明文上游直连可达，保留
            std::string::String::from("223.5.5.5"),
            std::string::String::from("114.114.114.114"),
            // 已加密上游原样保留
            std::string::String::from("tls://8.8.4.4:853"),
            std::string::String::from("https://8.8.4.4/dns-query"),
        ],
        "仅境外公共 DNS 明文形式被改写为 DoH"
    );

    // respect-rules: true 时 nameserver 中的境外上游亦自动补齐 PROXY 保通规则
    let rules: Vec<&str> = sanitized
        .get("rules")
        .unwrap()
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(rules.contains(&"IP-CIDR,8.8.8.8/32,PROXY,no-resolve"));
    assert!(rules.contains(&"IP-CIDR,8.8.4.4/32,PROXY,no-resolve"));
    assert!(rules.contains(&"IP-CIDR,1.1.1.1/32,PROXY,no-resolve"));
    assert!(!rules.contains(&"IP-CIDR,223.5.5.5/32,PROXY,no-resolve"));
    assert!(!rules.contains(&"IP-CIDR,114.114.114.114/32,PROXY,no-resolve"));

    // default-nameserver 按协议定义必须为明文 IP，不得改写
    let defaults: Vec<std::string::String> = dns
        .get("default-nameserver")
        .unwrap()
        .as_sequence()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(
        defaults,
        vec![
            std::string::String::from("223.6.6.6"),
            std::string::String::from("8.8.8.8"),
        ],
        "default-nameserver 保持明文不动"
    );

    // 无 dns 段 / 无 nameserver 时安全跳过
    let no_dns: Mapping = serde_yaml_ng::from_str("mixed-port: 7890").unwrap();
    let untouched = sanitize_dns_upstreams(no_dns);
    assert!(untouched.get("dns").is_none());
    let no_nameserver: Mapping = serde_yaml_ng::from_str("dns:\n  enable: true").unwrap();
    let untouched = sanitize_dns_upstreams(no_nameserver);
    assert!(untouched.get("dns").unwrap().get("nameserver").is_none());
}

#[test]
fn dns_upstream_proxy_rules_skip_domestic_public_dns() {
    use super::dns::{collect_dns_field_strings, dns_upstream_proxy_rules_from_hosts, extract_upstream_host};
    use serde_yaml_ng::Mapping;

    // C11：境内公共 DNS IP / DoH 域名即使误配在 fallback 中也不得生成 PROXY 规则
    let dns: Mapping = serde_yaml_ng::from_str(
        r"
fallback:
  - 223.5.5.5
  - 114.114.114.114
  - https://doh.pub/dns-query
  - https://dns.alidns.com/dns-query
  - https://1.1.1.1/dns-query
",
    )
    .unwrap();

    let hosts: Vec<std::string::String> = collect_dns_field_strings(&dns, "fallback")
        .iter()
        .filter_map(|raw| extract_upstream_host(raw))
        .collect();
    let rules: Vec<std::string::String> = dns_upstream_proxy_rules_from_hosts(hosts.iter().map(|s| s.as_str()));
    assert_eq!(
        rules,
        vec![
            std::string::String::from("IP-CIDR,1.1.1.1/32,PROXY,no-resolve"),
            std::string::String::from("IP-CIDR,1.0.0.1/32,PROXY,no-resolve"),
        ]
    );
}

#[test]
fn extract_upstream_host_handles_all_mihomo_forms() {
    use super::dns::extract_upstream_host;

    assert_eq!(extract_upstream_host("1.1.1.1").as_deref(), Some("1.1.1.1"));
    assert_eq!(extract_upstream_host("8.8.8.8:53").as_deref(), Some("8.8.8.8"));
    assert_eq!(
        extract_upstream_host("https://1.1.1.1/dns-query").as_deref(),
        Some("1.1.1.1")
    );
    assert_eq!(extract_upstream_host("tls://8.8.4.4:853").as_deref(), Some("8.8.4.4"));
    assert_eq!(
        extract_upstream_host("https://dns.google/dns-query#h3=true").as_deref(),
        Some("dns.google")
    );
    assert_eq!(
        extract_upstream_host("https://[2606:4700:4700::1111]/dns-query").as_deref(),
        Some("2606:4700:4700::1111")
    );
    // 裸 IPv6 不带方括号
    assert_eq!(
        extract_upstream_host("2606:4700:4700::1001").as_deref(),
        Some("2606:4700:4700::1001")
    );
    assert_eq!(extract_upstream_host("   ").as_deref(), None);
}

#[tokio::test]
async fn test_enforce_mini_agreements_deduplicates_clones_and_skips_confirmed_fake() {
    use super::agreements::enforce_mini_agreements;
    use serde_yaml_ng::{Mapping, Value};

    let _ = crate::utils::dirs::PORTABLE_FLAG.get_or_init(|| true);

    let yaml_str = r"
proxies:
  - name: 距离下次重置剩余：25 天
    type: ss
    server: 198.51.100.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-real-clone
  - name: 香港1
    type: ss
    server: 198.51.100.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-real-clone
  - name: 伪装正常名实则已拉黑
    type: ss
    server: 198.51.100.99
    port: 8443
    cipher: aes-128-gcm
    password: pass-confirmed-fake
";
    let config: Mapping = serde_yaml_ng::from_str(yaml_str).unwrap();
    let fake_mapping = config
        .get("proxies")
        .and_then(Value::as_sequence)
        .and_then(|s| s[2].as_mapping())
        .unwrap();
    let fake_fp = crate::utils::node::proxy_endpoint_fingerprint(fake_mapping).unwrap();

    let now_ts = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or(i64::MAX);

    // 先验证超过 24h 的过期隔离记录不会阻止节点入组
    crate::utils::node::set_verify_record_for_test(
        &fake_fp,
        Some(crate::utils::node::NodeVerifyRecord {
            verified_true: false,
            fail_count: crate::utils::node::HTTPS_VERIFY_MAX_FAILS + 1,
            last_verified_at: 100,
        }),
    );
    let enforced_expired = enforce_mini_agreements(config.clone()).await;
    let expired_proxies = enforced_expired
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .and_then(|g| g[0].as_mapping())
        .and_then(|m| m.get("proxies"))
        .and_then(Value::as_sequence)
        .unwrap();
    let expired_names: Vec<&str> = expired_proxies.iter().filter_map(Value::as_str).collect();
    assert_eq!(expired_names, vec!["香港1", "伪装正常名实则已拉黑"]);

    // 在 24h 软隔离窗口内时必须跳过该节点
    crate::utils::node::set_verify_record_for_test(
        &fake_fp,
        Some(crate::utils::node::NodeVerifyRecord {
            verified_true: false,
            fail_count: crate::utils::node::HTTPS_VERIFY_MAX_FAILS + 1,
            last_verified_at: now_ts,
        }),
    );

    let enforced = enforce_mini_agreements(config).await;
    crate::utils::node::set_verify_record_for_test(&fake_fp, None);

    let groups = enforced.get("proxy-groups").and_then(Value::as_sequence).unwrap();
    assert_eq!(groups.len(), 2);
    let proxy_proxies = groups[0]
        .as_mapping()
        .unwrap()
        .get("proxies")
        .and_then(Value::as_sequence)
        .unwrap();
    let names: Vec<&str> = proxy_proxies.iter().filter_map(Value::as_str).collect();
    assert_eq!(names, vec!["香港1"]);
}

#[test]
fn test_apply_builtin_guards_mode_and_hysteria_alpn() {
    use super::apply_builtin_guards;
    use serde_yaml_ng::{Mapping, Value};

    let yaml_str = r"
mode: script
proxies:
  - name: hy1
    type: hysteria
    server: 198.51.100.1
    port: 443
    alpn: h3
  - name: hy_arr
    type: hysteria
    server: 198.51.100.2
    port: 443
    alpn:
      - h3
  - name: hy2
    type: hysteria2
    server: 198.51.100.3
    port: 443
    alpn: h3
";
    let input: Mapping = serde_yaml_ng::from_str(yaml_str).unwrap();

    // enable_builtin = false 时不做任何改写
    let disabled = apply_builtin_guards(input.clone(), Some("mini-mihomo"), false);
    assert_eq!(disabled.get("mode").and_then(Value::as_str), Some("script"));

    // 启用内建卫兵：mode: script -> rule；hysteria 字符串 alpn -> 单元素数组；其余不变
    let guarded = apply_builtin_guards(input, Some("mini-mihomo"), true);
    assert_eq!(guarded.get("mode").and_then(Value::as_str), Some("rule"));
    let proxies = guarded.get("proxies").and_then(Value::as_sequence).unwrap();
    assert_eq!(
        proxies[0].as_mapping().unwrap().get("alpn"),
        Some(&Value::Sequence(vec![Value::String("h3".into())]))
    );
    assert_eq!(
        proxies[1].as_mapping().unwrap().get("alpn"),
        Some(&Value::Sequence(vec![Value::String("h3".into())]))
    );
    assert_eq!(
        proxies[2].as_mapping().unwrap().get("alpn").and_then(Value::as_str),
        Some("h3")
    );

    // 其他有效 mode 不被覆盖
    let mut rule_cfg = Mapping::new();
    rule_cfg.insert(Value::from("mode"), Value::from("global"));
    let kept = apply_builtin_guards(rule_cfg, None, true);
    assert_eq!(kept.get("mode").and_then(Value::as_str), Some("global"));
}

#[tokio::test]
async fn test_enforce_mini_agreements_eliminates_hk_backed_banner_clones() {
    use super::agreements::enforce_mini_agreements;
    use serde_yaml_ng::{Mapping, Value};

    let _ = crate::utils::dirs::PORTABLE_FLAG.get_or_init(|| true);

    // 复现用户订阅真实结构：4 条信息横幅全部绑定同一真实香港后端，混入任意未知广告文案；
    // 另外 2 条挂载在真「日本1」端点上的横幅（未知广告文案排在前面）。
    let yaml_str = r"
proxies:
  - name: 剩余流量：167.26 GB (045017)
    type: ss
    server: 203.0.113.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-banner-shared
  - name: 距离下次重置剩余：24 天 (045017)
    type: ss
    server: 203.0.113.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-banner-shared
  - name: 套餐到期：2027-02-09 (045017)
    type: ss
    server: 203.0.113.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-banner-shared
  - name: 永久官网:666.boostqz.com (045017)
    type: ss
    server: 203.0.113.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-banner-shared
  - name: 全新盲盒活动：详情见频道
    type: ss
    server: 203.0.113.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-banner-shared
  - name: 客服通知：欢迎使用
    type: ss
    server: 203.0.113.20
    port: 8443
    cipher: aes-128-gcm
    password: pass-jp-real
  - name: 日本1 (045017)
    type: ss
    server: 203.0.113.20
    port: 8443
    cipher: aes-128-gcm
    password: pass-jp-real
";
    let config: Mapping = serde_yaml_ng::from_str(yaml_str).unwrap();
    let enforced = enforce_mini_agreements(config).await;

    let groups = enforced.get("proxy-groups").and_then(Value::as_sequence).unwrap();
    let proxy_proxies = groups[0]
        .as_mapping()
        .unwrap()
        .get("proxies")
        .and_then(Value::as_sequence)
        .unwrap();
    let names: Vec<&str> = proxy_proxies.iter().filter_map(Value::as_str).collect();
    // 横幅专用端点的全部 5 条连坐清零；寄生于真节点的未知文案折叠归真为「日本1 (045017)」
    assert_eq!(names, vec!["日本1 (045017)"]);

    // 端口复用校验：后续配置把同一端点重新分配给真实香港节点时，不得被前序内存态连坐误伤
    let reassigned_yaml = r"
proxies:
  - name: 香港1 (045017)
    type: ss
    server: 203.0.113.10
    port: 8443
    cipher: aes-128-gcm
    password: pass-banner-shared
";
    let reassigned: Mapping = serde_yaml_ng::from_str(reassigned_yaml).unwrap();
    let enforced_reassigned = enforce_mini_agreements(reassigned).await;
    let reassigned_names: Vec<&str> = enforced_reassigned
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .and_then(|g| g[0].as_mapping())
        .and_then(|m| m.get("proxies"))
        .and_then(Value::as_sequence)
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(reassigned_names, vec!["香港1 (045017)"]);
}

#[tokio::test]
async fn test_bug405_store_selected_disabled_and_proxy_head_promoted() {
    use super::agreements::{apply_proxy_group_head_and_store_selected, enforce_mini_agreements};
    use serde_yaml_ng::{Mapping, Value};

    let _ = crate::utils::dirs::PORTABLE_FLAG.get_or_init(|| true);

    let yaml_str = r"
profile:
  store-selected: true
proxies:
  - name: 香港1 (184629)
    type: ss
    server: 198.51.100.1
    port: 443
    cipher: aes-128-gcm
    password: p1
  - name: 台湾1 (184629)
    type: ss
    server: 198.51.100.2
    port: 443
    cipher: aes-128-gcm
    password: p2
  - name: 新加坡3 (184629)
    type: ss
    server: 198.51.100.3
    port: 443
    cipher: aes-128-gcm
    password: p3
";
    let config: Mapping = serde_yaml_ng::from_str(yaml_str).unwrap();
    let mut enforced = enforce_mini_agreements(config).await;

    assert_eq!(
        enforced
            .get("profile")
            .and_then(Value::as_mapping)
            .and_then(|m| m.get("store-selected"))
            .and_then(Value::as_bool),
        Some(false)
    );

    apply_proxy_group_head_and_store_selected(&mut enforced, Some("新加坡3 (184629)"), "-香港 -台湾");
    let proxy_proxies: Vec<&str> = enforced
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .and_then(|g| g[0].as_mapping())
        .and_then(|m| m.get("proxies"))
        .and_then(Value::as_sequence)
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        proxy_proxies,
        vec!["新加坡3 (184629)", "香港1 (184629)", "台湾1 (184629)"]
    );
}

mod nameserver_policy_tests {
    use super::super::dns::sanitize_dns_upstreams;
    use serde_yaml_ng::{Mapping, Value};

    #[test]
    fn rewrites_unreachable_foreign_upstreams_and_preserves_keys_when_respect_rules_disabled() {
        let config: Mapping = serde_yaml_ng::from_str(
            r#"
dns:
  enable: true
  nameserver:
    - https://dns.alidns.com/dns-query
    - https://dns.google/dns-query
    - 8.8.8.8
  fallback:
    - https://1.1.1.1/dns-query
    - 8.8.4.4
  default-nameserver:
    - 223.5.5.5
    - 8.8.8.8
  nameserver-policy:
    "+.google.com": https://dns.cloudflare.com/dns-query
    "+.github.com": 8.8.8.8
    "+.unknown-foreign.example": https://dns.mullvad.net/dns-query
    "+.mixed.example":
      - https://dns.cloudflare.com/dns-query
      - https://dns.alidns.com/dns-query
      - https://dns.alidns.com/dns-query
    "+.corp.local": 10.0.0.1
    "geosite:cn": https://dns.alidns.com/dns-query
    "*.lan": system
"#,
        )
        .unwrap();

        let cleaned = sanitize_dns_upstreams(config);
        let dns = cleaned.get("dns").unwrap();

        // C10：respect-rules: false 下，nameserver 与 fallback 中的海外加密/明文上游亦就地降级为境内 DoH 并去重
        let nameserver: Vec<&str> = dns
            .get("nameserver")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(nameserver, vec!["https://dns.alidns.com/dns-query"]);

        let fallback: Vec<&str> = dns
            .get("fallback")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(fallback, vec!["https://dns.alidns.com/dns-query"]);

        // default-nameserver 保持明文不动
        let defaults: Vec<&str> = dns
            .get("default-nameserver")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(defaults, vec!["223.5.5.5", "8.8.8.8"]);

        let policy = dns.get("nameserver-policy").and_then(Value::as_mapping).unwrap();

        // BUG-364：绝不因海外上游不可达而删除策略键，统一就地改写为配置推导的可达境内 DoH
        assert_eq!(
            policy.get("+.google.com").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(
            policy.get("+.github.com").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(
            policy.get("+.unknown-foreign.example").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(policy.get("+.corp.local").and_then(Value::as_str), Some("10.0.0.1"));
        assert_eq!(
            policy.get("geosite:cn").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(policy.get("*.lan").and_then(Value::as_str), Some("system"));

        let mixed = policy.get("+.mixed.example").and_then(Value::as_sequence).unwrap();
        assert_eq!(mixed.len(), 1);
        assert_eq!(mixed[0].as_str(), Some("https://dns.alidns.com/dns-query"));
    }

    #[test]
    fn preserves_nameserver_policy_with_doh_pub_fallback_when_no_domestic_nameserver() {
        let config: Mapping = serde_yaml_ng::from_str(
            r#"
dns:
  enable: true
  nameserver:
    - https://1.1.1.1/dns-query
  nameserver-policy:
    "+.google.com": https://dns.cloudflare.com/dns-query
    "+.youtube.com": https://dns.google/dns-query
"#,
        )
        .unwrap();

        let cleaned = sanitize_dns_upstreams(config);
        let dns = cleaned.get("dns").unwrap();
        let nameserver: Vec<&str> = dns
            .get("nameserver")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(nameserver, vec!["https://doh.pub/dns-query"]);

        let policy = dns.get("nameserver-policy").and_then(Value::as_mapping).unwrap();
        assert_eq!(
            policy.get("+.google.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
        assert_eq!(
            policy.get("+.youtube.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
    }

    #[test]
    fn respect_rules_true_rewrites_plain_udp_keeps_doh_and_injects_dual_proxy_rules() {
        let config: Mapping = serde_yaml_ng::from_str(
            r#"
dns:
  enable: true
  respect-rules: true
  nameserver:
    - https://doh.pub/dns-query
  fallback:
    - https://doh.pub/dns-query
    - 223.5.5.5
  nameserver-policy:
    "+.google.com": https://dns.cloudflare.com/dns-query
    "+.github.com": 8.8.8.8
    "+.blocked-direct.com": https://dns.google/dns-query#DIRECT
    "+.proxied-h3.com": https://8.8.8.8/dns-query#PROXY&h3=true
    "+.cn-in-fallback.com": https://doh.pub/dns-query
    "+.local.lan": 192.168.1.1
rules:
  - RULE-SET,gfwlist,PROXY
  - IP-CIDR,8.8.8.8/32,PROXY,no-resolve
  - IP-CIDR,127.0.0.0/8,DIRECT,no-resolve
  - GEOIP,CN,DIRECT
  - MATCH,DIRECT
"#,
        )
        .unwrap();

        let cleaned = sanitize_dns_upstreams(config);
        let policy = cleaned
            .get("dns")
            .unwrap()
            .get("nameserver-policy")
            .and_then(Value::as_mapping)
            .unwrap();

        assert_eq!(
            policy.get("+.google.com").and_then(Value::as_str),
            Some("https://dns.cloudflare.com/dns-query")
        );
        assert_eq!(
            policy.get("+.github.com").and_then(Value::as_str),
            Some("https://8.8.8.8/dns-query")
        );
        // 废弃的 `#DIRECT` 海外条目不再删除键，改写为境内可达 DoH 且不写入 `#DIRECT`
        assert_eq!(
            policy.get("+.blocked-direct.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
        // C12：剥离 `#PROXY` 伪后缀的同时保留合法 Mihomo `#h3=true` 参数
        assert_eq!(
            policy.get("+.proxied-h3.com").and_then(Value::as_str),
            Some("https://8.8.8.8/dns-query#h3=true")
        );
        // C11：境内 DoH 即使出现在 fallback 中也不得被误判为境外
        assert_eq!(
            policy.get("+.cn-in-fallback.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
        assert_eq!(policy.get("+.local.lan").and_then(Value::as_str), Some("192.168.1.1"));

        let rules: Vec<&str> = cleaned
            .get("rules")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(
            rules,
            vec![
                "RULE-SET,gfwlist,PROXY",
                "IP-CIDR,8.8.8.8/32,PROXY,no-resolve",
                "DOMAIN,dns.cloudflare.com,PROXY",
                "IP-CIDR,1.1.1.1/32,PROXY,no-resolve",
                "IP-CIDR,1.0.0.1/32,PROXY,no-resolve",
                "IP-CIDR,8.8.4.4/32,PROXY,no-resolve",
                "IP-CIDR,127.0.0.0/8,DIRECT,no-resolve",
                "GEOIP,CN,DIRECT",
                "MATCH,DIRECT",
            ]
        );
    }
}

#[tokio::test]
async fn test_enforce_mini_agreements_strips_dangling_provider_proxy_and_merges_dialer_proxy() {
    use super::agreements::{enforce_mini_agreements, get_merged_proxies, get_merged_proxy_providers};
    use crate::config::PrfItem;
    use crate::config::profiles::IProfiles;
    use crate::utils::dirs;
    use serde_yaml_ng::{Mapping, Value};

    let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
    let profiles_dir = dirs::app_profiles_dir().unwrap();
    tokio::fs::create_dir_all(&profiles_dir).await.unwrap();

    let f1 = "test_bug425_p1.yaml";
    let f2 = "test_bug425_p2.yaml";
    let p1_path = profiles_dir.join(f1);
    let p2_path = profiles_dir.join(f2);
    let p1_yaml = r"
proxies:
  - name: Relay-Entry
    type: vless
    server: relay.example.com
    port: 443
    uuid: 11111111-1111-1111-1111-111111111111
  - name: Landing-Exit
    type: socks5
    server: 198.51.100.2
    port: 1080
    dialer-proxy: Relay-Entry
proxy-providers:
  sub1:
    type: http
    url: https://example.com/p1.yaml
    path: ./p1.yaml
    proxy: 已删除原策略组
";
    let p2_yaml = r"
proxies:
  - name: Relay-Entry
    type: ss
    server: ss2.example.com
    port: 8388
    cipher: aes-256-gcm
    password: pw
proxy-providers:
  sub1:
    type: http
    url: https://example.com/p2.yaml
    path: ./p2.yaml
    proxy: DIRECT
";
    tokio::fs::write(&p1_path, p1_yaml).await.unwrap();
    tokio::fs::write(&p2_path, p2_yaml).await.unwrap();

    let profiles = IProfiles {
        config_version: None,
        current: Some("uid_p1".into()),
        items: Some(vec![
            PrfItem {
                uid: Some("uid_p1".into()),
                itype: Some("remote".into()),
                file: Some(f1.into()),
                updated: Some(1_700_000_001),
                ..Default::default()
            },
            PrfItem {
                uid: Some("uid_p2".into()),
                itype: Some("remote".into()),
                file: Some(f2.into()),
                updated: Some(1_700_000_002),
                ..Default::default()
            },
        ]),
    };

    let merged_proxies = get_merged_proxies(&profiles);
    let landing = merged_proxies
        .iter()
        .find(|p| {
            p.get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| n.starts_with("Landing-Exit"))
        })
        .unwrap();
    let relay = merged_proxies
        .iter()
        .find(|p| {
            p.get("type").and_then(Value::as_str) == Some("vless")
                && p.get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| n.starts_with("Relay-Entry"))
        })
        .unwrap();
    assert_eq!(
        landing.get("dialer-proxy").and_then(Value::as_str),
        relay.get("name").and_then(Value::as_str),
        "同订阅内链式代理 dialer-proxy 必须同步随节点改名映射"
    );

    let merged_providers = get_merged_proxy_providers(&profiles);
    assert_eq!(
        merged_providers.len(),
        2,
        "跨订阅 proxy-providers 均应保留且冲突键自动后缀隔离"
    );
    assert!(merged_providers.get("sub1").is_some());

    let mut cfg = Mapping::new();
    cfg.insert(Value::from("proxies"), Value::from(merged_proxies));
    cfg.insert(Value::from("proxy-providers"), Value::from(merged_providers));
    let final_cfg = enforce_mini_agreements(cfg).await;
    let final_providers = final_cfg.get("proxy-providers").and_then(Value::as_mapping).unwrap();
    assert!(
        final_providers
            .get("sub1")
            .and_then(Value::as_mapping)
            .unwrap()
            .get("proxy")
            .is_none(),
        "指向已删除原策略组的悬空 proxy 字段应被移除"
    );
    let second_key = final_providers
        .keys()
        .filter_map(Value::as_str)
        .find(|k| *k != "sub1")
        .unwrap();
    assert_eq!(
        final_providers
            .get(second_key)
            .and_then(Value::as_mapping)
            .unwrap()
            .get("proxy")
            .and_then(Value::as_str),
        Some("DIRECT"),
        "合法内置目标 DIRECT 应原样保留"
    );

    let _ = tokio::fs::remove_file(&p1_path).await;
    let _ = tokio::fs::remove_file(&p2_path).await;
}
