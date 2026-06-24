# Explorer M1-1 Task Plan

## Objective
Investigate Target Layout Bug:
- Root cause of the top active connection outbound node card layout.
- Size inflation of its delay indicator icon (diamond cursor).
- Speed test cursor occupying the screen.

## Methodology
- Search for "active-node-card.tsx" and related styles/SVGs.
- Inspect how the delay indicator icon and speed test cursor are implemented.
- Check styling/classes (e.g. Tailwind CSS, width/height) causing size inflation.
