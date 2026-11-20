# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：历史已验收条目详见 [Changelog.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Changelog.md) 与 Git 提交历史。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-412 间歇性黑洞节点在二次确认借备用 URL 逃生后长期锁死、且故障切离未清理原死节点滞留代理连接

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：`2026-11-20 13:05 ~ 13:26` 实证日志发现：当活跃节点（如 `日本5 (031306)`）陷入「短时黑洞丢包 / UDP QUIC 勉强维持但 TCP HTTP/2 长流持续卡死」的间歇性劣化（Flapping / Brownout）状态时，存在三重滞留机制：① [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)::`evaluate_failover` 仅对 `consecutive_fails == 0` 启用单目标免打备用 URL（`allow_single_target_skip`），第 2 次确认判死时即使主测速地址已有 `35+` 个真实节点响应（证实测速地址未失真），仍强行再打备用 `gstatic` 给故障节点第 3 次压线逃生机会；② 只要间歇性黑洞节点在 3 秒复检中挤回一次 `204` 空包，`consecutive_fails` 即清零，20 分钟内触发 11 次 `Stalled`（有上行零下行）与 5 次 `ShouldFailover` 仍无法自动切走；③ 后台确认故障切往新节点后不清理仍粘滞在原 `dead_node` 上的半开 TCP 长连接，导致长流应用持续卡在旧连接重传中。
- **修复方案**：
  1. 在 `evaluate_failover` 中对单活跃节点（`std::slice::from_ref(&active_node)`）全阶段统一启用 `allow_single_target_skip = true`：当同组已有 `>= 3` 个真实节点通过主测速地址响应时，第 2 次确认超时的节点不再借备用 URL 逃生（`< 3` 个时仍完整保留备用 URL 全局防失真复核）；
  2. 新增 `BROWNOUT_WINDOW_SECS (600s)` / `BROWNOUT_CONFIRM_FAILS (2)` 与 `record_brownout_episode`，在保留 `NoAction` 恢复正常单次抖动防误切的同时，于 `decide_self_heal` 中对同一活跃节点在 600s 滑动窗口内发生 `>= 2` 次独立 `ShouldFailover` 故障周期直接放行自愈切点；
  3. 新增 `should_close_dead_node_proxy_connection` 与 `close_dead_node_proxy_connections`，在 `self_heal_with_accounting` 确认切离 `dead_node != new_node` 时仅精准断开滞留在该故障旧节点上的 `PROXY` 存量代理连接（零触碰 `DIRECT` / `REJECT` 与其他节点），促使 HTTP/2 / gRPC 等长连接立即重建于新健康出口，并补齐 Rust 边界单测。
- **状态**：代码已修正，待用户确认

### BUG-411 内核进程意外消失（命名管道 `os error 2`）时自愈状态机空等 3 轮 API 轮询与 30 秒就绪超时致断流近 4 分钟

- **对应模块**：[ipc.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/ipc.rs)、[mihomo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/mihomo.rs)、[lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/lib.rs)、[manager/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/mod.rs)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：`2026-11-20 12:27:40 ~ 12:31:27` 实证日志复现：当后台 `mini-mihomo.exe` 进程消失导致 `\\.\pipe\mini-mihomo` 报 `os error 2`（管道文件不存在）时，[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 未区分「管道瞬时繁忙」与「管道服务端本身已不存在」，仍机械执行 `3 轮 × (3 × 15s API 异常轮询 + 30s wait_for_clash_ready 空等)` 累计约 **216 秒**后才调用 `restart_core()`（而重启本身仅耗时 0.86 秒即恢复）。此外，`FailoverVerdict::NoAction` 稳态恢复时未清零 `auto_select_fail_count` / `api_dead_heal_fail_count`。
- **修复方案**：
  1. 在 [ipc.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/ipc.rs) 新增零句柄侵占探测 `is_named_pipe_server_missing` / `is_pipe_missing_os_error`（使用 `WaitNamedPipeW(..., 1)` 仅对 `ERROR_FILE_NOT_FOUND` / `ERROR_PATH_NOT_FOUND` / `ERROR_BAD_PATHNAME` 判为缺管，严格排除繁忙态 `ERROR_SEM_TIMEOUT` / `ERROR_PIPE_BUSY`），并在 [mihomo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/mihomo.rs) 暴露 `is_local_socket_missing()`；
  2. 在 [manager/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/mod.rs) 新增 `should_expect_core_pipe_alive()`（排除退出态、`NotRunning`、配置更新及生命周期锁占期间）；
  3. 在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 新增 `try_fast_restart_missing_pipe`（经 `500ms` 二次复核与 `25s` 冷却门控后立即调用 `execute_core_restart_with_accounting` 重启内核），同时在 `ApiChannelDown` 入口通过 `precheck_api_channel_down` 复核仍不可达时直接跳过 30 秒 `wait_for_clash_ready()` 空等，并在 `NoAction` 恢复时清零双失败计数，补齐双 Crate 单元测试。
- **状态**：代码已修正，待用户确认

### BUG-410 切换为宽窗口后底部流量曲线图消失及上游遗留大盘图表/Worker 管线冗余

- **对应模块**：[mini-traffic-panel.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/mini-traffic-panel.tsx)、[traffic-graph.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/traffic-graph.tsx)、[parse-traffic.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/parse-traffic.ts)、[use-traffic-data.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-data.ts)、[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、[parse-traffic.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/parse-traffic.test.ts)
- **问题描述**：窗口跨过 `285px` 断点拉宽后，[mini-traffic-panel.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/mini-traffic-panel.tsx) 会卸载窄窗曲线 [TrafficGraph](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/traffic-graph.tsx) 并切换挂载上游遗留的大盘组件 `EnhancedCanvasTrafficGraph`，引发四重叠加缺陷致宽窗流量曲线图消失：① [use-traffic-data.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-data.ts) 与原 `use-traffic-monitor.ts` 各自异步订阅 `useVisibility()`，建连时 `setupHandlers` 捕获到 `isActive === false` 的陈旧 `appendData` 闭包后丢弃所有采样点，导致 `displayData` 恒空并执行 `clearCanvas`；② Worker 采样快照节流间隔为 `3000ms`，而画布 `STALE_DATA_THRESHOLD` 为 `2500ms` 且开启了 `pause_render_traffic_stats_on_blur`，造成跨时必触发 `shouldSkipGraphDraw()` 冻结首帧重绘；③ 宽窗底栏绘制区仅约 `80px`–`99px` 高，大盘组件上下固定留白吃掉 `63px` 且 10 分钟线性纵轴极易被前期高延迟测速峰值压平；④ [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) 上方面板使用 `flex: '80 0 calc(0% + 30px)'` 导致矮窗拉宽时底栏缺少收缩退让能力而被挤出视口。
- **修复方案**：
  1. 在 [mini-traffic-panel.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/mini-traffic-panel.tsx) 与 [use-traffic-data.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-data.ts) 中全面统一为宽窄共用的单一实时滚动曲线 [TrafficGraph](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/traffic-graph.tsx)（跨 `285px` 断点零卸载不断线，分段对数纵轴对日常小流量保持灵敏），并移除重复的 `useVisibility()` 订阅；
  2. 彻底清理 `enhanced-canvas-traffic-graph.tsx`、`use-traffic-monitor.ts`、`traffic.worker.ts`、`traffic-sampler.ts` 及对应 Worker 消息类型与常量 `TRAFFIC_WORKER_DESTROY_DELAY_MS`；
  3. 在 [parse-traffic.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/parse-traffic.ts) 扩展 `63` 点环形缓冲（`TRAFFIC_GRAPH_BUFFER_LEN`）与宽度映射 `resolveTrafficVisiblePoints` / `sliceTrafficVisibleBuffer`；[traffic-graph.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/traffic-graph.tsx) 基于 `isWindowResizing() ? window.innerWidth > 285 : !isMinimalWidth` 同时适配手动拖拽边框与点击最大化切换：窄窗呈极简 `30s` 双参考线，宽窗自适应展开为 `30s → 60s` 时间跨度并展示 `1M`/`10K`/`100B` 对数刻度网格、渐变面积填充与时间角标；同时改为非中断式同帧即时重绘（`redrawImmediate`）、纵轴钳制 `[2, height - 2]` 防描边裁切并修复 [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) 宽窗 flex 伸缩布局，在 [parse-traffic.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/parse-traffic.test.ts) 补齐窄/拖宽/最大化回归单测。
- **状态**：代码已修正，待用户确认

### BUG-409 全局 Merge 配置二次合并冗余及快捷路径控制规则保存后二次重复热重载

- **对应模块**：[enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)、[save_profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/save_profile.rs)、[quick-routing.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/quick-routing.ts)、[quick-routing.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/quick-routing.test.ts)
- **问题描述**：举一反三全栈排查发现两处 `Merge.yaml` 冗余执行问题：① [enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs) 已通过 `global_merge` 无条件加载全局 `"Merge"`，但当激活订阅未显式配置专属 `option.merge`（`None`）时，`merge_uid` 默认回退为 `"Merge"` 再次加载并在 `process_profile_items` 二次执行 `use_merge`；② [save_profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/save_profile.rs)::`profile_affects_runtime` 在激活订阅存在空串或非 `"Merge"` 专属项时漏判全局 `"Merge"` 的运行时影响，而前端 [quick-routing.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/quick-routing.ts) 在 `saveProfileFile('Merge', ...)` 已完成 `update_config_forced` 重载后仍无条件再调一次 `enhanceProfiles()`，触发双倍 `mihomo -t` 子进程校验与双倍命名管道热重载。
- **修复方案**：
  1. 在 [enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs) 中仅当 `current_item.current_merge()` 为非空且不等于 `"Merge"` 时才加载配置级 `merge_item`，消除全局 `"Merge"` 二次重复合并；
  2. 在 [save_profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/save_profile.rs)::`profile_affects_runtime` 中对有当前激活配置时的 `"Merge"` 恒判 `true` 并过滤空白关联项，补齐单测；
  3. 在 [quick-routing.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/quick-routing.ts) 中根据 `saveProfileFile` 返回值仅在未生效时兜底调用 `enhanceProfiles()`，并将路由导入长操作静默区间前移包裹整个保存流程。
- **状态**：代码已修正，待用户确认

### BUG-408 开启 TUN 安装系统服务时的三连内核重启风暴、Session 0 残留进程抢占及安装器强杀服务异常日志

- **对应模块**：[service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs)、[use-service-installer.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-service-installer.ts)、[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、[clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs)、[installer.nsi](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi)
- **问题描述**：`2026-11-19 22:19:22` 与 `22:21:46` 两次日志实证确认：① 普通用户点击开启 TUN 安装系统服务时，刚拉起的 `Clash Verge Service` 会按历史残留状态自动拉起 `LocalSystem` 权限的 `mini-mihomo.exe`（`PID: 21244` / `1500`），与仍在运行的用户态 Sidecar 争抢 `\\.\pipe\mini-mihomo` 与 `10801` 端口，且用户态 `kill_all_mini_cores` 因权限不足无法清理；② 前端 `installServiceAndRestartCore` 在 `enable_tun_mode` 仍为 `false` 时先白做一次 Sidecar 重启，480ms 后 `patchVerge({ enable_tun_mode: true })` 又触发第二次重启切入 Service，1 秒内连续三次停启内核；③ NSIS 安装脚本 [installer.nsi](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi) 在 `SimpleSC::StopService` / `RemoveVergeService` 之前先对 `clash-verge-service.exe` 执行强杀，导致 Windows 事件查看器记录 Event ID `7031` 服务意外终止并触发 SCM 5 秒自动重拉竞态。
- **修复方案**：
  1. 在 [service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs)::`apply_service_status` 完成服务安装/启动/重装 IPC 就绪后，若当前 `CoreManager` 并非 `RunningMode::Service`，立即调用 `stop_clash()` 清理服务自启拉起的 Session 0 孤儿内核；
  2. 将 [use-service-installer.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-service-installer.ts) 收敛为 `installServiceAndSync`（仅安装服务并刷新系统状态，不再提前重启 Sidecar），交由紧随其后的 `patchVerge` 一次性原子完成 `Sidecar -> Service` 切换，并同步清理零消费的 `restart_core` IPC 与冗余词条；
  3. 在 [installer.nsi](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi) `CheckAllVergeProcesses` 中优先通过 `SimpleSC::StopService` 优雅停止服务，并在卸载段将 `RemoveVergeService` 置于进程强杀兜底之前。
- **状态**：代码已修正，待用户确认

### BUG-407 `ensure_proxy_group_selection` 持排他门锁同步长阻塞及重载/重启瞬间前后端命名管道并发洪峰

- **对应模块**：[lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)、[manager/config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/config.rs)、[core_updater.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/core_updater.rs)、[handle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/handle.rs)、[use-mihomo-ws-subscription.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-mihomo-ws-subscription.ts)
- **问题描述**：① `apply_config`、`restart_core` 与 `core_updater` 在持有 `config_update_in_progress` / `lifecycle_lock` / `VERGE_PATCH_LOCK` / `CLASH_PATCH_LOCK` 期间同步 `await` 了 `ensure_proxy_group_selection`（内置 `30s + 20s + 90s` 预算与 `50ms` 高频轮询），一旦管道阻塞即导致前端 IPC 长时间挂起，且 `core_updater` 存在 `start_core` 与显式调用并发双跑；② 重载/重启完成瞬间，`Handle::refresh_clash()` 同时派发 `RefreshClash` 与 `RefreshProxies`（触发前端 `200ms` + `250ms` 双次 `calcuProxies`），多处外层调用者又重复补发 `Handle::refresh_clash()`，叠加前端 `250ms` 同毫秒并发重连 4 路 WebSocket 以及 `restart_core` 立即唤醒 `MONITOR_WAKEUP_NOTIFY` 全组 `delay_group` 拨测，对刚就绪的命名管道形成瞬时并发冲击。
- **修复方案**：
  1. 在 [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs) 引入基于代际号 `SELECTION_ENSURE_EPOCH` 自取消的异步 `spawn_ensure_proxy_group_selection`（`120ms` 稳态缓冲、`250ms` 温和轮询、`12s`/`10s`/`15s` 收紧上限，快照合规节点变更时前置写回持久化），全面移出 `apply_config`、`restart_core` 与 `core_updater` 的主锁同步等待链，并移除 `restart_core` 末尾的 `MONITOR_WAKEUP_NOTIFY` 抢跑拨测；
  2. 在 [handle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/handle.rs) 中令 `Handle::refresh_clash()` 仅发送 `RefreshClash`（前端已包含代理缓存失效），并清理 `cmd/profile.rs`、`cmd/save_profile.rs`、`feat/config.rs`、`feat/profile.rs` 中紧随 `apply_config`/`restart_core` 之后的重复 `refresh_clash()` 调用；
  3. 在 [use-mihomo-ws-subscription.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-mihomo-ws-subscription.ts) 将 `REFRESH_CLASH_DEBOUNCE_MS` 错峰至 `400ms`（避开 `250ms` REST 状态拉取），并对多路常驻 WebSocket 施加 `45ms` 步进式错峰重连。
- **状态**：代码已修正，待用户确认

### BUG-406 命名管道连接池跨双 Tokio 运行时 IOCP 错位复用、超时未中止驱动任务泄漏句柄及停核/断连残留引发管道假死

- **对应模块**：[ipc.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/ipc.rs)、[mihomo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/mihomo.rs)、[lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)
- **问题描述**：`2026-11-19 22:19:42` 与 `22:22:16` 两次实证复现 `IPC send timeout` 与 `Named pipe busy timeout exceeded`（`ERROR_PIPE_BUSY`）根因在于：① [ipc.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/ipc.rs) 中后台守护 `DAEMON_RUNTIME` 打开的 `NamedPipeClient` 绑定在守护运行时 IOCP 上，却通过 `tauri::async_runtime::spawn` 将 `http1::Connection` 驱动派发至主运行时并推入全局单一队列交叉复用；② `create_connection` 丢弃了 `conn_driver` 的 `JoinHandle`，当外层请求超时取消或清理连接池时，`conn_driver` 未被 `abort()`，导致孤儿任务长期占用客户端与服务端命名管道句柄；③ [mihomo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/mihomo.rs) 中 `reload_config` 发送非幂等 `PUT /configs` 前未清理半死空闲管道、`disconnect` 在本地命名管道及 `forceTimeout=0` 下仍强制上限 `500ms` 写关闭帧，且 [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)::`stop_core_inner` 停核前未调用 `clear_all_ws_connections()` 释放 4 路常驻 WebSocket 管道句柄。
- **修复方案**：
  1. 在 [ipc.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/ipc.rs) 中将连接池升级为按 `tokio::runtime::Id` 物理分桶隔离（`HashMap<tokio::runtime::Id, VecDeque<IpcConnection>>`），`conn_driver` 与 `wait_named_pipe_blocking` 统一改用当前运行时的 `tokio::spawn` / `tokio::task::spawn_blocking`；在 `IpcConnection` 中持有 `driver_handle` 并实现 `Drop` 自动 `abort()`，确保超时、报错、空闲淘汰及清池均在 `0ms` 内终止驱动并释放 Win32 管道句柄；
  2. 在 [mihomo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/mihomo.rs) 中对 `LocalSocket` 模式的 `reload_config` 执行前后双向清池，对 `LocalSocket` 或 `force_timeout == Some(0)` 的 `disconnect` 跳过阻塞写帧直接释放读写端；
  3. 在 [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)::`stop_core_inner` 中统一调用 `clear_all_ws_connections()` 同时释放全部 WebSocket 与连接池管道句柄，并补齐双运行时隔离与 `Drop` 中止单测。
- **状态**：代码已修正，待用户确认

### BUG-405 内核启动/重启/重载后 select 组无静态 now 回落首节点导致排除地区（如香港/台湾）被短暂选中并残留路由连接

- **对应模块**：[node.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/node.rs)、[enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)、[clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/clash.rs)、[config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/config.rs)、[lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)、[manager/config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/config.rs)、[core_updater.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/core_updater.rs)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：`2026-11-19 19:32` 日志实证确认：当用户在过滤框排除香港/台湾（`filterText: "-香港 -台湾"`）并处于 `新加坡3 (184629)` 出口期间触发 `restart_core`（或冷启动 / `reload_config` / 内核升级）时，由于 Mihomo 运行时 YAML 的 `select` 组（`PROXY`）不含静态 `now` 字段且默认 `proxies[0]` 仍为订阅首位节点（`香港1 (184629)`），加之 `store-selected: true` 在 `cache.db` 命中旧选择或未命中时按首节点初始化，导致内核刚拉起约 `1.4s` 窗口内默认选中 `香港1`；期间应用重建的长连接立刻绑定至香港节点并滞留。此外 `start_core` 缺少选点对齐、`apply_config` 缺少持久化选点兜底、`restore_proxy_group_now` 未校验 `filterText` 且不清理越界连接、`evaluate_failover` 也未对已越界活跃节点触发迁移。
- **修复方案**：
  1. **生成期 0ms 合规首节点与关闭旧缓存复活**：在 [clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/clash.rs)、[tmpl.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/tmpl.rs) 及 [enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)::`sync_runtime_proxy_group_head` 中将 `profile.store-selected` 统一设为 `false`（以 `profiles.yaml::selected` 与 `proxy_head_state.json` 为唯一真源），并在 `enforce_mini_agreements` 及 [config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/config.rs)::`generate_file(Run)` 落盘前通过 [node.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/node.rs)::`promote_preferred_proxy_first` 将符合过滤条件的持久节点或合规子集首节点提前至 `PROXY.proxies[0]`，使内核启动第 0 毫秒即直接落位合规出口；
  2. **生命周期全路径闭环校验与越界连接清理**：在 [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs) 新增 `resolve_desired_proxy_selection` 与 `ensure_proxy_group_selection` 统一覆盖 `start_core`、`restart_core`、[manager/config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/config.rs)::`apply_config` 与 [core_updater.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/core_updater.rs)，即使快照缺失或目标不在过滤子集内也按子集兜底并同步持久化，收尾调用 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)::`close_excluded_proxy_connections` 清理任何滞留在已排除节点上的代理连接；
  3. **后台守护越界主动迁移**：在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)::`evaluate_failover` 增加 `should_migrate_filtered_out_active`，并在 `NodeMigration` 迁出成功后触发 `close_excluded_proxy_connections`，并补齐双模块单元测试。
- **状态**：代码已修正，待用户确认

### BUG-404 全栈死代码剥离、编译器 dead_code 门禁开启与代码/设计书去历史化精简重构

- **对应模块**：[lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs)、[cmd/](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/mod.rs)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、[profile-coordination.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/utils/profile-coordination.ts)、[proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx)、[clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md)
- **问题描述**：随着多轮迭代演进，工程中累积了四类非必要膨胀，导致维护噪音大、新改动易触及冗余路径：① 部分已退役或无消费者的后端 Tauri 命令与孤立导出（如 `get_runtime_config` / `uninstall_service` / 孤立 Rust 辅助函数与前端未消费导出），且 [lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs) 长期悬挂 `#![allow(dead_code)]` 掩盖了死代码滋生；② [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) 仍保留与后端 `monitor.rs` 重叠的 `visibilitychange` 全组主动拨测器（`triggerWakeupLatencyTest`）以及多处大段复制粘贴样式/导入/切换代码；③ `monitor.rs` API 异常自愈处理分支重复，且各核心模块及 [clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md) 中混杂大量历史事故复盘叙事与过程流水账，掩盖了当前真实生效的不变量与设计契约。
- **修复方案**：
  1. 在 100% 忠实保留全部现有功能、阈值与后台防断流机制前提下，彻底移除零消费 `get_runtime_config` / `uninstall_service` 等死 IPC 与全栈死代码，摘除 [lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs) `#![allow(dead_code)]` 开启全仓静态死代码门禁；
  2. 退役 [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) `triggerWakeupLatencyTest` 重复拨测链（窗口切回前台超过 3s 冷却仅静默刷新状态展示，探测/自愈唯一收口于 `monitor.rs`），收敛批量测速 `-2` 占位与快照回滚共享辅助器及 `handleImportProfile` / `handleTakeoverModeChange` / 3D 按钮样式等重复逻辑；
  3. 收敛 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 重复 API 异常自愈块，并将前后端关键文件及 [clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md) 去历史化重构为简明精确的现态规范文档（历史过程统一归入 `Changelog.md` 与 Git 历史）。
- **状态**：代码已修正，待用户确认

### BUG-403 物理网络 `TCP:53` 探针未复用活跃下行证活且每次并发三路公网握手

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：后台守护循环在在线状态下每隔 `60s`（以及每轮真 HTTPS 验真前）都会通过 `futures::future::select_ok` 同时向 `223.5.5.5:53`、`119.29.29.29:53`、`1.1.1.1:53` 并发发起 3 个直连公网 TCP SYN 握手——即便上一轮刚刚观测到活跃代理连接存在真实下行回包（`Flowing`）可确定物理网卡在线，仍重复向公网 DNS 53 端口高频发包。
- **修复方案**：
  1. 在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 新增 `LAST_FLOWING_TRAFFIC_AT`、`ONLINE_CHECK_FLOWING_SKIP_SECS (60s)` 及纯函数 `has_recent_flowing_traffic` / `is_recent_flowing_traffic_active`（随 `reset_monitor_static_state` 同步复位）；当近 60 秒内观测到 `ActiveTrafficState::Flowing` 时，主循环 `60s` 在线探测与 `run_https_verification_pass` 前置在线检查直接复用证活结论跳过公网握手；
  2. `is_physical_network_online` 改为优先单路直连主 DNS `223.5.5.5:53`（超时上限 `PRIMARY_NET_PROBE_TIMEOUT_MS = 250ms`），仅当主路超时或异常时再并发回退其余两路与域名 DNS 查询，常态直连公网握手减少 67%–100%，并补齐 Rust 边界单测。
- **状态**：代码已修正，待用户确认

### BUG-402 窗口关闭/唤醒前后端重复拨测、切回前台传 `undefined` 绕过缓存与遗留 10 秒二次补测

- **对应模块**：[lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs)、[profile-coordination.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/utils/profile-coordination.ts)、[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、[use-mihomo-ws-subscription.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-mihomo-ws-subscription.test.ts)
- **问题描述**：① `entry_lightweight_mode` 关闭窗口时触发 `MONITOR_WAKEUP_NOTIFY` 将后台计时器提前回拨强行拉起体检；② 切回前台（`visibilitychange -> visible`）的 `triggerWakeupLatencyTest` 与轻量唤醒的 `triggerAutoSelectAndRefresh` 未先检查当前活跃出口是否已有新鲜可用延迟，且前者调用 `triggerAutoSelect(uid, undefined, 0, false)` 导致 `is_frontend_subset` 为 `false` 绕过后端 `<30s` 快照缓存复用；③ [profile-coordination.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/utils/profile-coordination.ts) 每次挂载后还遗留一个 `10s` `fallbackTimerRef` 定时器二次触发全组测速。
- **修复方案**：
  1. [lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs)：`entry_lightweight_mode` 移除多余的 `MONITOR_WAKEUP_NOTIFY.notify_one()`，关窗不再额外触发体检；
  2. [profile-coordination.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/utils/profile-coordination.ts) 与 [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)：新增 `isProxyHistoryFreshAndValid` 与 `hasFreshActiveExitLatency`，轻量唤醒（`skipProbeIfFresh: true`）与窗口可见恢复当活跃出口已持有效新鲜延迟时零拨测秒开；确实需要唤醒测速时传过滤子集 `names` 恢复后端 30s 快照复用；彻底删除 `fallbackTimerRef` 及其 `10s` 补测定时器，并补齐 Vitest 回归单测。
- **状态**：代码已修正，待用户确认

### BUG-401 单节点目标主 URL 整体正常时过度触发全组备用 URL 第二轮复核

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：`probe_metrics_group` 原直接通过 `should_recheck_with_fallback` 判定是否用 `FALLBACK_TEST_URL`（`gstatic.com`）重打整组拨测。对于期望集只有 1 个节点（点击单个坏节点延迟徽章或活跃节点单点初检失败）且同组其它真实节点已在主测速地址大量正常响应的场景，该节点本身大概率为单点故障而非主测速地址失真，却每次连坐触发第二轮全组并发复核，使单节点故障期信号翻倍。
- **修复方案**：
  1. 保留原 `should_recheck_with_fallback` 专用于快照复用/合并门控；新增 `should_recheck_primary_with_fallback` / `should_recheck_primary_with_fallback_with`：当开启 `allow_single_target_skip` 且 `expected_nodes.len() == 1` 时，若主 URL 本轮已有 `>= SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS (3)` 个非假真实节点测出有效延迟，直接跳过备用 URL 第二轮全组复核；
  2. 前端单点测速（`valid_nodes.len() == 1`）与 `evaluate_failover` 第 1 次初检失败（`consecutive_fails == 0`）启用该跳过优化，而 `consecutive_fails >= 1` 的二次确认判死前复核与多节点全灭场景仍完整保留备用 URL 防失真兜底，并补齐 Rust 单测。
- **状态**：代码已修正，待用户确认

### BUG-400 后台 180s 真 HTTPS 验真受长期离线死节点拖累引发全组每 3 分钟陪跑探测

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：后台 `run_https_verification_pass` 原先每 `180s` 仅检查是否存在「未 `verified_true` 且未软隔离」的条目即发起全组 `delay_group`。订阅中只要混有 1 个普通离线/超时节点尚未累计到 6 次失败，已验真健康节点也会每 3 分钟陪跑全组 GitHub/Wikipedia 请求（且 24h 软隔离到期后周而复始），即使活跃节点及多个备用节点均已验真可用也不停歇。
- **修复方案**：
  1. 将 `HTTPS_VERIFY_INTERVAL_SECS` 由 `180s` 放宽至 `600s`（10 分钟），新增「够用即止」门限 `PERIODIC_HTTPS_VERIFY_SUFFICIENT_STANDBYS = 3` 与纯门控函数 `should_run_periodic_https_verify_with`；
  2. `start_background_monitor` 触发后台定期验真前先筛选符合当前 `filter_text` 的真实候选：彻底排除非 `NodeStatus::Alive(_)` 的普通离线死节点（只对普通测速通但未验真的可疑节点触发）；且当当前活跃节点已 `verified_true` 并在子集内已储备 `>= 3` 个已验真 `Alive` 备用节点时直接跳过全组后台验真（切点缺候选时仍由「选点前置验真」即时覆盖），并补齐 Rust 门控单测。
- **状态**：代码已修正，待用户确认

### BUG-399 内核 `PROXY__METRICS` 与 `proxy-providers` 每 5 分钟自治盲测架空后台被动流量证活且免测窗偏短

- **对应模块**：[mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts)、[ARCHITECTURE.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/ARCHITECTURE.md)
- **问题描述**：[mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs) 原将隐藏测量组 `PROXY__METRICS` 设为 `interval: 300, lazy: false` 且原样透传上游 `proxy-providers` 的非惰性健康检查配置，致使即便系统无人用网（`Idle`）或正在通过真实下行流量被动证活（`Flowing`），Mihomo 内核仍每隔 5 分钟在后台无条件并发拨测全组节点（不受 Rust `METRICS_PROBE_LOCK` 约束），既制造大量冗余外部连接信号又架空 `monitor.rs` 按需降频设计；同时 `monitor.rs` 原 `IDLE_HEALTH_FRESH_SECS (300s)` / `PASSIVE_TRAFFIC_MAX_SKIP_SECS (480s)` 免测上限也偏短。
- **修复方案**：
  1. 在 [mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs) 新增 `enforce_lazy_provider_health_checks` 将所有 `proxy-providers.*.health-check.lazy` 强制设为 `true`，并将 `PROXY__METRICS` 调整为 `interval: 1800, lazy: true`（零无意义自治后台拨测；Rust 按需 `delay_group` 调用照常运行）；
  2. 在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 将 `IDLE_HEALTH_FRESH_SECS` 放宽至 `900s`（15 分钟）、`PASSIVE_TRAFFIC_MAX_SKIP_SECS` 放宽至 `1800s`（30 分钟），并同步更新前端 [delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts) 的 `HISTORY_FALLBACK_MAX_AGE_MS = 45 分钟` 以防止健康证活期间 UI 误落超时，更新并扩展 Rust 单元测试。
- **状态**：代码已修正，待用户确认

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

历史已确认关闭条目（`BUG-057` ~ `BUG-391`）历史演进记录详见 [Changelog.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Changelog.md) 与 Git 提交历史。

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
