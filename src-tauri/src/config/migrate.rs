//! 配置文件版本迁移框架
//!
//! ## 背景
//!
//! `IClashTemp` / `IVerge` / `IProfiles` 直接序列化到 YAML。当配置格式发生**破坏性变更**
//!（字段重命名、类型变更、结构调整、默认值语义变化）时，旧配置文件可能反序列化失败或
//! 静默丢数据，且无法判断用户配置来自哪个版本，难以做定向迁移。
//!
//! ## 约定
//!
//! - 各配置文件顶层带 `config_version: u32`；**缺失视为 v1**（历史版本）。
//! - 版本号**独立于应用版本号**，仅在配置格式发生破坏性变更时 +1。
//! - 读取时按版本号**逐级**执行迁移函数；写入时始终写当前版本号。
//! - 迁移函数必须是**纯函数**（只改内存结构、不做 IO），便于单元测试。
//! - 真正执行迁移前先备份原文件为 `<name>.bak.v<from>`；**备份失败则中止本次迁移**
//!   （配置保持旧版本原样，下次启动重试），绝不在无备份时改写用户配置。
//! - 配置版本**高于**本程序支持的版本（用户降级）→ 只告警、尽量兼容读取，
//!   绝不写回降级，避免把高版本配置「改坏」。
//!
//! ## 新增一次迁移的步骤
//!
//! 1. `CURRENT_*_CONFIG_VERSION` +1；
//! 2. 写一个 `migrate_verge_vN_to_vN1(&mut IVerge)`；
//! 3. 在 `migrate_*` 的 `match` 里挂上；
//! 4. 在文件末尾的 `#[cfg(test)]` 里补一条单测。

use std::path::{Path, PathBuf};

use clash_verge_logging::{Type, logging};

use crate::config::{IProfiles, IVerge};

/// 缺失 `config_version` 的历史配置统一视为 v1。
pub const IMPLICIT_LEGACY_VERSION: u32 = 1;

/// `verge.yaml` 当前配置版本。
pub const CURRENT_VERGE_CONFIG_VERSION: u32 = 3;

/// `profiles.yaml` 当前配置版本。
pub const CURRENT_PROFILES_CONFIG_VERSION: u32 = 1;

/// `clash.yaml` 当前配置版本。
pub const CURRENT_CLASH_CONFIG_VERSION: u32 = 2;

/// 读取配置 Mapping 中的 `config_version`；缺失视为 v1（历史版本）。
///
/// 【顺序约束·勿改】调用方必须在**模板/默认值合并之前**调用本函数：
/// 模板通常自带 `config_version = 当前版本`，若先合并模板再检测，
/// 存量配置（无 config_version）的起始版本会被误判为最新版，
/// v1→vN 迁移将永远不会触发（clash.yaml 集成曾因顺序颠倒踩过此坑）。
pub fn detect_config_version(map: &serde_yaml_ng::Mapping) -> u32 {
    map.get("config_version")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(IMPLICIT_LEGACY_VERSION)
}

/// 迁移前备份原配置文件，返回备份路径（失败返回 `None`）。
///
/// 备份文件命名 `<原文件名>.bak.v<from_version>`，便于用户回滚与事后追查。
pub fn backup_before_migration(path: &Path, from_version: u32) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let backup = path.with_file_name(format!("{file_name}.bak.v{from_version}"));
    if backup.exists() {
        // 同一版本只备份一次，避免重复启动时反复覆盖（保留最早那份原始配置）
        logging!(info, Type::Config, "配置迁移备份已存在，跳过覆盖: {}", backup.display());
        return Some(backup);
    }
    match std::fs::copy(path, &backup) {
        Ok(_) => {
            logging!(info, Type::Config, "配置迁移前已备份: {}", backup.display());
            Some(backup)
        }
        Err(e) => {
            logging!(
                warn,
                Type::Config,
                "配置迁移备份失败（调用方应中止本次迁移，配置保持原样）: {e}"
            );
            None
        }
    }
}

/// 对 `IVerge` 执行版本迁移（纯函数）。返回迁移后的版本号。
pub fn migrate_verge(cfg: &mut IVerge, from_version: u32) -> u32 {
    let mut version = from_version.max(IMPLICIT_LEGACY_VERSION);

    if version > CURRENT_VERGE_CONFIG_VERSION {
        logging!(
            warn,
            Type::Config,
            "verge.yaml 配置版本 {version} 高于本程序支持的 {CURRENT_VERGE_CONFIG_VERSION}（可能由更高版本创建）：仅告警，尽量兼容读取，不执行迁移、不写回降级"
        );
        return version;
    }

    while version < CURRENT_VERGE_CONFIG_VERSION {
        match version {
            1 => migrate_verge_v1_to_v2(cfg),
            2 => migrate_verge_v2_to_v3(cfg),
            other => {
                logging!(
                    warn,
                    Type::Config,
                    "缺少 verge.yaml v{other} 的迁移实现，停止迁移（配置停在 v{other}）"
                );
                break;
            }
        }
        version += 1;
    }

    version
}

/// v1 → v2：把历史遗留的 `start_page == "/home"` 归一化为 `"/"`。
///
/// 这段兼容原先作为临时补丁写在 `IVerge::new()` 里，收编为正式迁移，
/// 使后续同类兼容逻辑有统一落点、可被单测覆盖。
fn migrate_verge_v1_to_v2(cfg: &mut IVerge) {
    if cfg.start_page.as_deref() == Some("/home") {
        cfg.start_page = Some("/".into());
        logging!(
            info,
            Type::Config,
            "配置迁移 v1→v2：start_page \"/home\" 归一化为 \"/\""
        );
    }
}

/// v2 → v3：把日志保留个数从旧默认值 8 提升到 40（≈4.5 小时 → ≈26 小时）。
///
/// 背景：128KB × 8 ≈ 4.5 小时。实测一次断流事故排查几乎要翻遍全部日志，
/// 且用户若次日才反馈，现场已被轮转删除。**仅当用户仍是旧默认值 8 时才调整**，
/// 用户自定义过的其它值一律尊重。
fn migrate_verge_v2_to_v3(cfg: &mut IVerge) {
    const OLD_DEFAULT_COUNT: usize = 8;
    const NEW_DEFAULT_COUNT: usize = 40;
    if cfg.app_log_max_count == Some(OLD_DEFAULT_COUNT) {
        cfg.app_log_max_count = Some(NEW_DEFAULT_COUNT);
        logging!(
            info,
            Type::Config,
            "配置迁移 v2→v3：日志保留个数 {OLD_DEFAULT_COUNT} → {NEW_DEFAULT_COUNT}（约 4.5 小时 → 约 26 小时）"
        );
    }
}

/// 对 `IProfiles` 执行版本迁移（纯函数）。返回迁移后的版本号。
///
/// 目前尚无破坏性变更（当前版本为 v1）。保留完整骨架，便于后续扩展：
/// 新增迁移时只需 +1 版本号并挂上 `match` 分支。
pub fn migrate_profiles(_profiles: &mut IProfiles, from_version: u32) -> u32 {
    let version = from_version.max(IMPLICIT_LEGACY_VERSION);

    if version > CURRENT_PROFILES_CONFIG_VERSION {
        logging!(
            warn,
            Type::Config,
            "profiles.yaml 配置版本 {version} 高于本程序支持的 {CURRENT_PROFILES_CONFIG_VERSION}：仅告警，尽量兼容读取，不执行迁移"
        );
        return version;
    }

    if version < CURRENT_PROFILES_CONFIG_VERSION {
        logging!(
            warn,
            Type::Config,
            "缺少 profiles.yaml v{version} 的迁移实现，停止迁移（配置停在 v{version}）"
        );
    }

    version
}

/// 对 `IClashTemp` 底层的 `serde_yaml_ng::Mapping` 执行版本迁移（纯函数）。返回迁移后的版本号。
pub fn migrate_clash(map: &mut serde_yaml_ng::Mapping, from_version: u32) -> u32 {
    let mut version = from_version.max(IMPLICIT_LEGACY_VERSION);

    if version > CURRENT_CLASH_CONFIG_VERSION {
        logging!(
            warn,
            Type::Config,
            "clash.yaml 配置版本 {version} 高于本程序支持的 {CURRENT_CLASH_CONFIG_VERSION}：仅告警，尽量兼容读取，不执行迁移、不写回降级"
        );
        return version;
    }

    while version < CURRENT_CLASH_CONFIG_VERSION {
        match version {
            1 => migrate_clash_v1_to_v2(map),
            other => {
                logging!(
                    warn,
                    Type::Config,
                    "缺少 clash.yaml v{other} 的迁移实现，停止迁移（配置停在 v{other}）"
                );
                break;
            }
        }
        version += 1;
    }

    version
}

/// v1 → v2：归一化 tun.stack（旧默认 "system" 改为跨平台更稳定的 "gvisor"）。
fn migrate_clash_v1_to_v2(map: &mut serde_yaml_ng::Mapping) {
    if let Some(tun) = map.get_mut("tun").and_then(|v| v.as_mapping_mut()) {
        if tun.get("stack").and_then(|v| v.as_str()) == Some("system") {
            tun.insert("stack".into(), crate::constants::tun::DEFAULT_STACK.into());
            logging!(
                info,
                Type::Config,
                "配置迁移 v1→v2：clash.yaml tun.stack \"system\" 归一化为 \"{}\"",
                crate::constants::tun::DEFAULT_STACK
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_config_without_version_is_treated_as_v1() {
        let mut cfg = IVerge::default();
        cfg.config_version = None;
        // 缺失版本 = v1，应迁移到当前版本
        let v = migrate_verge(&mut cfg, IMPLICIT_LEGACY_VERSION);
        assert_eq!(v, CURRENT_VERGE_CONFIG_VERSION);
    }

    #[test]
    fn v1_to_v2_normalizes_legacy_start_page() {
        let mut cfg = IVerge::default();
        cfg.start_page = Some("/home".into());
        migrate_verge(&mut cfg, 1);
        assert_eq!(cfg.start_page.as_deref(), Some("/"));
    }

    #[test]
    fn v1_to_v2_keeps_other_start_pages() {
        let mut cfg = IVerge::default();
        cfg.start_page = Some("/proxy".into());
        migrate_verge(&mut cfg, 1);
        assert_eq!(cfg.start_page.as_deref(), Some("/proxy"));
    }

    #[test]
    fn migration_is_idempotent() {
        let mut cfg = IVerge::default();
        cfg.start_page = Some("/home".into());
        migrate_verge(&mut cfg, 1);
        let after_first = cfg.start_page.clone();
        // 再跑一次（已是最新版本）不应改变任何东西
        migrate_verge(&mut cfg, CURRENT_VERGE_CONFIG_VERSION);
        assert_eq!(cfg.start_page, after_first);
    }

    #[test]
    fn future_version_is_not_downgraded() {
        let mut cfg = IVerge::default();
        cfg.start_page = Some("/home".into());
        // 高于当前程序支持的版本 → 不迁移、不降级，原样返回
        let v = migrate_verge(&mut cfg, CURRENT_VERGE_CONFIG_VERSION + 5);
        assert_eq!(v, CURRENT_VERGE_CONFIG_VERSION + 5);
        // 迁移函数未被调用，start_page 保持原值
        assert_eq!(cfg.start_page.as_deref(), Some("/home"));
    }

    #[test]
    fn v2_to_v3_raises_legacy_log_count() {
        let mut cfg = IVerge::default();
        cfg.app_log_max_count = Some(8);
        migrate_verge(&mut cfg, 2);
        assert_eq!(cfg.app_log_max_count, Some(40));
    }

    #[test]
    fn v2_to_v3_keeps_custom_log_count() {
        let mut cfg = IVerge::default();
        cfg.app_log_max_count = Some(12);
        migrate_verge(&mut cfg, 2);
        assert_eq!(cfg.app_log_max_count, Some(12));
    }

    #[test]
    fn legacy_clash_config_without_version_is_treated_as_v1() {
        let mut map = serde_yaml_ng::Mapping::new();
        let mut tun = serde_yaml_ng::Mapping::new();
        tun.insert("stack".into(), "system".into());
        map.insert("tun".into(), tun.into());

        let v = migrate_clash(&mut map, IMPLICIT_LEGACY_VERSION);
        assert_eq!(v, CURRENT_CLASH_CONFIG_VERSION);

        let tun_res = map.get("tun").and_then(|t| t.as_mapping()).unwrap();
        assert_eq!(
            tun_res.get("stack").and_then(|s| s.as_str()),
            Some(crate::constants::tun::DEFAULT_STACK)
        );
    }

    #[test]
    fn clash_migration_is_idempotent() {
        let mut map = serde_yaml_ng::Mapping::new();
        let mut tun = serde_yaml_ng::Mapping::new();
        tun.insert("stack".into(), "gvisor".into());
        map.insert("tun".into(), tun.into());

        let v = migrate_clash(&mut map, CURRENT_CLASH_CONFIG_VERSION);
        assert_eq!(v, CURRENT_CLASH_CONFIG_VERSION);

        let tun_res = map.get("tun").and_then(|t| t.as_mapping()).unwrap();
        assert_eq!(tun_res.get("stack").and_then(|s| s.as_str()), Some("gvisor"));
    }

    #[test]
    fn future_clash_version_is_not_downgraded() {
        let mut map = serde_yaml_ng::Mapping::new();
        let mut tun = serde_yaml_ng::Mapping::new();
        tun.insert("stack".into(), "system".into());
        map.insert("tun".into(), tun.into());

        let future_ver = CURRENT_CLASH_CONFIG_VERSION + 3;
        let v = migrate_clash(&mut map, future_ver);
        assert_eq!(v, future_ver);

        // 不应被迁移降级改写
        let tun_res = map.get("tun").and_then(|t| t.as_mapping()).unwrap();
        assert_eq!(tun_res.get("stack").and_then(|s| s.as_str()), Some("system"));
    }

    #[test]
    fn detect_config_version_defaults_to_v1_when_missing() {
        let map = serde_yaml_ng::Mapping::new();
        assert_eq!(detect_config_version(&map), IMPLICIT_LEGACY_VERSION);
    }

    #[test]
    fn detect_config_version_reads_explicit_version() {
        let mut map = serde_yaml_ng::Mapping::new();
        map.insert("config_version".into(), 3u32.into());
        assert_eq!(detect_config_version(&map), 3);
    }

    /// 回归：模板合并会注入 `config_version = 当前版本`。若版本检测发生在合并之后，
    /// 存量配置会被误判为最新版、v1→v2 迁移（tun.stack "system" 归一化）整体失效。
    /// 检测必须先于模板合并 —— 本用例锁定该顺序契约。
    #[test]
    fn detection_must_precede_template_merge() {
        // 存量配置：无 config_version，tun.stack 为旧默认 "system"
        let mut legacy = serde_yaml_ng::Mapping::new();
        let mut tun = serde_yaml_ng::Mapping::new();
        tun.insert("stack".into(), "system".into());
        legacy.insert("tun".into(), tun.into());

        // 合并前检测：正确识别为 v1
        let from_version = detect_config_version(&legacy);
        assert_eq!(from_version, IMPLICIT_LEGACY_VERSION);

        // 模拟模板合并（注入 config_version = 当前版本）：合并后检测会被污染为最新版，
        // 这正是集成顺序颠倒时迁移失效的机理
        let mut merged = legacy.clone();
        merged.insert("config_version".into(), CURRENT_CLASH_CONFIG_VERSION.into());
        assert_eq!(detect_config_version(&merged), CURRENT_CLASH_CONFIG_VERSION);

        // 以合并前检测到的 v1 为起始版本，tun.stack "system" 才会被归一化
        let v = migrate_clash(&mut legacy, from_version);
        assert_eq!(v, CURRENT_CLASH_CONFIG_VERSION);
        let tun_res = legacy.get("tun").and_then(|t| t.as_mapping()).unwrap();
        assert_eq!(
            tun_res.get("stack").and_then(|s| s.as_str()),
            Some(crate::constants::tun::DEFAULT_STACK)
        );
    }
}
