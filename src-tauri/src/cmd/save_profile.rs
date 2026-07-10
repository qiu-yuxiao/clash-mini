use super::CmdResult;
use crate::{
    cmd::StringifyErr as _,
    cmd::validate::{ValidationNoticeTarget, handle_validation_notice},
    config::{Config, IProfiles, PrfItem},
    core::{
        CoreManager, handle,
        validate::{CoreConfigValidator, ValidationOutcome},
    },
    module::auto_backup::{AutoBackupManager, AutoBackupTrigger},
    utils::dirs,
};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use tokio::fs;

/// 保存profiles的配置
#[tauri::command]
pub async fn save_profile_file(index: String, file_data: Option<String>) -> CmdResult<ValidationOutcome> {
    let file_data = match file_data {
        Some(d) => d,
        None => return Ok(ValidationOutcome::Valid),
    };

    let backup_trigger = match index.as_str() {
        "Merge" => Some(AutoBackupTrigger::GlobalMerge),
        "Script" => Some(AutoBackupTrigger::GlobalScript),
        _ => None,
    };

    // 在异步操作前获取必要元数据并释放锁
    let (rel_path, is_merge_file, is_script_file, affects_runtime) = {
        let profiles = Config::profiles().await;
        let profiles_guard = profiles.latest_arc();
        let item = profiles_guard.get_item(&index).stringify_err()?;
        let is_merge = item.itype.as_ref().is_some_and(|t| t == "merge");
        let path = item.file.clone().ok_or("file field is null")?;
        let is_script = item.itype.as_ref().is_some_and(|t| t == "script") || path.ends_with(".js");
        let affects_runtime = profile_affects_runtime(&profiles_guard, &index);
        (path, is_merge, is_script, affects_runtime)
    };

    // 读取原始内容（在释放profiles_guard后进行）
    let original_content = PrfItem {
        file: Some(rel_path.clone()),
        ..Default::default()
    }
    .read_file()
    .await
    .stringify_err()?;

    let unchanged = if original_content == file_data {
        true
    } else {
        original_content.replace("\r\n", "\n") == file_data.replace("\r\n", "\n")
    };

    if unchanged {
        return Ok(ValidationOutcome::Valid);
    }

    let profiles_dir = dirs::app_profiles_dir().stringify_err()?;
    let file_path = profiles_dir.join(rel_path.as_str());
    let file_path_str = file_path.to_string_lossy().to_string();

    // 保存新的配置文件
    fs::write(&file_path, &file_data).await.stringify_err()?;

    logging!(
        info,
        Type::Config,
        "[cmd配置save] 开始验证配置文件: {}, 是否为merge文件: {}",
        file_path_str,
        is_merge_file
    );

    let changes_applied = handle_saved_profile_file(
        &file_path_str,
        &file_path,
        &original_content,
        is_merge_file,
        is_script_file,
        affects_runtime,
    )
    .await?;

    if changes_applied.is_valid()
        && let Some(trigger) = backup_trigger
    {
        AutoBackupManager::trigger_backup(trigger);
    }

    Ok(changes_applied)
}

async fn restore_original(file_path: &std::path::Path, original_content: &str) -> Result<(), String> {
    fs::write(file_path, original_content).await.stringify_err()
}

fn profile_affects_runtime(profiles: &IProfiles, index: &str) -> bool {
    let Some(current_uid) = profiles.get_current() else {
        return false;
    };
    if current_uid == index {
        return true;
    }

    let Ok(item) = profiles.get_item(current_uid) else {
        return false;
    };
    [
        item.current_merge().map_or("Merge", String::as_str),
        item.current_script().map_or("Script", String::as_str),
        item.current_rules().map_or("Rules", String::as_str),
        item.current_proxies().map_or("Proxies", String::as_str),
        item.current_groups().map_or("Groups", String::as_str),
    ]
    .contains(&index)
}

async fn handle_saved_profile_file(
    file_path_str: &str,
    file_path: &std::path::Path,
    original_content: &str,
    is_merge_file: bool,
    is_script_file: bool,
    affects_runtime: bool,
) -> CmdResult<ValidationOutcome> {
    let (target, file_type) = if is_script_file {
        (ValidationNoticeTarget::Script, "脚本文件")
    } else if is_merge_file {
        (ValidationNoticeTarget::Merge, "合并配置文件")
    } else {
        (ValidationNoticeTarget::Runtime, "YAML配置文件")
    };

    logging!(
        info,
        Type::Config,
        "[cmd配置save] 开始{}验证: {}",
        file_type,
        file_path_str
    );

    match CoreConfigValidator::validate_config_file_outcome(file_path_str, Some(is_merge_file)).await {
        Ok(outcome) if outcome.is_valid() => {
            logging!(info, Type::Config, "[cmd配置save] 文件验证通过: {}", file_path_str);
        }
        Ok(outcome) => {
            logging!(warn, Type::Config, "[cmd配置save] 文件验证失败: {}", outcome);
            restore_original(file_path, original_content).await?;
            handle_validation_notice(&outcome, target, file_type);
            return Ok(outcome);
        }
        Err(e) => {
            logging!(error, Type::Config, "[cmd配置save] 验证过程发生错误: {}", e);
            restore_original(file_path, original_content).await?;
            return Err(e.to_string().into());
        }
    }

    if !affects_runtime {
        return Ok(ValidationOutcome::Valid);
    }

    logging!(
        info,
        Type::Config,
        "[cmd配置save] 保存项影响当前运行时配置，开始统一应用"
    );
    match CoreManager::global().update_config_forced().await {
        Ok(outcome) if outcome.is_valid() => {
            handle::Handle::refresh_clash();
            Ok(ValidationOutcome::Valid)
        }
        Ok(outcome) => {
            logging!(warn, Type::Config, "[cmd配置save] 运行时配置应用失败: {}", outcome);
            restore_original(file_path, original_content).await?;
            handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "运行时配置");
            Ok(outcome)
        }
        Err(err) => {
            logging!(error, Type::Config, "[cmd配置save] 运行时配置应用错误: {}", err);
            restore_original(file_path, original_content).await?;
            Err(err.to_string().into())
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_profile_file_read_before_write() {
        // Force portable flag to true
        let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);

        // Ensure profiles directory exists
        let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
        tokio::fs::create_dir_all(&profiles_dir)
            .await
            .expect("failed to create profiles dir");

        let file_name = "test_save_profile_file_r_b_w.yaml";
        let file_path = profiles_dir.join(file_name);

        // Create initial file content
        let initial_content = "key: value\r\nlist:\r\n  - item1\r\n";
        tokio::fs::write(&file_path, initial_content.as_bytes())
            .await
            .expect("write initial file failed");

        // Insert the item into profiles config draft
        let index = "test_index_rbw";
        let profiles_draft = Config::profiles().await;
        profiles_draft.edit_draft(|d| {
            let item = PrfItem {
                uid: Some(index.into()),
                file: Some(file_name.into()),
                itype: Some("local".into()), // itype is local so we don't have triggers
                ..Default::default()
            };
            d.items = Some(vec![item]);
        });
        profiles_draft.apply();

        // Check file status
        let metadata_first = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_first = metadata_first.modified().expect("modified time failed");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Call save_profile_file with identical content (different line endings)
        // Since the content is identical normalized, it should return Ok(ValidationOutcome::Valid)
        // and skip any file write (mtime should not change).
        let identical_content = "key: value\nlist:\n  - item1\n";
        let outcome = save_profile_file(index.into(), Some(identical_content.into()))
            .await
            .expect("save_profile_file failed");
        assert!(outcome.is_valid());

        let metadata_second = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_second = metadata_second.modified().expect("modified time failed");

        assert_eq!(
            mtime_first, mtime_second,
            "mtime changed, meaning save_profile_file wrote to the file unnecessarily"
        );

        // Clean up
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    #[tokio::test]
    async fn test_save_profile_file_edge_cases() {
        // Force portable flag to true
        let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);

        // Ensure profiles directory exists
        let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
        tokio::fs::create_dir_all(&profiles_dir)
            .await
            .expect("failed to create profiles dir");

        let file_name = "test_save_profile_file_edges.yaml";
        let file_path = profiles_dir.join(file_name);

        // Create initial file content (empty string)
        tokio::fs::write(&file_path, b"")
            .await
            .expect("write initial file failed");

        // Insert the item into profiles config draft
        let index = "test_index_edges";
        let profiles_draft = Config::profiles().await;
        profiles_draft.edit_draft(|d| {
            let item = PrfItem {
                uid: Some(index.into()),
                file: Some(file_name.into()),
                itype: Some("local".into()),
                ..Default::default()
            };
            d.items = Some(vec![item]);
        });
        profiles_draft.apply();

        // 1. Check empty string is saved without modifying time on redundant call
        let metadata_1 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_1 = metadata_1.modified().expect("modified time failed");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let empty_content = "";
        let outcome = save_profile_file(index.into(), Some(empty_content.into()))
            .await
            .expect("save_profile_file empty failed");
        assert!(outcome.is_valid());

        let metadata_2 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_2 = metadata_2.modified().expect("modified time failed");
        assert_eq!(mtime_1, mtime_2, "mtime changed for redundant empty string save");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 2. CRLF vs LF edge cases
        // Pre-write crlf content directly
        let crlf_content = "key: value\r\nlist:\r\n  - item1\r\n";
        tokio::fs::write(&file_path, crlf_content.as_bytes())
            .await
            .expect("write crlf directly failed");

        let metadata_3 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_3 = metadata_3.modified().expect("modified time failed");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Call save_profile_file with LF content (which resolves to same normalized form)
        let lf_content = "key: value\nlist:\n  - item1\n";
        let outcome = save_profile_file(index.into(), Some(lf_content.into()))
            .await
            .expect("save_profile_file lf failed");
        assert!(outcome.is_valid());

        let metadata_4 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_4 = metadata_4.modified().expect("modified time failed");
        assert_eq!(
            mtime_3, mtime_4,
            "mtime changed unnecessarily for crlf/lf normalized equivalence"
        );

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 3. Type safety with smartstring (short/inline vs long/allocated)
        // Short smartstring
        let short_content = "foo: bar\n";
        tokio::fs::write(&file_path, short_content.as_bytes())
            .await
            .expect("write short content directly failed");

        let metadata_5 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_5 = metadata_5.modified().expect("modified time failed");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let short_data: smartstring::alias::String = "foo: bar\n".into();
        let outcome = save_profile_file(index.into(), Some(short_data))
            .await
            .expect("save_profile_file short data failed");
        assert!(outcome.is_valid());

        let metadata_6 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_6 = metadata_6.modified().expect("modified time failed");
        assert_eq!(
            mtime_5, mtime_6,
            "mtime changed unnecessarily for identical short smartstring"
        );

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Long smartstring
        let long_raw_yaml = format!("key: {}\n", "a".repeat(100));
        tokio::fs::write(&file_path, long_raw_yaml.as_bytes())
            .await
            .expect("write long content directly failed");

        let metadata_7 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_7 = metadata_7.modified().expect("modified time failed");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let long_data: smartstring::alias::String = long_raw_yaml.as_str().into();
        let outcome = save_profile_file(index.into(), Some(long_data))
            .await
            .expect("save_profile_file long data failed");
        assert!(outcome.is_valid());

        let metadata_8 = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_8 = metadata_8.modified().expect("modified time failed");
        assert_eq!(
            mtime_7, mtime_8,
            "mtime changed unnecessarily for identical long smartstring"
        );

        // Clean up
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    #[tokio::test]
    async fn test_save_profile_file_missing_file() {
        // Force portable flag to true
        let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);

        // Ensure profiles directory exists
        let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
        tokio::fs::create_dir_all(&profiles_dir)
            .await
            .expect("failed to create profiles dir");

        let file_name = "test_save_profile_file_missing.yaml";
        let file_path = profiles_dir.join(file_name);

        // Clean up any existing file
        let _ = tokio::fs::remove_file(&file_path).await;

        // Insert the item into profiles config draft
        let index = "test_index_missing";
        let profiles_draft = Config::profiles().await;
        profiles_draft.edit_draft(|d| {
            let item = PrfItem {
                uid: Some(index.into()),
                file: Some(file_name.into()),
                itype: Some("local".into()),
                ..Default::default()
            };
            d.items = Some(vec![item]);
        });
        profiles_draft.apply();

        // The file is missing, so calling save_profile_file should fail
        let result = save_profile_file(index.into(), Some("some: yaml\n".into())).await;
        assert!(
            result.is_err(),
            "Expected save_profile_file to fail when target file is missing"
        );

        // Clean up
        let _ = tokio::fs::remove_file(&file_path).await;
    }
}
