## 2026-06-13T13:16:31Z

You are a Worker (Worker 5).
Your working directory is c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_worker_profiles_gen3.
Your task is to re-implement the read-before-write optimization in `PrfItem::save_file` in `src-tauri/src/config/prfitem.rs` that was accidentally wiped out by a challenger.

Here is the exact implementation to place in `save_file` (lines 724-733):
```rust
    /// save the file data
    pub async fn save_file(&self, data: String) -> Result<()> {
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("could not find the file"))?;
        let path = dirs::app_profiles_dir()?.join(file.as_str());

        let should_write = match fs::read_to_string(&path).await {
            Ok(existing_content) => {
                if existing_content == data {
                    false
                } else {
                    existing_content.replace("\r\n", "\n") != data.replace("\r\n", "\n")
                }
            }
            Err(_) => true,
        };

        if should_write {
            fs::write(path, data.as_bytes())
                .await
                .context("failed to save the file")
        } else {
            Ok(())
        }
    }
```

Please run cargo check and cargo test to verify that the unit tests compile and pass.
MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Write your handoff.md in your directory, and reply with your progress.
