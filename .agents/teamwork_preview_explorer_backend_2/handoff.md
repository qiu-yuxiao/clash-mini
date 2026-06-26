# Handoff Report: Backend Audit of `lightweight.rs`

## 1. Observation

- **Modified FFI addition**: A compare of the target file `src-tauri/src/module/lightweight.rs` against git tag `v1.8.9` showed that the only modification is the addition of Windows Working Set memory trimming code (Lines 163–190).
- **Atomic state updates and logging**:
  - In `src-tauri/src/module/lightweight.rs`:
    - Line 37: `static LIGHTWEIGHT_STATE: AtomicU8 = AtomicU8::new(LightweightState::Normal as u8);`
    - Lines 51-54:
      ```rust
      #[inline]
      fn record_state_and_log(state: LightweightState) {
          LIGHTWEIGHT_STATE.store(state.as_u8(), Ordering::Release);
      ```
    - Lines 111-116:
      ```rust
      if !try_transition(LightweightState::Normal, LightweightState::In) { ... }
      record_state_and_log(LightweightState::In);
      ```
    - Lines 197-206:
      ```rust
      if !try_transition(LightweightState::In, LightweightState::Exiting) { ... }
      record_state_and_log(LightweightState::Exiting);
      ```
    - Line 214:
      ```rust
      record_state_and_log(LightweightState::Normal);
      ```
- **Silent start logic and window destruction**:
  - In `src-tauri/src/module/lightweight.rs` (Lines 117-123):
    ```rust
    let result = WindowManager::destroy_main_window();
    if result == WindowOperationResult::Failed {
        logging!(warn, Type::Lightweight, "销毁主窗口失败，回滚轻量模式状态");
        try_transition(LightweightState::In, LightweightState::Normal);
        refresh_lightweight_tray_state().await;
        return false;
    }
    ```
  - In `src-tauri/src/utils/window_manager.rs` (Lines 343-354):
    ```rust
    pub fn destroy_main_window() -> WindowOperationResult {
        if let Some(window) = Self::get_main_window() {
            let _ = window.destroy();
            ...
            return WindowOperationResult::Destroyed;
        }
        WindowOperationResult::Failed
    }
    ```
  - In `src-tauri/src/utils/resolve/mod.rs` (Lines 65, 78, 188-196):
    - `init_window()` is called before `init_auto_lightweight_boot()`.
    - If `is_silent_start` is true, `WindowManager::create_window(false).await` is called (which returns false and does not construct the main window).
- **Debounced Window Showing**:
  - In `src-tauri/src/module/lightweight.rs` (Lines 207-214):
    ```rust
    let result = WindowManager::show_main_window().await;
    if result == WindowOperationResult::Failed {
        logging!(warn, Type::Lightweight, "显示主窗口失败，回滚轻量模式状态");
        try_transition(LightweightState::Exiting, LightweightState::In);
        refresh_lightweight_tray_state().await;
        return false;
    }
    record_state_and_log(LightweightState::Normal);
    ```
  - In `src-tauri/src/utils/window_manager.rs` (Lines 140-145):
    ```rust
    pub async fn show_main_window() -> WindowOperationResult {
        if !should_handle_window_operation() {
            return WindowOperationResult::NoAction;
        }
    ```
- **Async GC Task Spawn**:
  - In `src-tauri/src/module/lightweight.rs` (Lines 128-161):
    ```rust
    AsyncHandler::spawn(|| async {
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        ...
        if let Err(err) = mihomo.close_all_connections().await { ... }
        if let Err(err) = mihomo.clear_all_ws_connections().await { ... }
        ...
    });
    ```
- **Obsolete Timer functions**:
  - In `src-tauri/src/module/lightweight.rs` (Lines 88-99):
    ```rust
    pub async fn enable_auto_light_weight_mode() {
        if let Err(e) = Timer::global().init().await { ... }
        ...
    }
    ```
  - In `src-tauri/src/utils/resolve/mod.rs` (Line 76, 124):
    `Timer::global().init()` is executed during boot.

---

## 2. Logic Chain

1. **State Machine Race Condition**:
   - `try_transition(from, to)` uses safe compare-and-swap (CAS) to change states.
   - `record_state_and_log(state)` runs a raw store operation `LIGHTWEIGHT_STATE.store(state.as_u8(), Release)`.
   - If Thread A (entry) executes and succeeds in transitioning `Normal -> In`, then Thread B (exit) executes and succeeds in transitioning `In -> Exiting`.
   - If Thread A is delayed and calls `record_state_and_log(In)` after Thread B has set the state to `Exiting`, Thread A's raw store will write `In` to the atomic, overwriting `Exiting` back to `In`.
   - This corrupts the state machine, causing subsequent window operations to run in wrong states.
2. **Silent Start Failure**:
   - During silent start, no window is created.
   - On boot, `auto_lightweight_boot()` calls `entry_lightweight_mode()`, which tries to transition to `In` and calls `destroy_main_window()`.
   - Since no window exists, `destroy_main_window()` returns `Failed`.
   - `entry_lightweight_mode()` treats `Failed` as a critical failure, logs a warning, rolls back the state to `Normal`, and returns `false`.
   - Thus, silent start is unable to enter lightweight mode on boot, which is incorrect because the target window-less state is already satisfied.
3. **Out-of-Sync State via Debounce**:
   - In `exit_lightweight_mode()`, `show_main_window()` is called.
   - If the call is rate-limited by debounce logic, `show_main_window()` returns `NoAction`.
   - Because `exit_lightweight_mode()` only checks if the result is `Failed`, it treats `NoAction` as success and transitions to `Normal`.
   - This leaves the window hidden/destroyed, while the state machine transitions to `Normal` (not in lightweight mode), putting them out of sync.
4. **Async Cleanup Race Condition**:
   - `entry_lightweight_mode()` spawns an async task to close connections and subscriptions.
   - If the user exits lightweight mode and opens the window right after entering, the frontend creates new WebSocket connections.
   - The spawned async task makes no state check and might run after the window is restored, calling `clear_all_ws_connections()` and severing the new frontend connections.
5. **Redundancy/Obsoletion**:
   - The timer-based delay is deprecated.
   - `enable_auto_light_weight_mode()` only performs redundant `Timer::global().init()` (already done during setup).

---

## 3. Caveats

- Due to terminal permission timeouts, `cargo check` and `cargo clippy` could not be executed on the user's system. The audit relies entirely on static analysis of the Rust source code.
- Assumptions are made that the custom `tauri-plugin-mihomo` WebSocket connections represent all telemetry feeds from frontend to backend.

---

## 4. Conclusion

The audit identified one critical race condition that corrupts the lightweight mode state machine, three warning-level logic issues (silent start entering failure, rate-limiting desync, and cleanup task race condition), and one minor optimization regarding deprecated timer functions. A detailed audit findings document has been saved to `findings.md`.

---

## 5. Verification Method

- **Files to Inspect**:
  - `src-tauri/src/module/lightweight.rs`
  - `src-tauri/src/utils/window_manager.rs`
  - `src-tauri/src/utils/resolve/mod.rs`
- **Verification Commands (when permission is granted)**:
  `cd src-tauri; cargo check; cargo clippy`
