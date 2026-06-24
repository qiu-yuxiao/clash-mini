# Handoff Report: ClashVerge Frontend Codebase Audit

This report summarizes the findings from the read-only audit of ClashVerge frontend speed test logic, UI rendering issues, and error handling in TS/React (React 19, MUI).

---

## 1. Observation

### Speed Test and Latency Logic
*   **Lack of Cancellation in Speed Tests**:
    *   In [src/services/delay.ts (Line 296)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L296), `DelayManager.checkListDelay` schedules concurrent workers but does not provide an abort controller or cancellation mechanism.
    *   In [src/pages/_layout.tsx (Line 174)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L174), `frontendAutoSelect` invokes `DelayManager.checkListDelay`. If a new auto-select test is triggered, the previous timer is cleared, but the old running promises are not aborted.
*   **Unmounted State Update Warnings**:
    *   In [src/hooks/use-proxy-delay-state.ts (Line 79)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-proxy-delay-state.ts#L79), the hook calls `setDelayState(await delayManager.checkDelay(...))` without checking if the component is still mounted.
    *   In [src/pages/_layout.tsx (Line 303)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L303), the fallback timer callback continues to execute `refreshProxy()` and `setHeadState(...)` (which invokes `setHeadStateForSort` inside the layout component) after the layout unmounts.
    *   In [src/pages/_layout/components/active-node-card.tsx (Line 128)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx#L128), `handleTestDelay` sets `setTesting(false)` inside `finally` after awaiting async operations. Additionally, the mount effect (Line 92) schedules `Promise.resolve().then(() => setNodeAddr(''))` without checking the unmounted/cancelled flag.
    *   In [src/components/proxy/proxy-chain.tsx (Line 332)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-chain.tsx#L332), `handleConnect` triggers `onUpdateChain([])` (Line 364) and `setIsConnecting(false)` (Line 409) after awaiting async API tasks, regardless of unmount state.

### Styling and Viewport Rendering
*   **WebView2 Viewport Horizontal Overflow**:
    *   In [src/pages/_layout.tsx (Line 1584, Line 1615)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1584), elements are styled with `width: '100vw'`.
*   **Hardcoded Absolute Positioning Layout Limits**:
    *   In [src/pages/_layout.tsx (Line 1895, Line 1957)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1895), selectors are absolutely positioned at `bottom: 0` with `zIndex: 200`. The Excel Style Selector has `left: '177.5px'` and `width: '462.5px'`. The Left Settings Column has `width: '240px'`.
*   **MiniTrafficPanel Text Clipping**:
    *   In [src/pages/_layout/components/mini-traffic-panel.tsx (Line 95, Line 164, Line 223, Line 290)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/mini-traffic-panel.tsx#L95), metric containers are locked to `height: '18px'` and `whiteSpace: 'nowrap'` with child text size `fontSize: '13px'`.

### Error Boundaries and Exceptions
*   **Unhandled Promise Rejection Risk in UnlockPage**:
    *   In [src/pages/unlock.tsx (Line 202)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/unlock.tsx#L202), `useEffect` runs:
        ```typescript
        useEffect(() => {
          void (async () => {
            const { items: storedItems, time: storedTime } = loadResultsFromStorage()
            if (storedItems && storedItems.length > 0) {
              setUnlockItems(sortItemsByName(storedItems))
              await getUnlockItems(storedItems, storedTime)
            } else {
              await getUnlockItems()
            }
          })()
        }, ...)
        ```
*   **Global Unhandled Promise Rejection Logger**:
    *   In [src/main.tsx (Line 99)](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/main.tsx#L99), the listener only prints the error in console (`console.error('[main.tsx] Unhandled promise rejection:', event.reason)`) without notifying the UI or updating state.

---

## 2. Logic Chain

### Speed Test Cancellation and Concurrency Races
1. **Observation**: `checkListDelay` runs multiple concurrent asynchronous workers accessing a global `DelayManager` cache, and is not equipped with an abort token.
2. **Observation**: `frontendAutoSelect` starts `checkListDelay` on every run but does not await or terminate existing runs when a new auto-select is requested.
3. **Inference**: If the user re-triggers auto-select, multiple instances of `checkListDelay` will execute concurrently. This leads to duplicate Tauri IPC calls, high CPU load, and race conditions where values written back to cache via `setDelay` clash, causing the auto-select algorithm to read stale or incorrect latencies.

### Unmounted State Updates (React warnings / memory leaks)
1. **Observation**: Event callbacks (`onDelay`, fallback timer, `handleTestDelay`, `handleConnect`) perform asynchronous tasks (`checkDelay`, `getProxyByName`, `healthcheckProxyProvider`, `selectNodeForGroup`) and then call local state setters (`setDelayState`, `setHeadState`, `setTesting`, `setNodeAddr`, `setIsConnecting`) after the `await` statement finishes.
2. **Observation**: No check is made on whether the component is still mounted after the promise resolves.
3. **Inference**: When users switch between pages (e.g. from `/` to `/unlock` or vice versa) during a test, the components unmount. When the pending promises resolve, the setters execute on the unmounted components. This violates React's state update constraints, throwing console warnings and causing memory leaks.

### WebView2 Viewport Horizontal Overflow
1. **Observation**: Layout styles specify `width: '100vw'` on the main container.
2. **Inference**: In WebView2, `100vw` includes the width of the scrollbar. If vertical scrolling is required, the element becomes wider than the viewport client area, producing horizontal scrollbars and rendering glitches during resizing.

### Positioning Collisions and Hardcoded Layout Limits
1. **Observation**: Left Settings Column is `240px` wide. Language/Style selectors are placed at `bottom: 0` with `zIndex: 200`, starting at `left: 177.5px` and spanning `462.5px` width.
2. **Inference**: The selectors will overlap with the right `ConnectionsPanel` container. Since they have a high z-index, they float on top, blocking user clicks and mouse events on the bottom section of the connections table. Furthermore, on narrow screens (e.g. width < 640px), the hardcoded dimensions will exceed the container bounds, getting clipped off-screen and becoming unreachable.

### MiniTrafficPanel Text Clipping
1. **Observation**: Metric cards have a hardcoded `height: '18px'` and text size `13px` with `whiteSpace: 'nowrap'`.
2. **Inference**: Under system-level font scaling (125% or 150%) or zoom, the actual text height exceeds `18px`. Due to the hardcoded height, the text is vertically clipped (descenders are cut off), or overflows/overlaps horizontally.

### Unhandled Promise Rejections in UnlockPage
1. **Observation**: The `useEffect` calls `sortItemsByName(storedItems)` inside an async block with no `try-catch` wrapper.
2. **Observation**: `sortItemsByName` executes `a.name.localeCompare(b.name)`.
3. **Inference**: If local storage is corrupted and contains an item without a `name` property, `localeCompare` throws a TypeError. Since this is in an async wrapper without a try-catch, it bubbles up as an unhandled promise rejection. Since the global handler only logs it, the application fails to display an error notification or recovery UI.

---

## 3. Caveats

*   **Tauri API Boundaries**: The audit is strictly frontend-focused (TS/React). The behavior of backend APIs (`delayProxyByName`, `healthcheckProxyProvider`, `getProxyByName`) is assumed to match the documented interfaces.
*   **OS/Webview Zoom Configurations**: Different Windows OS versions and WebView2 runtimes handle default font scaling and viewport calculations differently; some rendering issues (e.g. scrollbar flickering) may only manifest on specific Zoom/DPR ratios.

---

## 4. Conclusion

The ClashVerge frontend contains several logic flaws, state update issues, and styling anomalies:
1.  **Concurrency Vulnerability**: Lacking abort capability on speed test tasks leads to race conditions and duplicate tests.
2.  **Unmounted State updates**: Components such as `ProxyGroups`, `ActiveNodeStatusCard`, and `ProxyChain` fail to guard state updates against unmount.
3.  **MUI/CSS Layout Flaws**: Utilizing `100vw` causes viewport clipping; hardcoded dimensions in the selectors lead to layout collision and clipping on narrow screens; and hardcoded `18px` metric heights cause text clipping under font scaling.
4.  **Error Handling gaps**: Uncaught exceptions in `UnlockPage`'s `useEffect` lead to unhandled promise rejections.

---

## 5. Verification Method

To verify these findings:

1.  **Code Inspection**:
    *   Inspect `checkListDelay` at [src/services/delay.ts#L296](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L296) to confirm the lack of cancellation tokens.
    *   Trace the `useEffect` block in `src/pages/unlock.tsx` [src/pages/unlock.tsx#L202](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/unlock.tsx#L202) to confirm the lack of error catch block around `sortItemsByName`.
    *   Check absolute positioning and layout dimensions in `src/pages/_layout.tsx` [src/pages/_layout.tsx#L1895](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1895).
2.  **Runtime Verification (Console Logs)**:
    *   Trigger auto-selection and immediately switch pages to `/unlock` to check for React unmounted state update warnings in the Webview console.
    *   Inject malformed data (e.g. `[{"status": "Pending"}]`) into the `clash_verge_unlock_results` local storage item and reload the page to trigger the `UnlockPage` unhandled promise rejection.
3.  **Visual Verification**:
    *   Open Settings Drawer and inspect overlap between the selectors at `bottom: 0` and the `ConnectionsPanel` at a viewport height > 830px.
    *   Change system display scaling to 150% and inspect if text inside `MiniTrafficPanel` metrics (download/upload speed) gets vertically clipped.
