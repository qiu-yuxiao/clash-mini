# Rust/Tauri Backend Code Audit Handoff Report

## 1. Observation

A comprehensive static analysis of the Rust/Tauri backend (located in `src-tauri/`) was performed. Five main categories of observations were recorded, as detailed below.

---

### Finding 1: Concurrency Risk - `RwLockReadGuard` Held Across Async Yield Points
* **File Path**: `src/utils/connections_stream.rs` (and other files using `Handle::mihomo()`)
* **Line Range**: 80-90 (in `connections_stream.rs`), also present in `core/manager/config.rs:144`, `feat/clash.rs:77`, `feat/window.rs:79`, and `utils/connections_stream.rs:150`
* **Problem Description**:
  The function `handle::Handle::mihomo()` returns a `RwLockReadGuard<'static, Mihomo>`. The code chains async method calls (such as `.ws_traffic().await` or `.patch_base_config().await`) directly after retrieving the guard. 
  In Rust, temporary values created in an expression chain are dropped at the very end of the statement (the semicolon). Because these chained methods are asynchronous, the temporary `RwLockReadGuard` is held across the `.await` boundary. If another task (e.g. core switching or re-configuration) requests a write lock (`.write().await`) on the `Mihomo` instance, it will be blocked. In tokio's `RwLock` implementation, writer priority/starvation prevention can cause subsequent reader requests to also block, potentially leading to application deadlock or severe latency spikes.
* **Code Snippet**:
  ```rust
  // src/utils/connections_stream.rs:80-90
  let connection_id = handle::Handle::mihomo()
      .await
      .ws_traffic({
          let message_tx = message_tx.clone();
          move |message| {
              if let Some(event) = parse_traffic_event(&message) {
                  try_send_internal_event(&message_tx, event);
              }
          }
      })
      .await?;
  ```
* **Proposed Refactoring or Fix**:
  Retrieve the future first, drop the read guard explicitly, and then await the future:
  ```rust
  let ws_future = {
      let mihomo = handle::Handle::mihomo().await;
      mihomo.ws_traffic({
          let message_tx = message_tx.clone();
          move |message| {
              if let Some(event) = parse_traffic_event(&message) {
                  try_send_internal_event(&message_tx, event);
              }
          }
      })
  }; // Read guard is dropped here
  let connection_id = ws_future.await?;
  ```

---

### Finding 2: Performance Risk - Synchronous File I/O on Tokio Worker Threads
* **File Path**: `src/module/monitor.rs` and `src/core/updater.rs`
* **Line Range**: `src/module/monitor.rs:276`, `src/core/updater.rs:475` (inside `check_and_download`)
* **Problem Description**:
  `trigger_backend_auto_select` (in `monitor.rs`) and `check_and_download` (in `updater.rs`) are `async` functions that execute on the Tokio worker thread pool.
  - In `monitor.rs`, `get_active_filter_config` is called synchronously and uses `std::fs::read_to_string` to read `proxy_head_state.json`.
  - In `updater.rs`, `Self::write_cache(&bytes, &version)` uses `std::fs::write` to write the update installer binary (which can be tens of megabytes) to disk.
  Executing blocking synchronous I/O operations directly on Tokio worker threads prevents those threads from polling other active futures, causing latency spikes, UI freezing, or proxy connection glitches.
* **Code Snippet**:
  ```rust
  // src/module/monitor.rs:276
  let filter_config = get_active_filter_config(profile_uid);
  
  // src/core/updater.rs:475-477
  if let Err(e) = Self::write_cache(&bytes, &version) {
      logging!(warn, Type::System, "Silent updater: failed to write cache: {e}");
  }
  ```
* **Proposed Refactoring or Fix**:
  For `monitor.rs`, convert `get_active_filter_config` to use `tokio::fs::read_to_string`:
  ```rust
  pub async fn get_active_filter_config(profile_uid: &str) -> FilterConfig {
      // ...
      let content = match tokio::fs::read_to_string(path).await {
          Ok(c) => c,
          Err(_) => return FilterConfig::default(),
      };
      // ...
  }
  ```
  For `updater.rs`, perform the write cache operation in `tokio::task::spawn_blocking`:
  ```rust
  let version_clone = version.clone();
  let bytes_clone = bytes.clone();
  if let Err(e) = tokio::task::spawn_blocking(move || {
      Self::write_cache(&bytes_clone, &version_clone)
  }).await.unwrap() {
      logging!(warn, Type::System, "Silent updater: failed to write cache: {e}");
  }
  ```

---

### Finding 3: Performance Risk - Synchronous Process Scanning Blocks Tokio Workers
* **File Path**: `src/core/manager/state.rs`
* **Line Range**: 143-164
* **Problem Description**:
  `CoreManager::kill_all_mini_cores` utilizes `sysinfo::System::new_all()` to scan the entire OS process list. This is a CPU and OS-bound blocking synchronous call that can take hundreds of milliseconds.
  This method is invoked directly in `stop_core_by_sidecar()` (called in `stop_core()` async context) and during app shutdown in `feat/window.rs`. It runs on the Tokio executor thread and blocks it.
* **Code Snippet**:
  ```rust
  // src/core/manager/state.rs:143-149
  pub fn kill_all_mini_cores() {
      logging!(
          info,
          Type::Core,
          "Scanning and killing leftover mini-mihomo processes..."
      );
      let system = sysinfo::System::new_all();
  ```
* **Proposed Refactoring or Fix**:
  Wrap the process scanning and killing inside `tokio::task::spawn_blocking` or `AsyncHandler::spawn_blocking`:
  ```rust
  pub async fn kill_all_mini_cores_async() {
      tokio::task::spawn_blocking(|| {
          Self::kill_all_mini_cores();
      }).await.unwrap_or_default();
  }
  ```

---

### Finding 4: Stability/Hang Risk - Synchronous `block_on` call inside Tauri Setup Hook
* **File Path**: `src/lib.rs`
* **Line Range**: 256-260
* **Problem Description**:
  In the Tauri `.setup()` hook, `tauri::async_runtime::block_on` is used to run `try_install_on_startup(&app_handle)`. 
  Because `.setup()` executes on the main UI/event loop thread, blocking it will freeze the app startup sequence. Since `try_install_on_startup` contains a 30-second timeout for the installer, if the install process hangs or takes time, the app will appear completely frozen. Furthermore, it creates a splash window (`show_update_splash`) before checking/installing, but because the event loop hasn't started running, that window cannot render its HTML/CSS contents, resulting in a blank/white window.
* **Code Snippet**:
  ```rust
  // src/lib.rs:256-260
  let is_updating = tauri::async_runtime::block_on(async {
      crate::core::updater::SilentUpdater::global()
          .try_install_on_startup(&app_handle)
          .await
  });
  ```
* **Proposed Refactoring or Fix**:
  Do not block the main thread during startup. Let the setup hook finish, and run the startup update check in an asynchronous spawned task. Alternatively, perform update installation in a separate small launcher executable before launching the main Tauri application.

---

### Finding 5: Portability Risk - Unix Timestamp Cast to `usize`
* **File Path**: `src/config/prfitem.rs`
* **Line Range**: 249, 341, 432, 623, 642, 661, 676, 691, 706
* **Problem Description**:
  The code calls `chrono::Local::now().timestamp() as usize` to store the profile update timestamp.
  `timestamp()` returns an `i64`. Casting this directly to `usize` is dangerous on 32-bit targets (where `usize` is 32-bit), as it will truncate the timestamp (causing a Year 2038 overflow bug or general corruption). Unix timestamps should always be stored in `i64` or `u64`.
* **Code Snippet**:
  ```rust
  // src/config/prfitem.rs:249
  updated: Some(chrono::Local::now().timestamp() as usize),
  ```
* **Proposed Refactoring or Fix**:
  Change the type of the `updated` field in `PrfItem` and `IProfiles` to `Option<i64>` (or `u64`), and avoid the `as usize` cast:
  ```rust
  updated: Some(chrono::Local::now().timestamp()),
  ```

---

## 2. Logic Chain

1. **Observation 1** demonstrates that `handle::Handle::mihomo()` returns `RwLockReadGuard<'static, Mihomo>`, which is held across `.await` yield points in `connections_stream.rs` and other files.
2. Holding synchronous lock guards (like tokio's `RwLockReadGuard`) across async boundaries yields the thread while holding the lock. If another task (e.g. config reloading or service re-init) requires a write lock, it will block. This blocks any subsequent read requests due to tokio's reader/writer prioritization, resulting in **deadlock or severe UI latency**.
3. **Observation 2** shows synchronous filesystem calls (`std::fs::read_to_string` and `std::fs::write`) executed directly within async functions `trigger_backend_auto_select` and `check_and_download` on the tokio runtime thread.
4. Blocking thread pool worker threads with disk I/O prevents tokio from scheduling other active futures on those threads, leading to **performance degradation and UI stuttering**.
5. **Observation 3** shows `sysinfo::System::new_all()` executed synchronously on the tokio thread.
6. Traversing the system process tree is a heavy, blocking syscall operation that can take hundreds of milliseconds, blocking the scheduler thread and causing **latency issues**.
7. **Observation 4** shows `block_on` blocking the main thread inside the Tauri `setup` hook.
8. Blocking the main thread before starting the Tauri event loop prevents window rendering, creates **blank splash windows**, and can freeze native OS dialogs.
9. **Observation 5** shows Unix timestamps cast from `i64` to `usize` in `prfitem.rs`.
10. Casting to `usize` will cause data truncation on 32-bit platforms, causing the **Year 2038 problem** or timestamp corruption.

---

## 3. Caveats

- **No compilation / runtime testing**: The codebase was audited purely via static analysis without building or running the project, as dictated by the non-modifying code audit constraints. We assume the code compiles successfully in its current state.
- **Third-party library internals**: For `tauri-plugin-mihomo`, `sysproxy-rs`, and `boa_engine`, we reviewed the API usage but did not audit the full internal source code of those external crates.
- **AppKit/Win32 FFI**: The OS-specific integrations (like Objective-C objc2 wrappers or Win32 COM APIs) were checked for safety and scope, but we assume the underlying system library calls are stable.

---

## 4. Conclusion

The ClashVerge Rust/Tauri backend shows high code quality overall, with extensive use of safe-math wrappers (`saturating_mul`, `saturating_sub`), very few unhandled panic points (`.unwrap()` or `.expect()` are absent in production flows), and consistent error conversion conventions (`CmdResult` / `StringifyErr`). 

However, several latency, deadlock, and platform compatibility issues exist in relation to lock holding patterns, synchronous I/O blocking tokio threads, main thread blocking during startup, and timestamp casting. Implementing the proposed refactoring steps will significantly improve the app's performance, stability, and longevity.

### Summary Checklist

- [ ] Refactor `Handle::mihomo()` usage to prevent holding `RwLockReadGuard` across `.await` boundaries (preventing potential deadlocks).
- [ ] Migrate synchronous disk operations (`std::fs`) in async functions (`monitor.rs` and `updater.rs`) to `tokio::fs` or `spawn_blocking` (preventing tokio thread blocking).
- [ ] Wrap the synchronous `sysinfo` process scanning in `spawn_blocking` in `state.rs` (preventing tokio thread blocking).
- [ ] Refactor the silent update process check on startup to run asynchronously instead of blocking the main thread in the `.setup()` hook.
- [ ] Change `usize` casts for Unix timestamps in `prfitem.rs` to `i64` or `u64` (preventing 32-bit integer truncation).

---

## 5. Verification Method

To independently verify these findings:
1. **Mutex Lock Lifetimes**: Inspect `src/utils/connections_stream.rs` lines 80-90. Verify that `handle::Handle::mihomo().await` is called within a statement containing an outer `.await`.
2. **Synchronous File I/O**: Inspect `src/module/monitor.rs` line 276. Note that `get_active_filter_config` performs synchronous file reading while being called directly in an async function. Inspect `src/core/updater.rs` line 475 to confirm the synchronous `write_cache` call in `check_and_download`.
3. **Synchronous Process Scanning**: Inspect `src/core/manager/state.rs` lines 143-164. Note that `kill_all_mini_cores` calls `sysinfo::System::new_all()` synchronously and is executed on tokio worker threads.
4. **Startup block_on**: Inspect `src/lib.rs` line 256. Verify that `block_on` is called in the `setup` hook.
5. **Timestamp Cast**: Inspect `src/config/prfitem.rs` line 249 (and others). Observe `chrono::Local::now().timestamp() as usize`.
