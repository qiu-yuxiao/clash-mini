# Project: Clash Mini Performance Optimization

## Architecture
- Frontend: React UI built with Vite and MUI, utilizing Tauri API to communicate with Rust backend, tanstack/react-query for caching, and WebSocket for real-time traffic and connection data.
- Backend: Rust Tauri app containing system tray integration, clash config manager, profiles manager, sysproxy integration, and background worker threads.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | Frontend CPU & IPC | Optimize `useVisibility`, `useTrafficMonitorEnhanced`, and `useLogData` to disable high-frequency WebSockets/timers when window is hidden or minimized. | None | DONE (Handoff: .agents/sub_orch_m1/handoff.md) |
| 2 | Backend Disk I/O | Optimize `save_yaml` and profile writes to prevent redundant disk I/O when file contents have not changed. | None | DONE (Handoff: .agents/sub_orch_m2_gen3/handoff.md) |
| 3 | Backend Guard Loops | Review and throttle background checking loops (like service waits or proxy guard checks) using yield control and tokio sleep. | None | DONE (Handoff: .agents/sub_orch_m3/handoff.md) |

## Interface Contracts
### Frontend ↔ Backend (WebSocket API)
- `MihomoWebSocket` connections are opened only when the window is active and visibility state is visible.
- Re-render triggers are minimized and throttled when the page is visible.
- When hidden/minimized, the subscriptions are completely closed and background workers stop.

### Configuration Storage
- `save_yaml` in `utils/help.rs` is the common interface for config saves.
- Profile storage saving is managed in `config/prfitem.rs` and `cmd/save_profile.rs`.

## Code Layout
- Frontend Source: `src/`
- Frontend Hooks: `src/hooks/`
- Tauri Backend Source: `src-tauri/src/`
- Backend Config: `src-tauri/src/config/`
- Backend Core Managers: `src-tauri/src/core/`
- Backend Utilities: `src-tauri/src/utils/`
