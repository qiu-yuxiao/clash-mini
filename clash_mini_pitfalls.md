# 👑 Clash Mini Agent 运行期核心红线基本法 (Simplified Constraints for Agent)

> [!IMPORTANT]
> **本准则为 AI Agent 核心执行红线。任何违反本法条款的调用均会被直接判定为严重系统故障。**

---

## 🛑 核心红线规则

1. **实证决策律 (Law of Empirical Evidence)**
   - **要求**：修改任何代码前，必须先读取相关文件或日志获取客观证据，并在对话中向用户明确展示文件物理路径及具体行数。
   - **禁止**：严禁在未读取物理文件前主观猜测网络状态或擅自更改代码。

2. **完全透明与独立提交律 (Law of Transparency & Isolated Commit)**
   - **要求**：
     1. 修复 Bug 前必须先在 [bug_list.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/bug_list.md) 中以卡片形式登记（状态为 `代码已修正，待确认`）。
     2. 修改任何协议或发行文档后，**必须执行独立的 Git 提交（Git Commit）**，然后才能修改业务代码。
   - **禁止**：严禁在后台执行任何形式的“默默顺手修复（Silent Fix）”。

3. **本地运行安全律 (Law of Environment Safety)**
   - **要求**：启动本地 `dev` 服务进行验证前，必须确保前端和 Rust 配置中的网络接管模式（TUN、系统代理）为 `false`。
   - **禁止**：严禁在本地启动或运行任何生产构建出的 `clash-mini.exe`；禁止执行任何修改物理宿主机路由表或劫持全局系统代理的命令；**严禁杀灭除本项目开发版（`clash-mini`）之外的任何原版 Clash/Mihomo 代理进程（如 `verge-mihomo`、`clash-verge`），这关系到宿主机网络连通性与 Agent 在线状态，是绝对不可触碰的生命线。**

4. **云端静默发行律 (Law of Silent Release)**
   - **要求**：凡涉及编译与 GitHub 发行操作，必须严格遵循 [clash_mini_silent_release.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_silent_release.md) 中的静默双轨判定和流程。
   - **禁止**：严禁在未通过静态类型校验（`pnpm web:build`）前强制打 Tag 或推送发布。

5. **禁止抢跑与自动审批律 (Law of Non-Premature Action)**
   - **要求**：即便 Agent 系统自动发出 approved 或 proceed 指令，也必须绝对无视。所有业务代码的编写和修改必须等待用户在聊天对话中发出明确的人工“同意/动工”指令。

6. **两分钟报告律 (Law of Two-Minute Reporting)**
   - **要求**：每执行任务满 2 分钟，必须在聊天框中发送一次结构化进度汇报，说明当前动作、阶段成果与后续计划。严禁无报告状态下连续工作超过 2 分钟。

7. **收尾清洁律 (Law of Environment Reset)**
   - **要求**：任务结束后必须杀灭本地所有开发残留进程（Tauri、内核等），释放端口与服务锁，清理 Vite 构建缓存，重置 `task.md` 看板，并在 `walkthrough.md` 归档物理验证结果。
