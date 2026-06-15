# BRIEFING — 2026-06-15T04:30:20+08:00

## Mission
Analyze and design a global optimization proposal to profile frontend-backend IPC data payloads, identify root causes of high IPC throughput in Clash Mini, and propose a global optimization design to reduce the throughput to ~4.4MB.

## 🔒 My Identity
- Archetype: teamwork_preview_orchestrator
- Roles: orchestrator, user_liaison, human_reporter, successor
- Working directory: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_orchestrator_ipc_opt
- Original parent: main agent
- Original parent conversation ID: abdd5222-8319-4dfc-9336-a134a6300241

## 🔒 My Workflow
- **Pattern**: Project
- **Scope document**: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_orchestrator_ipc_opt\PROJECT.md
1. **Decompose**: We will decompose this into two main milestones:
   - Milestone 1: IPC Bottleneck Analysis and Mapping (Explorer subagent)
   - Milestone 2: Design of Global Diff-Based Optimization Protocol & Documentation (Worker/Reviewer subagents)
2. **Dispatch & Execute**:
   - **Direct (iteration loop)**: We'll run the Explorer -> Worker -> Reviewer cycle directly since we are proposing a design without writing/testing code.
3. **On failure** (in this order):
   - Retry: nudge stuck agent or re-send task
   - Replace: spawn fresh agent with partial progress
   - Skip: proceed without (only if non-critical)
   - Redistribute: split stuck agent's remaining work
   - Redesign: re-partition decomposition
   - Escalate: report to parent (sub-orchestrators only, last resort)
4. **Succession**: at 16 spawns, write handoff.md, spawn successor
- **Work items**:
  1. Audit IPC Event Bottlenecks [pending]
  2. Design Global Diff-Based Optimization Protocol [pending]
  3. Write Proposal Document docs/ipc_optimization_proposal.md [pending]
- **Current phase**: 1
- **Current focus**: Audit IPC Event Bottlenecks

## 🔒 Key Constraints
- NEVER write, modify, or create source code files directly.
- NEVER run build/test commands yourself — require workers to do so.
- You MAY use file-editing tools ONLY for metadata/state files (.md) in your .agents/ folder.
- Never reuse a subagent after it has delivered its handoff — always spawn fresh

## Current Parent
- Conversation ID: abdd5222-8319-4dfc-9336-a134a6300241
- Updated: not yet

## Key Decisions Made
- Use Project Orchestrator pattern to structure files under .agents/teamwork_preview_orchestrator_ipc_opt.
- Milestone 1: Static analysis of IPC emitter/listener bottlenecks.
- Milestone 2: Formulate optimization protocol and write design proposal.

## Team Roster
| Agent | Type | Work Item | Status | Conv ID |
|-------|------|-----------|--------|---------|
| explorer_1 | teamwork_preview_explorer | Audit Rust backend emitters | completed | c48ea6ce-2c38-4ec4-aed5-c1e2ec70fc57 |
| explorer_2 | teamwork_preview_explorer | Audit TS frontend listeners | completed | e075e2b9-790b-499c-9f88-3ec73528a266 |
| explorer_3 | teamwork_preview_explorer | Audit payload sizes and diff schemas | completed | bd64fc3e-9752-41f5-bc47-0d4da8dfd9a0 |
| worker_1 | teamwork_preview_worker | Write optimization proposal | completed | f7156d3d-fb8d-4ca2-b172-0b0f6b043df1 |
| reviewer_1 | teamwork_preview_reviewer | Review optimization proposal | completed | 6f8fe9c1-82b8-4679-aa3f-48587f56d13f |
| reviewer_2 | teamwork_preview_reviewer | Review optimization proposal | completed | 404004ad-31e9-4852-b5b3-1b3fa6138c6e |
| challenger_1 | teamwork_preview_challenger | Challenge payload calculations | completed | ba6564be-3b96-499e-af7d-3180067ed5e3 |
| challenger_2 | teamwork_preview_challenger | Challenge visibility design | completed | 1f102656-0dd3-4380-ae07-5a11c75073f1 |
| auditor_1 | teamwork_preview_auditor | Verify proposal integrity | completed | 096fe3ad-d5f0-472b-a0dc-b4a06bdf8a9b |
| worker_2 | teamwork_preview_worker | Update optimization proposal | completed | ac22f13e-1a88-4d1c-a03c-fdff1c8bb6a4 |

## Succession Status
- Succession required: no
- Spawn count: 10 / 16
- Pending subagents: none
- Predecessor: none
- Successor: not yet spawned

## Active Timers
- Heartbeat cron: task-9
- Safety timer: none
- On succession: kill all timers before spawning successor
- On context truncation: run `manage_task(Action="list")` — re-create if missing

## Artifact Index
- c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_orchestrator_ipc_opt\PROJECT.md — Global architecture, milestones and scope
- c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_orchestrator_ipc_opt\progress.md — Liveness and execution progress tracker
