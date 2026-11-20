#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::filter::{has_region_hint, is_dummy_node, is_valid_node_server, match_node_filter};
use super::fingerprint::{
    RawProxyEntry, deduplicate_cloned_proxies, proxy_endpoint_fingerprint, resolve_proxy_clone_topology,
};
use super::verify_store::{
    HTTPS_VERIFY_BAN_TTL_SECS, HTTPS_VERIFY_MAX_FAILS, NodeVerifyRecord, NodeVerifyStoreData,
    apply_verify_batch_to_records, clear_unverified_failures_in_records, load_verify_store_from_path,
};
use std::collections::HashMap;

#[test]
fn test_valid_node_server() {
    // 有效：IP 字面量与正常域名
    assert!(is_valid_node_server("1.2.3.4"));
    assert!(is_valid_node_server("::1"));
    assert!(is_valid_node_server("a03.jiffdl.net"));
    assert!(is_valid_node_server(" jp.example.com "));
    // 无效：占位假节点（BUG-301 现场 server='1'）与空值
    assert!(!is_valid_node_server("1"));
    assert!(!is_valid_node_server(""));
    assert!(!is_valid_node_server("   "));
}

#[test]
fn test_dummy_node_detection() {
    // 应被识别为广告/假节点的典型命名
    assert!(is_dummy_node("剩余流量：100GB"));
    assert!(is_dummy_node("过期时间：2024-12-31"));
    assert!(is_dummy_node("官网 https://example.com"));
    assert!(is_dummy_node("套餐到期"));
    assert!(is_dummy_node("续费链接"));
    assert!(is_dummy_node("公告栏"));
    assert!(is_dummy_node("购买地址"));
    assert!(is_dummy_node("群：123456"));
    assert!(is_dummy_node("expire soon"));
    assert!(is_dummy_node("traffic left"));
    assert!(is_dummy_node("website"));
    assert!(is_dummy_node("http://example.com"));
    assert!(is_dummy_node("https://example.com"));
    assert!(is_dummy_node("subscribe now"));
}

#[test]
fn test_dummy_node_false_positives() {
    // 真实节点不应被误杀
    assert!(!is_dummy_node("HK-流量优化-01"));
    assert!(!is_dummy_node("日本群组-A"));
    assert!(!is_dummy_node("CN2-购买入口"));
    assert!(!is_dummy_node("BGP-公告频道"));
    assert!(!is_dummy_node("新加坡-官网加速"));
    assert!(!is_dummy_node("美国-网址专线"));
    assert!(!is_dummy_node("韩国-剩余节点-01"));
    assert!(!is_dummy_node("台湾-套餐到期提醒")); // 虽然包含关键词，但不是以关键词开头
    assert!(!is_dummy_node("香港-续费专线"));
    assert!(!is_dummy_node("日本-过期时间测试"));
    assert!(!is_dummy_node("上海-群节点-01"));
}

#[test]
fn test_dummy_node_leak_side() {
    // 广告短语被包裹/加前缀时也应被识别，修复此前 starts_with 导致的漏判
    assert!(is_dummy_node("【剩余流量】"));
    assert!(is_dummy_node("【剩余流量：100GB】"));
    assert!(is_dummy_node("(购买入口)"));
    assert!(is_dummy_node("（购买入口）"));
    assert!(is_dummy_node("[官网]"));
    assert!(is_dummy_node("「公告频道」"));
    assert!(is_dummy_node("节点-购买入口"));
    assert!(is_dummy_node("节点：剩余流量"));
    // 多层包裹
    assert!(is_dummy_node("【(剩余流量)】"));
}

#[test]
fn test_match_node_filter() {
    // 空值、纯空白及孤立符号全部放行（避免输入未完成时列表闪烁清空）
    for empty in ["", "   ", "-", "!", "！", "|", "｜", "  -  "] {
        assert!(match_node_filter("香港1 (005024)", empty), "empty-like '{empty}'");
    }
    assert!(match_node_filter("日本 01 [IPLC]", "日本 -"));

    // 单关键词大小写不敏感匹配
    assert!(match_node_filter("HK-BGP-01", "hk"));
    assert!(match_node_filter("日本 01 [IPLC]", "日本"));
    assert!(!match_node_filter("新加坡 01", "日本"));

    // 多关键词 OR 匹配（半角 | / 全角 ｜ / 两侧空格 / 尾部悬空 |）
    let or_queries = ["日本|新加坡|台湾", "日本 | 新加坡 ｜ 台湾", "日本|"];
    for q in or_queries {
        assert!(match_node_filter("日本 01", q), "query '{q}'");
        if !q.ends_with('|') {
            assert!(match_node_filter("新加坡-专线", q), "query '{q}'");
            assert!(match_node_filter("台湾-原生", q), "query '{q}'");
        }
        assert!(!match_node_filter("香港1 (005024)", q), "query '{q}'");
    }

    // 纯排除语法（- / ! / ！）与多排除
    assert!(!match_node_filter("香港1 (005024)", "-香港"));
    assert!(match_node_filter("日本 01", "-香港"));
    assert!(!match_node_filter("HK-01", "!hk"));
    assert!(!match_node_filter("高倍率 3x", "！倍率"));
    assert!(!match_node_filter("香港-专线", "-香港 -英国"));
    assert!(!match_node_filter("英国-01", "-香港|-英国"));
    assert!(match_node_filter("新加坡-01", "-香港|-英国"));

    // 组合语法：OR + AND + 排除
    assert!(match_node_filter("日本 专线 01", "日本 | 新加坡 -倍率 -香港"));
    assert!(match_node_filter("新加坡 BGP 02", "日本 | 新加坡 -倍率 -香港"));
    assert!(!match_node_filter("日本 2x高倍率", "日本 | 新加坡 -倍率 -香港"));
    assert!(!match_node_filter("香港-日本中转", "日本 | 新加坡 -倍率 -香港"));

    assert!(match_node_filter("HK BGP IPLC-01", "hk bgp -倍率"));
    assert!(!match_node_filter("HK BGP 3倍率", "hk bgp -倍率"));

    // 含内部连字符与空格的真实节点名
    assert!(match_node_filter("英国-3倍率", "英国-3倍率"));
    assert!(match_node_filter("香港1 (005024)", "香港1 (005024)"));
}

#[test]
fn test_proxy_endpoint_fingerprint_ignores_name_and_key_order() {
    let a: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
        r"
name: 距离下次重置剩余：25 天
type: anytls
server: 1.2.3.4
port: 443
password: secret
sni: example.com
",
    )
    .unwrap();
    let b: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
        r"
sni: example.com
password: secret
port: 443
server: 1.2.3.4
type: anytls
name: 香港1
",
    )
    .unwrap();
    let c: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
        r"
name: 香港2
type: anytls
server: 1.2.3.5
port: 443
password: secret
",
    )
    .unwrap();
    let missing_port: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
        r"
name: 无端口
type: anytls
server: 1.2.3.4
",
    )
    .unwrap();

    let fp_a = proxy_endpoint_fingerprint(&a).unwrap();
    let fp_b = proxy_endpoint_fingerprint(&b).unwrap();
    let fp_c = proxy_endpoint_fingerprint(&c).unwrap();
    assert_eq!(fp_a, fp_b);
    assert_ne!(fp_a, fp_c);
    assert!(proxy_endpoint_fingerprint(&missing_port).is_none());
}

#[test]
fn test_deduplicate_cloned_proxies_prefers_non_colon_name() {
    let entries = vec![
        ("距离下次重置剩余：25 天".to_owned(), Some("fp1".to_owned())),
        ("日本1".to_owned(), Some("fp2".to_owned())),
        ("香港1".to_owned(), Some("fp1".to_owned())),
        ("无指纹直连".to_owned(), None),
        ("新加坡：备用".to_owned(), Some("fp3".to_owned())),
        ("新加坡：主".to_owned(), Some("fp3".to_owned())),
    ];
    let (names, removed) = deduplicate_cloned_proxies(entries);
    assert_eq!(removed, 2);
    assert_eq!(names, vec!["香港1", "日本1", "无指纹直连", "新加坡：备用"]);
}

#[test]
fn test_verify_state_machine_sticky_pass_threshold_and_offline_guard() {
    let mut records: HashMap<String, NodeVerifyRecord> = HashMap::new();
    let good = "fp-good".to_owned();
    let dead = "fp-dead".to_owned();

    // 0 个参照节点通过时，禁止累加任何节点的 fail_count（防止断网/梯子挂了误杀真节点）
    let o0 = apply_verify_batch_to_records(&mut records, &[], &[good.clone(), dead.clone()], 100);
    assert!(!o0.dirty);
    assert!(records.is_empty());

    // good 验真通过 1 次 -> 单向粘滞为真；dead 同批失败 -> fail_count = 1
    let o1 = apply_verify_batch_to_records(
        &mut records,
        std::slice::from_ref(&good),
        std::slice::from_ref(&dead),
        101,
    );
    assert_eq!(o1.newly_verified, 1);
    assert_eq!(o1.newly_confirmed_fake, 0);
    assert!(records[&good].verified_true);
    assert_eq!(records[&dead].fail_count, 1);
    assert!(!records[&dead].is_confirmed_fake_at(101));

    // good 随后即使作为参照失败也不会降级，fail_count 保持 0
    for round in 2..=HTTPS_VERIFY_MAX_FAILS {
        let ts = 100 + i64::from(round);
        let o = apply_verify_batch_to_records(&mut records, &["fp-ref".to_owned()], &[good.clone(), dead.clone()], ts);
        assert_eq!(o.newly_confirmed_fake, 0);
        assert!(records[&good].verified_true);
        assert_eq!(records[&good].fail_count, 0);
        assert_eq!(records[&dead].fail_count, round);
        assert!(!records[&dead].is_confirmed_fake_at(ts));
    }

    // 第 6 次失败（超过 HTTPS_VERIFY_MAX_FAILS = 5）-> 进入 24h 软隔离假节点
    let o6 = apply_verify_batch_to_records(
        &mut records,
        std::slice::from_ref(&good),
        std::slice::from_ref(&dead),
        200,
    );
    assert_eq!(o6.newly_confirmed_fake, 1);
    assert!(records[&dead].is_confirmed_fake_at(200));

    // 24h 软隔离期内再次失败不重复计数
    let o7 = apply_verify_batch_to_records(
        &mut records,
        std::slice::from_ref(&good),
        std::slice::from_ref(&dead),
        201,
    );
    assert_eq!(o7.newly_confirmed_fake, 0);
}

#[test]
fn test_verify_state_machine_quarantine_ttl_and_manual_clear() {
    let mut records: HashMap<String, NodeVerifyRecord> = HashMap::new();
    let ref_fp = "fp-ref".to_owned();
    let node_a = "fp-temp-down".to_owned();
    let node_b = "fp-recovered".to_owned();

    records.insert(
        ref_fp.clone(),
        NodeVerifyRecord {
            verified_true: true,
            fail_count: 0,
            last_verified_at: 200,
        },
    );
    records.insert(
        node_a.clone(),
        NodeVerifyRecord {
            verified_true: false,
            fail_count: HTTPS_VERIFY_MAX_FAILS + 1,
            last_verified_at: 200,
        },
    );
    records.insert(
        node_b.clone(),
        NodeVerifyRecord {
            verified_true: false,
            fail_count: HTTPS_VERIFY_MAX_FAILS + 1,
            last_verified_at: 200,
        },
    );

    // 1. 满 24h 前一秒仍处于软隔离；满 24h 自动解除
    assert!(records[&node_a].is_confirmed_fake_at(200 + HTTPS_VERIFY_BAN_TTL_SECS - 1));
    assert!(!records[&node_a].is_confirmed_fake_at(200 + HTTPS_VERIFY_BAN_TTL_SECS));

    // 2. 期满后重新参加一轮验真：node_b 通过升级粘滞真值，node_a 再次失败则清零重计为 fail_count = 1
    let ttl_ts = 200 + HTTPS_VERIFY_BAN_TTL_SECS;
    let o_recheck = apply_verify_batch_to_records(
        &mut records,
        std::slice::from_ref(&node_b),
        std::slice::from_ref(&node_a),
        ttl_ts,
    );
    assert_eq!(o_recheck.newly_verified, 1);
    assert_eq!(o_recheck.newly_confirmed_fake, 0);
    assert!(records[&node_b].verified_true);
    assert_eq!(records[&node_b].fail_count, 0);
    assert_eq!(records[&node_a].fail_count, 1);
    assert!(!records[&node_a].is_confirmed_fake_at(ttl_ts));

    // 3. 手动更新订阅清零未验真失败记录，但完整保留 verified_true 白名单
    let cleared = clear_unverified_failures_in_records(&mut records);
    assert_eq!(cleared, 1);
    assert!(!records.contains_key(&node_a));
    assert!(records[&ref_fp].verified_true);
    assert!(records[&node_b].verified_true);
}

#[test]
fn test_verify_store_json_roundtrip() {
    let tmp = std::env::temp_dir().join(format!("clash_mini_verify_store_test_{}.json", std::process::id()));
    let mut store = NodeVerifyStoreData::default();
    store.records.insert(
        "abc".to_owned(),
        NodeVerifyRecord {
            verified_true: true,
            fail_count: 0,
            last_verified_at: 123_456,
        },
    );
    let serialized = serde_json::to_string_pretty(&store).unwrap();
    std::fs::write(&tmp, serialized).unwrap();

    let loaded = load_verify_store_from_path(&tmp);
    assert_eq!(loaded.records.get("abc"), store.records.get("abc"));
    let _ = std::fs::remove_file(&tmp);

    let missing = load_verify_store_from_path(&tmp);
    assert!(missing.records.is_empty());
}

#[test]
fn test_has_region_hint_and_structural_banner_detection() {
    // 1. 真实订阅实战漏网文案：含冒号倒计时/配额/日期或内嵌横幅域名（无地区暗示）
    assert!(is_dummy_node("距离下次重置剩余：24 天 (045017)"));
    assert!(is_dummy_node("永久官网:666.boostqz.com (045017)"));
    assert!(is_dummy_node("有效期至：2027-02-09"));
    assert!(is_dummy_node("更新日期：2026年11月"));
    assert!(is_dummy_node("Remaining: 30 days"));
    assert!(is_dummy_node("Quota: 167.26 GB"));
    assert!(is_dummy_node("剩余流量：100\u{3000}GB"));
    assert!(is_dummy_node("Contact us: boostqz.com"));
    // .hk 后缀相邻点号不算地区暗示，应识别为官网域名横幅
    assert!(!has_region_hint("永久官网:abc.hk"));
    assert!(is_dummy_node("永久官网:abc.hk"));

    // 2. 地区暗示校验：中英文、国旗、边界 IATA/国家码与 CN2/US 消歧
    assert!(has_region_hint("🇭🇰 HK-01"));
    assert!(has_region_hint("香港1 (045017)"));
    assert!(has_region_hint("us-01 [VLESS]"));
    assert!(has_region_hint("CN2-GIA-01"));
    assert!(!has_region_hint("About us: boostqz.com"));

    // 3. 防误杀真实节点名（不含横幅结构或带真实地区暗示）
    assert!(!is_dummy_node("vps1.myserver.com"));
    assert!(!is_dummy_node("新加坡：备用"));
    assert!(!is_dummy_node("专线：10Gbps"));
    assert!(!is_dummy_node("【剩余流量】80GB"));
    assert!(!is_dummy_node("最新官网发布页"));
    assert!(!is_dummy_node("香港1 (045017)"));
}

#[test]
fn test_resolve_proxy_clone_topology_taint_and_parasite_cases() {
    // 场景 A：机场把多条横幅绑在同一真实可用香港端点上，且混入全新未知广告文案（不含任何地区真节点）
    // 预期：整组指纹连坐 0 保留全灭，绝不留任何一个横幅进入 PROXY
    let entries_banner_only = vec![
        RawProxyEntry {
            name: "剩余流量：167.26 GB (045017)".to_owned(),
            fingerprint: Some("fp-banner".to_owned()),
            is_dummy: is_dummy_node("剩余流量：167.26 GB (045017)"),
        },
        RawProxyEntry {
            name: "距离下次重置剩余：24 天 (045017)".to_owned(),
            fingerprint: Some("fp-banner".to_owned()),
            is_dummy: is_dummy_node("距离下次重置剩余：24 天 (045017)"),
        },
        RawProxyEntry {
            name: "全网独创全新福利宣传语".to_owned(), // 完全未知文案，is_dummy = false
            fingerprint: Some("fp-banner".to_owned()),
            is_dummy: is_dummy_node("全网独创全新福利宣传语"),
        },
        RawProxyEntry {
            name: "香港1 (045017)".to_owned(),
            fingerprint: Some("fp-hk1".to_owned()),
            is_dummy: is_dummy_node("香港1 (045017)"),
        },
    ];
    let out_a = resolve_proxy_clone_topology(entries_banner_only);
    assert_eq!(out_a.names, vec!["香港1 (045017)"]);
    assert_eq!(out_a.tainted_banner_removed, 1);
    assert!(out_a.tainted_fps.contains("fp-banner"));

    // 场景 B：机场把未知文案横幅直接挂在真实「香港1」节点同一指纹上，且排在「香港1」前面
    // 预期：有真实地区节点同行不整组全灭，但优选等级强制折叠归真为「香港1 (045017)」，令 -香港 可靠排除
    let entries_parasite = vec![
        RawProxyEntry {
            name: "距离下次重置剩余：24 天 (045017)".to_owned(),
            fingerprint: Some("fp-hk1".to_owned()),
            is_dummy: true,
        },
        RawProxyEntry {
            name: "全网独创客服宣传语".to_owned(),
            fingerprint: Some("fp-hk1".to_owned()),
            is_dummy: false,
        },
        RawProxyEntry {
            name: "香港1 (045017)".to_owned(),
            fingerprint: Some("fp-hk1".to_owned()),
            is_dummy: false,
        },
    ];
    let out_b = resolve_proxy_clone_topology(entries_parasite);
    assert_eq!(out_b.names, vec!["香港1 (045017)"]);
    assert_eq!(out_b.clone_removed, 1);
    assert!(out_b.tainted_fps.is_empty());

    // 场景 C：真实双带冒号地区节点共享同一指纹（如「新加坡：备用」「新加坡：主」）不被误伤
    let entries_dual_colon_region = vec![
        RawProxyEntry {
            name: "新加坡：备用".to_owned(),
            fingerprint: Some("fp-sg".to_owned()),
            is_dummy: false,
        },
        RawProxyEntry {
            name: "新加坡：主".to_owned(),
            fingerprint: Some("fp-sg".to_owned()),
            is_dummy: false,
        },
    ];
    let out_c = resolve_proxy_clone_topology(entries_dual_colon_region);
    assert_eq!(out_c.names, vec!["新加坡：备用"]);
    assert!(out_c.tainted_fps.is_empty());
}

#[test]
fn test_https_verify_migrate_min_fails_threshold() {
    use super::verify_store::{HTTPS_VERIFY_MIGRATE_MIN_FAILS, record_has_verify_failure};
    assert_eq!(HTTPS_VERIFY_MIGRATE_MIN_FAILS, 2);
    let now = 1_700_000_000;
    // 仅 1 次失败不触发活跃节点迁移（防偶发抖动）
    assert!(!record_has_verify_failure(false, 1, now - 10, now));
    // 连续 >= 2 次失败且在 24h TTL 内允许迁移
    assert!(record_has_verify_failure(false, 2, now - 10, now));
    // 曾通过真验真或已超 TTL 均不触发
    assert!(!record_has_verify_failure(true, 3, now - 10, now));
    assert!(!record_has_verify_failure(
        false,
        3,
        now - HTTPS_VERIFY_BAN_TTL_SECS,
        now
    ));
}

#[test]
fn test_filter_completeness_and_promote_preferred_proxy_first() {
    use super::{has_effective_filter_term, is_incomplete_filter_input, promote_preferred_proxy_first};

    // 1. 半输入与有效过滤词判定（与前端 1:1 对齐）
    assert!(!has_effective_filter_term(""));
    assert!(!has_effective_filter_term("   "));
    assert!(!has_effective_filter_term("- | !"));
    assert!(has_effective_filter_term("-香港 -台湾"));
    assert!(is_incomplete_filter_input(""));
    assert!(is_incomplete_filter_input("-"));
    assert!(is_incomplete_filter_input("日本 -"));
    assert!(is_incomplete_filter_input("-香港|"));
    assert!(!is_incomplete_filter_input("-香港 -台湾"));
    assert!(!is_incomplete_filter_input("hk-"));

    // 2. 合规首选节点提前至 PROXY.proxies[0]，其余保持相对顺序
    let mut names = vec![
        "香港1".to_string(),
        "台湾1".to_string(),
        "日本1".to_string(),
        "新加坡3".to_string(),
    ];
    let picked = promote_preferred_proxy_first(&mut names, Some("新加坡3"), "-香港 -台湾", |n| n != "DIRECT", |_| true);
    assert_eq!(picked.as_deref(), Some("新加坡3"));
    assert_eq!(names, vec!["新加坡3", "香港1", "台湾1", "日本1"]);

    // 3. 若持久首选节点被过滤排除（如香港1），自动拒绝并在合规子集中选首个已验真节点提前
    let mut names_stale = vec![
        "香港1".to_string(),
        "台湾1".to_string(),
        "日本1".to_string(),
        "新加坡3".to_string(),
    ];
    let picked_fallback = promote_preferred_proxy_first(
        &mut names_stale,
        Some("香港1"),
        "-香港 -台湾",
        |n| n != "DIRECT",
        |n| n == "新加坡3",
    );
    assert_eq!(picked_fallback.as_deref(), Some("新加坡3"));
    assert_eq!(names_stale, vec!["新加坡3", "香港1", "台湾1", "日本1"]);

    // 4. 过滤子集无任何匹配时保持原顺序返回 None（绝不把禁止节点擅自视为匹配）
    let mut only_blocked = vec!["香港1".to_string(), "台湾1".to_string()];
    let picked_none = promote_preferred_proxy_first(
        &mut only_blocked,
        Some("香港1"),
        "-香港 -台湾",
        |n| n != "DIRECT",
        |_| true,
    );
    assert_eq!(picked_none, None);
    assert_eq!(only_blocked, vec!["香港1", "台湾1"]);
}

#[test]
fn test_pick_subset_fallback_node_respects_filter_boundary() {
    use super::pick_subset_fallback_node;

    let all = vec![
        "剩余流量：100 GB".to_string(),
        "香港-01".to_string(),
        "日本-01".to_string(),
        "日本-02".to_string(),
    ];
    let is_valid = |n: &str| !n.contains("剩余流量");

    assert_eq!(
        pick_subset_fallback_node(&all, "日本", is_valid, |n| n == "日本-02"),
        Some("日本-02")
    );
    assert_eq!(
        pick_subset_fallback_node(&all, "日本", is_valid, |_| false),
        Some("日本-01")
    );
    assert_eq!(
        pick_subset_fallback_node(&all, "新加坡", is_valid, |n| n == "香港-01"),
        None
    );
    assert_eq!(
        pick_subset_fallback_node(&all, "   ", is_valid, |n| n == "日本-01"),
        Some("日本-01")
    );
    assert_eq!(
        pick_subset_fallback_node(&all, "", is_valid, |_| false),
        Some("香港-01")
    );
}
