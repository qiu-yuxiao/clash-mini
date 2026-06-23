# Clash Mini System Resource Optimization Audit Report

## 1. Executive Summary

This audit report represents a comprehensive, multi-dimensional evaluation of system resource consumption (CPU, Memory, Threads, Handles/Sockets, and Disk I/O) in the Clash Mini project. By auditing both the React/TypeScript frontend (`src/`) and the Rust/Tauri backend (`src-tauri/` and workspace crates), we have identified critical bottlenecks, redundant tasks, leaked resources, and unbuffered I/O processes.

No code modifications were performed in accordance with the strict code-isolation requirements. The proposed optimizations below target:
- **CPU Idle Wakeups and Main Thread Overhead**: Reducing idle wakeups by introducing event-driven mechanisms in the background monitor, eliminating busy-polling during startup, and utilizing correct React memoization.
- **Memory Consumption and Leak Prevention**: Cleaning up timer handle leaks in the unlock page, optimizing TanStack row rendering memory, and preventing redundant Tauri query subscriptions.
- **File I/O and Socket Lifecycles**: Preventing blind disk writes for profiles, DNS configurations, and UI states. Minimizing socket churn by enabling connection pooling and client caching.
- **Tauri IPC Telemetry Throughput**: Suspending background polling and WebSocket/IPC streams when the application window is minimized or hidden.

---

## 2. Point-by-Point Findings & Optimization Specifications

### A. CPU & React Render Optimization (Frontend)

#### Finding A.1: Broken Memoization in Connection Table Row Component
- **Affected File**: `src/components/connection/connection-table.tsx`
- **Line Range**: 185–191
- **Root Cause**: The component uses a custom memoization comparator `(prev, next) => prev.row === next.row && ...`. However, `row` represents a TanStack Table `Row` wrapper instance that is reconstructed on every render by the parent component. Because reference equality checks fail, the row components re-render every second when data updates, consuming substantial CPU resources.
- **Optimization Strategy**: Compare internal properties (`row.original` and `row.index`) instead of the wrapper reference.
- **Suggested Code Diff**:
```diff
<<<<
  (prev, next) =>
    prev.row === next.row &&
    prev.virtualStart === next.virtualStart &&
    prev.virtualSize === next.virtualSize &&
    prev.onShowDetail === next.onShowDetail &&
    prev.onContextMenu === next.onContextMenu,
)
====
  (prev, next) =>
    prev.row.original === next.row.original &&
    prev.row.index === next.row.index &&
    prev.virtualStart === next.virtualStart &&
    prev.virtualSize === next.virtualSize &&
    prev.onShowDetail === next.onShowDetail &&
    prev.onContextMenu === next.onContextMenu,
)
>>>>
```

#### Finding A.2: Missing Memoization and Hot Loops in Logs rendering
- **Affected File**: `src/components/log/log-item.tsx`
- **Line Range**: 51–122
- **Root Cause**: `LogItem` components are rendered in high volume inside a virtualized list. Since the component is not memoized, any new log entry updates the parent state and triggers a full re-render of all visible items. Within each render, the regex highlight parser is compiled and run multiple times, creating CPU spikes.
- **Optimization Strategy**: Wrap `LogItem` in `React.memo` with a custom shallow comparator.
- **Suggested Code Diff**:
```diff
<<<<
const LogItem = ({ value, searchState }: Props) => {
  const renderHighlightText = (text: string) => {
  ...
  return (
    <Item>
      ...
    </Item>
  )
}

export default LogItem
====
import { memo } from 'react'

const LogItem = memo(({ value, searchState }: Props) => {
  const renderHighlightText = (text: string) => {
  ...
  return (
    <Item>
      ...
    </Item>
  )
}, (prev, next) => {
  return prev.value === next.value && prev.searchState === next.searchState
})

export default LogItem
>>>>
```

#### Finding A.3: O(N) Global Window Resize Listeners
- **Affected File**: `src/components/proxy/use-window-width.ts`
- **Line Range**: 13–31
- **Root Cause**: `ProxyItem` triggers `useWindowWidth` to detect if layout size is minimal. When rendering grids of proxies (often 30+ visible cards), each node registers an independent listener on the global window `resize` event, compounding listener execution overhead.
- **Optimization Strategy**: Hoist `isMinimal` or `windowWidth` to the parent component (`ProxyGroup` or `ProxyList`) and pass it down as a prop, reducing the resize listeners to $O(1)$.

#### Finding A.4: O(N) Redundant Configuration Queries
- **Affected File**: `src/hooks/use-proxy-delay-state.ts`
- **Line Range**: 34–35
- **Root Cause**: `useProxyDelayState` is invoked inside every `ProxyItem` node to fetch the latency timeout parameter using `useVerge()`. This instantiates duplicate TanStack Query cache subscriptions for every proxy.
- **Optimization Strategy**: Fetch the latency timeout config once in the parent container and pass it as a prop to `useProxyDelayState`.

#### Finding A.5: Stale Caches in Proxy List Re-sorting
- **Affected File**: `src/components/proxy/use-filter-sort.ts`
- **Line Range**: 48–64
- **Root Cause**: The `useFilterSort` hook's sorting memoization misses the state variable `_` (triggered by the `bumpRefresh` reducer when delay updates are broadcast). As a result, the sorted list doesn't update dynamically when delay tests complete.
- **Optimization Strategy**: Add `_` to the `useMemo` dependency array to force recalculation upon latency changes.
- **Suggested Code Diff**:
```diff
<<<<
  const compute = useMemo(() => {
    const fp = filterProxies(proxies, groupName, filterText, searchState)
    const sp = sortProxies(fp, groupName, sortType, verge?.default_latency_timeout)
    return sp
  }, [proxies, groupName, filterText, sortType, searchState, verge?.default_latency_timeout])
====
  const compute = useMemo(() => {
    const fp = filterProxies(proxies, groupName, filterText, searchState)
    const sp = sortProxies(fp, groupName, sortType, verge?.default_latency_timeout)
    return sp
  }, [_, proxies, groupName, filterText, sortType, searchState, verge?.default_latency_timeout])
>>>>
```

---

### B. Concurrency, Locks & Thread Management (Backend)

#### Finding B.1: Unbounded Task Spawning in Latency Testing
- **Affected File**: `src-tauri/src/module/monitor.rs`
- **Line Range**: 307–328
- **Root Cause**: During auto-select or latency sweep, `tokio::spawn` is called immediately for all nodes inside a loop. If a profile contains thousands of proxies, thousands of tasks are allocated. Although concurrent requests are throttled using a semaphore permit, the task allocations themselves are unthrottled, leading to memory overhead and scheduler contention.
- **Optimization Strategy**: Use a concurrency-limited stream iterator (`futures::stream::iter` with `.buffer_unordered()`) to lazily generate futures and control spawning.
- **Suggested Code Proposal**:
```rust
use futures::StreamExt;

// Replace unbounded spawning with:
let results: Vec<(String, u32)> = futures::stream::iter(valid_nodes)
    .map(|node_name| {
        let mihomo = mihomo.clone();
        let test_url = test_url.clone();
        async move {
            if let Ok(delay_info) = mihomo.delay_proxy_by_name(&node_name, &test_url, 2000).await {
                if delay_info.delay >= 30 && delay_info.delay < 2000 {
                    return Some((node_name, delay_info.delay));
                }
            }
            None
        }
    })
    .buffer_unordered(MAX_CONCURRENT_DELAY_TESTS) // Throttled concurrency
    .filter_map(|res| async { res })
    .collect()
    .await;
```

#### Finding B.2: Global Mutex Lock Blocked Across Long Network Retries
- **Affected File**: `src-tauri/src/core/service.rs`
- **Line Range**: 473–497 & 545–560
- **Root Cause**: `SERVICE_MANAGER` is a global static `tokio::sync::Mutex`. In `handle_service_status`, the lock is acquired, and inside it, `wait_for_service_ipc` is awaited. This helper function executes a connection retry backoff loop (up to 20 attempts with 250ms delays, totaling 5s). Holding this lock blocking all other calls attempting to fetch service status.
- **Optimization Strategy**: Perform the connection retries locally outside the Mutex context, and only lock the `SERVICE_MANAGER` to commit the final state change.
- **Suggested Code Proposal**:
```rust
async fn wait_for_service_ipc(status: &mut ServiceManager, reason: &str) -> Result<()> {
    status.0 = ServiceStatus::Unavailable(reason.into());
    let config = ServiceManager::config();
    let backoff = ConstantBuilder::default()
        .with_delay(config.retry_delay)
        .with_max_times(config.max_retries);

    // Run connection test without holding the global lock
    let is_connected = (|| async {
        if Path::new(clash_verge_service_ipc::IPC_PATH).exists() {
            clash_verge_service_ipc::connect().await?;
            Ok(())
        } else {
            Err(anyhow!("IPC path not ready"))
        }
    })
    .retry(backoff)
    .await
    .is_ok();

    if is_connected {
        status.0 = ServiceStatus::Ready;
        Ok(())
    } else {
        Err(anyhow::anyhow!("IPC connection timeout"))
    }
}
```

#### Finding B.3: Race Condition and Lost Updates in Tray Coordination Menu Sync
- **Affected File**: `src-tauri/src/cmd/proxy.rs`
- **Line Range**: 16–56
- **Root Cause**: Synchronizing the tray menu uses complex atomic flag states (`TRAY_SYNC_RUNNING` and `TRAY_SYNC_PENDING`). Under high update frequency, exiting update loops clean flags set by newer requests, causing updates to be dropped and the tray menu to fall out of sync with selected proxies.
- **Optimization Strategy**: Replace the lockless atomics with a single tokio bounded channel of capacity 1. Updates will be sent to the channel. If the loop is updating, the channel remains full, which naturally coalesces multiple update notifications into a single execution.
- **Suggested Code Proposal**:
```rust
use tokio::sync::mpsc;
use once_cell::sync::OnceCell;

static TRAY_SYNC_TX: OnceCell<mpsc::Sender<()>> = OnceCell::new();

fn get_tray_sync_tx() -> &'static mpsc::Sender<()> {
    TRAY_SYNC_TX.get_or_init(|| {
        let (tx, mut rx) = mpsc::channel::<()>(1);
        AsyncHandler::spawn(move || async move {
            while rx.recv().await.is_some() {
                if let Err(e) = Tray::global().update_menu().await {
                    logging!(error, Type::Cmd, "Failed to sync tray: {e}");
                }
            }
        });
        tx
    })
}

#[tauri::command]
pub async fn sync_tray_proxy_selection() -> CmdResult<()> {
    let _ = get_tray_sync_tx().try_send(());
    Ok(())
}
```

#### Finding B.4: Busy-Waiting Sleep Loop During Startup
- **Affected File**: `src-tauri/src/core/timer.rs`
- **Line Range**: 419–427
- **Root Cause**: When the app starts up, `wait_until_resolve_done` loops every 200ms checking the atomic boolean `is_resolve_done()`. This causes continuous CPU wakeups during startup.
- **Optimization Strategy**: Implement `tokio::sync::Notify` to block and await resolving status asynchronously.
- **Suggested Code Proposal**:
```rust
use tokio::sync::Notify;
use once_cell::sync::Lazy;

static RESOLVE_NOTIFY: Lazy<Notify> = Lazy::new(Notify::new);

pub fn resolve_done() {
    RESOLVE_DONE.store(true, Ordering::Release);
    RESOLVE_NOTIFY.notify_waiters();
}

// In wait_until_resolve_done:
async fn wait_until_resolve_done(max_wait: Duration) {
    if is_resolve_done() {
        return;
    }
    let _ = timeout(max_wait, RESOLVE_NOTIFY.notified()).await;
}
```

#### Finding B.5: High-Frequency 1-Second Sleep Loop in Monitor
- **Affected File**: `src-tauri/src/module/monitor.rs`
- **Line Range**: 416–425
- **Root Cause**: The background monitor loop wakes up every second to check if the current profile UID has changed. This causes constant CPU wakeups on an idle system.
- **Optimization Strategy**: Modify the background monitor to wake up on profile switch events using a `Notify` trigger or a watch channel.
- **Suggested Code Proposal**:
```rust
static PROFILE_SWITCH_NOTIFY: Lazy<Notify> = Lazy::new(Notify::new);

pub fn notify_profile_changed() {
    PROFILE_SWITCH_NOTIFY.notify_waiters();
}

// Inside the monitor loop:
loop {
    let check_interval = if is_retry_mode { Duration::from_secs(3) } else { Duration::from_secs(15) };
    
    tokio::select! {
        _ = tokio::time::sleep(check_interval) => {
            // Periodic checks
        }
        _ = PROFILE_SWITCH_NOTIFY.notified() => {
            // Immediate check on change
        }
    }
}
```

---

### C. File I/O & Socket Management (Backend)

#### Finding C.1: Blind Profile and Configuration Disk Writes
- **Affected File**: `src-tauri/src/config/profiles.rs` (Lines 167, 188, 289), `src/cmd/proxy.rs` (Line 134), `src/cmd/clash.rs` (Line 134)
- **Root Cause**: Profile downloads, state saves, and DNS config saves write output files blindly to disk without validating if the contents are identical to what is already on disk, incurring redundant I/O operations and disk wear.
- **Optimization Strategy**: Read the file first and write only if there is a modification.
- **Suggested Code Proposal**:
```rust
// Inside profiles.rs append_item / update_item:
let should_write = match fs::read_to_string(&path).await {
    Ok(existing) => existing != file_data,
    Err(_) => true,
};
if should_write {
    fs::write(&path, file_data.as_bytes()).await?;
}
```

#### Finding C.2: Keep-Alive Connection Pool Disabled
- **Affected File**: `src-tauri/src/utils/network.rs` (Lines 64–91) and `src-tauri/src/cmd/media_unlock_checker/mod.rs` (Lines 58–69)
- **Root Cause**: `NetworkManager` explicitly configures `.pool_max_idle_per_host(0)` and constructs a fresh client for every request. Rebuilding reqwest clients disables connection pooling. This triggers high socket churn (creating a new TCP socket and TLS handshake for every call) and risks descriptor exhaustion.
- **Optimization Strategy**: Share a static `Client` instance or state and configure a sensible connection pool limit (e.g. `5` idle connections, with 30s timeouts).
- **Suggested Code Proposal**:
```rust
let mut builder = Client::builder()
    .tls_backend_rustls()
    .redirect(reqwest::redirect::Policy::limited(10))
    .tcp_keepalive(Duration::from_secs(60))
    .pool_max_idle_per_host(5) // Allow socket reuse
    .pool_idle_timeout(Some(Duration::from_secs(30)));
```

#### Finding C.3: Unbuffered Write Operations on Core Downloads & Backups
- **Affected File**: `src-tauri/src/core/core_updater.rs` (Lines 109–133) and `src-tauri/src/core/backup.rs` (Lines 240–258)
- **Root Cause**: Downloading core binary chunks and generating local zip entries write directly to unbuffered file descriptors. For large binaries or highly fragmented ZIP headers, this translates to thousands of small write syscalls.
- **Optimization Strategy**: Wrap target file handles in `BufWriter` for both synchronous (`std::io::BufWriter`) and asynchronous (`tokio::io::BufWriter`) workflows.
- **Suggested Code Proposal**:
```rust
// In backup.rs:
let file = std::fs::File::create(&value)?;
let buffered_file = std::io::BufWriter::new(file);
let mut zip = zip::ZipWriter::new(buffered_file);
```

---

### D. Tauri IPC & Visibility Throttling (Cross-Module)

#### Finding D.1: Unchecked Telemetry Processing on Blurred/Hidden Window
- **Affected File**: `crates/tauri-plugin-mihomo/src/mihomo.rs` (inside WebSocket connection streams) and `src/hooks/use-system-state.ts` (polling timer)
- **Root Cause**: WebSocket telemetry tasks (delivering connection states, logs, and traffic graphs to the frontend) continue to parse connection deltas and emit heavy serialization payloads via Tauri IPC even when the application window is blurred, minimized, or minimized to tray. In parallel, `useSystemState` continues polling the backend every 30s.
- **Optimization Strategy**:
  1. **Frontend**: Use `useVisibility()` hook to pause or clear the `refetchInterval` for `getSystemState` when `pageVisible === false`.
  2. **Backend**: Check if the window is visible using Tauri's window manager before running intensive loop serialization.
- **Suggested Code Proposal (Backend check in telemetry loop)**:
```rust
while let Some(text) = rx.recv().await {
    // Check if main window is active/visible
    let is_visible = app_handle.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if !is_visible {
        // Sleep and throttle updates while in background
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        continue;
    }

    // Otherwise proceed with serializing/emitting telemetry...
}
```

---

## 3. Verification & Compliance Methods

To ensure the suggested improvements are safe and correct, the following verification methods must be performed by workers:

1. **Compilation Validation**:
   - Run `cargo check` in the `src-tauri` directory.
   - Run `npm run build` in the root directory to verify TS configuration changes and prop type adjustments.
2. **Disk I/O Verification**:
   - Save identical configuration states or triggers.
   - Execute `git diff` or monitor system write triggers to ensure that file modification times (`mtime`) on `profiles.yaml` and `proxy_head_state.json` remain unchanged if content is identical.
3. **Socket Lifecycle Audit**:
   - Initiate a series of latency tests or media unlock checks.
   - Run a TCP connection watcher (e.g. `netstat` or resource monitor) to verify that connection states transition to `ESTABLISHED` and reuse existing ports rather than spawning massive numbers of ephemeral ports.
4. **Visibility & Backgrounding Telemetry Check**:
   - Hide the window (minimize/blur).
   - Verify that Tauri event listeners do not fire high-frequency messages (e.g., traffic update payloads) and that network/websocket stream rates fall to zero.
