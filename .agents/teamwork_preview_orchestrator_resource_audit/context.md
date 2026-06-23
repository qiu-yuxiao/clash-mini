# System Resource Optimization Audit Context

## Overview
This file maintains context-specific findings, facts, and links during the Clash Mini resource optimization audit.

## Findings Archive

### Frontend Resource Audit (M1) - Complete
Report Path: `.agents/teamwork_preview_explorer_frontend_resource_audit/handoff.md`
Key Findings:
1. **Ineffective Memoization in ConnectionTable**: TanStack Table `row` wrappers evaluated on every render causing complete breakdown of memoization inside `RowComponent`. (File: `src/components/connection/connection-table.tsx` lines 185-191).
2. **Missing Memoization in LogItem**: Virtualized log lists lack `React.memo` wrapping, triggering regex compilation three times per log render during updates. (File: `src/components/log/log-item.tsx` lines 51-122).
3. **Redundant Window Resize Listeners**: `ProxyItem` instantiates a `useWindowWidth` resize listener per visible node (O(N) growth). (File: `src/components/proxy/use-window-width.ts` lines 13-31).
4. **Redundant Config Queries**: `ProxyItem` triggers `useVerge` config fetches individually. (File: `src/hooks/use-proxy-delay-state.ts` lines 34-35).
5. **Background Polling**: `useSystemState` query runs even if the app is hidden/minimized. (File: `src/hooks/use-system-state.ts` lines 38-53).
6. **Stale Cache / Missing Dependency**: `useFilterSort` misses delay trigger variable `_` in `useMemo` dependency array, preventing UI updates on latency change. (File: `src/components/proxy/use-filter-sort.ts` lines 48-64).
7. **Leaked Timeout Timers**: `UnlockPage` leaves `setTimeout` pending even after `Promise.race` is won by command resolver. (File: `src/pages/unlock.tsx` lines 215-230).

### Backend Concurrency & CPU Audit (M2) - Complete
Report Path: `.agents/teamwork_preview_explorer_backend_concurrency_audit/handoff.md`
Key Findings:
1. **Inefficient/Unbounded Task Spawning in Latency Testing**: `tokio::spawn` is called for every node inside a loop, allocating thousands of tasks up front. (File: `src-tauri/src/module/monitor.rs` lines 307-328).
2. **Global Mutex Blocked During Retries**: `SERVICE_MANAGER` Mutex lock is held across a 5-second backoff await retry loop in `wait_for_service_ipc`. (File: `src-tauri/src/core/service.rs` lines 473-497, 545-560).
3. **Race Condition and Lost Updates in Tray Menu Sync**: Faulty atomic-based lockless state coordination in `sync_tray_proxy_selection` results in lost menu updates. (File: `src-tauri/src/cmd/proxy.rs` lines 16-56).
4. **Busy-Waiting on Atomic Bool During Startup**: Polling `is_resolve_done()` every 200ms in a sleep loop. (File: `src-tauri/src/core/timer.rs` lines 419-427).
5. **High-Frequency 1-Second Sleep/Polling Loop**: Background monitor loop wakes up every 1 second to poll profile status. (File: `src-tauri/src/module/monitor.rs` lines 416-425).

### Backend I/O & Socket Audit (M3) - Complete
Report Path: `.agents/teamwork_preview_explorer_backend_io_audit/handoff.md`
Key Findings:
1. **Redundant Disk Write Operations**: Profile subscription updates, frontend UI state saving, and DNS configuration saving write blindly to disk without content change comparisons. (Files: `src/config/profiles.rs` lines 167/188/289, `src/cmd/proxy.rs` line 134, `src/cmd/clash.rs` line 134).
2. **Socket Reuse & Connection Pooling Limitations**: Re-creates HTTP reqwest client per-request, and disables connection pooling via `.pool_max_idle_per_host(0)`, causing socket churn and descriptor exhaustion risks. (Files: `src/utils/network.rs` lines 64-91, `src/cmd/media_unlock_checker/mod.rs` lines 58-69).
3. **Unbuffered Core Updates & ZIP Backup**: Downloads core binaries and creates ZIP backups using unbuffered file handles. (Files: `src/core/core_updater.rs` lines 109-133, `src/core/backup.rs` lines 240-258).
4. **Tauri IPC Event Emission Bottlenecks**: WebSocket streams and Tauri events (`ws_connections`, `ws_traffic`, `ws_memory`, `ws_logs`) continue diffing and emitting heavy IPC payloads even when the app is blurred, minimized, or hidden in tray. (Files: `crates/tauri-plugin-mihomo/src/mihomo.rs` and `commands.rs`).
