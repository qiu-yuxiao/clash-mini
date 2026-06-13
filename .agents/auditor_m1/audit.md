## Forensic Audit Report

**Work Product**: Tauri Window Visibility & WebSocket Subscription Lifecycle Optimizations
**Profile**: General Project
**Verdict**: CLEAN

### Phase Results
- **Hardcoded Output Detection**: PASS — Analyzed target hooks; no hardcoded test results, mock behaviors, or bypassed logic were found. State variables and functions resolve dynamically.
- **Facade Detection**: PASS — Code changes implement actual Tauri window events (`currentWindow.onResized`, `currentWindow.onFocusChanged`) and conditionally close/suspend active WebSocket subscriptions via TanStack Query and custom subscription lifecycle hooks.
- **Fabricated Verification Output**: PASS — No pre-populated logs, fake test runners, or simulated test results exist in the codebase.
- **Dependency Audit / Execution Delegation**: PASS — Core logic uses `@tauri-apps/api/window` and React standard APIs appropriately without delegating functionality to unauthorized pre-built wrappers or mocking.
- **Tauri Integration & Safety**: PASS — Window check APIs are wrapped in try-catch blocks to prevent crashes/failures in non-Tauri, browser, or testing environments.
- **State Integrity & Memory Leaks Check**: PASS — Cleanup functions unsubscribe listeners, reset reference counts, and clear timer objects when component unmounts or active state changes.

### Evidence

#### 1. `src/hooks/use-visibility.ts` Diffs:
```diff
@@ -1,19 +1,21 @@
+import { getCurrentWindow } from '@tauri-apps/api/window'
 import { useEffect, useState } from 'react'
 
 export const useVisibility = () => {
-  const [visible, setVisible] = useState(() =>
+  const [documentVisible, setDocumentVisible] = useState(() =>
     typeof document === 'undefined'
       ? true
       : document.visibilityState === 'visible',
   )
+  const [isMinimized, setIsMinimized] = useState(false)
 
   useEffect(() => {
     const handleVisibilityChange = () => {
-      setVisible(document.visibilityState === 'visible')
+      setDocumentVisible(document.visibilityState === 'visible')
     }
 
-    const handleFocus = () => setVisible(true)
-    const handlePointerDown = () => setVisible(true)
+    const handleFocus = () => setDocumentVisible(true)
+    const handlePointerDown = () => setDocumentVisible(true)
 
     document.addEventListener('focus', handleFocus)
     document.addEventListener('pointerdown', handlePointerDown)
@@ -26,5 +28,67 @@ export const useVisibility = () => {
     }
   }, [])
 
-  return visible
+  useEffect(() => {
+    let active = true
+    let unlistenResized: (() => void) | null = null
+    let unlistenFocus: (() => void) | null = null
+
+    const initTauri = async () => {
+      try {
+        const currentWindow = getCurrentWindow()
+        const minimized = await currentWindow.isMinimized()
+        if (active) {
+          setIsMinimized(minimized)
+        }
+
+        const unR = await currentWindow.onResized(async () => {
+          try {
+            const min = await currentWindow.isMinimized()
+            if (active) {
+              setIsMinimized(min)
+            }
+          } catch {
+            // ignore
+          }
+        })
+        if (active) {
+          unlistenResized = unR
+        } else {
+          unR()
+        }
+
+        const unF = await currentWindow.onFocusChanged(async () => {
+          try {
+            const min = await currentWindow.isMinimized()
+            if (active) {
+              setIsMinimized(min)
+            }
+          } catch {
+            // ignore
+          }
+        })
+        if (active) {
+          unlistenFocus = unF
+        } else {
+          unF()
+        }
+      } catch {
+        // Fallback for non-Tauri / browser / testing environments
+      }
+    }
+
+    initTauri()
+
+    return () => {
+      active = false
+      if (unlistenResized) {
+        unlistenResized()
+      }
+      if (unlistenFocus) {
+        unlistenFocus()
+      }
+    }
+  }, [])
+
+  return documentVisible && !isMinimized
 }
```

#### 2. `src/hooks/use-traffic-data.ts` Diffs:
```diff
@@ -27,13 +28,15 @@ const shouldSkipDuplicateTraffic = (traffic: Traffic) => {
 
 export const useTrafficData = (options?: { enabled?: boolean }) => {
   const enabled = options?.enabled ?? true
+  const isVisible = useVisibility()
+  const active = enabled && isVisible
 
   const {
     graphData: { appendData },
-  } = useTrafficMonitorEnhanced({ subscribe: false, enabled })
+  } = useTrafficMonitorEnhanced({ subscribe: false, enabled: active })
   const { response, refresh } = useMihomoWsSubscription<ITrafficItem>({
     storageKey: 'mihomo_traffic_date',
-    buildSubscriptKey: (date) => (enabled ? `getClashTraffic-${date}` : null),
+    buildSubscriptKey: (date) => (active ? `getClashTraffic-${date}` : null),
     fallbackData: FALLBACK_TRAFFIC,
```

#### 3. `src/hooks/use-log-data.ts` Diffs:
```diff
@@ -54,12 +55,14 @@ export const useLogData = () => {
   const logLevel = clashLog?.logLevel ?? 'info'
   const allowedTypes = LOG_LEVEL_FILTERS[logLevel] ?? DEFAULT_LOG_TYPES
   const hasLoadedInitialLogsRef = useRef(false)
+  const isVisible = useVisibility()
+  const active = enableLog && isVisible
 
   const { response, refresh, subscriptionCacheKey } = useMihomoWsSubscription<
     ILogItem[]
   >({
     storageKey: 'mihomo_logs_date',
-    buildSubscriptKey: (date) => (enableLog ? `getClashLog-${date}` : null),
+    buildSubscriptKey: (date) => (active ? `getClashLog-${date}` : null),
     fallbackData: [],
```

#### 4. Typecheck Verification:
TypeScript compiles successfully (`pnpm typecheck` returns 0 compile errors).
Linter checks on modified files show zero lint errors or warnings.
