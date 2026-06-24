# Project: Clash Verge/Mini Comprehensive Code Audit

## Architecture
The project is a Tauri-based application:
- **Frontend**: TypeScript, React, and Material-UI (MUI). Located under `src/` (or frontend roots). It manages speed test triggers, latency rendering, and configuration switches.
- **Backend**: Rust (Tauri core). Located under `src-tauri/`. It manages system configuration, backend speed test logic, latency computation, and file I/O operations.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | Frontend Speed Test & Latency Audit | Audit of TS/React speed test mode switching, latency display components, rendering logic, and frontend core states. | None | DONE |
| 2 | Backend Rust Logic & Exception Audit | Audit of Tauri command handlers, unhandled Result/Option, error handling, panic risks, concurrency safety in speed tests. | None | DONE |
| 3 | Layout, MUI Styling & WebView2 Audit | Audit of CSS/MUI styling anomalies under WebView2 and different OS environments, font/icon scaling, and viewport rendering bugs. | None | DONE |
| 4 | Audit Report Synthesis | Synthesize findings from Milestones 1, 2, 3 into a single cohesive report docs/comprehensive_code_audit_report.md. | M1, M2, M3 | DONE |

## Interface Contracts
- **Tauri Commands**: Frontend-to-backend communication via invoke. Check Tauri command payloads and return values (e.g. results of speed tests, configuration options).
- **Core State**: Verification of how latency data and speed test modes flow between TS and Rust.
