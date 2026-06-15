# Original User Request

## Initial Request — 2026-06-15T04:30:20+08:00

You are the Project Orchestrator (teamwork_preview_orchestrator).
Your identity: conversation ID to be assigned.
Your working directory is: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_orchestrator_ipc_opt

Your task is to analyze and design a global optimization proposal to profile frontend-backend IPC data payloads, identify the root causes of the high IPC communication throughput (38MB+ in 15 seconds) in Clash Mini, and propose a global optimization design (such as data difference diffs, payload pruning, and event throttling) to reduce the throughput close to Clash Verge's level (~4.4MB) without writing code.

The verbatim user request is recorded at c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\ORIGINAL_REQUEST.md (and also at c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\ORIGINAL_REQUEST.md).

Requirements:
R1. Audit Frontend & Backend IPC Event Bottlenecks
Perform a complete static analysis of the codebase to identify all high-frequency and high-volume Tauri IPC events (e.g., /traffic, /connections, /logs). Map out the exact Rust backend emitter code, the TypeScript frontend listener code, and the average payload structure.

R2. Design Global Diff-Based Optimization Protocol
Design a comprehensive optimization proposal that specifies:
- A differential update protocol (e.g., key-value diffing, JSON patch, or delta push) for heavy states like connection lists.
- Event throttling and visibility-based subscription pausing.
- Backwards-compatible data schemas for both Rust structs and React state hooks.

R3. Output Document
Write the final, complete design proposal document to the workspace at docs/ipc_optimization_proposal.md.

Acceptance Criteria:
- The optimization proposal file exists at docs/ipc_optimization_proposal.md.
- The file is fully formatted in markdown, with no placeholders or TODOs.
- The document lists all active high-frequency IPC events, including payload sizes and their corresponding code files.
- The document pinpoints the exact cause of the 38MB payload size discrepancy compared to Clash Verge.
- The proposal defines concrete data structure schemas (Rust structs and TypeScript types) for differential updates.
- The design specifies how updates are throttled or suspended when the application window is hidden or minimized.
- The proposal includes a theoretical performance estimation showing how the payload volume will be reduced to ~4.4MB.

Please read the codebase files, analyze them, coordinate with subagents if necessary, and write the final document. Once the proposal document is written and verified, report completion back to the Sentinel.
