# Handoff Report — Global IPC Optimization Proposal

## 1. Observation
- Verified that key frontend hooks and components reside at:
  - `src/hooks/use-connection-data.ts`
  - `src/hooks/use-traffic-data.ts`
  - `src/hooks/use-visibility.ts`
  - `src/components/home/enhanced-canvas-traffic-graph.tsx`
- Verified that key Rust backend plugin files and methods reside at:
  - `crates/tauri-plugin-mihomo/src/commands.rs` (containing commands `ws_traffic`, `ws_memory`, `ws_connections`, `ws_logs`)
  - `crates/tauri-plugin-mihomo/src/mihomo.rs` (containing `connect` and checking loops)
  - `crates/tauri-plugin-mihomo/src/models.rs` (defining structs `Traffic`, `Memory`, `Log`, `Connections`, `Connection`, `ConnectionMetaData`)
- Observed that the fallback polling block in `useConnectionData` (lines 123-153) contains the early-return condition `if (isWsActive || !isVisible || !enabled) return;`, which always evaluates to `true` (since `isWsActive` is `enabled && isVisible`).
- Wrote the complete proposal to the workspace at `docs/ipc_optimization_proposal.md`.

## 2. Logic Chain
- The user requested a comprehensive global optimization proposal to profile frontend-backend IPC data payloads, identify root causes of Clash Mini's 38MB+ throughput in 15 seconds, and design an optimization scheme to reduce it close to Clash Verge's level (~4.4MB).
- Based on the audited code and the findings from the three Explorer subagents' reports, the root causes are:
  1. Full connection lists sent every second (scaling $O(N)$ with connection count, averaging 750 bytes per object).
  2. Background visibility leaks due to `useVisibility` only tracking DOM visibility and not native Tauri window minimization/tray hiding states.
  3. Redundant background REST polling calling `getConnections()` (which fetches the full list) just to extract upload and download totals.
  4. Event-driven debug log streams sending unbatched logs line-by-line.
- We designed a **Differential Update Protocol (Delta Push)** consisting of a full `Snapshot` on connection, and periodic `Delta` updates carrying only `added` connections, `updated` bandwidth metrics, and `removed` connection IDs.
- Evaluated two update payload formats and recommended the **Option B: Ultra-Compact Tuple** representation (`[id, upload, download]`) because it eliminates object key names, yielding a ~37.5% bandwidth saving over Key-Value Structs.
- Designed native window visibility gating on both the frontend and backend, along with dynamic throttling of the connection list table (increasing throttle from 16ms to 1000ms/2000ms) and log batching.
- Constructed a theoretical mathematical model that proves:
  - Background throughput drops from 15.30MB to 0.00MB.
  - Visible closed-drawer throughput drops from 4.05MB to ~300.9KB.
  - Active monitoring throughput with 1,000 active connections drops from 11.55MB to ~1.22MB, which is well below the target baseline of 4.4MB.

## 3. Caveats
- The proposed differential protocol assumes reliable, in-order delivery of WebSocket messages (which TCP guarantees). In case of connection drops, the client must trigger a clean reconnect to fetch a new full snapshot.
- Native window visibility events on the backend assume standard Tauri single/main window visibility hooks, which would need minor adjustments if the app uses multiple windows in the future.

## 4. Conclusion
- The complete design proposal document has been successfully generated and written to the workspace at `docs/ipc_optimization_proposal.md`. All required sections, schemas, files, and mathematical estimations are included with no placeholders or TODOs.

## 5. Verification Method
- **File Inspection**: Directly inspect `docs/ipc_optimization_proposal.md` to verify it is fully formatted in Markdown and contains all required specifications.
- **Section Checklist**:
  - Event registry map with Rust and TS code references.
  - Root cause diagnostics for the 38MB+ discrepancy.
  - Detailed differential update protocol.
  - Concrete Rust and TypeScript schemas.
  - Native window visibility gating and throttling design.
  - Theoretical mathematical estimation demonstrating throughput reduction below 4.4MB.
