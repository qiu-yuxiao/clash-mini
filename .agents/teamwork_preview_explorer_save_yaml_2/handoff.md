# Handoff Report: save_yaml Optimization Investigation

This report presents our analysis, proposed optimization, and edge cases for the `save_yaml` function in `src-tauri/src/utils/help.rs`.

---

## 1. Observation

In `src-tauri/src/utils/help.rs`, the `save_yaml` function is defined as follows (lines 61-75):

```rust
/// save the data to the file
/// can set `prefix` string to add some comments
pub async fn save_yaml<T: Serialize + Sync>(path: &PathBuf, data: &T, prefix: Option<&str>) -> Result<()> {
    let data_str = with_encryption(|| async { serde_yaml_ng::to_string(data) }).await?;

    let yaml_str = match prefix {
        Some(prefix) => format!("{prefix}\n\n{data_str}"),
        None => data_str,
    };

    let path_str = path.as_os_str().to_string_lossy().to_string();
    tokio::fs::write(path, yaml_str.as_bytes())
        .await
        .with_context(|| format!("failed to save file \"{path_str}\""))?;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Ok(())
}
```

We also observed:
1. `save_yaml` is used to persist configurations across the application (`clash.rs`, `config.rs`, `profiles.rs`, `verge.rs`, `manager/config.rs`, `init.rs`).
2. If `with_encryption` is active and serializing encrypted fields (such as WebDAV credentials in `verge.rs` lines 212-237), a random nonce is generated via `getrandom::fill` in `encrypt.rs` lines 25-27, producing a different ciphertext on every invocation.

---

## 2. Logic Chain

1. Currently, `save_yaml` calls `tokio::fs::write` unconditionally.
2. Writing to a file changes its modification time (`mtime`) and triggers file watcher events (e.g. from the `notify` crate), which may cause redundant config-reload cycles or rendering updates.
3. Every write is followed by a compulsory `tokio::time::sleep(std::time::Duration::from_millis(50))` delay, stalling the calling task.
4. If the content on disk is already identical to `yaml_str`, the write and the delay are redundant.
5. By reading the existing file via `tokio::fs::read` and comparing the bytes to `yaml_str.as_bytes()`, we can determine if changes exist.
6. Skipping the write and sleep when content is identical saves disk I/O, prevents file watchers from triggering reloads, and removes the 50ms delay.
7. If the file does not exist or cannot be read, the comparison fails gracefully, and it proceeds with the write as normal.

---

## 3. Caveats

- **Non-deterministic Encryption**: When configuring WebDAV backup (encrypted fields in `verge.json`/`verge.yaml`), the encryption uses a random nonce. This results in a different serialized string on every save even if the underlying data has not changed. In this scenario, the read-before-write check will always register a difference and write to disk. This is a safe/correct fallback, but the optimization will not reduce writes for active WebDAV configs. For all other files (clash configs, profiles, templates, etc.) or when WebDAV is not configured, the optimization works perfectly.
- **Time-of-Check to Time-of-Use (TOCTOU)**: There is a minimal race condition window between checking if the file matches and executing the write. Since this function is not atomic and does not employ file locking in the first place, this does not add any new risks.
- **Minor read overhead**: In cases where the file *has* changed, we incur a read operation before writing. Since config files are small (typically < 100KB), the overhead of a read (< 1ms) is negligible compared to the write.

---

## 4. Conclusion

We recommend optimizing `save_yaml` in `src-tauri/src/utils/help.rs` to read the existing file content and compare it with the newly serialized bytes before writing.

### Implementation Plan (Proposed Diff)

```rust
pub async fn save_yaml<T: Serialize + Sync>(path: &PathBuf, data: &T, prefix: Option<&str>) -> Result<()> {
    let data_str = with_encryption(|| async { serde_yaml_ng::to_string(data) }).await?;

    let yaml_str = match prefix {
        Some(prefix) => format!("{prefix}\n\n{data_str}"),
        None => data_str,
    };

    let yaml_bytes = yaml_str.as_bytes();

    // Read the file and check if the content is unchanged
    let should_write = match tokio::fs::read(path).await {
        Ok(existing_bytes) => existing_bytes != yaml_bytes,
        Err(_) => true, // Write if file doesn't exist, has permission error, etc.
    };

    if should_write {
        let path_str = path.as_os_str().to_string_lossy().to_string();
        tokio::fs::write(path, yaml_bytes)
            .await
            .with_context(|| format!("failed to save file \"{path_str}\""))?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(())
}
```

---

## 5. Verification Method

### 1. Compile & Unit Tests
Run the compiler check and project unit tests in the `src-tauri` directory to ensure no compile errors:
```powershell
cd src-tauri
cargo check
cargo test
```

### 2. Integration Test Verification
A test case can be added to verify that unchanged data does not rewrite the file or update its `mtime`. Write this test under `src-tauri/src/utils/help.rs` or in a test file:

```rust
#[tokio::test]
async fn test_save_yaml_optimization() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.yaml");
    let test_data = "hello world".to_string();

    // 1. Initial write
    save_yaml(&file_path, &test_data, None).await.unwrap();
    let m1 = tokio::fs::metadata(&file_path).await.unwrap().modified().unwrap();

    // Wait 100ms to ensure a new mtime would be distinct
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2. Unchanged write - should skip writing
    save_yaml(&file_path, &test_data, None).await.unwrap();
    let m2 = tokio::fs::metadata(&file_path).await.unwrap().modified().unwrap();

    assert_eq!(m1, m2, "mtime should not change when content is identical");
}
```
