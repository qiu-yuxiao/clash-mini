# Project: Clash Mini Pre-Release Code Audit

## Architecture
- Frontend: React UI built with Vite and MUI, utilizing Tauri API to communicate with Rust backend, tanstack/react-query for caching, and WebSocket for real-time traffic and connection data.
- Backend: Rust Tauri app containing system tray integration, clash config manager, profiles manager, sysproxy integration, and background worker threads.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | Frontend Code Audit | Static analysis of `src/pages/_layout.tsx` and all components in `src/pages/_layout/components/` against skin compatibility, concurrency/race conditions, resource leaks, and code quality. | None | DONE (Conv ID: 8e7a1460-c72c-4faa-8117-838e369ab890) |
| 2 | Backend Code Audit | Static analysis of `src-tauri/src/module/monitor.rs` and commands `proxy.rs`, `clash.rs`, `profile.rs` under `src-tauri/src/cmd/` against concurrency, safety, and correctness. | None | DONE (Conv ID: 6d62c7fa-20a7-4c9d-a5a0-606c5a40d870) |
| 3 | Report Synthesis & Review | Aggregating subagent findings, writing the final `audit_report.md` in the required path, and verifying formatting, standard links, and completeness. | 1, 2 | DONE (Output: .agents/orchestrator/audit_report.md) |

## Interface Contracts
- Front-to-Back: Communication uses Tauri IPC commands and WebSockets (`MihomoWebSocket`).
- State Preservation: Frontend states stored in LocalStorage and synchronized with backend via `save_proxy_head_state`.

## Code Layout
- Frontend Page: `src/pages/_layout.tsx`
- Frontend Components: `src/pages/_layout/components/`
- Backend Monitor Module: `src-tauri/src/module/monitor.rs`
- Backend Tauri Commands: `src-tauri/src/cmd/` (`proxy.rs`, `clash.rs`, `profile.rs`)
