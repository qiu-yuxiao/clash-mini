# Scope: Milestone 1 - Frontend CPU & IPC Optimization

## Architecture
- React frontend hooks managing visibility and WebSocket subscriptions.
- `useVisibility` hook in `src/hooks/use-visibility.ts` is the central component.
- `useTrafficData` and `useConnectionData` retrieve WebSocket streams using `useMihomoWsSubscription`.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1.1 | Tauri Window visibility | Integrate Tauri's window events (minimize, focus) in `useVisibility.ts`. | None | DONE |
| 1.2 | WebSocket Disconnection | Ensure `useTrafficMonitorEnhanced` stops the client when `isVisible` is false, and `useLogData` checks `isVisible`. | 1.1 | DONE |

## Interface Contracts
- `useVisibility()` returns true if and only if:
  1. `document.visibilityState === 'visible'`
  2. The window is not minimized (checked asynchronously via `getCurrentWindow().isMinimized()`).
- High-frequency WebSockets (`traffic`, `connections`, `logs`) disconnect when `useVisibility` returns false.
