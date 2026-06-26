# Task: Settings Drawer Conditional Rendering Memory Leak Investigation

## Objective
Investigate memory usage regression under lightweight mode in Clash Mini v1.8.9 compared to v1.8.2, focusing on the Settings Drawer layout rendering.

## Context
In v1.8.9, Settings Drawer in `src/pages/_layout.tsx` is conditionally rendered using `{drawerOpen && !isMiniStatus && ...}`. In v1.8.2, a CSS translation strategy (`transform: translate(100%, -100%)`) was used. The destruction and reconstruction of complex Material-UI subcomponents (`ProfileImportCard`, `BasicSettingsCard`, `ThemeSettingsCard`, `ConnectionsPanel`, etc.) might lead to React memory leaks.

## Requirements
1. Analyze `src/pages/_layout.tsx` and all subcomponents imported for the Settings Drawer.
2. Inspect the difference between the conditional mount approach and the CSS translation approach.
3. Identify how mounting/unmounting affects memory, focusing on event listeners, timers, styles, or DOM references that might not be cleaned up in the subcomponents.
4. Provide detailed explanation, exact files/functions/lines, and propose a fix/rollback patch as a Git diff block.
5. Write your findings to `.agents/teamwork_preview_explorer_m2/handoff.md`.
