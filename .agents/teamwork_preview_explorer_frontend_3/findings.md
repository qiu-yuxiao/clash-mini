# Frontend Audit Findings

This document summarizes the findings from the detailed audit of the modified React frontend files in ClashVerge.

---

## 1. Broken Web Worker Reuse & Resource Leak on Error

- **Severity Level**: `Warning` (to `Critical` if worker failures are frequent)
- **File Link**: [use-traffic-monitor.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-monitor.ts)
- **Line References**: Lines 226-230
- **Code Snippet**:
  ```typescript
  worker.onerror = (error) => {
    debugLog('[TrafficWorkerClient] Web Worker runtime error, falling back to inline:', error)
    this.stop()
    this.startInline(initMessage)
  }
  ```
- **Physical Explanation**:
  When a Web Worker encounters a runtime error, `worker.onerror` is fired. The client calls `this.stop()` and falls back to `this.startInline(...)`.
  However, `this.stop()` does not call `worker.terminate()` or clear the `this.worker` reference (setting it to `null`).
  Because `this.worker` remains non-null, when the client is subsequently stopped and restarted (for example, when the application is minimized to system tray and then restored, causing the reference count to drop to 0 and go back up), `start()` executes this block:
  ```typescript
  if (this.worker) {
    this.mode = 'worker'
    this.ready = true
    this.post(initMessage)
    ...
    return
  }
  ```
  It assumes the worker is healthy, sets the mode back to `'worker'`, and attempts to reuse the broken worker instance. This completely breaks the inline fallback logic and leads to a loop of silent failures or repetitive error triggers.
- **Proposed Fix**:
  Terminate the worker and clear its reference in the `onerror` handler:
  ```typescript
  worker.onerror = (error) => {
    debugLog('[TrafficWorkerClient] Web Worker runtime error, falling back to inline:', error)
    if (this.worker) {
      this.worker.onmessage = null
      this.worker.onerror = null
      this.worker.terminate()
      this.worker = null
    }
    this.stop()
    this.startInline(initMessage)
  }
  ```

---

## 2. Leaked Timeout in useEffect (Potential Memory/State Leak)

- **Severity Level**: `Warning`
- **File Link**: [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)
- **Line References**: Line 1034
- **Code Snippet**:
  ```typescript
  useEffect(() => {
    if (!drawerOpen) {
      setTimeout(() => setIsPanelVisible(false), 0)
      return
    }
    ...
  }, [drawerOpen])
  ```
- **Physical Explanation**:
  When `drawerOpen` is falsy, a timeout is scheduled to set `isPanelVisible` to `false` in the next event loop tick. However, the timeout ID is not tracked or cleared in the cleanup function.
  If the layout component unmounts or `drawerOpen` toggles rapidly before the timeout fires, this can attempt to perform state updates on an unmounted component or result in race conditions where the panel visibility state becomes desynchronized.
  This is flagged by ESLint as `@eslint-react/web-api-no-leaked-timeout`.
- **Proposed Fix**:
  Track the timeout ID and clear it in the cleanup function:
  ```typescript
  useEffect(() => {
    let timerId: any = null
    if (!drawerOpen) {
      timerId = setTimeout(() => setIsPanelVisible(false), 0)
      return () => {
        if (timerId) clearTimeout(timerId)
      }
    }
    const element = connectionsPanelRef.current
    if (!element) return

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setIsPanelVisible(entry.contentRect.width > 10)
      }
    })

    observer.observe(element)
    return () => {
      observer.disconnect()
      if (timerId) clearTimeout(timerId)
    }
  }, [drawerOpen])
  ```

---

## 3. Redundant Promise.resolve().then() Deferrals

- **Severity Level**: `Optimization`
- **File Link**: [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)
- **Line References**: Lines 1084, 1086
- **Code Snippet**:
  ```typescript
  useEffect(() => {
    const vPort = verge?.verge_mixed_port
    const cPort = clashInfo?.mixed_port
    if (vPort !== undefined && vPort !== null) {
      Promise.resolve().then(() => setMixedPortVal(vPort))
    } else if (cPort !== undefined && cPort !== null) {
      Promise.resolve().then(() => setMixedPortVal(cPort))
    }
  }, [verge?.verge_mixed_port, clashInfo?.mixed_port])
  ```
- **Physical Explanation**:
  The microtask deferral via `Promise.resolve().then()` is typically used to avoid React's warning about updating state of another component during rendering. However, because this runs inside a `useEffect` hook (which is already executed asynchronously after the render cycle completes), updating state here is safe.
  Wrapping state updates in `Promise.resolve().then()` is redundant and slightly delays the state change.
- **Proposed Fix**:
  Simplify the code to update the state directly:
  ```typescript
  useEffect(() => {
    const vPort = verge?.verge_mixed_port
    const cPort = clashInfo?.mixed_port
    if (vPort !== undefined && vPort !== null) {
      setMixedPortVal(vPort)
    } else if (cPort !== undefined && cPort !== null) {
      setMixedPortVal(cPort)
    }
  }, [verge?.verge_mixed_port, clashInfo?.mixed_port])
  ```

---

## 4. Redundant Time Formatting String Duplication

- **Severity Level**: `Optimization`
- **File Link**: [traffic.worker.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/traffic.worker.ts)
- **Line References**: Line 62
- **Code Snippet**:
  ```typescript
  name: `${new Date(timestamp).getHours().toString().padStart(2, '0')}:${new Date(timestamp).getMinutes().toString().padStart(2, '0')}:${new Date(timestamp).getSeconds().toString().padStart(2, '0')}`
  ```
- **Physical Explanation**:
  The Web Worker manually constructs a `HH:mm:ss` string format for the data point. However, a helper function `formatTrafficName` is already defined in `src/utils/traffic-sampler.ts` which performs exactly the same formatting.
  Since the worker already imports `TrafficDataSampler` from `../utils/traffic-sampler`, it is highly redundant to duplicate this formatting logic.
- **Proposed Fix**:
  Import and reuse `formatTrafficName` in the worker:
  ```typescript
  import { TrafficDataSampler, formatTrafficName } from '../utils/traffic-sampler'
  // ...
  name: formatTrafficName(timestamp)
  ```

---

## 5. Behavioral Inconsistency on stop (Worker vs Inline Monitor)

- **Severity Level**: `Optimization` (to `Warning`)
- **File Links**: 
  - [use-traffic-monitor.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-monitor.ts) (Lines 83-90)
  - [traffic.worker.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/traffic.worker.ts) (Lines 89-100)
- **Code Snippets**:
  - `use-traffic-monitor.ts` (`InlineTrafficMonitor`):
    ```typescript
    stop() {
      if (this.throttleTimer !== null) {
        clearTimeout(this.throttleTimer)
        this.throttleTimer = null
      }
      // Do not clear sampler
      this.lastTimestamp = undefined
    }
    ```
  - `traffic.worker.ts` (`Web Worker`):
    ```typescript
    case 'stop': {
      if (throttleTimer !== null) {
        clearTimeout(throttleTimer)
        throttleTimer = null
      }
      if (sampler) {
        sampler.clear()
        sampler = null
      }
      lastTimestamp = undefined
      break
    }
    ```
- **Physical Explanation**:
  The inline monitor explicitly retains the sampler data during a temporary stop (like when the window is hidden or minimized) to preserve history. However, the Web Worker clears the sampler entirely.
  This causes inconsistent UX: when Web Workers are active, hiding/minimizing the app deletes all traffic history, whereas when falling back to the inline monitor, traffic history is preserved.
- **Proposed Fix**:
  Align the Web Worker stop implementation with the inline monitor. Avoid calling `sampler.clear()` on stop:
  ```typescript
  case 'stop': {
    if (throttleTimer !== null) {
      clearTimeout(throttleTimer)
      throttleTimer = null
    }
    // Do not clear sampler, let it retain history
    lastTimestamp = undefined
    break
  }
  ```
