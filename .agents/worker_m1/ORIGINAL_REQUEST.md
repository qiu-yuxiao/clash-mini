## 2026-06-13T08:22:07Z
You are the Worker subagent for Milestone 1.
Your working directory is: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\worker_m1
Your task:
- Implement the optimized visibility check in `src/hooks/use-visibility.ts`. It must check document visibility state AND Tauri's window minimized state. Use Tauri's window event listeners (`onResized`, `onFocusChanged`) and query state via `getCurrentWindow().isMinimized()`. Wrap Tauri calls in try/catch for testing/browser environments.
- Modify `src/hooks/use-traffic-data.ts` to disable the hook when `isVisible` is false, stopping the WebSocket connection when invisible.
- Modify `src/hooks/use-log-data.ts` to disable the log WebSocket when `isVisible` is false.
- Modify `src/hooks/use-traffic-monitor.ts` according to the strategy:
  1. Bind subscription and reference counting in `useTrafficMonitorEnhanced` to `isActive = enabled && isVisible`.
  2. Reuse `inlineMonitor` in `TrafficWorkerClient` and do not destroy it on stop (do not set to null).
  3. Do not clear sampler in `InlineTrafficMonitor.stop()`.
  4. Do not recreate `sampler` on `init` message if it already exists.
- Run build commands (e.g. `pnpm build` or other project build scripts) and verify tests pass.
- Write a report of changes made in c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\worker_m1\changes.md and send a completion message to the caller conversation ID.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
