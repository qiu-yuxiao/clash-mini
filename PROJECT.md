# Project: Clash Mini Code Audit

## Architecture
Clash Mini is a hybrid application built using:
- **Frontend**: React, TypeScript, Vite, Tailwind CSS, Monaco Editor. Key UI features include custom theme systems (monochrome, cyberpunk, Trump-3D/Retro-3D) with physical simulation (vibrancy, sliders, custom switches).
- **Backend**: Rust, Tauri, Cargo. Features active node monitoring, proxy selection, service management, and API bridging with Mihomo (clash) core.
- **IPC/Bridge**: Tauri commands and event systems.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Frontend Code Audit | Audit of React/TypeScript frontend (`src/`) for performance, safety, clean code, design patterns, and UI agreements | none | DONE (Conv: 45852f56-45e1-4f9b-a80d-5b7b12e14f30) |
| 2 | Backend Code Audit | Audit of Rust/Tauri backend (`src-tauri/`) for concurrency, safety, memory, clean code, and API agreements | none | DONE (Conv: 02da9e40-7abf-4a1c-b518-cac7dc1e47ab) |
| 3 | Agreement Compliance Audit | Map code implementations to the 26 requirements in `clash_mini_agreements.md` and document compliance gaps | M1, M2 | DONE (Conv: b53181f5-f4b0-4412-aca6-334239699c7a) |
| 4 | Final Report Synthesis | Synthesize findings into the final report at `docs/clash_mini_audit_report.md` | M3 | DONE |

## Code Layout
- Frontend Source: `src/`
  - Components: `src/components/`
  - Hooks: `src/hooks/`
  - Pages: `src/pages/`
  - Styles: `src/assets/styles/`
- Backend Source: `src-tauri/`
  - Core Logic: `src-tauri/src/`
  - Dependencies: `Cargo.toml`, `Cargo.lock`
- Core Documents:
  - Agreements: `clash_mini_agreements.md`
  - Bug list: `bug_list.md`
  - Pitfalls: `clash_mini_pitfalls.md`
