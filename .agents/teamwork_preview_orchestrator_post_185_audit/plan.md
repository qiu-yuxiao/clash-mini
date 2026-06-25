# Audit Plan — Post 1.8.5 Changes

## Objective
Audit commits and changes from `fd26ae0a` to `47877a1e` in Clash Verge/Mini, identify logical bugs, UI/layout bugs, resource management issues, and backend Clippy warnings, and compile a report to `docs/post_185_changes_audit_report.md` without modifying any program code (keeping git status clean).

## Steps

### Step 1: Retrieve Git History and Diff Range
- **Action**: Run `git log` and `git diff` for commit range `fd26ae0a..47877a1e`.
- **Subagent**: `teamwork_preview_explorer_post_185_git_analysis`
- **Output**: Git logs and list of modified files with diff overview.

### Step 2: Audit Frontend Changes
- **Action**: Static analysis of frontend modifications, targeting:
  - `focus` and `visibilitychange` listeners in `_layout.tsx` and `app-data-provider.tsx`.
  - `resetIdleTimer` in `window-provider.tsx`.
  - Resource optimizations (unmounting `ProxyGroups`, query disabling, window size listeners).
  - Memory leaks (uncleaned event listeners, uncleared timers) or query double-triggering.
- **Subagent**: `teamwork_preview_explorer_post_185_frontend`
- **Output**: Finding notes and diff solutions for frontend.

### Step 3: Audit Backend Changes and Warnings
- **Action**: Static analysis of backend modifications and clippy recommendations:
  - Clippy check on backend Rust code under `src-tauri`.
  - Lock safety, concurrency, warning cleanup.
- **Subagent**: `teamwork_preview_explorer_post_185_backend`
- **Output**: Finding notes and diff solutions for backend.

### Step 4: Synthesize Findings and Draft Report
- **Action**: Combine findings from Steps 2 & 3. Draft `docs/post_185_changes_audit_report.md`.
- **Subagent**: `teamwork_preview_worker_post_185_report`
- **Output**: Draft report in the required directory.

### Step 5: Review and Verify Report
- **Action**: Review markdown file links, ensure correct formatting, verify git status is clean, and verify no source files are modified.
- **Subagent**: `teamwork_preview_reviewer_post_185_report`
- **Output**: Review report and confirmation.

### Step 6: Forensic Audit
- **Action**: Perform final integrity check using Forensic Auditor.
- **Subagent**: `teamwork_preview_auditor_post_185_report`
- **Output**: Audit verdict.

### Step 7: Completion
- **Action**: Report results back to parent agent.
