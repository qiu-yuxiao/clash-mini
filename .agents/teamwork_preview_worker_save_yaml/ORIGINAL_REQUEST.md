## 2026-06-13T08:41:38Z
You are a Worker subagent. Your task is to implement the read-before-write optimization in `save_yaml` within `src-tauri/src/utils/help.rs`.
Refer to the Explorer handoff reports at:
- `c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_save_yaml_1\handoff.md`
- `c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_save_yaml_2\handoff.md`
- `c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_save_yaml_3\handoff.md`

Requirements:
1. Implement the read-before-write optimization in `save_yaml` so that it avoids calling `tokio::fs::write` and `tokio::time::sleep` if the file already exists and its contents match the new data.
2. Implement corresponding unit tests to verify the behavior (e.g. `mtime` is unchanged when saving identical data, but is updated when saving modified data).
3. Run `cargo check` and `cargo test` in the backend directory to verify compilation and correctness.
4. Report changes, compilation results, and test results back to the sub-orchestrator in a handoff report at: `c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_worker_save_yaml\handoff.md`.

MANDATORY INTEGRITY WARNING — DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
