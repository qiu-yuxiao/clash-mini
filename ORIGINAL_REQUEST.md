# Original User Request

## Initial Request — 2026-06-13T16:07:12Z

An optimization task to profile, locate, and fix the root causes of high CPU usage and frequent/heavy disk read/write operations in the Clash Mini project.

Working directory: c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge
Integrity mode: development

## Requirements

### R1. Audit Frontend CPU & IPC Usage
Identify React component re-renders, WebSocket subscriptions, or Tauri IPC events that consume high CPU. Ensure that background hooks (such as traffic and connections monitoring) are completely throttled or disconnected when the application window is hidden or minimized.

### R2. Audit Backend CPU & Disk I/O
Locate frequent disk write operations in the Rust backend (e.g., config saves, log file updates, profile updates) and background thread sleep/check loops. Ensure no operations write to disk repeatedly without changes or spin in unthrottled hot loops.

### R3. Implement Targeted Refactoring
Implement optimizations such as throttling updates, caching values, writing to files only on mutation, and suspending active background query/subscription loops when hidden.

## Acceptance Criteria

### Performance Optimization
- [ ] No file writes (like configs, YAMLs, or profiles) are triggered repeatedly unless the contents actually change.
- [ ] All high-frequency WebSocket streams and network traffic updates are paused or throttled to low frequency when `pageVisible === false`.
- [ ] Backend loop checking functions (like the service and proxy guard checks) utilize throttled timings and yield control correctly to prevent hot spinning.
- [ ] All code modifications comply strictly with the rules in `clash_mini_agreements.md`.

## Follow-up — 2026-06-15T04:29:51+08:00

An analysis and design task to profile frontend-backend IPC data payloads, identify the root causes of the high IPC communication throughput (38MB+ in 15 seconds) in Clash Mini, and propose a global optimization design (such as data difference diffs, payload pruning, and event throttling) to reduce the throughput close to Clash Verge's level (~4.4MB) without writing code.

Working directory: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge
Integrity mode: benchmark

## Requirements

### R1. Audit Frontend & Backend IPC Event Bottlenecks
Perform a complete static analysis of the codebase to identify all high-frequency and high-volume Tauri IPC events (e.g., `/traffic`, `/connections`, `/logs`). Map out the exact Rust backend emitter code, the TypeScript frontend listener code, and the average payload structure.

### R2. Design Global Diff-Based Optimization Protocol
Design a comprehensive optimization proposal that specifies:
- A differential update protocol (e.g., key-value diffing, JSON patch, or delta push) for heavy states like connection lists.
- Event throttling and visibility-based subscription pausing.
- Backwards-compatible data schemas for both Rust structs and React state hooks.

### R3. Output Document
Write the final, complete design proposal document to the workspace at docs/ipc_optimization_proposal.md.

## Acceptance Criteria

### Deliverable Verification
- [ ] The optimization proposal file exists at `docs/ipc_optimization_proposal.md`.
- [ ] The file is fully formatted in markdown, with no placeholders or TODOs.

### Analysis & Audit Completeness
- [ ] The document lists all active high-frequency IPC events, including payload sizes and their corresponding code files.
- [ ] The document pinpoints the exact cause of the 38MB payload size discrepancy compared to Clash Verge.

### Design Protocol Quality
- [ ] The proposal defines concrete data structure schemas (Rust structs and TypeScript types) for differential updates.
- [ ] The design specifies how updates are throttled or suspended when the application window is hidden or minimized.
- [ ] The proposal includes a theoretical performance estimation showing how the payload volume will be reduced to ~4.4MB.
