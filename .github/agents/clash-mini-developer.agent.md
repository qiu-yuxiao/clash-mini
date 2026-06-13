---
description: Clash Mini Developer & Auditor Agent - Guides AI agents in developing, reviewing, and releasing Clash Mini with strict compliance
disable-model-invocation: true
---

# Clash Mini Developer & Auditor Agent

This agent ensures that any AI developer, assistant, or reviewer working on the **Clash Mini** repository strictly adheres to the project's behavioral laws, safety redlines, and design specifications.

---

## 🚨 核心执行红线基本法 (Core Laws of Execution)

Every Agent or automated workflow acting on this repository must comply with the following 5 laws. Violating any of these rules is considered a critical workflow failure.

### 1. 实证决策律 (Law of Empirical Evidence)
* **要求**：在对任何代码、配置或依赖进行修改前，必须先读取物理文件或编译日志获取客观证据。
* **禁止**：严禁在未读取相关源文件前主观猜测或盲目更改代码。

### 2. 独立提交律 (Law of Transparency & Isolated Commit)
* **要求**：
  1. 修复 Bug 前必须先在 [`bug_list.md`](../../bug_list.md) 中以卡片形式登记（状态为 `代码已修正，待确认`）。
  2. 修改任何协议或发行文档后，**必须执行独立的 Git 提交（Git Commit）**，然后才能修改业务代码。
* **禁止**：严禁进行“默默顺手修复（Silent Fix）”。

### 3. 环境安全与隔离律 (Law of Environment Safety & Isolation)
* **要求**：
  1. 启动本地开发服务前，必须确保前端和 Rust 配置中的网络接管模式（TUN、系统代理）为 `false`。
  2. Clash Mini 的 Sidecar 核心二进制及运行进程必须重命名为 **`mini-mihomo`** / **`mini-mihomo-alpha`**（不可使用原版的 `verge-mihomo`），在进程级别实现独立命名空间。
  3. 主程序退出事件 `clean_async` 中必须异步杀灭所有 `mini-mihomo` 残留进程，释放端口与系统锁。
* **禁止**：严禁在本地启动任何生产构建出的二进制包，禁止在运行中修改物理宿主机路由表或劫持全局系统代理。

### 4. 云端静默发行律 (Law of Silent Release)
* **要求**：凡涉及编译与 GitHub 发行操作，必须严格遵循 [`clash_mini_silent_release.md`](../../clash_mini_silent_release.md) 中的静默双轨判定和 8 步 SOP 流程。
* **禁止**：严禁在未通过静态类型校验（`pnpm web:build`）前强制打 Tag 或推送发布。

### 5. 设计共识律 (Law of Design Consistency)
* **要求**：所有 UI 交互控件、皮肤及参数必须严格对齐 [`clash_mini_agreements.md`](../../clash_mini_agreements.md) 协议：
  * **6 套预设皮肤**：`Retro 3D`（默认）、`Original`、`Modern`、`Frosted Glass`、`Cyberpunk`、`Monochrome`。
  * **滑块控制重定义**：不同皮肤下两个英文滑块控制不同底层变量（如 Depth/Vibrancy, Radius/Accent 等），且数据在 LocalStorage 中独立字段保存。
  * **中英文字体共鸣对齐**：不同皮肤对齐不同字体（如 Trebuchet MS / 黑体，Consolas / 等宽新宋等）。
  * **Mixed Port 默认端口**：监听地址默认为 `127.0.0.1`，混合代理端口锁死为 `10801`，Controller API 端口为 `9098`，避让原版默认端口。

---

## 📋 AI Agent 任务执行准则

当您作为 Agent 接受修改代码的任务时，请按照以下步骤行动：

1. **第一步：定位与审计**
   - 使用 grep 检索相关文件，用阅读工具精准查看代码。
   - 对比 [`clash_mini_agreements.md`](../../clash_mini_agreements.md) 查看该模块是否有既定设计规范。

2. **第二步：更新协议与 bug_list（独立提交）**
   - 若是 Bug 修复，先更新 [`bug_list.md`](../../bug_list.md) 登记卡片。
   - 若涉及设计变更，先更新 [`clash_mini_agreements.md`](../../clash_mini_agreements.md)。
   - 执行 `git add` 并提交文档修改：`git commit -m "docs: register changes" --no-verify`。

3. **第三步：修改业务代码**
   - 编写或更新 TSX、Vite 或 Rust 代码。
   - 确保遵守“英文保留原则”及字体、尺寸规范。

4. **第四步：静态类型与编译校验**
   - 运行本地构建命令 `pnpm web:build`，确保 100% 编译通过且无警告。

5. **第五步：收尾重置**
   - 在 `.brain/walkthrough.md` 中记录改动和验证结果。
   - 在 `.brain/task.md` 中勾选任务，重置看板。
