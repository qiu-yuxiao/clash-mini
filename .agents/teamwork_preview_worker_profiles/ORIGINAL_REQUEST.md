## 2026-06-13T09:08:50Z
<USER_REQUEST>
You are a Worker subagent. Your task is to implement the read-before-write optimizations for profile save operations.
Locations:
1. `PrfItem::save_file` in `src-tauri/src/config/prfitem.rs`
2. `save_profile_file` in `src-tauri/src/cmd/save_profile.rs`

Requirements:
1. In `PrfItem::save_file` (in `prfitem.rs`), implement a read-before-write check. Read the existing file as a string, compare it with the incoming `data` string (normalizing line endings by replacing `\r\n` with `\n` to avoid OS-specific discrepancies), and only write the file if there is a difference. If content is unchanged, skip writing and return `Ok(())`.
2. In `save_profile_file` (in `save_profile.rs`), compare the incoming `file_data` with the already loaded `original_content` (normalizing line endings by replacing `\r\n` with `\n`). If they are identical, return `Ok(ValidationOutcome::Valid)` immediately, skipping the write, validation, reloads, and backup triggers.
3. Write unit tests in both files to verify that if contents are identical, no file writes occur (e.g. checking `mtime` or verifying no errors/reloads are triggered).
4. Run `cargo check` and `cargo test` in `src-tauri` directory.
5. Document all implementation details, test results, and file diffs in a handoff report at: `c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_worker_profiles\handoff.md`.

MANDATORY INTEGRITY WARNING — DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
</USER_REQUEST>
