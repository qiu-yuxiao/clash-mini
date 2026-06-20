# GitHub Copilot Custom Instructions for Clash Mini

This file defines custom instructions and behavioral guardrails for **GitHub Copilot, Copilot Workspace, and Copilot Coding Agents** working on the **Clash Mini** repository. 

All Copilot Agent tasks (visible under the **Agents** tab) must strictly adhere to the safety redlines, development laws, and design specifications defined below.

---

## 🚨 Core Agent Laws

Any AI Agent performing tasks in this repository must comply with the following 5 laws. Failing to do so will result in workflow failure.

### 1. Law of Empirical Evidence
* **Rule**: AI agents must read physical files or compile logs to gather concrete evidence before proposing or modifying any code, configuration, or dependency.
* **Prohibition**: Do not make assumptions or blindly alter code without reading the relevant source files.

### 2. Law of Transparency & Isolated Commit
* **Rule**: 
  1. Before fixing a bug, register it in [`bug_list.md`](../bug_list.md) under the "Active & Pending Bugs" section (setting its status to `Code corrected, pending user confirmation`).
  2. Any modification to documentation (like `clash_mini_agreements.md` or `bug_list.md`) **must be committed separately** before writing or modifying any business logic code.
* **Prohibition**: Silent fixes (modifying code without documenting/registering changes first) are strictly prohibited.

### 3. Law of Environment Safety & Isolation
* **Rule**: 
  1. In local development or CI environments, the network takeover options (TUN Mode, System Proxy) must be set to `false`.
  2. Clash Mini's Sidecar kernel binaries and processes must be named **`mini-mihomo`** / **`mini-mihomo-alpha`** (not the upstream `verge-mihomo`) to establish an isolated namespace.
  3. The main application exit hook `clean_async` must asynchronously kill all remaining `mini-mihomo` processes to release ports and file locks.
* **Prohibition**: Do not run production build binaries in local development, and do not execute commands that modify the host OS routing tables or hijack the global system proxy.

### 4. Law of Silent Release
* **Rule**: Follow the 8-step silent release SOP in [`clash_mini_silent_release.md`](../clash_mini_silent_release.md).
* **Prohibition**: Do not push release tags or publish builds without passing static type checks (`pnpm web:build`).

### 5. Law of Design Consistency
* **Rule**: All UI changes must conform to the design agreements in [`clash_mini_agreements.md`](../../clash_mini_agreements.md):
  * **6 Visual Skins**: `Retro 3D` (default), `Original`, `Modern`, `Frosted Glass`, `Cyberpunk`, `Monochrome`.
  * **Double Sliders**: Different skins map the two sliders to different parameters (e.g., Depth/Vibrancy, Radius/Accent) stored in separate LocalStorage fields.
  * **Font Resonance**: Match fonts per skin (e.g., Trebuchet MS / SimHei for Retro 3D).
  * **Network Ports**: Default Mixed Port is `10801`, and Controller API is `9098` to isolate from upstream Clash Verge.

---

## 📋 AI Agent Workflow Checklist

When executing a task via Copilot Workspace:

1. **Audit First**: Read the relevant files and reference [`clash_mini_agreements.md`](../clash_mini_agreements.md) for specs.
2. **Isolated Doc Commit**: If adding a feature or fixing a bug, update `clash_mini_agreements.md` or `bug_list.md` first, and perform a git commit (e.g., `git commit -m "docs: register changes" --no-verify`) before editing code.
3. **Write Code**: Ensure you preserve "English retention" for UI labels and match theme-specific font/styling rules.
4. **Compile & Typecheck**: Run `pnpm web:build` locally or in the runner to ensure zero errors and zero warnings.
5. **Reset & Walkthrough**: Write your validation results to `.brain/walkthrough.md` and check off items in `.brain/task.md`.
