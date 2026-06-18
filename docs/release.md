# Clash Mini 发布流程文档

> 最后更新：2026-06-19
> 负责人：Master

## 概述

Clash Mini 使用 **Tauri 2** 的自动更新机制。发布新版本需要完成以下步骤：
1. 更新版本号（3 个文件）
2. 构建并签名（生成 `setup.exe` 和 `.sig`）
3. 更新 `updater/app-update.json`
4. 推送 tag 触发 CI
5. CI 自动构建、签名、上传 Release

---

## 版本号文件（需同步更新）

| 文件 | 字段 |
|------|------|
| `package.json` | `"version"` |
| `src-tauri/tauri.conf.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version = "x.y.z"` |

`Cargo.lock` 会在构建时自动更新，不需要手动改。

---

## 方式一：本地脚本发布（推荐）

运行 PowerShell 脚本，全自动完成：

```powershell
# 交互式（会询问版本号）
.\scripts\release.ps1

# 直接指定版本号
.\scripts\release.ps1 1.3.9

# 只准备不推送（用于测试）
.\scripts\release.ps1 1.3.9 -NoPush

# 跳过构建（已有构建产物）
.\scripts\release.ps1 1.3.9 -SkipBuild
```

脚本会自动完成：
- 更新 3 个版本号文件
- `pnpm i` → `pnpm run prebuild` → `pnpm run web:build` → `pnpm tauri build`
- 用本地私钥签名 `setup.exe`
- 更新 `updater/app-update.json`
- 提交代码、创建 tag、推送到 GitHub
- CI 自动接管后续流程

---

## 方式二：手动发布

### 第 1 步：更新版本号

手动修改以下文件的版本号：
- `package.json` → `"version": "1.3.9"`
- `src-tauri/tauri.conf.json` → `"version": "1.3.9"`
- `src-tauri/Cargo.toml` → `version = "1.3.9"`

### 第 2 步：构建

```bash
pnpm i
pnpm run prebuild x86_64-pc-windows-msvc
pnpm run web:build
pnpm tauri build --target x86_64-pc-windows-msvc
```

构建产物在：
`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Clash.Mini_1.3.9_x64-setup.exe`

### 第 3 步：签名

```bash
# 设置私钥环境变量（或从 tauri-key 文件读取）
pnpm tauri signer sign "src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Clash.Mini_1.3.9_x64-setup.exe"
```

签名文件会生成在同目录：`Clash.Mini_1.3.9_x64-setup.exe.sig`

### 第 4 步：更新 app-update.json

编辑 `updater/app-update.json`：

```json
{
  "version": "1.3.9",
  "notes": "Clash Mini v1.3.9 — 填写更新说明",
  "pub_date": "2026-06-19T06:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "（填入 .sig 文件的完整内容，去掉换行）",
      "url": "https://github.com/qiu-yuxiao/clash-mini/releases/download/v1.3.9/Clash.Mini_1.3.9_x64-setup.exe",
      "size": 42265376
    }
  }
}
```

**注意：** `signature` 必须是 `.sig` 文件的 base64 内容，**不能有换行符或非法字符**（之前曾因 `0x0c` 字符导致解码失败）。

### 第 5 步：提交并打 tag

```bash
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock updater/app-update.json
git commit -m "release: bump version to 1.3.9"
git tag v1.3.9
git push origin dev
git push origin v1.3.9
```

### 第 6 步：等待 CI

CI 会自动：
1. 构建 `setup.exe`
2. 用 `TAURI_PRIVATE_KEY` 和 `TAURI_KEY_PASSWORD` 签名
3. 上传 `setup.exe` 和 `.sig` 到 GitHub Release
4. 更新 Release Notes
5. 用户打开旧版本时会收到自动更新提示

---

## CI 配置说明

配置文件：`.github/workflows/release.yml`

### 环境变量 / Secrets

| 名称 | 说明 | 设置位置 |
|------|------|----------|
| `TAURI_PRIVATE_KEY` | Tauri 签名私钥（base64 编码） | GitHub Repository Secrets |
| `TAURI_KEY_PASSWORD` | 私钥密码 | GitHub Repository Secrets |
| `GITHUB_TOKEN` | GitHub API 访问令牌（自动提供） | GitHub Actions 自动注入 |

### CI 流程

```
push tag v*.*.*
    │
    ▼
check_tag_version（校验 tag 来自 dev 分支，版本号一致）
    │
    ▼
release（Windows 构建）
    ├── Tauri build（生成 setup.exe，自动签名）
    ├── Sign setup.exe（确保 .sig 存在）
    ├── Upload .sig to Release
    └── Update app-update.json and push to dev ← 关键步骤
    │
    ▼
publish_release（更新 Release Notes）
```

---

## 自动更新机制

### 客户端如何检测更新

客户端（`tauri.conf.json` 配置）：
```json
"plugins": {
  "updater": {
    "pubkey": "（公钥，与私钥配对）",
    "endpoints": [
      "https://raw.githubusercontent.com/qiu-yuxiao/clash-mini/dev/updater/app-update.json"
    ]
  }
}
```

每次启动时会访问 `app-update.json`，如果 `version` 比当前版本新，就弹出更新提示。

### 自动更新流程

1. 客户端下载 `app-update.json`
2. 比对 `version` 字段
3. 如果新版本 → 下载 `url` 指定的 `setup.exe`
4. 用 `pubkey` 验证 `signature`
5. 验证通过 → 运行 `setup.exe`（NSIS 被动模式安装）
6. 安装完成后自动重启应用

---

## 常见问题

### Q: 自动更新失败，提示 "Invalid encoding in minisign data"

**原因：** `app-update.json` 里的 `signature` 有非法字符（如 `0x0c`）或换行符。

**解决：** 用脚本读取 `.sig` 文件 → `tr -d '\n\r'` → 写入 JSON，不要手动编辑 signature 字段。

### Q: 更新后变成了安装版，便携版还在吗？

**是的。** 从 v1.3.8 开始，`app-update.json` 指向 `setup.exe`，自动更新会安装到：
`C:\Users\用户名\AppData\Local\Programs\clash-mini\`

旧的便携版文件夹需要手动删除。

### Q: 如何回滚版本？

1. 在 GitHub Release 页面删除当前版本的 Release
2. 将 `app-update.json` 的 `version` 改回旧版本
3. 推送更新后的 `app-update.json`

---

## 密钥管理

- **私钥文件：** `tauri-key`（已加入 `.gitignore`，不会提交到 Git）
- **公钥文件：** `tauri-key.pub`（已加入 `.gitignore`）
- **私钥密码：** `clash-mini-auto-update-2026`
- **GitHub Secrets：** `TAURI_PRIVATE_KEY`（私钥内容的 base64）、`TAURI_KEY_PASSWORD`

**⚠️ 警告：** 绝对不要把私钥内容写进 `.git/config`、命令行参数、或任何日志里。

---

## 检查清单（发布前）

- [ ] `package.json` 版本号已更新
- [ ] `tauri.conf.json` 版本号已更新
- [ ] `Cargo.toml` 版本号已更新
- [ ] `bug_list.md` 已更新（关闭已修复的 bug）
- [ ] 代码已提交到 `dev` 分支
- [ ] Tag 已创建并推送
- [ ] CI 构建成功
- [ ] `.sig` 文件已上传到 Release
- [ ] `app-update.json` 已更新（版本号、签名、size 都正确）
- [ ] `app-update.json` 已推送到 `dev` 分支
- [ ] GitHub 上的 `app-update.json` 可以访问（用浏览器打开验证）
