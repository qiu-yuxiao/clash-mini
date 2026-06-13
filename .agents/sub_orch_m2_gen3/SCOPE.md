# Scope: Milestone 2 - Backend Disk I/O Optimization

## Architecture
- `save_yaml` in `src-tauri/src/utils/help.rs` is the common utility for config saves.
- Profile storage saving is managed in `src-tauri/src/config/prfitem.rs` (`save_file`) and `src-tauri/src/cmd/save_profile.rs` (`save_profile_file`).

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 2.1 | save_yaml Optimization | Implement read-before-write comparison in `save_yaml` to avoid writing unchanged content to disk. | None | DONE |
| 2.2 | Profile Saves Optimization | Optimize `save_file` in `prfitem.rs` and `save_profile_file` in `save_profile.rs` to read and compare file content before writing. | None | DONE |

## Interface Contracts
- No file writes (like configs, YAMLs, or profiles) are triggered repeatedly unless the contents actually change.
- Diffs are evaluated on the exact byte sequence/string representation before calling write/save.
