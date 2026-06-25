# Handoff Report — Memory Usage Regression Investigation

This handoff report summarizes the investigation and compilation of the Clash Mini memory usage regression report comparing v1.8.9 against v1.8.2.

## 1. Observation

- **Task Requirements** (`task.md` lines 12–31):
  - **Title**: Clash Mini Memory Usage Regression Investigation Report (检测分析报告)
  - **Root Cause 1**: Web Worker Lifecycle Leak in `src/hooks/use-traffic-monitor.ts` and `src/hooks/traffic.worker.ts`. Frequent creation/destruction of worker instances, reference cycles formed by event listeners.
  - **Root Cause 2**: Settings Drawer Conditional Rendering Leak in `src/pages/_layout.tsx` and subcomponents, causing Emotion style tag accumulation in `<head>` and MUI event listener/portal leaks.
  - **Codebase Cleanliness Constraint**: "Ensure the codebase remains unmodified (only the report file is created/written)."
  - **Recommendation patches**: Reconstruct Git patches from `bfc5330e` and `41693533`.

- **Web Worker Instantiation and Termination Logic** (`src/hooks/use-traffic-monitor.ts` lines 198–214 & 239–250):
  - Line 198: `const worker = new TrafficWorker()` (creates a new worker instance inside `start`).
  - Lines 239–241: 
    ```typescript
    if (this.worker) {
      this.worker.terminate()
    }
    ```
    (terminates the worker instance inside `stop`).

- **Settings Drawer Conditional Render Logic** (`src/pages/_layout.tsx` lines 1792–1812):
  - Line 1792: `{drawerOpen && !isMiniStatus && (` (conditionally mounts/unmounts settings drawer theme-panel container).

- **Current Repository Status** (`git status --porcelain`):
  - The working directory has no modified source code files. Only `.agents/` metadata and the newly created `docs/memory_regression_report.md` are present.

---

## 2. Logic Chain

1. **Root Cause 1 (Web Worker Lifecycle Leak)**:
   - Observation of `use-traffic-monitor.ts` shows that every time `start()` is triggered on window visibility change, `new TrafficWorker()` is executed, and on visibility loss, `worker.terminate()` is called.
   - Frequent creation and termination cycles in WebView2 leave dangling threads and heap fragments.
   - Strong references from `onmessage` and `onerror` event listeners prevent garbage collection.
   - Therefore, reusing the worker instance, sending a `'stop'` message to clear timers/memory in the worker, clearing event listeners, and terminating only on `beforeunload` directly resolves the thread/handle leaks.

2. **Root Cause 2 (Settings Drawer Conditional Rendering Leak)**:
   - Observation of `_layout.tsx` shows the settings drawer is conditionally mounted via `{drawerOpen && !isMiniStatus && ...}`.
   - When components are repeatedly mounted/unmounted, Emotion injects style tags (`<style data-emotion="...">`) into `<head>` that are never removed.
   - Additionally, MUI interactive components leak event listeners and portal structures on unmount.
   - Under low-power target modes, the JS engine retains strong references to these leaked elements, rendering `SetMemoryUsageTargetLevel(LOW)` ineffective.
   - Reverting to the v1.8.2 CSS translation strategy (keeping the drawer mounted, and using `transform: translate(100%, -100%)` / `pointer-events: none`) avoids style accumulation and unmount leaks entirely.

3. **No Program Code Modification**:
   - The user has mandated that NO program source code files are modified.
   - The final output has been written solely to `docs/memory_regression_report.md`.
   - Running `git status --porcelain` confirms the source tree remains clean.

---

## 3. Caveats

- **External WebView2 Engine Garbage Collection**: The timing of WebView2's garbage collection is determined by the underlying Chromium engine and OS paging configurations. Reclaiming memory via target levels may still have slight delays depending on the OS load.
- **DOM Node Footprint**: While the CSS translation strategy avoids stylesheet injection and event listener leaks, it keeps the drawer DOM node tree permanently in the DOM. This is the intended trade-off to resolve the 50% memory regression as verified in v1.8.2.

---

## 4. Conclusion

The investigation report at `docs/memory_regression_report.md` has been successfully created. It details the root causes (Web Worker lifecycle leaks and Settings Drawer mount leaks), compares the memory overhead between v1.8.2 and v1.8.9, and outlines the precise recommendation patches to restore stability. No source code files have been modified, leaving the codebase in a clean state.

---

## 5. Verification Method

- **Code Cleanliness Verification**:
  Run `git status --porcelain` in the workspace root to ensure no program files (`*.ts`, `*.tsx`, `*.rs`, etc.) are modified.
- **Report Verification**:
  Inspect `docs/memory_regression_report.md` to confirm it contains the Title, Root Cause 1 (Web Worker Lifecycle Leak), Root Cause 2 (Settings Drawer Conditional Rendering Leak), comparison table, and Git patches (`bfc5330e` and `41693533`).
