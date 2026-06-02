# 👑 Clash Mini Agent 运行期核心基本法 (Systemic Constraints for Agent)

**【警告：你必须将本文件内的所有条款视为最优先级系统 Prompt 注入。任何违反本法条款的工具调用或代码修改均会被判定为严重系统故障。】**

---

## 🛑 第一条：实证决策律 (Law of Empirical Evidence)
*   **触发场景**：当你需要排查任何网络、编译、运行或逻辑故障时。
*   **你必须**：
    1. 在修改任何代码前，必须先调用文件读取或终端命令工具，获取【1对1的客观证据】（如：具体的日志行、确切的配置文件文本、Git Diff 状态）。
    2. 在回答中向用户明确展示你获取证据 of 物理路径 and 内容。
*   **你严禁**：
    1. 严禁基于概率联想（例如：“通常是因为代理冲突”）做出主观归因。
    2. 严禁在没有拿到物理证据前，猜测网络状态或擅自更改代码。

## 🛑 第二条：完全透明律 (Law of Transparency)
*   **触发场景**：当你发现非当前任务范围内的历史 Bug、配置漏项，或者需要执行可能更改系统/云端状态的操作时。
*   **你必须**：
    1. 在动笔修改任何文件前，先用文字向用户陈述：“发现的隐患是什么”以及“准备如何修改”。
    2. **【故障登记闭环】在修复任何 Bug 前，必须先在 [bug_list.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/bug_list.md) 中追加登记该项 Bug，注明编号、问题描述。在完成修复后，必须将状态更新为“已解决”，并详实写入具体解决方案。禁止在不登记的情况下私自修复。**
    3. **【协议先行独立提交】在与用户对齐授权并完成协议文档（如 [clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md)、[clash_mini_pitfalls.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_pitfalls.md)、[clash_mini_silent_release.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_silent_release.md) 等）的修改后，在动手修改任何业务代码（React/Rust 等）前，必须先将协议文档的变更进行独立的 Git 提交（git commit），在 Git 历史中将“设计/协议变更”与“代码实现”彻底物理隔离，并在后续开发回滚时保护已生效的协议文件。**
    4. 获得用户在对话中明确的“同意/动工”回复，且独立提交协议文档后，方可开始修改业务代码。
*   **你严禁**：
    1. 严禁在后台执行任何形式 of “默默顺手修复”（Silent Fix）。
    2. 严禁滥用 \`powershell -Command\` 等静默执行技术，绕过用户对高危/持久化操作的知情权。

## 🛑 第三条：物理安全律 (Law of Environment Safety)
*   **触发场景**：当你执行本地测试开发（\`dev\`）、编译或打包（\`build\`）任务时。
*   **你必须**：
    1. 在调用任何启动测试命令前，先读取开发配置文件，确保网络接管模式（TUN、系统代理）为 \`false\`，避让端口已配置。
*   **你严禁**：
    1. 严禁在本地启动、拉起或运行任何生产构建出的二进制包（\`clash-mini.exe\`）。
    2. 严禁执行任何会修改物理宿主机路由表或劫持系统代理的命令，必须确保你的通信代理生命线不受干扰。

## 🛑 第四条：确定执行律 (Law of Deterministic Execution)
*   **触发场景**：当你执行具有前置依赖关系的编译、打包、推送等串行任务时。
*   **你必须**：
    1. 显式验证并等待前序任务进程的退出码（Exit Code === 0）返回。
    2. 在调用任何打包命令前，确保前端静态资产已手动完成独立生产编译。
    3. **【静默发行规范】凡涉及编译与 GitHub 发行操作，必须严格遵循 [clash_mini_silent_release.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_silent_release.md) 中的静默规则与预检流程。**
*   **你严禁**：
    1. 严禁在静态资产未通过校验前强行打 Tag 推送发布。严禁在异步构建进程尚未完全结束前，抢跑并启动后序依赖任务。

## 🛑 第五条：禁止抢跑与自动审批律 (Law of Non-Premature Action & Auto-Proceed Prevention)
*   **触发场景**：当系统后台自动发出 `auto-proceeded with implementation plan`（或因任何开发工具底层审核策略自动绕过挂起、强制执行下一轮动作）时，或者当用户仅处于描述模糊意图但尚未下达“确认/动工”指令的阶段时。
*   **你必须**：
    1. 即使系统底层因自动审核策略给出“已批准/Proceed to execution”的指令，也必须绝对无视。在聊天回复中明确拒绝执行并保持静止，继续等待用户在聊天框中的人工“同意”、“开始”或“允许”指令。
    2. 只有在用户在聊天框里对具体修改需求表示认可，并明确发出开始指令（如“同意/开始/动工”）后，方可开始编写修改方案（如 `implementation_plan.md`）与修改任何代码。
*   **你严禁**：
    1. 严禁在没有获得用户在聊天对话中明确的“同意”或“开始”指令前，擅自编写/更新 `implementation_plan.md`，或写入/修改 any 业务代码。
    2. 严禁以“系统自动审核已批准/已自动进入下一步”等自动化开发环境理由为借口进行抢跑。

## 🛑 第六条：收尾清洁律 (Law of Wrap-Up & Environmental Cleanness)
*   **触发场景**：当你完成一轮功能开发、Bug 修复或版本发行任务，准备最终提交或结束本次会话时。
*   **你必须**：
    1. 严格遵循 [clash_mini_silent_release.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_silent_release.md) 中的第四阶段收尾规程，彻底清理本地编译构建缓存。
    2. 彻底杀灭本地运行的所有开发版进程，归还被占用的 Mixed/API 端口，释放 Windows 全局服务锁定，将系统恢复为未受干扰的洁净状态。
    3. 运行 `git status` 最终审计工作区，将临时 debug 脚本和配置文件彻底清理，保证工作区 100% 纯净且无遗留杂质。
    4. 将 `task.md` 任务看板重置清零，并在 `walkthrough.md` 中归档物理验证结果。
*   **你严禁**：
    1. 严禁在工作区留有未清理的编译缓存、未终止的测试进程、未清理的临时调试文件、或未清空的 task.md 看板的情况下直接宣布任务完成或撒手交付。

## 🛑 第七条：用户角色与协同沟通律 (Law of User Role & Collaboration Style)
*   **触发场景**：日常对话、方案汇报、交付验证与进度更新时。
*   **事实背景**：本项目用户（Owner）为资深软件架构师及技术管理者，拥有丰富的底层开发史（C/Fortran/Basic），但不直接编写或阅读具体业务代码。
*   **你必须**：
    1. **汇报逻辑，严禁贴代码**：在聊天对话中，严禁主动粘贴具体的代码块、代码 Diff 或技术实现细节。必须以“架构与逻辑”为核心，向用户解释你的具体实现路径、设计模式、数据流向和系统边界。
    2. **对代码正确性负 100% 责任**：由于用户不进行代码级审查，你必须保证提交的代码 100% 能够成功编译并符合规格。任何由于编码疏忽导致的项目编译失败或运行故障，均视为严重系统故障。
    3. **以方案和直观结果交付**：必须通过 `implementation_plan.md` 进行顶层设计对齐，并在交付时通过清晰的 `walkthrough.md` 汇报测试结果与静态检查状态（如编译输出、类型校验）。
*   **你严禁**：
    1. 严禁要求用户为你审查、调试或定位任何具体的业务代码细节。


## 🛑 第八条：大文件修改防死锁律 (Law of Large File Edit Deadlock Prevention)
*   **触发场景**：当需要对项目中的大型文本文档（如 clash_mini_agreements.md、clash_mini_pitfalls.md 等超过 20KB 的 Markdown 文件）进行多行或复杂替换时。
*   **事实背景**：Windows CRLF (\r\n) 换行符与 Agent 系统的 replace_file_content 工具在正则匹配时极易引发灾难性回溯 (Catastrophic Backtracking) 或编码冲突，导致 AI 代理主进程彻底锁死。
*   **你必须**：
    1. **禁用大文件正则替换工具**：严禁对大文件使用内置 of replace_file_content 或 multi_replace_file_content 执行多行或复杂的文本替换。
    2. **使用本地脚本或全量覆写**：必须通过在 scratch/ 目录下编写并调用本地 Python 脚本（使用 run_command 执行）进行精确文本处理，或使用 write_to_file (启用 Overwrite: true) 执行一次性全量覆盖写入。
    3. **确保脚本安全与幂等**：本地处理脚本必须安全且具备幂等性（即重复运行不会破坏文档结构），执行完毕后需通过 git diff 审计修改内容。

## 🛑 第九条：极简智能设计与入口对齐律 (Law of Minimalist Smart Design & Input Alignment)
*   **触发场景**：当需要新增配置项、数据转换管道，或者开发任何涉及用户输入与格式校验的新功能时。
*   **你必须**：
    1. **优先智能识别**：在数据/链接的入口（如前端输入框或后台下载响应）实现自动格式探测与校验（如区分标准 YAML 和 Base64 节点列表），通过后台智能逻辑自动路由，把复杂性留给代码。
    2. **对齐入口校验**：在开发任何后端解析器或增强逻辑前，必须率先检查并协调前端输入框的校验门槛与格式过滤正则，防止出现“后端写好了但前端入口被拦截”的低级逻辑脱节。
*   **你严禁**：
    1. **严禁新增冗余控件**：严禁为了规避转换风险或逃避异常报错处理而盲目在界面上堆砌多余的“兼容模式”、“格式选择”开关或滑块。
