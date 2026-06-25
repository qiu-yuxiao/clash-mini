# Clash Verge/Mini Code Audit Report: Commits fd26ae0a to 47877a1e

This report presents a detailed code audit of the commits in the range `fd26ae0a` to `47877a1e` for the Clash Verge/Mini codebase.

---

## 1. Overview of Commits After Version 1.8.5

The audited commit range consists of 9 commits following version 1.8.5:
1. **`fd26ae0a`**: Bumped version to 1.8.6 and fixed latency display for provider-managed proxies after waking up from lightweight mode.
2. **`791f857e`**: Optimized resources in mini window mode by unmounting `ProxyGroups` when `isMiniStatus` is true.
3. **`351e073c`**: Updated updater configuration for v1.8.6.
4. **`78102584`**: Shifted settings drawer state `isSettingsOpen` to global `SystemContext` and implemented conditional rendering to unmount the drawer when closed, automatically disabling `getSystemProxy` and `getRunningMode` queries.
5. **`2a2fad90`**: Formatted Rust network monitor module and fixed eslint dependency.
6. **`fd246690`**: Resolved startup race condition for `isMiniStatus`/`isMinimalWidth` by defaulting them to `false` and using `setTimeout(..., 0)` to perform the initial size evaluation.
7. **`588cfa47`**: Added `focus` and `visibilitychange` listeners to compensate for tray silent startup WebView 0x0 size bugs, and added synchronous size evaluations to the window provider's idle timer.
8. **`4243096d`**: Bumped version to 1.8.7.
9. **`47877a1e`**: Updated `Cargo.lock` for version 1.8.7.

---

## 2. Audit of `focus` and `visibilitychange` Event Listeners

### Locations
- [src/pages/_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L503-L523)
- [src/providers/app-data-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/app-data-provider.tsx#L64-L87)

### Analysis
- **Unmount Removal**: Both files properly register the listeners on mount and return cleanup functions that call `removeEventListener` for all registered events (`resize`, `focus`, and `visibilitychange`), as well as `clearTimeout` for the startup timer.
- **Memory Leaks**: No memory leaks exist. The listener callback `handleResize` is defined inside `useEffect` with an empty dependency array `[]`. It only captures state setters which are stable, so it does not retain changing values or cause closures to leak.
- **Double-Triggering of Queries**: 
  - **Normal Operation**: No double-triggering occurs when focusing or switching visibility, because React bails out of state updates when the width/height thresholds are not crossed.
  - **Silent Startup (Tray Startup) & Mini Startup**: **Yes**. Because `isMiniStatus` defaults to `false` (full mode), the query `getProxies` runs immediately in full mode. Shortly after, the `setTimeout(..., 0)` runs, detects that the window is hidden/0x0 (or mini), and sets `isMiniStatus` to `true`. This state transition triggers `useEffect` to refetch the proxies (in minimal mode). This causes two consecutive queries (`getProxies` in full mode, followed by minimal mode) at startup.

### Proposed Resolution
To prevent the double-trigger of the `getProxies` query at startup, initialize `isMiniStatus` and `isMinimalWidth` synchronously using `window.innerWidth` and `window.innerHeight`. Because we already have `setTimeout` and compensations, any initial 0x0 size at normal startup will be safely corrected without causing redundant queries.

#### Git Diff Proposal for `src/pages/_layout.tsx`
```diff
diff --git a/src/pages/_layout.tsx b/src/pages/_layout.tsx
index 1cd6c302..a1b2c3d4 100644
--- a/src/pages/_layout.tsx
+++ b/src/pages/_layout.tsx
@@ -502,9 +502,19 @@ const Layout = () => {
-  const [isMinimalWidth, setIsMinimalWidth] = useState(false)
+  const [isMinimalWidth, setIsMinimalWidth] = useState(() => {
+    if (typeof window !== 'undefined') {
+      return window.innerWidth <= 285
+    }
+    return false
+  })
 
-  const [isMiniStatus, setIsMiniStatus] = useState(false)
+  const [isMiniStatus, setIsMiniStatus] = useState(() => {
+    if (typeof window !== 'undefined') {
+      return window.innerWidth <= 285 && window.innerHeight <= 100
+    }
+    return false
+  })
```

#### Git Diff Proposal for `src/providers/app-data-provider.tsx`
```diff
diff --git a/src/providers/app-data-provider.tsx b/src/providers/app-data-provider.tsx
index 2159bda1..b2c3d4e5 100644
--- a/src/providers/app-data-provider.tsx
+++ b/src/providers/app-data-provider.tsx
@@ -63,9 +63,19 @@ export const AppDataProvider = ({
-  const [isMinimalWidth, setIsMinimalWidth] = useState(false)
+  const [isMinimalWidth, setIsMinimalWidth] = useState(() => {
+    if (typeof window !== 'undefined') {
+      return window.innerWidth <= 285
+    }
+    return false
+  })
 
-  const [isMiniStatus, setIsMiniStatus] = useState(false)
+  const [isMiniStatus, setIsMiniStatus] = useState(() => {
+    if (typeof window !== 'undefined') {
+      return window.innerWidth <= 285 && window.innerHeight <= 100
+    }
+    return false
+  })
```

---

## 3. Audit of Resource Optimization: Unmounting `ProxyGroups` & Settings Drawer

### Locations
- [src/pages/_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1778-L1790) (ProxyGroups unmounting)
- [src/pages/_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1791-L1812) (Settings Drawer conditional rendering)

### Analysis
- **Unmounting ProxyGroups**:
  - **Dangling State / Re-renders**: No dangling global state or memory leaks. The query hooks inside `useRenderList` (used by `ProxyGroups`) clean up their timers and listeners correctly when unmounted.
  - **UX Caveat / Layout Bug**: Unmounting `ProxyGroups` completely discards its local states, such as scroll position and active proxy group selections. Resizing the window back to normal resets `ProxyGroups` back to the default state, which is a minor usability issue. However, this is required by Design Agreement 39 (BUG-240).
- **Conditional Rendering of Settings Drawer**:
  - **Dangling State**: No dangling state; WebSocket subscriptions (via `useConnectionData`) are paused/closed correctly because unmounting sets the connections panel width to 0, which updates `isPanelVisible` to `false` and disables the query.
  - **Layout/UI Bug in Mini Window Mode**: **Yes**. The Settings Drawer is rendered via `{drawerOpen && ...}`. However, it is **not** conditioned on `!isMiniStatus`. If a user goes to mini window mode while the settings drawer is open (or starts the app in mini mode with `drawerOpen: true`), the settings drawer remains mounted. It overlays the tiny 270x80 screen, making the traffic graph invisible and the UI unusable. Furthermore, it triggers connection data WebSocket polling because the drawer is mounted and its connections panel gets width > 10px in the flex layout, wasting substantial CPU and networking resources.

### Proposed Resolution
Condition the settings drawer rendering on `!isMiniStatus` as well. This guarantees it unmounts in mini mode, saving system resources and preventing rendering glitches.

#### Git Diff Proposal
```diff
diff --git a/src/pages/_layout.tsx b/src/pages/_layout.tsx
index 1cd6c302..c3d4e5f6 100644
--- a/src/pages/_layout.tsx
+++ b/src/pages/_layout.tsx
@@ -1791,3 +1791,3 @@ const Layout = () => {
             {/* Settings Drawer (Instantly mounted when drawerOpen is true) */}
-            {drawerOpen && (
+            {drawerOpen && !isMiniStatus && (
               <div
```

---

## 4. Audit of `resetIdleTimer` in `src/providers/window/window-provider.tsx`

### Location
- [src/providers/window/window-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/window/window-provider.tsx#L13) (`MINIMAL_WIDTH_THRESHOLD`)
- [src/providers/window/window-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/window/window-provider.tsx#L125-L144) (`resetIdleTimer` function)

### Analysis
- **Memory Leaks with Timers**: No memory leaks exist. The old timer in `idleTimerRef.current` is cleared before setting a new one, and the timer is cleared inside the `useEffect` cleanup function on unmount.
- **Out-of-Sync Conditions**:
  - **Threshold Mismatch Bug**: **Yes**. `window-provider.tsx` defines `MINIMAL_WIDTH_THRESHOLD = 290`, whereas `_layout.tsx` and `app-data-provider.tsx` check width against `285` (极窄模式). If the window width is between 286px and 290px (e.g., 288px), `window-provider.tsx` considers the window to be in minimal width and hides decorations when idle. However, the UI thinks it is in normal width and renders the full UI. This leads to a full-size window with hidden native window decorations, leaving the user with no close/minimize buttons or title bar.
  - **Stale Ref in Timeout Callback**: **Yes**. `checkMaximized` (the resize event listener) is debounced by 300ms. If the idle timer fires during this 300ms window, `isMinimalWidthRef.current` will be stale. The idle timer will hide the native window chrome based on stale size information.

### Proposed Resolution
1. Align `MINIMAL_WIDTH_THRESHOLD` in `window-provider.tsx` to `285` to match the rest of the application.
2. Query `window.innerWidth` dynamically inside the `setTimeout` callback to get the live, accurate window width.

#### Git Diff Proposal
```diff
diff --git a/src/providers/window/window-provider.tsx b/src/providers/window/window-provider.tsx
index 83bc28ee..d4e5f6a7 100644
--- a/src/providers/window/window-provider.tsx
+++ b/src/providers/window/window-provider.tsx
@@ -13,3 +13,3 @@
 /** Width threshold (CSS px) below which the window is in "traffic monitor" mode */
-const MINIMAL_WIDTH_THRESHOLD = 290
+const MINIMAL_WIDTH_THRESHOLD = 285
 
@@ -132,4 +132,5 @@ export const WindowProvider: React.FC<{ children: React.ReactNode }> = ({
     idleTimerRef.current = setTimeout(async () => {
       // Only hide if currently at minimal width and not already hidden
-      if (!isMinimalWidthRef.current || isDecorationsHiddenRef.current) return
+      const currentIsMinimal = typeof window !== 'undefined' ? window.innerWidth <= MINIMAL_WIDTH_THRESHOLD : false
+      if (!currentIsMinimal || isDecorationsHiddenRef.current) return
       try {
```

---

## 5. Audit of Waking Up Latency Display Fix in `src/services/delay.ts`

### Location
- [src/services/delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L199-L211) (`getDelayFix` function)

### Analysis
- **Root Cause & Fix**: The deletion of `!proxy.provider` is correct. Previously, if a proxy node had a provider (which is true for all provider-managed nodes in normal mode after waking up), `getDelayFix` would skip checking `getDelayUpdate` (the local cache of `DelayManager` updated by speed tests) and return the stale history delay. This caused speed test updates to fail to show up on the UI after waking up.
- **Leaks / Issues**: None. Cache expiration is managed by a `setInterval` running every 2 hours in the constructor, which deletes expired items (TTL = 30 min). This prevents memory bloat.

---

## Summary of Recommendations

| Issue / Mismatch | Root Cause | Proposed Solution | Impact |
| --- | --- | --- | --- |
| Double-triggering of queries at startup | Defaulting `isMiniStatus` to `false` causes initial query, then `setTimeout` sets to `true` (silent startup) triggering a refetch. | Initialize state synchronously using `window.innerWidth <= 285`. | Eliminates 1 duplicate query at startup. |
| Settings Drawer visible and active in Mini Mode | Drawer is conditionally rendered using `drawerOpen` instead of `drawerOpen && !isMiniStatus`. | Condition rendering on `!isMiniStatus` in `_layout.tsx`. | Saves CPU/network by disabling WebSockets in mini mode; fixes layout overlays. |
| Hidden decorations on full UI (width 286-290px) | `window-provider.tsx` uses 290px threshold, while React components use 285px. | Change `MINIMAL_WIDTH_THRESHOLD` in `window-provider.tsx` to `285`. | Resolves out-of-sync titlebar hiding bug. |
| Chrome hiding based on stale size data | Debouncing of size updates causes the timer callback to check a stale ref. | Query `window.innerWidth` dynamically inside the timer callback. | Prevents incorrect chrome hiding during active resizes. |
