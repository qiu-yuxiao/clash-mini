# Backend Audit Analysis Report: Clash Mini (d3831a0ce5ecc6b2c040368570773f2622d0b91b..196e7c01)

This report presents a detailed security and concurrency audit of the Rust backend modifications in Clash Mini between release commit `d3831a0ce5ecc6b2c040368570773f2622d0b91b` and latest HEAD (`196e7c01`), focusing on the transition to the new Mihomo local socket client and background monitor updates.

---

## Executive Summary
The transition from reqwest-based HTTP clients to a custom local IPC socket client (`IpcConnectionPool`) introduced several design flaws that can lead to thread suspension, connection leaks, and complete deadlocks in the backend under ordinary operation (e.g., connection issues or config reloads). 

Six critical vulnerabilities and code quality issues have been identified, along with proposed patch-level recommendations.

---

## Detailed Findings

### Finding 1: Windows Named Pipe Busy Infinite Loop (Thread Suspension Risk)
*   **File Path**: `crates/tauri-plugin-mihomo/src/ipc.rs`
*   **Line Numbers**: 230-245
*   **Root Cause**: In the Windows implementation of `connect_to_socket`, the loop matching named pipe errors handles `ERROR_PIPE_BUSY` by sleeping without decrementing `max_retry_count`.
    ```rust
    let client = loop {
        match ClientOptions::new().open(socket_path) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => (), // No count decrement!
            Err(e) => {
                log::warn!("failed to connect to named pipe: {socket_path}, {e}");
                if max_retry_count == 0 { ... }
                max_retry_count -= 1;
            }
        }
        tokio::time::sleep(RETRY_DELAY).await;
    };
    ```
*   **Impact**: If the named pipe is persistently busy (e.g., Mihomo core hangs, crashes, or is slow to respond during reload), this loop spins infinitely. This halts the executing thread.
*   **Recommendation / Proposed Fix**: Enforce a separate retry count for busy status or decrement the main retry count on `ERROR_PIPE_BUSY`.
    ```rust
    // Proposed Fix:
    let mut max_retry_count = 3;
    let mut busy_retry_count = 5; // Prevent infinite spins on busy
    const RETRY_DELAY: Duration = Duration::from_millis(125);

    let client = loop {
        match ClientOptions::new().open(socket_path) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                if busy_retry_count == 0 {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Named pipe busy timeout exceeded"
                    )));
                }
                busy_retry_count -= 1;
            }
            Err(e) => {
                if max_retry_count == 0 {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Failed to connect to named pipe: {socket_path}, {e}"),
                    )));
                }
                max_retry_count -= 1;
            }
        }
        tokio::time::sleep(RETRY_DELAY).await;
    };
    ```

---

### Finding 2: Socket Connection Timeout Bypass (Concurrency & Resource Hang)
*   **File Path**: `crates/tauri-plugin-mihomo/src/ipc.rs`
*   **Line Numbers**: 556-562
*   **Root Cause**: The client-provided reqwest timeout (`timeout_dur`) only wraps the HTTP request/response transaction (`process`). However, `pool.get_connection(socket_path).await` is called *before* this timeout is applied.
    ```rust
    let pool = IpcConnectionPool::global()?;
    let (conn, _permit) = pool.get_connection(socket_path).await?; // Blocks here indefinitely if connection hangs
    ...
    match timeout_dur {
        Some(d) => timeout(*d, process).await?,
        None => process.await,
    }
    ```
*   **Impact**: Any hang in the connection pool (such as the infinite busy loop in Finding 1 or waiting for semaphore permits) bypasses the HTTP request timeout, suspending the caller task permanently.
*   **Recommendation / Proposed Fix**: Move connection retrieval and handshake inside the timed block.
    ```rust
    // Proposed Fix:
    let process = async {
        let pool = IpcConnectionPool::global()?;
        let (conn, _permit) = pool.get_connection(socket_path).await?;
        // (rest of the handshake and sending logic...)
    };

    match timeout_dur {
        Some(d) => timeout(*d, process).await?,
        None => process.await,
    }
    ```

---

### Finding 3: Empty Connection Pool / Redundant Sockets (Resource Management)
*   **File Path**: `crates/tauri-plugin-mihomo/src/ipc.rs`
*   **Line Numbers**: 467-474, 513-525, 556-562
*   **Root Cause**: `IpcConnectionPool` manages raw socket streams (`WrapStream`). However, `send_by_local_socket` consumes the stream via `http1::handshake(conn.stream)`. Hyper takes ownership of the stream, meaning it is never returned to the pool (there is no `connections.push` call).
*   **Impact**: The "connection pool" is effectively always empty. Every single REST API call spawns a brand new named pipe or unix socket connection, leading to excessive socket overhead and file handle consumption during concurrent health checks (e.g., 32 concurrent tests during auto-select).
*   **Recommendation / Proposed Fix**: Pool active Hyper clients (`http1::SendRequest`) instead of raw streams, or implement connection keep-alive/reuse via a shared reqwest connector.

---

### Finding 4: RwLock Writer Starvation on WebSocket Disconnect (Lock Safety)
*   **File Path**: `crates/tauri-plugin-mihomo/src/commands.rs` & `crates/tauri-plugin-mihomo/src/mihomo.rs`
*   **Line Numbers**: `commands.rs` line 296, `mihomo.rs` lines 368-386
*   **Root Cause**: The Tauri command `ws_disconnect` calls `state.read().await.disconnect(id, force_timeout).await`. This holds a read lock on the global `Mihomo` state across the entire I/O call. Inside `disconnect`, if `force_timeout` is `None`, the socket `writer.send(close_message).await` runs without a timeout.
*   **Impact**: If the socket is frozen, `writer.send` hangs forever, holding the `RwLock` read guard. Any subsequent write operation (e.g., `update_controller` or config reload) will block waiting for a write lock. Once a writer is queued, all subsequent read commands (like `get_version`) will also block to prevent writer starvation, causing a complete backend deadlock.
*   **Recommendation / Proposed Fix**: 
    1. Enforce a default timeout (e.g., 1000ms) on closing messages in `disconnect`.
    2. Clone the `Mihomo` struct and drop the read lock immediately before running the async operations in commands.
    ```rust
    // Proposed Fix in commands.rs:
    #[command]
    pub(crate) async fn ws_disconnect(
        state: State<'_, RwLock<Mihomo>>,
        id: ConnectionId,
        force_timeout: Option<u64>,
    ) -> Result<()> {
        let mihomo = state.read().await.clone(); // Drop read lock immediately after cloning
        mihomo.disconnect(id, force_timeout).await
    }
    ```

---

### Finding 5: Blocked Auto-Select on Profile Switch (Concurrency & Performance)
*   **File Path**: `src-tauri/src/module/monitor.rs`
*   **Line Numbers**: 210-227, 401-413
*   **Root Cause**: When a profile switch occurs, the background monitor immediately tries to run a new auto-select test. If the previous auto-select is still running (testing the old profile's nodes), the new call returns `AUTO_SELECT_BUSY` (since the lock `AUTO_SELECT_RUNNING` is held). The background monitor then enters an infinite loop, retrying every 500ms.
*   **Impact**: High delay on profile switches. The system is blocked from testing the new profile's nodes until all test requests for the old profile complete, even though the old results will be discarded immediately upon completion.
*   **Proposed Recommendation**: Introduce a cancellation token to abort the active delay tasks when the profile changes, allowing the lock to release immediately.

---

### Finding 6: Closed Semaphore Bypass (Concurrency Safety)
*   **File Path**: `src-tauri/src/module/monitor.rs`
*   **Line Numbers**: 300
*   **Root Cause**: In the spawned latency test tasks, the permit is acquired as:
    ```rust
    let _permit = sem.acquire().await.ok();
    ```
    If the semaphore is closed (returning `Err`), `.ok()` evaluates to `None`. The task does not check if the permit is `Some` and proceeds to call `delay_proxy_by_name` anyway.
*   **Impact**: Bypasses the rate-limiting concurrency control if the semaphore is closed.
*   **Proposed Recommendation**: Ensure the task returns early if the permit acquisition fails:
    ```rust
    let Ok(_permit) = sem.acquire().await else { return None; };
    ```
