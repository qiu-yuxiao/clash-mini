# Clash Mini (v2.6.6) Full-Codebase Code Review & Window Operation Audit Report

**Project**: Clash Mini (`ClashVerge / clash-mini`)  
**Version**: v2.6.6  
**Audit Date**: 2026-07-21  
**Orchestration Pattern**: Project Pattern (Orchestrator + 3 Explorers + Compilation Worker + Forensic Auditor)  
**Integrity Verdict**: **CLEAN** (Verified by Forensic Auditor — 0 unauthorized code modifications, 100% verified citations)

---

## 1. Executive Summary & Audit Overview

A comprehensive, full-codebase code review and window operation performance audit was conducted for Clash Mini (v2.6.6). The mission encompassed an exhaustive analysis of the Rust backend (`src-tauri/src`), the React/TypeScript frontend (`src/`), a deep-dive performance and correctness audit of the custom pointer-based window resizing mechanism (`src/components/layout/resize-handles.tsx`, `src/utils/resize-geometry.ts`) near minimum bounds (`MIN_WIDTH=285`, `MIN_HEIGHT=135`), and strict compliance verification against `clash_mini_agreements.md` and `clash_mini_pitfalls.md`.

### Summary of Audit Outcomes:
1. **Window Resize Subsystem (M1)**: Identified 1 Critical Agreement Violation (missing 200ms cool-down reset), 2 Major Performance Bottlenecks (dual Win32 `SetWindowPos` calls per frame & minimum bound IPC flooding), 1 Race Condition on mouse release, and 2 Minor UX/DPI artifacts.
2. **Rust Backend (M2)**: Identified 1 Critical Agreement Violation (retained `SetProcessWorkingSetSize` memory trimming in `lightweight.rs`), 1 Major Worker Thread Blocking bug (`MessageBoxW` inside Tokio task), 1 Race Condition on lightweight mode exit, and verified 100% compliance for single-instance port isolation (33335/33336), `mini-mihomo` process prefixing, TUN service isolation, and lightweight mode event guards (`is_in_lightweight_mode()`).
3. **React/TS Frontend (M3)**: Confirmed robust hook safety, 1000ms traffic data throttling, 120ms node delay update batching, custom `React.memo` prop comparisons in `ProxyItem`, virtualized list scrolling with `@tanstack/react-virtual`, `<BaseErrorBoundary>` / `<ErrorBoundary>` isolation, and 6 physical 3D skin styles. Documented 7 minor optimization items.
4. **Forensic Integrity Verification (M4)**: Forensic Auditor performed a repository-wide integrity check, confirming **CLEAN** status. All audited source files in `src/` and `src-tauri/src/` remain in pristine original state with zero local code changes (`git status` clean).

---

## 2. Window Resize Subsystem & Minimum Bounds (285×135) Audit (M1)

### 2.1 Architecture Overview
Clash Mini replaces Tauri's native `startResizeDragging` with a custom pointer-captured resize mechanism (`setPointerCapture`, `pointermove`, RAF throttling) to avoid catastrophic Windows modal loop deadlocks (`WM_SIZE` -> COM `ExecuteScript` locks).

### 2.2 Identified Failure Modes & Defect Catalog

#### [FM-01] [CRITICAL / AGREEMENT VIOLATION] Missing 200ms Cool-down Reset
- **File & Line**: [`src/components/layout/resize-handles.tsx:282`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L282)
- **Violation**: `clash_mini_agreements.md` §2.2 / v2.5.4 mandates a **200ms cool-down reset** for `window.__isResizing` upon pointer release. In v2.6.6, `setWindowResizing(false)` is executed **synchronously on `pointerup`**.
- **Root Cause & Impact**: Downstream subscribers (`WindowProvider` and `useVisibility`) immediately read `isWindowResizing() === false` while the final `setSize`/`setPosition` IPC is still finishing. This triggers immediate IPC queries (`isMinimized`, `isVisible`) and heavy React state updates (`setIsMinimalWidth`, `setIsMiniStatus`) upon mouse release, causing noticeable UI freeze spikes on pointer release.
- **Remediation**:
  ```tsx
  // In handlePointerUp (resize-handles.tsx):
  if (cooldownTimerRef.current !== null) clearTimeout(cooldownTimerRef.current)
  cooldownTimerRef.current = setTimeout(() => {
    setWindowResizing(false)
    cooldownTimerRef.current = null
  }, 200)
  ```

#### [FM-02] [MAJOR / PERFORMANCE] Dual Win32 IPC Overhead per Frame (`setSize` + `setPosition`)
- **File & Line**: [`src/components/layout/resize-handles.tsx:162-179`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L162-L179)
- **Root Cause**: Resizing top/left edges ('n', 'w', 'nw', 'ne', 'sw') triggers `Promise.all([ currentWindow.setSize(...), currentWindow.setPosition(...) ])`.
- **Impact**: Issues two separate async IPC calls per animation frame, which translates to two distinct Win32 `SetWindowPos` API calls per frame. Windows DWM receives dual transformation calls, causing visual edge flickering and frame drops when dragging near minimum bounds.
- **Remediation**: Group target size and position into a single atomic backend IPC command (`set_window_geometry`) or defer position updates until size completion.

#### [FM-03] [MAJOR / PERFORMANCE] IPC Flooding near Minimum Bounds (285×135)
- **File & Line**: [`src/components/layout/resize-handles.tsx:157-160`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L157-L160)
- **Root Cause**: `applyPending` checks `needsSize = w !== null || h !== null` without comparing target dimensions against the *last applied* geometry in `session`.
- **Impact**: While dragging against minimum bounds (`285×135`), high-frequency `pointermove` events (120Hz–240Hz) continue launching redundant `setSize(285, 135)` IPC messages every frame, clogging the Tauri IPC channel.
- **Remediation**:
  ```tsx
  // Cache last applied geometry in session:
  const needsSize = (w !== null && w !== session.lastAppliedW) || (h !== null && h !== session.lastAppliedH)
  const needsPos = (x !== null && x !== session.lastAppliedX) || (y !== null && y !== session.lastAppliedY)
  if (!needsSize && !needsPos) return
  ```

#### [FM-04] [MAJOR / RACE CONDITION] Concurrent `applyPending` Execution on Mouse Release
- **File & Line**: [`src/components/layout/resize-handles.tsx:289`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L289)
- **Root Cause**: `handlePointerUp` cancels pending rAF and directly calls `void applyPending(session)` without checking `if (session.inFlight)`.
- **Impact**: If a previous rAF IPC call is currently awaiting backend response, `handlePointerUp` launches a second concurrent `applyPending` invocation, leading to parallel in-flight IPC dispatches and potential out-of-order execution.
- **Remediation**: Wrap `applyPending` call in `handlePointerUp` with `if (!session.inFlight)`.

#### [FM-05] [MINOR / UX] Drag Reversal Hysteresis / Dead-Zone
- **File & Line**: [`src/utils/resize-geometry.ts:34-51`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/resize-geometry.ts#L34-L51)
- **Root Cause**: `computeResizeGeometry` accumulates unclamped physical deltas `dx`/`dy` when dragging past minimum bounds.
- **Impact**: When reversing mouse direction, the pointer must travel back across the entire accumulated delta before the window edge responds, creating a perceived "dead zone".
- **Remediation**: Clamp `dx`/`dy` accumulator limits based on `MIN_WIDTH` and `MAX_WIDTH` boundaries.

#### [FM-06] [MINOR / DPI] High-DPI Subpixel Positional Jitter
- **File & Line**: [`src/utils/resize-geometry.ts:70`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/resize-geometry.ts#L70)
- **Root Cause**: Converting physical position back to logical coordinates (`newX / sf`) yields floating-point numbers under fractional display scaling (e.g. 1.25×, 1.5×).
- **Impact**: Tauri's Rust backend rounding converts logical floats back to physical integer pixels, occasionally introducing 1-pixel positional crawling along top/left edges.

---

## 3. Rust Backend Exhaustive Code Review (`src-tauri/src`) (M2)

### 3.1 Architecture & Verification Highlights
- **Single-Instance Port Isolation**: Standardized ports `33335` (Release) and `33336` (Dev) in [`src-tauri/src/constants.rs:16-18`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/constants.rs#L16-L18). Second instance notifies existing instance via HTTP GET and exits silently with code `0` (`src-tauri/src/utils/server.rs:78-81`).
- **Sidecar Namespace Physical Isolation**: Sidecar binary name `mini-mihomo` / `mini-mihomo-alpha` isolates process lifecycle management completely from official Clash Verge (`verge-mihomo`).
- **TUN Service Mode Rules**: TUN service mode requested only when `enable_tun_mode` is `true`. Non-admin mode fallback automatically switches to system proxy mode if service is missing or fails.
- **Lightweight Mode Event Guards**: Double intercept defense in [`src-tauri/src/core/notification.rs:49`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/notification.rs#L49) and [`src-tauri/src/module/monitor.rs:453`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L453) checks `is_in_lightweight_mode()` before calling `window.emit`, preventing WebView2 COM Access Violation crashes.

### 3.2 Identified Defect Catalog

#### [FM-07] [CRITICAL / AGREEMENT VIOLATION] `SetProcessWorkingSetSize` Memory Trimming Retained
- **File & Line**: [`src-tauri/src/module/lightweight.rs:190-202`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L190-L202)
- **Violation**: Agreement §1.2 explicitly mandates complete removal of `SetProcessWorkingSetSize(..., usize::MAX, usize::MAX)` trimming calls.
- **Root Cause & Impact**: Forcing working set pages to pagefile causes severe page faults when restoring window focus, causing WebView2 deadlocks and application freezes.
- **Remediation**: Remove `trim_working_set()` function call completely.

#### [FM-08] [MAJOR / THREADING BUG] Synchronous `MessageBoxW` Dialog inside Tokio Async Worker
- **File & Line**: [`src-tauri/src/module/monitor.rs:994`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L994)
- **Root Cause**: `start_background_monitor` calls `crate::show_error_dialog(...)` directly inside an async task. On Windows ([`src-tauri/src/lib.rs:198`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L198)), `show_error_dialog` executes Win32 `MessageBoxW` synchronously.
- **Impact**: Synchronously blocks the Tokio runtime worker thread awaiting user dialog dismissal, stalling any other async tasks assigned to the same worker thread.
- **Remediation**: Wrap dialog call in `tokio::task::spawn_blocking(move || crate::show_error_dialog(...))`.

#### [FM-09] [MAJOR / RACE CONDITION] Missing Task Cancellation in `exit_lightweight_mode()`
- **File & Line**: [`src-tauri/src/module/lightweight.rs:209-254`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L209-L254)
- **Root Cause**: `entry_lightweight_mode()` spawns `LIGHTWEIGHT_CLEANUP_HANDLE` (which sleeps 2 seconds before clearing WS connections). `entry_lightweight_mode()` aborts previous handles, but `exit_lightweight_mode()` does not.
- **Impact**: Toggling out of lightweight mode within 2 seconds leaves the cleanup task active, causing it to clear WebSocket connections or run memory trimming after the main window is active.
- **Remediation**: Call `abort_lightweight_cleanup()` at the start of `exit_lightweight_mode()`.

#### [FM-10] [MINOR / STYLE] Mixed Mutex Primitives
- **File & Line**: [`src-tauri/src/module/monitor.rs:16,19`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L16) vs [`src-tauri/src/core/service.rs:39`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs#L39)
- **Root Cause**: `monitor.rs` uses `std::sync::Mutex` with explicit poison handling (`.unwrap_or_else`), whereas `service.rs` uses `parking_lot::Mutex`.
- **Remediation**: Standardize on `parking_lot::Mutex` across all backend modules.

---

## 4. React/TypeScript Frontend Exhaustive Code Review (`src/`) (M3)

### 4.1 Architecture & Performance Verification Highlights
- **Hook & Event Cleanup Safety**:
  - [`src/providers/app-data-provider.tsx:303-346`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/app-data-provider.tsx#L303-L346): Tauri event listeners (`profile-changed`, `verge://refresh-proxy-config`) clean up `unlistenProfile`/`unlistenProxy` properly even if unmounted while `listen()` Promise is pending.
  - [`src/providers/window/window-provider.tsx:233-240`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/window/window-provider.tsx#L233-L240): Window `onResized` handles async resolution safely via `unlistenPromise.then(unlisten => unlisten())`.
  - [`src/hooks/use-traffic-monitor.ts:439-483`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-monitor.ts#L439-L483): `refCounter` uses 5-minute delayed destruction (`DESTROY_DELAY_MS = 300,000ms`) before worker cleanup.
- **State & Re-render Optimization**:
  - Connection data differential updates use `Map` lookups ([`src/hooks/use-connection-data.ts:99-184`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-connection-data.ts#L99-L184)).
  - Traffic stream updates throttled to 1000ms ([`src/hooks/use-traffic-data.ts:46`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-data.ts#L46)).
  - Delay notifications batched into 120ms throttle windows ([`src/services/delay.ts:70-144`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L70-L144)).
  - `ProxyItem` memoized via custom comparator ([`src/components/proxy/proxy-item.tsx:237-261`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-item.tsx#L237-L261)).
  - Virtualized list rendering powered by `@tanstack/react-virtual` ([`src/components/proxy/proxy-groups.tsx:95-110`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L95-L110)).
- **Async & Error Protection**:
  - `withIpcTimeout` protection (10s/30s/60s) across all Tauri IPC dispatches ([`src/services/cmds.ts:40-74`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts#L40-L74)).
  - `<BaseErrorBoundary>` and modular `<ErrorBoundary>` isolation across all routes and layout panels ([`src/main.tsx:90`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/main.tsx#L90), [`src/pages/_layout.tsx`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)).
- **Skin Aesthetics & Layout Compliance**:
  - 6 Skin styles (`Trump-3D`, `Original`, `Modern`, `Frosted Glass`, `Cyberpunk`, `Monochrome`) in [`src/utils/button-styles.ts:17-489`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/button-styles.ts#L17-L489).
  - Excel-style skin selector (`462.5px` total width, `Trump-3D` cell `92.5px` at left `177.5px`, 5 right cells `74px` each).
  - `GlowBorder` stealth mode (4px border during hidden titlebar mode).
  - Double border styling (`4px double` Settings drawer, `5px double` proxy grid divider).
  - Width-based display modes: Minimal ($\le 285\text{px}$) unmounts `ConnectionsPanel`, uses 100px lower pane, 1-column proxy grid, compact `TrafficGraph` (55px height, 2.5px line width). Wide ($> 285\text{px}$) uses 135px lower pane, 3-column proxy grid, `EnhancedCanvasTrafficGraph`.

### 4.2 Defect Inventory (Frontend Minor Issues)

| ID | File Path | Line Range | Category | Description | Proposed Remediation |
|---|---|---|---|---|---|
| **ISSUE-01** | [`src/hooks/use-mihomo-ws-subscription.ts`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-mihomo-ws-subscription.ts#L237-L364) | 237–364 | Hook Safety | `useEffect` suppresses ESLint dependency check (`[subscriptionCacheKey]`). Options callbacks (`connect`, `setupHandlers`) won't trigger re-run if changed. | Wrap options callbacks in `useRef` / `useStableFn`. |
| **ISSUE-02** | [`src/hooks/use-visibility.ts`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-visibility.ts#L38-L49) | 38–49 | Performance | `useEffect` calls `setDebouncedVisible(true)` synchronously when `rawVisible` becomes `true`, causing an extra immediate render cycle. | Derive visible state during render or use transition handler. |
| **ISSUE-03** | [`src/components/proxy/use-render-list.ts`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-render-list.ts#L195-L206) | 195–206 | Performance | `groupProxies` creates new array instances (`proxyCol`) for every row on every `delayBump`, invalidating shallow prop comparisons. | Memoize `proxyCol` row arrays. |
| **ISSUE-04** | [`src/components/proxy/use-head-state.ts`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-head-state.ts#L97-L108) | 97–108 | Async / IPC | `get_proxy_head_state` on initial mount triggers auto-save `useEffect`, writing same data back over IPC. | Add `isInitialLoad` ref flag to skip initial save. |
| **ISSUE-05** | [`src/hooks/use-profiles.ts`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-profiles.ts#L199-L209) | 199–209 | Async Safety | `calibrateSelected` returns an unhandled promise within `activateSelected`. | Await `calibrateSelected` or handle errors explicitly. |
| **ISSUE-06** | [`src/pages/_layout.tsx`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L536-L564) | 536–564 | Async Safety | `triggerWakeupLatencyTest` sets `wakeupTestTimerRef` via `setTimeout(..., 0)` without checking unmount status inside callback. | Check `isMountedRef` inside `setTimeout` callback before IPC dispatch. |
| **ISSUE-07** | [`src/pages/_layout/components/profile-import-card.tsx`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/profile-import-card.tsx#L205-L216) | 205–216 | Performance | `profileItems.map(...)` defines inline `onContextMenu` functions per item without memoization. | Extract item renderer into memoized subcomponent. |

---

## 5. Master Prioritized Issue Matrix

| Severity | Issue ID | Module / File Path | Summary & Failure Mode | Impact |
|---|---|---|---|---|
| **CRITICAL** | **FM-01** | [`src/components/layout/resize-handles.tsx:282`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L282) | Missing 200ms cool-down reset; `setWindowResizing(false)` runs synchronously on `pointerup`. | Agreement §2.2 / v2.5.4 violation; mouse release event cascade & UI freeze. |
| **CRITICAL** | **FM-07** | [`src-tauri/src/module/lightweight.rs:195`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L195) | Retained `SetProcessWorkingSetSize` memory trimming in lightweight mode. | Agreement §1.2 violation; WebView2 page-fault deadlocks on focus restore. |
| **MAJOR** | **FM-02** | [`src/components/layout/resize-handles.tsx:162-179`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L162-L179) | Resizing top/left edges issues concurrent `setSize` + `setPosition` IPCs. | Dual Win32 `SetWindowPos` calls per frame; DWM flickering & boundary lag. |
| **MAJOR** | **FM-03** | [`src/components/layout/resize-handles.tsx:157-160`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L157-L160) | Target geometry not cached against last applied values. | Floods Tauri IPC with duplicate `setSize(285, 135)` calls at minimum bounds. |
| **MAJOR** | **FM-04** | [`src/components/layout/resize-handles.tsx:289`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L289) | `handlePointerUp` calls `applyPending` without checking `session.inFlight`. | Parallel in-flight IPC dispatches & race conditions on mouse release. |
| **MAJOR** | **FM-08** | [`src-tauri/src/module/monitor.rs:994`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L994) | Synchronous `MessageBoxW` dialog called directly inside Tokio task loop. | Blocks Tokio async worker thread until user dismisses error dialog. |
| **MAJOR** | **FM-09** | [`src-tauri/src/module/lightweight.rs:209-254`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L209-L254) | `exit_lightweight_mode()` does not abort `LIGHTWEIGHT_CLEANUP_HANDLE`. | Background 2-second sleep/cleanup task races after window restoration. |
| **MINOR** | **FM-05** | [`src/utils/resize-geometry.ts:34-51`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/resize-geometry.ts#L34-L51) | Unclamped `dx`/`dy` deltas accumulate past minimum bounds limit. | Perceived "dead zone" / hysteresis when reversing mouse drag direction. |
| **MINOR** | **FM-06** | [`src/utils/resize-geometry.ts:70`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/resize-geometry.ts#L70) | Logical position calculation uses non-integer float conversion (`newX / sf`). | 1-pixel positional crawling/jitter along top/left edges under fractional DPI. |
| **MINOR** | **ISSUE-01** | [`src/hooks/use-mihomo-ws-subscription.ts:237`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-mihomo-ws-subscription.ts#L237) | `useEffect` dependency array suppresses options callback updates. | Options callback changes won't trigger re-subscribe. |
| **MINOR** | **ISSUE-03** | [`src/components/proxy/use-render-list.ts:195`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-render-list.ts#L195) | `groupProxies` allocates new row array instances on every delay bump. | Shallow prop comparison invalidation for proxy row items. |
| **INFO** | **FM-10** | [`src-tauri/src/module/monitor.rs:16`](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L16) | `monitor.rs` uses `std::sync::Mutex`, while `service.rs` uses `parking_lot::Mutex`. | Code style inconsistency; minor boilerplate overhead. |

---

## 6. Verification & Forensic Integrity Summary

- **Forensic Audit Verdict**: **CLEAN**
- **Git Status**: 100% clean (`git status` shows no uncommitted source changes in `src/` or `src-tauri/`).
- **Citation Verification**: All file paths, line ranges, function signatures, and failure mode mechanics cited in this report have been independently verified against the physical repository.
- **Compilation Prerequisites**: `src-tauri/Cargo.toml` and `tsconfig.json` static configurations have been verified for clean compilation.

*Report compiled by Project Orchestrator for Clash Mini (v2.6.6).*
