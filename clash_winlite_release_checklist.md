# Clash WinLite 版本发布标准作业程序与检查清单 (Release SOP & Checklist)

本文件定义了 Clash WinLite 项目发行新版本的标准流程和自动化检查项。所有 AI 编码助手在准备发布新版本或修改环境时，**必须严格按照本清单逐项核对并勾选**，坚决杜绝遗漏和重复犯错。

---

## 📋 第一阶段：开发与测试期安全规范 (Development & Dev-Run Verification)

- [ ] **1.1 开发服务器启动前配置审计**：
  - 读取并核实开发版配置文件中（`AppData\Local\io.github.clash-winlite.clash-winlite.dev` 或 Roaming 下对应的 `verge.yaml` 和 `config.yaml`），`enable_tun_mode`、`enable_system_proxy` 和 `tun.enable` 均必须为 `false`。
  - 确认 Mixed Port 混合代理端口已避让为 `10801`，Controller API 端口已避让为 `9098`，严禁与生产/原版程序冲突。
  - **红线规诫**：严禁以 TUN 模式或系统代理模式启动开发服务器（`pnpm dev`），防止接管宿主机网络导致 AI 助手因断网与云端失联。

- [ ] **1.2 后台服务文件锁定状态检查**：
  - 运行开发编译前，检查工作空间下的 `resources/clash-verge-service.exe` 是否被 Windows 系统服务占用锁定。
  - 若出现 `os error 32`（共享占用冲突），必须引导用户在生产版/原版客户端中点击重新安装/修复服务，将 Windows 全局服务路径指回原版的正式安装目录，从而释放开发工作空间的文件锁定。

- [ ] **1.3 前端视图一致性校验**：
  - 任何时候如果发现运行测试时界面“退回了旧版”，必须立刻运行 `git status`、`git diff` 和 `git log` 检查本地最近的提交记录，严禁产生“代码存放在别处”的幻觉。
  - 确认清理了 `node_modules/.vite` 与 `AppData\Local\io.github.clash-winlite.clash-winlite` 的本地缓存后再试，确保看到的是本地最新的代码表现。

---

## 🔨 第二阶段：编译前安全与合规性检查 (Pre-build Verification)

- [ ] **2.1 前端资源独立编译（核心校验）**：
  - **必须手动运行前端生产编译指令**：`pnpm web:build`（它执行 `tsc --noEmit && vite build`）。
  - **必须等待前端编译进程完全退出且 Exit Code 为 0**。
  - **红线规诫**：由于 `tauri.conf.json` 中配置的 `beforeBuildCommand` 为空，直接运行 `tauri build` **绝不会**自动触发前端重新编译！若不手动运行 `pnpm web:build`，打包出来的程序将包含过期的前端静态资产。
  - **依赖审计**：切勿为了清理“无用文件”而删除 `src/polyfills` 文件夹下的任何兼容垫片文件（如 `matchMedia.js`, `WeakRef.js`, `RegExp.js`），必须确保 `vite.config.mts` 中引用的所有静态资源文件 100% 存在且未发生未决删除。

- [ ] **2.2 代码规范性与类型检查**：
  - 前端类型校验：确保 `tsc --noEmit` 没有抛出任何 TypeScript 类型编译错误。
  - 后端静态校验：运行 `cargo check` 确保 Rust 后端没有编译和语法错误。
  - 版本号一致性：检查 `package.json`、`src-tauri/Cargo.toml` 以及 `src-tauri/tauri.conf.json` 中的版本号已全部同步更新为即将发布的版本（如 `0.3.2`）。

- [ ] **2.3 清理本地缓存与私有配置**：
  - 检查并在打包前，彻底删除编译输出目录（如 `target/release/.config`）中可能遗留的本地运行数据与缓存文件夹（如 `io.github.clash-winlite.clash-winlite` 和 `io.github.clash-winlite.clash-winlite.dev`）。
  - 确保 `.config` 文件夹下仅存在 `PORTABLE` 空标识文件，彻底杜绝个人订阅链接和代理证书泄露。

- [ ] **2.4 性能与资源控制审计**：
  - 检查 `src-tauri/src/utils/resolve/window.rs` 中的 Windows WebView 窗口构建配置，确保已注入限制浏览器磁盘缓存大小的命令行参数：`.additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disk-cache-size=31457280")`。

---

## 🚀 第三阶段：构建、打包与推送 (Build & Packaging)

- [ ] **3.1 生产构建（编译后端与打包）**：
  - 运行 `pnpm build`（即 `tauri build`），将最新的前端构建静态资产（来自已成功生成的 `dist` 目录）与 Rust 后端代码一同编译打包。
  - 必须等待编译进程完全退出且 Exit Code 为 0. 严禁在后台编译尚未结束时抢跑。

- [ ] **3.2 便携版绿色打包**：
  - 编译结束后，运行 `pnpm portable`（它会触发修改后的 `portable.mjs`），将最新的二进制文件 and 完全干净的 `.config` 目录打包成 `Clash.WinLite_[Version]_[Arch]_portable.zip`。

- [ ] **3.3 双重大小与内容复核**：
  - 检查生成的压缩包体积是否正常（纯净包大小通常在 40MB~65MB 之间；如果体积异常增大至 70MB+，代表可能误将本地大缓存打包进去了，必须立即解压复核）。

- [ ] **3.4 代码合并与 Git 推送**：
  - 将所有修改的代码、依赖垫片、以及版本号变更文件提交（`git commit`），并推送到 GitHub 远程仓库的开发分支（如 `dev`）。如果 pre-push hook 中的 clippy 规则在非业务代码（如测试用例、已停用功能）上报 warning，可使用 `git push origin dev --no-verify` 合规推送。
  - 在本地打上对应版本号的 Git 标签（例如 `v0.3.2`）：`git tag -a v0.3.2 -m "release v0.3.2"`。
  - 将该 Tag 推送至远程仓库：`git push origin v0.3.2 --no-verify`。

---

## 🚧 第四阶段：【核心阀门】人机协作与确认门禁 (Human-in-the-Loop Gatekeeper)

- [ ] **4.1 状态与现状陈述**：
  - 在对话中，AI 助手必须向用户清晰陈述以下信息：
    - 即将发布的版本号。
    - 前端静态类型校验与后端 `cargo check` 是否 100% 通过。
    - 独立前端编译（`pnpm web:build`）与后端编译（`tauri build`）是否已经彻底完成退出（Exit Code 0）。
    - 打包压缩包的物理大小，以及是否已通过 `.config` 私有信息清理审计。
    - 代码和标签是否已成功推送至 GitHub 远程分支。

- [ ] **4.2 获取用户口头同意**：
  - **禁止在后台静默发布**。AI 助手陈述完上述现状后，必须显式询问用户是否可以进行正式发布。
  - **必须在对话中获得用户亲口输入的“同意/批准/可以发行”等指令后，方可触发 GitHub Release 创建与资产上传脚本**。
  - **这一步是防止 AI 因为并发逻辑漏洞、静默失败引发事故的最后一道人机纠错安全网。**
