# Global IPC Optimization Proposal: Profiling, Root Cause Diagnostics, and Differential Update Design for Clash Mini

## 1. Executive Summary

This proposal outlines a global optimization design to resolve the inter-process communication (IPC) throughput issues observed in Clash Mini (a Mihomo/Clash core GUI client built on Tauri). Under high network load (e.g., active peer-to-peer torrenting with approximately 1,000 to 2,500 active connections) and active debug logging, the IPC throughput between the Rust backend and the TypeScript WebView frontend can surge to over **38MB in 15 seconds**. This volume represents an unsustainable load that leads to significant CPU and memory utilization, rendering lags, and frame drops. In comparison, Clash Verge keeps its throughput down to approximately **4.4MB** under similar conditions.

By analyzing the Rust backend emitters, TypeScript frontend listeners, and payload structures, we have identified three primary root causes:
1. **$O(N)$ connection lists** sent in full every second, repeating heavy static metadata.
2. **Background visibility leaks**, where WebSockets remain active when the window is minimized or hidden in the system tray.
3. **Redundant REST API fallback polling** that retrieves full lists just to fetch upload/download totals.

To mitigate this, we propose:
1. A **Differential Update Protocol (Delta Push)** for active connections, sending only changes (`added`, `updated` bandwidth, `removed`) and reducing the payload size of subsequent updates by over 98%.
2. **Native Window Visibility Gating** on both the frontend and backend to suspend data streams when the application is minimized or hidden.
3. **Dynamic Frontend Throttling and Log Batching** to decrease React render overhead and V8 engine timer usage.

This document serves as the implementation blueprint. The proposed optimizations collectively reduce the 15-second IPC payload volume under high load from **38MB+** down to **~1.25MB** for 1,000 active connections and **~2.64MB** for 2,500 active connections (with background mode reducing this to **0.00MB** due to window visibility gating), easily satisfying the optimization target of reaching Clash Verge's level (~4.4MB). Under extreme loads of 5,000 active connections, viewport-based pagination guarantees that throughput is kept to a minimal **0.50MB**, well below the threshold.

---

## 2. IPC Event Profiling & Subscription Registry Map

The backend proxies the Clash/Mihomo core's local WebSocket API and exposes these data streams to the frontend via Tauri IPC channels. The table below lists all high-frequency IPC channels, their sizes, frequencies, and code references.

| Event Endpoint | Tauri Command | Avg Payload Size | Frequency / Trigger | Backend Code Files | Frontend Code Files |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Traffic Updates** `/traffic` | `ws_traffic` | ~80 bytes | 0.33Hz (Every 3s) | **Registration:**<br>`crates/tauri-plugin-mihomo/src/commands.rs` (lines 239–248)<br>**Emission Loop:**<br>`crates/tauri-plugin-mihomo/src/mihomo.rs` (lines 250–287, 316–353) | **Subscription Hook:**<br>`src/hooks/use-traffic-data.ts`<br>**Canvas Graph Component:**<br>`src/components/home/enhanced-canvas-traffic-graph.tsx`<br>**Tray Speed Task:**<br>`src-tauri/src/utils/connections_stream.rs` (lines 76–97) |
| **Active Connections** `/connections` | `ws_connections` | $O(N)$ (~750 bytes per active connection object) | 1Hz (Every 1s) | **Registration:**<br>`crates/tauri-plugin-mihomo/src/commands.rs` (lines 263–272)<br>**Emission Loop:**<br>`crates/tauri-plugin-mihomo/src/mihomo.rs` (lines 250–287, 316–353) | **Subscription Hook:**<br>`src/hooks/use-connection-data.ts`<br>**Table Components:**<br>`src/components/connection/connection-table.tsx`<br>`src/components/connection/connection-detail.tsx` |
| **System & Core Logs** `/logs` | `ws_logs` | ~150–200 bytes per log line | Event-driven (can burst to >100Hz in debug mode) | **Registration:**<br>`crates/tauri-plugin-mihomo/src/commands.rs` (lines 275–286)<br>**Emission Loop:**<br>`crates/tauri-plugin-mihomo/src/mihomo.rs` (lines 250–287, 316–353) | **Subscription Hook:**<br>`src/hooks/use-log-data.ts`<br>**API Client Definitions:**<br>`crates/tauri-plugin-mihomo/guest-js/index.ts` (lines 477–480) |
| **Memory Statistics** `/memory` | `ws_memory` | ~40 bytes | 1Hz (Every 1s) | **Registration:**<br>`crates/tauri-plugin-mihomo/src/commands.rs` (lines 251–260)<br>**Emission Loop:**<br>`crates/tauri-plugin-mihomo/src/mihomo.rs` (lines 250–287, 316–353) | **API Client Definition:**<br>`crates/tauri-plugin-mihomo/guest-js/index.ts` (lines 461–465)<br>*(Note: Not subscribed directly in UI; memory is read from the `/connections` payload).* |

### Data Structures & Backend transparent Proxying
Currently, the Rust backend acts as a **transparent byte proxy**. In `crates/tauri-plugin-mihomo/src/mihomo.rs` (specifically inside the WebSocket reader loop), the text frames read from the core WebSocket are converted directly to `InvokeResponseBody::Raw(text.as_bytes().to_vec())` and forwarded to the Tauri channel. The Rust backend does not deserialize or re-serialize the JSON payloads. While this avoids CPU overhead in Rust, it pushes the entire serialization burden onto the V8 process in the frontend.

---

## 3. Discrepancy Diagnostics: Root Causes of 38MB+ Throughput

The 38MB+ IPC volume discrepancy in Clash Mini compared to the ~4.4MB baseline in Clash Verge is driven by four primary causes:

### 3.1. $O(N)$ Bandwidth Scaling of Connection Snapshots
The core WebSocket `/connections` returns the entire active connection list every second. Under standard usage, a connection object contains **31 fields** (e.g., source IP, destination IP, source GeoIP array, destination GeoIP array, ASN metadata, DNS mode, process name, process path, and rule info) and averages **~750 bytes** when serialized.
* For **1,000 connections**, this equates to $750\text{ KB/s}$ ($11.25\text{ MB}$ over 15 seconds).
* For **2,500 connections** (common in P2P file-sharing), this equates to $1.875\text{ MB/s}$ ($28.125\text{ MB}$ over 15 seconds).
* **The Defect:** Over 95% of this payload is static metadata (IP addresses, ports, start times, rules, and process paths) that does not change throughout the connection's lifetime. Only the `upload` and `download` fields (accumulated byte metrics) change dynamically.

### 3.2. Background Visibility leaks (Minimization and Hiding to Tray)
* **The Defect:** The frontend's `useVisibility` hook (found in `src/hooks/use-visibility.ts`) relies primarily on `document.visibilityState === 'visible'`. It does not listen to native window events representing minimized window state or when the app is hidden to the system tray.
* **The Impact:** When Clash Mini is minimized or hidden in the tray, the DOM visibility state remains `'visible'` or is not correctly evaluated by Tauri. Consequently, the subscriptions to `/connections` (1Hz), `/traffic` (1Hz), and `/logs` (variable) remain fully active, serializing, emitting, and parsing megabytes of background data, draining system resources while the app is out of sight.

### 3.3. Redundant REST Polling of Full Connection Lists
* **The Defect:** In `src/hooks/use-connection-data.ts`, a fallback `useEffect` is registered to poll connection totals when the WebSocket is inactive. Due to a logical bug in the early return statement:
  ```typescript
  if (isWsActive || !isVisible || !enabled) return;
  ```
  This returns early for all states because `isWsActive` is `enabled && isVisible`. While this logical bug prevents the poll from executing in Clash Verge, resolving it incorrectly or having parallel components call it triggers a 3-second REST poll to the Mihomo `getConnections()` endpoint.
* **The Impact:** Because the Mihomo core does not expose a "totals-only" endpoint, calling `getConnections()` returns the entire connection list. Fetching the full list of 1,000 connections every 3 seconds transfers an additional $750\text{ KB}$ per poll ($3.75\text{ MB}$ in 15 seconds) just to extract the `uploadTotal` and `downloadTotal` fields.
* **In addition:** The traffic subscription hook `useTrafficData` in `src/hooks/use-traffic-data.ts` triggers a REST `getConnections()` call on startup to initialize the baseline upload and download totals.

### 3.4. Event-Driven Logs Stream Spikes
Under `debug` log levels, the core logs `/logs` emit up to 100+ times per second during heavy traffic. Forwarding these raw logs line-by-line over Tauri IPC creates excessive IPC message dispatching overhead. If the Log page is closed but the subscription is not properly torn down due to reference leakages or background activity, it contributes an additional ~300KB to 1.5MB of throughput in 15 seconds.

---

## 4. Differential Update Protocol Design

To eliminate the $O(N)$ transmission of static metadata every second, we design a **Differential Update Protocol (Delta Push)** for the `/connections` endpoint.

```
       Rust Backend (Plugin)                        TS Frontend (WebView)
                 │                                            │
                 ├───────── [1] Initial Snapshot ────────────>│ (Store full state, reset cache,
                 │                                            │  store sequence_id & epoch_id)
                 │              ─── 1s passes ───             │
                 │                                            │
                 ├───────── [2] ConnectionsDelta ────────────>│ (Validate sequence & epoch,
                 │              ├─ epoch_id: String           │  apply changes to cache)
                 │              ├─ sequence_id: u64           │
                 │              ├─ added: Vec<Conn>           │
                 │              ├─ updated: Flat Array        │
                 │              └─ removed: Vec<UUID>         │
```

### 4.1. Mechanics of the Protocol
1. **Initial Baseline (Snapshot)**:
   When the client subscribes to the connections stream, the backend emits a `Snapshot` payload containing all current connections with their full metadata. The frontend stores this list as the active baseline cache.
2. **Periodic Delta Updates**:
   Every 1 second, the backend diffs the current active connections against the prior frame. It compiles and emits a `Delta` payload containing:
   * **`added`**: An array of `Connection` objects representing new connections established in the last second. These contain full metadata fields.
   * **`updated`**: A flat 1D array layout (see Option B below) representing metrics-only update entries of connections that have transferred data in the last second.
   * **`removed`**: An array of strings representing connection `id`s (UUIDs) that closed in the last second.
3. **Re-Sync, Epoch, and Sequence Tracking**:
   * **`epoch_id` (String)**: A unique UUID generated on the backend whenever a new connection stream is initialized.
   * **`sequence_id` (u64)**: A monotonic counter incremented by 1 for each successive message sent on the stream (starting at 0 for the `Snapshot`).
   * Both `Snapshot` and `Delta` schemas include `epoch_id` and `sequence_id`.
   * **Frontend Sequence Validation**:
     - The TS frontend stores `lastSequenceId` and `currentEpochId` in state.
     - On receiving a `Snapshot`, the frontend completely overwrites its connections cache, sets `currentEpochId = snapshot.epochId`, and sets `lastSequenceId = snapshot.sequenceId`.
     - On receiving a `Delta`, the frontend checks if `delta.epochId === currentEpochId && delta.sequenceId === lastSequenceId + 1`.
     - If this validation check fails (indicating a packet drop, network delay, or backend restart), the frontend immediately discards its connections cache, terminates the WebSocket connection, and initiates a reconnection to obtain a fresh `Snapshot`.
     - If validation succeeds, it updates `lastSequenceId = delta.sequenceId` and merges the delta.
   * **Periodic Resync**:
     - To prevent long-term cache drift or zombie records due to unforeseen client-side bugs, the backend automatically emits a full `Snapshot` instead of a `Delta` every 60 seconds (or after 100 updates). The frontend handles this snapshot by overwriting the cache.

### 4.2. Update Payload Format Comparison
To optimize the `updated` list and minimize allocation/parsing overhead, we evaluate two structures:

#### Option A: Key-Value Struct
The backend sends a struct format:
```json
{ "id": "d3b07384-d113-4956-be4d-045388c3a10e", "upload": 204850, "download": 1048200 }
```
* **Pros:** Highly readable and easy to parse directly into frontend models.
* **Cons:** Contains structural key metadata (`"id"`, `"upload"`, `"download"`) in every entry. For 200 updated connections per second, this structure adds ~30 bytes of structural overhead per entry ($6\text{ KB/s}$).

#### Option B: Flat 1D Array Layout (Recommended)
Instead of wrapping each metrics update in individual object structures or nested array tuples (which would trigger thousands of allocations), the backend serializes all updates into a single, contiguous flat 1D array of alternating values:
```json
["d3b07384-d113-4956-be4d-045388c3a10e", 204850, 1048200, "f8c07212-e114-4957-ae4e-045388c3a10f", 50212, 908112]
```
* **Pros:** 
  - Eliminates structural key names entirely, saving ~30 bytes per entry.
  - Reduces V8 Garbage Collection (GC) pressure by **99.95%**. In a nested array representation `[[id1, up1, down1], ...]`, rendering 2,000 updates forces the V8 engine to allocate 2,001 array objects every second, causing frequent GC pauses and micro-stutters. Serializing updates as a single flat array reduces the allocation count to exactly **1** array object per update frame.
* **Cons:** Requires the frontend to iterate through the flat array in steps of 3 (`i += 3`) to extract positional elements. Tuple positions must be strictly maintained and cannot be altered without synchronizing the backend and frontend.

---

## 5. TypeScript and Rust Data Structures

Below are the exact structs (Rust) and interfaces (TypeScript) defining the differential updates.

### 5.1. Rust Data Structures (`crates/tauri-plugin-mihomo/src/models.rs`)

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use std::collections::HashMap;
use serde_json::Value;

// We reference the existing Connection struct from the plugin, including its extra fields
// pub struct Connection {
//     pub id: String,
//     pub upload: u64,
//     pub download: u64,
//     ...
//     #[serde(flatten)]
//     pub extra: HashMap<String, Value>,
// }

#[derive(Debug, Serialize, Deserialize, TS, Clone, PartialEq, Eq)]
#[ts(export)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum ConnectionMessage {
    Snapshot(ConnectionsSnapshot),
    Delta(ConnectionsDelta),
}

#[derive(Debug, Serialize, Deserialize, TS, Clone, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionsSnapshot {
    pub epoch_id: String,
    #[ts(type = "number")]
    pub sequence_id: u64,
    #[ts(type = "number")]
    pub download_total: u64,
    #[ts(type = "number")]
    pub upload_total: u64,
    pub connections: Vec<Connection>,
    #[ts(type = "number")]
    pub memory: u64,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionsDelta {
    pub epoch_id: String,
    #[ts(type = "number")]
    pub sequence_id: u64,
    #[ts(type = "number")]
    pub download_total: u64,
    #[ts(type = "number")]
    pub upload_total: u64,
    #[ts(type = "number")]
    pub memory: u64,
    pub added: Vec<Connection>,
    pub updated: Vec<serde_json::Value>, // Alternating flat layout: [id1, up1, down1, id2, up2, down2, ...]
    pub removed: Vec<String>,
}
```

### 5.2. TypeScript Data Structures and Frontend Merge Algorithm (`src/types/connection_diff.ts`)

```typescript
import { IConnectionsItem } from './global';

export type ConnectionMessage =
  | { type: 'snapshot'; data: ConnectionsSnapshot }
  | { type: 'delta'; data: ConnectionsDelta };

export interface ConnectionsSnapshot {
  epochId: string;
  sequenceId: number;
  downloadTotal: number;
  uploadTotal: number;
  connections: IConnectionsItem[];
  memory: number;
}

export interface ConnectionsDelta {
  epochId: string;
  sequenceId: number;
  downloadTotal: number;
  uploadTotal: number;
  memory: number;
  added: IConnectionsItem[];
  updated: (string | number)[]; // Flat 1D Array Layout: [id1, up1, down1, id2, up2, down2, ...]
  removed: string[];
}

export interface ConnectionMonitorData {
  uploadTotal: number;
  downloadTotal: number;
  activeConnections: IConnectionsItem[];
  closedConnections: IConnectionsItem[];
}

const MAX_CLOSED_CONNS_NUM = 100;
const initConnData: ConnectionMonitorData = {
  uploadTotal: 0,
  downloadTotal: 0,
  activeConnections: [],
  closedConnections: [],
};

// State tracker variables within the frontend WebSocket subscription hook
let currentEpochId: string | null = null;
let lastSequenceId: number = -1;

/**
 * Merges incoming WebSocket differential messages into the React state cache.
 * Resolves both the connection speed-freeze bug and optimizes reference stability.
 */
export const mergeConnectionMessage = (
  message: ConnectionMessage,
  previous: ConnectionMonitorData = initConnData,
  triggerReconnect: () => void
): ConnectionMonitorData => {
  // 1. Snapshot processing: overwrite state completely
  if (message.type === 'snapshot') {
    currentEpochId = message.data.epochId;
    lastSequenceId = message.data.sequenceId;

    return {
      uploadTotal: message.data.uploadTotal,
      downloadTotal: message.data.downloadTotal,
      activeConnections: message.data.connections.map(conn => ({
        ...conn,
        curUpload: 0,
        curDownload: 0,
      })),
      closedConnections: previous.closedConnections, // Retain history of closed connections
    };
  }

  // 2. Delta processing: validate and incrementally merge
  if (message.type === 'delta') {
    const delta = message.data;

    // Sequence & Epoch Validation
    if (delta.epochId !== currentEpochId || delta.sequenceId !== lastSequenceId + 1) {
      console.warn('Sequence mismatch or epoch change. Triggering connection resync.');
      triggerReconnect();
      return previous;
    }

    lastSequenceId = delta.sequenceId;

    const previousActive = previous.activeConnections ?? [];
    const previousClosed = previous.closedConnections ?? [];
    const activeMap = new Map<string, IConnectionsItem>();

    // Copy previous active connections into map for O(1) lookups and updates
    for (let i = 0; i < previousActive.length; i++) {
      activeMap.set(previousActive[i].id, { ...previousActive[i] });
    }

    // 2.1. Process Removals
    const dropped: IConnectionsItem[] = [];
    for (let i = 0; i < delta.removed.length; i++) {
      const id = delta.removed[i];
      const conn = activeMap.get(id);
      if (conn) {
        activeMap.delete(id);
        dropped.push(conn);
      }
    }

    const updatedSet = new Set<string>();

    // 2.2. Process Updates (Flat 1D layout: [id1, up1, down1, id2, up2, down2, ...])
    for (let i = 0; i < delta.updated.length; i += 3) {
      const id = delta.updated[i] as string;
      const upload = delta.updated[i + 1] as number;
      const download = delta.updated[i + 2] as number;
      const conn = activeMap.get(id);
      if (conn) {
        conn.curUpload = upload - conn.upload;
        conn.curDownload = download - conn.download;
        conn.upload = upload;
        conn.download = download;
        updatedSet.add(id);
      }
    }

    // Reset speeds to 0 for connections that did NOT receive metrics updates in this frame
    // This fixes the connection speed-freeze bug where speeds freeze at the last non-zero value.
    for (let i = 0; i < previousActive.length; i++) {
      const id = previousActive[i].id;
      if (!updatedSet.has(id)) {
        const conn = activeMap.get(id);
        if (conn) {
          conn.curUpload = 0;
          conn.curDownload = 0;
        }
      }
    }

    // 2.3. Process Additions
    for (let i = 0; i < delta.added.length; i++) {
      const conn = delta.added[i];
      activeMap.set(conn.id, { ...conn, curUpload: 0, curDownload: 0 });
    }

    // 2.4. Optimize Reference Stability to prevent unnecessary React re-renders
    const activeConnections: IConnectionsItem[] = [];
    for (let i = 0; i < previousActive.length; i++) {
      const prev = previousActive[i];
      const next = activeMap.get(prev.id);
      if (next) {
        if (
          prev.upload === next.upload &&
          prev.download === next.download &&
          prev.curUpload === 0 &&
          next.curUpload === 0 &&
          prev.curDownload === 0 &&
          next.curDownload === 0
        ) {
          activeConnections.push(prev); // Keep prev object reference
        } else {
          activeConnections.push(next);
        }
        activeMap.delete(prev.id);
      }
    }

    // Add remaining (newly added) connections
    for (const conn of activeMap.values()) {
      activeConnections.push(conn);
    }

    // 2.5. Merge Closed Connections History
    const rawClosedLen = previousClosed.length + dropped.length;
    let closedConnections: IConnectionsItem[];
    if (rawClosedLen <= MAX_CLOSED_CONNS_NUM) {
      closedConnections = previousClosed.concat(dropped);
    } else {
      const skipPrev = rawClosedLen - MAX_CLOSED_CONNS_NUM;
      closedConnections =
        skipPrev >= previousClosed.length
          ? dropped.slice(skipPrev - previousClosed.length)
          : previousClosed.slice(skipPrev).concat(dropped);
    }

    return {
      uploadTotal: delta.uploadTotal ?? 0,
      downloadTotal: delta.downloadTotal ?? 0,
      activeConnections,
      closedConnections,
    };
  }

  return previous;
};
```

### 5.3. Backwards Compatibility & Capability Negotiation
Due to Clash Mini's desktop deployment model where the frontend and backend are compiled and released in lockstep as a single binary, backward compatibility and negotiation between mismatching versions is explicitly rejected to avoid over-engineering. Both frontend and backend will upgrade synchronously to the differential update protocol in version v1.2.4, completely removing the legacy connections payload format and parameters.

---

## 6. Window Visibility Tracking and Suspend/Throttling Design

To prevent background leakage and reduce unnecessary rendering thread execution, we design a window visibility-based throttling and suspension policy.

```
                  ┌──────────────────────────────────────────┐
                  │          Tauri Window Event              │
                  │      (Minimized, Hidden, Resized)        │
                  └────────────────────┬─────────────────────┘
                                       │
               Frontend Debounces Window State (1000ms)
                                       │
             ┌─────────────────────────┴─────────────────────────┐
             ▼                                                   ▼
   [Frontend WebView]                                     [Rust Backend]
   - Clear cache on minimize                              - Call clear_all_ws_connections()
   - Disconnect /connections WS                           - Terminate active tokio tasks
   - Render loading skeleton on restore                   - Close local sockets to Mihomo
   - REST prefetch on restore before WS
```

### 6.1. Native Window Visibility Gating & Backend Simplification

#### 1. Focus Loss Suspend Correction
Data stream suspension and WebSocket disconnection must **only** occur when the window is **minimized** or **hidden** (e.g., in the system tray). Pausing on window focus loss (`Focused(false)`) is prohibited, as it breaks multi-monitor workflows (e.g., users who place Clash Mini on a secondary screen to monitor traffic while working on their primary screen). When the window is visible but unfocused, the data streams must remain active, although they may optionally be throttled to a lower update frequency (e.g. 3,000ms).

#### 2. Stateless Backend Simplification
Rather than implementing custom background tasks, buffer structures, or message-dropping loops inside the Rust WebSocket proxies (which adds statefulness, complexity, and risk of memory leaks or socket backpressure), the backend will leverage the existing stateless `clear_all_ws_connections()` function. When all application windows are minimized or hidden:
- The backend triggers `clear_all_ws_connections()`, terminating all active proxy sockets.
- The frontend tears down its subscriptions.
- Upon window restoration, the frontend's built-in reconnection logic in `useMihomoWsSubscription` will naturally wake up, reconnect, and request a fresh `Snapshot` with a new `epoch_id`.

#### 3. Connection Thrashing Debouncing
To prevent rapid setup and teardown of WebSocket streams during quick window state transitions (e.g., dragging the window across desktops, fast minimize/restore cycles), the frontend visibility state updates are debounced by **1,000ms**. Disconnection or reconnection actions will only trigger if the window state remains minimized/hidden or visible/restored for more than 1,000ms.

#### 4. Visual Flash & Stale Data Mitigations on Restore
To avoid presenting stale connection metadata to the user or causing jarring UI jumps upon restoring the application window:
- **Cache Cleardown**: The frontend immediately clears the connection list cache when the window becomes invisible or minimized.
- **Loading Overlay**: On window restore, while the WebSocket handshake is in progress, the UI displays a loading skeleton or a translucent overlay.
- **Immediate REST Prefetch**: To bypass the WebSocket connection handshake latency (which can take 50ms to 500ms), the frontend immediately fires a single HTTP REST request to `/connections` (`getConnections()`) upon window restore. This updates the table layout instantly. When the WebSocket connection establishes and sends the first `Snapshot`, the UI transitions seamlessly to the WebSocket differential updates.

#### 5. Virtualized Table Rendering
Rendering thousands of connection table rows in React every second will block the browser thread and cause severe lagging. To guarantee smooth performance:
- The connection table must use virtualized list rendering (e.g., `@tanstack/react-virtual` or `react-window`).
- This restricts DOM node count to only the visible rows in the viewport, ensuring React rendering overhead remains constant ($O(1)$) regardless of the total connection count.

### 6.2. Dynamic Throttling & Log Batching Design
1. **Frontend Connection List Throttling:**
   We increase the connection table update throttle to **1,000ms** (1Hz) or **2,000ms** (0.5Hz) to reduce React re-rendering overhead to a human-legible rate.
2. **Traffic Graph & Canvas Redraw Throttling:**
   The `/traffic` update frequency and the Canvas line graph redraw loop are capped at **3,000ms** (0.33Hz) globally across all window layouts. This reduces React rendering triggers and Canvas paint calculations by 66%, saving system CPU resources.
3. **Log Batching with Real-time Error Flush:**
   - Rather than forwarding every log line via a separate IPC event, the Rust backend will implement a thread-safe batching buffer. Logs are accumulated and pushed in batches when the buffer reaches 50 entries, or when a time limit of **250ms** is reached.
   - **Real-time Error Flush**: To prevent troubleshooting delays, any log entry with an `Error` level immediately flushes the batching buffer, ensuring critical logs appear in the UI without latency.
   - **Rate Limiting**: If the log rate exceeds 500 logs/sec (e.g., debug log flood), the backend will automatically increase the flush interval to 500ms and temporarily drop `Trace`/`Debug` logs to protect IPC bandwidth.

### 6.3. Static Border Style Association with 6 Aesthetic Themes

To prevent layout shifts and maintain a uniform visual footprint, the static window border is locked to a uniform width of **4px** across all themes. It dynamically adapts its border-style (solid vs. double), gradient color palette, and shadows based on the active theme (`theme.controlSkin`):

1. **Retro 3D**:
   - **Border Style**: `4px double` (two ~1px solid lines with a 2px gap, simulating a physical dual brass/metal console rim).
   - **Color**: Static diagonal gradient using **Amber-Gold** (`#FFC400`) and **Electric-Blue** (`#0084FF`).
   - **Glow/Shadow**: Subtly embossed inner shadow (`box-shadow: inset 0 0 8px rgba(212,175,55,0.4)`).
2. **Original**:
   - **Border Style**: `4px solid` (flat single outline, matching classic grey/white console boundaries).
   - **Color**: Light-grey (`#E0E0E0`) in light mode, or dark-grey (`#2D2D2D`) in dark mode.
   - **Glow/Shadow**: None.
3. **Modern**:
   - **Border Style**: `4px solid` (clean single accent outline).
   - **Color**: Solid accent theme color (or auto-detected system theme color).
   - **Glow/Shadow**: Very soft, flat drop-shadow.
4. **Frosted Glass**:
   - **Border Style**: `4px double` (two 1px semi-transparent white lines with a transparent gap, creating a futuristic glass pane edge).
   - **Color**: Semi-transparent white (`rgba(255, 255, 255, 0.2)`).
   - **Glow/Shadow**: Large, diffused soft drop-shadow to separate the glass panel from the desktop.
5. **Cyberpunk**:
   - **Border Style**: `4px double` (two 1px neon lines with a dark gap, rendering a high-tech hollow neon wireframe).
   - **Color**: Static gradient from **Neon Cyan** (`#00F5FF`) to **Neon Magenta** (`#FF007F`).
   - **Glow/Shadow**: Strong, hardware-accelerated neon outer drop shadow (`filter: drop-shadow(0 0 4px rgba(0, 245, 255, 0.5))`).
6. **Monochrome**:
   - **Border Style**: `4px solid` (flat slate outline).
   - **Color**: Solid medium grey (`#808080`) in light mode, or dark grey (`#404040`) in dark mode.
   - **Glow/Shadow**: None.

---

## 7. Theoretical Mathematical / Performance Estimation

We model the IPC payload size and throughput over a **15-second window ($T = 15$)** across three distinct operation cases. All calculations are standardized on decimal metric units ($1\text{ KB} = 1,000\text{ bytes}$, $1\text{ MB} = 1,000,000\text{ bytes}$) for mathematical consistency.

### 7.1. Model Parameters and Assumptions
* **$N$ (Average Connections):** $1,000$ active connections.
* **$S_{\text{full}}$ (Serialized Connection Object size):** $\approx 750\text{ bytes}$.
* **$S_{\text{snapshot}}$ (Initial connections snapshot size):** $N \times S_{\text{full}} = 750,000\text{ bytes} = 750\text{ KB}$.
* **$R_{\text{add}}$ (New connections rate):** $5\text{ connections/second}$.
* **$R_{\text{rem}}$ (Closed connections rate):** $5\text{ connections/second}$.
* **$R_{\text{up}}$ (Active transferring connections rate):** $200\text{ connections/second}$ (scales to $500\text{/second}$ when active connections $N = 2,500$, and is capped at $100\text{/second}$ under viewport-based pagination).
* **$S_{\text{update\_flat}}$ (Flat array entry size contribution):** $\approx 50\text{ bytes}$ per updated connection (representing ID, upload, and download values serialized in the flat array).
* **$S_{\text{remove}}$ (Closed connection ID size):** $\approx 40\text{ bytes}$ (36-byte UUID + quotes + comma).
* **$S_{\text{traffic}}$ (Traffic update event size):** $\approx 60\text{ bytes}$.
* **$S_{\text{log\_line}}$ (Log entry size):** $\approx 200\text{ bytes}$.
* **$L_{\text{debug}}$ (Log stream frequency in debug mode):** $100\text{ log lines/second}$.

---

### 7.2. Case 1: App is Minimized or Hidden (Background Mode)

#### A. Baseline Throughput (Before Optimizations):
Because there is no window visibility check for the WebSockets and REST fallback polling continues in the background, all data streams run continuously.
* **Connections WebSocket:** $15\text{ s} \times 750,000\text{ bytes/s} = 11,250,000\text{ bytes} = 11.25\text{ MB}$.
* **Redundant REST Polls:** 5 polls (every 3s) returning the full list: $5 \times 750,000\text{ bytes} = 3,750,000\text{ bytes} = 3.75\text{ MB}$.
* **Traffic WebSocket:** $15\text{ s} \times 60\text{ bytes} = 900\text{ bytes} = 0.9\text{ KB} = 0.0009\text{ MB}$.
* **Logs WebSocket:** $15\text{ s} \times 100\text{ logs/s} \times 200\text{ bytes} = 300,000\text{ bytes} = 300\text{ KB} = 0.30\text{ MB}$.
$$\text{Total Baseline} = 11.25 + 3.75 + 0.0009 + 0.30 = \mathbf{15.3009\text{ MB}}$$

#### B. Optimized Throughput:
With native window visibility tracking and `clear_all_ws_connections()`, all WebSockets are closed and background REST polling is suspended.
$$\text{Total Optimized} = \mathbf{0.00\text{ MB}} \quad (\text{100\% reduction})$$

---

### 7.3. Case 2: App is Visible, Connection Drawer is Closed

#### A. Baseline Throughput:
The connection drawer is closed, but the app continues to receive traffic and log streams. The REST fallback poll runs in the background.
* **Traffic WebSocket:** $15\text{ s} \times 60\text{ bytes} = 900\text{ bytes} = 0.9\text{ KB} = 0.0009\text{ MB}$.
* **Logs WebSocket:** $15\text{ s} \times 100\text{ logs/s} \times 200\text{ bytes} = 300,000\text{ bytes} = 0.30\text{ MB}$.
* **Redundant REST Polls:** $5 \times 750,000\text{ bytes} = 3,750,000\text{ bytes} = 3.75\text{ MB}$.
$$\text{Total Baseline} = 0.0009 + 0.30 + 3.75 = \mathbf{4.0509\text{ MB}}$$

#### B. Optimized Throughput:
The `/connections` WebSocket subscription is closed because the drawer is not open. Fallback polling is eliminated; the totals (`uploadTotal` and `downloadTotal`) are updated incrementally using the `/traffic` stream instead.
* **Traffic WebSocket (Throttled to 3s):** $5 \times 60\text{ bytes} = 300\text{ bytes} = 0.3\text{ KB} = 0.0003\text{ MB}$.
* **Logs WebSocket (Debug):** $15\text{ s} \times 100\text{ logs/s} \times 200\text{ bytes} = 300,000\text{ bytes} = 300\text{ KB} = 0.30\text{ MB}$.
* **REST Connection Poll:** $0\text{ KB}$.
$$\text{Total Optimized} = 300\text{ bytes} + 300,000\text{ bytes} = 300,300\text{ bytes} = 300.3\text{ KB} \approx \mathbf{0.300\text{ MB}} \quad (\text{92.6\% reduction})$$

---

### 7.4. Case 3: App is Visible, Connection Drawer is Open (Active Monitoring)

#### A. Baseline Throughput:
The connections list is sent in full every second, along with traffic and logs.
* **Connections WebSocket:** $15\text{ s} \times 750,000\text{ bytes} = 11,250,000\text{ bytes} = 11.25\text{ MB}$ (at $N=1,000$) and **$28.125\text{ MB}$** (at $N=2,500$ where snapshot is $1,875\text{ KB}$).
* **Traffic WebSocket:** $15\text{ s} \times 60\text{ bytes} = 900\text{ bytes} = 0.0009\text{ MB}$.
* **Logs WebSocket:** $15\text{ s} \times 100\text{ logs/s} \times 200\text{ bytes} = 300,000\text{ bytes} = 0.30\text{ MB}$.
$$\text{Total Baseline}_{1000} = 11.25 + 0.0009 + 0.30 = \mathbf{11.5509\text{ MB}}$$
$$\text{Total Baseline}_{2500} = 28.125 + 0.0009 + 0.30 = \mathbf{28.4259\text{ MB}}$$

#### B. Optimized Throughput (Differential Update Protocol):

##### 1. Under Active Load ($N = 1,000$, $R_{\text{up}} = 200/s$)
At $t=0$, we transmit 1 full `Snapshot` ($750\text{ KB}$ or $750,000\text{ bytes}$). For $t = 1 \dots 14$ (14 frames), we transmit only `Delta` updates using the flat array layout.
$$\text{Size}_{\text{delta\_flat}} = (R_{\text{add}} \times S_{\text{full}}) + (R_{\text{up}} \times S_{\text{update\_flat}}) + (R_{\text{rem}} \times S_{\text{remove}})$$
$$\text{Size}_{\text{delta\_flat}} = (5 \times 750) + (200 \times 50) + (5 \times 40) = 3,750 + 10,000 + 200 = 13,950\text{ bytes}$$
* **Total Connections Volume over 15s:**
  $$\text{Vol}_{\text{conn}} = S_{\text{snapshot}} + (14 \times \text{Size}_{\text{delta\_flat}})$$
  $$\text{Vol}_{\text{conn}} = 750,000 + (14 \times 13,950) = 750,000 + 195,300 = 945,300\text{ bytes} = 945.3\text{ KB}$$
* **Total Optimized Throughput (including Throttled Traffic and Logs):**
  $$\text{Total Optimized}_{1000} = 945,300\text{ bytes} + 300\text{ bytes} + 300,000\text{ bytes} = 1,245,600\text{ bytes} \approx \mathbf{1.25\text{ MB}} \quad (\text{89.2\% reduction})$$

##### 2. Under High Active Load ($N = 2,500$, $R_{\text{up}} = 500/s$)
When active connections scale to 2,500, active P2P throughput causes the churn rates to scale to $R_{\text{add}} = 10/s, R_{\text{rem}} = 10/s$, and the active transferring connection rate $R_{\text{up}}$ scales from 200/s to 500/s.
* **Initial Snapshot Size ($N=2,500$):**
  $$S_{\text{snapshot}} = 2,500 \times 750\text{ bytes} = 1,875,000\text{ bytes} = 1,875\text{ KB}$$
* **Delta Size per second ($N=2,500$):**
  $$\text{Size}_{\text{delta\_flat\_2500}} = (10 \times 750) + (500 \times 50) + (10 \times 40) = 7,500 + 25,000 + 400 = 32,900\text{ bytes}$$
* **Total Connections Volume over 15s:**
  $$\text{Vol}_{\text{conn}} = 1,875,000 + (14 \times 32,900) = 1,875,000 + 460,600 = 2,335,600\text{ bytes} = 2,335.6\text{ KB}$$
* **Total Optimized Throughput (including Throttled Traffic and Logs):**
  $$\text{Total Optimized}_{2500} = 2,335,600\text{ bytes} + 300\text{ bytes} + 300,000\text{ bytes} = 2,635,900\text{ bytes} \approx \mathbf{2.64\text{ MB}} \quad (\text{90.7\% reduction})$$
Both scenarios are well below the target Clash Verge threshold of **4.4MB**.

##### 3. Viewport-Based Pagination under Extreme Load ($N = 5,000$)
Under extreme loads of $N = 5,000$ active connections, sending a full initial snapshot of $3.75\text{ MB}$ to a fresh subscription would consume 85% of the 4.4MB target budget in a single frame. To prevent this, we mandate **Viewport-Based Pagination**.
- The backend/frontend pagination limits updates strictly to connections currently visible in the UI viewport (e.g., page size $P = 100$, typically sorted by active bandwidth).
- **Initial Snapshot Size ($P=100$):**
  $$S_{\text{snapshot\_viewport}} = 100 \times 750\text{ bytes} = 75,000\text{ bytes} = 75\text{ KB}$$
- **Delta Size per second ($P=100$):**
  The update rate is capped by the viewport size. Even if all 100 connections are active ($R_{\text{up}} = 100/s$) and churn within the page is $R_{\text{add}} = 5/s, R_{\text{rem}} = 5/s$:
  $$\text{Size}_{\text{delta\_viewport}} = (5 \times 750) + (100 \times 50) + (5 \times 40) = 3,750 + 5,000 + 200 = 8,950\text{ bytes}$$
- **Total Connections Volume over 15s:**
  $$\text{Vol}_{\text{conn\_viewport}} = 75,000 + (14 \times 8,950) = 75,000 + 125,300 = 200,300\text{ bytes} = 200.3\text{ KB}$$
- **Total Viewport-Paginated Throughput (including Throttled Traffic and Logs):**
  $$\text{Total Viewport Paginated} = 200,300\text{ bytes} + 300\text{ bytes} + 300,000\text{ bytes} = 500,600\text{ bytes} \approx \mathbf{0.50\text{ MB}}$$
By decoupling the IPC throughput from total backend connections, viewport-based pagination guarantees that fresh subscriptions under any connection count (even 5,000 or 10,000) remain extremely lightweight, well within the **4.4MB** target.
