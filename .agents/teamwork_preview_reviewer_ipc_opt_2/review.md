# Review Report: Global IPC Optimization Proposal

## Review Summary

**Verdict**: REQUEST_CHANGES

The global IPC optimization proposal in `docs/ipc_optimization_proposal.md` outlines a solid and much-needed strategy to optimize Tauri IPC throughput in Clash Mini. However, there are critical gaps in backwards-compatibility and mathematical edge-case handling that must be resolved before this blueprint is ready for implementation. Specifically:
1. The proposed WebSocket message schema (`ConnectionMessage` tagged union) breaks backwards-compatibility between older/newer frontends and backends, which will crash the client or show empty lists.
2. The flat tuple design (`ConnectionUpdateTuple`) introduces maintenance risks due to positional mapping.
3. The math model contains minor unit representation mismatches and does not account for the backend's internal $O(N)$ deserialization cost.
4. There is no fallback mechanism (like periodic resync) to prevent state drift/zombie connections in the frontend cache if packets are missed or during reconnections.

---

## Findings

### [Critical] Finding 1: Backward Compatibility Break on WebSocket Message Format

- **What**: Changing the shape of the data returned by the `/connections` WebSocket stream from the raw `Connections` shape to the tagged union `ConnectionMessage` breaks compatibility.
- **Where**: `docs/ipc_optimization_proposal.md` Section 5.1 and 5.2.
- **Why**: 
  - **New frontend + Old backend**: A new frontend expecting `ConnectionMessage` will try to parse `{ type: "snapshot" | "delta", data: ... }`. When connected to an old backend that sends raw `Connections` (e.g., `{"downloadTotal": ..., "uploadTotal": ..., "connections": [...]}`), the frontend will fail to match the `type` tag and crash or fail to process connections.
  - **Old frontend + New backend**: An old frontend expecting raw `Connections` will parse `ConnectionMessage` and read `.connections`, which will be `undefined`, causing the UI to render zero active connections.
- **Suggestion**: 
  - **Option A (Untagged Serialization)**: Apply `#[serde(untagged)]` on `ConnectionMessage` in Rust so that the snapshot payload serializes to the exact same shape as the original `Connections` struct. Add an optional parameter `enable_delta: Option<bool>` to the `ws_connections` command. The backend will only emit deltas if requested. The frontend can check if `'added' in payload` to detect delta mode, defaulting to snapshot mode otherwise.
  - **Option B (Separate Command)**: Introduce a new Tauri command `ws_connections_delta` specifically for the optimized stream. The new frontend tries to connect via `ws_connections_delta` first and falls back to `ws_connections` if the command does not exist (e.g., on an older backend).

### [Major] Finding 2: Positional Indexing Risk in Flat Tuple Updates

- **What**: Positional indexing for `ConnectionUpdateTuple` (`[id, upload, download]`) introduces maintenance fragility.
- **Where**: `docs/ipc_optimization_proposal.md` Section 4.2 and 5.
- **Why**: Since array elements have no keys, if fields are added, reordered, or modified on the Rust side, the TypeScript side will decode incorrect values (e.g., swapping upload and download metrics) without any compile-time or parsing errors.
- **Suggestion**: Strictly enforce that `ts-rs` generates the TS definitions for `ConnectionUpdateTuple` and verify in build tests that serialization/deserialization matches. Clear documentation must warn about the danger of changing tuple field order.

### [Major] Finding 3: Shifted CPU Deserialization Overhead on the Rust Backend

- **What**: The proposal implies that $O(N)$ connection overhead is entirely optimized away, but it is actually shifted.
- **Where**: `docs/ipc_optimization_proposal.md` Section 3.1 and 4.
- **Why**: Currently, the Rust backend is a transparent byte proxy. In the optimized design, to compute the diff, the Rust backend must deserialize the full $O(N)$ connection JSON from the Clash core WebSocket *every second*. Thus, the $O(N)$ deserialization overhead is not eliminated; it is shifted from the WebView V8 runtime to the Rust Tokio runtime.
- **Suggestion**: Document this architectural trade-off. Acknowledge that while Rust's deserialization is significantly faster and doesn't block the UI thread, backend CPU usage will still scale linearly with connection size.

### [Minor] Finding 4: Inconsistent Units in Math Model Estimation

- **What**: The performance estimation mixes decimal kilobytes ($1\text{ KB} = 1000\text{ bytes}$) and binary kilobytes ($1\text{ KiB} = 1024\text{ bytes}$).
- **Where**: `docs/ipc_optimization_proposal.md` Section 7.4.
- **Why**: The snapshot size uses decimal ($1000 \text{ connections} \times 750\text{ bytes} = 750\text{ KB}$), but the delta size uses binary ($13,950\text{ bytes} \approx 13.62\text{ KB}$). This leads to slightly incorrect sums (e.g., $750 + 190.68 = 940.68\text{ KB}$).
- **Suggestion**: Standardize on binary units (KiB/MiB) or decimal units (KB/MB) throughout the performance estimation.

---

## Verified Claims

- **Claim**: Redundant REST polling triggers a 3-second poll returning the full list.
  - **Verified via**: `view_file` on `src/hooks/use-connection-data.ts` lines 123-153.
  - **Status**: PASS. The hook indeed polls `getConnections()` every 3 seconds if `isWsActive` is false but `isVisible` is true. Since the Clash API does not provide a totals-only endpoint, it pulls the entire connections list.
- **Claim**: TS type mapping and Rust model definitions match except for extra fields.
  - **Verified via**: Comparing `crates/tauri-plugin-mihomo/src/models.rs` and `src/types/global.d.ts`.
  - **Status**: PASS. The proposed fields map correctly, but the proposal omits the `extra` field (`HashMap<String, Value>`) which is currently present in `Connections` and `Connection` to flatten unknown properties.

---

## Coverage Gaps

- **Upstream Connection API Gaps**: The proposal does not explore whether a custom Mihomo core build or a local middleware could expose a totals-only connections endpoint, which would eliminate the initial startup poll overhead.
  - **Risk Level**: Low.
  - **Recommendation**: Accept risk, as modifying the Clash/Mihomo core binary is out of scope.

---

## Unverified Items

- **Actual throughput savings of 89.4%**: Cannot be verified directly through run commands as no implementation code exists yet in the workspace.
  - **Reason**: The changes are currently only a design proposal.

---

## Challenge Summary

**Overall risk assessment**: MEDIUM

While the differential design is theoretically sound, it introduces state synchronization risks (drift) and potential rendering blocks under extreme loads.

## Challenges

### [High] Challenge 1: State Drift & Zombie Connections in Frontend Cache

- **Assumption challenged**: The frontend cache will always remain in sync with the backend.
- **Attack scenario**: If the WebSocket drops a message due to backpressure or is temporarily interrupted/reconnected, the frontend might miss a `removed` UUID update. The closed connection will remain in the active connection list forever as a "zombie" connection.
- **Blast radius**: High. Frozen/zombie connections will accumulate in the UI table, leaking memory and showing incorrect data.
- **Mitigation**: 
  1. The backend must start every new connection stream with a full `Snapshot` to clear previous frontend state.
  2. Implement a periodic full resync: send a full `Snapshot` every 60 seconds (or 100 frames) instead of a `Delta` to let the frontend reconcile and prune drifted records.

### [Medium] Challenge 2: React UI Blocking under 100% Active Connections

- **Assumption challenged**: Throttling table updates to 1s resolves React rendering lag.
- **Attack scenario**: If $N = 2500$ connections are active and updating, rendering 2500 table rows in React every 1 second will completely freeze the UI thread in V8.
- **Blast radius**: High. Frame drops and lags will persist despite IPC payload optimizations.
- **Mitigation**: Virtualized list rendering (e.g., `@tanstack/react-virtual` or `react-window`) must be strictly mandated as a prerequisite for the connections drawer UI.

### [Medium] Challenge 3: Log Batching Buffer Flooding

- **Assumption challenged**: Batching logs to max 50 entries or 250ms prevents IPC spikes.
- **Attack scenario**: Under heavy debug loops or core crashes, logs can burst to 10,000+ entries/sec. A batch size of 50 will trigger 200 IPC events/sec, flooding the channel and rendering thread.
- **Blast radius**: Medium. UI lag and CPU spike during high log frequency.
- **Mitigation**: Add a dynamic rate-limiting cap. If log frequency exceeds e.g., 500 logs/sec, increase the batch size or drop log messages below a certain severity level (e.g. debug/trace).

## Stress Test Results

- **Worst-Case 100% Connections Active (N=1000)**:
  - **Scenario**: All 1000 connections transfer data in a single second ($R_{\text{up}} = 1000$).
  - **Predicted Behavior**: Delta size increases to $(5 \times 750) + (1000 \times 50) + (5 \times 40) = 53,950 \text{ bytes} \approx 52.7\text{ KB/s}$. Over 15s, total volume is $732.4 + 14 \times 52.7 = 1,470.2 \text{ KB} \approx 1.44\text{ MB}$.
  - **Status**: PASS. Still well below the Clash Verge target of 4.4MB.
- **Extreme P2P Connections (N=2500, 100% Active)**:
  - **Scenario**: 2500 active connections, all transferring data ($R_{\text{up}} = 2500$, $R_{\text{add}} = 10$, $R_{\text{rem}} = 10$).
  - **Predicted Behavior**: Delta size increases to $(10 \times 750) + (2500 \times 50) + (10 \times 40) = 132,900 \text{ bytes} \approx 129.8\text{ KB/s}$. Over 15s, total volume is $1831 \text{ KB} + 14 \times 129.8\text{ KB} \approx 3.56\text{ MB}$.
  - **Status**: PASS. Even under extreme load, it remains below the 4.4MB target.

## Unchallenged Areas

- **Native window visibility tracking implementation**: The mechanism using Tauri window events and emitting status changes is a standard and robust approach; it is not challenged.
