# Audit Report: High-Frequency & High-Volume Tauri IPC Events in ClashVerge

## 1. Executive Summary
This audit investigates the Rust backend of ClashVerge (specifically `crates/tauri-plugin-mihomo` and `src-tauri`) to map all high-frequency and high-volume Tauri IPC events. The primary events identified are the WebSocket proxy channels `/traffic`, `/connections`, `/memory`, and `/logs`. Among these, `/connections` is identified as the highest-volume channel (transmitting large JSON objects containing active connection states every second), while `/logs` represents the highest-frequency channel (dynamic, event-driven stream of log events).

Backend optimization strategies have been proposed to mitigate serialization overhead, IPC message overhead, and CPU load on both the Rust backend and the WebView frontend.

---

## 2. IPC Event Registry & Subscription Map

The Rust backend handles high-frequency events by proxying the Clash/Mihomo core API. Below is the mapping of all high-frequency events, their registration points, and the exact files and lines involved.

| Event Type / Path | Tauri Command | Target WebSocket Endpoint | Registration File & Line | Emission Loop (Backend Task) |
|---|---|---|---|---|
| **Traffic Updates** (`/traffic`) | `ws_traffic` | `ws://{host}:{port}/traffic?token={secret}` | `crates/tauri-plugin-mihomo/src/commands.rs:239-248` | `crates/tauri-plugin-mihomo/src/mihomo.rs:250-287` (TCP)<br>`crates/tauri-plugin-mihomo/src/mihomo.rs:316-353` (Local Socket) |
| **Memory Stats** (`/memory`) | `ws_memory` | `ws://{host}:{port}/memory?token={secret}` | `crates/tauri-plugin-mihomo/src/commands.rs:251-260` | `crates/tauri-plugin-mihomo/src/mihomo.rs:250-287` (TCP)<br>`crates/tauri-plugin-mihomo/src/mihomo.rs:316-353` (Local Socket) |
| **Active Connections** (`/connections`) | `ws_connections` | `ws://{host}:{port}/connections?token={secret}` | `crates/tauri-plugin-mihomo/src/commands.rs:263-272` | `crates/tauri-plugin-mihomo/src/mihomo.rs:250-287` (TCP)<br>`crates/tauri-plugin-mihomo/src/mihomo.rs:316-353` (Local Socket) |
| **Logs Stream** (`/logs`) | `ws_logs` | `ws://{host}:{port}/logs?token={secret}&level={level}` | `crates/tauri-plugin-mihomo/src/commands.rs:275-286` | `crates/tauri-plugin-mihomo/src/mihomo.rs:250-287` (TCP)<br>`crates/tauri-plugin-mihomo/src/mihomo.rs:316-353` (Local Socket) |

### Backend-Internal Event Subscriptions (Tray Icon)
- **File**: `src-tauri/src/utils/connections_stream.rs` (Lines 76-97)
- **Function**: `connect_traffic_stream`
- **Purpose**: Subscribes internally to `/traffic` via the Rust client and forwards events to a bounded `tokio::sync::mpsc::channel` with a capacity of 8 (`MIHOMO_WS_STREAM_BUFFER_SIZE`).
- **Consumer**: `src-tauri/src/core/tray/speed_task.rs` (macOS Tray Speed Controller) uses these events to render speed titles on the status bar at a regular rate.

---

## 3. Data Structs & Payload Types

The following Rust structs (defined in `crates/tauri-plugin-mihomo/src/models.rs`) represent the payload structures for each respective endpoint:

### 3.1. Traffic Payload (`Traffic`)
```rust
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Traffic {
    #[ts(type = "number")]
    pub up: u64,
    #[ts(type = "number")]
    pub down: u64,
    #[serde(rename = "upTotal", skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub up_total: Option<u64>,
    #[serde(rename = "downTotal", skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub down_total: Option<u64>,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}
```

### 3.2. Memory Payload (`Memory`)
```rust
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Memory {
    pub inuse: u64,
    pub oslimit: u64,
}
```

### 3.3. Logs Payload (`Log`)
```rust
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Log {
    #[serde(rename = "type")]
    pub log_type: String,
    pub payload: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}
```

### 3.4. Connections Payload (`Connections`, `Connection`, `ConnectionMetaData`)
```rust
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Connections {
    #[ts(type = "number")]
    pub download_total: u64,
    #[ts(type = "number")]
    pub upload_total: u64,
    pub connections: Option<Vec<Connection>>,
    pub memory: u64,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: String,
    pub metadata: ConnectionMetaData,
    #[ts(type = "number")]
    pub upload: u64,
    #[ts(type = "number")]
    pub download: u64,
    pub start: String,
    pub chains: Vec<String>,
    #[serde(default)]
    pub provider_chains: Option<Vec<String>>,
    pub rule: String,
    pub rule_payload: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionMetaData {
    pub network: Network,
    #[serde(rename = "type")]
    pub connection_type: ConnectionType,
    #[serde(rename = "sourceIP")]
    pub source_ip: String,
    #[serde(rename = "destinationIP")]
    pub destination_ip: String,
    #[serde(rename = "sourceGeoIP")]
    pub source_geo_ip: Option<Vec<String>>,
    #[serde(rename = "destinationGeoIP")]
    pub destination_geo_ip: Option<Vec<String>>,
    #[serde(rename = "sourceIPASN")]
    pub source_ip_asn: String,
    #[serde(rename = "destinationIPASN")]
    pub destination_ip_asn: String,
    pub source_port: String,
    pub destination_port: String,
    #[serde(rename = "inboundIP")]
    pub inbound_ip: String,
    pub inbound_port: String,
    pub inbound_name: String,
    pub inbound_user: String,
    pub host: String,
    pub dns_mode: DNSMode,
    pub uid: u32,
    pub process: String,
    pub process_path: String,
    pub special_proxy: String,
    pub special_rules: String,
    pub remote_destination: String,
    pub dscp: u8,
    pub sniff_host: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}
```

---

## 4. Frequency, Buffer, and Loop Analysis

### 4.1. Emission Frequencies
- **Traffic, Memory, and Connections**: The Clash/Mihomo core publishes updates to `/traffic`, `/memory`, and `/connections` WebSockets once every **1 second (1Hz)**.
- **Logs**: Event-driven. When Clash/Mihomo produces a log, it is emitted **immediately** over `/logs`. Under active network traffic or debug mode, this can spike to **hundreds of events per second**.

### 4.2. Loop Mechanism
- The `tauri-plugin-mihomo` WebSocket reader spawns a tokio task (`tokio::spawn(async move { ... })`) in `crates/tauri-plugin-mihomo/src/mihomo.rs`.
- This loop reads from the split WebSocket stream using `reader.next().await`. It does not contain any throttling, delay, or buffering; it forwards messages to the Tauri IPC `Channel` immediately upon receipt.
- **Zero-Copy Payload Hand-off**: The WebSocket text frame is converted directly to `InvokeResponseBody::Raw(text.as_bytes().to_vec())`. This means the Rust backend **does not deserialize or re-serialize** the JSON payload. Instead, it acts as a transparent byte proxy, offloading all parsing overhead to the WebView frontend.

### 4.3. Buffer Constraints & Backpressure
- **Tauri IPC Channel**: If the frontend window is closed or the receiver is dropped, `on_message.send()` returns an error, which causes the backend to immediately clean up the task and close the WebSocket connection.
- **Tray Speed Stream**: In `src-tauri/src/utils/connections_stream.rs`, the internal `tokio::sync::mpsc::channel` uses a bounded buffer size of **8** (`MIHOMO_WS_STREAM_BUFFER_SIZE`). If the buffer is full (indicating the consumer task is busy), events are dropped (`try_send_internal_event`), preventing memory bloat.

---

## 5. Suggested Back-End Optimization Strategies

To reduce the CPU overhead on both the Rust backend and WebView frontend, we recommend the following optimization strategies:

### 5.1. Global Window Visibility Tracking (Rust-Driven Subscription Control)
- **Mechanism**: Implement a centralized window visibility/state store on the Rust backend that listens to Tauri window event hooks (e.g., `is_minimized()`, `on_window_event`).
- **Strategy**: When the application window is minimized or hidden, the backend should temporarily suspend reading from the WebSockets or pause pushing to the Tauri `Channel`. Currently, the frontend handles closing and reopening subscriptions, but a backend guard provides a robust defense-in-depth against zombie frontend subscriptions.

### 5.2. Log Batching & Debouncing
- **Mechanism**: Rather than forwarding every single log entry as a separate Tauri IPC invocation, introduce a micro-batching buffer.
- **Strategy**: Accumulate log messages in a thread-safe buffer (e.g., a `Vec<String>`) and flush them to the frontend using a periodic timer (e.g., every **200ms** or **500ms**). This groups hundreds of individual IPC calls into a single array payload, dramatically lowering message passing overhead and context switching.

### 5.3. Payload Filtering (Selective Deserialization)
- **Mechanism**: Define filtering masks for high-volume endpoints (particularly `/connections`).
- **Strategy**: The `/connections` payload is massive because it includes deeply nested connection metadata (e.g., GeoIP arrays, ASN details, rules, process paths) which may not be visible in standard connection views. The backend can filter or remove fields before forwarding the payload, or the frontend can specify a set of required fields when initiating the `Channel`.

### 5.4. Delta/Diff Updates for Connections
- **Mechanism**: Maintain the state of active connections on the backend and send only differential updates.
- **Strategy**: Calculate the diff (new connections, closed connections, bandwidth changes) on the Rust side, and send only the diff. This avoids transmitting the entire list of connections every second, reducing IPC payload volume by up to **90%** in environments with high numbers of concurrent connections.
