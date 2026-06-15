## 2026-06-15T04:31:39Z

Your working directory is: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_ipc_opt_2
Your task is to audit the TypeScript frontend code to identify and map all high-frequency and high-volume Tauri IPC listeners and React state hooks (e.g., /traffic, /connections, /logs).
Focus areas:
1. Search for where the frontend listens to Tauri IPC events or connects to the WebSocket streams (look at `src/hooks/use-connection-data.ts`, `src/hooks/use-traffic-data.ts`, `src/hooks/use-log-data.ts`, and `src/hooks/use-mihomo-ws-subscription.ts`).
2. Map out how these listener hooks update the React state (e.g., React Query cache updates, component re-renders, canvas drawing requests).
3. Audit how visibility-based subscription pausing is currently implemented in hooks like `useVisibility` and where it is applied (e.g., checking if `document.hidden` is true, or if `active` is true in `useTrafficData` and `useConnectionData`).
4. Look for redundant polling or listener leakage in the frontend (such as the dead code useEffect polling in `useConnectionData` or uncleared timers in `src/services/delay.ts`).
5. Suggest frontend strategies to throttle, debounce, or pause these updates to save CPU/GPU/memory.
Write your full findings in your `analysis.md` and a summary handoff in `handoff.md` in your working directory.
