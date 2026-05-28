# Clash WinLite 版本发布标准作业程序与检查清单 (Release SOP & Checklist)

本文件定义了 Clash WinLite 项目发行新版本的标准流程和自动化检查项。所有 AI 编码助手在准备发布新版本时，**必须严格按照本清单逐项核对并勾选**。

---

## 📋 第一阶段：编译前安全与合规性检查 (Pre-build Verification)
- [ ] **代码规范性静态校验**：
  - 运行 `tsc --noEmit`（或项目对应的 typecheck 脚本），确保前端 TypeScript 代码没有任何编译与类型错误。
  - 运行 `cargo check` 或 `cargo test` 确保 Rust 后端没有编译性错误。
- [ ] **清理本地缓存与私有配置**：
  - 检查并在编译输出目录（如 `target/release/.config`）中，**彻底删除**可能遗留的本地运行数据与缓存文件夹（如 `io.github.clash-winlite.clash-winlite` 和 `io.github.clash-winlite.clash-winlite.dev`）。
  - 确保 `.config` 文件夹下仅存在 `PORTABLE` 空标识文件，彻底杜绝个人订阅链接和代理证书泄露。
- [ ] **版本号一致性升级**：
  - 运行 `release-version` 脚本同步更新 `package.json`、`src-tauri/Cargo.toml` 以及 `src-tauri/tauri.conf.json` 中的版本号为即将发布的版本（如 `0.3.1`）。

---

## 🔨 第二阶段：构建与打包验证 (Build & Packaging)
- [ ] **等待生产编译彻底结束**：
  - 运行生产编译指令（如 `npm run build`）。
  - **必须等待编译进程完全退出且 Exit Code 为 0**。严禁在后台编译尚未结束时抢跑。
- [ ] **便携版绿色打包**：
  - 编译结束后，运行 `npm run portable`（它会触发修改后的 `portable.mjs`），将最新的二进制文件和完全干净的 `.config` 目录打包成 `Clash.WinLite_[Version]_[Arch]_portable.zip`。
- [ ] **双重大小与内容复核**：
  - 检查生成的压缩包体积是否正常（纯净包大小通常在 40MB~65MB 之间；如果体积异常增大至 70MB+，代表可能误将本地大缓存打包进去了，必须立即解压复核）。

---

## 🚀 第三阶段：代码合流与推送 (Git Push & Tagging)
- [ ] **提交并推送代码**：
  - 将所有修改的代码以及版本号变更文件提交（`git commit`），并推送到 GitHub 远程仓库的开发分支（如 `dev`）。
- [ ] **创建并推送 Git Tag**：
  - 在本地打上对应版本号的 Git 标签（例如 `v0.3.1`）：`git tag -a v0.3.1 -m "release v0.3.1"`。
  - 将该 Tag 推送至远程仓库：`git push origin v0.3.1`。

---

## 🚧 第四阶段：【核心阀门】人机协作与确认门禁 (Human-in-the-Loop Gatekeeper)
- [ ] **状态与现状陈述**：
  - 在对话中，AI 助手必须向用户清晰陈述以下信息：
    - 即将发布的版本号（如 `0.3.1`）。
    - 静态校验（类型检查与后端检查）是否 100% 通过。
    - 编译（npm run build）是否已经彻底完成退出（Exit Code 0）。
    - 打包压缩包的物理大小，以及是否已通过 `.config` 私有信息清理审计。
    - 代码和标签是否已成功推送至 GitHub 远程分支。
- [ ] **获取用户口头同意**：
  - **禁止在后台静默发布**。AI 助手陈述完上述现状后，必须显式询问用户是否可以进行正式发布。
  - **必须在对话中获得用户亲口输入的“同意/批准/可以发行”等指令后，方可触发 GitHub Release 创建与资产上传脚本**。
  - **这一步是防止 AI 因为并发逻辑漏洞、静默失败引发事故的最后一道人机纠错安全网。**
