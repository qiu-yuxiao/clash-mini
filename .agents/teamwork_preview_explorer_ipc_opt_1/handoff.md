# Handoff Report: Audit of High-Frequency & High-Volume Tauri IPC Events

## 1. Observation
- **Tauri IPC Command Registration**: In `crates/tauri-plugin-mihomo/src/commands.rs` (lines 239-286), four WebSocket proxy commands are registered:
  - `ws_traffic` (line 239)
  - `ws_memory` (line 251)
  - `ws_connections` (line 263)
  - `ws_logs` (line 275)
  Each command takes a Tauri 2.0 `Channel<InvokeResponseBody>` to stream data back to the frontend.
- **WebSocket Streaming Loop**: In `crates/tauri-plugin-mihomo/src/mihomo.rs`, the websocket reader task is spawned at line 250 (TCP) and line 316 (Local Socket).
  - Loop extracts frames: `message = reader.next() =>` (lines 260, 326)
  - Converts immediately to raw bytes body: `InvokeResponseBody::Raw(text.as_bytes().to_vec())` (lines 43-44, 51)
  - Emits over channel: `on_message.send(data).is_ok()` (lines 246, 258, 270, 283 in `commands.rs`)
- **Backend Payload Structs**: In `crates/tauri-plugin-mihomo/src/models.rs`:
  - `Traffic` (line 1019) has fields: `up`, `down`, `up_total`, `down_total`.
  - `Memory` (line 1039) has fields: `inuse`, `oslimit`.
  - `Log` (line 1047) has fields: `log_type` (`type`), `payload`.
  - `Connections` (line 878) has fields: `download_total`, `upload_total`, `connections: Option<Vec<Connection>>`, `memory`.
- **Backend Internal Stream**:
  - `src-tauri/src/utils/connections_stream.rs` (line 76) defines `connect_traffic_stream` subscribing to `/traffic` internally via `ws_traffic`. It uses a bounded channel size of `8` (`MIHOMO_WS_STREAM_BUFFER_SIZE`).
  - `src-tauri/src/core/tray/speed_task.rs` (line 48) consumes speed events to render status titles on macOS.

## 2. Logic Chain
- **Step 1**: The Rust backend proxies the Clash/Mihomo websocket API endpoints (`/traffic`, `/memory`, `/connections`, `/logs`) directly to the frontend.
- **Step 2**: The backend's proxy task in `mihomo.rs` loop executes as fast as data arrives from the WebSocket connection, offering no internal throttle or batching.
- **Step 3**: The `/traffic`, `/memory`, and `/connections` updates are naturally paced at 1Hz (once per second) by the Clash/Mihomo core.
- **Step 4**: The `/logs` updates are event-driven and can flood the system at high frequency (hundreds of Hz) in verbose logs / debug mode, creating IPC overhead.
- **Step 5**: The `/connections` payload is potentially very large (containing full lists of active connections and nested metadata), creating high serialization, transfer, and rendering costs every second.
- **Step 6**: The Rust backend does not deserialize the WebSocket messages; it acts as a transparent byte proxy, avoiding Rust-side deserialization CPU costs but passing all raw JSON parsing overhead directly to WebView.

## 3. Caveats
- Did not profile the actual CPU/memory usage of these IPC channels under load (e.g. hundreds of concurrent connections). Assumed the standard performance characteristics of Tauri IPC channel serialization.
- Assumed standard Clash/Mihomo core behavior where `/traffic`, `/memory`, and `/connections` are fixed at 1Hz interval.

## 4. Conclusion
The high-frequency events (`/traffic`, `/memory`, `/connections`, `/logs`) are proxy-routed through Tauri commands in `tauri-plugin-mihomo` using raw byte Tauri Channels.
To optimize:
1. Implement log batching/debouncing on the backend (e.g., 200ms flushes of array payloads).
2. Filter the `/connections` payload fields or implement backend delta updates to avoid sending the full connection list every second.
3. Track window visibility state on the backend to pause WebSocket readers when the application window is minimized or hidden.

## 5. Verification Method
- Code auditing can be verified by inspecting `crates/tauri-plugin-mihomo/src/commands.rs`, `crates/tauri-plugin-mihomo/src/mihomo.rs`, and `crates/tauri-plugin-mihomo/src/models.rs`.
- The compilation status of the project can be verified by running:
  `cargo check` in the workspace directory.
