# Project: Clash Verge/Mini Post-1.8.5 Changes Audit

## Architecture
- Frontend: React UI built with Vite and MUI, utilizing Tauri API to communicate with Rust backend, tanstack/react-query for caching, and WebSocket for real-time traffic/connection data.
- Backend: Rust Tauri app containing system tray integration, clash config manager, profiles manager, sysproxy integration, and background worker threads.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | Plan Initialization & Setup | Initialize plan.md, progress.md, and briefing.md | None | DONE |
| 2 | Commit Scanning & Focused Listener Audit | Audit commits `fd26ae0a` to `47877a1e`. Audit focus/visibility listeners in `_layout.tsx` and `app-data-provider.tsx`. Audit `resetIdleTimer` fix in `window-provider.tsx`. | None | IN_PROGRESS |
| 3 | Static Checks & Clippy Audit | Run TypeScript compiler checks, ESLint, and cargo clippy. Compile warnings and recommend cleanups. | None | PLANNED |
| 4 | Synthesis & Final Report Generation | Compile findings and proposed diffs into `docs/post_185_changes_audit_report.md`. Ensure git status remains clean. | 2, 3 | PLANNED |

## Interface Contracts
- Front-to-Back: Communication uses Tauri IPC commands and WebSockets.
- Focus and visibility change events trigger react-query refetches and window active state changes.

## Code Layout
- Frontend Page Layout: `src/pages/_layout.tsx`
- Frontend Providers: `src/providers/app-data-provider.tsx`, `src/providers/window-provider.tsx`
- Backend Modules: `src-tauri/src/` (especially commands and core modules)
