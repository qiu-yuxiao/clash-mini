# 👑 Clash Mini 一键静默发行准则 (One-Click Silent Release Guidelines)

**【警告：本准则具有法律级效力，仅次于宪法（.cursorrules）。凡执行发行动作的 Agent 必须 100% 遵守本规则。违反本规则的任何原生弹窗命令均被判定为严重故障。】**

---

## 🔍 第一阶段：环境就绪预检 (Environmental Pre-Check)

当接收到用户发出的发行/动工指令时，**必须首先在后台静默执行以下三项检测**。严禁不作检测直接修改或编译。

1. **服务锁检测 (Service Lock Check)**：
   - 运行：`powershell -Command "Get-Service clash_verge_service -ErrorAction SilentlyContinue"`
   - 验证：其 `Status` 必须为 `Stopped`。若为 `Running`，必须提醒用户停止服务以释放 `clash-verge-service.exe` 锁。
2. **残留进程锁检测 (Process Lock Check)**：
   - 运行：`powershell -Command "Get-Process -Name clash-mini, clash-verge, verge-mihomo, verge-mihomo-alpha -ErrorAction SilentlyContinue"`
   - 验证：输出必须为空，不得有任何残留代理或内核进程，以防文件锁死。
3. **端口占用检测 (Port Conflict Check)**：
   - 运行：`powershell -Command "Get-NetTCPConnection -LocalPort 10801, 9098 -ErrorAction SilentlyContinue"`
   - 验证：输出必须为空，目标 Mixed 端口和 API 端口不得被占用。

### 📢 反馈警报与放行规则
- **检测通过**：在聊天窗口向用户输出：**「环境检测通过，接下来的编译与发行环境已完全就绪，我可以独立跑完所有自动化任务，您可以离开了。」** 之后开始后续步骤。
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
  ```powershell
  powershell -Command "pnpm publish-version <版本号>"
  ```
  *(例如：`pnpm publish-version 1.1.1`)*
- 该脚本会自动在本地更新三端版本号，提交更改，自动建立 Git Tag `v<版本号>`，并使用 Git Push 推送 Tag 到 origin，从而触发云端 Actions 自动编译出绿色便携版。

---

## 🏁 第四阶段：收尾与Walkthrough归档

- 检查远端构建流水线进度（若适用），并在本地更新 `walkthrough.md` 记录本次发行所涉及的修改细节。
- 清空或重置 `task.md` 作为下一个里程碑的起点。
- 给用户留下一句简炼的总结消息，报告发行圆满完成。
