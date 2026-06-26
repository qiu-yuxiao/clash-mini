# BRIEFING — 2026-06-25T17:55:30Z

## Mission
Investigate the Settings Drawer conditional rendering leaks and produce a detailed report.

## 🔒 My Identity
- Archetype: explorer
- Roles: Teamwork explorer, read-only investigator
- Working directory: c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_m2
- Original parent: 9a6831ba-c126-471c-bdbb-4948fc427a3d
- Milestone: Settings Drawer Conditional Rendering Memory Leak Investigation

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- CODE_ONLY network mode: no external web/service access, no curl/wget/lynx to external URLs.

## Current Parent
- Conversation ID: 9a6831ba-c126-471c-bdbb-4948fc427a3d
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `src/pages/_layout.tsx` (Settings Drawer rendering logic)
  - `src/pages/_layout/components/` (Settings Drawer subcomponents: `ProfileImportCard`, `TakeoverModeCard`, `RoutingPreferenceCard`, `BasicSettingsCard`, `ThemeSettingsCard`, `ConnectionsPanel`, `HelpMenuButton`)
  - `src/hooks/use-connection-data.ts` & `src/hooks/use-mihomo-ws-subscription.ts` (WebSocket lifecycle management)
  - Git history comparing `v1.8.2` and `v1.8.9`
- **Key findings**:
  - In `v1.8.2`, the Settings Drawer was always mounted and toggled via CSS translation, resulting in stable DOM/Emotion stylesheet nodes and a clean memory footprint.
  - In `v1.8.9`, it was conditionally mounted, causing repeated creation/destruction of complex Material-UI subcomponents, dynamic style sheet injections, and DOM-related leaks (e.g. portals/tooltips/event listeners) that WebView2 cannot garbage collect even under memory pressure (low target level).
  - The WebSocket subscription and system data queries are already gated by `drawerOpen` and `isPanelVisible` visibility controls, meaning always-mounting the drawer does not cause background polling.
- **Unexplored areas**: None

## Key Decisions Made
- Recommend rolling back from conditional rendering to the CSS translation strategy of v1.8.2 while keeping the visibility-based polling gates.

## Artifact Index
- c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_m2\ORIGINAL_REQUEST.md — User request and timestamp
- c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_m2\BRIEFING.md — Persistent briefing index
- c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_m2\progress.md — Progress log
- c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\.agents\teamwork_preview_explorer_m2\handoff.md — Investigation and rollback patch report
