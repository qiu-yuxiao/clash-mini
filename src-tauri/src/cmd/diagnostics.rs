//! 诊断包导出
//!
//! ## 为什么需要
//!
//! 用户报「断流 / 卡顿 / 节点全红」时，仅靠口头描述无法定位。本功能一键打包现场：
//! 最近的轮转日志 + 脱敏配置 + 版本与运行摘要，用户把 zip 发出来即可开始排查，
//! 省去多轮索要材料。
//!
//! ## 脱敏是硬要求
//!
//! 配置里含**节点密码 / UUID / 密钥 / 订阅链接 / 内核 secret**，这些绝不能随诊断包外泄。
//! 所有 YAML 在入包前统一走 [`redact_yaml_text`]；解析失败的配置**只写错误说明、不写正文**，
//! 宁可缺材料也不泄露。

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context as _, Result};
use chrono::Local;
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::Value;

use crate::cmd::{CmdResult, StringifyErr as _};
use crate::config::Config;
use crate::utils::dirs;

/// 敏感键名（小写比较）。命中后其标量值整体替换为占位符。
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "uuid",
    "psk",
    "auth",
    "auth-str",
    "obfs-password",
    "secret",
    "token",
    "url",
    "private-key",
    "public-key",
];

const REDACTED: &str = "***REDACTED***";

const BUNDLE_README: &str = "\
Clash Mini 诊断包
=================

本包由「设置 → 帮助 → 导出诊断包」生成，用于问题排查。

内容：
  logs/                     最近的轮转日志（按大小轮转，默认保留约 24 小时）
  runtime-config.redacted.yaml  内核运行时配置（已脱敏）
  verge.yaml.redacted.yaml      应用配置（已脱敏）
  profiles.yaml.redacted.yaml   订阅配置（已脱敏）
  system-info.txt           版本与运行摘要

脱敏说明：
  节点密码 / UUID / 密钥 / 订阅链接 / 内核 secret 均已替换为 ***REDACTED***，
  可直接发送。若仍担心，可自行解压后检查再发出。

排查建议：
  断流请重点看 logs/ 中时间点附近的以下关键字：
    「连续 2 次检测失败」  自愈触发
    「自愈选点」          选点过程与结果
    「所有测速节点均不可达」  节点侧或测速链路异常
    「测速链路不可用」    测速失真（非节点死亡）
    「失败服务器避让」    因服务器间歇不可达而跳过该服务器
";

/// 递归脱敏 YAML 值：命中敏感键的标量直接替换，容器继续向下走。
fn redact_value(value: &mut Value) {
    match value {
        Value::Mapping(map) => {
            for (key, val) in map.iter_mut() {
                let key_lower = key.as_str().unwrap_or_default().to_ascii_lowercase();
                let sensitive = SENSITIVE_KEYS.contains(&key_lower.as_str());
                if sensitive && !val.is_mapping() && !val.is_sequence() {
                    *val = Value::String(REDACTED.into());
                } else {
                    redact_value(val);
                }
            }
        }
        Value::Sequence(seq) => {
            for item in seq.iter_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}

/// YAML 文本脱敏。解析失败时**只返回错误说明，绝不返回原始正文**。
fn redact_yaml_text(text: &str) -> String {
    match serde_yaml_ng::from_str::<Value>(text) {
        Ok(mut value) => {
            redact_value(&mut value);
            serde_yaml_ng::to_string(&value)
                .unwrap_or_else(|e| format!("# 脱敏成功但序列化失败: {e}\n"))
        }
        Err(e) => format!("# 配置解析失败，为避免泄露原始内容，本文件不含配置正文: {e}\n"),
    }
}

/// 每次创建文件都新建选项（FileOptions 不保证 Copy，避免因版本差异编译失败）。
fn file_opts() -> zip::write::SimpleFileOptions {
    zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
}

/// 收集版本与运行摘要（不含任何敏感内容）。
async fn collect_system_info() -> String {
    let verge = Config::verge().await.latest_arc();
    // IClashTemp 是运行时配置结构，没有 ipv6 字段；用户持久化设置要经 get_client_info 取
    let clash_info = Config::clash().await.data_arc().get_client_info();

    // 节点数：直接数运行时配置里的 proxies（避免额外打内核 API）
    let mut node_count = 0usize;
    if let Ok(home) = dirs::app_home_dir()
        && let Ok(text) = tokio::fs::read_to_string(home.join(crate::constants::files::RUNTIME_CONFIG)).await
        && let Ok(value) = serde_yaml_ng::from_str::<Value>(&text)
    {
        node_count = value
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .map(|s| s.len())
            .unwrap_or(0);
    }

    format!(
        "生成时间        : {}\n\
         应用版本        : {}\n\
         操作系统        : {} / {}\n\
         verge 配置版本  : {:?}（程序支持 {}）\n\
         TUN 模式        : {:?}\n\
         系统代理        : {:?}\n\
         Allow IPv6      : {:?}\n\
         规则兜底        : {:?}\n\
         日志级别        : {:?}\n\
         日志轮转        : {:?} KB × {:?} 个\n\
         节点总数        : {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        crate::core::handle::Handle::app_handle()
            .package_info()
            .version,
        std::env::consts::OS,
        std::env::consts::ARCH,
        verge.config_version,
        crate::config::CURRENT_VERGE_CONFIG_VERSION,
        verge.enable_tun_mode,
        verge.enable_system_proxy,
        clash_info.ipv6,
        verge.rule_fallback,
        verge.app_log_level,
        verge.app_log_max_size,
        verge.app_log_max_count,
        node_count,
    )
}

/// 导出诊断包，返回生成的 zip 绝对路径（由前端提示用户）。
#[tauri::command]
pub async fn export_diagnostic_bundle() -> CmdResult<String> {
    export_bundle_inner().await.stringify_err()
}

async fn export_bundle_inner() -> Result<String> {
    let home = dirs::app_home_dir()?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let out_dir: PathBuf = home.join("diagnostics");
    tokio::fs::create_dir_all(&out_dir)
        .await
        .with_context(|| format!("创建诊断目录失败: {}", out_dir.display()))?;
    let zip_path = out_dir.join(format!("clash-mini-diag-{stamp}.zip"));

    let file = std::fs::File::create(&zip_path)
        .with_context(|| format!("创建诊断包失败: {}", zip_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);

    // 1) 日志：打包当前全部轮转文件（默认约 24 小时）
    let mut log_files = 0usize;
    let logs_dir = home.join("logs");
    if let Ok(mut rd) = tokio::fs::read_dir(&logs_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("log") {
                continue;
            }
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
                continue;
            };
            if let Ok(text) = tokio::fs::read_to_string(&path).await {
                zip.start_file(format!("logs/{name}"), file_opts())?;
                zip.write_all(text.as_bytes())?;
                log_files += 1;
            }
        }
    }

    // 2) 配置：一律脱敏后再入包
    for (src_name, out_name) in [
        (
            crate::constants::files::RUNTIME_CONFIG,
            "runtime-config.redacted.yaml",
        ),
        ("verge.yaml", "verge.yaml.redacted.yaml"),
        ("profiles.yaml", "profiles.yaml.redacted.yaml"),
    ] {
        if let Ok(text) = tokio::fs::read_to_string(home.join(src_name)).await {
            zip.start_file(out_name, file_opts())?;
            zip.write_all(redact_yaml_text(&text).as_bytes())?;
        }
    }

    // 3) 版本与运行摘要
    zip.start_file("system-info.txt", file_opts())?;
    zip.write_all(collect_system_info().await.as_bytes())?;

    // 4) 说明
    zip.start_file("README.txt", file_opts())?;
    zip.write_all(BUNDLE_README.as_bytes())?;

    zip.finish()?;

    logging!(
        info,
        Type::Cmd,
        "诊断包已导出: {}（含 {} 个日志文件）",
        zip_path.display(),
        log_files
    );
    Ok(zip_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_masks_node_secrets() {
        let yaml = r#"
proxies:
  - name: 日本01
    type: ss
    server: 1.2.3.4
    port: 443
    password: super-secret
    uuid: 0000-1111
    psk: abcdef
"#;
        let out = redact_yaml_text(yaml);
        assert!(!out.contains("super-secret"), "password 未脱敏: {out}");
        assert!(!out.contains("0000-1111"), "uuid 未脱敏: {out}");
        assert!(!out.contains("abcdef"), "psk 未脱敏: {out}");
        // 非敏感字段保留，便于排查
        assert!(out.contains("日本01"));
        assert!(out.contains("1.2.3.4"));
    }

    #[test]
    fn redact_masks_subscription_urls() {
        let yaml = r#"
proxy-providers:
  main:
    url: https://airport.example/sub?token=very-secret-token
    interval: 3600
"#;
        let out = redact_yaml_text(yaml);
        assert!(!out.contains("very-secret-token"), "订阅链接未脱敏: {out}");
        assert!(out.contains("main"));
    }

    #[test]
    fn unparsable_yaml_never_leaks_body() {
        // 用制表符缩进构造必然解析失败的 YAML，且正文里带敏感串
        let bad = "leak: LEAK_ME\n\tbad_tab_indent: 1\n";
        let out = redact_yaml_text(bad);
        assert!(!out.contains("LEAK_ME"), "解析失败时仍泄露了正文: {out}");
        assert!(out.contains("配置解析失败"));
    }
}
