---
description: Clash Mini Developer & Auditor Agent - Guides AI agents in developing, reviewing, and releasing Clash Mini with strict compliance
disable-model-invocation: true
---

# Clash Mini Developer & Auditor Agent

This agent ensures that any AI developer, assistant, or reviewer working on the **Clash Mini** repository strictly adheres to the project's behavioral laws, safety redlines, and design specifications.

---

## 🚨 Core Laws of Execution

Every Agent or automated workflow acting on this repository must comply with the following 5 laws. Violating any of these rules is considered a critical workflow failure.

### 1. Law of Empirical Evidence
* **Requirement**: Before modifying any code, configuration, or dependency, you must read physical files or compile logs to obtain objective evidence.
* **Prohibition**: Do not make assumptions or blindly alter code without reading the relevant source files.

### 2. Law of Transparency & Isolated Commit
* **Requirement**:
  1. Before fixing a bug, you must register it in [`bug_list.md`](../../bug_list.md) in the form of a card (setting its status to `Code corrected, pending user confirmation`).
  2. Any modification to protocols or release documentation **must be committed separately** before writing or modifying any business logic code.
* **Prohibition**: "Silent Fixes" (modifying code without documenting/registering changes first) are strictly prohibited.

### 3. Law of Environment Safety & Isolation
* **Requirement**:
  1. Before starting the local development service, ensure that the network takeover options (TUN Mode, System Proxy) in both the frontend and Rust configurations are set to `false`.
  2. Clash Mini's Sidecar kernel binaries and processes must be renamed to **`mini-mihomo`** / **`mini-mihomo-alpha`** (do not use the original `verge-mihomo`) to achieve an independent namespace at the process level.
  3. The main application exit event `clean_async` must asynchronously kill all remaining `mini-mihomo` processes to release ports and system locks.
* **Prohibition**: Running production-built binary packages locally is strictly prohibited. Modifying the host OS routing table or hijacking the global system proxy during execution is strictly prohibited.

### 4. Law of Silent Release
* **Requirement**:
  1. Any compilation and GitHub release operations must strictly follow the silent double-track decision and 8-step SOP process in [`clash_mini_silent_release.md`](../../clash_mini_silent_release.md).
  2. **Dependency Whitebox Review**: For any bugs caused by third-party plugin or dependency updates, you must verify `Cargo.lock` or `pnpm-lock.yaml` before packaging to confirm that the latest fixed version number and Git Commit Hash are locked.
  3. **Local Log Verification**: Before publishing, you must execute and print the content extracted by `node scripts/extract_update_logs.mjs <version>` in the console to ensure that the release note details map seamlessly to the bugs resolved in this version.
* **Prohibition**: Forcing tags or pushing releases without passing static type checks (`pnpm web:build`) is strictly prohibited.

### 5. Law of Design Consistency
* **Requirement**: All UI interaction controls, skins, and parameters must strictly align with the [`clash_mini_agreements.md`](../../clash_mini_agreements.md) agreement:
  * **6 Visual Skins**: `Retro 3D` (default), `Original`, `Modern`, `Frosted Glass`, `Cyberpunk`, `Monochrome`.
  * **Redefined Slider Controls**: Under different skins, the two English sliders control different underlying variables (such as Depth/Vibrancy, Radius/Accent, etc.), and the data is stored in independent fields in LocalStorage.
  * **Font Alignment (Chinese & English)**: Different skins align to different fonts (e.g., Trebuchet MS / SimHei, Consolas / NSimSun, etc.).
  * **Mixed Port Default**: The listening address defaults to `127.0.0.1`, the mixed proxy port is locked to `10801`, and the Controller API port is `9098` to isolate from the upstream default ports.

---

## 📋 AI Agent Task Execution Guidelines

When you accept a task to modify code as an Agent, please follow these steps:

1. **Step 1: Locate & Audit**
   - Use grep to retrieve relevant files and read tools to inspect code accurately.
   - Compare with [`clash_mini_agreements.md`](../../clash_mini_agreements.md) to check if the module has established design specifications.

2. **Step 2: Update Agreement & bug_list (Isolated Commit)**
   - If it's a bug fix, update [`bug_list.md`](../../bug_list.md) first to register a card.
   - If it involves design changes, update [`clash_mini_agreements.md`](../../clash_mini_agreements.md) first.
   - Execute `git add` and commit the document changes: `git commit -m "docs: register changes" --no-verify`.

3. **Step 3: Modify Business Code**
   - Write or update TSX, Vite, or Rust code.
   - Ensure you follow the "English preservation principle" and font/dimension specifications.

4. **Step 4: Static Type & Compile Validation**
   - Run the local build command `pnpm web:build` to ensure 100% compilation passes with zero warnings.

5. **Step 5: Wrap Up & Reset**
   - Record modifications and verification results in `.brain/walkthrough.md`.
   - Check off tasks in `.brain/task.md` and reset the kanban.
