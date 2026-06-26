# Project: Clash Mini (Clash Verge) Code Audit since v1.8.9

## Architecture
- **Backend (Rust)**: Tauri-based application. Interacts with the Clash core, handles system tray, profiles, configurations, and window states.
- **Frontend (React/TypeScript)**: Web-based UI communicating with the Rust backend via Tauri IPC (`invoke`).

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Discovery | Identify modified files and commit logs from tag `v1.8.9` to `dev` | None | DONE |
| 2 | Rust Backend Audit | Run clippy/check; audit async tasks, thread safety, Tauri windows, and socket/file resource leaks | M1 | DONE |
| 3 | React Frontend Audit | Run eslint; audit StrictMode compatibility, useEffect cleanups, and async race conditions | M1 | DONE |
| 4 | Final Report Synthesis | Synthesize findings into the markdown report in the brain directory | M2, M3 | DONE |

## Interface Contracts
### Rust Backend ↔ React Frontend
- Tauri IPC commands invoked by the React frontend to fetch state, update configs, trigger actions.
- Event listeners for frontend to receive backend events (e.g., sys-log, profile-change).
