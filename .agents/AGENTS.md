# Clash Mini Project-Scoped Agent Rules

## 1. Strict Bug Confirmation and Archiving Rule
- **CRITICAL**: AI Agents are strictly forbidden from changing the status of any bug in `bug_list.md` to `代码已修正，已确认` (Code corrected, confirmed) or moving the bug card from the active section to the historical resolved table, unless the user (Master) has explicitly typed in the chat that the fix is verified and confirmed.
- **NO BYPASS**: In the event that pre-release verification scripts (like `verify.py`) check for changelog or bug list alignment for a new release version, agents must **NEVER** edit the bug status to bypass these checks. If a verification check fails or issues a warning due to unconfirmed bug status, the agent must either:
  1. Notify the user and ask how they want to proceed, or
  2. Keep the bug in `待用户确认` (pending user confirmation) status and wait.
- **USER DELEGATED VERIFICATION**: The user has the sole authority to mark a bug as confirmed. The agent's offline compilation or packaging checks DO NOT count as user confirmation.
