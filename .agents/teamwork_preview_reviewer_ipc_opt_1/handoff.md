# Handoff Report

## 1. Observation
- **File Checked**: `docs/ipc_optimization_proposal.md`
- **File Checked**: `crates/tauri-plugin-mihomo/src/commands.rs` (lines 239–248, 263–272, 275–286)
- **File Checked**: `crates/tauri-plugin-mihomo/src/mihomo.rs` (lines 250–287, 316–353)
- **File Checked**: `src/hooks/use-visibility.ts`
- **File Checked**: `src/hooks/use-connection-data.ts` (lines 123-125: `if (isWsActive || !isVisible || !enabled) return`)
- **File Checked**: `src/types/global.d.ts` (lines 220–242: `IConnectionsItem` interface)
- **File Checked**: `src-tauri/src/utils/connections_stream.rs` (lines 76-97: `connect_traffic_stream`)
- **File Checked**: `src-tauri/src/utils/window_manager.rs` (lines 62-82: `get_main_window_with_state` checking minimization, visibility, and focus)
- **File Checked**: `crates/tauri-plugin-mihomo/guest-js/index.ts` (lines 461-465, 477-480)

## 2. Logic Chain
- **Tauri Commands & Files Alignment**: By comparing the code references in `docs/ipc_optimization_proposal.md` Section 2 with the actual files `commands.rs`, `mihomo.rs`, `use-visibility.ts`, `use-connection-data.ts`, `connections_stream.rs`, and `index.ts`, we confirm that all file names, line numbers, and API definitions align exactly with the current codebase.
- **Tautology / Logic Bug Verification**: We analyzed the fallback poll logic:
  ```typescript
  const isWsActive = enabled && isVisible
  ...
  if (isWsActive || !isVisible || !enabled) return
  ```
  Since `isWsActive` is $E \land V$, the early return condition is $(E \land V) \lor \neg V \lor \neg E$.
  Applying De Morgan's laws: $\neg V \lor \neg E \equiv \neg (V \land E) \equiv \neg \text{isWsActive}$.
  Thus, the condition simplifies to $\text{isWsActive} \lor \neg \text{isWsActive}$, which is a tautology (always true).
  This confirms the diagnostic in Section 3.3 that the low-frequency REST poll never runs in the current frontend code due to this early return bug.
- **Visibility-based Pause Design Defect**: In Section 6.1, the proposal suggests pausing the streams using the `WindowEvent::Focused(focused)` event.
  - However, in typical desktop usage (e.g. multi-monitor setups or side-by-side tiling), windows often lose focus while remaining fully visible.
  - Suspending update streams when focus is lost would freeze the user interface (such as graphs or connection tables) for a fully visible window, which degrades user experience and looks like a hang.
  - Thus, focus events must not trigger stream suspension. Only minimization and hidden events should pause streams.
- **Differential Update Speed-Freeze Bug**: In `use-connection-data.ts` line 46:
  ```typescript
  if (prev.upload === next.upload && prev.download === next.download) {
      carried.push(prev)
  }
  ```
  If a connection does not transfer data between frames, it reuses the previous connection reference. Because the previous connection contains a non-zero `curUpload` and `curDownload` from the interval in which it last transferred data, this reuse causes the displayed speed to freeze at a non-zero value instead of dropping to 0. In a differential update scheme, any connection *not* updated in the delta payload must be explicitly set to 0 speed, and we must fix this reference reuse bug.

## 3. Caveats
- We could not run live cargo checks or frontend tests because terminal execution permissions were not approved. All code verification was performed via static analysis of the source code.
- macOS and Linux window visibility events (specifically tray hiding behavior) were not experimentally verified; platform-specific quirks with Tauri's `.is_visible()` and `.is_minimized()` on non-Windows platforms should be verified during implementation.

## 4. Conclusion
The global optimization proposal is structurally sound, mathematically correct, and aligns very well with the codebase. However, before implementation, the following changes must be incorporated:
1. **Fix Visibility Pause Logic**: Do not pause streams based on `Focused(false)` events. Pause only on `Minimized` or programmatic `hide()` events.
2. **Provide Delta Merge Logic**: Use the $O(N)$ frontend merge algorithm detailed in `review.md` that resets transfer speeds of idle connections to 0 and fixes the connection speed-freeze bug.
3. **Simplify Backend suspension**: Instead of writing complex backend stream pausing and buffering logic, call the existing `clear_all_ws_connections()` function when windows are minimized/hidden.
4. **Scale $R_{\text{up}}$ in Math Model**: Update the math model to scale the active update rate with connection count for $N=2500$ connections.

## 5. Verification Method
- Inspection of `review.md` in the working directory for detailed review and proposed merge algorithm.
- Comparison of the proposed TS interfaces and Rust models in `docs/ipc_optimization_proposal.md` against their counterparts in `src/types/global.d.ts` and `crates/tauri-plugin-mihomo/src/models.rs`.
