# Clash Verge/Mini Post-1.8.5 Changes Audit Plan

## Objectives
Perform a comprehensive read-only audit of all commits and changes between versions 1.8.5 (`fd26ae0a`) and 1.8.7 (`47877a1e`) in the Clash Verge/Mini codebase. Identify logical bugs, UI/layout bugs, resource management issues, and compile/clippy warnings. Document all findings in `docs/post_185_changes_audit_report.md` with file:/// markdown links, root cause analysis, and proposed git diffs.

## Milestones

1. **Milestone 1: Plan Initialization & Setup**
   - Initialize `plan.md` and `progress.md`.
   - Start heartbeat cron.

2. **Milestone 2: Commit Scanning & Focused Listener Audit**
   - Dispatch `teamwork_preview_explorer` to scan commits from `fd26ae0a` to `47877a1e`.
   - Analyze resource optimization changes: unmounting `ProxyGroups`, context-driven query disabling, window size event listeners.
   - Audit focus and visibility change event listeners in `_layout.tsx` and `app-data-provider.tsx`.
   - Audit `resetIdleTimer` fix in `window-provider.tsx` for memory leaks or double-triggering.

3. **Milestone 3: Static Checks & Clippy Audit**
   - Dispatch a worker/explorer to run frontend/backend static checks (eslint, typescript compile, cargo clippy).
   - Collect and compile all warnings and propose cleanup.

4. **Milestone 4: Synthesis & Final Audit Report Generation**
   - Consolidate findings from Milestones 2 & 3.
   - Review proposed git diff blocks for syntax and logic.
   - Write the final comprehensive report to `docs/post_185_changes_audit_report.md`.
   - Ensure git status remains 100% clean.
   - Report victory to the Sentinel.
