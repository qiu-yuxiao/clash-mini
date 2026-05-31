# 👑 Clash Mini 一键静默发行准则 (One-Click Silent Release Guidelines)

**【警告：本准则具有法律级效力，仅次于宪法（.cursorrules）。凡执行发行动作的 Agent 必须 100% 遵守本规则。违反本规则的任何原生弹窗命令均被判定为严重故障。】**

---

## 🔍 第一阶段：环境就绪预检与本地验证 (Environmental Pre-Check & Local Verification)

当接收到用户发出的发行/动工指令时，**必须首先执行以下环境检测、缓存清理与验证流程**。严禁省略此流程直接修改代码或编译发布。

1. **服务锁检测 (Service Lock Check)**：
   - 运行：`powershell -Command "Get-Service clash_verge_service -ErrorAction SilentlyContinue"`
   - 验证：其 `Status` 必须为 `Stopped`。若为 `Running`，必须提醒用户停止服务以释放 `clash-verge-service.exe` 锁。
2. **残留进程锁检测 (Process Lock Check)**：
   - 运行：`powershell -Command "Get-Process -Name clash-mini, clash-verge, verge-mihomo, verge-mihomo-alpha -ErrorAction SilentlyContinue"`
   - 验证：输出必须为空，不得有任何残留代理或内核进程，以防文件锁死。
3. **端口占用检测 (Port Conflict Check)**：
   - 运行：`powershell -Command "Get-NetTCPConnection -LocalPort 10801, 9098 -ErrorAction SilentlyContinue"`
   - 验证：输出必须为空，目标 Mixed 端口和 API 端口不得被占用。
4. **【强制】本地清理缓存与真机 Dev 验证核对 (Local Cache Purge & Dev Verification)**：
   - **清理缓存**：必须首先在工作区运行 `Remove-Item -Recurse -Force node_modules/.vite` 命令清除旧的前端 Vite 编译缓存，确保编译状态为最新。
   - **Dev 运行**：本地必须使用 `pnpm dev`（安全隔离模式下，TUN 和系统代理为 `false`，端口 10801/9098 隔离）拉起开发调试服务。
   - **对齐汇报**：Agent 必须在本地对所有修改的 UI 元素（如按钮 3D 样式、卡片折行自适应、组件尺寸等）及后端逻辑进行物理效果与交互核对，并在聊天窗口中向用户提供详实的真机表现文字或状态证据汇报。**获得用户在对话中明确的“确认/同意”指令后，方可进行下一步的 GitHub 编译和发行。** 以后所有的发布，都必须强制走完这个本地双重核对流程，绝对禁止绕过。
5. **GitHub API 访问凭证准备 (GitHub Token Preparation)**：
   - 检查并读取项目根目录下的 [github_token.txt](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/github_token.txt)。
   - 验证并在此后的 Actions 状态查询或 Release 操作中，必须将该 Token 附在 API 请求的 `Authorization` 头部，严禁以无 Token 状态频繁匿名请求 GitHub 接口以免造成 IP 访问受限。

### 📢 反馈警报与放行规则
- **检测通过**：在聊天窗口向用户输出：**「环境检测与缓存清理通过，接下来的编译与验证环境已就绪。我将拉起本地 dev 服务进行第一轮物理效果验证，稍后为您汇报对齐结果，等待您的确认指令。」** 之后开始后续步骤。
- **检测失败**：立刻终止发行，并在聊天窗口中清晰指出哪一项被占用（如服务未停、端口冲突），并指导用户进行具体的手动操作，清除障碍后再试。

---

## 🛠️ 第二阶段：协议与 Bug 清单登记

在修改任何代码文件之前，必须率先更新文档，将本次修复或功能点落实在字面上：
1. **更新基本协议**：在 `clash_mini_agreements.md` 的正本正文对应章节中写入新功能或修改细节；并在「五、用户反馈问题状态跟踪清单」中追加登记 Bug，注明状态为“已解决”，写明具体方案。
2. **更新 Bug 跟踪表**：在 `bug_list.md` 的历史 Bug 表格中追加登记此 Bug ID、缺陷描述、解决版本及具体修复方案。

---

## 🚀 第三阶段：完全静默开发与校验流水线 (Silent Pipeline)

为避免弹窗干扰用户，所有的编译、测试与构建指令**必须使用 `powershell -Command` 进行静默封装**。

### 1. 代码修改与单元测试
- 编写修复代码（Rust 与前端 TSX）。
- 静默运行 Rust 单元测试：
  ```powershell
  powershell -Command "cargo test --package clash-mini --lib -- enhance::tests::test_enforce_mini_agreements_logic"
  ```
- 单元测试不通过，必须立即中断，严禁推标签。

### 2. 前端资产编译静态校验
- 运行前端静态类型检查与编译，验证无任何报错：
  ```powershell
  powershell -Command "pnpm web:build"
  ```
- 存在任何静态类型或编译错误，必须立刻停下，严禁强推。

### 3. 一键版本升级与推送
- 运行发布脚本修改版本号、打 tag 并推送至远端仓库（GitHub）：
  - **极速便携版发布 (默认，仅 Windows x64 便携版)**：
    ```powershell
    powershell -Command "pnpm publish-version <版本号>"
    ```
    *(例如：`pnpm publish-version 1.1.5`，这将触发 GitHub Actions 仅编译 Windows 64位绿色便携包，耗时约 2-3 分钟)*
  - **全平台完整发布 (指定 -full 或 -all 尾缀)**：
    ```powershell
    powershell -Command "pnpm publish-version <版本号>-full"
    ```
    *(例如：`pnpm publish-version 1.1.5-full`，这将编译并发布所有平台如 macOS、Linux、以及 Windows WebView2 固定版等，耗时约 10-15 分钟)*
- 该脚本会自动在本地更新三端版本号，提交更改，自动建立对应的 Git Tag，并使用 Git Push 推送 Tag 到 origin，从而在云端触发对应的构建流水线。

### 4. 远程构建监控与便携包自动拉回 (CI/CD Watch & Auto-Pull)
- 推送 Tag 后，构建过程需要几分钟。为了保证对用户的零弹窗打扰，**必须优先使用「浏览器静默监控」**；只有在必要时才使用「命令行监控」。
  
  - **方法 A：浏览器静默监控 (推荐 - 100% 零弹窗)**：
    1. **输出监控链接**：在启动监控的第一时间，必须将对应的 GitHub Actions Run 网页链接（`https://github.com/qiu-yuxiao/clash-mini/actions/runs/<RUN_ID>`）在对话中打印出来，提示用户可以自主点击该链接在浏览器中实时、直观地监视构建状态，从而无需依赖任何弹窗提醒。
    2. 访问或在后台使用 `read_url_content` 静默获取 GitHub Run 详情 API：`https://api.github.com/repos/qiu-yuxiao/clash-mini/actions/runs/<RUN_ID>`
    3. 或获取网页 HTML：`https://github.com/qiu-yuxiao/clash-mini/actions/runs/<RUN_ID>`
    4. 利用本地 `schedule` 定时器定时唤醒，在后台静默轮询 API/HTML 状态（检查 `"status": "completed"` 或 `streaming-graph-job` 的完成图标）。
    5. 这种方式完全不执行本地命令行，因此在整个编译监控期间**不会触发任何用户沙箱审批弹窗**。
  
  - **方法 B：命令行监控 (备用 - 需用户 Approve 弹窗)**：
    - 若需要使用本地 `gh` 工具监控，在根目录下使用 `github_token.txt` 作为凭证调用：
      ```powershell
      powershell -Command "$token = (Get-Content 'github_token.txt' -Raw).Trim(); $env:GH_TOKEN = $token; & 'C:\Program Files\GitHub CLI\gh.exe' run watch --repo qiu-yuxiao/clash-mini"
      ```
      *注意：运行本地命令需要用户在聊天界面点击「Submit」按钮审批。*

- **监控到发行完成后，立刻将 Windows 免安装版拉回指定测试路径**：
  - 运行下载命令将生成的 Windows x64 便携版绿色压缩包下载并覆盖至 `portable_test` 目录：
    ```powershell
    powershell -Command "$token = (Get-Content 'github_token.txt' -Raw).Trim(); $env:GH_TOKEN = $token; & 'C:\Program Files\GitHub CLI\gh.exe' release download v<版本号> --pattern '*_x64_portable.zip' --dir 'C:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\portable_test' --clobber --repo qiu-yuxiao/clash-mini"
    ```
  - *(注：便携包拉回后，即可在该目录下解压并由人工/AI 进行真机最后的 Bug 校验与回归测试)*

---

## 🏁 第四阶段：收尾与环境复原 (Wrap-Up & Environment Reset)

在本地或远程发布构建完成后，**必须依次执行以下收尾清理程序**，将工作区与宿主机状态恢复为最洁净状态，严禁草率交付：

1. **杀灭开发版残留进程 (Kill Resilient Dev Processes)**：
   - 运行：`powershell -Command "Stop-Process -Name clash-mini, tauri -ErrorAction SilentlyContinue"`
   - 验证并确保所有后台开发版程序、UI 和内核已被干净清空，将 `10801`、`9098` 端口及系统服务锁完全释放并归还宿主机。
2. **工作区环境打扫 (Workspace Cleaning)**：
   - 运行：`powershell -Command "Remove-Item -Recurse -Force node_modules/.vite"` 以彻底清除开发期的 Vite 构建缓存。
   - 审计：手动检查并删除除 `.gitignore` 中 `scratch/` 目录外的所有在开发期间临时产生或修改过的调试脚本、临时日志、调试 YAML 配置文件等，保证 Git Status 干净。
3. **缺陷闭环与 Walkthrough 归档 (Defect Tracking & Walkthrough Archive)**：
   - 缺陷更新：确保 `bug_list.md` 和 `clash_mini_agreements.md` 的 Bug 跟踪清单中，本次修复的缺陷已更新为“已解决”，并详实记录了修复方案。
   - 编写归档：在本地更新并保存 `walkthrough.md`，详细记录本次改动的物理事实、验证表现与测试结果，作为历史审计凭证。
4. **任务看板重置 (Task Board Reset)**：
   - 清空或重置 `task.md`，彻底清除已完成的 checklist 项，为下一个版本迭代的全新起点做好准备。
- 给用户留下一句简炼的总结消息，报告发行圆满完成。
