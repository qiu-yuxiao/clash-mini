# Project: Clash Verge / Clash Mini IPC Optimization

## Architecture
- **Frontend**: React-based Single Page Application (TypeScript) using Tauri's `@tauri-apps/api/event` or custom hooks to listen to IPC events.
- **Backend**: Rust (using Tauri event system to emit/send events). E.g., connection logs, traffic events, system logs.
- **Core issue**: High frequency and high volume of Tauri IPC events (e.g. `/traffic`, `/connections`, `/logs`) in Clash Mini, resulting in 38MB+ payload in 15 seconds. Clash Verge uses ~4.4MB.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | IPC Audit & Mapping | Search and identify high-frequency and high-volume Tauri IPC events. Map backend emitters and frontend listeners. | None | DONE (c48ea6ce, e075e2b9, bd64fc3e) |
| 2 | Optimization Protocol Design | Design differential updates, throttling, visibility pause, and backward-compatible schemas. | Milestone 1 | DONE (ac22f13e) |
| 3 | Final Design Proposal Document | Write final, complete proposal to `docs/ipc_optimization_proposal.md`. | Milestone 2 | DONE (ac22f13e) |

## Interface Contracts
- **Tauri IPC Events**: Emitters from Rust side (`tauri-plugin-mihomo` or `src-tauri`), listeners on React side (`src/hooks` or `src/components`).

## Code Layout
- `src-tauri/` and `crates/` - Rust backend emitter code.
- `src/` - TS/JS frontend listener code and state hooks.
