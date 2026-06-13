# Scope: Milestone 3 - Backend Guard Loops Throttling

## Architecture
- Target files: `src-tauri/src/core/sysopt.rs` and `src-tauri/src/core/service.rs`.
- System Service Checks: wait mechanisms during startup or status handling.
- Proxy Guard: `sysproxy::GuardMonitor` settings and loop execution.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 3 | Backend Guard Loops | Audit `sysopt.rs` and `service.rs`. Ensure background checking loops (like service checks and proxy guard checks) utilize throttled timings, do not spin in unthrottled hot loops, and yield control correctly using tokio sleep/delay queue to prevent hot spinning. | None | DONE |

## Interface Contracts
- Service status checks and proxy guard checks must have proper throttling (e.g. non-zero delays/sleeps) and not spin/block.
- All code modifications must comply strictly with `clash_mini_agreements.md`.
