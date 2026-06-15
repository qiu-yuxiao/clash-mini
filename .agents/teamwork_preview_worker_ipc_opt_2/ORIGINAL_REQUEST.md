## 2026-06-14T20:39:21Z

Your working directory is: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_worker_ipc_opt_2
Your task is to update the global optimization proposal document at `docs/ipc_optimization_proposal.md` in the workspace to address the critical, major, and minor feedback raised by the reviewers and challengers.

Specifically, read these review reports:
1. Reviewer 1 review: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_reviewer_ipc_opt_1\review.md
2. Reviewer 2 review: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_reviewer_ipc_opt_2\review.md
3. Challenger 1 challenge: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_challenger_ipc_opt_1\challenge.md
4. Challenger 2 challenge: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_challenger_ipc_opt_2\challenge.md

Please make the following changes in the proposal at `docs/ipc_optimization_proposal.md`:
- **Backwards Compatibility**: Define capability negotiation or a separate command (`ws_connections_v2`) and query parameter (`?delta=true`) so that the backend can fall back to the original raw snapshot format if the client does not support deltas.
- **Re-Sync & Sequence/Epoch Tracking**: Add `sequence_id` (number) and `epoch_id` (string - UUID generated on stream startup) to both `Snapshot` and `Delta` schemas. Explain how the TS frontend handles sequence validation and resets the cache/triggers reconnect to request a new Snapshot if sequence_id is non-sequential.
- **Visual Flash on Window Re-activation**: Clear cache on minimize, display a loading skeleton/translucent overlay during WebSocket reconnection, or fetch a quick REST snapshot immediately on window restore before WebSocket completes.
- **V8 GC Pressure & Flat Array Layout**: Replace the positional compact tuple `[id, upload, download]` with a flat 1D array representation: `[id1, up1, down1, id2, up2, down2, ...]` to reduce V8 object allocation counts by 99.95%, easing GC pressure. Also require virtualized rendering (`react-window`/`react-virtualized`) for the connection table.
- **Connection Thrashing**: Add a 1000ms debounce to window visibility state updates before triggering socket connection teardown or setup.
- **Focus Loss Suspend Correction**: Remove any recommendation to pause on focus loss (`Focused(false)`). Pausing should only occur on minimized and hidden states.
- **Backend Simplification**: Recommend calling the existing `clear_all_ws_connections()` function when windows are minimized/hidden rather than implementing custom pausing/buffering in Rust.
- **Math Model Consistency & Scaling**: Fix the mixed base math (use consistent metric units: e.g. decimal MB or binary MiB). Correct the active update rate scaling for N=2500 (scale R_up from 200 to 500 when active connections scale from 1,000 to 2,500). Explain how viewport-based pagination prevents sending the entire list of N=5,000 connections, keeping active throughput within the 4.4MB target even during fresh subscriptions.

Ensure there are no placeholders or TODOs.
MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
