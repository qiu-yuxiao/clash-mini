# Handoff Report: Rust Backend (src-tauri) I/O, Sockets, and IPC Audit

## 1. Observation

During our read-only audit of the Clash Mini Rust backend (located under `src-tauri` and the local workspace crate `crates/tauri-plugin-mihomo`), we observed several areas of concern regarding redundant disk writes, unbuffered I/O, socket reuse inefficiencies, and Tauri IPC event emission bottlenecks.

### A. Redundant Disk Write Operations
We observed that files are written directly to disk without first checking if the new content is identical to the existing content:
1. **Profile Subscription Updates (`src/config/profiles.rs`)**:
   In `append_item` (lines 167-169 and lines 188-190) and `update_item` (lines 289-291):
   ```rust
   // Line 167
   fs::write(&path, file_data.as_bytes())
       .await
       .with_context(|| format!("failed to write to file \"{file}\""))?;
   ```
   ```rust
   // Line 188
   fs::write(&path, file_data.as_bytes())
       .await
       .with_context(|| format!("failed to write to file \"{file}\""))?;
   ```
   ```rust
   // Line 289
   fs::write(&path, file_data.as_bytes())
       .await
       .with_context(|| format!("failed to write to file \"{file}\""))?;
   ```
   *Quoted from `src/config/profiles.rs`*
   Unlike `save_profile_file` (in `src/cmd/save_profile.rs` lines 52-60) and `save_yaml` (in `src/utils/help.rs` lines 71-74), these update commands write the downloaded profile YAML data directly to disk without checking if the data has changed from what is already present.

2. **Frontend UI State Save (`src/cmd/proxy.rs`)**:
   In `save_proxy_head_state` (lines 131-135):
   ```rust
   #[tauri::command]
   pub async fn save_proxy_head_state(state: serde_json::Value) -> CmdResult<()> {
       let path = dirs::app_home_dir().stringify_err()?.join("proxy_head_state.json");
       let content = serde_json::to_string_pretty(&state).stringify_err()?;
       tokio::fs::write(path, content).await.stringify_err()?;
       Ok(())
   }
   ```
   *Quoted from `src/cmd/proxy.rs`*
   This configuration state (containing frontend group settings, filters, and sorts) is saved blindly via `tokio::fs::write` every time the command is called, regardless of whether the state has changed.

3. **DNS Configuration Save (`src/cmd/clash.rs`)**:
   In `save_dns_config` (lines 134-137):
   ```rust
   let yaml_str = serde_yaml_ng::to_string(&dns_config).stringify_err()?;
   fs::write(&dns_path, yaml_str).await.stringify_err()?;
   ```
   *Quoted from `src/cmd/clash.rs`*
   This performs a blind write of DNS settings without a pre-write content comparison.

---

### B. Unbuffered I/O and Socket / Connection Re-creation Issues
1. **Network Manager Connection Reuse Limitation (`src/utils/network.rs`)**:
   In `build_client` (lines 64-91) and `get_with_tls_mode` (lines 191-212):
   ```rust
   let mut builder = Client::builder()
       .tls_backend_rustls()
       .redirect(reqwest::redirect::Policy::limited(10))
       .tcp_keepalive(Duration::from_secs(60))
       .pool_max_idle_per_host(0)
       .pool_idle_timeout(None);
   ```
   *Quoted from `src/utils/network.rs`*
   First, the HTTP client is built fresh for every single request in `get_with_tls_mode` using `create_request_with_tls_mode`. Second, `.pool_max_idle_per_host(0)` explicitly disables the keep-alive connection pool. This forces a complete DNS lookup, TCP socket allocation, and TLS handshake on *every single request*, leading to high socket churn and risk of socket/file descriptor exhaustion.

2. **Media Unlock Checker Client Allocation (`src/cmd/media_unlock_checker/mod.rs`)**:
   In `check_media_unlock` (lines 58-69):
   ```rust
   pub async fn check_media_unlock() -> Result<Vec<UnlockItem>, String> {
       let client = match Client::builder()
           ...
           .build() { ... };
   ```
   *Quoted from `src/cmd/media_unlock_checker/mod.rs`*
   A fresh `reqwest::Client` is built and allocated on every invocation of the media unlock check command, rather than utilizing a shared client instance in the application state.

3. **Unbuffered Core Updates (`src/core/core_updater.rs`)**:
   In `download_core` (lines 109-133):
   ```rust
   let mut dest_file = tokio::fs::File::create(dest_path).await...
   ...
   tokio::io::AsyncWriteExt::write_all(&mut dest_file, &chunk).await...
   ```
   *Quoted from `src/core/core_updater.rs`*
   Network chunks are written directly to disk. Writing to `tokio::fs::File` without wrapping it in a `BufWriter` causes a system write call for every network packet, which is inefficient.

4. **Unbuffered Backup ZIP Creation (`src/core/backup.rs`)**:
   In `create_backup` (lines 240-258):
   ```rust
   let file = AsyncHandler::spawn_blocking(move || std::fs::File::create(&value)).await??;
   let mut zip = zip::ZipWriter::new(file);
   ```
   *Quoted from `src/core/backup.rs`*
   `std::fs::File` is passed directly to the `ZipWriter` without a `BufWriter`. Since writing ZIP entries generates numerous tiny writes for local headers, this incurs a severe I/O bottleneck.

---

### C. Tauri IPC Event Emission Bottlenecks on Blur / Background Execution
1. **Unthrottled WebSocket Streams (`crates/tauri-plugin-mihomo/src/commands.rs` & `crates/tauri-plugin-mihomo/src/mihomo.rs`)**:
   In `commands.rs` (lines 266-308) and `mihomo.rs` (lines 414-642):
   Commands like `ws_connections`, `ws_traffic`, `ws_memory`, and `ws_logs` set up long-running asynchronous loops that receive frames from the Mihomo/Clash core via WebSockets, parse them, compute deltas (for connections), serialize the results, and emit them via Tauri's IPC channel.
   
   However, there is no visibility check: the backend continues to query Mihomo, perform delta diffing, serialize heavy payloads, and send Tauri IPC events even when the frontend window is minimized, hidden in the system tray, or blurred.

---

## 2. Logic Chain

1. **Redundant writes**: Writing to files (specifically large YAML profiles and configuration JSONs) incurs disk wear and system performance overhead. If the downloaded or input configuration is identical to the current configuration on disk, writing it again serves no purpose. Since the backend normalization methods (`replace("\r\n", "\n")` and byte comparisons) are already proven and used in `save_profile_file` and `save_yaml`, extending this check to profiles' `append_item`/`update_item`, DNS config, and UI state saving will eliminate redundant disk writes.
2. **Connection Pools and Socket Reuse**: `reqwest::Client` maintains an internal connection pool. When `pool_max_idle_per_host(0)` is set, or when the `Client` is built brand new for every individual request (as in `NetworkManager` and `check_media_unlock`), the connection pool is effectively disabled or destroyed immediately. This forces the OS to allocate and tear down TCP sockets on every request, wasting time on handshakes and risking file descriptor exhaustion. Reusing a shared client instance via Tauri state or lazy static, and enabling a connection pool, resolves this.
3. **Buffered Writes**: `tokio::fs::File` and `std::fs::File` are unbuffered. If writes are performed in chunks (like network packet downloads in `core_updater.rs` or header writes in `backup.rs`), they trigger numerous syscalls. Wrapping them in a `BufWriter` aggregates writes in memory, reducing system call overhead.
4. **Background IPC Overhead**: The front-end only renders traffic graph, connections list, logs, and memory stats when the window is visible/focused. If the window is hidden, sending Tauri IPC messages is a waste of CPU cycles. Checking window focus/visibility status in the backend stream loops will prevent these CPU-intensive operations during background execution.

---

## 3. Caveats

* We assume that checking `app_handle.get_webview_window("main").and_then(|w| w.is_visible().ok())` is safe to call across thread boundaries. Tauri handles are designed to be thread-safe (`Send` + `Sync`), so this assumption is robust.
* In some network environments, maintaining a pool of idle connections through proxy servers might trigger timeout warnings on the proxy server's logs, which could explain the historical inclusion of `pool_max_idle_per_host(0)`. However, disabling it completely is an anti-pattern for performance, and a reasonable pool timeout (e.g. 30 seconds) is the standard trade-off.

---

## 4. Conclusion

The audit identifies clear performance improvements:
1. **Disk I/O**: Prevent redundant writes on profiles updates, DNS config, and UI state by implementing a pre-write check. Integrate `BufWriter` on core downloads and backup zip file writes.
2. **Sockets/Networking**: Keep and reuse a single `reqwest::Client` instance instead of rebuilding it per request, and enable connection pooling.
3. **Tauri IPC**: Pause websocket stream processing and IPC event emission when the main window is minimized, hidden, or blurred.

---

## 5. Suggested Optimizations (Code Diffs & Pseudo-code)

### Optimization A: Prevent Redundant Profile and UI State Writes

**Target File:** `src/config/profiles.rs` (Lines 167, 188, 289)
*Proposal:* Read the file and compare before writing:
```rust
// Inside profiles.rs: replace blind writes with:
let should_write = match fs::read_to_string(&path).await {
    Ok(existing) => existing != file_data,
    Err(_) => true,
};
if should_write {
    fs::write(&path, file_data.as_bytes())
        .await
        .with_context(|| format!("failed to write to file \"{file}\""))?;
}
```

**Target File:** `src/cmd/proxy.rs` (Line 134)
*Proposal:* Check if content is different before saving proxy state:
```rust
#[tauri::command]
pub async fn save_proxy_head_state(state: serde_json::Value) -> CmdResult<()> {
    let path = dirs::app_home_dir().stringify_err()?.join("proxy_head_state.json");
    let content = serde_json::to_string_pretty(&state).stringify_err()?;
    
    let should_write = match tokio::fs::read_to_string(&path).await {
        Ok(existing) => existing != content,
        Err(_) => true,
    };
    if should_write {
        tokio::fs::write(path, content).await.stringify_err()?;
    }
    Ok(())
}
```

---

### Optimization B: Reuse Network Sockets & Enable Connection Pooling

**Target File:** `src/utils/network.rs` (Lines 64-91)
*Proposal:* Change `pool_max_idle_per_host(0)` to a pooled value (e.g. `5`) and use a cached/global client rather than constructing one per-request:
```rust
// In build_client, adjust pool:
let mut builder = Client::builder()
    .tls_backend_rustls()
    .redirect(reqwest::redirect::Policy::limited(10))
    .tcp_keepalive(Duration::from_secs(60))
    .pool_max_idle_per_host(5) // Allow idle connections
    .pool_idle_timeout(Some(Duration::from_secs(30)));
```

---

### Optimization C: Buffered I/O for ZIP Backup and Core Updates

**Target File:** `src/core/backup.rs` (Line 240)
*Proposal:* Wrap the standard file in a `BufWriter`:
```rust
let file = AsyncHandler::spawn_blocking(move || std::fs::File::create(&value)).await??;
let buffered_file = std::io::BufWriter::new(file);
let mut zip = zip::ZipWriter::new(buffered_file);
```

**Target File:** `src/core/core_updater.rs` (Lines 109, 130)
*Proposal:* Wrap the tokio file in a `BufWriter`:
```rust
let dest_file = tokio::fs::File::create(dest_path)
    .await
    .context("failed to create temp download file")?;
let mut buffered_dest = tokio::io::BufWriter::new(dest_file);

// Inside chunk loop:
tokio::io::AsyncWriteExt::write_all(&mut buffered_dest, &chunk).await?;

// After loop:
buffered_dest.flush().await?;
```

---

### Optimization D: Throttling / Pausing IPC Events on Window Blur/Hide

**Target File:** `crates/tauri-plugin-mihomo/src/mihomo.rs` (inside stream loops like `ws_connections_checked` / `ws_traffic_checked`)
*Proposal:* Query window visibility before processing/diffing/emitting:
```rust
// Inside the tokio::spawn loop in ws_connections_checked:
while let Some(text) = rx.recv().await {
    // Check if the main window is visible or active
    let is_visible = app_handle.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if !is_visible {
        // Window is hidden/tray. Sleep and skip execution to avoid high CPU and IPC overhead.
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        continue;
    }

    // Otherwise, perform the delta diffing and serialize/emit
    let conn_frame: Connections = serde_json::from_str(&text)?;
    ...
}
```

---

## 6. Verification Method

* Run backend integration tests using the standard test suite:
  ```powershell
  cargo test
  ```
  Ensure all tests under `src/cmd/save_profile.rs` (e.g. `test_save_profile_file_redundant_writes`) continue to pass.
* Verify that file modification times (`mtime`) on profile yaml files, `dns.yaml`, and `proxy_head_state.json` do not change when saving identical configurations.
* Monitor open socket handles/file descriptors during media unlock check commands to ensure connection count returns to baseline.
