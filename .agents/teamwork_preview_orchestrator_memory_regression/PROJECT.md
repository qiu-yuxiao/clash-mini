# Project: Memory Regression Investigation (v1.8.9 vs v1.8.2)

## Architecture
- **Web Worker Lifecycle**: Web Worker instantiated in `src/hooks/use-traffic-monitor.ts` and executing `src/hooks/traffic.worker.ts`. We need to inspect how the worker is initialized, how `start()` and `stop()` lifecycle events are managed, and how resources (threads, event listeners, JS heap) are cleaned up or leaked.
- **DOM Layout Mounting**: Conditional rendering in `src/pages/_layout.tsx` (specifically the Settings Drawer render logic `{drawerOpen && !isMiniStatus && ...}`) compared to the CSS-based layout translation logic used in v1.8.2. We need to inspect the rendering path, Material-UI component mount/unmount triggers, and check for React memory leaks (dangling hooks, event listeners, styles).
- **Report & Remediation**: Compile all findings into `docs/memory_regression_report.md` with detailed comparison, file and line analysis, and explicit git diff proposals.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Web Worker Lifecycle Investigation | Analyze `use-traffic-monitor.ts` and `traffic.worker.ts` for leaks in WebView2 during window hide/restore transitions. | None | DONE |
| 2 | DOM Layout Mount/Unmount Investigation | Analyze `src/pages/_layout.tsx` for Settings Drawer rendering difference (1.8.9 vs 1.8.2) and React/MUI memory leaks. | None | DONE |
| 3 | Report Synthesis & Compilation | Synthesize findings into `docs/memory_regression_report.md` with explicit patch code snippets. | M1, M2 | DONE |

## Interface Contracts
- **Investigation Report**: Must be written to `docs/memory_regression_report.md`.
- **No Modifications**: All files in the codebase must remain unmodified.

## Code Layout
- `src/hooks/use-traffic-monitor.ts`
- `src/hooks/traffic.worker.ts`
- `src/pages/_layout.tsx`
- `docs/memory_regression_report.md`
