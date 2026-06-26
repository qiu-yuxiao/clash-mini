# Task: Compile Memory Usage Regression Report

## Objective
Write a comprehensive investigation report at `docs/memory_regression_report.md` detailing the root causes, mechanisms, code locations, comparisons, and recommendation patches for the memory usage regression in Clash Mini v1.8.9 compared to v1.8.2.

## Working Directory
`.agents/teamwork_preview_worker_m3/`

## Requirements
You must create `docs/memory_regression_report.md` using the exact layout and content details synthesized from the Explorer findings:

1. **Title**: Clash Mini Memory Usage Regression Investigation Report (检测分析报告)
2. **Root Cause 1: Web Worker Lifecycle Leak**
   - **File names/functions/line ranges**: `src/hooks/use-traffic-monitor.ts` (specifically `TrafficWorkerClient.start`, `stop`, `useTrafficMonitorEnhanced` hook, `ReferenceCounter`), `src/hooks/traffic.worker.ts`.
   - **Root Cause & Mechanism**: The frequent creation (`new Worker`) and destruction (`worker.terminate()`) on window visibility change (debounced by 1s in `useVisibility`) leaves dangling threads, uncollected JavaScript heap fragments, and unclosed event listeners in WebView2. The event listeners `onmessage` and `onerror` form reference cycles with the global `TrafficWorkerClient` singleton, which delays garbage collection in WebView2.
   - **Comparison (v1.8.2 vs v1.8.9)**: In v1.8.2, Web Worker usage was completely disabled ("Hardcoding inline sampler to ensure 100% stability"), while v1.8.9 re-introduced it. Repeated instantiation leads to process and handle leaks.
   - **Code Patch**: Provide the Git patch from `bfc5330e` to reuse the worker, handle `'stop'` message to clear timers/memory, clear event listeners on terminate, and terminate on `beforeunload`.
3. **Root Cause 2: Settings Drawer Conditional Rendering Leak**
   - **File names/functions/line ranges**: `src/pages/_layout.tsx` (specifically line 1792 `{drawerOpen && !isMiniStatus && ...}`), and drawer subcomponents: `ProfileImportCard`, `BasicSettingsCard`, `ThemeSettingsCard`, `ConnectionsPanel`, `HelpMenuButton`, `TakeoverModeCard`, `RoutingPreferenceCard`.
   - **Root Cause & Mechanism**: Conditionally mounting the Settings Drawer leads to Emotion stylesheet accumulation in the `<head>` tag because style tags injected dynamically by Emotion/MUI during component mount are not purged upon unmount. In addition, nested MUI components (like Select, Tooltip) leak event listeners/portals if unmounted/mounted repeatedly. When window is closed to tray, `SetMemoryUsageTargetLevel(LOW)` cannot reclaim this leaked memory because of strong references in the JS engine.
   - **Comparison (v1.8.2 vs v1.8.9)**: v1.8.2 used a CSS translation strategy (`transform: translate(100%, -100%)`) which kept components mounted exactly once at startup. This avoids stylesheet accumulation and listener leak cycles. Gating connections and WebSocket updates via `drawerOpen`/`isSettingsOpen` ensures no polling overhead when closed.
   - **Code Patch**: Provide the Git patch from `41693533` to restore the CSS translation strategy in `src/pages/_layout.tsx`.
4. **Overall Synthesized Findings and Comparison Table**
   - Create a table comparing v1.8.2 vs v1.8.9 on:
     - Drawer Mount Strategy
     - Stylesheet Injection Impact
     - Event Listener Leak Potential
     - Web Worker Instantiation
     - WebView2 Background Footprint
     - Connection Polling when Closed
5. **No Program Code Modification**: Double check that NO source code files (`*.ts`, `*.tsx`, etc.) are modified. Only the report file `docs/memory_regression_report.md` should be created.
