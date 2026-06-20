# 👑 Clash Mini 一键静默发行准则 (One-Click Silent Release Guidelines)

> [!IMPORTANT]
> **本准则供 AI Agent 自动发行使用。若违反本规则导致任何非预期弹窗或流程错误，均判定为严重故障。**
>
> **【内核版本发行准则】所有发行版本取消旧版内核强制绑定，默认动态拉取并使用官方最新发布的正式版内核（或 Prerelease-Alpha）进行打包发行。**

---

## 🔐 Token 使用铁律（最高优先级）

- GitHub 通信所用 Token **统一从项目根目录的 `github_token.txt` 文件读取**
- 必须以**设定环境变量**的方式使用（`$env:GH_TOKEN = ...`），绝对禁止将 Token 值打印、输出至日志或嵌入命令行字符串中暴露
- Token 读取示例（唯一合规写法）：
  ```powershell
  $env:GH_TOKEN = (Get-Content 'github_token.txt' -Raw).Trim()
  # 此后所有 gh 命令自动使用该环境变量，无需再传参
  ```

---

## 🛡️ 代理服务器保护铁律（最高优先级）

- 本机运行环境基于 TUN 模式代理服务器，**发行流程无需任何代理配置**
- **绝对禁止**杀灭或干涉以下进程及端口：`verge-mihomo`、`clash-verge`、`mini-mihomo`，端口 `10801`、`9098`
- 发行脚本中不得出现任何 `Stop-Process`、`taskkill` 针对上述进程的命令

---

## 🚦 标准发行路径（唯一推荐流程）

### 触发条件
用户本地代码已提交完毕，需要发行新版本。

### 一键执行
```powershell
.\scripts\release.ps1 <版本号>
# 示例：.\scripts\release.ps1 1.4.5
```

用户**仅需点击一次 Submit 确认**，脚本随后自动完成以下全部阶段，无需守候：

---

## 🃏 阶段一：本地准备（~1 分钟）

脚本自动执行，无任何交互弹窗：

1. **Token 读取**：从 `github_token.txt` 读取并设为 `$env:GH_TOKEN`（静默，不暴露值）
2. **前置检查**：
   - 确认当前在 `dev` 分支
   - 确认本地工作区干净（无未提交改动）
   - 确认版本号格式合法（`x.y.z`）
   - 确认 tag `v<版本号>` 不与现有 tag 冲突
3. **版本号更新**：调用 `python scripts/bump_version.py <版本号>` 更新以下三个文件：
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
4. **Git 操作（静默执行）**：
   ```
   git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
   git commit -m "release: bump version to <版本号>" --no-verify
   git push origin dev --no-verify
   git tag v<版本号>
   git push origin v<版本号> --no-verify
   ```
5. **打印 Actions 监控链接**：`https://github.com/qiu-yuxiao/clash-mini/actions`

> [!WARNING]
> **从 `git push origin v<版本号>` 触发 CI 直到 `update_dev` job 完成之前，禁止向 `dev` 分支推送任何新提交。**
> 原因：CI 的 `update_dev` job 会自动生成并 force-push `app-update.json` 到 `dev`，此前的任何 `git push origin dev` 都可能与其产生冲突或覆盖 CI 提交。

---

## 🃠 阶段二：云端构建监控（每 3 分钟轮询，约 30 分钟）

脚本通过 GitHub API 持续监控 Actions 运行状态，**每 3 分钟**打印一次进度：

```
[12:05] ⏳ CI 状态: in_progress | 已运行: 3 分钟 | 阶段: build
[12:08] ⏳ CI 状态: in_progress | 已运行: 6 分钟 | 阶段: build
...
[12:32] ✅ CI 状态: completed | 结论: success | 总耗时: 27 分钟
```

### 自动纠错机制

| 失败类型 | 自动处理方式 |
|---|---|
| 网络超时 / API 临时错误 | 等待 3 分钟后自动重试轮询，最多 2 次 |
| CI Run 排队超时（> 15 分钟未开始） | 自动删除 tag 并重推，触发新 Run，最多重试 1 次 |
| 构建失败（编译/签名错误） | 打印完整失败日志 URL，停止并报告，需人工介入 |

---

## 🏁 阶段三：验收与结束（~2 分钟）

CI 成功后，脚本自动：

1. 等待 Draft Release 转为正式发布（轮询 `draft=false`）
2. 下载 `setup.exe` 到 `portable_test/` 目录
3. 校验文件大小 > 0（确认产物可用）
4. 打印结束报告：
   ```
   ✅ 发行成功！
   版本：v1.4.5
   文件：portable_test/Clash.Mini_1.4.5_x64-setup.exe（42.3 MB）
   Release：https://github.com/qiu-yuxiao/clash-mini/releases/tag/v1.4.5
   ```

---

## 🔧 紧急回退方案（仅在 release.ps1 完全不可用时使用）

> [!CAUTION]
> 以下为手动应急操作，正常情况下**绝对不走此路径**。每一步均需手动执行，风险由操作者自行承担。

1. **更新版本号**：手动修改 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 中的版本字段
2. **文档登记**：在 `clash_mini_agreements.md` 登记新特性，在 `bug_list.md` 更新 Bug 状态
3. **Git 提交**：
   ```powershell
   git add .
   git commit -m "release: bump version to <版本号>" --no-verify
   git push origin dev --no-verify
   git tag v<版本号>
   git push origin v<版本号> --no-verify
   ```
4. **监控 CI**：手动访问 `https://github.com/qiu-yuxiao/clash-mini/actions` 查看构建状态
5. **拉回产物**：
   ```powershell
   $env:GH_TOKEN = (Get-Content 'github_token.txt' -Raw).Trim()
   gh release download v<版本号> --pattern '*_x64-setup.exe' --dir 'portable_test' --clobber --repo qiu-yuxiao/clash-mini
   ```
6. **更新 Git 配置**（若推送失败，配置 OpenSSL 后重试）：
   ```powershell
   git config --local http.sslBackend openssl
   # 推送后恢复：
   git config --local --unset http.sslBackend
   ```

---

## ⚙️ GitHub Release Workflow 规范（Workflow Architecture）

`release.yml` 是 Clash Mini 的**唯一正式发行流水线**，仅在 `git push v<version>` 打 tag 时触发。

### Job 依赖链（四步无死锁设计）

```
validate → build → update_dev → publish
```

- `validate`：快速校验 tag 来源（仅限 dev 分支）与版本一致性（~2 分钟）
- `build`：Windows x64 云端编译 Tauri，打包 setup.exe + portable.zip，上传为 Draft Release，自动签名并上传 .sig（~25-30 分钟）
- `update_dev`：从 Release 下载产物，自动生成 `app-update.json`（含正确的 sig/size/url），提交并 force-push 到 dev（~1 分钟）
- `publish`：从 Changelog.md 提取更新日志，将 Draft Release 发布为正式版（~1 分钟）

> [!IMPORTANT]
> **`app-update.json` 的维护权完全归 CI 的 `update_dev` job 所有。`release.ps1` 不生成、不提交此文件。**
> CI 会在构建完成后自动写入正确的签名、文件大小和下载 URL。

### 发布产物（从 v1.3.8 起）

**正式发行产物仅限 Windows x64 平台两项：**

- **【必需】** `Clash.Mini_<version>_x64-setup.exe`（NSIS 安装包，支持 Tauri 自动更新）
- **【可选】** `Clash.Mini_<version>_x64_portable.zip`（免安装绿色版，`continue-on-error: true`）

**以下产物任何情况下均不得发行：**
- ❌ Linux 构建 (.deb / .rpm / .AppImage)
- ❌ macOS 构建 (.dmg)
- ❌ ARM Windows 构建
- ❌ WebView2 固定版

### 禁止事项

- 严禁在 `autobuild.yml` 的 `TAG_NAME` 使用正式版本号，autobuild 必须固定为 `autobuild` tag
- 严禁在 `release.yml` 中混合引入会被条件 skip 的 job 作为 `needs` 依赖
- 严禁在正式 Release 发行说明中包含非 Windows 平台的产物链接

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

- 私钥文件：`tauri-key`（存放在项目根目录，已加入 `.gitignore`，严禁提交到 Git）
- 公钥文件：`tauri-key.pub`（内容已写入 `tauri.conf.json` 的 `updater.pubkey`）
- CI 构建时自动读取 `TAURI_PRIVATE_KEY` 和 `TAURI_KEY_PASSWORD` 两个 GitHub Secrets 完成签名

### app-update.json 自动维护

- CI `update_dev` job 在每次构建成功后自动生成并提交此文件，无需手动操作
- 确保 `signature` 字段与 `.sig` 文件内容完全一致（由 CI 保证）
- 确保 `url` 字段指向正确的 Release 下载链接（由 CI 保证）
- 确保 `size` 字段与实际文件字节数一致（由 CI 保证）

---

## 📝 发布检查清单

发布前必须确认：

- [ ] 本地代码已全部提交且推送（`git log origin/dev..dev` 无输出）
- [ ] `clash_mini_agreements.md` 已登记新特性/修改（并已独立提交）
- [ ] `bug_list.md` 已更新 Bug 状态（并已独立提交）
- [ ] `Changelog.md` 已添加本版本更新记录（供 CI 自动生成 Release Notes）
- [ ] 版本号已在 `package.json`、`tauri.conf.json`、`Cargo.toml` 中一致（由 release.ps1 自动完成）
- [ ] CI 构建成功，Release 已发布（非 Draft）
- [ ] `portable_test/` 中已有本版本 setup.exe 且文件大小正常（由 release.ps1 自动验收）
- [ ] GitHub 上可访问 `https://raw.githubusercontent.com/qiu-yuxiao/clash-mini/dev/updater/app-update.json`
- [ ] 旧版本客户端可收到自动更新提示
