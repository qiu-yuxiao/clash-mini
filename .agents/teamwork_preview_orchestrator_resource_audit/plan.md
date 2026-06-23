# System Resource Optimization Audit Plan

## Objective
Thoroughly audit the frontend (React/TS) and backend (Rust/Tauri) of Clash Mini for system resource improvements (CPU, memory, threads, handles, I/O) without modifying any source files. Deliver a point-by-point recommendation report with code diffs.

## Execution Steps
1. **Step 1: Setup and Initialize** (Done)
   - Create orchestration files: `plan.md`, `progress.md`, `context.md`, `BRIEFING.md`, `PROJECT.md`.
   - Setup a heartbeat cron for liveness tracking.

2. **Step 2: Milestone 1 - Frontend Resource Optimization Audit**
   - Dispatch Explorer to analyze frontend React/TS components, hooks, render cycles, visibility API subscriptions, and IPC listeners.
   - Analyze potential CPU and memory bottlenecks (e.g. unthrottled traffic/connection streams).
   - Produce a detailed findings report.

3. **Step 3: Milestone 2 - Backend Concurrency & Task Optimization Audit**
   - Dispatch Explorer to analyze Rust backend async tasks (`tokio::spawn`), locking strategies (`Mutex`, `RwLock`), thread synchronization, and CPU-intensive polling loops.
   - Map concurrency issues that consume unnecessary cycles or memory.
   - Produce a detailed findings report.

4. **Step 4: Milestone 3 - Backend I/O & Socket Optimization Audit**
   - Dispatch Explorer to analyze file I/O operations (config saves, log writes), socket handles management, potential handle leaks, and network IPC buffering.
   - Identify redundant disk writes or unbuffered reads.
   - Produce a detailed findings report.

5. **Step 5: Milestone 4 - Synthesis and Report Drafting**
   - Aggregate M1, M2, and M3 reports.
   - Draft non-intrusive optimization recommendations with precise file paths, line ranges, and suggested diffs/pseudo-code.
   - Verify layout and formatting constraints.
   - Compile into final report.

6. **Step 6: Completion and Handback**
   - Ensure working directory is 100% clean (run git status).
   - Mark progress.md as completed.
   - Message parent agent with findings and final report path.
