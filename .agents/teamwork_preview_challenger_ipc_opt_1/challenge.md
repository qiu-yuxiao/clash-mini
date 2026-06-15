# Challenge Report — 2026-06-15T04:45:00+08:00

## Challenge Summary

**Overall risk assessment**: HIGH
While the proposed Differential Update Protocol (Delta Push) is a significant improvement over the current $O(N)$ full snapshot broadcasting and will drastically reduce bandwidth under normal conditions, the mathematical model contains several flaws and overlooks key overheads. Under extreme conditions ($N = 5,000$ active connections), the target of reducing IPC volume to **~4.4MB** in a 15-second window is **not achievable** for fresh subscriptions, and is only barely achievable in steady state if active connections transferring data are kept low. Additionally, the proposal introduces new CPU and memory overheads that were not evaluated.

---

## Challenges

### [High] Challenge 1: Fresh Subscription Snapshot Overhead under Extreme Load ($N=5,000$)
- **Assumption challenged**: The proposal assumes that the 15-second optimization target of ~4.4MB is easily satisfied under extreme conditions.
- **Attack scenario**: A user opens the connections drawer during a heavy P2P session (e.g. 5,000 active connections). The backend must immediately emit a full `Snapshot` payload containing all 5,000 connections.
- **Blast radius**: 
  - A single snapshot for 5,000 connections is $5,000 \times 750\text{ bytes} = 3.75\text{ MB}$.
  - Over a 15-second window, even with the compact Tuple format and moderate churn ($R_{\text{add}} = 25/s$, $R_{\text{up}} = 1,000/s$), the delta updates add $14 \times 69.75\text{ KB} = 976.5\text{ KB}$.
  - Combined with traffic updates and logs, the total IPC throughput over 15 seconds is **5.03 MB** (decimal) or **4.79 MiB** (binary), which exceeds the 4.4MB target.
  - Under heavy active P2P ($R_{\text{up}} = 2,000/s$), this rises to **6.00 MB**, and under worst-case P2P ($R_{\text{up}} = 5,000/s$), it surges to **8.66 MB**, completely violating the target.
- **Mitigation**: 
  - **Paginated/Windowed Updates**: The backend should only emit snapshot and delta updates for the connections currently visible in the UI viewport (e.g., top 50-100 connections sorted by bandwidth or active status), rather than all 5,000 connections.
  - **Lazy Metadata Loading**: The snapshot should only contain connection IDs and bandwidth; static metadata (process name, process path, IPs) should only be requested when the user clicks to open the details drawer for a specific connection.

### [Medium] Challenge 2: Mathematical Inconsistencies and Mixed Unit Bases
- **Assumption challenged**: The calculations of the proposed optimized throughput are mathematically correct and consistent.
- **Attack scenario**: Direct addition of binary and decimal values in the formula leads to inaccurate bounds.
- **Blast radius**:
  - The proposal states: $S_{\text{snapshot}} = N \times S_{\text{full}} = 750\text{ KB}$ for $N=1,000$. This is decimal ($750,000\text{ bytes} / 1000 = 750\text{ KB}$).
  - It then computes $\text{Size}_{\text{delta\_tuple}} = 13,950\text{ bytes} \approx 13.62\text{ KB}$ using binary division ($13,950 / 1024 = 13.623\text{ KB}$).
  - It adds them: $750\text{ KB} + 14 \times 13.62\text{ KB} = 940.68\text{ KB}$ (adding decimal KB and binary KB).
  - It then converts this to $0.92\text{ MB}$ using binary division ($940.68 / 1024 = 0.9186\text{ MB}$).
  - Finally, it adds logs: $0.92\text{ MB}$ (binary) $+ 0.30\text{ MB}$ (decimal $300\text{ KB}$) $= 1.22\text{ MB}$ (mixed).
  - The correct total bytes should be $750,000 + 14 \times 13,950 + 900\text{ (traffic)} + 300,000\text{ (logs)} = 1,246,200\text{ bytes}$.
  - This is actually **1.25 MB** (decimal) or **1.19 MiB** (binary).
- **Mitigation**: Standardize all calculations on either binary units (KiB, MiB using base-2) or decimal units (KB, MB using base-10) to ensure mathematical consistency.

### [Medium] Challenge 3: TypeScript Garbage Collection (GC) Pressure from Nested Tuples
- **Assumption challenged**: Sending compact JSON array tuples `[id, upload, download]` has no negative performance impact.
- **Attack scenario**: At $N = 5,000$ and $R_{\text{up}} = 2,000$, the frontend receives an array of 2,000 tuples every second.
- **Blast radius**:
  - V8 must allocate 2,000 individual array objects and 2,000 string IDs every second, then discard them in the next frame.
  - This results in high garbage collection pressure, leading to frequent GC pauses (stop-the-world sweeps) that cause micro-stutters and frame drops in the WebView UI.
- **Mitigation**:
  - **Flat Array Serialization**: Instead of nested arrays `[[id1, up1, down1], [id2, up2, down2]]`, the backend should serialize updates as a single flat array of alternating values: `[id1, up1, down1, id2, up2, down2, ...]`. This reduces the number of allocated array objects from $2,001$ to just $1$ per second (a 99.95% reduction in object allocations).
  - **Binary Serialization**: Use typed arrays (e.g. Uint8Array/Float64Array) via Tauri's binary protocol, bypassing V8 JSON string parsing entirely.

### [Low] Challenge 4: Shifting CPU Overhead from Frontend to Backend
- **Assumption challenged**: The backend transparent proxying replacement has no performance cost.
- **Attack scenario**: To compute the diff, the Rust backend must deserialize the full $O(N)$ JSON payload from the Clash core WebSocket, maintain a map of active connections, compute the delta, and serialize the custom Delta structure.
- **Blast radius**:
  - While this keeps the WebView UI thread smooth, it shifts significant CPU deserialization/serialization work to the Rust backend (especially with $N = 5,000$ connections parsed every second).
- **Mitigation**:
  - Optimize the Rust deserialization using lightweight JSON parsers (e.g. `simd-json` or selective parsing) to extract only needed fields before computing the delta.

---

## Stress Test Results

We simulated the IPC payload volume (in bytes) over a **15-second window ($T=15$)** across different scenarios:

| Scenario | $N$ | Churn ($R_{\text{add}}$ / $R_{\text{rem}}$) | Active ($R_{\text{up}}$) | Proposed Model (Mixed base) | Verified Decimal (Base-10) | Verified Binary (Base-2) | Target (<4.4MB) Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1. Proposal Baseline** | 1,000 | 5/s | 200/s | 1.22 MB | **1.25 MB** | **1.19 MiB** | **PASS** |
| **2. Low Churn / Active (5k)** | 5,000 | 5/s | 200/s | N/A | **4.25 MB** | **4.05 MiB** | **PASS** |
| **3. Moderate Churn / Active (5k)** | 5,000 | 25/s | 1,000/s | N/A | **5.03 MB** | **4.79 MiB** | **FAIL** (Fresh sub exceeds) |
| **4. Heavy Churn / Active (5k)** | 5,000 | 50/s | 2,000/s | N/A | **6.00 MB** | **5.73 MiB** | **FAIL** (Fresh sub exceeds) |
| **5. Worst Case P2P (5k)** | 5,000 | 100/s | 5,000/s | N/A | **8.66 MB** | **8.26 MiB** | **FAIL** (Fresh sub & steady state exceed) |

*Note: Steady state (excluding the initial snapshot) for Scenario 3 & 4 stays below the 4.4MB budget (1.35MB and 2.39MB respectively), but Scenario 5 steady state exceeds it (5.24 MB).*

---

## Unchallenged Areas
- **Visibility-based gating logic**: The design to suspend all IPC subscriptions when the window is minimized or hidden in the system tray is highly robust and will successfully achieve a 100% reduction in background traffic.
- **Log batching design**: Batching logs to flush every 250ms or 50 entries is an excellent way to reduce IPC message dispatching frequency.

---

## Attack Surface
- **Hypotheses tested**: 
  - Tested whether the 4.4MB target is achievable under extreme loads ($N = 5,000$). Result: Found that it fails for fresh subscriptions in all realistic scenarios and fails in steady state for worst-case P2P scenarios.
  - Tested mathematical consistency of the proposal. Result: Discovered mixed binary (base-2) and decimal (base-10) bases in the proposal calculations.
- **Vulnerabilities found**:
  - Single point of failure: Initial snapshot size scales linearly ($O(N)$) and becomes a bottleneck at $N=5,000$ ($3.75\text{ MB}$), consuming $85\%$ of the budget in a single message.
  - Garbage collection pressure: High frequency nested array allocation leading to micro-stutters.
  - CPU shift: Rust backend overhead to deserialize and diff 5,000 connections every second.
- **Untested angles**:
  - The impact of actual IPC serialization/deserialization CPU time on Windows WebView2.
  - The impact of core WebSocket connection dropouts (reconnection loops triggering multiple snapshots).

---

## Loaded Skills
- None
