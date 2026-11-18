# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：`BUG-057` ~ `BUG-391` 已通过用户验收并完整归档于 `clash_mini_agreements.md` 附录 A。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-398 2026-11-19 `clash-mini.exe` 连续 5 次瞬杀真因定位（邻库 `ClashWin` 单测 `cwrun.exe` 误杀）与 `useroptm2` 预防性防误杀指南收口

- **对应模块**：[轻量模式防误杀指南.txt](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/轻量模式防误杀指南.txt)、[antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md)、[prebuild.mjs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/scripts/prebuild.mjs)、[portable.mjs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/scripts/portable.mjs)、[installer.nsi](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi)、[.gitignore](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/.gitignore)、[app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs)、[lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs)、[cmds.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts)、[help-menu-button.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/help-menu-button.tsx)、[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、`src/locales/*/layout.json`
- **问题描述**：① 2026-11-19 实机在已配置 `whiteinternal.ini`、随后又补齐 `useroptm2.ini` 与 `useroptm_game2.ini` 并升级至 `v3.5.5` 后，`clash-mini.exe` 仍先后在 `00:52:48`、`01:31:59`、`03:14:27`、`03:59:32`、`05:21:15` 发生 5 次日志断尾式瞬杀（均无崩溃转储，且 SYSTEM 权限运行的 `mini-mihomo.exe` 每次均存活）。经交叉比对 `360Safe` `galaxy2.dat`、`PopWndTrackerLog`、`performancerecord.db` 及 Win32 `GetPrivateProfileIntW` 实测，确认 360 三份白名单均已生效、`01:31:59` 及全部 5 次强杀均非 360 所为；5 次死亡时间点与邻库 `ClashWin` 的单元测试运行器 `cwrun.exe` 启动时间（`00:51:51`、`01:31:04`、`03:13:23`、`03:58:35`、`05:20:42`）5/5 一一对应。进一步追踪 `ClashWin` 源码确认：`tests/test_t18_sysproxy.cpp`（`t18_tun_patch_chain_verge_and_clash_layers`）在无内核运行状态下调用 `patch_verge_config({"enable_tun_mode": true})`，触发 `src/cmd/cmd_verge.cpp` 在检查 `if (is_core_running())` 之前无条件调用 `system::coexist::kill_other_processes()`，对同用户会话下的 `clash-mini.exe` 执行 `::TerminateProcess(p, 1)` 瞬杀（而对 SYSTEM 权限的 `mini-mihomo.exe` 因 `ERROR_ACCESS_DENIED` 未能杀掉）；已同步在 `ClashWin` 侧补齐测试宿主隔离保护并收紧 `cmd_verge.cpp` 调用前置条件；② `v3.5.5` 随包发布的 `轻量模式防误杀指南.txt` 及帮助菜单直链入口作为对 360 `netmon\360perfoptm2.dll` 第二引擎（`FastOptm`/`DoOptm`/`FastOptmForGameMode` 读取 `useroptm2.ini` 与 `useroptm_game2.ini` 的 `check=0`）的预防性防护指南予以保留，不再依赖自绘 `AntivirusWhitelistDialog` 弹窗承载大段文本。
- **修复方案**：
  1. 在仓库根目录创建纯文本指南 [轻量模式防误杀指南.txt](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/轻量模式防误杀指南.txt)（同步对齐 [antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md)），严格基于实证拆解两套引擎，指导用户在 **Windows 安全模式** 一次补齐 `whiteinternal.ini` + `useroptm2.ini` + `useroptm_game2.ini`（`[clash-mini.exe]` / `[clash mini_0]` / `[mini-mihomo.exe]` 均设 `check=0`）共 3 个底层白名单文件，并如实说明「静默启动」仅避开机制一的边界；
  2. 参照 `用户必读.txt` 打包管线，更新 [.gitignore](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/.gitignore)、[prebuild.mjs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/scripts/prebuild.mjs)、[installer.nsi](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi) 与 [portable.mjs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/scripts/portable.mjs)，使安装目录（`$INSTDIR\轻量模式防误杀指南.txt`、`resources\`）与便携包根目录随包释放该文档并在卸载时同步清理；
  3. 后端在 [app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs) 与 [lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs) 新增并注册 `open_antivirus_guide` 命令（按执行目录 → `resources/` → dev 工作区顺序解析路径并以 `open::that` 直接用系统文本查看器打开，附路径优先顺序单元测试）；前端新增 `openAntivirusGuide`（[cmds.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts)），保留 `?` 帮助菜单 [help-menu-button.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/help-menu-button.tsx) 中的 `🛡️ 轻量模式防误杀指南` 菜单项并在点击时直接调用系统查看器打开 TXT 文档，彻底删除 `antivirus-whitelist-dialog.tsx`、移除 [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) 弹窗状态与 13 语种 `layout.json` 中 15 个废弃弹窗键（仅保留 `menuLabel`）。
- **状态**：代码已修正，待用户确认

### BUG-397 闪电图标 ⚡ 与 F4 批量测速误触发节点切换及切断存量长连接

- **对应模块**：[node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts)、[node.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.test.ts)、[proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx)、[proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx)、[proxy-render.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx)、[proxy.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/proxy.rs)
- **问题描述**：节点列表右上角闪电图标 `⚡`（`BoltOutlined`）与快捷键 `F4` 此前和过滤框自动触发共用无参 `handleCheckAll`（硬编码 `select=true, clean_stale_connections=true`），导致用户仅仅想顺手看一眼当前列表各节点的最新毫秒延迟时，若恰好存在别的更低延迟节点，就会被动更换选中节点并依据 v3.5.3 前台切点规则切断正在运行的长连接（如下载或 AI 会话）。
- **修复方案**：
  1. 在 [node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts) 新增共享意图裁决函数 `resolveBatchCheckIntent('bolt' | 'f4' | 'filter' | 'f1')`（补齐 [node.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.test.ts) 单测），明确 `bolt` 与 `f4` 返回 `{ select: false, cleanStale: false }`，`filter` 与 `f1` 保持 `{ select: true, cleanStale: true }`；
  2. 更新 [proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx)、[proxy-render.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx) 与 [proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx)：点击闪电图标或按 `F4` 仅刷新当前可见列表延迟、不切换节点亦不断开任何存量连接；过滤框 700ms 防抖 / Enter 与 `F1` 全量优选继续自动切换并精准清理旧代理连接；并在 `handleCheckAll` 新增 `activeTestSelectRef` / `pendingRecheckSelectRef` 竞态追踪，确保「纯测速在途期间输入过滤词」可平滑排队升级为 `{ select: true }`。
- **状态**：代码已修正，待用户确认

### BUG-396 前台切点未清理非目标旧出口粘滞 HTTP/2 长连接，及路径控制详情缺显实际出口节点标注

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[proxy.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/proxy.rs)、[path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/path-control.ts)、[mihomo-api.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/mihomo-api.ts)、[cmds.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts)、[use-proxy-selection.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-proxy-selection.ts)、[active-node-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx)、[proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx)、[connection-detail.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-detail.tsx)、`src/locales/*/connections.json`
- **问题描述**：① 实机重装 `v3.5.2` 开启 TUN 后先走香港节点为 Antigravity 建立了 HTTP/2 长连接，随后用户输入 `-香港 -台湾` 并将节点切到日本/新加坡，但因 `useProxySelection` 受默认关闭的 `auto_close_connection=false` 阻断，且顶栏轮换、过滤框 / `F4` / `F1` 均不清旧代理连接，导致旧香港 HTTP/2 隧道持续复用报错 `400/403` 数分钟（而新开连接走新节点的 YouTube 正常）；② 宽窗「路径控制」表左键点击条目的详情弹窗中缺少直观高亮的当前实际出口「代理节点」及是否属于「旧节点连接」标识，用户难以一眼排查后台切点后的遗留多路连接。
- **修复方案**：
  1. **前后台严格二分（后台保活不变，前台切点立即清理非新目标的旧 `PROXY` 连接）**：保持后台 15 秒自愈与冷启动零断流热切换；当用户经 4 个前台入口（列表点击、顶栏胶囊轮换、过滤框/`F4`/`F1` 选点、异常胶囊在线重测）主动切节点时，经同源纯函数 `should_close_stale_proxy_connection` / `shouldCloseStaleProxyConnection` 精准关闭 `chains` 含 `PROXY` 且不含新目标节点的旧代理连接，彻底消除多跳残留粘滞且绝不触碰 `DIRECT` 与同节点连接；
  2. **详情弹窗直显出口节点与旧节点提醒**：[connection-detail.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-detail.tsx) 在 `主机` 正下方追加 `代理节点` 行（`resolveConnectionNodeDisplay`，13 语种 `proxyNode` / `staleNodeTag`），若活跃代理连接仍驻留在切换前的旧节点上，则以警示色标出 `（旧节点连接）`，并补齐 Rust 与 Vitest 回归单测。
- **状态**：代码已修正，待用户确认

### BUG-394 极端高负载下退出轻量模式重建窗口引发主线程与 Tokio 工作池连环死锁导致托盘/单实例与后台自愈假死

- **对应模块**：[async_handler.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/process/async_handler.rs)、[window.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/window.rs)、[window_manager.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/window_manager.rs)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[server.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/server.rs)、[notification.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/notification.rs)、[tray/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/tray/mod.rs)、[mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/mod.rs)、[Cargo.toml](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/Cargo.toml)
- **问题描述**：实机连续 20 分钟 `80%～100%` CPU 满载压测期间（`logs/2026-11-17_15-23-14.log` `14:55:59.539`），触发退出轻量模式重建主窗口时发生两阶段连环假死：① `build_new_window` 投递建窗至 Win32 主线程后在 `100%` CPU 饥饿下 WebView2 冷启动 `wait_with_pump` 卡死导致主线程无响应、托盘左/右键双双失灵；② Tauri `window.window_handle()`、`window.is_minimized()/is_visible()/is_focused()` 与 `available_monitors()` 等跨线程 Getter 在 4 核机器共有仅 4 条 Tokio 工作线程上执行无超时同步阻塞 `mpsc::rx.recv()` 死等卡死的主线程，工作池耗尽后将共用同一 Tokio 运行时的 `embed_server`（`127.0.0.1:33335`）与 `monitor.rs` 后台断流自愈守护线程全部拖死（进程仍在、托盘无反应、二次启动报 `33335` 请求超时、后台 `14:57:00.590` 后停止自愈推进）。
- **修复方案**：
  1. 在 [async_handler.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/process/async_handler.rs) 新增物理隔离的 2 线程专属守护 Tokio 运行时 `DAEMON_RUNTIME`（`spawn_daemon` / `spawn_daemon_supervised`），将 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 后台断流自愈与 [server.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/server.rs) 单实例/PAC HTTP 服务迁入守护运行时，彻底免疫 Tauri UI/命令线程池阻塞；
  2. 在 [window.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/window.rs) 引入原子窗口句柄追踪 `MAIN_WINDOW_HWND`（`WM_NCDESTROY` 及 `destroy_main_window` 自动清零），将 `strip_caption_style_hwnd`、`force_set_window_outer_size_hwnd`、`setup_wm_sizing_hook_hwnd` 全部收拢至主线程闭包内就地执行并配合 `12s` 超时失败销毁回滚；
  3. 在 [window_manager.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/window_manager.rs) 将 `get_main_window_with_state`、`is_main_window_visible`/`focused`/`minimized` 与 `is_position_on_screen` 全面改造为零跨线程阻塞的原生 Win32 直查（`IsWindow` / `IsIconic` / `IsWindowVisible` / `GetForegroundWindow` + [tray/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/tray/mod.rs) 原子焦点缓存、`EnumDisplayMonitors` + `GetMonitorInfoW`）；
  4. 在 [notification.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/notification.rs) 叠加 `is_main_window_ready()` 守卫阻断建窗完成前抢跑事件，并优化 `exit_lightweight_mode` 失败回退避免重复建窗。
- **状态**：代码已修正，待用户确认

### BUG-395 新手引导瘦身、帮助菜单「轻量模式防误杀指南」重构与「启动时最小化」全场景静默进托盘防露窗机制

- **对应模块**：[lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs)、[mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/mod.rs)、[server.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/server.rs)、[tray/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/tray/mod.rs)、[lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs)、[first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx)、[antivirus-whitelist-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/antivirus-whitelist-dialog.tsx)、[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、`src/locales/*/layout.json`、[antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md)
- **问题描述**：① 经实证 360 普通「信任区」无法阻止其性能引擎以 `DoOptmGroupResidual`（监控“开窗→关窗残留”）强杀轻量模式进程，原新手引导 Step 2 与帮助弹窗内「加入信任区」指引会误导用户且平添新手流程冗余；② 旧版仅在携带 `--silent` 命令行参数时才触发 `enable_silent_start`，导致开启「静默启动」后用户手动双击图标启动（或被误杀后重开、轻量态再点图标）依然强行弹窗露面，随后关闭窗口又会重新触发 360 关窗残留记录。
- **修复方案**：
  1. 新增 `lightweight::should_enter_silent_tray_on_launch()` 与纯函数 `has_real_profile_items()`：凡已开启 `enable_silent_start` 且存在至少一条真实订阅/本地配置，无论开机自启（`--silent`）还是手动双击图标启动或二次点击单实例唤醒（`server.rs::/commands/visible`），一律不创建/不弹出主窗口而直接维持轻量模式驻留托盘（仅左键点击右下角托盘图标打开界面），同时通过原生 `Shell_NotifyIconW(NIF_INFO)` 气泡（`Tray::notify_silent_running`，2s 防抖、不生成 GUI 窗口）给出清晰就绪提示（单实例客户端请求超时同步放宽至 `1500ms`）；
  2. 从 [first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx) Step 2 彻底剥离安全软件说明卡片及 [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) 透传回调；
  3. 将 `?` 帮助菜单对应条目升级为「🛡️ 轻量模式防误杀指南」，同步重构 [antivirus-whitelist-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/antivirus-whitelist-dialog.tsx)、全部 13 语种 `layout.json` 与 [antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md)：清晰展示「省心静默防杀（支持弹窗内一键开启启动时最小化 + 日常避免开窗露面叮嘱，误杀后重开即静默恢复）」与「Windows 安全模式修改 `whiteinternal.ini` 根治方案（附一键复制路径与追加条目按钮）」。
- **状态**：代码已修正，待用户确认

### BUG-392 过滤框中文输入法中间态抢跑与测速在途重触发丢弃导致排除词（如 -香港）偶发漏切

- **对应模块**：[node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts)、[base-search-box.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-search-box.tsx)、[proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx)、[use-head-state.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-head-state.ts)、[proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：用户实机日志（`latest.log` `04:41:52` / `04:42:28`）复现出一条三环叠加竞态：① `BaseSearchBox` 未拦截中文输入法 IME 组词中间态（`onCompositionStart`/`onCompositionEnd`）且 `ProxyHead` 仅判 `!trimmed`，导致键入 `"-"` 或拼音中间串 `"-x"` 停顿 ≥700ms 即抢跑触发全量 41 节点 5 秒实时拨测；② 当该 5 秒探测在途期间用户正式提交 `"-香港"` 时，`ProxyHead` 将 `lastTriggeredFilterRef` 更新为 `"-香港"` 后调用 `ProxyGroups::handleCheckAll`，但 `handleCheckAll` 因 `testingGroupsRef.current[groupName]` 为 `true`（或后端返回 `AUTO_SELECT_BUSY`）直接 `return` 静默丢弃新子集请求且无尾部重测队列；③ 过期 41 节点探测完成后，`monitor.rs::trigger_backend_auto_select_inner` 先执行 `select_node_for_group("PROXY", "香港3 (043311)")` 再重读 `filter_text`，虽跳过回写 `profile.selected` 并记 `WARN`，却已把运行时出口切到被排除的香港节点。30s 缓存期内因旧探测 40ms 速通而无法稳定复现，冷探测 5s 窗口则必现。
- **修复方案**：
  1. 在 [node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts) 新增纯函数 `hasEffectiveFilterTerm` 与 `isIncompleteFilterInput` 并补全 [node.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.test.ts) 单测，识别孤立算子与末尾悬空前缀（如 `"-"`, `"!"`, `"日本 -"`, `"日本|"`）；
  2. 在 [base-search-box.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-search-box.tsx) 引入 IME 组词本地缓冲（`isComposingRef` + `composingText`），组词期间保持输入框平滑回显而不向外派发拼音中间态；在 [proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx) 拦截未完成过滤串抢跑触发；
  3. 在 [use-head-state.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-head-state.ts) 导出 `flushHeadStatePersist` 并在 [proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx) 的 `handleCheckAll`/`handleCheckFull` 选点前冲刷落盘；为 `handleCheckAll` 增加「过滤词变更尾部重测队列」（`pendingRecheckGroupsRef`）与有限次 `AUTO_SELECT_BUSY` 自动续试；
  4. 在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 抽出 `select_fastest_candidate` 并在 `select_node_for_group` **切换前**实时重读 `latest_filter_text` 过滤淘汰越界候选（若本批全数越界则安全跳过本次过期切换），并补齐 Rust 回归单测。
- **状态**：代码已修正，待用户确认

### BUG-393 新手引导与白名单说明补齐信任区具体目录/进程路径及 10 语种陈旧看门狗文案清理

- **对应模块**：[first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx)、[antivirus-whitelist-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/antivirus-whitelist-dialog.tsx)、`src/locales/*/layout.json`、[antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md)
- **问题描述**：① 新手引导第二步「使用 360 等安全软件？」提示卡片（`layout.components.antivirus.brief`）仅写「加入信任区」，未告知用户应把哪一个安装目录或可执行文件加入信任区；② 全文引导文档与 `antivirus.how` 此前仅列 `clash-mini.exe`，未并列说明整目录 `C:\Program Files\Clash Mini\` 及代理内核 `mini-mihomo.exe`；③ `ar`/`de`/`es`/`fa`/`id`/`jp`/`ko`/`ru`/`tr`/`tt` 共 10 个语种的 `antivirus.*` 仍残留 v3.3.3 已废除的「内置进程看门狗自动拉起」过时表述。
- **修复方案**：
  1. 在 13 个语种的 `antivirus.brief` 与 `antivirus.how` 及 [antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md) 明确写明加入安装目录（默认 `C:\Program Files\Clash Mini\`）或其中的 `clash-mini.exe` 与 `mini-mihomo.exe`；
  2. 为 [first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx) 与 [antivirus-whitelist-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/antivirus-whitelist-dialog.tsx) 补充 `overflowWrap: 'anywhere'` 防止 `271px` 窄窗下长路径溢出；
  3. 将 10 个滞后语种的 `why`/`benefit`/`consequence` 同步至现行「轻量模式关窗无主窗 + `mini-mihomo.exe` 驻留」真实架构描述。
- **状态**：代码已修正，待用户确认

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

历史已确认关闭条目（`BUG-057` ~ `BUG-391`）完整追溯映射详见 `clash_mini_agreements.md`「附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表」。

| 缺陷编号 | 对应版本基线 | 状态 |
| :--- | :---: | :---: |
| **BUG-391** | v3.5.0 | 代码已修正，已确认 |
| **BUG-390** | v3.5.0 | 代码已修正，已确认 |
| **BUG-389** | v3.5.0 | 代码已修正，已确认 |
| **BUG-388** | v3.5.0 | 代码已修正，已确认 |
| **BUG-387** | v3.4.2 | 代码已修正，已确认 |
| **BUG-386** | v3.4.2 | 代码已修正，已确认 |
| **BUG-385** | v3.4.2 | 代码已修正，已确认 |
| **BUG-384** | v3.4.2 | 代码已修正，已确认 |
| **BUG-383** | v3.4.1 | 代码已修正，已确认 |
| **BUG-382** | v3.4.0 | 代码已修正，已确认 |
| **BUG-381** | v3.4.0 | 代码已修正，已确认 |
| **BUG-380** | v3.4.0 | 代码已修正，已确认 |
| **BUG-379** | v3.4.0 | 代码已修正，已确认 |
| **BUG-378** | v3.3.4 | 文档已修正（废弃草案已物理移除），已确认 |
| **BUG-377** | v3.3.4 | 代码已修正，已确认 |
| **BUG-376** | v3.3.4 | 代码已修正，已确认 |
| **BUG-375** | v3.3.4 | 代码已修正，已确认 |
| **BUG-374** | v3.3.4 | 代码已修正，已确认 |
| **BUG-373** | v3.3.4 | 代码已修正，已确认 |
| **BUG-372** | v3.3.4 | 代码已修正，已确认 |
| **BUG-371** | v3.3.3 | 代码已修正，已确认 |
| **BUG-370** | v3.3.3 | 代码已修正，已确认 |
| **BUG-369** | v3.3.3 | 代码已修正，已确认 |
| **BUG-368** | v3.3.2 | ②④现行生效，①已由 BUG-377 收口，③清连接已停用移除，已确认 |
| **BUG-367** | v3.3.0 | 代码已修正，已确认 |
| **BUG-366** | v3.3.0 | 代码已修正，已确认 |
| **BUG-365** | v3.2.9 | 看门狗与 Session-0 拉起已于 v3.3.3 废止移除（仅留 WER/CrashDumps 取证），已确认 |
| **BUG-364** | v3.2.8 | 代码已修正，已确认 |
| **BUG-363** | v3.2.7 | 看门狗与哨兵窗已于 v3.3.3 废止移除（仅留安装清理与白名单指引），已确认 |
| **BUG-362** | v3.2.6 | 代码已修正，已确认 |
| **BUG-361** | v3.2.6 | 代码已修正，已确认 |
| **BUG-360** | v3.2.6 | 代码已修正，已确认 |
| **BUG-359** | v3.2.6 | 代码已修正，已确认 |
| **BUG-358** | v3.2.6 | 代码已修正，已确认 |
| **BUG-357** | v3.2.6 | 代码已修正，已确认 |
| **BUG-356** | v3.2.6 | 代码已修正，已确认 |
| **BUG-355** | v3.2.6 | 代码已修正，已确认 |
| **BUG-354** | v3.2.6 | 代码已修正，已确认 |
| **BUG-353** | v3.2.6 | 代码已修正，已确认 |
| **BUG-352** | v3.2.6 | 代码已修正，已确认（与 BUG-328 同源归档） |
| **BUG-351** | v3.2.6 | 代码已修正，已确认（与 BUG-329/BUG-330 同源归档） |
| **BUG-350** | v3.2.6 | 代码已修正，已确认（与 BUG-332 同源归档） |
| **BUG-349** | v3.2.6 | 代码已修正，已确认（与 BUG-322 同源归档） |
| **BUG-348** | v3.2.6 | 代码已修正，已确认（与 BUG-320 同源归档） |
| **BUG-347** | v3.2.6 | 代码已修正，已确认 |
| **BUG-346** | v3.2.6 | 代码已修正，已确认（与 BUG-319 同源归档） |
| **BUG-345** | v3.2.6 | 代码已修正，已确认（与 BUG-317 同源归档） |
| **BUG-344** | v3.2.6 | 直测已回退并收口加固，已确认 |
| **BUG-343** | v3.2.6 | 代码已修正，已确认 |
| **BUG-342** | v3.2.6 | 代码已修正，已确认 |
| **BUG-341** | v3.2.6 | 代码已修正，已确认 |
| **BUG-340** | v3.2.6 | 代码已修正，已确认 |
| **BUG-339** | v3.2.6 | 代码已修正，已确认 |
| **BUG-338** | v3.2.6 | 代码已修正，已确认 |
| **BUG-337** | v3.2.6 | 代码已修正，已确认 |
| **BUG-336** | v3.2.6 | 代码已修正，已确认 |
| **BUG-335** | v3.2.6 | 代码已修正，已确认 |
| **BUG-334** | v3.2.6 | 代码已修正，已确认 |
| **BUG-333** | v3.2.6 | copy_clash_env 零调用死命令已移除（10801 默认端口现行生效），已确认 |
| **BUG-332** | v3.2.6 | 代码已修正，已确认 |
| **BUG-331** | v3.2.6 | 代码已修正，已确认 |
| **BUG-330** | v3.2.6 | 代码已修正，已确认 |
| **BUG-329** | v3.2.6 | 代码已修正，已确认 |
| **BUG-328** | v3.2.6 | 代码已修正，已确认 |
| **BUG-327** | v3.2.6 | 代码已修正，已确认 |
| **BUG-326** | v3.2.6 | 代码已修正，已确认 |
| **BUG-325** | v3.2.6 | 代码已修正，已确认 |
| **BUG-324** | v3.2.6 | 代码已修正，已确认 |
| **BUG-323** | v3.2.6 | 代码已修正，已确认 |
| **BUG-322** | v3.2.6 | 代码已修正，已确认 |
| **BUG-321** | v3.2.6 | 代码已修正，已确认 |
| **BUG-320** | v3.2.6 | 代码已修正，已确认 |
| **BUG-319** | v3.2.6 | 代码已修正，已确认 |
| **BUG-318** | v3.2.6 | 代码已修正，已确认 |
| **BUG-317** | v3.2.6 | 代码已修正，已确认 |
| **BUG-316** | v3.2.5 | 代码已修正，已确认 |
| **BUG-315** | v3.2.4 | 代码已修正，已确认 |
| **BUG-314** | v3.2.3 | 代码已修正，已确认 |
| **BUG-312** | v3.2.2 | 代码已修正，已确认 |
| **BUG-311** | v3.2.1 | 代码已修正，已确认 |
