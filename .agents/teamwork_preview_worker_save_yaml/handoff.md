# Handoff Report — Save YAML Read-Before-Write Optimization

## 1. Observation
- The target function `save_yaml` is defined in `src-tauri/src/utils/help.rs` (lines 61-75) as:
  ```rust
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
- Two attempts to run `cargo check` in the `src-tauri` directory timed out waiting for user permission on the command-line execution:
  ```
  Encountered error in step execution: Permission prompt for action 'command' on target 'cargo check' timed out waiting for user response.
  ```

## 2. Logic Chain
- Unconditional writes to disk and sleeps (50ms) are performed on every `save_yaml` invocation.
- If the file exists and its content matches the newly serialized YAML data, the write and sleep operations are redundant.
- By reading the existing file bytes via `tokio::fs::read(path).await` and comparing them against the target YAML bytes, we can determine whether to bypass the write.
- If they are identical, we avoid calling `tokio::fs::write` and `tokio::time::sleep`.
- If the file does not exist or matches incorrectly (or reading it fails for any reason), we proceed with writing the file and sleeping for 50ms.
- To verify this optimization, we can inspect the file's modification time (`mtime`). If identical data is saved, `mtime` must remain unchanged. If modified data is saved, `mtime` must update.

## 3. Caveats
- Since command execution was blocked due to permission prompt timeouts, we could not run `cargo check` or `cargo test` locally. The code changes and tests must be verified when execution permissions are granted.
- The unit test relies on the system temporary directory (`std::env::temp_dir()`) and sleeps for 100ms between writes to allow the filesystem's `mtime` resolution to detect changes.

## 4. Conclusion
- The read-before-write optimization was successfully implemented in `src-tauri/src/utils/help.rs`.
- A unit test module (`tests`) was added to the bottom of `src-tauri/src/utils/help.rs` with `test_save_yaml_read_before_write` to assert `mtime` behavior.

## 5. Verification Method
- **Verify Compilation**: In the `src-tauri` directory, run:
  ```powershell
  cargo check
  ```
- **Run Unit Tests**: In the `src-tauri` directory, run:
  ```powershell
  cargo test -- utils::help::tests
  ```
- **Files to Inspect**:
  - `src-tauri/src/utils/help.rs` (specifically the updated `save_yaml` function and the `tests` module at the bottom).
