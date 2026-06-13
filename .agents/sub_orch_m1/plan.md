# Plan - Milestone 1: Frontend CPU & IPC Optimization

This plan coordinates the optimization of the React frontend visibility check and WebSocket subscriptions.

## Steps

### Step 1: Exploration and Audit
- Spawn 3 Explorer subagents to independently audit and suggest optimization strategies for:
  - `src/hooks/use-visibility.ts`
  - `src/hooks/use-traffic-monitor.ts`
  - `src/hooks/use-log-data.ts`
- Compare explorer reports, synthesize findings, and decide the final implementation plan.

### Step 2: Implementation
- Spawn a Worker subagent to implement the changes according to the synthesized plan:
  - Optimize `use-visibility.ts` to hook into Tauri's window minimized/focused events (e.g., using `@tauri-apps/api/event` or `@tauri-apps/api/window` APIs).
  - Stop/disconnect WebSocket connections for traffic monitor, connections, and logs when `isVisible` (from `useVisibility`) is false.
  - Stop traffic client when `isVisible` is false in `useTrafficMonitorEnhanced`.
- Worker must build and run existing tests to verify that no regressions were introduced.

### Step 3: Verification & Challenger
- Spawn 2 Reviewers to review code correctness, completeness, and interface conformance.
- Spawn 2 Challengers to write/run verification tests or stress tests to verify WebSocket disconnection behavior.

### Step 4: Forensic Audit
- Spawn a Forensic Auditor to perform integrity validation checks, checking for clean execution and no cheating.

### Step 5: Handoff
- Write the final handoff report in `.agents/sub_orch_m1/handoff.md` and notify the parent orchestrator.
