# Project: Clash Mini System Resource Optimization Audit

## Architecture
Clash Mini is a cross-platform GUI client built with React/TypeScript frontend and Rust/Tauri backend.
- **Frontend (src)**: Manages UI state, views, themes, configurations, and communicates with the backend via Tauri IPC (commands and event streams).
- **Backend (src-tauri)**: Manages Mihomo core lifecycle, connection states, logs, configuration files, system metrics, and emits telemetry to the frontend.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Frontend Resource Audit | Audit React render loops, hooks dependencies, WebSocket subscriptions, visibility-based throttling, and unnecessary renders. | none | DONE (Conv: 980b28a4-b6e4-463e-bec0-c4fa35b3ef08) |
| 2 | Backend Concurrency & Task Audit | Audit Rust backend thread management, tokio spawns, mutex lock contentions, CPU hot loops, and channel usage. | none | DONE (Conv: 134ccb9f-afcb-426e-935a-e5f57af5fca5) |
| 3 | Backend I/O & Socket Audit | Audit config writes, log writes, socket descriptors, file-handle leaks, and connection/traffic event buffering. | none | DONE (Conv: b8855edf-d5aa-4478-b99e-82b2334fb822) |
| 4 | Final Report Synthesis | Synthesize findings from M1-M3 into a structured markdown report containing precise paths, lines, analysis, and optimization diffs. | M1, M2, M3 | DONE |

## Code Layout
- **Frontend Source**: `src/`
- **Backend Source**: `src-tauri/`
- **Core Documents**:
  - `clash_mini_agreements.md`
  - `clash_mini_pitfalls.md`
  - `bug_list.md`
