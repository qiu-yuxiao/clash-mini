# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：`BUG-057` ~ `BUG-383` 已通过用户验收并完整归档于 `clash_mini_agreements.md` 附录 A。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-384 Monochrome 皮肤下基础设置四枚小开关误呈圆角胶囊而非方形开关

- **对应模块**：[base-switch.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-switch.tsx)（`getSwitchSkinStyle` / `Switch`）
- **问题描述**：`Monochrome`（黑白极简工业风）皮肤下按钮、分段控制器、卡片与滑块均采用 `0px` 直角锐利造型，但 `base-switch.tsx` 中的 `monochrome` 分支仍残留旧的大尺寸圆角胶囊样式（`56×28` / `borderRadius: 14` 与圆形滑块），导致「基础设置」卡片中的四枚 `size="small"` 小开关（开机自动启动、Allow LAN、启动时最小化、Allow IPv6）未呈现与 `Cyberpunk` 一致的小巧方形开关外观。
- **修复方案**：重构 [base-switch.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-switch.tsx) 导出 `getSwitchSkinStyle(theme)`，将 `monochrome` 分支对齐 `cyberpunk` 的小巧直角方形尺寸几何（外框 `28×14`、`sizeSmall` `18×10`、轨道 `borderRadius: 0`、滑块 `12×12` / `sizeSmall` `8×8` 且 `borderRadius: '0px'`、`transitionDuration: '150ms'`），并赋予高反差黑白纯平配色（未选中 `#737373` 描边配灰块、选中态亮/暗黑纯白实心高对比与 `1px` 锐利方框）。
- **状态**：代码已修正，待用户确认

### BUG-385 设置与宽窄窗不可见组件彻底卸载、30秒回收闭环及中途关闭设置防混乱加固

- **对应模块**：[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、[connections-panel.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/connections-panel.tsx)、[basic-settings-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/basic-settings-card.tsx)、[settings-guards.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/utils/settings-guards.ts)、[proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx)、[use-traffic-monitor.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-traffic-monitor.ts)、[constants.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/constants.ts)
- **问题描述**：① 打开设置面板期间，被遮盖的主页节点列表 `ProxyGroups` 仍保持挂载渲染；② `useConnectionData` 与 `usePathControl` 挂在常驻 `Layout` 顶层，关闭设置或缩窄窗口后因 `lastSubscriptionCacheKeyRef` 保持 Query Observer 活跃，最后一批最多 500 条连接快照未随 `gcTime(30s)` 回收，且宽窗下每秒连接更新会带动整个 `Layout` 重绘；③ `TrafficWorkerClient` 闲置销毁延迟长达 `5 分钟`；④ 中途直接关闭设置时，挂在抽屉内的 `AddUrlDialog` 被强制卸载却使 `addUrlOpen` 残留为 `true`（导致 `isAnyOverlayOpen` 永久锁死主页 `F1/F4/Ctrl+F` 快捷键）、关闭按钮 `zIndex: 9999` 越过模态弹窗、`Mixed Port` 未提交修改因 `onBlur` 误保存，以及菜单锚点未联动复位。
- **修复方案**：
  1. `_layout.tsx` 对 `ProxyGroups` 统一加 `{!drawerOpen && !isMiniStatus && (...)}` 条件卸载，并在 `ProxyHead` 补全 `60ms` 延迟定时器卸载清理（关闭设置返回主页时 `ProxyGroups` 经既有 `useLayoutEffect + localStorage` 同步还原滚动位置）；
  2. 将 `useConnectionData`、`usePathControl`、`filterConn` 与 `<ConnectionDetail />` 内聚迁入 `ConnectionsPanel`，卸载面板后立即释放 Observer 并在 30s 后触发 `gcTime` 彻底回收 500 条连接缓存、消除 `Layout` 级每秒连带重绘；
  3. 将 `TrafficWorkerClient` 闲置销毁延迟由 `5 分钟` 收敛为 `TRAFFIC_WORKER_DESTROY_DELAY_MS = 30_000`（对齐 `gcTime`）；
  4. 将 `AddUrlDialog` 移出抽屉至外层弹窗区、将设置关闭按钮 `zIndex` 从 `9999` 降为 `SETTINGS_CLOSE_BUTTON_Z_INDEX (250)` 并标 `data-close-settings="true"`、关闭抽屉时统一清空帮助/订阅/粘贴右键菜单锚点，并为 `BasicSettingsCard` 端口框加入 `shouldSkipMixedPortBlurSave`（支持 `Esc` 放弃未提交修改与直接点叉跳过保存）。
- **状态**：代码已修正，待用户确认

### BUG-386 鼠标拖拽缩放窗口时剧烈边缘抖动与反复拖拽跨断点引发卡死

- **对应模块**：[window.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/window.rs)、[app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs)、[lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs)、[resize-handles.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx)、[window-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/window/window-provider.tsx)、[window-resizing.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/window-resizing.ts)、[cmds.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts)
- **问题描述**：① 左/上/斜角拖拽手柄使用相对窗口客户区的 `e.clientX` / `e.clientY` 计算位移并与动态锚点滑动叠加，窗口原点移动反过来改变 `clientX/Y`，在鼠标静止时触发 60Hz 正反馈震荡；② `currentWindow.setSize` 与 `setPosition` 拆分为两次非原子 IPC（且 `tao::set_inner_size` 按缓存的带框样式累加偏移），造成锚定边缘撕裂；③ `WindowProvider` 的 150ms `checkMaximized` 在 `isWindowResizing()` 期间仍持续放行，反复拖过 `285px`/`126px` 断点时高频触发 `ConnectionsPanel` 挂载卸载与 WebSocket/Worker/VirtualList 重建风暴并阻塞 Win32 UI 线程；④ `pointerdown` 缺少单调手势序号保护且 `lib.rs` 的 `Resized/Moved` 监听器在每像素缩放中重复查询几何与缩放比。
- **修复方案**：
  1. 在 Rust [window.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/window.rs) 与 [app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs) 实现原生原子 Win32 缩放管线（`begin_window_resize` / `step_window_resize` / `end_window_resize`）：以 `GetCursorPos` 全局屏幕坐标、不变起始矩形、`GetWindowRect`/`GetClientRect`/`GetDpiForWindow` 与防死区游标滑动结合 `PENDING_SWP_RECT` + `RESIZE_SWP_QUEUED` 合并主线程派发单次 `SetWindowPos(..., SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOCOPYBITS)`，同步将 `sizing_subclass_proc` 升级至全 8 向 `WM_SIZING` 宽高钳制及 `WM_ENTERSIZEMOVE`/`WM_EXITSIZEMOVE` 状态与收尾落盘，并在 [lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs) 拖拽期间短路 `Resized`/`Moved` 冗余几何查询；
  2. 在 [resize-handles.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx) 接入单调 `gestureSeq` + `rAF` 背压步进并由 [window-resizing.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/window-resizing.ts) 的 `RESIZE_ENDED_EVENT` 透传最终尺寸与缩放比；
  3. 在 [window-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/window/window-provider.tsx) 仅记录最新 `onResized` 物理像素并在 `isWindowResizing() === true` 时冻结 `checkMaximized` 断点翻转，松手触发 `RESIZE_ENDED_EVENT` 时一次性精准对齐布局，杜绝拖拽反复跨越断点时的重组件风暴与卡死。
- **状态**：代码已修正，待用户确认

### BUG-387 节点断流时顶栏残留假绿延迟与恢复后列表滞留假 Timeout 的双向显示失步修复

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts)、[delay.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.test.ts)
- **问题描述**：① 当活跃节点实际卡死黑洞（`Stalled`：有上行发包但零下行回包）且整组拨测 `Ok({})` 全超时、或主/备测速地址双双返回 `all proxies timeout` 令 `probe_metrics_group` 返回 `None` 时，`evaluate_failover` 因 `!probed_delays.is_empty()` 门控或提前 `NoAction` 返回而未通知前端刷新延迟，顶栏与列表长期残留断流前的假绿延迟；② 当 `primary_url_distorted = true`（主测速地址全局失真、备用地址仅部分外区节点响应）时，`evaluate_failover` 与 `trigger_backend_auto_select_inner` 仍用 `.unwrap_or(0)` 把未响应节点批量漆成 `0 (Timeout)` 写入 30 分钟长缓存；同时 `trigger_backend_auto_select_with_options` 对仅测试 1~N 个子集的结果也会置位 `reported_live_to_ui = true`；③ Mihomo 在探测超时时不向 `proxy.history` 追加失败记录，而旧 `LAST_SYNCED_ACTIVE_HISTORY_TIME` 在向 UI 推送 `0 (Timeout)` 时仍记录了断流前的旧成功时间戳，导致数秒后活跃节点免拨测复检恢复 `Alive` 时因时间戳未变且健康态仍为 `ok` 而静默跳过回传；④ 前端 `DelayManager.getDelayFix` 在节点已凭 `proxy.alive === true` + 更新时间严格新于超时缓存的 `history` 记录恢复后仍优先返回旧 `0 (Timeout)` 缓存（长达 30 分钟），且 `injectBatchResults(..., { fromProbe: false })` 会沿用此前绿缓存长达 30 分钟的 `updatedAt` 锁定复用 `Timeout`。
- **修复方案**：
  1. 在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 将 `LAST_SYNCED_ACTIVE_HISTORY_TIME` 升级为 `UiActiveSyncState`（`LAST_UI_ACTIVE_SYNC`，含 `node_name`、`history_time`、`reported_alive`），并通过 `build_probe_ui_display` / `compute_next_ui_active_sync` / `should_sync_alive_ui` 保证：`!primary_url_distorted` 且 `probed_delays` 为空时同样向前端下发全组 `0 (Timeout)`；推送 `0 (Timeout)` 时绝不记录旧成功 `history_time` 而置 `reported_alive = false` 并在恢复 `Alive` 时无条件回传唤醒；`primary_url_distorted = true` 仅推送真实测得的有效延迟（不把未响应节点漆成 `0 (Timeout)` 且不返回全黑 `0` 选点展示）；`Stalled` 流量黑洞叠加 `probe_metrics_group -> None` 时在维持 `NoAction` 不乱切点前提下推送 30s 短效 `(active_node, 0)`（`from_probe = false`）警示 UI；
  2. 为 `AutoSelectOutcome` 增加 `covers_all_group_nodes` 字段，仅全组无过滤非失真拨测才置位 `LAST_METRICS_PROBE.reported_live_to_ui = true`，并在选点/子集同步后对称维护 `LAST_UI_ACTIVE_SYNC`；
  3. 在 [delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts) 修复 `getDelayFix`：当缓存为 `Timeout`（`0` 或 `>= NODE_DELAY_MAX_MS`）而 `proxy.alive === true` 且最新有效 `history` 时间戳严格新于 `update.updatedAt`（且未超 `HISTORY_FALLBACK_MAX_AGE_MS`）时立即淘汰陈旧超时缓存并回显最新有效延迟（健康绿缓存仍优先于 `alive` 标记）；同步收紧 `injectBatchResults(..., { fromProbe: false })` 仅健康延迟允许延续旧有效 `updatedAt`，`Timeout` 一律封顶 `REUSED_METRICS_MAX_TTL_MS`（`30s`）；
  4. 严守零切换规则变更边界（`BUG-372`/`375`/`376`/`377`/`379` 自愈门限均不变），并在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 与 [delay.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.test.ts) 补齐双向同步与缓存淘汰单测锁定。
- **状态**：代码已修正，待用户确认

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

历史已确认关闭条目（`BUG-057` ~ `BUG-383`）完整追溯映射详见 `clash_mini_agreements.md`「附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表」。

| 缺陷编号 | 对应版本基线 | 状态 |
| :--- | :---: | :---: |
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
