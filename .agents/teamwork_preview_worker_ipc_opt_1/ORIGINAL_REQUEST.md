## 2026-06-14T20:34:20Z

Your working directory is: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_worker_ipc_opt_1
Your task is to write a comprehensive global optimization proposal document to profile frontend-backend IPC data payloads, identify the root causes of the high IPC communication throughput (38MB+ in 15 seconds) in Clash Mini, and propose a global optimization design to reduce the throughput close to Clash Verge's level (~4.4MB).

Please write the final, complete design proposal document to the workspace at `docs/ipc_optimization_proposal.md`.

Use the findings from the three Explorer subagents' reports:
1. Explorer 1 findings on Rust backend emitters: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_ipc_opt_1\analysis.md
2. Explorer 2 findings on TypeScript frontend listeners: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_ipc_opt_2\analysis.md
3. Explorer 3 findings on payload structures, root causes, and diff design: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_ipc_opt_3\analysis.md

The proposal MUST satisfy these requirements:
1. Fully formatted in Markdown with no placeholders or TODOs.
2. List all active high-frequency IPC events (including `/traffic`, `/connections`, `/logs`, `/memory`), their payload sizes, and their corresponding code files in both the Rust backend and TypeScript frontend.
3. Pinpoint the exact causes of the 38MB payload size discrepancy compared to Clash Verge (~4.4MB).
4. Define a detailed differential update protocol for connections list (e.g. key-value diffing, JSON patch, or delta push) sending only changes (added, updated, removed) instead of full list.
5. Specify concrete data structure schemas in Rust (structs) and TypeScript (interfaces/types) for the differential updates.
6. Detail how updates are throttled or suspended when the application window is hidden or minimized (native window visibility tracking).
7. Include a theoretical mathematical/performance estimation showing how the payload volume will be reduced to ~4.4MB over 15 seconds.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
