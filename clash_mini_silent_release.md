# 👑 Clash Mini 一键静默发行准则 (One-Click Silent Release Guidelines)

> [!IMPORTANT]
> **【内核版本发行准则】所有发行版本从现在起取消 v1.19.26 旧版内核强制绑定，默认动态拉取并使用官方最新发布的正式版内核（或 Prerelease-Alpha）进行打包发行。**
> **本准则供 AI Agent 自动发行使用。若违反本规则导致任何非预期弹窗或流程错误，均判定为严重故障。**

---

## 🚦 决策树：选择发行路径

1. **默认路径：云端直接极速发布（模块二）**
   - **触发条件**：用户未明确要求"本地测试"/"真机看效果"，或直接说"静默发布"/"直接发布"。
   - **流程**：直接跳过本地服务与端口检测，直接进入【模块二】。
2. **分支路径：本地验证后发布（模块一 + 模块二）**
   - **触发条件**：用户明确提出"要在本地测试"/"启动测试服务"/"真机确认效果"。
   - **流程**：执行【模块一】进行本地预检与对齐，用户确认后再执行【模块二】。

---

## 🃏 模块一：本地环境预检与真机对齐 (Local Verification)

1. **清理环境锁**：
   - 检查本地服务 `clash_verge_service`。若状态为 `Running`，**提醒用户手动运行 `Stop-Service -Name clash_verge_service`** 释放文件锁。
   - 检查并确保无 `clash-mini` 残留进程，且端口 `10801`, `9098` 未被占用。
2. **本地 Dev 验证**：
   - 清除缓存：`Remove-Item -Recurse -Force node_modules/.vite`。
   - 运行开发服务：`pnpm dev`（隔离模式，系统代理/TUN 默认为 `false`）。
   - **对齐汇报**：在聊天窗口向用户陈述 UI 表现（尺寸、3D、折行等），**等待用户在对话中明确回复"同意/开始发行"**后，终止本地服务并进入【模块二】。

---

## 🃠 模块二：云端静默发布与监控 (Remote Release)

### 方式A：使用本地脚本发布（推荐）

运行 `scripts/release.ps1` 脚本，交互式完成发布：

```powershell
.\scripts\release.ps1 1.3.9
```

脚本自动完成：
1. 更新 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 版本号
2. 提交版本变更
3. 本地构建并签名 `setup.exe`
4. 更新 `updater/app-update.json`
5. 创建 tag 并推送到 GitHub 触发 CI

### 方式B：手动云端发布

1. **协议与缺陷登记**（修改顺序铁律）：
   - 在 `clash_mini_agreements.md` 登记新特性/修改，在 `bug_list.md` 登记 Bug 状态为"代码已修正，待用户确认"。
   - **先独立提交文档**：运行 `git add` 仅包含协议与 Bug List，执行 `git commit -m "docs: register changes" --no-verify` 提交。
2. **静态资产校验**（云端发布唯一的防线）：
   - 运行前端编译：`pnpm web:build`。若有任何报错/警告，必须立即中止。
3. **提交业务代码**：
   - 运行 `git add .` 并执行 `git commit -m "feat: implement logic" --no-verify` 提交。
   - 配置 Git SSL 校验后端（Windows 平台必配，用以解决默认 Schannel 握手超时失败的问题）：
     ```powershell
     git config --local http.sslBackend openssl
     ```
   - 若是重新发布当前版本，先清除本地与远端同名 Tag，**同时清理残留的旧 Release**（防止 publish_release 因重复 Release 而失败）：
     ```powershell
     git tag -d v<版本号>
     git push origin :refs/tags/v<版本号>
     # 删除 GitHub 上与该 tag 关联的所有旧 Release
     $tag = "v<版本号>"
     $ids = gh api "repos/qiu-yuxiao/clash-mini/releases?per_page=10" --jq ".[] | select(.tag_name == `"`"$tag`"`") | .id"
     foreach ($id in $ids) { gh api -X DELETE "repos/qiu-yuxiao/clash-mini/releases/$id" --silent }
     ```
   - **发版顺序铁律（必须严格执行）：**
     1. 先确认本地所有提交已推送：`git log origin/dev..dev` 无输出才算干净
     2. 再升级版本号、打 tag、推送 tag 触发 CI：
     ```powershell
     # 手动修改版本号
     # package.json
     # src-tauri/tauri.conf.json
     # src-tauri/Cargo.toml
     git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
     git commit -m "release: bump version to <版本号>" --no-verify
     git push origin dev --no-verify
     git tag v<版本号>
     git push origin v<版本号> --no-verify
     ```
     **严格按顺序：先 `git push origin dev`，再 `git push origin v<版本号>`。**
     *(注：对于极速发布，GitHub Actions 仅编译 Windows x64，云端构建耗时约 25-30 分钟)*
4. **云端 Actions 监控**：
   - 打印 Actions 运行链接（形如 `https://github.com/qiu-yuxiao/clash-mini/actions`）引导用户查看。
   - 读取 `github_token.txt` 中的 Token 作为 API Authorization 头部。
   - 后台静默调用 GitHub API 获取当前 Run 状态直至完成，在此期间不调用 any 命令行，实现零弹窗。
5. **拉回包与环境复原**：
   - 运行下载命令将生成的 Windows x64 安装包拉回至 `portable_test`：
     ```powershell
     $token = (Get-Content 'github_token.txt' -Raw).Trim(); $env:GH_TOKEN = $token; gh release download v<版本号> --pattern '*_x64-setup.exe' --dir 'portable_test' --clobber --repo qiu-yuxiao/clash-mini
     ```
   - 恢复 Git 配置：
     ```powershell
     git config --local --unset http.proxy
     git config --local --unset http.sslBackend
     ```
   - 重置 `task.md` 看板，并在 `walkthrough.md` 归档测试表现，清空 Vite 缓存。

---

## 🔄 模块三：自动更新元数据维护 (Updater Metadata Maintenance)

从 v1.3.8 开始，自动更新使用 `setup.exe`（NSIS 安装包）。发布完成后需更新 `updater/app-update.json`：

### 方式A：使用本地脚本（推荐）

`scripts/release.ps1` 脚本已自动完成此步骤。

### 方式B：手动更新

1. **下载安装包并签名**：
   - 下载 GitHub Release 上的 `Clash.Mini_<版本号>_x64-setup.exe`
   - 使用本地私钥签名：
     ```powershell
     $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content 'tauri-key' -Raw).Trim()
     $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = 'clash-mini-auto-update-2026'
     pnpm tauri signer sign "Clash.Mini_<版本号>_x64-setup.exe"
     ```
   - 上传 `.sig` 文件到 Release：
     ```powershell
     gh release upload v<版本号> "Clash.Mini_<版本号>_x64-setup.exe.sig" --repo qiu-yuxiao/clash-mini
     ```

2. **更新配置文件**：
   - 打开 `updater/app-update.json`。
   - 更新 `"version"`、`"notes"` 和 `"pub_date"` (ISO 8601 格式，例如 `"2026-06-14T15:16:44Z"`)。
   - 在 `"platforms"` 下添加/更新发布平台对应的包下载链接与签名：
     ```json
     "platforms": {
       "windows-x86_64": {
         "signature": "<对应的 .sig 签名文件内容>",
         "url": "https://github.com/qiu-yuxiao/clash-mini/releases/download/v<版本号>/Clash.Mini_<版本号>_x64-setup.exe",
         "size": <文件字节数>
       }
     }
     ```

3. **提交并推送更新**：
   - 确认修改后，将 `updater/app-update.json` 提交并推送至 `dev` 分支：
     ```powershell
     git add updater/app-update.json
     git commit -m "chore(updater): update app-update.json to v<版本号>" --no-verify
     git push origin dev
     ```

---

## ⚙️ GitHub Release Workflow 规范（Workflow Architecture）

`release.yml` 是 Clash Mini 的**唯一正式发行流水线**，仅在 `git push v<version>` 打 tag 时触发。

### Job 依赖链（无死锁设计）

`release.yml` 仅有三个 job：`check_tag_version` → `release` → `publish_release`。

- `publish_release` **仅依赖 `release` job**，不依赖任何会被条件 skip 的 job，彻底消除死锁。
- 删除 `generate_matrix`、`release-for-linux-arm`、`release-for-fixed-webview2`、`release-update`、`release-update-for-fixed-webview2`、`submit-to-winget`、`notify-telegram` 等所有多余 job。

### 发布产物（从 v1.3.8 起）

**Clash Mini 的正式发行产物仅限于以下 Windows x64 平台的两项，除此之外禁止发行任何其他平台或格式的构建产物：**

- **【必需】主要产物**：`Clash.Mini_<version>_x64-setup.exe`（NSIS 安装包），支持 Tauri 自动更新器。**每次发行必须包含此产物。**
- **【可选】次要产物**：`Clash.Mini_<version>_x64_portable.zip`（免安装绿色版），仅供手动下载使用，**不用于自动更新**。CI 中此步骤设置了 `continue-on-error: true`，失败不影响发行流程。

**以下产物在任何情况下均不得发行**（包括但不限于）：
- ❌ Linux 构建 (.deb / .rpm / .AppImage / snap)
- ❌ macOS 构建 (.dmg)
- ❌ ARM Windows 构建 (ARM64 / x86)
- ❌ WebView2 固定版
- ❌ DEB/RPM 包
- ❌ 任何其他未在此章节列出的产物

### 发布流程

1. `git tag v<version>` 打 tag 并 `git push origin v<version>`
2. `check_tag_version` 校验 tag 来源（仅限 dev 分支）与版本一致性
3. `release` 在 Windows 虚拟机中编译 Tauri 并打包 `setup.exe` 和 `portable.zip`，上传为 Draft Release
4. CI 自动签名 `setup.exe` 并上传 `.sig` 文件
5. `publish_release` 获取已有 Draft Release，写入正式 Release Notes（含下载地址），PATCH `draft=false` 完成发布

### 禁止事项

- 严禁在 `autobuild.yml` 的 `TAG_NAME` 使用正式版本号（如 `v1.3.8`），autobuild 必须固定为 `autobuild` tag。
- 严禁在 `release.yml` 中混合引入会被条件 skip 的 job 作为 `needs` 依赖。
- 严禁在正式 Release 发行说明中包含非 Windows 平台的产物链接。

---

## 📦 发布产物说明（从 v1.3.8 起）

### `Clash.Mini_<version>_x64-setup.exe`（主要产物）

- NSIS 安装包，支持 Tauri 自动更新器
- 用户下载后运行安装，程序安装在 `C:\Users\<用户名>\AppData\Local\Programs\clash-mini\`
- 自动更新时会下载新版本的 `setup.exe` 并自动运行安装

### `Clash.Mini_<version>_x64_portable.zip`（可选产物）

- 免安装绿色版，解压即用
- **不支持自动更新**（Tauri 更新器在 Windows 上不支持 ZIP）
- 仅供不想安装的用户手动下载使用

---

## 🔐 签名与自动更新规范

### 私钥管理

- 私钥文件：`tauri-key`（存放在项目根目录）
- 公钥文件：`tauri-key.pub`（存放在项目根目录，内容已写入 `tauri.conf.json` 的 `updater.pubkey`）
- **严禁**将私钥文件提交到 Git 仓库（已加入 `.gitignore`）

### CI 签名

- CI 构建时自动读取 `TAURI_PRIVATE_KEY` 和 `TAURI_KEY_PASSWORD` 两个 GitHub Secrets
- `tauri build` 命令会自动签名生成的安装包
- `.sig` 文件会随构建产物一起上传到 Release

### app-update.json 更新

- 每次发布新版本后，必须手动更新 `updater/app-update.json`
- 确保 `signature` 字段与 `.sig` 文件内容完全一致
- 确保 `url` 字段指向正确的 Release 下载链接
- 确保 `size` 字段与 actual 文件字节数一致

---

## 📝 发布检查清单

发布前必须确认：

- [ ] `package.json` 版本号已更新
- [ ] `src-tauri/tauri.conf.json` 版本号已更新
- [ ] `src-tauri/Cargo.toml` 版本号已更新
- [ ] `release.yml` 中 `git push` 命令使用 `HEAD:dev` 格式（非 `origin dev`），确保 CI 在 detached HEAD 状态下能正确推送
- [ ] `release.yml` 中 build artifacts 路径与 `tauri.conf.json` 的 `productName` 一致（防止 find 找不到文件）
- [ ] `clash_mini_agreements.md` 已登记新特性/修改
- [ ] `bug_list.md` 已更新 Bug 状态
- [ ] 本地无未推送提交（`git log origin/dev..dev` 无输出）
- [ ] CI 构建成功，Release 已发布（非 Draft）
- [ ] `.sig` 文件已上传到 Release
- [ ] `updater/app-update.json` 已更新并推送到 `dev` 分支
- [ ] GitHub 上可访问 `https://raw.githubusercontent.com/qiu-yuxiao/clash-mini/dev/updater/app-update.json`
- [ ] 旧版本客户端可收到自动更新提示
