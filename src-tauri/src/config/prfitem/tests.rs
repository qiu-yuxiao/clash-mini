#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{PrfItem, PrfOption, normalize_custom_user_agent};
use crate::utils::{dirs, network::validate_url_no_ssrf};
use smartstring::alias::String;
use tauri::Url;

#[tokio::test]
async fn test_prf_item_save_file_read_before_write() {
    // Force portable flag to true so we don't need a Tauri app handle
    let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);

    // Ensure profiles directory exists
    let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
    tokio::fs::create_dir_all(&profiles_dir)
        .await
        .expect("failed to create profiles dir");

    let file_name = "test_prf_item_save_r_b_w.yaml";
    let file_path = profiles_dir.join(file_name);

    // Clean up any existing file
    let _ = tokio::fs::remove_file(&file_path).await;

    let item = PrfItem {
        file: Some(file_name.into()),
        ..Default::default()
    };

    let initial_data: String = "key: value\r\nlist:\r\n  - item1\r\n".into();

    // 1. Initial save (should write)
    item.save_file(initial_data.clone()).expect("initial save failed");
    assert!(file_path.exists());

    let metadata_first = tokio::fs::metadata(&file_path).await.expect("metadata failed");
    let mtime_first = metadata_first.modified().expect("modified time failed");

    // Sleep to ensure modification time can be distinguished if a write happens
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2. Save identical content but with different line endings (should skip writing)
    let identical_data = "key: value\nlist:\n  - item1\n".into();
    item.save_file(identical_data).expect("second save failed");

    let metadata_second = tokio::fs::metadata(&file_path).await.expect("metadata failed");
    let mtime_second = metadata_second.modified().expect("modified time failed");
    assert_eq!(
        mtime_first, mtime_second,
        "mtime changed, meaning file was written unnecessarily"
    );

    // Sleep
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 3. Save actually different content (should write)
    let different_data = "key: different_value\nlist:\n  - item1\n".into();
    item.save_file(different_data).expect("third save failed");

    let metadata_third = tokio::fs::metadata(&file_path).await.expect("metadata failed");
    let mtime_third = metadata_third.modified().expect("modified time failed");
    assert_ne!(
        mtime_second, mtime_third,
        "mtime did not change, meaning file was not written when it should have been"
    );

    // Clean up
    let _ = tokio::fs::remove_file(&file_path).await;
}

#[tokio::test]
async fn test_prf_item_save_file_empty_strings() {
    let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
    let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
    tokio::fs::create_dir_all(&profiles_dir)
        .await
        .expect("failed to create profiles dir");

    let file_name = "test_prf_item_save_empty.yaml";
    let file_path = profiles_dir.join(file_name);
    let _ = tokio::fs::remove_file(&file_path).await;

    let item = PrfItem {
        file: Some(file_name.into()),
        ..Default::default()
    };

    // Save empty string
    item.save_file("".into()).expect("save empty string failed");
    assert!(file_path.exists());
    let content = tokio::fs::read_to_string(&file_path).await.expect("read failed");
    assert_eq!(content, "");

    let metadata_first = tokio::fs::metadata(&file_path).await.expect("metadata failed");
    let mtime_first = metadata_first.modified().expect("modified time failed");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Save empty string again (should be skipped)
    item.save_file("".into()).expect("save empty string second time failed");
    let metadata_second = tokio::fs::metadata(&file_path).await.expect("metadata failed");
    let mtime_second = metadata_second.modified().expect("modified time failed");
    assert_eq!(
        mtime_first, mtime_second,
        "mtime changed for redundant empty string write"
    );

    // Clean up
    let _ = tokio::fs::remove_file(&file_path).await;
}

#[tokio::test]
async fn test_prf_item_save_file_missing_file() {
    let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
    let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
    tokio::fs::create_dir_all(&profiles_dir)
        .await
        .expect("failed to create profiles dir");

    let file_name = "test_prf_item_save_missing.yaml";
    let file_path = profiles_dir.join(file_name);
    let _ = tokio::fs::remove_file(&file_path).await;

    let item = PrfItem {
        file: Some(file_name.into()),
        ..Default::default()
    };

    // File is missing, saving should succeed and create the file
    item.save_file("some content".into())
        .expect("saving to missing file failed");
    assert!(file_path.exists());
    let content = tokio::fs::read_to_string(&file_path).await.expect("read failed");
    assert_eq!(content, "some content");

    // Clean up
    let _ = tokio::fs::remove_file(&file_path).await;
}

#[tokio::test]
async fn test_prf_item_save_file_line_endings() {
    let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
    let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
    tokio::fs::create_dir_all(&profiles_dir)
        .await
        .expect("failed to create profiles dir");

    let file_name = "test_prf_item_save_endings.yaml";
    let file_path = profiles_dir.join(file_name);
    let _ = tokio::fs::remove_file(&file_path).await;

    let item = PrfItem {
        file: Some(file_name.into()),
        ..Default::default()
    };

    // 1. Save LF content
    item.save_file("line1\nline2\n".into()).expect("save LF failed");
    let metadata_first = tokio::fs::metadata(&file_path).await.expect("metadata failed");
    let mtime_first = metadata_first.modified().expect("mtime failed");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2. Save CRLF equivalent content -> should skip write
    item.save_file("line1\r\nline2\r\n".into()).expect("save CRLF failed");
    let metadata_second = tokio::fs::metadata(&file_path).await.expect("metadata failed");
    let mtime_second = metadata_second.modified().expect("mtime failed");
    assert_eq!(mtime_first, mtime_second, "mtime changed for normalized CRLF content");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 3. Save mixed content -> should skip write if normalized equivalent
    item.save_file("line1\r\nline2\n".into()).expect("save mixed failed");
    let metadata_third = tokio::fs::metadata(&file_path).await.expect("mtime failed");
    let mtime_third = metadata_third.modified().expect("mtime failed");
    assert_eq!(mtime_first, mtime_third, "mtime changed for normalized mixed content");

    // Clean up
    let _ = tokio::fs::remove_file(&file_path).await;
}

#[test]
fn test_prf_item_smartstring_type_safety() {
    // Verify we can manipulate PrfItem using smartstrings as expected
    let uid_smart: smartstring::alias::String = "test_uid".into();
    let name_smart: smartstring::alias::String = "test_name".into();
    let file_smart: smartstring::alias::String = "test_file.yaml".into();

    let item = PrfItem {
        uid: Some(uid_smart.clone()),
        name: Some(name_smart.clone()),
        file: Some(file_smart.clone()),
        ..Default::default()
    };

    assert_eq!(item.uid.unwrap(), uid_smart);
    assert_eq!(item.name.unwrap(), name_smart);
    assert_eq!(item.file.unwrap(), file_smart);
}

#[test]
fn test_validate_url_no_ssrf_blocks_internal_and_mapped_ips() {
    let blocked_urls = [
        "http://localhost:8080/sub",
        "http://localhost./sub",
        "http://intranet/sub",
        "http://router.local/sub",
        "http://nas.lan/sub",
        "http://127.0.0.1:7890/sub",
        "http://192.168.1.1/sub",
        "http://169.254.169.254/latest/meta-data/",
        "http://[::1]:8080/sub",
        "http://[fd00::1]/sub",
        "http://[fe80::1]/sub",
        "http://[::ffff:127.0.0.1]:8080/sub",
        "http://[::ffff:10.0.0.1]/sub",
    ];
    for raw in blocked_urls {
        let parsed = Url::parse(raw).unwrap();
        assert!(
            validate_url_no_ssrf(&parsed).is_err(),
            "expected SSRF guard to block {raw}"
        );
    }

    // 公网 IP 与本地无法直连解析但语法合法的公网订阅域名应放行（由代理链路拉取）
    let allowed_ip = Url::parse("https://1.1.1.1/sub").unwrap();
    assert!(validate_url_no_ssrf(&allowed_ip).is_ok());
    let unresolvable_public = Url::parse("https://sub.nonexistent-airport-domain-test.example/api/v1").unwrap();
    assert!(validate_url_no_ssrf(&unresolvable_public).is_ok());
}

#[test]
fn test_prf_option_custom_user_agent_normalization_and_merge() {
    // BUG-362: 单订阅自定义逃生阀 option.user_agent 去空白保留、空串自动清除回退默认兼容标识
    assert_eq!(normalize_custom_user_agent(None), None);
    assert_eq!(normalize_custom_user_agent(Some("")), None);
    assert_eq!(normalize_custom_user_agent(Some("   ")), None);
    assert_eq!(
        normalize_custom_user_agent(Some("  SpecialPanelUA/3.0 ")),
        Some("SpecialPanelUA/3.0".into())
    );

    let existing = PrfOption {
        user_agent: Some("CustomUA/1.0".into()),
        update_interval: Some(60),
        ..Default::default()
    };
    let update_keep = PrfOption {
        user_agent: None,
        update_interval: Some(120),
        ..Default::default()
    };
    let merged_keep = PrfOption::merge(Some(&existing), Some(&update_keep)).unwrap();
    assert_eq!(merged_keep.user_agent.as_deref(), Some("CustomUA/1.0"));
    assert_eq!(merged_keep.update_interval, Some(120));

    let update_clear = PrfOption {
        user_agent: Some("   ".into()),
        ..Default::default()
    };
    let merged_clear = PrfOption::merge(Some(&existing), Some(&update_clear)).unwrap();
    assert_eq!(merged_clear.user_agent, None);
}

#[tokio::test]
async fn test_from_url_direct_supports_yaml_singbox_sip008_and_new_uris() {
    let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
    let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
    tokio::fs::create_dir_all(&profiles_dir)
        .await
        .expect("failed to create profiles dir");
    let direct_path = profiles_dir.join("L_Direct_Imports.yaml");
    let prev_content = tokio::fs::read(&direct_path).await.ok();

    let clean_item = PrfItem::from_url("clear", None, None, None).await.unwrap();
    assert_eq!(clean_item.desc.as_deref(), Some("0"));

    // 1. 直接粘贴 hy2:// 短协议别名
    let item1 = PrfItem::from_url(
        "hy2://secret@hy2.example.com:443?mport=20000-30000#HY2-Direct",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(item1.desc.as_deref(), Some("1"));

    // 2. 直接粘贴 Clash YAML 片段并合并到 L_Direct_Imports
    let yaml_input = "proxies:\n  - name: Yaml-Node\n    type: ss\n    server: 1.2.3.4\n    port: 8388\n    cipher: aes-256-gcm\n    password: pw\n";
    let item2 = PrfItem::from_url(yaml_input, None, None, None).await.unwrap();
    item2.save_file(item2.file_data.clone().unwrap()).unwrap();
    assert_eq!(item2.desc.as_deref(), Some("1"));

    // 3. 直接粘贴 sing-box JSON 并与已落盘 Yaml-Node 合并
    let singbox = r#"{"outbounds":[{"type":"vless","tag":"SB-Node","server":"sb.example.com","server_port":443,"uuid":"11111111-1111-1111-1111-111111111111"}]}"#;
    let item3 = PrfItem::from_url(singbox, None, None, None).await.unwrap();
    assert_eq!(item3.desc.as_deref(), Some("2"));

    // 4. 校验深链解包辅助函数
    let (inner, name) = crate::utils::resolve::scheme::parse_scheme_subscription_url(
        "clash-mini://install-config?url=https%3A%2F%2Fexample.com%2Fsub%3Fflag%3Dmeta&name=TestSub",
    )
    .unwrap();
    assert_eq!(inner, "https://example.com/sub?flag=meta");
    assert_eq!(name.as_deref(), Some("TestSub"));

    // 5. 直接粘贴完整 Clash YAML 时，L_Direct_Imports 不应污染顶层 dns/rules 键，
    //    且多行文本首行含 https:// 不得误走单行远程 URL 路径
    let _ = PrfItem::from_url("clear", None, None, None).await.unwrap();
    let full_yaml = "port: 7890\ndns:\n  enable: true\nproxies:\n  - name: Clean-Only\n    type: ss\n    server: 1.2.3.4\n    port: 8388\n    cipher: aes-256-gcm\n    password: pw\nrules:\n  - MATCH,DIRECT\n";
    let clean_direct = PrfItem::from_url(full_yaml, None, None, None).await.unwrap();
    let serialized_direct = clean_direct.file_data.unwrap();
    assert!(!serialized_direct.contains("\nrules:"));
    assert!(!serialized_direct.contains("\ndns:"));
    assert!(
        PrfItem::from_url("https://example.com/sub\nextra-line", None, None, None)
            .await
            .is_err()
    );

    // 恢复现场，避免污染其它单测
    if let Some(bytes) = prev_content {
        let _ = tokio::fs::write(&direct_path, bytes).await;
    } else {
        let _ = tokio::fs::remove_file(&direct_path).await;
    }
}
