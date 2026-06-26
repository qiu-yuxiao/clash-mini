# Audit Findings: `src-tauri/src/module/lightweight.rs`

This document details the audit of `src-tauri/src/module/lightweight.rs` by comparing its current implementation (specifically the addition of Windows Working Set trimming FFI and related logic) with version 1.8.9, focusing on thread safety, window states, resource leaks, and potential deadlocks.

---

## 1. State Machine Race Condition via Raw Stores (Atomic Clobbering)

- **Severity Level**: `Critical`
- **File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs) (Lines 37, 51–59, 111–116, 197–206, 214)
- **Code Snippet**:
  ```rust
  // Line 37
  static LIGHTWEIGHT_STATE: AtomicU8 = AtomicU8::new(LightweightState::Normal as u8);

  // Lines 51-59
  #[inline]
  fn record_state_and_log(state: LightweightState) {
      LIGHTWEIGHT_STATE.store(state.as_u8(), Ordering::Release);
      match state { ... }
  }
  ```
- **Physical Explanation**:
  `try_transition` uses `compare_exchange` (CAS) to change the state atomically, which is correct and thread-safe. However, after `try_transition` succeeds, the code immediately calls `record_state_and_log(state)`, which runs a raw `store()` operation on the atomic variable `LIGHTWEIGHT_STATE`.
  Because entering and exiting lightweight mode are asynchronous operations, multiple threads can execute concurrently (e.g. user toggles tray icon rapidly or multiple events fire). A race condition exists:
  1. **Thread A (Entry)**: Calls CAS `try_transition(Normal, In)` -> succeeds. The state becomes `In`.
  2. **Thread B (Exit)**: Calls CAS `try_transition(In, Exiting)` -> succeeds. The state becomes `Exiting`.
  3. **Thread A (Entry)**: Belatedly calls `record_state_and_log(In)` -> executes `store(In, Release)`.
  The state is now clobbered back to `In` by Thread A's raw store, overwriting Thread B's update to `Exiting`. This corrupts the state machine, causing subsequent window operations to run in incorrect states, resulting in a completely out-of-sync backend and UI.
- **Proposed Fix**:
  State changes must *only* occur inside the CAS (`try_transition`) operation. Eliminate `record_state_and_log`'s write and refactor into a single `transition_and_log(from, to)` helper:
  ```rust
  fn transition_and_log(from: LightweightState, to: LightweightState) -> bool {
      if LIGHTWEIGHT_STATE.compare_exchange(from.as_u8(), to.as_u8(), Ordering::AcqRel, Ordering::Relaxed).is_ok() {
          match to {
              LightweightState::Normal => logging!(info, Type::Lightweight, "轻量模式已关闭"),
              LightweightState::In => logging!(info, Type::Lightweight, "轻量模式已开启"),
              LightweightState::Exiting => logging!(info, Type::Lightweight, "正在退出轻量模式"),
          }
          true
      } else {
          false
      }
  }
  ```
  Replace all `try_transition` followed by `record_state_and_log` with `transition_and_log`. In `exit_lightweight_mode()`, replace the final `record_state_and_log(LightweightState::Normal)` with `transition_and_log(LightweightState::Exiting, LightweightState::Normal)`.

---

## 2. Silent Start Fails to Enter Lightweight Mode

- **Severity Level**: `Warning`
- **File Path**: 
  - [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs) (Lines 117-123)
  - [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/window_manager.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/window_manager.rs) (Lines 343–355)
- **Code Snippet**:
  ```rust
  // lightweight.rs
  let result = WindowManager::destroy_main_window();
  if result == WindowOperationResult::Failed {
      logging!(warn, Type::Lightweight, "销毁主窗口失败，回滚轻量模式状态");
      try_transition(LightweightState::In, LightweightState::Normal);
      ...
      return false;
  }
  ```
  ```rust
  // window_manager.rs
  pub fn destroy_main_window() -> WindowOperationResult {
      if let Some(window) = Self::get_main_window() {
          let _ = window.destroy();
          ...
          return WindowOperationResult::Destroyed;
      }
      WindowOperationResult::Failed
  }
  ```
- **Physical Explanation**:
  When the application starts silently (`enable_silent_start = true`), the main window is not created during boot (`WindowManager::create_window(false)` is called).
  When `auto_lightweight_boot()` runs, if `is_silent_start` is true and `is_enable_auto` is true, it calls `entry_lightweight_mode()`.
  Inside `entry_lightweight_mode()`, it attempts to transition to `In` and calls `WindowManager::destroy_main_window()`. However, because the window was never created, `get_main_window()` returns `None`, causing `destroy_main_window()` to return `WindowOperationResult::Failed`.
  `entry_lightweight_mode()` intercepts this as a failure, logs a warning, rolls back the state to `Normal`, and returns `false`.
  As a result, silent start will fail to enter lightweight mode on boot, despite the window already being non-existent (which is the target outcome of window destruction).
- **Proposed Fix**:
  Modify `WindowManager::destroy_main_window()` to return `WindowOperationResult::NoAction` if the window is already absent:
  ```rust
  pub fn destroy_main_window() -> WindowOperationResult {
      if let Some(window) = Self::get_main_window() {
          let _ = window.destroy();
          logging!(info, Type::Window, "窗口已摧毁");
          #[cfg(target_os = "macos")]
          {
              handle::Handle::global().set_activation_policy_accessory();
          }
          return WindowOperationResult::Destroyed;
      }
      WindowOperationResult::NoAction
  }
  ```
  In `entry_lightweight_mode()`, treat both `Destroyed` and `NoAction` as success.

---

## 3. Window Showing Debounce Rate Limit Causes Out-of-Sync State

- **Severity Level**: `Warning`
- **File Path**: 
  - [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs) (Lines 207-214)
  - [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/window_manager.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/window_manager.rs) (Lines 140–145)
- **Code Snippet**:
  ```rust
  // lightweight.rs
  let result = WindowManager::show_main_window().await;
  if result == WindowOperationResult::Failed {
      logging!(warn, Type::Lightweight, "显示主窗口失败，回滚轻量模式状态");
      try_transition(LightweightState::Exiting, LightweightState::In);
      ...
      return false;
  }
  record_state_and_log(LightweightState::Normal);
  ```
- **Physical Explanation**:
  If `exit_lightweight_mode()` is invoked, it calls `WindowManager::show_main_window()`.
  If the operation occurs within the debounce window of a previous window toggle, `show_main_window()` will be rate-limited by `should_handle_window_operation()` and return `WindowOperationResult::NoAction`.
  However, `exit_lightweight_mode()` only checks if the result is `WindowOperationResult::Failed`. Since `NoAction` != `Failed`, it treats the rate-limiting as a success and transitions the state to `Normal`.
  As a result, the window remains hidden/destroyed (since `show()` was skipped), but the backend state is marked as `Normal`. This puts the window state and the backend state out of sync.
- **Proposed Fix**:
  In `exit_lightweight_mode()`, ensure that a state transition to `Normal` is only done if the window was actually created or shown. Treat `NoAction` (rate limited) as a reason to not transition the state, or return an error/false:
  ```rust
  let result = WindowManager::show_main_window().await;
  match result {
      WindowOperationResult::Shown | WindowOperationResult::Created => {
          transition_and_log(LightweightState::Exiting, LightweightState::Normal);
      }
      WindowOperationResult::NoAction => {
          logging!(debug, Type::Lightweight, "窗口操作被防抖限制，跳过状态变更");
          try_transition(LightweightState::Exiting, LightweightState::In); // Roll back to In
          return false;
      }
      _ => {
          logging!(warn, Type::Lightweight, "显示主窗口失败，回滚轻量模式状态");
          try_transition(LightweightState::Exiting, LightweightState::In);
          refresh_lightweight_tray_state().await;
          return false;
      }
  }
  ```

---

## 4. Background Connection Cleanup Races with Window Restoration

- **Severity Level**: `Warning`
- **File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs) (Lines 128-161)
- **Code Snippet**:
  ```rust
  AsyncHandler::spawn(|| async {
      let mihomo = crate::core::handle::Handle::mihomo().await.clone();
      
      logging!(info, Type::Lightweight, "[轻量模式] 触发进入时连接清理与数据订阅熔断...");
      
      // 激进清空所有网络连接 (GC)
      if let Err(err) = mihomo.close_all_connections().await { ... }
      
      // 清理所有 WebSocket 连接 (熔断订阅)
      if let Err(err) = mihomo.clear_all_ws_connections().await { ... }
  });
  ```
- **Physical Explanation**:
  When entering lightweight mode, a background asynchronous task is spawned to close all network connections and terminate telemetry WebSocket connections (subscriptions).
  If `exit_lightweight_mode()` is called immediately afterwards (e.g. the user clicks the tray icon right after closing the window), the main window is created, and the frontend starts up, establishing new WebSocket connections.
  However, because the cleanup task runs in the background and is not awaited or gated by the current state, it might execute *after* the new window has already opened. It will then invoke `clear_all_ws_connections()`, which will close all newly established frontend WebSocket connections, breaking UI telemetry (charts, connections list) immediately after the window is restored.
- **Proposed Fix**:
  Check `is_in_lightweight_mode()` inside the background task before performing destructive operations:
  ```rust
  AsyncHandler::spawn(|| async {
      if !is_in_lightweight_mode() {
          logging!(debug, Type::Lightweight, "[轻量模式] 已退出轻量模式，跳过连接清理");
          return;
      }
      let mihomo = crate::core::handle::Handle::mihomo().await.clone();
      
      if !is_in_lightweight_mode() { return; }
      if let Err(err) = mihomo.close_all_connections().await { ... }
      
      if !is_in_lightweight_mode() { return; }
      if let Err(err) = mihomo.clear_all_ws_connections().await { ... }
  });
  ```

---

## 5. Redundancies and Obsolete Timer Functions

- **Severity Level**: `Optimization`
- **File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs) (Lines 85–99)
- **Code Snippet**:
  ```rust
  pub async fn enable_auto_light_weight_mode() {
      if let Err(e) = Timer::global().init().await {
          logging!(error, Type::Lightweight, "Failed to initialize timer: {e}");
          return;
      }
      logging!(info, Type::Lightweight, "开启自动轻量模式（关闭窗口即刻进入）");
  }

  pub fn disable_auto_light_weight_mode() {
      logging!(info, Type::Lightweight, "关闭自动轻量模式");
  }
  ```
- **Physical Explanation**:
  1. The comments note that the "delayed trigger timer" has been deprecated and removed. Lightweight mode is now triggered immediately when the window closes.
  2. `Timer::global().init()` is already called at startup during async setup in `resolve_setup_async()`. Calling it again inside `enable_auto_light_weight_mode()` is redundant and does not configure lightweight mode.
  3. `disable_auto_light_weight_mode()` does nothing except print a log.
- **Proposed Fix**:
  Simplify these functions. If they are kept for compatibility with UI hooks, remove the redundant `Timer::global().init()` call:
  ```rust
  pub async fn enable_auto_light_weight_mode() {
      logging!(info, Type::Lightweight, "开启自动轻量模式（关闭窗口即刻进入）");
  }
  ```
