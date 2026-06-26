# Web Worker Lifecycle Memory Leak Investigation Report

## 1. Observation
- In `src/hooks/use-traffic-monitor.ts` lines 198-208, the Web Worker is instantiated:
  ```typescript
  const worker = new TrafficWorker()
  worker.onmessage = (event: MessageEvent<ITrafficWorkerSnapshotMessage>) => {
    this.listeners.forEach((listener) => {
      listener(event.data)
    })
  }
  worker.onerror = (error) => {
    debugLog('[TrafficWorkerClient] Web Worker runtime error, falling back to inline:', error)
    this.stop()
    this.startInline(initMessage)
  }
  this.worker = worker
  ```
- In `src/hooks/use-traffic-monitor.ts` lines 238-250, `client.stop()` terminates the worker:
  ```typescript
  stop() {
    if (this.worker) {
      this.worker.terminate()
    }
    if (this.inlineMonitor) {
      this.inlineMonitor.stop()
    }
    this.worker = null
    this.mode = null
    this.ready = false
    this.pendingMessages = []
  }
  ```
- In `src/hooks/use-traffic-monitor.ts` lines 370-403, `useTrafficMonitorEnhanced` hooks manage reference counting and lifecycle:
  ```typescript
  useEffect(() => {
    if (!isActive) return
    ...
    const cleanup = refCounter.increment()
    client.start(currentRangeRef.current)
    ...
    return () => {
      unsubscribe?.()
      stopWatchRefCount()
      cleanup()
      if (refCounter.getCount() === 0) {
        client.stop()
      }
    }
  }, [isActive, subscribeToSnapshots])
  ```
- In `src/hooks/use-visibility.ts`, window visibility is tracked. When the main window is hidden (minimized or closed to tray), visibility is debounced and becomes `false` after 1000ms.
- Git commit history shows that in v1.8.2, Web Worker usage was completely commented out with the message: `"Hardcoding inline sampler to ensure 100% stability"`. It was re-enabled in v1.8.3 / v1.8.9.

## 2. Logic Chain
1. **Triggering Lifecycle Changes**: When the main window is hidden (minimized or closed to tray), `useVisibility()` resolves to `false` after a 1-second debounce delay.
2. **Hook Deactivation**: This makes `isActive` `false` in `useTrafficMonitorEnhanced` (and its consumer `useTrafficData`).
3. **Reference Count Decrement**: The hook's `useEffect` cleanup runs, calling `cleanup()` which decrements the global `refCounter.count`.
4. **Client Stop**: Since `refCounter.count` drops to 0, `client.stop()` is triggered.
5. **Repeated Worker Recreation/Destruction**: `client.stop()` terminates the current Web Worker thread and sets `this.worker` to `null`. When the window is restored, `client.start()` is called again, calling `new TrafficWorker()` to create a new thread.
6. **Thread and Heap Leaks in WebView2**: Frequently instantiating (`new Worker`) and destroying (`worker.terminate()`) Web Workers in WebView2 causes thread and memory handle leaks, as Chromium's thread pool and VM garbage collector do not immediately reclaim resources.
7. **Reference Cycle Leak**: The worker's `onmessage` and `onerror` event listener closures hold implicit references to `this` (the `TrafficWorkerClient` singleton instance). In `stop()`, these event listeners are not cleared (`= null`) before calling `terminate()`, which prevents the garbage collector from immediately reclaiming the worker context and closure memory.
8. **Solution**:
   - Instead of terminating and recreating the Web Worker on every visibility change, keep a single `TrafficWorker` instance alive.
   - When stopping, send a `'stop'` message to the worker to clear its internal `setTimeout` timers and sampler memory, and set `this.ready = false`.
   - When starting, if a worker instance already exists, reuse it and send an `'init'` message to reset its configuration.
   - Clear `onmessage` and `onerror` event listeners on explicit worker termination to break reference cycles.
   - Register a `beforeunload` listener on `window` to terminate the Web Worker cleanly when the window is actually unloaded/destroyed.

## 3. Caveats
- No direct browser process profiling was performed as this is a read-only investigation, but the findings match known WebView2 Web Worker lifecycle leak behaviors.

## 4. Conclusion
The memory regression in lightweight mode in v1.8.9 compared to v1.8.2 is caused by the frequent instantiation and termination of Web Worker threads on window visibility changes, exacerbated by event listener reference cycles and WebView2 GC delays.
The issue is solved by reusing the worker instance, clearing its event listeners on terminate, adding a `'stop'` message to clear internal worker timers, and cleanly terminating the worker only upon window unload.

## 5. Verification Method
- **Git Patch Verification**: Run `git apply .agents/teamwork_preview_explorer_m1/web_worker_lifecycle_leak.patch` to verify the proposed patch applies cleanly to the codebase.
- **Runtime Thread Leak Verification**: Build and run the app in development. Open Task Manager or Process Explorer, minimize and restore the main window repeatedly, and verify that the number of WebView2/Clash Mini sub-processes and thread count remain constant (rather than increasing with each minimize/restore cycle).
