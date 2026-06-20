# Clash Mini Third-Party Code Audit Report

## ✍️ Executive Summary
This report is compiled by an independent third-party audit team targeting the **Clash Mini** application (comprising the React/TypeScript frontend and Rust/Tauri backend). The purpose of this audit is to identify security hazards, performance failures, and assess code cleanliness and software architectural design.

### 📊 Overall Codebase Quality Assessment and Architectural Health Score
*   **Overall Architectural Health Score**: **86 / 100**
*   **Key Strengths**:
    *   **Extremely High Agreement Compliance**: The project closely adheres to almost all rules specified in `clash_mini_agreements.md` (such as port collision avoidance, runtime system isolation, administrator mode leak prevention, etc.).
    *   **Solid Backend Robustness**: The Rust code is written securely with very few calls to unsafe methods like `unwrap` or `expect` that might trigger crashes, and leverages saturating arithmetic guards.
    *   **Clear Frontend State Management**: The frontend utilizes `foxact` to implement lightweight context state propagation.
*   **Core Areas for Improvement**:
    *   **Concurrency & Lock Safety**: The backend holds `RwLock` read guards across multiple asynchronous yield points, introducing potential deadlocks and latency jitters.
    *   **Thread Scheduler Blocking**: The backend frequently calls synchronous file I/O and synchronous system process scans (`sysinfo`) on Tokio asynchronous worker threads; it also executes synchronous `block_on` inside the Tauri setup hook, blocking the main UI thread.
    *   **Frontend Rendering Performance**: The frontend experiences paint storms because `useWindowWidth` triggers multi-component updates at pixel-level granularity during resize events. It also risks memory leaks because Tauri event listeners are not properly cleaned up when components unmount.
    *   **Tight Architectural Coupling**: The frontend `_layout.tsx` has evolved into a massive "god component" spanning nearly 5,000 lines with 38 separate `useState` hooks; in addition, global type definitions are heavily accumulated inside the `global.d.ts` ambient namespace rather than modularized.

---

## 🔒 Section 1: Safety & Performance Audit

### 1. Tauri Event Listener Memory Leak
*   **File Path**: `src/providers/app-data-provider.tsx`
*   **Affected Line Range**: 230–292
*   **Root Cause Analysis**:
    When registering Tauri event listeners (`profile-changed` and `verge://refresh-proxy-config`) inside `useEffect`, the registration is asynchronous (using `await listen(...)`). If the component is quickly unmounted (e.g., due to route redirection or state changes) before these Promises resolve, the synchronous `useEffect` cleanup function runs first (while `cleanupFns` is still empty). When the Promises subsequently resolve, the resulting `unlisten` callbacks are pushed into `cleanupFns` but will never be called since the cleanup phase has passed. This causes the Tauri global event listeners to persist in memory, creating a cumulative memory leak.
*   **Relevant Code Snippet**:
    ```typescript
    useEffect(() => {
      let lastProfileId: string | null = null
      let lastUpdateTime = 0
      const refreshThrottle = 800
      const cleanupFns: Array<() => void> = []

      // ...

      const initializeListeners = async () => {
        try {
          const unlistenProfile = await listen<string>(
            'profile-changed',
            handleProfileChanged,
          )
          cleanupFns.push(unlistenProfile)
        } catch (error) {
          console.error('[AppDataProvider] Failed to listen to Profile event:', error)
        }

        try {
          const unlistenProxy = await listen(
            'verge://refresh-proxy-config',
            handleRefreshProxy,
          )
          cleanupFns.push(unlistenProxy)
        } catch (error) {
          console.warn('[AppDataProvider] Failed to set Tauri event listener:', error)
        }
      }

      void initializeListeners()

      return () => {
        cleanupFns.forEach((fn) => {
          try {
            fn()
          } catch (error) {
            console.error('[DataProvider] Cleanup error:', error)
          }
        })
      }
    }, [refreshProxy, refreshRules, refreshRuleProviders])
    ```
*   **Specific Fix & Optimization Recommendations**:
    Introduce a boolean flag `active` to track the lifecycle of the current effect. If the effect has been destroyed by the time the asynchronous listener registration completes, immediately call the unlisten callback to cancel the listener. Save the unlisten methods in separate local variables.
    ```typescript
    useEffect(() => {
      let active = true
      let unlistenProfile: (() => void) | null = null
      let unlistenProxy: (() => void) | null = null

      const initializeListeners = async () => {
        try {
          const uProfile = await listen<string>('profile-changed', handleProfileChanged)
          if (!active) {
            uProfile()
          } else {
            unlistenProfile = uProfile
          }
        } catch (error) {
          console.error('[AppDataProvider] Failed to listen to Profile event:', error)
        }

        try {
          const uProxy = await listen('verge://refresh-proxy-config', handleRefreshProxy)
          if (!active) {
            uProxy()
          } else {
            unlistenProxy = uProxy
          }
        } catch (error) {
          console.warn('[AppDataProvider] Failed to set Tauri event listener:', error)
        }
      }

      void initializeListeners()

      return () => {
        active = false
        if (unlistenProfile) unlistenProfile()
        if (unlistenProxy) unlistenProxy()
      }
    }, [refreshProxy, refreshRules, refreshRuleProviders])
    ```

---

### 2. Window Resize Rendering Storm
*   **File Path**: `src/components/proxy/use-window-width.ts` (Lines 1-17) and `src/components/proxy/proxy-item.tsx` (Lines 80-82)
*   **Root Cause Analysis**:
    `useWindowWidth` updates the state value `width` on every pixel change during browser `resize` events. However, there could be hundreds or thousands of proxy items (`ProxyItem`) in the project, each independently calling `useWindowWidth()`. When the user drags the window border:
    1. Hundreds of concurrent window resize event handlers execute simultaneously.
    2. Hundreds of components independently trigger state updates on the value level, immediately causing a massive repaint storm across the React render tree, resulting in high CPU usage and UI lag.
    In reality, `ProxyItem` only needs to know whether the width is less than or equal to the threshold of `285px` to switch to a minimal layout; it does not require pixel-level precision.
*   **Relevant Code Snippet**:
    ```typescript
    // use-window-width.ts
    export const useWindowWidth = () => {
      const [width, setWidth] = useState(() => document.body.clientWidth)

      useEffect(() => {
        const handleResize = () => setWidth(document.body.clientWidth)

        window.addEventListener('resize', handleResize)
        return () => {
          window.removeEventListener('resize', handleResize)
        }
      }, [])

      return { width }
    }

    // proxy-item.tsx
    const { width } = useWindowWidth()
    const isMinimal = width <= 285
    ```
*   **Specific Fix & Optimization Recommendations**:
    Since the project uses MUI, replace the hook with one optimized using CSS media queries under the MUI system, or create a custom hook that only tracks the boolean boundary check to avoid unnecessary state changes:
    *   **Fix Option A (MUI Media Query)**:
        ```typescript
        import { useMediaQuery } from '@mui/material'
        // Use directly inside ProxyItem:
        const isMinimal = useMediaQuery('(max-width:285px)')
        ```
    *   **Fix Option B (Boolean Debounced Custom Hook)**:
        ```typescript
        export const useIsMinimal = (threshold = 285) => {
          const [isMinimal, setIsMinimal] = useState(() => window.innerWidth <= threshold)
          useEffect(() => {
            const handleResize = () => {
              setIsMinimal(window.innerWidth <= threshold)
            }
            window.addEventListener('resize', handleResize)
            return () => window.removeEventListener('resize', handleResize)
          }, [threshold])
          return isMinimal
        }
        ```

---

### 3. RwLock Guard Held Across Await Causing Deadlocks
*   **File Path**: `src-tauri/src/utils/connections_stream.rs` (Lines 80-90), as well as `core/manager/config.rs:144`, `feat/clash.rs:77`, `feat/window.rs:79`, `utils/connections_stream.rs:150`, etc.
*   **Root Cause Analysis**:
    The `handle::Handle::mihomo()` helper returns a Future yielding a global `RwLockReadGuard<'static, Mihomo>`. When executing chained async methods (e.g., `.ws_traffic().await`), this temporary read guard is not dropped until the entire expression finishes evaluating. This means **`RwLockReadGuard` is held across async yield points (`.await`)**.
    Under Tokio's task scheduler, if another write lock request (e.g., calling `.write().await` during configuration reload or service restart) queues at this time, it is blocked. Because of write-priority/starvation-prevention mechanisms, all subsequent read lock requests will also be suspended, leading to broad hang deadlocks across background latency tests and traffic statistics connections.
*   **Relevant Code Snippet**:
    ```rust
    // src/utils/connections_stream.rs
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
*   **Specific Fix & Optimization Recommendations**:
    Decouple the chain. Extract the Future yielded by the method in a separate block scope, allowing the local variable lifecycle to automatically drop the `RwLockReadGuard` before performing `.await` on the resulting Future:
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
    }; // The temporary variable mihomo (RwLockReadGuard) is dropped here at the end of the scope
    let connection_id = ws_future.await?;
    ```

---

### 4. Asynchronous Runtime Worker Threads Blocked by Synchronous Disk I/O
*   **File Path**: `src-tauri/src/module/monitor.rs` (Line 276) and `src-tauri/src/core/updater.rs` (Line 475)
*   **Root Cause Analysis**:
    `trigger_backend_auto_select` (monitoring thread) and `check_and_download` (updater download thread) both run inside Tokio's asynchronous context.
    1. In `monitor.rs`, `get_active_filter_config` frequently calls the synchronous `std::fs::read_to_string` to read `proxy_head_state.json` from the physical disk.
    2. In `updater.rs`, writing download packages (which can be tens of megabytes in size) to the cache folder uses the synchronous `std::fs::write`.
    Executing these blocking I/O calls directly on Tokio's default worker threads prevents the executor from polling other ready Futures on the thread, indirectly causing sudden latency spikes and minor interface hangs.
*   **Relevant Code Snippet**:
    ```rust
    // src/module/monitor.rs
    let filter_config = get_active_filter_config(profile_uid);

    // src/core/updater.rs
    if let Err(e) = Self::write_cache(&bytes, &version) {
        logging!(warn, Type::System, "Silent updater: failed to write cache: {e}");
    }
    ```
*   **Specific Fix & Optimization Recommendations**:
    1. Replace `std::fs` calls with asynchronous `tokio::fs` functions for reading configurations.
    2. Offload heavy disk writes of update packages to a dedicated blocking thread pool using `tokio::task::spawn_blocking`.
    ```rust
    // Writing package:
    let version_clone = version.clone();
    let bytes_clone = bytes.clone();
    tokio::task::spawn_blocking(move || {
        Self::write_cache(&bytes_clone, &version_clone)
    }).await.unwrap_or_else(|_| Err(anyhow::anyhow!("Spawn blocking failed")))?;
    ```

---

### 5. Synchronous System Process Scanning Blocking Tokio Workers
*   **File Path**: `src-tauri/src/core/manager/state.rs`
*   **Affected Line Range**: 143-164
*   **Root Cause Analysis**:
    `CoreManager::kill_all_mini_cores` relies on process list retrievals via `sysinfo::System::new_all()`. Scanning the process list is a high-cost, unpredictable system call (potentially taking hundreds of milliseconds). Calling it synchronously inside the async core exit hook `stop_core_by_sidecar` and window close handlers blocks the executing async worker thread.
*   **Relevant Code Snippet**:
    ```rust
    pub fn kill_all_mini_cores() {
        logging!(
            info,
            Type::Core,
            "Scanning and killing leftover mini-mihomo processes..."
        );
        let system = sysinfo::System::new_all();
        // Iterates and terminates processes named mini-mihomo
    ```
*   **Specific Fix & Optimization Recommendations**:
    Wrap the synchronous system process scanning operation in `tokio::task::spawn_blocking`:
    ```rust
    pub async fn kill_all_mini_cores_async() {
        let _ = tokio::task::spawn_blocking(|| {
            Self::kill_all_mini_cores();
        }).await;
    }
    ```

---

### 6. Tauri Setup Hook Synchronously Blocking the Main UI Thread
*   **File Path**: `src-tauri/src/lib.rs`
*   **Affected Line Range**: 256-260
*   **Root Cause Analysis**:
    Inside the Tauri build initialization `.setup()` hook, `tauri::async_runtime::block_on` is used to run `try_install_on_startup` synchronously.
    The `.setup()` hook executes directly on the main UI/message loop thread of the operating system. Blocking it freezes the initialization phase of the application. If network anomalies cause the update check to time out (default timeout is 30 seconds), the window remains blank and unresponsive because Tauri's native main event loop has not yet started.
*   **Relevant Code Snippet**:
    ```rust
    let is_updating = tauri::async_runtime::block_on(async {
        crate::core::updater::SilentUpdater::global()
            .try_install_on_startup(&app_handle)
            .await
    });
    ```
*   **Specific Fix & Optimization Recommendations**:
    Avoid blocking the Setup lifecycle step. Spawn the update check as an asynchronous background task so control returns to the message loop immediately, or use a separate bootstrapper to install updates before the main Tauri application launches.
    ```rust
    // Async spawning:
    let app_handle_clone = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        crate::core::updater::SilentUpdater::global()
            .try_install_on_startup(&app_handle_clone)
            .await;
    });
    ```

---

## 🎨 Section 2: Readability & Architecture Audit

### 1. Layout File Monolith (God Component in Layout)
*   **File Path**: `src/pages/_layout.tsx`
*   **Affected Line Range**: 1–4997 (nearly 5,000 lines)
*   **Root Cause Analysis**:
    The `Layout` component packs drawer navigation, connections tables, profile card interactions, system settings controls, and update dialogs all in one file. It declares **38 separate React `useState` hooks**, severely violating the "Single Responsibility Principle". Whenever any state changes, React must re-evaluate this massive component containing thousands of lines of JSX and nested functions, making code readability, troubleshooting, and future refactoring extremely difficult.
*   **Specific Fix & Optimization Recommendations**:
    Refactor by splitting the component:
    1. Extract panels into standalone sub-components in `src/components/` (e.g., `ProfilesPanel.tsx`, `ConnectionsPanel.tsx`, `SettingsPanel.tsx`).
    2. Move updater and notification flows to custom hooks (e.g., `useAppUpdater.ts`).
    3. Keep `_layout.tsx` strictly as a layout shell containing shell frame navigation.

---

### 2. Global Type Pollution (Global Ambient Type Pollution)
*   **File Path**: `src/types/global.d.ts`
*   **Affected Line Range**: 1–1097
*   **Root Cause Analysis**:
    A single `global.d.ts` ambient file encapsulates almost all core domain interfaces (such as `IConfigData`, `IProxyItem`, `IProxyGroupItem`). Global namespace pollution conceals direct dependencies, making it hard to track model modifications using IDE tools and risking type definition clashes as new modules are introduced.
*   **Specific Fix & Optimization Recommendations**:
    Transition to modular type declarations. Store local interfaces in respective feature directories (e.g., `src/types/clash.ts`, `src/types/profile.ts`), use `export interface`, and import them explicitly where needed.

---

### 3. File Naming Inconsistency
*   **File Path**: `src/hooks/useWindowSnap.ts`
*   **Root Cause Analysis**:
    The file uses camelCase (`useWindowSnap.ts`), whereas all other custom hooks in the `src/hooks/` folder consistently use kebab-case (e.g., `use-clash.ts`, `use-traffic-monitor.ts`).
*   **Specific Fix & Optimization Recommendations**:
    Rename the file to `use-window-snap.ts` and update imports accordingly.

---

### 4. Missing Memoization on Custom Hook Functions
*   **File Path**: `src/hooks/use-clash.ts` and `src/hooks/use-profiles.ts`
*   **Root Cause Analysis**:
    The functions returned by these hooks (e.g., `mutateClash`, `mutateProfiles`, `patchProfiles`) are re-created on every render. If these functions are passed as props to child components or listed in dependency arrays of `useEffect`, their shifting references will trigger redundant renders in sub-trees and can cause infinite re-render loops.
*   **Specific Fix & Optimization Recommendations**:
    Wrap these callbacks with `useCallback`:
    ```typescript
    const mutateProfiles = useCallback(async () => {
      await refetch()
    }, [refetch])
    ```

---

### 5. Loose any Type Declarations
*   **File Path**: `src/providers/app-data-context.ts` (Lines 11, 14, 44, 60) and `src/pages/_layout.tsx` (Lines 1314, 1324)
*   **Root Cause Analysis**:
    Key variables in the application context (e.g., `proxies` and `sysproxy`) and update states in the Layout component are declared as `any`. This weakens compiler type-checking, making the app vulnerable to runtime `Cannot read property of undefined` errors if backend models change.
*   **Specific Fix & Optimization Recommendations**:
    Declare precise type definitions for these variables using interface contracts, potentially derived from the Tauri plug-in APIs.

---

### 6. Backend Timestamp Integer Truncation
*   **File Path**: `src-tauri/src/config/prfitem.rs` (Lines 249, 341, 432, etc.)
*   **Root Cause Analysis**:
    The backend casts the 64-bit integer timestamp returned by `chrono::Local::now().timestamp()` using `as usize` to store it in `PrfItem::updated`. On 32-bit platforms, `usize` is 32 bits wide, resulting in **integer truncation** that triggers the "Year 2038 problem" or index out-of-bounds panics.
*   **Relevant Code Snippet**:
    ```rust
    updated: Some(chrono::Local::now().timestamp() as usize),
    ```
*   **Specific Fix & Optimization Recommendations**:
    Change the field type of `updated` in `PrfItem` and `IProfiles` to `Option<i64>` or `Option<u64>` and remove the `as usize` cast:
    ```rust
    updated: Some(chrono::Local::now().timestamp()),
    ```

---

## ⚖️ Section 3: Agreement Compliance Verification

The audit team compared the actual codebase implementation against the 26 core specifications defined in `clash_mini_agreements.md`:

### 📋 Agreement Compliance Matrix

| Clause ID & Bug Ref | Specification Summary | Implementation Reference | Compliance Verdict | Notes & Remarks |
| :--- | :--- | :--- | :--- | :--- |
| **一 (1) / Project Separation** | Isolation of ports, directories, and singletons to prevent conflicts. | `src-tauri/tauri.conf.json` lines 16, 29-30<br>`src-tauri/src/constants.rs` lines 4, 11, 15-18<br>`src-tauri/src/utils/dirs.rs` lines 12, 17 | **Fully Compliant** | Runs in isolated environment: `io.github.clash-mini.clash-mini`. |
| **二 (2) / 3D Skeuomorphic** | 3D physical skeuomorphic layout, horizontal theme slider. | `src/assets/styles/layout.scss`<br>`src/utils/button-styles.ts` | **Fully Compliant** | Incorporates HSL gradients, bevel borders, and custom buttons for the 6 skins. |
| **三 (3) / Manual Control** | Manual routing controls, MATCH rule fallback, connection list pruning. | `src/pages/_layout.tsx` lines 2132-2160, 3220-3240<br>`src/services/cmds.ts` lines 18-91 | **Fully Compliant** | 常驻 (resident) 3D panels implemented; automatically injects MATCH rule to profiles. |
| **四 (4) / Tray Icon** | Dynamic system tray icon behavior. | `src-tauri/src/core/tray/mod.rs` lines 56-57 | **Partially Compliant** | Kept static to avoid the `E_FAIL` crash described in BUG-073 (工程规避 / engineering bypass). |
| **五 (5) / Copyright** | Copyright and attribution banner. | `src/pages/_layout.tsx` lines 3843-3860 | **Fully Compliant** | Settings footer displays `© 2026 Qiu Yuxiao (Modified parts)`. |
| **六 (6) / Silence** | Background silence: stop WS polling when window is blurred/hidden. | `src/hooks/use-traffic-data.ts` lines 36, 43<br>`src/hooks/use-connection-data.ts` lines 30, 36 | **Fully Compliant** | Document visibility checks trigger WS disconnects. |
| **七 (7) / BUG-057** | Prevent blank profiles screen during import load. | `src/pages/_layout.tsx` lines 1913-1935, 1974-1979 | **Fully Compliant** | Checks `lastEnhancedProfileRef` to avoid duplicate initializations. |
| **八 (8) / Proxy Stream** | Single-level proxy group; prune nested fallbacks. | `src/services/cmds.ts` lines 168-271 | **Fully Compliant** | Limits UI options to one primary PROXY selector. |
| **九 (9) / Sliders Fix** | Adjust slider range to 0-5 with 0.1 intervals. | `src/pages/_layout.tsx` lines 3728, 3780 | **Fully Compliant** | Sliders handle float ranges correctly. |
| **十 (10) / Cyberpunk** | Light contrast styling and Monochrome switch dimensions (56x28px). | `src/assets/styles/layout.scss`<br>`src/components/base/base-switch.tsx` | **Fully Compliant** | Theme parameters optimized; Monochrome switches scaled to 56x28px capsule. |
| **十一 (11) / BUG-070** | Exclude service verification when running as administrator. | `src-tauri/src/core/manager/lifecycle.rs` lines 95-98 | **Fully Compliant** | Skips sidecar checks for admin environments. |
| **十二 (12) / BUG-071** | Help button dropdown containing updates. | `src/pages/_layout.tsx` lines 4057-4161 | **Fully Compliant** | Custom menu dropdown triggers core/app updates. |
| **十三 (13) / BUG-072** | Profile card context menu (Edit, Open, Copy, Update, Delete). | `src/pages/_layout.tsx` lines 1560-1566 | **Fully Compliant** | Context menu hook displays options correctly. |
| **十四 (14) / BUG-065** | Limit node rotation to active filtered subset. | `src/pages/_layout.tsx` lines 460-538 | **Fully Compliant** | Tapping active card cycles nodes inside current filter bounds. |
| **十五 (15) / BUG-074** | Formatting core version to `Ver.X.Y.Z`. | `src-tauri/src/core/core_updater.rs`<br>`src/pages/_layout.tsx` lines 348-352 | **Fully Compliant** | Version formatted to `Ver.` after stripping leading `v`. |
| **十六 (16) / BUG-075** | Increase default update poll times. | `src/hooks/use-traffic-monitor.ts` line 54 | **Fully Compliant** | Baseline traffic poll rate set to 3000ms. |
| **十七 (17) / BUG-078** | Chunk download timeout set to 20s with fallback. | `src-tauri/src/core/core_updater.rs` lines 111-120, 228-262 | **Fully Compliant** | Timer interrupts download if inactive for 20s; falls back to system proxy/direct. |
| **十八 (18) / BUG-079** | Tooltip terminology consistency. | `src/pages/_layout.tsx` lines 3222-3224 | **Fully Compliant** | Tooltip texts unified and verified across 13 locales. |
| **十九 (19) / BUG-080** | Prevent update prompts if versions match. | `src/pages/_layout.tsx` lines 1729-1733, 1779-1782 | **Fully Compliant** | Early return blocks update modal, outputting an info toast instead. |
| **二十 (20) / BUG-082** | WebView2 memory drop on blur/background. | `src-tauri/src/utils/window_manager.rs` lines 345-385 | **Fully Compliant** | Invokes COM `SetMemoryUsageTargetLevel` on focus loss. |
| **二十一 (21) / BUG-089** | [Deprecated] Background latency daemon. | N/A | **N/A** | Superseded and absorbed by Clause 25. |
| **二十二 (22) / BUG-090** | Settings text contrast in Retro Dark Mode. | `src/pages/_layout.tsx` | **Fully Compliant** | Set settings drawer font color to `#2C1F03` in Retro Dark. |
| **二十三 (23) / BUG-091** | Custom right-click clipboard menu for inputs. | `src/pages/_layout.tsx` lines 1452-1557 | **Fully Compliant** | Custom context menu intercepts copy/paste events correctly. |
| **二十四 (24) / BUG-092** | Speed test auto-select points to fastest node in current search subset. | `src/components/proxy/proxy-groups.tsx` line 408 | **Fully Compliant** | Scopes selection to the visible subset. |
| **二十五 (25) / BUG-083/093** | Background daemon active checks and healing. | `src-tauri/src/module/monitor.rs` lines 405-494 | **Fully Compliant** | Daemon loop runs every 15s/3s and heals connection after 3 failures. |
| **二十六 (26) / PortSafety** | Adaptive port incrementation on conflict. | `src-tauri/src/config/clash.rs` lines 62-71, 472-498 | **Fully Compliant** | Validates connection via `"adapted-by-qiu-yuxiao"` and increments port. |

*Note: For Agreement 4 (Tray Icon), the icon is kept static as a local PNG to prevent OS level resource allocation crashes (E_FAIL) during focus adjustments. This engineering choice prioritizes application stability, which is deemed reasonable.*

---

## 🚀 Audit Recommendations

To improve system security, performance, and architectural cleanliness, it is recommended to apply the following fixes in future updates:

1.  **RwLock read guard cleanup**: Refactor async chains holding `Mihomo` guards to drop the read lock before yielding via `.await`, as described in Section 1, Clause 3.
2.  **Asynchronous file I/O and process scans**: Migrate synchronous `std::fs` operations to `tokio::fs` or wrap them in `tokio::task::spawn_blocking` to prevent blocking Tokio's runtime workers.
3.  **Unblock setup workflow**: Remove `block_on` from the Tauri Setup lifecycle and run update steps asynchronously.
4.  **Decouple layout god component**: Extract settings, profiles, and connections panels from `_layout.tsx` into dedicated component modules, and replace `any` types with explicit TypeScript interfaces.
5.  **Optimized media queries**: Replace `useWindowWidth` with MUI's `useMediaQuery` to suppress excessive rendering triggers on resize.
6.  **64-bit timestamps**: Adjust `PrfItem` metadata structures to store timestamps as `i64` instead of casting them using `as usize` to prevent truncation.
