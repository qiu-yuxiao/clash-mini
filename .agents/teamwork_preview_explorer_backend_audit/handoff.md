# Handoff Report: Rust Backend Audit (Clash Mini)

## 1. Observation
Between release commit `d3831a0ce5ecc6b2c040368570773f2622d0b91b` and latest HEAD (`196e7c01`), we retrieved the git diff and inspected:
*   `crates/tauri-plugin-mihomo/src/commands.rs` (comment additions only)
*   `crates/tauri-plugin-mihomo/src/mihomo.rs` (comment additions only)
*   `src-tauri/src/module/monitor.rs` (changed from direct HTTP reqwest calls to using the tauri plugin's `Mihomo` struct client)

We also deep-dived into `crates/tauri-plugin-mihomo/src/ipc.rs` where the local socket client and connection pooling are implemented. 

Verbatim code snippets observed:
1. **Windows Named Pipe Infinite loop**: In `crates/tauri-plugin-mihomo/src/ipc.rs` (lines 230-245):
   ```rust
   let client = loop {
       match ClientOptions::new().open(socket_path) {
           Ok(client) => break client,
           Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => (),
           Err(e) => { ... }
       }
       tokio::time::sleep(RETRY_DELAY).await;
   };
   ```
2. **Bypassed Connection Timeout**: In `crates/tauri-plugin-mihomo/src/ipc.rs` (lines 556-562):
   ```rust
   let pool = IpcConnectionPool::global()?;
   let (conn, _permit) = pool.get_connection(socket_path).await?;
   ...
   match timeout_dur {
       Some(d) => timeout(*d, process).await?,
       None => process.await,
   }
   ```
3. **Empty Connection Pool**: In `crates/tauri-plugin-mihomo/src/ipc.rs` (lines 456-464):
   Only one call to `self.connections.push(conn)` exists in the entire codebase, and it resides inside `cleanup_idle_connections` to put back non-timeout connections. No push occurs after connection usage, meaning `get_connection` pops the connection, and then `send_by_local_socket` consumes the stream via `http1::handshake` without ever pushing it back.
4. **RwLock held across I/O**: In `crates/tauri-plugin-mihomo/src/commands.rs` (line 296) and `mihomo.rs` (lines 368-386):
   ```rust
   #[command]
   pub(crate) async fn ws_disconnect(
       state: State<'_, RwLock<Mihomo>>,
       id: ConnectionId,
       force_timeout: Option<u64>,
   ) -> Result<()> {
       state.read().await.disconnect(id, force_timeout).await
   }
   ```
   If `force_timeout` is `None` or `0`, `disconnect` awaits `writer.send(close_message).await` with no timeout.

---

## 2. Logic Chain
1. **Observation 1** shows that `ERROR_PIPE_BUSY` results in a sleep and loop iteration without decrementing `max_retry_count`. Therefore, if the named pipe remains busy permanently (e.g., core hangs), the loop will run infinitely, causing a **thread suspension** bug.
2. **Observation 2** shows that `pool.get_connection` is called before the `timeout` wrapper is applied. If a connection attempt blocks infinitely (due to the loop in Observation 1), the reqwest request timeout is bypassed, causing the caller task (e.g., background monitor) to hang indefinitely.
3. **Observation 3** shows that connections are never pushed back after use, meaning the connection pool is always empty and every request creates a new socket connection. This degrades performance during auto-select where 32 concurrent requests are fired.
4. **Observation 4** shows that the `ws_disconnect` command holds the read lock of `RwLock<Mihomo>` while calling `disconnect`. If `disconnect` blocks indefinitely (due to dead socket / no timeout), the read lock is held indefinitely. Any writer (e.g., `update_controller`) will block waiting for a write lock, and subsequent readers will also block, creating a **plugin-wide deadlock**.

---

## 3. Caveats
No live runtime testing was performed as this is a read-only investigation (in line with the team guidelines and network constraints). Assumptions are based strictly on static code analysis of the rust files in the Clash Mini workspace.

---

## 4. Conclusion
The custom local socket connection pool and background monitor integration contain serious logic flaws. These flaws can lead to:
1. Infinite connection loops on Windows (named pipe busy state).
2. Uncontrolled socket timeouts that bypass the client configuration.
3. Complete plugin deadlocks due to `RwLock` read-guard leakage across long I/O operations without timeouts.

---

## 5. Verification Method
To verify these issues:
1. Inspect `crates/tauri-plugin-mihomo/src/ipc.rs` at line 230 and verify the loop structure on Windows.
2. Inspect `crates/tauri-plugin-mihomo/src/commands.rs` at line 296 and confirm `state.read().await` guard is held across the `.await` of `disconnect`.
3. Verify that `cargo test` command in `crates/tauri-plugin-mihomo/` compiles cleanly after implementing the proposed patches.
