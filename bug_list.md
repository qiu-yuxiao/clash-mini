# Clash Mini 待处?Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> > **🤖 AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES FOR AI AGENTS)**:
> > 
> > 本文件使?**「结构化卡片 + 极简索引表?* 的双轨设计。所有参与本项目?AI 协同助理在读?and 更新本文件时，必须严格遵守以下书写与维护规范，严禁格式退化：
> > 
> > 1. **严禁在表格中堆叠排查细节**：只有已?Master 验证确认关闭?Bug 才能放入「已解决的历?Bug 索引」表格中，且状态统一写为 `代码已修正，已确认`。表格中禁止包含任何 `设计要求`、`代码状态` 等冗余的后台开发过程细节?
> > 2. **待验?排查?Bug 强制卡片?*：任何状态为 `排查中` ?`代码已修正，待用户确认` ?Bug，必须在顶部的「待验证与活动中 Bug 详情」区以独立标?and 结构化字段登记?
> > 3. **强制记录排查记忆，防止重复劳?*：在卡片中，必须详实搜集、继承并持久化记录以下三个协同要素：

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-215: Active Connection Node status row layout collapse and styling loss under strict CSP
 - **现象描述**：启用严格 CSP 后，WebView2 拒绝加载未显式放行的 `tauri://` 与 `asset://` 协议下的静态 CSS 资源及 Emotion 动态注入的样式，导致页面全部类样式失效，界面彻底退化为无样式灰白色，活动出口节点卡片也由于样式失效而失去 Flex 和高度约束产生崩塌。
 - **当前状态**：`已随 CSP 回滚至 null 而废弃还原`
 - **目标版本**：`v1.8.0`

### BUG-216: Cleanup of temporary CSS bypass styling workarounds
 - **现象描述**：在 1.7.6 临时版本中，为了回避 Emotion 在 CSP 拦截下的尺寸溢出问题，将窗口控制按钮、顶部置顶及设置齿轮图标的样式临时写死为了内联 `style` 属性。
 - **当前状态**：`已随 CSP 回滚至 null 而废弃还原`
 - **目标版本**：`v1.8.0`

### BUG-223: Contrast and Invisible Borders Styling Issue in Frosted Glass Skin
 - **现象描述**：Frosted Glass（毛玻璃）下失效按钮的背景与边框被硬编码为半透明白色，在浅色主题下几乎不可见，对比度低于 2.6:1。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.1`

### BUG-224: React useMemo Dependency Array Omits theme and skin
 - **现象描述**：顶栏标题渲染在 `useMemo` 中被缓存，但其依赖数组未加入 `theme` 模式与 `skin`（`controlSkin`）变量，导致切换皮肤或深浅色模式时顶栏不刷新。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.1`

### BUG-233: CSS Specificity Conflict on Dialog/Menu Backgrounds under Frosted Glass
 - **现象描述**：毛玻璃皮肤下的优先级选择器覆盖了全局 Dialog 遮罩的 opacity 配置，将原本不透明背景重写为半透明，导致弹窗文字与网页文字叠显冲突，无法看清。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.1`

### BUG-234: Hardcoded Opaque Background Colors Overriding Theme Skins
 - **现象描述**：部分组件（如 base-page, unlock 页面等）在 dark mode 判定中强制写死深灰色背景，覆盖了毛玻璃或赛博朋克等特制皮肤的透明度与背景设计。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.1`

### BUG-235: Scrollbar Hiding Overrides and Layout Conflict
 - **现象描述**：全局 `index.scss` 强行指定了 `* { scrollbar-width: thin !important; }`，覆盖了连接表局部指定的 `scrollbarWidth: 'none'`，导致局部滚动条无法隐藏。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.1`

### BUG-237: Vertical Clipping Hazard in Compact Proxy Columns under High DPI
 - **现象描述**：代理节点列的行高写死了 `20px` 且强制 `overflow: hidden`，在 Windows 高 DPI 缩放（150%以上）时，文字和延迟标签会被底部横向物理裁剪。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.1`

### BUG-238: Navigation Menu Scrollbar Truncation
 - **现象描述**：侧边导航菜单硬编码隐藏了滚动条，在低分辨率小屏幕下导致用户无法察觉侧边栏还有可滚动的内容。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.1`

### BUG-239: High-frequency polling on proxies and connections causing high CPU/Resource consumption
 - **现象描述**：主页数据刷新采用前端主动发起、每隔 3 秒一次的高频轮询机制（calcuProxies），在闲置或数据无变化时产生持续的 CPU 唤醒与进程通信开销；同时，连接管理面板在后台或被遮挡时缺乏深度可见性判定。
 - **当前状态**：`代码已修正，待用户确认`
 - **需求与改进方案**：
   1. **轮询重构为事件信号驱动（Event-Driven）**：
      - 前端彻底关闭每 3 秒一次的 `refetchInterval` 定时请求。
      - 后端在节点切换成功、测速结束、故障自愈完成以及配置重载等核心状态发生真实改变时，通过 Tauri Event 机制向前端发送轻量级 `"REFRESH_PROXIES"` 信号。
      - 前端监听该信号事件，仅在收到信号时单次拉取最新数据，实现空闲时 0 轮询开销。
   2. **连接追踪可见性判定**：
      - 对设置抽屉中的连接列表（Connections Panel）进行组件级可见性监控，若面板处于遮挡或非活动标签状态，自动暂停高频 WebSocket 追踪。
 - **目标版本**：`v1.8.2`

### BUG-205: Settings Drawer Horizontal Layout Overflow
 - **现象描述**：在默认/最小窗口宽度（270px）下，设置抽屉的横向布局挤压右侧 Connections 列，导致 active/closed 连接列表宽度被压缩为 0px。经重新审计确认，此为项目 Agreement 设计规范中预期的“物理裁剪遮盖”设计，而非布局缺陷。
 - **验证方法**：已完全撤销本地换行和最小宽度修改，还原为原生单行横向并排布局与 minWidth: 0，确保窄窗口下连接面板被正常裁剪遮挡，窗口拉宽时正常侧向展露。
 - **当前状态**：`已还原并确认`
 - **目标版本**：`v1.7.2`

### BUG-206: Skin Switcher & Language Selector Hidden at Default Window Height
 - **现象描述**：设置抽屉内绝对定位的皮肤切换器和语言选择器，在窗口高度低于 830px 时被完全隐藏。经重新审计确认，此为项目 Agreement 设计规范中预期的“响应式高度裁剪规则”，旨在极窄/极扁高度下保持界面整洁，防止元素重叠，属于非缺陷的设计约束。
 - **验证方法**：已完全撤销本地的高度阈值修改，恢复为原始的 `@media (max-height: 830px)`，确保窄高度窗口下的正常裁剪机制发挥作用。
 - **当前状态**：`已还原并确认`
 - **目标版本**：`v1.7.2`

### BUG-213: Monochrome Skin Dark Mode Card Background Contrast Loss
 - **现象描述**：Monochrome 皮肤在深色模式下强制 cards/panels 背景与窗口整体背景（`--background-color`）一致，导致内容卡片完全融合在背景中，缺乏视觉层次感。
 - **验证方法**：Monochrome 皮肤在深色模式下，cards/panels 采用 `--theme-panel-bg` 渲染，与主背景 `#0f1423` 有对比色阶。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.7.2`

### BUG-214: Switch Component Sizing Inconsistency in Cyberpunk/Monochrome Skins
 - **现象描述**：Cyberpunk 和 Monochrome 皮肤的 small 尺寸开关没有适配比例，强行按普通尺寸渲染，导致小开关在 settings 卡片中宽度溢出或表现异常。
 - **验证方法**：small 尺寸的 BaseSwitch 组件渲染出的尺寸明显小于普通尺寸开关（Monochrome 为 28x14px，Cyberpunk 为 18x10px）。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.7.2`

### BUG-241: Settings Drawer active and resources polling in mini mode (AUDIT-02)
 - **现象描述**：当窗口缩放至迷你模式时，根据项目设计协议规范，除了当前活跃节点、4个流量小卡片和1个流量图之外，其他无关组件都必须强制物理卸载。然而，若用户在大窗口下打开了设置抽屉（即 `drawerOpen` 为 `true` 时），在窗口缩小后，由于设置抽屉仅根据 `drawerOpen` 条件渲染，它仍会保持挂载状态留在 DOM 树中。这违反了迷你模式的完全卸载规约，且会导致抽屉内的连接面板（Connections Panel）在后台继续进行高频 WebSocket 流量轮询，白白消耗系统 CPU 与 IPC 资源。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.8`

### BUG-242: Mismatched window threshold hiding titlebar in normal layout (AUDIT-03)
 - **现象描述**：`window-provider.tsx` 中定义的迷你宽度阈值为 `MINIMAL_WIDTH_THRESHOLD = 290`（像素），而在 React 前端渲染组件（如 `_layout.tsx` 和 `app-data-provider.tsx`）中检测迷你模式的阈值为 `285`（像素）。此阈值不一致导致在窗口宽度处于 286px 至 290px 之间时产生视觉异常：窗口服务认为窗口处于迷你模式并调用后端 IPC 隐藏了系统原生标题栏与窗口控制按钮，但 React 布局仍判定窗口处于正常模式并进行全量排版，这使得用户看到一个普通窗口却没有顶栏和任何窗口控制组件（最小化/最大化/关闭）。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.8`

### BUG-256: Shell Program Exits and Panics on Window Close in Auto Lightweight Mode
 - **现象描述**：当开启自动轻量模式时，用户点击主窗口关闭（X）按钮，主程序退出且系统托盘图标消失，但后端 mihomo 核心进程仍在后台运行。原因是 `WindowEvent::CloseRequested` 被注册在 `app.run` 事件循环中，此时拦截已失效，导致窗口被 Windows/Tauri 默认行为销毁；随后触发的 `ExitRequested` 误判轻量模式状态并触发退出逻辑，使得在窗口销毁与事件循环退出期间产生 Tao 窗口状态冲突，最终引发 `cannot move state from Destroyed` 崩溃退出。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.8.8`

### BUG-257: High CPU/Battery Consumption of Background Health Check in Lightweight Mode
 - **现象描述**：当主程序隐藏并进入轻量化模式后，后台监测线程 `start_background_monitor` 仍以固定 15 秒的周期频繁发起活跃代理节点的延迟探测（DNS 解析及 HTTP 请求），这在无人值守、纯后台挂机状态下会导致不必要的 CPU 唤醒、物理网卡工作和电池消耗。需要实现轻量模式下的自适应探测周期，当开启轻量模式时自动将正常检测的间隔时间放宽至 60 秒，并在恢复/退出轻量模式时瞬间重置/唤醒检测。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.9.0`

### BUG-258: Active TCP Connections Leftover in Mihomo Kernel on Entering Lightweight Mode
 - **现象描述**：当程序进入轻量化模式（主窗口销毁）时，Mihomo 内核进程中可能依然残留有大量之前网页浏览遗留的活动/空闲 TCP 连接和套接字。在纯后台挂机期间，这些连接依然会占用系统的网络套接字及内核的物理内存。需要在进入轻量模式的瞬间，由 Rust 后端向内核发送清空连接命令，清空所有活跃与空闲连接，使内核进入真正的低能耗状态。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.9.0`

### BUG-259: WebSocket Subscription CPU Overhead in Rust Backend on Entering Lightweight Mode
 - **现象描述**：主窗口销毁进入轻量模式后，Rust 后端如果仍维持与 Mihomo 内核的数据流（如流量、日志、连接明细等）WebSocket 订阅，会导致在后台继续进行无用的 JSON 反序列化 and 进程间通信（IPC）计算，产生不必要的 CPU 资源开销。需要在进入轻量模式时主动熔断、清理所有的后台 WebSocket 订阅，等主窗口重建时再重新订阅。
 - **当前状态**：`代码已修正，待用户确认`
 - **目标版本**：`v1.9.0`

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

所有已通过 Master 验证并确认关?of Bug，在此进行极简化表格索引?

| **BUG-212** | Original Skin Accent Color Dynamic Shift Mismatch | v1.7.2 | 代码已修正，已确认 |
| **BUG-211** | Window Close Unconditionally Destroys WebView State | v1.7.2 | 代码已修正，已确认 |
| **BUG-208** | Theme/CSS Injection Patch Updates Silently Ignored | v1.7.2 | 代码已修正，已确认 |
| **BUG-207** | Tightened Asset Protocol Blocks Icons in Portable Mode | v1.7.2 | 代码已修正，已确认 |
| **BUG-203** | Outfit Font Blocked by CSP causing Layout Chaos | v1.7.0 | 代码已修正，已确认 |

| **BUG-171** | Profile Switch/Import Auto-Select Hanging & Speed Test Error Responses | v1.5.7 | 代码已修正，已确认 |
| **BUG-168** | Upgrade Command RwLock Writer Starvation | v1.5.5 | 代码已修正，已确认 |
| **BUG-217** | Hook Timer Leak / Profile Switch Starvation | v1.8.1 | 代码已修正，已确认 |
| **BUG-218** | ActiveAutoSelectTimer Leak / Profile Switch State Corruption | v1.8.1 | 代码已修正，已确认 |
| **BUG-219** | Double Selection Request Race Condition | v1.8.1 | 代码已修正，已确认 |
| **BUG-220** | Memory Leak of Unresolved Promises in frontendAutoSelect | v1.8.1 | 代码已修正，已确认 |
| **BUG-221** | Unhandled Promise Rejection in checkDelay | v1.8.1 | 代码已修正，已确认 |
| **BUG-222** | Timeout Resource (Timer Handle) Leak in checkDelay | v1.8.1 | 代码已修正，已确认 |
| **BUG-225** | Zero-Length Slice Read in HTTP Latency Test (clash.rs) | v1.8.1 | 代码已修正，已确认 |
| **BUG-226** | Concurrency Hang / Infinite Loop in JS Script Validation | v1.8.1 | 代码已修正，已确认 |
| **BUG-227** | Core Validation Infinite Hang due to Missing Command Timeout | v1.8.1 | 代码已修正，已确认 |
| **BUG-228** | SSRF Bypass via DNS Resolution and IPv6 Local Ranges | v1.8.1 | 代码已修正，已确认 |
| **BUG-229** | Silent Application Exit on Port Collision | v1.8.1 | 代码已修正，已确认 |
| **BUG-230** | Lost Notification Bug in Background Monitor | v1.8.1 | 代码已修正，已确认 |
| **BUG-231** | Crash/Error on Relative Startup Script Paths | v1.8.1 | 代码已修正，已确认 |
| **BUG-232** | Unbounded Memory Allocation Risk on Subscriptions | v1.8.1 | 代码已修正，已确认 |
| **BUG-236** | Inactive Scrollbar Variables Set in CSS but Unused | v1.8.1 | 代码已修正，已确认 |
| **BUG-209** | Startup Script Blocks Window Initialization | v1.7.2 | 代码已修正，已确认 |
| **BUG-210** | Service Manager Reinstall Deadlock during Startup | v1.7.2 | 代码已修正，已确认 |
| **BUG-204** | Clippy Warnings Blocking Git Push | v1.7.1 | 代码已修正，已确认 |
| **BUG-202** | Windows Named Pipe IPC Path Validation Mismatch | v1.6.7 | 代码已修正，已确认 |
| **BUG-240** | getProxies query double-triggering during silent startup (AUDIT-01) | v1.8.8 | 代码已修正，已确认 |
| **BUG-243** | Stale size refs check in resetIdleTimer callback (AUDIT-04) | v1.8.8 | 代码已修正，已确认 |
| **BUG-244** | Unused isDark variable in base-page.tsx (WARN-01) | v1.8.8 | 代码已修正，已确认 |
| **BUG-245** | Shadowed/unused theme variable in proxy-groups.tsx (WARN-02) | v1.8.8 | 代码已修正，已确认 |
| **BUG-246** | Unused destructured prop mode in layout-dialogs.tsx (WARN-03) | v1.8.8 | 代码已修正，已确认 |
| **BUG-247** | Synchronous state update inside layout effect in _layout.tsx (WARN-04) | v1.8.8 | 代码已修正，已确认 |
| **BUG-248** | Explicit any in context refreshers in app-data-context.ts (WARN-05) | v1.8.8 | 代码已修正，已确认 |
| **BUG-249** | Explicit any in debounce rest parameters (WARN-06) | v1.8.8 | 代码已修正，已确认 |
| **BUG-250** | Explicit any in profile parser raw proxies in cmds.ts (WARN-07) | v1.8.8 | 代码已修正，已确认 |
| **BUG-251** | Explicit any in timer identifier in delay.ts (WARN-08) | v1.8.8 | 代码已修正，已确认 |
| **BUG-252** | Explicit any in language modules reduction in i18n.ts (WARN-09) | v1.8.8 | 代码已修正，已确认 |
| **BUG-253** | Explicit any in nameserver policy type dictionary in clash.ts (WARN-10) | v1.8.8 | 代码已修正，已确认 |
| **BUG-254** | Explicit any in traffic monitor validator functions in traffic.ts (WARN-11) | v1.8.8 | 代码已修正，已确认 |
| **BUG-255** | Explicit any in debug log rest parameters in debug.ts (WARN-12) | v1.8.8 | 代码已修正，已确认 |
| **BUG-167** | Rust Local Socket Timeout Inconsistency | v1.5.5 | 代码已修正，已确认 |
| **BUG-166** | Fallback Timeout Inconsistency in layout code comments | v1.5.5 | 代码已修正，已确认 |
| **BUG-162** | Timeout Resource (Timer Handle) Leak | v1.5.5 | 代码已修正，已确认 |
| **BUG-161** | Unhandled Promise Rejection in `checkDelay` | v1.5.5 | 代码已修正，已确认 |
| **BUG-160** | Memory Leak of Unresolved Promises | v1.5.5 | 代码已修正，已确认 |
| **BUG-159** | Double Selection Request Race Condition | v1.5.5 | 代码已修正，已确认 |
| **BUG-158** | Profile Switch State Race Condition & Background Timer Leak | v1.5.5 | 代码已修正，已确认 |
| **BUG-157** | React Hook Dependency Safety — Permanent Cancellation of Profile Activation | v1.5.5 | 代码已修正，已确认 |
| **BUG-156** | Fallback Timeout Inconsistency in `_layout.tsx` | v1.5.5 | 代码已修正，已确认 |
| **BUG-154** | Memory Leak / Stale Data Race Condition in `ActiveNodeStatusCard` | v1.5.5 | 代码已修正，已确认 |
| **BUG-153** | Cancelled SWR/Effect Race Condition on `lastEnhancedProfileRef` | v1.5.5 | 代码已修正，已确认 |
| **BUG-152** | Closed Semaphore Bypass | v1.5.5 | 代码已修正，已确认 |
| **BUG-151** | Empty Connection Pool / Redundant Sockets | v1.5.5 | 代码已修正，已确认 |
| **BUG-150** | Rust Local Socket Timeout Mismatch | v1.5.5 | 代码已修正，已确认 |
| **BUG-149** | Healthy Node Delay Threshold Mismatch | v1.5.5 | 代码已修正，已确认 |
| **BUG-165** | Monochrome Skin Slider Label Inconsistency | v1.5.5 | 代码已修正，已确认 |
| **BUG-164** | React `useMemo` Dependency Array Omits theme and skin | v1.5.5 | 代码已修正，已确认 |
| **BUG-163** | Contrast and Invisible Borders Styling Issue | v1.5.5 | 代码已修正，已确认 |
| **BUG-155** | Metrics Row Card Ordering Mismatch | v1.5.5 | 代码已修正，已确认 |
| **BUG-148** | Menu and Dialog Transparent Background | v1.5.5 | 代码已修正，已确认 |
| **BUG-147** | 主题参数调节滑动条上限及范围偏离规范 | v1.5.5 | 代码已修正，已确认 |
| **BUG-145** | `isImportingRef` Deadlock in `handleSelectProfile` | v1.5.5 | 代码已修正，已确认 |
| **BUG-144** | Blocked Auto-Select on Profile Switch | v1.5.5 | 代码已修正，已确认 |
| **BUG-143** | RwLock Writer Starvation on WebSocket Disconnect | v1.5.5 | 代码已修正，已确认 |
| **BUG-142** | Socket Connection Timeout Bypass | v1.5.5 | 代码已修正，已确认 |
| **BUG-141** | Windows Named Pipe Busy Infinite Loop | v1.5.5 | 代码已修正，已确认 |
| **BUG-140** | 启动后右上角的齿轮与图钉图标出现在原生窗口标题栏内 | v1.5.4 | 代码已修正，已确认 |
| **BUG-139** | 底部流量卡片排列方式不符合 agreements 规范（宽/窄窗口模式均不符） | v1.5.3 | 代码已修正，已确认 |
| **BUG-138** | 闪电光标（批量测速）点击后无任何反应，无法触发全节点测速 | v1.5.3 | 代码已修正，已确认 |
| **BUG-136** | 导入订阅链接报错无法导入节点 | v1.4.9 | 代码已修正，已确认 |
| **BUG-135** | 检查 DNS 配置文件是否存在命令同步阻塞 UI 主线程 | v1.4.8 | 代码已修正，已确认 |
| **BUG-134** | 删除某一个订阅链接后，界面卡死在空白状态，无法自动切换到其他可用订阅，日志报错“未找到指定 ID” | v1.4.8 | 代码已修正，已确认 |
| **BUG-133** | 异常回滚恢复 Profile 时未触发核心重载导致配置脱节 | v1.4.8 | 代码已修正，已确认 |
| **BUG-132** | DNS 切换应用命令对内核配置及持久化写入无? | v1.4.8 | 代码已修正，已确认 |
| **BUG-131** | 后台读取保存排序类型时使用效率低下的 YAML 解析器解析 JSON | v1.4.8 | 代码已修正，已确认 |
| **BUG-130** | 测速排序类型为 None 时忽略本地已存配置直接写死为 1 | v1.4.8 | 代码已修正，已确认 |
| **BUG-129** | 后台自愈优选检测因互斥原子锁忙而跳过配置重载切换 | v1.4.8 | 代码已修正，已确认 |
| **BUG-128** | 皮肤风格分段选择器渲染高频触发 LocalStorage 同步读取 | v1.4.8 | 代码已修正，已确认 |
| **BUG-127** | 底部流量指标卡片排列顺序不统一 | v1.4.8 | 代码已修正，已确认 |
| **BUG-126** | 弹出菜单与对话框层级透明背景导致文字重叠不可读 | v1.4.8 | 代码已修正，已确认 |
| **BUG-125** | 主题参数调节滑动条边界范围未遵守各皮肤的特定设计规范 | v1.4.8 | 代码已修正，已确认 |
| **BUG-124** | 活跃节点状态卡片频繁切换节点名称时产生内存泄漏与竞态覆盖 | v1.4.8 | 代码已修正，已确认 |
| **BUG-123** | 快速切换配置时 catch 逻辑相互覆盖引发 SWR 重复请求与重试 | v1.4.8 | 代码已修正，已确认 |
| **BUG-122** | 切换配置?profile 页面状态加载死锁，自动优选不触发 | v1.4.8 | 代码已修正，已确认 |
| **BUG-121** | 导入订阅后，活跃出口节点被固定节点长期占用，永不更新（10 分钟以上不变化） | v1.4.9 | 代码已修正，已确认 |
| **BUG-120** | 顶栏图钉与齿?叉子按钮未遵守六种皮肤风格，且两按钮之间有空? | v1.4.8 | 代码已修正，已确认 |
| **BUG-119** | 设置界面多处文字不随语言切换（基础设置、主题设置） | v1.3.9 | 代码已修正，已确?|
| **BUG-118** | 重启程序后，节点无法自动选优，持续等待（10 分钟以上）仍不恢复 | v1.5.3 | 代码已修正，已确认 |
| **BUG-117** | Trump-3D 按钮与卡片阴影恢复为多层 bevelShadowDark 固态挤压，修复 BUG-115 过度简化导致的 3D 质感丢失?| v1.3.8 | 代码已修正，已确?|
| **BUG-116** | 主题设置滑块布局由两行压缩为单行，名?数值与滑块本体合并为单行布局，垂直空间利用率优化?| v1.3.6 | 代码已修正，已确?|
| **BUG-115** | Retro-3D 皮肤按钮与卡?boxShadow 叠加层数过多问题，将多层 bevelShadowDark 叠加简化为单层内阴?单层外阴?单层顶高光结构，与分段选择器保持一致?| v1.3.5 | 代码已修正，已确?|
| **BUG-114** | 宽窗口上下容器高度重分配：上?30px /下部-30px，下部容器内?gap 设为零?| v1.3.5 | 代码已修正，已确?|
| **BUG-113** | 闪电光标点击后节点延迟无更新（回归），统一?handleCheckAll 流程，确?PROXY 组也通过 delayManager 执行完整延迟测试并更?UI?| v1.3.6 | 代码已修正，已确?|
| **BUG-112** | BaseSearchBox 小光标无视觉交互反馈，将正则/区分大小?匹配整词图标从裸 `SvgIcon` 包裹?`IconButton` 中，添加 hover/active 状态反馈?| v1.3.5 | 代码已修正，已确?|
| **BUG-111** | 应用启动时因代理数据未就绪导致渲染崩溃，在所有访?undefined 中间属性的代码处添加可选链或空值合并防护?| v1.3.4 | 代码已修正，已确?|
| **BUG-110** | useMihomoWsSubscription中QueryKey元素为undefined的隐患，安全处理 QueryKey 的空值防护?| v1.3.4 | 代码已修正，已确?|
| **BUG-109** | getAutotemProxy前端接口名称拼写错误，重构并统一更名?getAutoProxy?| v1.3.4 | 代码已修正，已确?|
| **BUG-108** | Vite配置中缺少base路径导致打包后静态资源加载失败，添加 base: './' 配置?| v1.3.4 | 代码已修正，已确?|
| **BUG-107** | useWindowWidth中document.body未定义导致的空指针异常，添加安全检查和 fallback?| v1.3.4 | 代码已修正，已确?|
| **BUG-106** | 修复 _layout.tsx 内部状态变量和核心数据对象宽松 any 类型定义导致类型系统失效的问题?| v1.3.3 | 代码已修正，已确?|
| **BUG-105** | 优化 use-profiles.ts 内部回调函数引用稳定性，使用 useCallback 进行 memoize 包装，防止子组件重复重绘?| v1.3.3 | 代码已修正，已确?|
| **BUG-104** | 修复 useWindowSnap.ts 文件命名大小写不一致问题，将其重命名为 use-window-snap.ts 并同步更新导入路径?| v1.3.3 | 代码已修正，已确?|
| **BUG-103** | 解决全局环境类型污染问题，将全局声明重构?ESM 模块化导入?| v1.3.4 | 代码已修正，已确?|
| **BUG-102** | 布局文件的上帝组件单体化，将 _layout.tsx 拆解为多个子组件（ActiveNodeStatusCard、MiniTrafficPanel、BasicSettingsCard、ThemeSettingsCard 等）?| v1.3.4 | 代码已修正，已确?|
| **BUG-101** | 优化 TS 辅助 Hook 内部函数与状态类型隐患，使用 useCallback 稳定回调函数引用，确?proxies 等核心变量类型?| v1.3.3 | 代码已修正，已确?|
| **BUG-100** | 修复后端时间戳数值强转在 32 位操作系统或嵌入式平台下的截断与溢出隐患，将相关字段统一调整?Option<i64> 类型?| v1.3.3 | 代码已修正，已确?|
| **BUG-099** | 修复客户端启动时 Tauri Setup 钩子同步 block_on 导致的主 UI 线程白屏挂起与无响应问题，改用异步并发启动更新任务?| v1.3.3 | 代码已修正，已确?|
| **BUG-098** | 修复同步系统进程扫描阻塞异步线程的问题，?sysinfo 系统进程扫描和强杀包裹?spawn_blocking 中执行?| v1.3.3 | 代码已修正，已确?|
| **BUG-097** | 修复异步工作线程被同步磁?I/O 阻塞的问题，配置读取迁移?tokio::fs 异步 API，大文件写入使用 spawn_blocking 处理?| v1.3.3 | 代码已修正，已确?|
| **BUG-096** | 修复后端常驻线程因异步跨越持?RwLock 读锁而导致的死锁问题，限制锁的生命周期在 await 之前 drop?| v1.3.3 | 代码已修正，已确?|
| **BUG-095** | 窗口缩放高频渲染风暴，useWindowWidth 只在跨越临界值时触发状态变更?| v1.3.4 | 代码已修正，已确?|
| **BUG-094** | 异步事件监听器内存泄漏，引入 active 布尔标志追踪生命周期，异?resolve 时检测并处理?| v1.3.4 | 代码已修正，已确?|
| **BUG-093** | Clash Mini 启动或重新加载配置后，节点自动激活与连接存在?1.5 ?3 分钟的显著延迟，以及订阅更新后恢复历史选点失效的问题?| v1.3.2 | 代码已修正，已确?|
| **BUG-092** | 手动触发全体测速完成后自动切换到过滤子集中的最快节点的问题?| v1.3.1 | 代码已修正，已确?|
| **BUG-091** | 订阅配置输入框无法使用鼠标右键进行复制、粘贴等右键菜单操作的问题?| v1.3.1 | 代码已修正，已确?|
| **BUG-090** | Trump-3D 皮肤在深色模式下设置模组与出口节点栏文字因为白色导致模糊不清的问题?| v1.3.1 | 代码已修正，已确?|
| **BUG-089** | 后台监测活跃节点延迟高频误报，导致正常的低延迟活跃节点被强制切换的问题?| v1.3.0 | 代码已修正，已确?|
| **BUG-088** | Trump-3D 皮肤在深色模式下默认实心按钮文本颜色由于与背景太接近导致几乎无法辨认的问题?| v1.3.0 | 代码已修正，已确?|
| **BUG-087** | 将现有的 Retro-3D 风格?UI 中改名为 Trump-3D，并重构其视觉样式为奢华黄金金条风格?| v1.3.0 | 代码已修正，已确?|
| **BUG-086** | 窄视口模式下底部的流量曲线图高频跌落至零并呈现锯齿状断裂的问题?| v1.3.0 | 代码已修正，已确?|
| **BUG-085** | 自动刷新检测节点健康时，频现“所有线路都繁忙”误报弹窗，且后台检查无静默运行的问题?| v1.3.0 | 代码已修正，已确?|
| **BUG-084** | 自动测速选择逻辑中存在冗?层层叠加?dummy 节点过滤的问题?| v1.3.0 | 代码已修正，已确?|
| **BUG-083** | 程序启动后测速选定节点未在用户筛选的子集中，需手动干预后才生效的问题?| v1.3.0 | 代码已修正，已确?|
| **BUG-082** | 程序运行?WebView2 后台内存占用过高，最小化或隐藏到系统托盘时未对内存使用目标进行回收优化?| v1.3.2 | 代码已修正，已确?|
| **BUG-081** | 导入或更新订阅链接后，节点列表不会立即更新，通常需要等待两分钟的问题?| v1.3.0 | 代码已修正，已确?|
| **BUG-080** | 内核或程序更新检查时版本一致会直接弹出更新对话框，且如果在升级逻辑中触发了相同版本的升级，未在各层级安全熔断并显示正确的友好通知?| v1.2.6 | 代码已修正，已确?|
| **BUG-079** | 分流策略倾向中“规则可调”气泡说明描述不准确且多国语言未对?| v1.2.6 | 代码已修正，已确?|
| **BUG-078** | 在客户端中点击检查并尝试升级 Mihomo 内核时，内核无法成功下载或升级，没有做超时和多重代理检测?| v1.2.5 | 代码已修正，已确?|
| **BUG-077** | 在客户端中点击检查并尝试升级 Mihomo 内核时，内核无法成功下载或升级，资源匹配不准确导致包匹配错误?| v1.2.4 | 代码已修正，已确?|
| **BUG-076** | 内核已是最新版本时，立即更新按钮仍可点击且触发下载重载 | v1.2.6 | 代码已修正，已确?|
| **BUG-075** | 系统资源?I/O 内存消耗过?| v1.2.6 | 代码已修正，已确?|
| **BUG-074** | 无法直接访问 GitHub 导致检查内核更新超时失败，且内核版本号前缀重复显示?`vvX.Y.Z`?| v1.2.1 | 代码已修正，已确?|
| **BUG-073** | Windows系统托盘图标及提示语未显示，启动后托盘不显示且日志报错?| v1.1.9 | 代码已修正，已确?|
| **BUG-072** | 在所?6 种皮肤下，将 Allow LAN (局域网共享) 开关拨到右侧开启后，开关的视觉状态卡在左侧不更新?| v1.2.0 | 代码已修正，已确?|
| **BUG-071** | 客户端与内核自动更新功能开发及 UI 下拉菜单适配，并兼容最新内核?| v1.1.8 | 代码已修正，已确?|
| **BUG-070** | 当以管理员权限启动程序且无服务运行时，若开?TUN 模式，系统会运行两个 mini-mihomo 进程且有 5 秒启动延迟?| v1.1.7 | 代码已修正，已确?|
| **BUG-069** | 解决 Monochrome 皮肤下设置开关选择 size="small" 时卡死在左侧无法正常拨动和交互的问题?| v1.3.3 | 代码已修正，已确?|
| **BUG-068** | ?`cyberpunk` 皮肤的浅色模式下，背景显示为深色导致文字不可读?| v1.1.5 | 代码已修正，已确?|
| **BUG-067** | ?`modern-flat` 皮肤下，调节 Shadow (阴影) 滑块无任何视觉反馈?| v1.1.3 | 代码已修正，已确?|
| **BUG-066** | ?`cyberpunk` 皮肤下，调节 Speed (速度) 滑块无效果，且面板外发光被裁剪?| v1.1.3 | 代码已修正，已确?|
| **BUG-065** | 置顶活跃出口节点点击轮换时未能限制在当前搜索、过滤及排序后的候选节点子集范围内?| v1.2.1 | 代码已修正，已确?|
| **BUG-064** | 三选一/分段选择器活动态文字颜色硬编码?`#1E1200` 导致浅色/Cyberpunk模式下文字不协调或无法阅读?| v1.1.3 | 代码已修正，已确?|
| **BUG-063** | ?`monochrome` 皮肤下，基础设置的三个开关无法拨动，且视觉尺寸过小、难以拨动?| v1.1.6 | 代码已修正，已确?|
| **BUG-062** | Original 风格下底部流量小卡片颜色 and 选中换肤按钮圆角不随滑块动态调节的问题?| v1.1.1 | 代码已修正，已确?|
| **BUG-057** | 导入新订阅后节点列表加载显示空白?| v4.1.0 | 代码已修正，已确?|
| **BUG-056** | 程序退出时残留内核/服务工作区孤儿进程?| v1.1.0 | 代码已修正，已确?|
| **BUG-055** | 托盘 `proxy_cache` 锁生命周期过长被拦截?Git 构建失败?| v3.0.6 | 代码已修正，已确?|
| **BUG-054** | 多套控件皮肤风格切换及滑块参数重定义与窗口裁剪?| v1.1.0 | 代码已修正，已确?|
| **BUG-053** | 首次导入或更新订阅自动切换激活及防空值误报?| v3.0.4 | 代码已修正，已确?|
| **BUG-052** | 自动连切背景运行时静默不弹窗及存活探活阈值放宽?| v1.1.0 | 代码已修正，已确?|
| **BUG-051** | 流量小卡片上传下载总量数值常驻为 0?| v3.0.4 | 代码已修正，已确?|
| **BUG-050** | CPU 资源消耗显著高于原版（常驻组件后台渲染耗能，Traffic可见性响应缺失）?| v3.0.3 | 代码已修正，已确?|
| **BUG-049** | 活性监控探活定时器?proxies 变动时反复重置?| v3.0.2 | 代码已修正，已确?|
| **BUG-048** | 无边框模式下窗口边缘磁吸及自适应吸附与脱离功能丢失?| v3.0.1 | 代码已修正，已确?|
| **BUG-047** | 本地 pre-push 钩子运行 cargo clippy 遇到测试模块中的 `.unwrap()` 报错拦截推送?| v3.0.1 | 代码已修正，已确?|
| **BUG-046** | 隐身模式下，单击窗口非避让区域无法正常恢复原生标题栏（点击唤醒失效）?| v3.0.0 | 代码已修正，已确?|
| **BUG-045** | 最窄窗口下，底部的流量图和数据卡片不够紧凑，且 Canvas 折线图缩放模糊?| v2.0.7 | 代码已修正，已确?|
| **BUG-044** | 本地导入节点卡片高亮激活逻辑未对齐，第二行元数据格式不统一?| v2.0.7 | 代码已修正，已确?|
| **BUG-043** | 全自动订阅格式识别、去重复用与合并编译支持?| v1.1.0 | 代码已修正，已确?|
| **BUG-042** | 支持本地通用?YAML 订阅链接解析（支?VMess, SS, Trojan, VLESS, Hysteria2 ?HTTP 格式）?| v2.0.4-full | 代码已修正，已确?|
| **BUG-041** | 启动或加载页面时，主页节点的表格（ProxyGroups）空白，需手动拉伸窗口方显现?| v2.0.4 | 代码已修正，已确?|
| **BUG-040** | 发行预检时强杀冲突进程，导?host 上的原版客户端被误杀?| v2.0.0 | 代码已修正，已确?|
| **BUG-039** | 出口节点延迟变慢/超时，无法自动连切，且背景测速间隔不够自适应?| v2.0.0 | 代码已修正，已确?|
| **BUG-038** | 设置页展开后遮挡了右上角关闭按钮，导致无法关闭设置窗口而被锁死（v1.1.9 修复）?| v1.1.9 | 代码已修正，已确?|
| **BUG-037** | 窗口始终置顶功能移除设置页开关，且设置页展开时隐藏顶部栏置顶图钉?| v1.1.7 | 代码已修正，已确?|
| **BUG-036** | 设置页展开后遮挡了右上角关闭按钮，导致无法关闭设置窗口而被锁死（v1.1.6 修复）?| v1.1.6 | 代码已修正，已确?|
| **BUG-035** | 置顶图钉引入后，活跃出口状态条发生遮挡，且无法自适应定位和右对齐?| v1.1.4 | 代码已修正，已确?|
| **BUG-034** | 激?切换订阅时无法自动触发测速、排序并选中最快可用节点?| v2.0.1 | 代码已修正，已确?|
| **BUG-033** | 导入 3 个或更多订阅时，侧边栏订阅列表没有滚动条?| v1.1.4 | 代码已修正，已确?|
| **BUG-032** | 流量接管模式切换时滑块先跳回“手动”再到目标模式，产生视觉闪烁与提权打扰?| v1.1.4 | 代码已修正，已确?|
| **BUG-031** | 开启系统代理时修改 Mixed Port，保存后滑块异常跳回手动，且系统代理配置未同步?| v1.1.4 | 代码已修正，已确?|
| **BUG-030** | 开机启动若默认是系统代理，滑块 UI 依然异常显示为“手动模式”?| v1.1.4 | 代码已修正，已确?|
| **BUG-029** | 当前活跃出口节点状态栏排列混乱，缩小时文本易错位折行?| v1.1.4 | 代码已修正，已确?|
| **BUG-028** | 开机启动时因网络未就绪自动更新失败后增加重试机制，且缺省更新间隔兜底为 24 小时?| v1.1.4 | 代码已修正，已确?|
| **BUG-027** | 路径控制下右键选择设为全局直连或代理分流，无实际规则写入和分流改变?| v1.1.4 | 代码已修正，已确?|
| **BUG-026** | 界面控件（主按钮、双滑块、三选一滑块）尺寸显得过于偏大?| v1.1.2 | 代码已修正，已确?|
| **BUG-025** | 滑块选项文本只有在被选中时才是粗体，与普通按钮常驻粗体不统一?| v1.1.2 | 代码已修正，已确?|
| **BUG-024** | Windows 标题栏数学粗体字符在部分 Windows 系统下显示为方块或乱码?| v1.1.2 | 代码已修正，已确?|
| **BUG-018** | 底部指标卡片整体偏大，且在折行及小屏状态下字体比例失调?| v1.1.2 | 代码已修正，已确?|
| **BUG-017** | 置顶当前活跃出口节点状态栏无法正确获取并渲染节点的协议?IP:port?| v1.1.7 | 代码已修正，已确?|
| **BUG-016** | 代理模式在非规则模式下隐藏表头导致定位、延迟排序按钮丢失，本地联网死锁?| v1.1.1 | 代码已修正，已确?|
| **BUG-015** | 配置文件激活时与未缓存?activateSelected 发生 useEffect 联动，导致无限循环更新?| v1.0.9 | 代码已修正，已确?|
| **BUG-014** | 在全局代理（Global）模式下，用户选择节点无法控制流量出口?| v1.0.4 | 代码已修正，已确?|
| **BUG-013** | 自动检查更新选项开启（auto_check_update）引发的冗余后台行为?| v1.0.3 | 代码已修正，已确?|
| **BUG-012** | 默认未开启自动轻量化模式（Memory Optimization）导致后台挂机占用内存较高?| v1.0.3 | 代码已修正，已确?|
| **BUG-011** | 默认未开启代理守护（Proxy Guard）导致系统代理易被篡改或静默失效?| v1.0.3 | 代码已修正，已确?|
| **BUG-010** | 便携版绿色压缩包?Release 时丢失或包含旧版本二进制的问题?| v0.3.2 | 代码已修正，已确?|
| **BUG-009** | 启动时提示有 Clash Verge 新版本并引导升级从而覆盖客户端的问题?| v0.0.1 | 代码已修正，已确?|
| **BUG-008** | 设置页面右上角设置齿轮按钮与底部的节点名?延迟重合且无法点击?| v0.2.0 | 代码已修正，已确?|
| **BUG-007** | 订阅管理中偶现没有名字的空行订阅项，重启后仍反复出现?| v0.2.0 | 代码已修正，已确?|
| **BUG-006** | 界面清除毛玻璃不彻底，部?Aero 样式变量残留导致滚动漏光重叠?| v0.2.0 | 代码已修正，已确?|
| **BUG-005** | 表格最外侧左、右竖边框线?5px double 且左侧表格线缩进空缺十几个像素?| v0.2.0 | 代码已修正，已确?|
| **BUG-004** | 代理模式在非规则模式下隐藏表头导致定位、延迟排序按钮丢失?| v0.2.0 | 代码已修正，已确?|
| **BUG-003** | DNS 覆写默认未开启，导致国内直连 and 防泄露体验不佳?| v1.0.3 | 代码已修正，已确?|
| **BUG-002** | 默认开?IPv6 且缺乏界面控制，可能导致某些网络环境?DNS 泄露或代理分流异常?| v1.0.3 | 代码已修正，已确?|
| **BUG-001** | 开机自启非管理员权限下修改后台静默失败且前?Switch 状态脱节?| v1.0.2 | 代码已修正，已确?|
| **BUG-177** | Monitor故障自愈补全error log | v1.5.9 | 用户已确认 |
| **BUG-178** | 协议章节编号二十九→三十/三十一 | v1.5.9 | 用户已确认 |
| **BUG-179** | AsyncHandler::spawn闭包补全错误处理 | v1.5.9 | 用户已确认 |
| **BUG-180** | injectBatchResults补传elapsed元数据 | v1.5.9 | 用户已确认 |
| **BUG-181** | handle_window_close参数重命名消除遮蔽 | v1.5.9 | 用户已确认 |
| **BUG-182** | enable_auto_light_weight_mode注释澄清 | v1.5.9 | 用户已确认 |
| **BUG-183** | DelayResults统一使用SmartString别名 | v1.5.9 | 用户已确认 |
| **BUG-184** | 故障自愈后重置last_check_time | v1.5.9 | 用户已确认 |
| **BUG-185** | CHANGELOG补录默认值变更 | v1.5.9 | 用户已确认 |
| **BUG-186** | CI rustfmt格式化检查配置确认 | v1.5.9 | 用户已确认 |
| **BUG-187** | 协议三十/三十一条文排列确认 | v1.5.9 | 用户已确认 |
| **BUG-175** | 关闭窗口即刻进入轻量模式 | v1.5.9 | 用户已确认 |
| **BUG-169** | Speed Test 超时硬限制导致 Error 显示 | v1.5.5 | 用户已确认 |
| **BUG-170** | 自启开关时序竞态与 UAC 提权修复 | v1.5.7 | 用户已确认 |
| **BUG-172** | getDelayFix || 1e6 导致超时节点显示 Error | v1.5.8 | 用户已确认 |
| **BUG-173** | Admin 模式 schtasks 缺 /RU "" 修复 | v1.5.7 | 用户已确认 |
| **BUG-174** | Retro-3D 深色 default 卡片文字色 #FFE082 → #2C1F03 | v1.5.9 | 用户已确认 |
| **BUG-176** | 后台 Monitor 测速结果不回传前端 UI | v1.5.9 | 用户已确认 |
| **BUG-146** | Inconsistent Property Access for Allow LAN Switch State | v1.5.5 | 用户已确认 |
| **BUG-188** | destroy_main_window/show_main_window 返回值被丢弃 | v1.6.0 | 用户已确认 |
| **BUG-189** | BUG-173 修复 /RU "" 被错误地同时加入 create_task 和 create_task_elevated，导致管理员和非管理员均无法开启自启 | v1.6.0 | 用户已确认 |
| **BUG-190** | Infinite Self-Healing Auto-Select Loop on Fast Nodes (< 30ms) | v1.6.3 | 用户已确认 |
| **BUG-191** | Redundant reqwest::Client Creation per Request | v1.6.3 | 用户已确认 |
| **BUG-193** | Semaphore Permit Inflation under RejectPolicy::New | v1.6.3 | 用户已确认 |
| **BUG-194** | Dead Sockets Kept in the IPC Connection Pool | v1.6.3 | 用户已确认 |
| **BUG-195** | RAM Buffering of Large File Updates | v1.6.3 | 用户已确认 |
| **BUG-196** | Uncompiled Dead Code (speed_task.rs) | v1.6.3 | 用户已确认 |
| **BUG-197** | Infinite Background Polling Loop in useConnectionData | v1.6.3 | 用户已确认 |
| **BUG-199** | Unhandled Tauri Listener Promise Rejection in useCustomTheme | v1.6.3 | 用户已确认 |
| **BUG-200** | Passive Cache Eviction Leak in DelayManager | v1.6.3 | 用户已确认 |
| **BUG-201** | High-Frequency Tauri IPC Polling in useVisibility | v1.6.3 | 用户已确认 |
| **BUG-192** | Unthrottled Core Updater IPC Progress Emitter | v1.6.3 | 用户已确认 |
| **BUG-198** | Unhandled Tauri Listener Promise Rejection in useWindowSnap | v1.6.3 | 用户已确认 |


