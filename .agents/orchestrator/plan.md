# Clash Mini Performance Optimization Plan

## Objective
Profile, locate, and fix the root causes of high CPU usage and frequent/heavy disk read/write operations in the Clash Mini project, complying strictly with `clash_mini_agreements.md`.

## Decomposition into Milestones
1. **Milestone 1: Frontend CPU & IPC Optimization**
   - Target files: `src/hooks/use-visibility.ts`, `src/hooks/use-traffic-monitor.ts`, `src/hooks/use-log-data.ts`.
   - Goal: Pause WebSocket subscriptions and background worker threads when page visibility is false (hidden/minimized). Detect minimizing/hidden states using Tauri's window APIs.
2. **Milestone 2: Backend Disk I/O Optimization**
   - Target files: `src-tauri/src/utils/help.rs`, `src-tauri/src/config/prfitem.rs`, `src-tauri/src/cmd/save_profile.rs`.
   - Goal: Read and compare file contents before writing. Avoid redundant disk I/O when file contents are unchanged.
3. **Milestone 3: Backend Guard Loop Throttling**
   - Target files: `src-tauri/src/core/sysopt.rs`, `src-tauri/src/core/service.rs`.
   - Goal: Ensure system service and proxy guard checks use throttled timings and yield control correctly using tokio sleep/delay queue to prevent hot spinning.

## Execution Strategy
- For each milestone, we will invoke a `sub_orch` subagent to manage the milestone's implementation.
- Each sub-orchestrator will run the iteration loop: Explorer -> Worker -> Reviewer -> Challenger -> Auditor.
- Verification checks: unit tests must pass, and the Forensic Auditor must verify the changes for security and integrity.
