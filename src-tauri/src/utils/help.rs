use crate::{config::with_encryption, enhance::seq::SeqMap};
use anyhow::{Context as _, Result, anyhow, bail};
use clash_verge_logging::{Type, logging};
use nanoid::nanoid;
use serde::{Serialize, de::DeserializeOwned};
use serde_yaml_ng::Mapping;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::{path::PathBuf, str::FromStr};

/// read data from yaml as struct T
pub async fn read_yaml<T: DeserializeOwned>(path: &PathBuf) -> Result<T> {
    if !tokio::fs::try_exists(path).await.unwrap_or(false) {
        bail!("file not found \"{}\"", path.display());
    }

    let yaml_str = tokio::fs::read_to_string(path).await?;

    Ok(with_encryption(|| async { serde_yaml_ng::from_str::<T>(&yaml_str) }).await?)
}

/// read mapping from yaml
pub async fn read_mapping(path: &PathBuf) -> Result<Mapping> {
    if !tokio::fs::try_exists(path).await.unwrap_or(false) {
        bail!("file not found \"{}\"", path.display());
    }

    let yaml_str = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read the file \"{}\"", path.display()))?;

    // YAML语法检查
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&yaml_str) {
        Ok(mut val) => {
            val.apply_merge()
                .with_context(|| format!("failed to apply merge \"{}\"", path.display()))?;

            Ok(val
                .as_mapping()
                .ok_or_else(|| anyhow!("failed to transform to yaml mapping \"{}\"", path.display()))?
                .to_owned())
        }
        Err(err) => {
            let error_msg = format!("YAML syntax error in {}: {}", path.display(), err);
            logging!(error, Type::Config, "{}", error_msg);

            crate::core::handle::Handle::notice_message("config_validate::yaml_syntax_error", &error_msg);

            bail!("YAML syntax error: {}", err)
        }
    }
}

/// read mapping from yaml fix #165
pub async fn read_seq_map(path: &PathBuf) -> Result<SeqMap> {
    read_yaml(path).await
}

/// save the data to the file
/// can set `prefix` string to add some comments
/// 使用原子写入（临时文件 + rename）避免 TOCTOU 竞态和写入中途损坏
pub async fn save_yaml<T: Serialize + Sync>(path: &PathBuf, data: &T, prefix: Option<&str>) -> Result<()> {
    let data_str = with_encryption(|| async { serde_yaml_ng::to_string(data) }).await?;

    let yaml_str = match prefix {
        Some(prefix) => format!("{prefix}\n\n{data_str}"),
        None => data_str,
    };

    let yaml_bytes = yaml_str.as_bytes();

    // 内容未变化时跳过写入，减少不必要的磁盘 IO
    if let Ok(existing_bytes) = tokio::fs::read(path).await {
        if existing_bytes == yaml_bytes {
            return Ok(());
        }
    }

    // 原子写入：先写临时文件，再 rename 到目标文件
    let parent_dir = path
        .parent()
        .ok_or_else(|| anyhow!("failed to get parent directory of \"{}\"", path.display()))?;

    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("failed to get file name of \"{}\"", path.display()))?;

    let tmp_file_name = format!("{}.tmp_{}", file_name.to_string_lossy(), std::process::id());
    let tmp_path = parent_dir.join(tmp_file_name);

    let path_str = path.as_os_str().to_string_lossy().to_string();
    let tmp_path_str = tmp_path.as_os_str().to_string_lossy().to_string();

    tokio::fs::write(&tmp_path, yaml_bytes)
        .await
        .with_context(|| format!("failed to write temp file \"{tmp_path_str}\""))?;

    // Windows 上 rename 不会自动覆盖已有文件：
    // 先尝试直接 rename（原文件不存在时成功），失败则尝试删除目标文件后重试
    // 若仍失败，则将临时文件保留（不删除），确保数据不会丢失
    let result = if cfg!(windows) {
        match std::fs::rename(&tmp_path, path) {
            ok @ Ok(_) => ok,
            Err(_) => {
                // 删除目标文件后重试（目标文件可能已存在）
                if let Err(e) = std::fs::remove_file(path) {
                    // 目标文件删不掉，把临时文件留下
                    return Err(e).with_context(|| format!("failed to save file \"{path_str}\" (cannot remove existing file)"));
                }
                std::fs::rename(&tmp_path, path)
            }
        }
    } else {
        std::fs::rename(&tmp_path, path)
    };

    if let Err(e) = result {
        // 重命名失败：临时文件还在目标位置（没被移动），但原文件可能已被删除（Windows 分支）
        // 尝试将临时文件重命名为原文件名的 .backup 后缀，尽可能保留数据
        let backup_path = path.with_extension("yaml.bak");
        let _ = std::fs::rename(&tmp_path, &backup_path);
        return Err(e).with_context(|| format!("failed to save file \"{path_str}\" (atomic rename failed)"));
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    Ok(())
}

const ALPHABET: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
    'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

/// generate the uid
pub fn get_uid(prefix: &str) -> String {
    let id = nanoid!(11, &ALPHABET);
    format!("{prefix}{id}")
}

/// parse the string
/// xxx=123123; => 123123
pub fn parse_str<T: FromStr>(target: &str, key: &str) -> Option<T> {
    target.split(';').map(str::trim).find_map(|s| {
        let mut parts = s.splitn(2, '=');
        match (parts.next(), parts.next()) {
            (Some(k), Some(v)) if k == key => v.parse::<T>().ok(),
            _ => None,
        }
    })
}

/// Mask sensitive parts of a subscription URL for safe logging.
/// Examples:
/// - `https://example.com/api/v1/clash?token=abc123` → `https://example.com/api/v1/clash?token=***`
/// - `https://example.com/abc123def456ghi789/clash` → `https://example.com/***/clash`
pub fn mask_url(url: &str) -> String {
    // Split off query string
    let (path_part, query_part) = match url.find('?') {
        Some(pos) => (&url[..pos], Some(&url[pos + 1..])),
        None => (url, None),
    };

    // Extract scheme+host prefix (everything up to the first '/' after "://")
    let host_end = path_part
        .find("://")
        .and_then(|scheme_end| {
            path_part[scheme_end + 3..]
                .find('/')
                .map(|slash| scheme_end + 3 + slash)
        })
        .unwrap_or(path_part.len());

    let scheme_and_host = &path_part[..host_end];
    let path = &path_part[host_end..]; // starts with '/' or empty

    let mut result = scheme_and_host.to_owned();

    // Mask path segments that look like tokens (longer than 16 chars)
    if !path.is_empty() {
        let masked: Vec<&str> = path
            .split('/')
            .map(|seg| if seg.len() > 16 { "***" } else { seg })
            .collect();
        result.push_str(&masked.join("/"));
    }

    // Keep query param keys, mask values
    if let Some(query) = query_part {
        result.push('?');
        let masked_query: Vec<String> = query
            .split('&')
            .map(|param| match param.find('=') {
                Some(eq) => format!("{}=***", &param[..eq]),
                None => param.to_owned(),
            })
            .collect();
        result.push_str(&masked_query.join("&"));
    }

    result
}

/// Mask all URLs embedded in an error/log string for safe logging.
///
/// Scans the string for `http://` or `https://` and replaces each URL
/// (terminated by whitespace or `)`, `]`, `"`, `'`) with its masked form.
/// Text between URLs is copied verbatim.
pub fn mask_err(err: &str) -> String {
    let mut result = String::with_capacity(err.len());
    let mut remaining = err;

    loop {
        let http = remaining.find("http://");
        let https = remaining.find("https://");
        let start = match (http, https) {
            (None, None) => {
                result.push_str(remaining);
                break;
            }
            (Some(a), None) | (None, Some(a)) => a,
            (Some(a), Some(b)) => a.min(b),
        };

        result.push_str(&remaining[..start]);
        remaining = &remaining[start..];

        let url_end = remaining
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '"' | '\''))
            .unwrap_or(remaining.len());

        result.push_str(&mask_url(&remaining[..url_end]));
        remaining = &remaining[url_end..];
    }

    result
}

/// get the last part of the url, if not found, return empty string
pub fn get_last_part_and_decode(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(""); // Splits URL and takes the path part
    let segments: Vec<&str> = path.split('/').collect();
    let last_segment = segments.last()?;

    Some(
        percent_encoding::percent_decode_str(last_segment)
            .decode_utf8_lossy()
            .to_string(),
    )
}

/// open file
pub fn open_file(path: PathBuf) -> Result<()> {
    open::that_detached(path.as_os_str())?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn linux_elevator() -> String {
    use std::process::Command;
    match Command::new("which").arg("pkexec").output() {
        Ok(output) => {
            if !output.stdout.is_empty() {
                // Convert the output to a string slice
                if let Ok(path) = std::str::from_utf8(&output.stdout) {
                    path.trim().to_string()
                } else {
                    "sudo".to_string()
                }
            } else {
                "sudo".to_string()
            }
        }
        Err(_) => "sudo".to_string(),
    }
}

#[cfg(target_os = "windows")]
/// copy the file to the dist path and return the dist path
pub fn snapshot_path(original_path: &Path) -> Result<PathBuf> {
    let temp_dir = original_path
        .parent()
        .ok_or_else(|| anyhow!("Invalid log path"))?
        .join("temp");

    std::fs::create_dir_all(&temp_dir)?;

    let temp_path = temp_dir.join(format!(
        "{}_{}.log",
        original_path.file_stem().unwrap_or_default().to_string_lossy(),
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    std::fs::copy(original_path, &temp_path)?;

    Ok(temp_path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::permissions_set_readonly_false)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_save_yaml_read_before_write() {
        let temp_dir = std::env::temp_dir();
        let file_name = format!("test_save_yaml_{}.yaml", get_uid(""));
        let file_path = temp_dir.join(file_name);

        let data = "hello world".to_string();

        // 1. Initial write
        save_yaml(&file_path, &data, None).await.unwrap();
        let metadata_first = std::fs::metadata(&file_path).unwrap();
        let mtime_first = metadata_first.modified().unwrap();

        // Sleep briefly to ensure resolution of modification times
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 2. Write same content again (should skip writing)
        save_yaml(&file_path, &data, None).await.unwrap();
        let metadata_second = std::fs::metadata(&file_path).unwrap();
        let mtime_second = metadata_second.modified().unwrap();
        assert_eq!(
            mtime_first, mtime_second,
            "Modification time should not change if content matches"
        );

        // 3. Write different content (should write)
        let new_data = "hello modified".to_string();
        save_yaml(&file_path, &new_data, None).await.unwrap();
        let metadata_third = std::fs::metadata(&file_path).unwrap();
        let mtime_third = metadata_third.modified().unwrap();
        assert_ne!(
            mtime_second, mtime_third,
            "Modification time must update when content changes"
        );

        // Cleanup
        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_save_yaml_non_existent_directory() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(get_uid("")).join("test.yaml");
        let data = "hello".to_string();

        let result = save_yaml(&file_path, &data, None).await;
        assert!(
            result.is_err(),
            "Saving to a non-existent directory should return an error"
        );
    }

    #[tokio::test]
    async fn test_save_yaml_read_only_file() {
        let temp_dir = std::env::temp_dir();
        let file_name = format!("test_save_yaml_readonly_{}.yaml", get_uid(""));
        let file_path = temp_dir.join(file_name);

        let data = "hello readonly".to_string();

        // 1. Initial write
        save_yaml(&file_path, &data, None).await.unwrap();

        // 2. Set read-only
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file_path, perms).unwrap();

        // 3. Save same content (should succeed because write is skipped)
        let result_same = save_yaml(&file_path, &data, None).await;
        assert!(
            result_same.is_ok(),
            "Saving same content to read-only file should succeed"
        );

        // 4. Save different content (should fail because write is attempted and denied)
        let new_data = "hello change".to_string();
        let result_diff = save_yaml(&file_path, &new_data, None).await;
        assert!(
            result_diff.is_err(),
            "Saving different content to read-only file should fail"
        );

        // Restore write permissions for cleanup
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_readonly(false);
        fs::set_permissions(&file_path, perms).unwrap();
        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_save_yaml_with_prefix() {
        let temp_dir = std::env::temp_dir();
        let file_name = format!("test_save_yaml_prefix_{}.yaml", get_uid(""));
        let file_path = temp_dir.join(file_name);

        let data = "hello prefix".to_string();
        let prefix = "# Important Comment";

        // 1. Initial write with prefix
        save_yaml(&file_path, &data, Some(prefix)).await.unwrap();
        let metadata_first = fs::metadata(&file_path).unwrap();
        let mtime_first = metadata_first.modified().unwrap();

        // Sleep briefly
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 2. Save same content and prefix (should skip write)
        save_yaml(&file_path, &data, Some(prefix)).await.unwrap();
        let metadata_second = fs::metadata(&file_path).unwrap();
        let mtime_second = metadata_second.modified().unwrap();
        assert_eq!(
            mtime_first, mtime_second,
            "Mtime should not change if content and prefix match"
        );

        // 3. Save same content with different prefix (should write)
        let new_prefix = "# Modified Comment";
        save_yaml(&file_path, &data, Some(new_prefix)).await.unwrap();
        let metadata_third = fs::metadata(&file_path).unwrap();
        let mtime_third = metadata_third.modified().unwrap();
        assert_ne!(mtime_second, mtime_third, "Mtime must change if prefix changes");

        // 4. Save same content with no prefix (should write)
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        save_yaml(&file_path, &data, None).await.unwrap();
        let metadata_fourth = fs::metadata(&file_path).unwrap();
        let mtime_fourth = metadata_fourth.modified().unwrap();
        assert_ne!(mtime_third, mtime_fourth, "Mtime must change if prefix is removed");

        let _ = std::fs::remove_file(file_path);
    }
}
