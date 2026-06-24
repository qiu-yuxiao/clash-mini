# Project: Clash Mini Active Connection Node Layout Audit

## Architecture
- React frontend layout.
- Styling components (CSS/SCSS).
- Page structure containing the active connection node and latency display row (introduced or modified post 1.6.5).
- Warning-commented design agreements for layout clipping/hiding.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | Explore components | Locate active node and latency display row components, associated styles, and design agreements | None | DONE |
| 2 | Root cause analysis | Identify the layout collapse / icon expansion issue introduced since v1.6.5 | M1 | DONE |
| 3 | Create report | Generate docs/active_node_layout_audit.md report with findings and suggested diff | M2 | DONE |
| 4 | Verification | Verify formatting, clickable links, design compliance, and git status | M3 | DONE |

## Code Layout
- Frontend components: `src/pages/_layout/components/active-node-card.tsx`
- Layout page: `src/pages/_layout.tsx`
- Style helpers: `src/pages/_layout/utils/style-helpers.tsx`
- Caching service: `src/services/delay.ts`
