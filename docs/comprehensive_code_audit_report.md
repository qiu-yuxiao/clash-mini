# Comprehensive Code Audit & Bug Review Report

This report presents the findings of a comprehensive, read-only code audit of the Clash Verge / Clash Mini repository, covering the TypeScript/React/Material-UI frontend and the Rust (`src-tauri/`) backend core logic.

---

## Executive Summary

A comprehensive code audit was conducted on the Clash Verge/Mini repository to evaluate its correctness, security, concurrency safety, and platform compatibility. The audit identified **22 distinct findings** categorized into three major domains:

1.  **Frontend & Latency UI Logic (8 findings)**: Major React Hook dependency arrays leading to hook state starvation, background timer leaks, double-selection backend request race conditions, memory leaks, and unhandled Promise rejections.
2.  **Rust Backend Core Logic & Concurrency (8 findings)**: Zero-length buffer reads in HTTP latency measurements (breaking proxied latency tests), infinite execution/CPU hangs during JS script and configuration validation, security SSRF protection bypasses, silent app crashes/exits on singleton check failures, background notify state loss, and relative path command execution hazards.
3.  **Layout, Styling & WebView2 Rendering (6 findings)**: CSS selector specificity conflicts rendering dialogs translucent, hardcoded background colors causing theme coherence leakage in Cyberpunk/Glass skins, scrollbar leak conflicts under new WebView2 engines, dead scrollbar color CSS variables, and vertical clipping hazards on high OS font scaling.

None of these findings have been modified in the workspace, complying with the **Strict Non-modification Constraint**.

---

## Detailed Audit Findings

### I. Frontend & Latency UI Logic

#### Finding 1: React Hook Dependency Safety — Permanent Cancellation of Profile Activation
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1114](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1114)
*   **Severity**: Critical
*   **Root Cause & Logic Analysis**:
    The profile enhancement `useEffect` hook monitors active profile switches. The hook's dependency array includes volatile dependencies: `refreshProxy`, `setHeadStateForSort`, and `t`.
    1. `setHeadStateForSort` (returned by `useHeadStateNew`) updates its reference whenever the profile list or configuration changes.
    2. The localization function `t` updates its reference whenever language packs are fetched.
    3. Any updates to these dependencies trigger the hook's cleanup function, which sets `cancelled = true`. This aborts the pending asynchronous `enhanceProfiles().then(...)` promise chain.
    4. However, before the promise finishes, `lastEnhancedProfileRef.current` has already been synchronously set to `currentProfileUid`.
    5. When the hook re-executes, the check `lastEnhancedProfileRef.current !== currentProfileUid` evaluates to `false`, leaving the execution block completely dead.
    *   **Impact**: Active profile enhancement, core readiness checks, and auto-node selection are permanently aborted mid-execution, preventing the backend from initializing correctly for the profile.
*   **Remediation Recommendation**:
    Store the volatile callback dependencies (`refreshProxy`, `setHeadStateForSort`, `t`) in stable `useRef` references that are updated on every render, keeping only `currentProfileUid` in the `useEffect` dependency array:
    ```tsx
    const refreshProxyRef = useRef(refreshProxy)
    const setHeadStateForSortRef = useRef(setHeadStateForSort)
    const tRef = useRef(t)

    useEffect(() => {
      refreshProxyRef.current = refreshProxy
      setHeadStateForSortRef.current = setHeadStateForSort
      tRef.current = t
    })

    useEffect(() => {
      if (currentProfileUid && lastEnhancedProfileRef.current !== currentProfileUid) {
        lastEnhancedProfileRef.current = currentProfileUid
        let cancelled = false
        enhanceProfiles().then(async () => {
          if (cancelled) return
          await activateSelectedRef.current()
          await waitForClashReady(tRef.current, () => cancelled)
          if (cancelled) return
          await triggerAutoSelectAndRefresh(refreshProxyRef.current, tRef.current, fallbackTimerRef, setHeadStateForSortRef.current)
        })
        return () => { cancelled = true }
      }
    }, [currentProfileUid])
    ```

---

#### Finding 2: Profile Switch State Race Condition & Background Timer Leak
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L220](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L220)
*   **Severity**: High
*   **Root Cause & Logic Analysis**:
    `frontendAutoSelect` starts a background timer using `setInterval` to periodically query the latency of proxy group nodes. When the user switches active profiles, the layout `useEffect` cancels the current promise using `cancelled = true`. However, the background interval timer `activeAutoSelectTimer` is **never cleared** in this cancellation path.
    *   **Impact**: The stale interval loop continues running in the background. When it eventually finds a low-latency node, it calls `selectNodeForGroup` and `refreshProxy`, overwriting the newly selected nodes on the new profile with old node selections, creating state corruption.
*   **Remediation Recommendation**:
    Pass a cancellation callback check `isCancelled: () => boolean` to `frontendAutoSelect` and clear the interval immediately inside the timer callback if cancelled:
    ```tsx
    activeAutoSelectTimer = setInterval(async () => {
      if (isCancelled()) {
        clearInterval(activeAutoSelectTimer)
        activeAutoSelectTimer = null
        resolve([])
        return
      }
      // ... check health nodes
    }, 200)
    ```

---

#### Finding 3: Double Selection Request Race Condition
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L247](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L247)
*   **Severity**: Medium
*   **Root Cause & Logic Analysis**:
    In the polling loop of `frontendAutoSelect`, the system defines two selector blocks: "临时闪连" (Temporary Selection, triggered on the first healthy node) and "极速终选" (Final Selection, triggered when enough nodes are measured). If the first tick of the timer detects that both conditions are met (e.g. 5 nodes measured instantly under rapid connections), both blocks execute concurrently.
    *   **Impact**: Two parallel, redundant HTTP requests are sent to the Tauri backend command `select_node` at the same time, leading to database lock contention or undefined active proxy states.
*   **Remediation Recommendation**:
    Ensure the temporary selection block does not execute if the final selection criteria are already met:
    ```tsx
    if (!isFinalSelection && !hasSelectedTemp && healthyNodes.length >= 1) {
      // Temporary block
    }
    ```

---

#### Finding 4: Memory Leak of Unresolved Promises in `frontendAutoSelect`
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L226](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L226)
*   **Severity**: Low
*   **Root Cause & Logic Analysis**:
    When `frontendAutoSelect` is re-invoked, it checks if `activeAutoSelectTimer` is already active. If so, it calls `clearInterval(activeAutoSelectTimer)`. However, the original `Promise` created by the previous invocation is **never resolved or rejected**, leaving it suspended in JavaScript memory.
    *   **Impact**: Memory leak of Promise objects and context closures, which can degrade rendering performance over long periods of app execution.
*   **Remediation Recommendation**:
    Maintain a global `activeAutoSelectReject` handle and invoke it before spawning a new selection:
    ```tsx
    if (activeAutoSelectReject) {
      activeAutoSelectReject(new Error('Cancelled by new invocation'))
      activeAutoSelectReject = null
    }
    ```

---

#### Finding 5: Unhandled Promise Rejection in `checkDelay`
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L225](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L225)
*   **Severity**: Medium
*   **Root Cause & Logic Analysis**:
    `Promise.race` is used to implement a timeout for node speed tests:
    ```tsx
    const result = await Promise.race([
      delayProxyByName(name, url, timeout),
      timeoutPromise,
    ])
    ```
    If `timeoutPromise` resolves first (timeout limit reached), the execution moves on. However, the background API request `delayProxyByName` remains running. If that request eventually fails (e.g. socket reset), the rejection is not caught, causing an uncaught promise rejection.
    *   **Impact**: Triggers an `UnhandledPromiseRejection` exception in the WebView browser context, which can cause UI instability.
*   **Remediation Recommendation**:
    Attach a `.catch` error-swallowing handler directly to the promise inside the race array:
    ```tsx
    const result = await Promise.race([
      delayProxyByName(name, url, timeout).catch(err => ({ delay: 1e6 })),
      timeoutPromise,
    ])
    ```

---

#### Finding 6: Timeout Resource (Timer Handle) Leak in `checkDelay`
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L220](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L220)
*   **Severity**: Low
*   **Root Cause & Logic Analysis**:
    `timeoutPromise` creates a timeout using `setTimeout(() => resolve({ delay: 0 }), timeout)`. If the actual latency test `delayProxyByName` completes successfully *before* the timeout, the timeout timer is never cleared.
    *   **Impact**: Over-allocation of browser timer handles, leading to minor memory and CPU overhead.
*   **Remediation Recommendation**:
    Store the timer ID and call `clearTimeout(timerId)` when the race completes.

---

#### Finding 7: Contrast and Invisible Borders Styling Issue
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/button-styles.ts#L324](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/button-styles.ts#L324)
*   **Severity**: Low
*   **Root Cause & Logic Analysis**:
    In the Frosted Glass (`frosted-glass`) skin, the background and border for the disabled button state are hardcoded to transparent white colors: `rgba(255, 255, 255, 0.03)` and `rgba(255, 255, 255, 0.05)`. Under light theme modes, the contrast ratio is sub-2.6:1, rendering disabled buttons completely borderless and invisible against white backgrounds.
    *   **Impact**: Violates Web Content Accessibility Guidelines (WCAG) contrast standards.
*   **Remediation Recommendation**:
    Conditionally apply darker translucent backdrops under light themes:
    ```tsx
    background: isLight ? 'rgba(0, 0, 0, 0.03) !important' : 'rgba(255, 255, 255, 0.03) !important',
    borderColor: isLight ? 'rgba(0, 0, 0, 0.05) !important' : 'rgba(255, 255, 255, 0.05) !important',
    ```

---

#### Finding 8: React `useMemo` Dependency Array Omits theme and skin
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1478](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1478)
*   **Severity**: Medium
*   **Root Cause & Logic Analysis**:
    The layout titlebar is cached inside a `useMemo` block. The titlebar components depend heavily on the custom skin (`controlSkin`) and theme palette mode (e.g. for generating 3D buttons). However, `theme` and `controlSkin` were removed from the `useMemo` dependency array.
    *   **Impact**: When the user switches themes or changes the active skin, the cached Titlebar UI element does not re-render, leaving it with stale styles and creating display issues.
*   **Remediation Recommendation**:
    Re-add `theme` and `controlSkin` to the `useMemo` dependency array.

---

### II. Rust Backend Core Logic & Concurrency

#### Finding 9: Zero-Length Slice Read in HTTP Latency Test (Correctness/Logic Bug)
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L163](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L163)
*   **Severity**: Critical
*   **Root Cause & Logic Analysis**:
    The HTTP latency testing function initializes the network read buffer using `BytesMut::with_capacity(1024)`. This creates a buffer with a capacity of 1024 bytes but a logical length of 0.
    When the stream reads from the socket, `stream.read(&mut buf)` is called. Since `BytesMut` coerces to a mutable slice (`&mut [u8]`) using its logical length (which is 0), it evaluates to `&mut []`. Reading into an empty slice returns `Ok(0)` immediately without waiting for bytes.
    *   **Impact**: Latency measurements are incorrect, returning 0ms instantly. In proxied scenarios, the time measured only reflects local socket handshakes, making the speed test features useless.
*   **Remediation Recommendation**:
    Resize the buffer to its capacity before performing the read operation, or use a fixed array:
    ```rust
    let mut buf = vec![0u8; 1024];
    let _ = stream.read(&mut buf).await?;
    ```

---

#### Finding 10: Concurrency Hang / Infinite Loop in Script Validation
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs#L247](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs#L247)
*   **Severity**: High
*   **Root Cause & Logic Analysis**:
    The JavaScript validation feature executes JS scripts via the `boa_engine` interpreter:
    ```rust
    let mut context = Context::default();
    let result = context.eval(Source::from_bytes(&content));
    ```
    This evaluation is fully synchronous and lacks execution limits, watchdog controls, or instruction thresholds.
    *   **Impact**: If a user validates a script containing an infinite loop (e.g. `while(true){}`), the tokio worker thread executes the loop indefinitely, consuming 100% CPU. The validator state remains locked in the `is_processing` state, causing subsequent config validations to hang.
*   **Remediation Recommendation**:
    Add instruction limits or timeouts to the JS execution context, or run the validation in a spawned blocking thread with a strict watchdog.

---

#### Finding 11: Core Validation Infinite Hang due to Missing Command Timeout
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs#L362](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs#L362)
*   **Severity**: High
*   **Root Cause & Logic Analysis**:
    Core configuration validation is triggered via `command.output().await?`. No timeout is specified for this command execution.
    *   **Impact**: If the spawned sub-process hangs due to port conflicts or driver issues, `output().await` blocks indefinitely, locking the configuration validation module in a busy state.
*   **Remediation Recommendation**:
    Wrap the execution command inside a `tokio::time::timeout` wrapper:
    ```rust
    let output = tokio::time::timeout(Duration::from_secs(5), command.output()).await??;
    ```

---

#### Finding 12: SSRF Bypass via DNS Resolution and IPv6 Local Ranges (Security Vulnerability)
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/prfitem.rs#L824](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/prfitem.rs#L824)
*   **Severity**: High
*   **Root Cause & Logic Analysis**:
    The SSRF validation function `validate_url_no_ssrf` only performs simple string checks on host domains and loops back to block loopback/unspecified IPv6 addresses.
    1. It lacks DNS resolution checks, allowing SSRF bypass via local DNS pointers (e.g. a domain pointing to `127.0.0.1`) or DNS rebinding.
    2. The IPv6 check only blocks loopback (`::1`) or unspecified (`::`), but fails to block local-link/ULA ranges (`fc00::/7` and `fe80::/10`).
    *   **Impact**: Malicious configurations can bypass SSRF controls to probe local networks and internal microservices.
*   **Remediation Recommendation**:
    Resolve hostnames to IP addresses before checking, and validate both IPv4 and IPv6 addresses against private/local ranges:
    ```rust
    // Resolve DNS and check all resolved IPs against local ranges
    ```

---

#### Finding 13: Silent Application Exit on Port Collision (UX/Reliability Bug)
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L234](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L234)
*   **Severity**: High
*   **Root Cause & Logic Analysis**:
    The application checks for duplicate singleton instances via `init_singleton_check()`. If the port is bound by a completely unrelated process, the HTTP check request fails and returns an `Err`. In `lib.rs`, `run()` exits immediately if `init_singleton_check()` fails.
    *   **Impact**: The app exits silently without showing an error popup or log notification to the user, creating a poor user experience.
*   **Remediation Recommendation**:
    Display an error message dialog (e.g., using `tauri::api::dialog::message`) before exiting when the singleton check fails.

---

#### Finding 14: Lost Notification Bug in Background Monitor
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L437](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L437) & [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/handle.rs#L51](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/handle.rs#L51)
*   **Severity**: Medium
*   **Root Cause & Logic Analysis**:
    `PROFILE_SWITCH_NOTIFY` uses `tokio::sync::Notify`. When a profile switch occurs, `notify_waiters()` is called. Unlike `notify_one()`, `notify_waiters()` only wakes up tasks that are currently waiting. If the background monitor is busy doing auto-selection, it is not awaiting `notified()`, meaning the notification is lost.
    *   **Impact**: The background monitor will wait for the full `check_interval` (up to 15s) to detect a profile switch, leading to a long delay before auto-select starts for the new profile.
*   **Remediation Recommendation**:
    Change the notification system to a channel or use `notify_one()` to store a permit:
    ```rust
    PROFILE_SWITCH_NOTIFY.notify_one();
    ```

---

#### Finding 15: Crash/Error on Relative Startup Script Paths
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/init.rs#L451](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/init.rs#L451)
*   **Severity**: Medium
*   **Root Cause & Logic Analysis**:
    When executing startup scripts, `script_dir.parent()` is queried to determine the working directory. If `script_path` is configured as a relative filename with no directory components (e.g., `"script.sh"`), `parent()` returns `None`. The fallback sets `working_dir` to `script_dir` ("script.sh"), which is a file, not a directory.
    *   **Impact**: Calling `.current_dir` with a file path causes `Command::spawn` to fail and crash script execution.
*   **Remediation Recommendation**:
    Ensure the path is canonicalized to an absolute path first:
    ```rust
    let absolute_path = std::fs::canonicalize(&script_path)?;
    ```

---

#### Finding 16: Unbounded Memory Allocation Risk on Subscriptions (Resource Leak/Crash Risk)
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/network.rs#L216](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/network.rs#L216)
*   **Severity**: Medium
*   **Root Cause & Logic Analysis**:
    `response.text().await` reads the entire HTTP body into a `String` without size checking or stream limits.
    *   **Impact**: Downloading a large subscription file or connecting to an infinite stream can allocate memory until an OOM crash occurs.
*   **Remediation Recommendation**:
    Limit the download size (e.g. up to 10MB) by processing the response body as a stream and checking the size:
    ```rust
    // Stream response body and enforce a maximum size limit
    ```

---

### III. Layout, Styling & WebView2 Rendering

#### Finding 17: CSS Specificity Conflict on Dialog/Menu/Popover/Autocomplete Backgrounds under Frosted Glass Skin
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L109-L115](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L109-L115) & [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L184-L190](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L184-L190)
*   **Severity**: High
*   **Root Cause & Logic Analysis**:
    The global safety rule to keep dialogs opaque uses two class selectors (specificity `0, 2, 0`):
    ```scss
    .MuiDialog-paper.theme-panel, ... { background: rgb(...) !important; opacity: 1 !important; }
    ```
    The skin-specific rule `html[data-control-skin="frosted-glass"] .theme-panel` uses an attribute selector, a class selector, and a tag selector (specificity `0, 2, 1`):
    ```scss
    html[data-control-skin="frosted-glass"] {
      .theme-panel { background: rgba(...) !important; ... }
    }
    ```
    Because the skin specificity is higher, dialogs in Frosted Glass mode are rendered with high transparency.
    *   **Impact**: Text underneath the dialog bleeds through, making content unreadable.
*   **Remediation Recommendation**:
    Increase the specificity of the safety rule to override the skin:
    ```scss
    html .MuiDialog-paper.theme-panel { background: rgb(var(--theme-panel-base-rgb, 255, 255, 255)) !important; opacity: 1 !important; }
    ```

---

#### Finding 18: Hardcoded Opaque Background Colors Overriding Theme Skins
*   **Target File Paths**:
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-page.tsx#L37](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-page.tsx#L37)
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/unlock.tsx#L380](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/unlock.tsx#L380)
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L780](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L780)
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/layout-dialogs.tsx#L536](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/layout-dialogs.tsx#L536)
*   **Severity**: Medium
*   **Root Cause & Logic Analysis**:
    Skins (such as Frosted Glass or Cyberpunk) specify unique styles using CSS variables like `--theme-panel-bg` or `--theme-bg`. However, several React components override these variables by hardcoding colors like `#1e1f27` or `#282a36` when `theme.palette.mode === 'dark'`.
    *   **Impact**: In dark mode, these components render as solid gray boxes instead of adopting the active skin's styles, breaking theme coherence.
*   **Remediation Recommendation**:
    Replace hardcoded colors with theme variables:
    ```tsx
    style={{ backgroundColor: 'var(--theme-bg, var(--background-color))' }}
    ```

---

#### Finding 19: Scrollbar Hiding Overrides and Layout Conflict in Standard Scrollbar Property
*   **Target File Paths**:
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx#L45](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx#L45)
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L589](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L589)
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L34](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L34)
*   **Severity**: Low
*   **Root Cause & Logic Analysis**:
    `index.scss` and `use-custom-theme.ts` globally inject a rule forcing a thin scrollbar: `* { scrollbar-width: thin !important; }`. The connection table tries to hide scrollbars using the inline style `scrollbarWidth: 'none'`.
    *   **Impact**: Because the global rule uses `!important`, it overrides the table's inline style, causing a scrollbar to display in the connection table on modern WebView2 engines (Chromium 121+).
*   **Remediation Recommendation**:
    Target the scrollbar rule more specifically or append `!important` to the table inline style.

---

#### Finding 20: Inactive Scrollbar Variables Set in CSS but Unused
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L430](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L430)
*   **Severity**: Low
*   **Root Cause & Logic Analysis**:
    The theme manager sets `--scrollbar-bg` and `--scrollbar-thumb` CSS variables on the root element. However, the injected CSS rules for `::-webkit-scrollbar-thumb` use hardcoded values like `transparent` or `var(--primary-main)`.
    *   **Impact**: Scrollbar style options configured in the settings are ignored.
*   **Remediation Recommendation**:
    Update the CSS rules to use the variables:
    ```css
    ::-webkit-scrollbar-thumb { background: var(--scrollbar-thumb) !important; }
    ```

---

#### Finding 21: Vertical Clipping Hazard in Compact Proxy Columns under High Font Scaling
*   **Target File Paths**:
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx#L239](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx#L239)
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-item.tsx#L101](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-item.tsx#L101)
    *   [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L198](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L198)
*   **Severity**: Low
*   **Root Cause & Logic Analysis**:
    Proxy list items have a locked height of `20px` with `overflow: hidden`. The virtualization list also estimates their height as `20`.
    *   **Impact**: Under high system font scaling (e.g. 150%+), text content scales beyond 20px and is vertically clipped.
*   **Remediation Recommendation**:
    Change fixed heights to min-heights and allow height auto-adjustment, or scale height dynamically based on font scale.

---

#### Finding 22: Navigation Menu Scrollbar Truncation
*   **Target File Path**: [file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/layout.scss#L89](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/layout.scss#L89)
*   **Severity**: Low
*   **Root Cause & Logic Analysis**:
    The navigation menu `.the-menu` has `scrollbar-width: none` and hides scrollbars via `-webkit-scrollbar { width: 0; }`.
    *   **Impact**: Users cannot see if the menu is scrollable, which can lead to hidden options on small screen viewports.
*   **Remediation Recommendation**:
    Show a subtle scrollbar on hover, or use visual indicators (like fades) when scrollable content is hidden.

---

## Conclusion & Verification

This audit highlights the need to resolve:
1.  **Correctness**: Fix the zero-length read bug in the HTTP latency test.
2.  **Concurrency & Stability**: Introduce timeouts to validation scripts and commands, and fix hook dependency arrays.
3.  **UI & Styling**: Resolve theme specificity conflicts and replace hardcoded colors with theme variables to maintain styling consistency.
