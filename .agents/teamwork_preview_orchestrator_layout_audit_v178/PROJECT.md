# Project: Clash Mini Layout and Styling Audit v1.7.8

## Architecture
- **Frontend Core Pages & Layouts**: `src/pages/_layout.tsx` (the core layout container) and `src/components/layout/window-controller.tsx` (window controls).
- **Styling Mechanisms**: Emotion (CSS-in-JS via `sx` prop) and standard HTML inline style attributes (bypassing Emotion / Content Security Policy constraints).
- **Core Layout Agreements**: `clash_mini_agreements.md` and `clash_mini_pitfalls.md`.
- **Packaging and Builds**: `tauri.conf.json`, `package.json`, Vite configs, and build scripts.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | M1: SvgIcon and Layout Audit (R1) | Static analysis of window controls, Pin button, Settings button, active node loading circular progress, and latency signal icons. Verify exact size constraints and inline styling bypass. | none | DONE (7a8c825f-16fc-4128-b72d-094ff9ee7142) |
| 2 | M2: Proxy List and Accordion Audit (R2) | Audit of proxy list rendering, column config, group headers, and collapsible accordion functionality. | none | DONE (1c298388-e864-4fb2-8bab-c154b14d16bf) |
| 3 | M3: Dependency and Build Consistency (R3) | Audit of package.json, tauri.conf.json, Vite configuration, and compile scripts. | none | DONE (aa86f612-75cf-485a-8dc4-4132662f35b4) |
| 4 | M4: Git Diff and Agreements Compliance (R4) | Review recent Git diffs and cross-reference with clash_mini_agreements.md and pitfalls. | M1, M2, M3 | DONE (7368e2f0-fd9f-4b22-9d09-08ecafe1cab4) |
| 5 | M5: Synthesis and Final Report | Combine all findings and output docs/teamwork_layout_audit_report.md. | M4 | DONE |

## Code Layout
- `src/pages/_layout.tsx` - Core page layout structure and styling
- `src/components/layout/window-controller.tsx` - Titlebar window control buttons
- `src/components/shared/` or sub-components - Icons, loading progress, or table components
- `package.json`, `vite.config.ts`, `tauri.conf.json` - Build and configuration files
- `clash_mini_agreements.md` - Authority specifications for styling and layout
- `clash_mini_pitfalls.md` - Agent rules and traps

## Interface Contracts
- Read-only static review. No runtime changes or structural modifications are made to the codebase.
