# Task: Web Worker Lifecycle Memory Leak Investigation

## Objective
Investigate memory usage regression under lightweight mode (when main window is closed/hidden) in Clash Mini v1.8.9 compared to v1.8.2, focusing on the Web Worker instantiation in `use-traffic-monitor.ts` and `traffic.worker.ts`.

## Context
In v1.8.9, Web Worker was introduced to handle traffic monitor data sampling. When window visibility changes, `start()` and `stop()` lifecycle hooks are triggered in `TrafficWorkerClient` / `useTrafficMonitorEnhanced`. This might be leaking resources (threads, JS heap, event listeners) in WebView2.

## Requirements
1. Analyze `src/hooks/use-traffic-monitor.ts` and `src/hooks/traffic.worker.ts`.
2. Inspect the git history or compare with v1.8.2 if possible to understand what was changed.
3. Identify how Web Worker is instantiated, stopped, and cleaned up.
4. Check for potential memory leaks, dangling references, or unclosed listeners.
5. Provide detailed explanation, exact files/functions/lines, and propose a fix/rollback patch as a Git diff block.
6. Write your findings to `.agents/teamwork_preview_explorer_m1/handoff.md`.
