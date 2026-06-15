# Challenge Report — IPC Optimization Proposal

## Challenge Summary

**Overall risk assessment**: HIGH

While the proposed optimizations (Differential Update Protocol, Native Window Visibility Gating, and Log Batching) are highly effective at reducing inter-process communication (IPC) throughput in typical scenarios, the current design contains critical flaws regarding client-state synchronization, visual correctness on window restoration, and code maintainability. 

Specifically, the absence of versioning/epoch tracking in the delta protocol creates a high risk of permanent UI state corruption in the face of network drops or transient packet losses. Furthermore, closing the connection during window minimization will display stale cached connection lists immediately upon window restoration, causing jarring UI flashes. Finally, the proposed tuple format is fragile, offering negligible byte savings at the cost of high maintenance complexity and risk of silent parsing errors.

---

## Challenges

### [High] Challenge 1: Permanent State Desynchronization in the Differential Protocol

- **Assumption challenged**: The proposal assumes that the frontend and backend will remain in perfect synchronization over the IPC channel, and that in-order delivery of every single `Delta` payload is guaranteed without loss or corruption. It also assumes the lifecycle of frontend resubscription perfectly aligns with backend WebSocket creation without any race conditions.
- **Attack scenario**: 
  1. The client subscribes and receives the initial `Snapshot` (say, Epoch 1, Sequence 0).
  2. The window is minimized or the system experiences a brief lag, causing a Tauri IPC queue backlog or a dropped message during the socket close transition.
  3. A single `Delta` update (containing connection closures and new metrics) is dropped, delayed, or fails to parse in the WebView.
  4. The frontend state now lacks updates for several connection items. Connections that were closed on the backend are never removed from the frontend cache, and new metric values are missed.
  5. Subsequent `Delta` updates arrive and are applied on top of this corrupted state. Because the protocol has no versioning, sequence ID, or epoch checks, the frontend has no way of detecting that its state has drifted from the backend. The UI displays ghost/zombie connections indefinitely until a manual reload or complete socket recreation.
- **Blast Radius**: Permanent corruption of the active connections table, memory leaks in the React/V8 state due to accumulating obsolete connection objects, incorrect total bandwidth accumulation, and poor user experience.
- **Mitigation**:
  1. **Add Sequence and Epoch IDs**: Include a monotonic `sequence_id` and a unique `epoch_id` in the `ConnectionsSnapshot` and `ConnectionsDelta` payloads.
  2. **Enforce Sequence Validation**: The frontend must keep track of `last_sequence_id`. When it receives a `Delta`, it must verify that `delta.epoch_id == current_epoch_id` and `delta.sequence_id == last_sequence_id + 1`. If there is a mismatch, the frontend must immediately request a fresh `Snapshot` to resynchronize its state.
  3. **Strict Reset on Snapshot**: When the frontend receives a message of type `'snapshot'`, it must completely overwrite the existing cache rather than trying to merge it.

---

### [High] Challenge 2: Stale Data Display and Visual Flashing on Window Restore

- **Assumption challenged**: The proposal assumes that immediately closing the WebSocket on minimization and reconnecting on restore provides a seamless transition.
- **Attack scenario**:
  1. The user minimizes the application or hides it to the system tray. The hook `useVisibility` evaluates to `false` and tears down the WebSocket subscription.
  2. The frontend retains the last known connection data in its React Query cache to avoid an empty screen.
  3. While minimized, multiple connections close, and new ones open on the backend.
  4. The user clicks the tray icon to restore the window.
  5. The window is restored instantly. The React UI displays the cached connection list, which is now **stale** (showing connections that no longer exist and outdated bandwidth numbers).
  6. Simultaneously, the frontend initiates a WebSocket connection to the backend. Resolving the connection, performing the handshake with Mihomo core, and waiting for the backend to serialize and emit the initial `Snapshot` takes between 50ms to 500ms.
  7. The user is presented with stale data for a fraction of a second, followed by a sudden visual "jump" or flash as the new `Snapshot` is loaded and rendered. If the connection fails or takes longer to connect, the user is left looking at stale data with no indication that it is obsolete.
- **Blast Radius**: Poor visual quality, UI flashing, potential for users to act on stale proxy/connection paths, and inconsistency with the tray speed display.
- **Mitigation**:
  1. **Visual Stale Indicator**: When the WebSocket is disconnected due to visibility gating, the frontend should immediately set a `stale` flag, dim the connections table, and show a subtle reloading spinner to indicate the data is being synchronized.
  2. **Grace Period on Minimization**: Implement a debounced teardown (e.g., 5–10 seconds) when the window is minimized. If the user restores the window quickly (a common pattern), the WebSocket connection is never closed, preventing connection thrashing and visual flashes.
  3. **Quick Cache Cleardown**: Alternatively, immediately clear the connections cache on minimize so that a loading state is shown upon restore, avoiding the display of incorrect stale data.

---

### [Medium] Challenge 3: Positional Brittleness and Maintainability of the Tuple Format (Option B)

- **Assumption challenged**: The proposal assumes that the ~30 bytes saved per connection update by using a flat array/tuple (`["id", upload, download]`) instead of a key-value object (`{"id": "id", "upload": u, "download": d}`) is worth the loss of self-documentation, type safety, and schema flexibility.
- **Attack scenario**:
  1. A developer adds a new field to connection updates (e.g., process CPU usage or connection speed).
  2. The developer updates the Rust tuple type `ConnectionUpdateTuple(pub String, pub u64, pub u64, pub u32)` but fails to update the index mapping in the TypeScript code (or vice-versa).
  3. During runtime, the frontend parses the new field as `download` and the old `download` field as something else, resulting in silent data corruption and erratic bandwidth graphs.
  4. In a local IPC context (Tauri), saving ~30 bytes per connection update translates to saving only ~6 KB/s for 200 updates. This savings is negligible, whereas the cost of debugging positional offset mismatches is high.
  5. From a JS execution perspective, destructuring and parsing mixed-type flat arrays `[string, number, number]` can force V8 into using dictionary-mode arrays (slower access) compared to highly optimized shapes/hidden classes used for small, monomorphic key-value objects.
- **Blast Radius**: High development maintenance overhead, fragile schema evolution, and hard-to-detect parsing bugs during client-server version mismatches.
- **Mitigation**:
  1. **Use Key-Value Structs (Option A)**: Adopt the struct format. It is self-documenting, resilient to field additions/reordering, and works natively with standard TypeScript interfaces.
  2. **Compact Key Names**: If payload size is critical, use short keys (e.g., `{"i": "uuid", "u": 123, "d": 456}`) which maintains structural safety while achieving a byte footprint identical to the tuple format.

---

### [Medium] Challenge 4: Connection Thrashing and Resource Exhaustion under Rapid Window Event Cycles

- **Assumption challenged**: The proposal assumes that window minimize/restore and focus change events occur at a low frequency.
- **Attack scenario**:
  1. The user moves the application window across virtual desktops, or rapidly clicks between the tray icon and other windows.
  2. This triggers rapid focus change and window resize/minimize events in quick succession.
  3. Each event immediately closes the existing WebSocket and spawns a new one.
  4. The backend is forced to repeatedly close sockets, clean up reader tasks, spawn new tokio tasks, establish new connections to Mihomo, and serialize and send full `Snapshot` payloads.
  5. This thrashing can lead to thread blocking, lock contention on the `Mihomo` state, local socket descriptor exhaustion, and potential application freezes.
- **Blast Radius**: Temporary application freezes, backend lock contention, and potential crashes due to socket exhaustion.
- **Mitigation**:
  1. **Debounce Transitions**: Introduce a debounce timer on the frontend before acting on visibility state changes. Only disconnect if the window remains invisible/minimized for at least 3 seconds.
  2. **Reconnection Rate Limiting**: Implement an exponential backoff or minimum interval (e.g., 2 seconds) between WebSocket connections to prevent spamming the backend.

---

## Stress Test Results

| Scenario | Expected Behavior | Predicted Behavior | Pass/Fail |
| :--- | :--- | :--- | :--- |
| **Normal Connection (Stateful)** | Backend sends Snapshot, then Deltas; client updates UI. | Client receives Snapshot, then Deltas; UI updates. | **PASS** |
| **Window Minimize/Restore Cycle** | Client disconnects on minimize, reconnects on restore. UI shows updated connections smoothly. | Client disconnects. Upon restore, UI shows stale data for 300ms, then flashes/jumps to new list. | **FAIL** (Visual defect) |
| **Rapid Minimize/Restore (< 1s)** | Client handles transitions without restarting socket unnecessarily. | Client opens and closes sockets repeatedly, creating resource thrashing. | **FAIL** (Performance risk) |
| **Dropped Delta Packet / Lag** | System detects missed update and resynchronizes. | Client state remains permanently out of sync (zombie rows appear). | **FAIL** (Data corruption) |
| **Core Restart / Reload** | Client reconnects and receives a new Snapshot, clearing all old connection cache. | Client reconnects, receives Snapshot, but if merge logic doesn't clear cache, old connections persist. | **FAIL** (Sync defect) |
| **Schema Evolution (Adding Fields)** | Adding a field is backwards compatible; older client parses gracefully. | Positional parsing breaks or shifts indices, causing incorrect fields. | **FAIL** (Maintainability) |

---

## Unchallenged Areas

- **Log Batching & Debouncing** — The proposed batching mechanism (flush at 50 entries or 250ms) is standard, robust, and carries minimal risk. The blast radius of a lost batch is low, and the performance gains are significant.
- **Traffic updates (`/traffic`)** — The 1Hz traffic update stream is already compact (~80 bytes) and requires no differential logic.
