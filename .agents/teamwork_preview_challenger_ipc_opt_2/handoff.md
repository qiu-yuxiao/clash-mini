# Handoff Report — IPC Optimization Proposal Challenge

## 1. Observation
- **Proposal Document**: `docs/ipc_optimization_proposal.md`
  - Option B: Ultra-Compact Tuple is proposed for updates:
    ```rust
    pub struct ConnectionUpdateTuple(pub String, pub u64, pub u64);
    ```
    ```typescript
    export type ConnectionUpdateTuple = [id: string, upload: number, download: number];
    ```
  - The native window visibility gating design closes WebSockets when `documentVisible && !isMinimized` becomes `false`:
    > "When `documentVisible && !isMinimized` becomes `false`, the hook immediately sets all active query keys to `null`, tearing down the WebSocket connections from the frontend."
- **Frontend Codebase**:
  - `src/hooks/use-visibility.ts` lines 93–94:
    ```typescript
    return documentVisible && !isMinimized
    ```
  - `src/hooks/use-connection-data.ts` line 95:
    ```typescript
    const isVisible = useVisibility()
    ```
  - `src/hooks/use-mihomo-ws-subscription.ts` lines 191–192:
    ```typescript
    const subscriptKey = buildSubscriptKey(date)
    const subscriptionCacheKey = subscriptKey ? `$sub$${subscriptKey}` : null
    ```
    And lines 327–352 handle clean-up and closing of the socket when `subscriptionCacheKey` becomes null.

---

## 2. Logic Chain
1. The proposal recommends using a positional tuple structure `[id, upload, download]` for bandwidth updates to save ~30 bytes of key metadata per connection.
2. In-order and reliable delivery of Tauri IPC messages is assumed. However, the proposal does not specify any sequence validation, message counter, or epoch ID in the delta payloads.
3. When `isWsActive` goes from `true` to `false` (e.g. on window minimize), `subscriptionCacheKey` becomes `null`, which invokes the cleanup hook in `useMihomoWsSubscription` and closes the WebSocket connection.
4. When the window is restored, `isVisible` becomes `true` again. React Query immediately returns the cached connection list from before it was minimized.
5. Simultaneously, a new WebSocket connection is initiated. The latency of opening the local socket, completing the handshake, and receiving the first `Snapshot` from the Mihomo core takes between 50ms and 500ms.
6. Consequently, the user is presented with stale data during this reconnection window, followed by a jarring visual layout flash once the fresh `Snapshot` payload is received and processed.
7. Furthermore, if a single `Delta` update is lost or processed out of order, the client state becomes permanently desynchronized with the backend, displaying "zombie" connection rows because there is no reconciliation mechanism.

---

## 3. Caveats
- No code modification was performed as this is a review-only task.
- We assume the backend WebSocket reader manages diffing state per connection rather than globally. If the backend shares a single global diffing state, subscribing from a new window will cause catastrophic data corruption.

---

## 4. Conclusion
While the proposed design significantly reduces IPC volume, it introduces high risks of data corruption, UI visual stuttering, and maintenance fragility. 
We recommend:
- Rejecting the positional tuple format in favor of key-value structs (or short keys like `{"i": id, "u": up, "d": down}`) for schema safety.
- Adding `epoch_id` and `sequence_id` to all `Snapshot` and `Delta` payloads to validate and enforce strict in-order processing.
- Introducing a visual stale/loading indicator in the UI during socket reconnection, and adding a 3-second debounce to window minimization teardowns to avoid socket thrashing.

---

## 5. Verification Method
- **Static Schema Inspection**: Verify that the implemented Rust types in `crates/tauri-plugin-mihomo/src/models.rs` and TypeScript interfaces in `src/types/connection_diff.ts` have `epoch_id` and `sequence_id` fields.
- **Trace Reconnection Visuals**: Run the build, open the connection drawer, minimize the application, wait 10 seconds, and restore it. Inspect if the connection table dims or shows a loading state during the reconnection window rather than displaying stale data.
- **Inspect Network Drop Resilience**: Simulate a local network or WebSocket drop and verify if the frontend client triggers a clean resynchronization via a full `Snapshot` instead of applying deltas on top of a corrupted state.
