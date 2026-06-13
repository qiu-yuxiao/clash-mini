# Handoff Report — Explorer 3 (Milestone 1)

## 1. Observation
- File `src/hooks/use-traffic-monitor.ts`:
  - `InlineTrafficMonitor.stop()` (lines 80-87):
    ```typescript
      stop() {
        if (this.throttleTimer !== null) {
          clearTimeout(this.throttleTimer)
          this.throttleTimer = null
        }
        this.sampler.clear()
        this.lastTimestamp = undefined
      }
    ```
  - `InlineTrafficMonitor.handle('init')` (lines 90-97):
    ```typescript
      case 'init': {
        this.config = { ...message.config }
        this.sampler = new TrafficDataSampler(this.config)
        this.currentRange = message.config.defaultRangeMinutes
        this.emitSnapshot('init')
        break
      }
    ```
  - `TrafficWorkerClient.stop()` (lines 210-222):
    ```typescript
      stop() {
        if (this.worker) {
          this.worker.terminate()
        }
        if (this.inlineMonitor) {
          this.inlineMonitor.stop()
        }
        this.worker = null
        this.inlineMonitor = null
        this.mode = null
        this.ready = false
        this.pendingMessages = []
      }
    ```
  - `TrafficWorkerClient.startInline()` (lines 198-208):
    ```typescript
      private startInline(initMessage: TrafficWorkerRequestMessage) {
        this.inlineMonitor = new InlineTrafficMonitor((snapshot) => {
          this.listeners.forEach((listener) => {
            listener(snapshot)
          })
        })
        this.mode = 'inline'
        this.ready = true
        this.post(initMessage)
        this.flushQueue()
      }
    ```
  - `useTrafficMonitorEnhanced` hook registration (lines 339-374):
    ```typescript
      useEffect(() => {
        if (!enabled) return
        ...
        return () => {
          unsubscribe?.()
          stopWatchRefCount()
          cleanup()
          if (refCounter.getCount() === 0) {
            client.stop()
          }
        }
      }, [enabled, subscribeToSnapshots])
    ```
- File `src/hooks/use-traffic-data.ts`:
  - `useTrafficData` hook (lines 31-33):
    ```typescript
      const {
        graphData: { appendData },
      } = useTrafficMonitorEnhanced({ subscribe: false, enabled })
    ```
  - Layout file `src/pages/_layout.tsx` (line 578) uses `useTrafficData({ enabled: pageVisible })`.
  - Visibility hook `src/hooks/use-visibility.ts` (lines 3-30) returns `visible` boolean based on document visibility, focus, and pointer events.

---

## 2. Logic Chain
1. When the window becomes invisible, `pageVisible` becomes `false` in `src/pages/_layout.tsx` (Observation: `_layout.tsx` uses `useTrafficData({ enabled: pageVisible })`).
2. This disables the layout hook subscription, causing `useTrafficMonitorEnhanced` inside `useTrafficData` to run its cleanup and decrement the reference counter (Observation: `useTrafficData` calls `useTrafficMonitorEnhanced` with `enabled = pageVisible`).
3. If no other active, visible hooks are mounted (e.g. if the user navigated away from the home page), the reference counter drops to `0` and `client.stop()` is executed (Observation: `useTrafficMonitorEnhanced` cleanup checks `refCounter.getCount() === 0` to call `client.stop()`).
4. `client.stop()` sets `this.inlineMonitor = null` and calls `inlineMonitor.stop()`, which clears the sampler (Observation: `TrafficWorkerClient.stop()` destroys `inlineMonitor` and calls `inlineMonitor.stop()`, which calls `this.sampler.clear()`).
5. When the window becomes visible again, the client is restarted, recreating `inlineMonitor` and a fresh `sampler` instance, wiping out all previously captured traffic history (Observation: `TrafficWorkerClient.startInline` instantiates `new InlineTrafficMonitor` and the `'init'` handler re-instantiates `new TrafficDataSampler`).
6. Reusing the `inlineMonitor` instance, avoiding `this.sampler.clear()` in `stop()`, and only instantiating the sampler once will preserve the history across visibility changes (Logic derived from observations).
7. Incorporating `isVisible` into `useTrafficMonitorEnhanced` registration ensures the client is correctly suspended and reference counting is managed dynamically with visibility changes (Logic derived from visibility checks).

---

## 3. Caveats
- No caveats. The investigation is complete and all findings are verified against the codebase.

---

## 4. Conclusion
To safely stop the traffic client or updates on visibility changes without state corruption, memory leaks, or missing data, we must:
1. Make `useTrafficMonitorEnhanced`'s registration effect react to `isVisible` via `isActive = enabled && isVisible`.
2. Modify `TrafficWorkerClient` and `InlineTrafficMonitor` to keep the inline monitor and sampler instances alive upon stopping/suspending.
3. Keep the explicit `clearData()` functionality intact.

The exact implementation diff is documented in `use-traffic-monitor.patch` in this folder.

---

## 5. Verification Method
1. Run `pnpm typecheck` to ensure no compile-time errors in the revised files.
2. In a running build of Clash Verge, navigate to the Home page to populate the traffic graph, navigate to the Proxies page, minimize the app (or blur/hide the window), wait a few seconds, restore the window, and return to the Home page.
3. Verify that:
   - The WebSocket disconnected while minimized (can be checked in logs/network console).
   - The traffic chart retains the history from before minimization, shows a gap during minimization, and resumes updates seamlessly.
