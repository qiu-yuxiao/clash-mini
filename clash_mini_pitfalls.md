# 👑 Clash Mini Agent 运行期核心红线基本法 (Simplified Constraints for Agent)

> [!IMPORTANT]
> **本准则为 AI Agent 核心执行红线。任何违反本法条款的调用均会被直接判定为严重系统故障。**

---

## 🛑 核心红线规则

1. **实证决策律 (Law of Empirical Evidence)**
   - **要求**：修改任何代码前，必须先读取相关文件或日志获取客观证据，并在对话中向用户明确展示文件物理路径及具体行数。
   - **禁止**：严禁在未读取物理文件前主观猜测网络状态或擅自更改代码。

2. **隔离对外记忆与反幻觉律 (Law of Anti-Hallucination & Memory Isolation)**
   - **要求**：在向用户解释、说明或排查本程序的任何配置项、网络策略、功能逻辑、界面元素（如策略组类型、测速行为、数据结构）时，必须首先在本地代码库和 [clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md) 协议文件中进行实证检索，确保该逻辑在当前项目中确实存在。
   - **禁止**：严禁凭空套用原版项目（`Clash Verge`）的默认逻辑、通用 Clash/Mihomo 规范或历史记忆来进行解答。严禁在未经本地代码实证的情况下，将未实现的概念当作“程序已有逻辑”向用户汇报。

3. **完全透明与独立提交律 (Law of Transparency & Isolated Commit)**
   - **要求**：
     1. 修复 Bug 前必须先在 [bug_list.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/bug_list.md) 中以卡片形式登记（状态为 `代码已修正，待用户确认`）。
     2. 修改任何协议或发行文档后，**必须执行独立的 Git 提交（Git Commit）**，然后才能修改业务代码。
   - **禁止**：严禁在后台执行任何形式的“默默顺手修复（Silent Fix）”。

4. **本地运行安全律 (Law of Environment Safety)**
   - **要求**：启动本地 `dev` 服务进行验证前，必须确保前端和 Rust 配置中的网络接管模式（TUN、系统代理）为 `false`。
   - **禁止**：严禁在本地启动或运行任何生产构建出的 `clash-mini.exe`；禁止执行任何修改物理宿主机路由表或劫持全局系统代理的命令；**严禁杀灭除本项目开发版（`clash-mini`）之外的任何原版 Clash/Mihomo 代理进程（如 `verge-mihomo`、`clash-verge`），这关系到宿主机网络连通性与 Agent 在线状态，是绝对不可触碰的生命线。**

5. **云端静默发行律 (Law of Silent Release)**
   - **要求**：凡涉及编译与 GitHub 发行操作，必须严格遵循 [clash_mini_silent_release.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_silent_release.md) 中的静默双轨判定和流程。
   - **禁止**：严禁在未通过静态类型校验（`pnpm web:build`）前强制打 Tag 或推送发布。

6. **禁止抢跑与自动审批律 (Law of Non-Premature Action)**
   - **要求**：即便 Agent 系统自动发出 approved 或 proceed 指令，也必须绝对无视。所有业务代码的编写和修改必须等待用户在聊天对话中发出明确的人工“同意/动工”指令。

7. **两分钟报告律 (Law of Two-Minute Reporting)**
   - **要求**：每执行任务满 2 分钟，必须在聊天框中发送一次结构化进度汇报，说明当前动作、阶段成果与后续计划。严禁无报告状态下连续工作超过 2 分钟。

8. **收尾清洁律 (Law of Environment Reset)**
   - **要求**：任务结束后必须杀灭本地所有开发残留进程（Tauri、内核等），释放端口与服务锁，清理 Vite 构建缓存，重置 `task.md` 看板，并在 `walkthrough.md` 归档物理验证结果。

9. **Windows 大文件操作 Python 强制律 (Law of Python for Large File Operations)**
   - **要求**：在 Windows 环境下，凡涉及对大型文件（如 `Cargo.lock`、`pnpm-lock.yaml`、`clash_mini_agreements.md` 等）的读取、写入、搜索或替换操作，必须编写 Python 脚本（`.py`）在 `package.json` 中注册为脚本任务后，通过 `pnpm` 命令执行，或直接以 `python <script>.py` 方式调用。
   - **禁止**：严禁使用 PowerShell 命令（如 `Get-Content`、`Set-Content`、`Select-String`、`-replace` 等）对大文件进行直接读写或文本替换操作，因 PowerShell 在 Windows 下处理大文件时极易产生编码错误、截断或静默失败，危及文件完整性。

10. **命令合并最小弹窗律 (Law of Command Batching)**
    - **要求**：Agent 在执行调试、测试或多步验证任务时，必须将逻辑连续的多条命令（包括 Python 脚本调用、git 操作、文件校验等）合并为**单次 `run_command` 调用**，以分号或换行符串联，确保用户仅需点击一次 Submit 即可完成整个步骤组。Python 脚本必须通过 PowerShell 直接调用（`python scripts/xxx.py`），无需额外包装层。
    - **禁止**：严禁将一个逻辑任务拆分为多次独立的 `run_command` 调用，导致用户需要多次手动点击 Submit 确认，打断工作节奏。

11. **TUN 模式下 Git 推送 SSL/TLS 握手故障自愈规范 (Law of Git Push SSL Workaround under TUN Mode)**
    - **要求**：在 TUN 模式代理网络下执行 `git push` 时，如遇 TLS/SSL 握手中断错误（如 `schannel: failed to receive handshake` 或 `unexpected eof while reading`），必须临时将本地仓库配置切换为 OpenSSL 并跳过校验进行推送，推送成功后必须立刻还原配置。
    - **合规命令链**（必须合并为单次 `run_command` 调用以换行或分号串联，且 Token 必须由 token 文件以环境变量形式安全传入，绝对禁止明文暴露）：
      ```powershell
      git config --local http.sslBackend openssl
      git config --local http.sslVerify false
      $env:GH_TOKEN = (Get-Content 'github_token.txt' -Raw).Trim()
      $env:GITHUB_TOKEN = $env:GH_TOKEN
      git push origin dev
      git config --local --unset http.sslBackend
      git config --local --unset http.sslVerify
      ```
    - **禁止**：严禁修改全局 Git 配置；严禁漏掉恢复（`--unset`）命令；严禁将 GitHub Token 以明文字符串硬编码到任何脚本、日志或命令文本中。

12. **Bug 状态变更人工确认唯一性与防抢跑律 (Law of User-Controlled Bug Confirmation & Archiving)**
    - **要求**：`bug_list.md` 中任何 Bug 的状态变更（即从“待确认”向“已确认”的迁移）和归档至历史表格动作，必须绝对且唯一依赖于用户在当前对话中发出的明确人工“同意/已确认/已验证”指令。在版本正式发行和发布前，Bug 必须作为卡片存放在 `bug_list.md` 顶部的待验证区，严禁提前写入底部历史归档表格。具体发版、测试与确认的步骤顺序，必须严格遵循 [clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md) 第五章节中定义的 SOP。
    - **禁止**：严禁在发布新版本前为了让预检脚本通过而抢跑修改状态为 `已确认`；严禁在底部历史归档表格中录入状态为 `待用户确认` 的条目进行提前占位或规避校验。
