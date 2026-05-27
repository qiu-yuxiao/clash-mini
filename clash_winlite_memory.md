# Clash WinLite Memory Checkpoint (记忆整理归档)

本文件是为防止开发测试服务重启或 AI 记忆截断后造成信息丢失而特别整理的记忆存盘。

---

## 📌 当前状态与开发进度 (State & Progress)
1. **毛玻璃与半透明彻底清除 (100% Solid Colors)**
   * **修改**：在全局 CSS (`layout.scss`)、前端组件 (`_layout.tsx`, `connection-table.tsx` 等) 以及 Rust 后端窗口配置 (`window.rs`) 中，完全移除了所有毛玻璃滤镜并将半透明背景提升为 100% 实体纯色（如亮色 `#ffffff`/`#f0f5ff`，暗色 `#1e2438`/`#0f1423`）。
   * **效果**：彻底消除了表格内容滚动到表头下方时漏光、文字重叠的问题，大幅降低了 WebView 渲染性能开销。
2. **流量图表与 3D 指标卡片 (Dashboard & 3D Cards)**
   * **修改**：调整了 `_layout.tsx` 高度结构，将下方流量仪表盘区域固定为 `178px`，将 Canvas 流量折线图高度还原为原版一致的 `130px`。
   * **效果**：完美解决了时间戳刻度线与 "Smooth"/"Points/FPS" 文字重叠的问题。同时在下方横向平铺了上传（淡金/琥珀）与下载（淡蓝）的 3D 立体彩色数据卡片，上边缘带高光微光。
3. **“外观主题”切换菜单 (Theme Switcher)**
   * **修改**：在左侧设置抽屉的“基础设置”内、日志按钮正上方，新增了“外观主题”下拉选择菜单。
   * **效果**：选项包含“系统默认”、“浅色模式”、“深色模式”四个字，大小字号与“代理策略”完全对齐，支持无刷新热重载。
4. **版本控制与变更记录规范 (Version & Tagging Policy)**
   * **固化起点**：以当前节点（版本 `0.2.0`，Git 提交 `531ba4ce`，标签 `重新出发`）为绝对固化的基准。
   * **版本递增**：后续任何功能修改、问题修复或协议变动，版本号一律以 `0.0.1` 为单位进行增量递增（例如下一次变更为 `0.2.1`）。
   * **标签与记录**：每次代码修改合并时，必须在 package.json/Tauri 配置中更新版本号，在 Git 中增加相应的版本标签（如 `v0.2.1`），并在《唯一协议》和本归档中作相应记录，以便清晰追踪自“重新出发”以来的所有变更。

---

## 🚨 核心生命安全红线 (Host Proxy Protection)
* **宿主机代理绝对不能杀**：AI 助手（AntiGravity）所在的运行环境依赖宿主机的原版 Clash Verge 代理服务器。在执行任何清理或重启命令时，**绝对禁止**调用类似 `taskkill /F /IM verge-mihomo.exe` 或 `clash-verge.exe` 的全局命令！
* **隔离策略**：仅能通过 `manage_task` 杀掉开发版 Tauri 进程，或者针对开发版的前端/后端进程进行精准清理。

---

## 🔍 启动前安全参数验证 (Config Validation)
我们已核实了开发版 AppData 配置（[verge.yaml](file:///C:/Users/sun_y/AppData/Roaming/io.github.clash-winlite.clash-winlite.dev/verge.yaml) 和 [config.yaml](file:///C:/Users/sun_y/AppData/Roaming/io.github.clash-winlite.clash-winlite.dev/config.yaml)）：
* **接管状态**：`enable_tun_mode: false`，`enable_system_proxy: false`，`tun.enable: false`（均已关闭，不会劫持宿主机网络）。
* **端口避让**：`mixed-port` / `verge_mixed_port` 均为 **`10801`**（与宿主机原版 Clash Verge 的默认端口隔离，防止端口占用崩溃）。

---

## 🧹 缓存清理目标 (Cache Purge)
为解决程序运行沉重（very sluggish/heavy）的问题，我们在重启前需要清理以下缓存：
1. **Vite 构建缓存**：工作区中的 `node_modules/.vite` 目录。
2. **WebView2 用户数据缓存 (UDF)**：本地的 `C:\Users\sun_y\AppData\Local\io.github.clash-winlite.clash-winlite` 目录（包含渲染器缓存、GPU 状态、Shader 编译等缓存）。

---

## 🔄 重新启动指南 (Dev Startup Guide)
1. **关闭旧任务**：确保终止正在运行的开发版 Tauri 任务。
2. **清理缓存**：在工作目录下运行 Powershell 缓存清理命令：
   ```powershell
   Remove-Item -Recurse -Force node_modules/.vite
   Remove-Item -Recurse -Force C:\Users\sun_y\AppData\Local\io.github.clash-winlite.clash-winlite
   ```
3. **重新启动**：向用户发起确认并运行以下开发调试启动命令：
   ```powershell
   node node_modules/@tauri-apps/cli/tauri.js dev -f verge-dev
   ```
