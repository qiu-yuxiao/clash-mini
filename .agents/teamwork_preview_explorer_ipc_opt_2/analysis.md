# IPC and State Update Audit Report

This report presents the findings of the audit conducted on the ClashVerge TypeScript frontend codebase, specifically targeting high-frequency Tauri IPC listeners, React state hook updates, visibility-based subscriptions, and redundant polling/listener leakages.

---

## 1. High-Frequency IPC and WebSocket Listeners Map

The ClashVerge frontend communicates with the backend core via Tauri's IPC channels and WebSockets exposed by the `tauri-plugin-mihomo-api` npm package. High-frequency operations are mapped below:

| Feature / Hook | Connection Method / Tauri Command | Frequency | Visibility Control | State Update Path |
| :--- | :--- | :--- | :--- | :--- |
| **Traffic Data**<br>`useTrafficData` | `MihomoWebSocket.connect_traffic()` -> `plugin:mihomo\|ws_traffic` | High (Typically 1s) | Pauses when invisible or minimized (`useVisibility`) | Pushes data to `TrafficWorkerClient` (inline sampler) + updates React Query cache. |
| **Connection Data**<br>`useConnectionData` | `MihomoWebSocket.connect_connections()` -> `plugin:mihomo\|ws_connections` | High (Throttled at 16ms / ~60 FPS) | Pauses when invisible or minimized (`useVisibility`) | Merges snapshots with ref stability, then updates React Query cache. |
| **Log Data**<br>`useLogData` | `MihomoWebSocket.connect_logs(level)` -> `plugin:mihomo\|ws_logs` | Variable (Throttled at 50ms flush) | Pauses when invisible or minimized (`useVisibility`) | Local array buffering + 50ms debounced push to React Query cache. |
| **Proxy Delay**<br>`DelayManager` | `delayProxyByName(...)` -> `plugin:mihomo\|delay_proxy_by_name` | On demand / batch (Parallel limit 10) | None | Updates internal map cache + schedules batch item & group listener flushes on `requestAnimationFrame`. |

---

## 2. React State Updates & Canvas Rendering Pipeline

### 2.1 Subscription State Flow (`useMihomoWsSubscription`)
All three high-frequency data streams (traffic, connections, logs) utilize the custom hook `useMihomoWsSubscription`. The flow is structured as follows:
1. **Shared WebSocket Pool**: Connections are shared across subscribers using `sharedSubscriptions` (a module-level `Map`), keeping reference counts (`refs`).
2. **Throttled Updates**: If `throttleMs` is defined, the `next` function is wrapped in a trailing-edge throttle timer:
   - The first message is dispatched immediately.
   - Subsequent messages are captured in `pendingData`.
   - A `setTimeout` of `throttleMs` fires to flush the latest update, capping update frequency.
3. **React Query Cache Integration**: Data updates are written directly to the React Query cache using `queryClient.setQueryData([subscriptionCacheKey], resolved)`.
4. **Subscriber Rerendering**: Components utilize `useQuery` bound to the subscription cache key. Writing to the cache triggers React hook updates and component re-renders.

### 2.2 Traffic Graph Rendering (`EnhancedCanvasTrafficGraph`)
The traffic graph uses an HTML5 Canvas to draw upload and download speeds:
- **Data Hook**: Subscribes to the traffic sampler via `useTrafficGraphDataEnhanced()`.
- **Debounced Processing**: A `useReducer` and a `50ms` `setTimeout` debounce incoming data points into `displayData` to prevent excessive React render passes.
- **Draw Scheduling**: Utilizes `requestAnimationFrame` to schedule canvas redrawing (`drawGraph` and `drawHoverOverlay`) whenever `displayData` or tooltip status changes.
- **Drawing Operations**: Custom Bezier curves (`quadraticCurveTo`) or linear segments (`lineTo`) are painted along with alpha gradients, Y-axis ticks, and grid backgrounds.

---

## 3. Visibility-Based Subscription Pausing

Visibility-based pausing is implemented through the `useVisibility` hook and applied upstream to pause backend events:

### 3.1 How `useVisibility` Works
- **Document Visibility**: Listens to the browser's `visibilitychange` event (tracking `document.visibilityState === 'visible'`). It also hooks into `focus` and `pointerdown` events on `document` to aggressively mark the window as visible.
- **Tauri Window State**: Tracks window minimization via the Tauri window API (`getCurrentWindow()`). It listens to `onResized` and `onFocusChanged` events to update an `isMinimized` state hook.
- **Combined Output**: Returns `documentVisible && !isMinimized`.

### 3.2 Where and How it is Applied
- **Tearing Down WebSockets**: In `useConnectionData`, `useTrafficData`, and `useLogData`, the visibility status controls the subscription cache key:
  ```typescript
  buildSubscriptKey: (date) => (active ? `getClashTraffic-${date}` : null)
  ```
  If `active` (which includes `isVisible`) becomes `false`, the subscription key becomes `null`. This unmounts/disables the React Query query and runs the cleanup function of `useMihomoWsSubscription`'s `useEffect`, decrementing the reference count for the shared WebSocket. If refs reach 0, the WebSocket connection to the Tauri Rust core is closed, stopping all backend event streaming.
- **Pausing Samplers**: In `useTrafficMonitorEnhanced`, the active state controls the lifetime of the `TrafficWorkerClient` and its internal sampling. If visibility is lost, the worker/sampler is stopped.
- **Skipping Canvas Draws**: In `EnhancedCanvasTrafficGraph`, `shouldSkipGraphDraw` checks window focus and visibility. If the window is blurred (when `pause_render_traffic_stats_on_blur` is enabled) or the document is hidden, the canvas redraw is skipped:
  ```typescript
  if (!isDocumentVisibleRef.current) return true
  if (!isWindowFocusedRef.current && pause_render_traffic_stats_on_blur) return true
  ```

---

## 4. Redundant Polling & Listener Leakage Analysis

During the audit, two prominent efficiency and resource leakage issues were identified:

### 4.1 Dead Code `useEffect` Polling Fallback in `useConnectionData`
- **Location**: `src/hooks/use-connection-data.ts`, lines 123-153.
- **Code**:
  ```typescript
  const isWsActive = enabled && isVisible
  useEffect(() => {
    if (isWsActive || !isVisible || !enabled) return
    ...
    // pollTotals using getConnections() every 3000ms
  }, [isWsActive, isVisible, enabled, ...])
  ```
- **Logical Defect**: The early return condition `(isWsActive || !isVisible || !enabled)` simplifies to:
  `if ((enabled && isVisible) || !isVisible || !enabled) return;`
  This logical expression evaluates to `true` for all possible values of `enabled` and `isVisible`. As a result, this `useEffect` **always** returns early.
- **Impact**: The fallback HTTP polling mechanism (which should poll connection statistics every 3 seconds when the WebSocket is inactive but the window is visible) is completely dead and never runs.

### 4.2 Uncleared Timer Leak in `src/services/delay.ts`
- **Location**: `src/services/delay.ts`, lines 220-228.
- **Code**:
  ```typescript
  const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
    setTimeout(() => resolve({ delay: 0 }), timeout)
  })
  const result = await Promise.race([
    delayProxyByName(name, url, timeout),
    timeoutPromise,
  ])
  ```
- **Logical Defect**: A `setTimeout` is scheduled for every single delay test. If the network call to `delayProxyByName` completes successfully before the timeout (e.g., in 50ms, while the timeout is set to 10,000ms), `Promise.race` resolves, but the `setTimeout` timer is **never cleared**.
- **Impact**: The timer remains active in the JavaScript event loop until the full timeout duration expires. During batch delay testing of a proxy group (e.g., 50-100 nodes), this leads to dozens of active timer objects running concurrently in the V8 engine, wasting CPU cycles and memory.

---

## 5. Optimization Strategies

To reduce CPU, GPU, and memory overhead, the following frontend strategies are suggested:

### 5.1 Ref-driven Canvas Rendering (Zero-React-Render Canvas Graph)
- **Current State**: The traffic graph updates React state (`displayData`) on a `50ms` debounce loop, causing the React component to re-render, after which the canvas draws on a `requestAnimationFrame` hook.
- **Optimized Strategy**: Bypass React state and re-renders entirely for canvas drawing.
  - Store incoming data points in a mutable `useRef` array.
  - Let the canvas component maintain a persistent `requestAnimationFrame` loop that directly reads from this ref and paints to the canvas.
  - React will only render the canvas container once upon mounting, eliminating all overhead from Virtual DOM diffing, hook execution, and state propagation during high-frequency streaming.

### 5.2 Dynamic WebSocket Throttling based on UI State
- **Current State**: Connection data is throttled at `16ms` (60 FPS) and log data at `50ms` (20 FPS).
- **Optimized Strategy**:
  - **Connection list throttling**: 60 FPS is unnecessary for connection listings. Increase the connection throttle to `1000ms` or `2000ms`. Humans cannot read list updates at 60 FPS, and rendering tables or lists of connections at high frequency is highly CPU-intensive.
  - **Dynamic Log throttling**: When logs are quiet, a short throttle is fine. However, during high-volume log bursts, increase the log buffering delay dynamically (e.g., from `50ms` to `500ms`) to batch write log items, dramatically reducing React Query cache writes and list re-renders.

### 5.3 Implement Exponential Backoff for WebSocket Reconnection
- **Current State**: `useMihomoWsSubscription` attempts reconnection every `1000ms` indefinitely if the connection fails (e.g., Clash backend is stopped).
- **Optimized Strategy**: Implement an exponential backoff with a jitter factor (e.g., starting at `1000ms`, doubling up to a maximum cap of `30000ms`). This stops console/network spam and saves CPU when the core is offline.

### 5.4 Move Module-Level State to Hook Instance State
- **Current State**: `useTrafficData` tracks the last traffic signature using module-level variables (`lastTrafficSignature` and `lastTrafficTimestamp`).
- **Optimized Strategy**: If multiple traffic listeners are mounted concurrently, they will overwrite each other's signature logs, causing erratic updates. Move this state to hook-specific `useRef` instances.

---

## 6. Implementation Proposals (Before vs. After Code Snippets)

### 6.1 Fix for Delay Test Timer Leak (`src/services/delay.ts`)
#### Before:
```typescript
      // 设置超时处理, delay = 0 为超时
      const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
        setTimeout(() => resolve({ delay: 0 }), timeout)
      })

      // 使用Promise.race来实现超时控制
      const result = await Promise.race([
        delayProxyByName(name, url, timeout),
        timeoutPromise,
      ])
```

#### After:
```typescript
      let timerId: ReturnType<typeof setTimeout> | undefined
      // 设置超时处理, delay = 0 为超时
      const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
        timerId = setTimeout(() => resolve({ delay: 0 }), timeout)
      })

      try {
        // 使用Promise.race来实现超时控制
        const result = await Promise.race([
          delayProxyByName(name, url, timeout),
          timeoutPromise,
        ])
        
        // 确保至少显示500ms的加载动画
        const elapsedTime = Date.now() - startTime
        if (elapsedTime < 500) {
          await new Promise((resolve) => setTimeout(resolve, 500 - elapsedTime))
        }

        const delay = result.delay
        const elapsed = elapsedTime
        debugLog(`[DelayManager] 延迟测试完成，代理: ${name}, 结果: ${delay}ms`)

        return this.setDelay(name, group, delay, { elapsed })
      } finally {
        if (timerId) {
          clearTimeout(timerId)
        }
      }
```

### 6.2 Fix for Connection Fallback Polling (`src/hooks/use-connection-data.ts`)
If connection fallback polling is still desired during WebSocket disconnects:
#### Before:
```typescript
  const isWsActive = enabled && isVisible
  // ...
  useEffect(() => {
    if (isWsActive || !isVisible || !enabled) return
    // ...
```
#### After:
```typescript
  const isWsActive = enabled && isVisible
  // Define a state to track actual connection health
  const [wsConnected, setWsConnected] = useState(false)
  
  // Update wsConnected in setupHandlers (onConnected sets to true, handleMessage errors set to false)
  // ...
  
  useEffect(() => {
    // Poll only if invisible/disabled are false, and the WebSocket is not currently connected
    if (wsConnected || !isVisible || !enabled) return
    // ...
```
Alternatively, remove the entire dead `useEffect` block if fallback HTTP polling is no longer required.
