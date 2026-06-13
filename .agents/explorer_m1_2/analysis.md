# Analysis: Conditional WebSocket Connections based on Window Visibility

This report analyzes the WebSocket subscriptions for traffic, connections, and logs in Clash Mini (Clash Verge), and outlines a precise strategy to conditionally connect or disconnect these subscriptions based on the application's visibility state (`isVisible`).

---

## 1. Underlying WebSocket Subscription Engine: `useMihomoWsSubscription`
All high-frequency WebSockets in the application are managed by `useMihomoWsSubscription` located in `src/hooks/use-mihomo-ws-subscription.ts`.

### Lifecycle and Caching Mechanism:
- **Sharing & Reference Counting**: It tracks active subscriptions in a global `sharedSubscriptions` map (`Map<string, SharedSubscriptionEntry>`). When a hook is mounted, it increments `entry.refs`. When unmounted, it decrements `entry.refs`.
- **Socket Disconnection**: If the reference count `refs` reaches `0`, it marks the entry as closed, deletes it from the shared map, and invokes `closeSharedSocket(entry)` to close the WebSocket client:
  ```typescript
  entry.refs -= 1
  if (entry.refs <= 0) {
    entry.closed = true
    if (entry.reconnectTimer) {
      clearTimeout(entry.reconnectTimer)
      entry.reconnectTimer = null
    }
    sharedSubscriptions.delete(subscriptionCacheKey)
    void closeSharedSocket(entry)
  }
  ```
- **Conditional Subscription Control via `null` Key**: If the option `buildSubscriptKey(date)` returns `null`, the hook evaluates `subscriptionCacheKey` as `null`. Under this condition, the `useEffect` block responsible for setting up the WebSocket connection returns immediately:
  ```typescript
  useEffect(() => {
    if (!subscriptionCacheKey) return
    // WebSocket connection setup occurs here...
  }, [subscriptionCacheKey])
  ```
- **Consequence**: When `subscriptionCacheKey` transitions from a valid string to `null`, the React `useEffect` clean-up runs, decrementing the reference count for the previous key, closing the WebSocket if references reach `0`. No new connection is established for the `null` key. When transitioning back from `null` to a valid string, the `useEffect` runs again and opens a new connection.

---

## 2. Analysis of the Target Hooks

### A. Connection Data Hook (`useConnectionData` in `src/hooks/use-connection-data.ts`)
- **Status**: **Fully Integrated & Optimized**.
- **Implementation**: It already uses `useVisibility()` to determine connection state and REST polling:
  - **WebSocket Connection**: It computes `isWsActive = enabled && isVisible`. If `isWsActive` is false, the subscription key is set to `null`, triggering disconnection.
  - **REST Polling**: It runs low-frequency (3-second) totals polling when the connection panel is closed but the window is visible. If the window becomes invisible (`!isVisible`), the polling `useEffect` returns immediately and does not register a timer:
    ```typescript
    useEffect(() => {
      if (isWsActive || !isVisible) return // Bypasses polling when window is hidden
      // 3s pollTotals setup...
    }, [isWsActive, isVisible, queryClient, subscriptionCacheKey])
    ```
- **Conclusion**: No modification is needed for `useConnectionData`.

---

### B. Traffic Data Hook (`useTrafficData` in `src/hooks/use-traffic-data.ts`)
- **Status**: **Partially Dependent on Caller**.
- **Current Behavior**:
  - Accepts an optional `enabled` property from the caller (`options?.enabled ?? true`).
  - Calls `useTrafficMonitorEnhanced({ subscribe: false, enabled })`.
  - Sets the subscription key via: `buildSubscriptKey: (date) => (enabled ? \`getClashTraffic-\${date}\` : null)`.
  - If `options?.enabled` is omitted or passed as `true`, it remains active regardless of window visibility.
- **Proposed Modification**:
  Import `useVisibility` and conditionally disable the hook internally if the window is hidden. This guarantees that traffic monitoring stops even if the parent component forgets to pass `pageVisible`.
- **Proposed Code Change**:
  ```typescript
  // Before:
  export const useTrafficData = (options?: { enabled?: boolean }) => {
    const enabled = options?.enabled ?? true
    
  // After:
  import { useVisibility } from '@/hooks/use-visibility'
  // ...
  export const useTrafficData = (options?: { enabled?: boolean }) => {
    const isVisible = useVisibility()
    const enabled = (options?.enabled ?? true) && isVisible
  ```

---

### C. Log Data Hook (`useLogData` in `src/hooks/use-log-data.ts`)
- **Status**: **Not Visibility-Aware**.
- **Current Behavior**:
  - Uses `enableLog = clashLog?.enable ?? true` to decide whether to connect the WebSocket.
  - Does not query `useVisibility()`. If the window is minimized or hidden, the WebSocket remains open, continuing to receive, parse, and buffer logs.
- **Proposed Modification**:
  Import `useVisibility` and combine the log configuration state with window visibility (`isVisible`).
- **Proposed Code Change**:
  ```typescript
  // Before:
  export const useLogData = () => {
    const queryClient = useQueryClient()
    const [clashLog] = useClashLog()
    const enableLog = clashLog?.enable ?? true
  
  // After:
  import { useVisibility } from '@/hooks/use-visibility'
  // ...
  export const useLogData = () => {
    const queryClient = useQueryClient()
    const [clashLog] = useClashLog()
    const isVisible = useVisibility()
    const enableLog = (clashLog?.enable ?? true) && isVisible
  ```

---

## 3. Detailed Strategy for Implementation

### Proposed Code Diffs

#### File: `src/hooks/use-traffic-data.ts`
```diff
diff --git a/src/hooks/use-traffic-data.ts b/src/hooks/use-traffic-data.ts
index e234567..f789abc 100644
--- a/src/hooks/use-traffic-data.ts
+++ b/src/hooks/use-traffic-data.ts
@@ -1,5 +1,6 @@
 import { MihomoWebSocket, Traffic } from 'tauri-plugin-mihomo-api'
 
+import { useVisibility } from '@/hooks/use-visibility'
 import { useMihomoWsSubscription } from './use-mihomo-ws-subscription'
 import { useTrafficMonitorEnhanced } from './use-traffic-monitor'
 
@@ -26,3 +27,4 @@ const shouldSkipDuplicateTraffic = (traffic: Traffic) => {
 
 export const useTrafficData = (options?: { enabled?: boolean }) => {
-  const enabled = options?.enabled ?? true
+  const isVisible = useVisibility()
+  const enabled = (options?.enabled ?? true) && isVisible
 
   const {
```

#### File: `src/hooks/use-log-data.ts`
```diff
diff --git a/src/hooks/use-log-data.ts b/src/hooks/use-log-data.ts
index f123456..a789bcd 100644
--- a/src/hooks/use-log-data.ts
+++ b/src/hooks/use-log-data.ts
@@ -5,4 +5,5 @@ import { MihomoWebSocket, type LogLevel } from 'tauri-plugin-mihomo-api'
 
 import { getClashLogs } from '@/services/cmds'
+import { useVisibility } from '@/hooks/use-visibility'
 
 import { useClashLog } from './use-clash-log'
@@ -50,5 +51,6 @@ export const useLogData = () => {
   const queryClient = useQueryClient()
   const [clashLog] = useClashLog()
-  const enableLog = clashLog?.enable ?? true
+  const isVisible = useVisibility()
+  const enableLog = (clashLog?.enable ?? true) && isVisible
   const logLevel = clashLog?.logLevel ?? 'info'
```

---

## 4. Verification and Safety Assessment
1. **Reconnection Verification**:
   When the window transitions from hidden to visible, `isVisible` becomes `true`. The subscription key shifts from `null` to its valid cache key. The `useEffect` inside `useMihomoWsSubscription` triggers, creating a new shared entry and successfully establishing the WebSocket connection again.
2. **Resource Cleanup**:
   Tauri WebSocket connections are closed via the Rust side when `.close()` is called on the client. Our strategy triggers the clean-up directly from React hooks, ensuring that WebSocket connections do not linger in the background.
3. **Log Cache Retention**:
   When the log WebSocket is disconnected due to visibility loss, the existing log entries in the react-query cache are preserved (thanks to `responseCacheKey` falling back to the last cache key). When the window is visible again, new logs continue appending from the new WebSocket connection, keeping the history intact (with a natural gap corresponding to the hidden duration).
