# 👑 Clash Mini 一键静默发行准则 (One-Click Silent Release Guidelines)

> [!IMPORTANT]
> **【内核版本发行准则】所有发行版本从现在起取消 v1.19.26 旧版内核强制绑定，默认动态拉取并使用官方最新发布的正式版内核（或 Prerelease-Alpha）进行打包发行。**
> **本准则供 AI Agent 自动发行使用。若违反本规则导致任何非预期弹窗或流程错误，均判定为严重故障。**

---

## 🚦 决策树：选择发行路径

1. **默认路径：云端直接极速发布（模块二）**
   - **触发条件**：用户未明确要求“本地测试”/“真机看效果”，或直接说“静默发布”/“直接发布”。
   - **流程**：直接跳过本地服务与端口检测，直接进入【模块二】。
2. **分支路径：本地验证后发布（模块一 + 模块二）**
   - **触发条件**：用户明确提出“要在本地测试”/“启动测试服务”/“真机确认效果”。
   - **流程**：执行【模块一】进行本地预检与对齐，用户确认后再执行【模块二】。

---

## ⚙️ 模块一：本地环境预检与真机对齐 (Local Verification)

1. **清理环境锁**：
   - 检查本地服务 `clash_verge_service`。若状态为 `Running`，**提醒用户手动运行 `Stop-Service -Name clash_verge_service`** 释放文件锁。
   - 检查并确保无 `clash-mini` 残留进程，且端口 `10801`, `9098` 未被占用。
2. **本地 Dev 验证**：
   - 清除缓存：`Remove-Item -Recurse -Force node_modules/.vite`。
   - 运行开发服务：`pnpm dev`（隔离模式，系统代理/TUN 默认为 `false`）。
   - **对齐汇报**：在聊天窗口向用户陈述 UI 表现（尺寸、3D、折行等），**等待用户在对话中明确回复“同意/开始发行”**后，终止本地服务并进入【模块二】。

---

## 🚀 模块二：云端静默发布与监控 (Remote Release)

1. **协议与缺陷登记**（修改顺序铁律）：
   - 在 [clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md) 登记新特性/修改，在 [bug_list.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/bug_list.md) 登记 Bug 状态为“代码已修正，待用户确认”。
   - **先独立提交文档**：运行 `git add` 仅包含协议与 Bug List，执行 `git commit -m "docs: register changes" --no-verify` 提交。
2. **静态资产校验**（云端发布唯一的防线）：
   - 运行前端编译：`pnpm web:build`。若有任何报错/警告，必须立即中止。
3. **提交业务代码**：
   - 运行 `git add .` 并执行 `git commit -m "feat: implement logic" --no-verify` 提交。
   - 配置 Git SSL 校验后端（Windows 平台必配，用以解决默认 Schannel 握手超时失败的问题）：
     ```powershell
     git config --local http.sslBackend openssl
     ```
   - 若是重新发布当前版本，先清除本地与远端同名 Tag：
     ```powershell
     git tag -d v<版本号>
     git push origin :refs/tags/v<版本号>
     ```
   - 升级版本号并推送 Tag 到 GitHub 触发 Actions 编译：
     ```powershell
     pnpm publish-version <版本号>
     ```
     *(注：对于极速发布，GitHub Actions 仅编译 Windows x64 便携版，云端构建耗时约 25-30 分钟；若是全平台完整发布 `<版本号>-full`，耗时约 30-40 分钟。)*
5. **云端 Actions 监控**：
   - 打印 Actions 运行链接（形如 `https://github.com/qiu-yuxiao/clash-mini/actions`）引导用户查看。
   - 读取 [github_token.txt](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/github_token.txt) 中的 Token 作为 API Authorization 头部。
   - 后台静默调用 GitHub API 获取当前 Run 状态直至完成，在此期间不调用 any 命令行，实现零弹窗。
6. **拉回包与环境复原**：
   - 运行下载命令将生成的 Windows x64 便携版绿色包拉回至 `portable_test`：
     ```powershell
     $token = (Get-Content 'github_token.txt' -Raw).Trim(); $env:GH_TOKEN = $token; gh release download v<版本号> --pattern '*_x64_portable.zip' --dir 'portable_test' --clobber --repo qiu-yuxiao/clash-mini
     ```
   - 恢复 Git 配置：
     ```powershell
     git config --local --unset http.proxy
     git config --local --unset http.sslBackend
     ```
   - 重置 `task.md` 看板，并在 `walkthrough.md` 归档测试表现，清空 Vite 缓存。

---

## 🔄 模块三：自动更新元数据维护 (Updater Metadata Maintenance)

在云端 Release 构建完成并生成对应的平台包及签名（Signature）后，需手动更新自动更新配置文件，以支持客户端的自动更新功能：

1. **更新配置文件**：
   - 打开 [updater/app-update.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/updater/app-update.json)。
   - 更新 `"version"`、`"notes"` 和 `"pub_date"` (ISO 8601 格式，例如 `"2026-06-14T15:16:44Z"`)。
   - 在 `"platforms"` 下添加/更新发布平台对应的包下载链接与签名。例如：
     ```json
     "platforms": {
       "windows-x86_64": {
         "signature": "<对应的 .sig 签名文件内容>",
         "url": "https://github.com/qiu-yuxiao/clash-mini/releases/download/v<版本号>/<安装包/压缩包文件名>.zip"
       }
     }
     ```
     *(注：若需支持自动安装，需提供带签名文件的 `.msi` 或 `.zip` 并在 platforms 中正确配置)*
2. **提交并推送更新**：
   - 确认修改后，将 `updater/app-update.json` 提交并推送至 `dev` 分支：
     ```powershell
     git add updater/app-update.json
     git commit -m "chore(updater): update app-update.json to v<版本号>" --no-verify
     git push origin dev
     ```
   - 未来如果您想要发布新更新并支持自动下载安装，只需在发布新版后，更新该 app-update.json 中的版本号和下载平台链接即可。

---

## ⚙️ GitHub Release Workflow 规范（Workflow Architecture）

为彻底解决 `release.yml` 中 `publish_release` job 因依赖被条件 skip 的 job 而导致的死锁问题，以及产物混乱问题，制定以下规范：

- **发布目标唯一化**：
  - `release.yml` 是 Clash Mini 的**唯一正式发行流水线**，仅在 `git push v<version>` 打 tag 时触发。
  - `autobuild.yml` 仅用于开发期间的自动构建测试，**严禁**将 `autobuild` 的产物当作正式 Release 使用。
- **Job 依赖链简化（无死锁设计）**：
  - `release.yml` 仅有三个 job：`check_tag_version` → `release` → `publish_release`。
  - `publish_release` **仅依赖 `release` job**，不依赖任何会被条件 skip 的 job，彻底消除死锁。
  - 删除 `generate_matrix`、`release-for-linux-arm`、`release-for-fixed-webview2`、`release-update`、`release-update-for-fixed-webview2`、`submit-to-winget`、`notify-telegram` 等所有多余 job。
- **唯一产物：Windows x64 绿色便携版**：
  - 正式 Release 只发行 `Clash.Mini_<version>_x64_portable.zip`（免安装绿色版）。
  - 不发行：Linux 构建、macOS 构建、WebView2 固定版、ARM Windows、DEB/RPM 包、安装版 `.exe` 等一切其他产物。
- **发布流程**：
  1. `git tag v<version>` 打 tag 并 `git push origin v<version>`
  2. `check_tag_version` 校验 tag 来源（仅限 dev 分支）与版本一致性
  3. `release` 在 Windows 虚拟机中编译 Tauri 并打包 portable.zip，上传为 Draft Release
  4. `publish_release` 获取已有 Draft Release，写入正式 Release Notes（含下载地址），PATCH `draft=false` 完成发布
- **禁止事项**：
  - 严禁在 `autobuild.yml` 的 `TAG_NAME` 使用正式版本号（如 `v1.3.4`），autobuild 必须固定为 `autobuild` tag。
  - 严禁在 `release.yml` 中混合引入会被条件 skip 的 job 作为 `needs` 依赖。
  - 严禁在正式 Release 发行说明中包含非 Windows 便携版的其他平台产物链接。
