## v1.9.0

### 🐞 Fixed Bugs

- **BUG-257~259 轻量挂机模式 Rust 后端与内核极致节电与资源 GC 优化**：
  - **自适应健康探测周期 (BUG-257)**：在轻量模式下，将常驻监测线程的探测间隔时间放宽至 60 秒一次，并在恢复前台时瞬间唤醒重置，大幅减少后台空闲时的 CPU 唤醒与网卡/DNS 活动。
  - **残留 TCP 连接垃圾回收 (BUG-258)**：进入轻量模式的瞬间强制清空内核中所有残留的活动及空闲连接，使 Mihomo 内核回归超低功耗的挂机状态。
  - **WebSocket 数据订阅自动熔断 (BUG-259)**：在轻量模式下彻底断开并清空后台的 WebSocket 数据订阅，避免主窗口销毁后多余的 JSON 反序列化及 IPC 通信资源开销。

- **BUG-260~261 前端 WebView2 内存膨胀与泄露问题深度修复**：
  - **Web Worker 单例复用与生命周期泄露修复 (BUG-260)**：将流量监控 Hook 中的 Web Worker 从“频繁 terminate 与重新 new 实例化”改为“单例长连接复用，发 stop 消息暂停采样”。完美消除了 WebView2 频繁启停线程带来的 V8 隔离堆及系统句柄泄露，实现常驻内存稳定不攀升。
  - **设置抽屉回归 CSS 平移隐藏机制 (BUG-261)**：撤销设置抽屉在大/小视口拉伸以及开关时采用 React 物理卸载 `{drawerOpen && ...}` 的逻辑，回退到 1.8.2 的经典 CSS `transform` 与 `pointer-events` 平移移出隐藏逻辑。从根本上杜绝了组件卸载时 Emotion 动态注入 `<style>` 标签的无限累加泄露以及 MUI 组件 unmount 监听器残留泄露，恢复 100% 的内存稳定性。

---

## v1.8.9

### 🐞 Fixed Bugs

- **BUG-256 自动轻量模式关闭窗口崩溃与托盘失效修复**：
  - 将窗口关闭（CloseRequested）、聚焦（Focused）和销毁（Destroyed）等生命周期事件的捕获逻辑从 `app.run` 移入 `tauri::Builder` 的 `on_window_event` 拦截器中，确保在窗口真正被 OS / 默认行为销毁前完成隐藏，并安全地在异步任务中执行轻量化转换，彻底解决了 Windows / Tao 引擎下关闭窗口导致的 `cannot move state from Destroyed` 崩溃 Panic。

---

## v1.8.8

### 🐞 Fixed Bugs

- **BUG-240~255 16项审计遗留代码规范与安全性改进修复**：
  - **组件双重触发与内存泄漏优化 (BUG-240, 241, 243, 247)**：修复静默启动期间因窗口尺寸初始为0引起的 `getProxies` 重复触发；确保在迷你模式下完全卸载设置抽屉，杜绝 WebSocket 后台高频连接开销；为空闲计时器在 `resetIdleTimer` 回调中采用实时尺寸校验以防止过期状态缓存导致的误隐藏问题；在 `_layout` 状态更新中使用异步非阻塞延迟以消除 React render 周期中的同步状态更新警告。
  - **窗口自适应阈值对齐与多余无用代码清理 (BUG-242, 244, 245, 246)**：使 window-provider 中的 chrome 隐藏判定阈值与 React 布局的迷你模式阈值均对齐为 `285px`，消除窄宽度下的黑边与无标题栏排版错位；清理 `base-page`、`proxy-groups` 与 `layout-dialogs` 中声明但未使用的多余变量或阴影变量。
  - **严格 TypeScript 去 `any` 强类型规约 (BUG-248, 249, 250, 251, 252, 253, 254, 255)**：对 `app-data-context`、`debounce` 工具类、`cmds` 原始代理组转换器、延迟测试定时器 `timerId`、i18n 语言加载 reduce 累加器、Mihomo 配置中的 `nameserver-policy` 字典、流量监控 validator 验证以及 `debugLog` 的参数类型进行重构，全面消除 `any` 类型声明，使用强类型或安全的 `unknown` 进行类型收窄，保证项目严苛编译校验通过。

---

## v1.8.1

### 🐞 Fixed Bugs

- **BUG-217~238 22项系统代码安全与样式兼容Bug修复**：
  - **测速逻辑与定时器漏孔 (BUG-217~222, 224)**：在配置切换时实现自动清理与彻底关闭后台测速定时器；利用 Promise 锁对选点行为执行严格串行化，避免临时闪连与极速终选对后端造成的并发请求竞态；吞掉延迟测试后的 unhandled rejections，并重新对齐了顶栏的 theme/skin 响应依赖。
  - **内核底层并发安全与 SSRF 防护 (BUG-225~232)**：修复 TcpStream 读取空切片导致测速永远 0ms 或超时的逻辑缺陷；将 Boa 解释器及外部验证命令输出封装至 tokio 超时机制及循环指令限制下，彻底避免死循环卡死 tokio 主线程；针对订阅/图片引入 ToSocketAddrs DNS 动态解析，并将解析出的 IP 地址与 IPv6 link-local 和 ULA 敏感子网对比，防范 SSRF DNS 重新绑定绕过；添加单例冲突时的 native MessageBox 错误弹窗提示；对订阅下载加入 10MB 物理上限及流式传输限制，防止 OOM 崩溃。
  - **界面层级特异性与高 DPI 裁剪 (BUG-223, 233~238)**：使用 `html[data-control-skin]` 提升 opaque 对话框遮罩的权重，消除毛玻璃下的文本重合穿透；修复磨砂卡片在浅色模式下的低对比度问题；移除全局 scss 的 `!important` scrollbar 限制以支持局部 `scrollbar-width: none` 连接表，并绑定 scrollbar CSS 变量；将 compact 代理项行高从 20px 统一扩容为 24px 并去除高度死锁，防止 Windows 高 DPI 缩放下的文字和延迟物理裁剪；对侧栏菜单提供 Hover-dependent 滚动条显隐机制。

---

## v1.8.0

### 🐞 Fixed Bugs

- **彻底回滚严格 CSP 安全策略并恢复界面排版正常显示**：
  - 将 `tauri.conf.json` 中的 `csp` 配置彻底回滚为 `null`，移除内容安全策略对 Emotion 动态样式注入的拦截。
  - 将 `assetProtocol.scope.allow` 的本地资源读取范围重新放开为 `["**"]`，恢复各目录下本地字体、图标等资源的正常读取协议许可。
  - 清理并还原了前端代码里为了“应付 CSP 拦截”临时加入的所有内联 `style` 绕路样式，将窗口控制、顶栏图标、测速加载状态及信号图标完美退回至标准的 Emotion/MUI `sx` 属性，彻底解决了界面排版在 WebView2 下容易崩塌塌陷的问题。

---

## v1.7.9

### 🐞 Fixed Bugs

- **修复系统测速模式切换通信接口报错与构建一致性校验**：
  - 修复前端 `FindProcessMode` 选项为首字母大写（`'Strict' | 'Always' | 'Off'`）而 Rust 后端只认纯小写（`'strict' | 'always' | 'off'`）导致的切换模式接口通信解析失败问题。
  - 同步 `tauri-plugin-mihomo` 插件的前端 package.json 版本号从 0.5.2 至 0.5.4，以对齐 Rust backend 的版本配置。
  - 在 `tauri.conf.json` 中将 `beforeBuildCommand` 恢复为 `pnpm web:build`，确保打包时包含完整的 TypeScript 类型校验（`tsc --noEmit`），防范未来潜在的类型不匹配隐患。

---

## v1.7.8

### 🐞 Fixed Bugs

- **彻底修复严格 CSP 下的活动出口节点与桌面端样式特异性覆盖冲突 (BUG-215, BUG-216)**：
  - 彻底解决了由于 Material-UI `<Chip>` 内部样式高优先级特异性覆盖导致测速信号钻石图标变大（占据整个顶栏）以及后台测速菊花加载圈失效的问题，改用原生的 inline `style` 属性绕过 CSP 与 CSS 特异性重写（1000 权重覆盖）。
  - 修复窄版面（640px 窗口宽度内）下当前活跃出口节点文字长度拉伸导致的顶部状态条拉伸变形，追加合理的响应式 `maxWidth`（`120px`/`180px`）自适应限制。
  - 修复代理页面分组表头列表数据丢失且折叠交互失效的缺陷，并在 `use-render-list.ts` 中恢复代理列数设置以符合单列/多列表格用户配置。

---

## v1.7.7


### 🐞 Fixed Bugs

- **从根本上修复严格CSP下的样式失效与活动节点布局崩溃 (BUG-215, BUG-216)**：
  - 修正了 `tauri.conf.json` 中的 CSP 策略配置，显式放行本地 `tauri:` 与 `asset:` 方案源。彻底解决了由于 WebView2 不判定本地自定义协议同源而拦截静态 CSS 与 Emotion 动态注入样式，导致主界面退化为灰白色且活动节点卡片失去高度约束塌陷的问题。
  - 完全还原并清除了 1.7.6 中为了回避 CSP 拦截而作出的 23 处内联 `style` 临时绕路样式，将窗口控制按钮、顶部置顶及设置齿轮图标等全部重构对齐回标准的 MUI `sx` 属性，保持代码库的纯正与规范。

---

## v1.7.6

### 🐞 Fixed Bugs

- **补全严格CSP下右上角控制按钮及窗口边框图标尺寸约束 (BUG-215)**：
  - 修复无边框和有边框模式下，右上角的设置齿轮、图钉固定、关闭按钮以及窗口控制按钮（最小化、最大化、关闭）由于缺少显式内联 `width` / `height` 导致在 CSP 下体积膨胀至 300px 撑大整个顶部控制栏的缺陷。
  - 为 `_layout.tsx` 中的 `PushPinRounded`、`CloseRounded`、`SettingsRoundedIcon` 增加显式 `width: '20px', height: '20px'` 内联样式。
  - 为 `window-controller.tsx` 中的控制按钮（`Close`、`Minimize`、`FilterNone`、`CropSquare`）增加显式内联宽高限制。

---

## v1.7.5

- 发行版本编译对齐与常规底层库更新。

---

## v1.7.4

### 🐞 Fixed Bugs

- **修复严格CSP下活跃连接节点状态条布局崩溃 (BUG-215)**：
  - 针对在启用严格 CSP 的环境下，由于 Emotion 动态样式注入延迟导致 MUI SvgIcon/loading 旋转图标尺寸变大（300px）从而撑破顶层布局的缺陷进行修复。
  - 为所有信号图标及 Loading 圈增加显式内联 `sx` 尺寸限制（`12px`），并对卡片及节点名标签增加 `minWidth: 0` 和 `overflow: 'hidden'` 保证长节点名干净截断不撑开布局。

---

## v1.7.3

### 🐞 Fixed Bugs

- **还原设置抽屉物理裁剪与响应式隐藏设计 (BUG-205, BUG-206)**：
  - 撤销对设置抽屉换行布局及最小宽度的修改，完全恢复为原生不换行布局与 `minWidth: 0`，以恢复在默认 270px 窄窗口下连接面板被正常物理裁剪遮挡的原有设计（BUG-205）。
  - 撤销对语言与皮肤选择器显示隐藏高度阈值的修改，完全恢复为原始的 `@media (max-height: 830px)` 响应式裁剪规则，确保窄高度窗口下的布局整洁（BUG-206）。

---

## v1.7.2

### 🐞 Fixed Bugs

- **多皮肤排版与样式调优 (BUG-212, BUG-213, BUG-214)**：
  - 修复 Original 皮肤 accent 颜色未跟滑块动态绑定的问题。
  - 修复 Monochrome 皮肤在深色模式下 cards/panels 缺乏对比度背景色融合的问题。
  - 修复 Cyberpunk 和 Monochrome 皮肤中 small 尺寸开关没有适配比例的尺寸溢出问题。
- **后台死锁与启动阻塞修复 (BUG-209, BUG-210)**：
  - 将 `init_startup_script` 的执行迁移到后台 `tokio::spawn` 异步线程，防止启动脚本长时间执行导致的主 UI 线程卡死/黑屏。
  - 解决 Service Manager 初始化提权重装服务时的同步阻塞，重装过程采用 `tokio::task::spawn_blocking` 以彻底避免 Mutex 死锁和启动挂起。
- **WebView 窗口销毁性能优化 (BUG-211)**：
  - 关闭窗口时，如果未开启“自动轻量化模式”，改为隐藏 WebView 并调用 WebView2 内存回收（Low 级别），不再无条件销毁整个窗口，从而极大地提升了二次打开窗口的速度，并保留了前端的临时交互状态。
- **便携模式资源协议范围放宽 (BUG-207)**：
  - 放宽 asset 协议的 scope，在 `tauri.conf.json` 中允许加载 `$EXE_DIR/**` 和 `$RESOURCE_DIR/**`，解决便携绿色版中 profile 缓存图片加载失败显示破损的问题。
- **主题即时更新事件补发 (BUG-208)**：
  - 修复 patch 修改主题设置后未能标记 config 变化的问题，确保即时触发前端 RefreshVerge 广播重新应用 CSS，不需要再手动重启程序。

---

## v1.7.1

### 🐞 Fixed Bugs

- **CSP 字体拦截修复**：将 `Outfit` 字体通过 `@fontsource/outfit` 本地化打包引入项目，完全摆脱对外部 Google 字体库（`fonts.googleapis.com`/`fonts.gstatic.com`）的依赖，解决了在启用严格 CSP 的系统环境下因字体缺失导致的主界面 3D 拟物排版与 Label 崩溃混乱的问题。
- **修复 Clippy 编译警告**：修复了 Rust 编译器下的 `unused_trait_names` 和 `unused_async` 警告以顺利通过 pre-push 编译质量拦截。

---

## v1.7.0

### 🐞 Fixed Bugs

- **修复 Rust 编译错误**：在 `window_manager.rs` 添加缺失的 `use std::pin::Pin`，清理多余 import。
- **修复 TypeScript import 类型错误**：`app-data-context.ts` 中 `BaseConfig`/`ProxyProvider`/`Rule`/`RuleProvider` 改为 `import type`，防止运行时因无法解析外部包类型导致前端白屏。
- **恢复滚动条 `!important` 样式**：`proxy-groups.tsx` 中 `scrollbarWidth` 加回 `!important`，确保各浏览器滚动条隐藏优先级。
- **修复代理组空回退逻辑**：`use-render-list.ts` 移除 GLOBAL 回退兜底，groups 为空时渲染空列表，避免将 GLOBAL.all 中的组名列表误作节点渲染。

---

## v1.6.8

### 🔒 Security Hardening

- **CSS Injection 安全加固**：后端 URL 验证 + 前端 Content-Security-Policy 双重防御。
- **加固 deny.toml**：禁止通配符依赖、添加许可证白名单、标注所有 RUSTSEC 通报。
- **锁定 4 个 Git 依赖到 commit hash**：构建完全可重现，消除供应链风险。
- **消除 `as any` 类型安全违规**：启用 `no-explicit-any` ESLint 规则，全量修复类型擦除。
- **替换 8 处空 catch 块**：补充 `console.warn` 日志，消除静默吞异常。

### 🚀 Optimizations

- **内联 sysinfo 插件**：移除独立 `tauri-plugin-clash-verge-sysinfo` crate，将功能内联到 `src-tauri/src/utils/sysinfo.rs`，减少编译产物体积。
- **自适应窗口防抖**：空闲用户窗口操作即时响应，高频点击渐进延迟，消除 UI 卡顿。
- **添加细粒度 ErrorBoundary**：代理列表、连接管理、路由区域各有独立错误边界，防止单点崩溃白屏。
- **清理 package.json**：移除 45 个一次性 scratch 脚本，减少依赖污染。

---

## v1.6.7

### 🔒 Security Hardening

- **CSS Injection 安全加固**：后端 URL 验证 + 前端 Content-Security-Policy 双重防御。
- **加固 deny.toml**：禁止通配符依赖、添加许可证白名单、标注所有 RUSTSEC 通报。
- **锁定 4 个 Git 依赖到 commit hash**：构建完全可重现，消除供应链风险。

### 🐞 Fixed Bugs

- **消除 `as any` 类型安全违规**：启用 `no-explicit-any` ESLint 规则，全量修复类型擦除。
- **替换 8 处空 catch 块**：补充 `console.warn` 日志，消除静默吞异常。

### 🚀 Optimizations

- **内联 sysinfo 插件**：移除独立 `tauri-plugin-clash-verge-sysinfo` crate，将功能内联到 `src-tauri/src/utils/sysinfo.rs`，减少编译产物体积。
- **自适应窗口防抖**：空闲用户窗口操作即时响应，高频点击渐进延迟，消除 UI 卡顿。
- **添加细粒度 ErrorBoundary**：代理列表、连接管理、路由区域各有独立错误边界，防止单点崩溃白屏。
- **清理 package.json**：移除 45 个一次性 scratch 脚本，减少依赖污染。

---

## v1.6.6

### 🔒 Security Hardening

- **CSP 内容安全策略**：设置严格 CSP，防止 XSS 攻击加载外部脚本。
- **权限最小化**：移除 shell 执行权限、收紧 FS/Asset 作用域、HTTP 插件域名白名单。
- **URL 输入验证**：open_web_url 协议验证、ZIP 解压防路径遍历、SSRF 防护（订阅/图标 URL）。
- **JS 沙箱加固**：冻结原型链防止逃逸，JSON 传递 name 参数替代字符串拼接。
- **运行时防护**：启用 release overflow-checks，YAML 解析 50MB 大小限制。
- **安全审计**：完成全量代码安全评估，修复 15/18 项安全问题（4 项架构级问题延期处理）。

### 🐞 Fixed Bugs

- Fix clippy `missing_const_for_fn` warning in service.rs.

---

## v1.6.5

### 🚀 Optimizations

- Item A.1: Optimized Connection Table rendering by comparing raw row data index/origin instead of TanStack Table wrapper references to prevent redundant row re-renders.
- Item A.2: Memoized LogItem component and cached Regex compilation to prevent duplicate compilations across visible logs.
- Item A.3: Implemented a global singleton resize listener in use-window-width to avoid registering O(N) resize event listeners for proxy cards.
- Item A.4: Replaced useVerge hook subscription with synchronous config cache reading getPreloadConfig in useProxyDelayState hook to reduce Query observers.
- Item A.5: Added refresh token trigger in use-filter-sort useMemo dependencies to ensure proxy sorting updates immediately upon latency checks.
- Item B.1: Replaced tokio::spawn loop in latency sweep with a fixed worker pool of size 32 using lock-free AtomicUsize indexing in monitor.rs.
- Item B.2: Moved service installation and IPC waiting logic outside the SERVICE_MANAGER Mutex lock to resolve GUI hangs.
- Item B.3: Simplified tray proxy selection sync to a direct zero-cost NO-OP for the static tray menu.
- Item B.4: Replaced 200ms sleep busy loop in timer.rs with a tokio::sync::Notify awaiter in resolve/mod.rs.
- Item B.5: Replaced 1-second busy loop in background monitor with a tokio::select! block waiting on a 15-second timer or PROFILE_SWITCH_NOTIFY.
- Item C.1: Implemented write_file_if_changed in profiles.rs to compare content before writing config, saving I/O overhead.
- Item C.2: Configured reqwest Client with connection pooling limits to avoid socket churn under high concurrency.
- Item C.3: Wrapped backup output file in std::io::BufWriter to aggregate small metadata writes.
- Item D.1: Integrated useVisibility event listeners to disable system state polling when the window is hidden or minimized.

### 🐞 Fixed Bugs

- Fix all ESLint warnings and React Compiler/Purity warnings in layout and settings components (active-node-card, basic-settings-card, help-menu-button, profile-import-card, routing-preference-card, takeover-mode-card, theme-settings-card, style-helpers).

---

## v1.6.3

### 🚀 New Features

- Optimize auto-select health check sensitivity: increased timeout from 500ms to 1000ms and failure threshold from 3 to 5 to prevent high-frequency node switching.

### 🐞 Fixed Bugs

- Fix BUG-190: Implement exponential backoff cooldown (60s to 15m) for self-healing auto-select daemon when no nodes >= 30ms are available.
- Fix BUG-191: Reuse reqwest::Client connection pool in Mihomo struct to prevent socket and CPU resource exhaustion.
- Fix BUG-192: Throttle core updater download progress emissions to only when the integer percentage changes.
- Fix BUG-193: Replace semaphore add_permits(1) with a PoolPermit enum to prevent connection pool capacity inflation under RejectPolicy::New.
- Fix BUG-194: Proactively check and clean up invalid/dead sockets inside the connection pool.
- Fix BUG-195: Stream-decompress zip/gz packages directly to disk during updates to avoid buffering large files (~40MB) in RAM.
- Fix BUG-196: Delete uncompiled dead code speed_task.rs.
- Fix BUG-197: Add an active mounting flag to useConnectionData hook to prevent infinite background polling on unmount.
- Fix BUG-198 / FEAT-DELETE-001: Completely remove the window snap (磁吸) feature and its associated hook.
- Fix BUG-199: Add null safety checks for unlistenPromise during useCustomTheme cleanup.
- Fix BUG-200: Add a 2-hour active GC timer to DelayManager cache to clean up expired entries.
- Fix BUG-201: Replace 1-second setInterval polling in useVisibility with 100% event-driven native Tauri listeners.

---

## v1.6.0

### 🐞 Fixed Bugs

- Fix BUG-188: Handle discarded return values of destroy_main_window/show_main_window by logging or returning errors properly to prevent potential silent failures in main window lifecycle management.
- Fix BUG-189: Remove incorrect `/RU ""` from `create_task` that was erroneously added in the BUG-173 fix. The `/RU ""` parameter conflicts with the XML `<UserId>` when running as admin, causing `schtasks` to fail with "未指定的错误". Only `create_task_elevated` (UAC elevation path) needs `/RU ""` because the process context is lost after elevation. `create_task` runs directly without elevation and the XML provides the UserId, so `/RU ""` is unnecessary and harmful.

---

## v1.5.9

### 🚀 New Features

- Default theme changed to dark mode for first-time users, improving out-of-the-box experience in low-light environments.
- `allow-lan` now defaults to `true`, enabling LAN device proxying without manual configuration.

### 🐞 Fixed Bugs

- Fix BUG-174: Resolve text color contrast issue in Retro-3D dark mode update log cards. Changed text color from low-contrast gold `#FFE082` to high-contrast dark bronze black `#2C1F03`.
- Fix BUG-175: Window close button now immediately enters lightweight mode after hiding the window, eliminating the 10-second delay. Removed obsolete timer chain and listener code.
- Fix BUG-177: Monitor health-check error logs in the fault-recovery branch were being silently dropped. Now properly logged via `logging!` macro.
- Fix BUG-183: `FrontendEvent::DelayResults` variant now uses `SmartString` instead of `std::string::String` for consistency with the rest of the codebase.

### 🚀 Optimizations

- Enhance BUG-176: Backend Monitor speed-test results are now pushed to the frontend UI in real-time via Tauri events (`verge://backend-delay-results`), eliminating the need for manual refresh to see auto-selected node latencies.
- Hardened AsyncHandler::spawn closures with error logging to prevent silent failures in background tasks (BUG-179).
- `injectBatchResults` now preserves `elapsed` metadata when batch-writing delay results (BUG-180).
- Reset `last_check_time` after Monitor fault recovery to prevent excessive immediate re-checks (BUG-184).
- Added CI code-formatting consistency check job (BUG-186).

---

## v1.5.7

### 🐞 Fixed Bugs

- Fix BUG-170: Fix autostart switch timing race, eliminate unnecessary UAC elevation prompts for standard user tasks, and resolve early-return syntax bug in configuration patching which caused failed config drafts to stay dirty in memory and get applied in subsequent unrelated edits.
- Fix BUG-173: Add /RU "" parameter to schtasks /Create commands to bypass the user password verification prompt when creating auto-launch tasks, resolving task creation failures in Administrator mode.
- Fix BUG-171: Resolve auto-select hanging issue on profile switch/import, improve Windows named pipe busy retries under concurrency, clear stale connection handles on restart, and gracefully map non-JSON API errors.

## v1.5.3

### 🐞 Fixed Bugs

- Fix BUG-138: Redesign lightning speed test button to test only visible (filtered) nodes instead of all nodes, using front-end delayManager concurrency for per-node shimmer animation, and prevent auto-switching active nodes.
- Fix BUG-139: Fix traffic metrics cards layout nesting error under both wide and narrow window modes.

### 🚀 Optimizations

- Optimize: Unify the default concurrency limit to 36 to match the configuration default.
- Refactor: Format imports and simplify logger calls in Rust backend `enhance/mod.rs`.

## v1.5.2

### 🐞 Fixed Bugs

- Fix BUG-138: Fix lightning speed test button failure caused by empty test URL string.
- Fix: Use pure CSS to fix narrow window folding and wide window equal-width layout.

## v1.5.1

### 🐞 Fixed Bugs

- Fix BUG-121/BUG-138: Use frontend `delayProxyByName` API path to prevent lock conflicts.
- Fix: Remove "WinAero" prefix from base settings card title.

## v1.5.0

### 🐞 Fixed Bugs

- Fix BUG-121: Resolve issue where the active outbound node is permanently occupied by a fixed node (advertising/dummy node) after importing a new subscription, and the auto-optimization process never triggers. Root cause: backend monitor thread's `trigger_backend_auto_select` locks `AUTO_SELECT_RUNNING` on the OLD config, and the frontend's `triggerAutoSelectAndRefresh` silently swallows the `AUTO_SELECT_BUSY` error without retry, causing subsequent `refreshProxy`, `setHeadState(sortType=1)`, and Fallback timer to never execute. Fix: added AUTO_SELECT_BUSY retry (up to 5 attempts, 600ms interval) in `triggerAutoSelectAndRefresh`, and moved `refreshProxy`/`setHeadState`/Fallback timer outside the try-catch block to ensure they always execute.
- Fix BUG-138: Resolve issue where clicking the lightning cursor (batch speed test) at the top of the proxy table has no response. Root cause: `trigger_backend_auto_select` path has three inconsistencies compared to the single-node speed test path: (1) `create_client()` had an aggressive 3-second timeout, (2) `delay > 50` lower-bound filter excluded low-latency nodes, (3) `handleCheckAll` had no retry for `AUTO_SELECT_BUSY`. Fix: increased client timeout to 10s, removed the `delay > 50` filter, added AUTO_SELECT_BUSY retry in `handleCheckAll`.

### 🚀 Optimizations

- Refactor: Merged "Takeover Mode" and "Routing Preference" cards into a single unified card module, removing the gap between them.
- Refactor: Swapped the positions of the Upload and Download traffic metric groups at the bottom of the window (Download on left, Upload on right).
- Change: Default sort mode of the table header cursor changed from "default sort" to "sort by delay".
- Change: Adjusted slider min/max values for various skin styles (Depth 2.0→1.0, Radius 5.0→3.0, Roundness 2.0→3.0, Opacity 2.0→5.0, Monochrome Radius 1.0→3.0 & min 0.3→0.0, Vibrancy 2.0→5.0, Shadow 2.0→5.0).

## v1.4.9

### 🐞 Fixed Bugs

- Fix BUG-136: Resolve issue where importing subscription links failed with error "订阅链接内容格式错误，既不是合法的 YAML 配置文件，也无法解析为节点链接列表". Supported Base64-encoded YAML files in the format detection funnel, and made Base64 decoding robust against internal newlines and whitespaces.

## v1.3.4

### 🐞 Fixed Bugs

- Fix BUG-103: Resolve global environment type pollution issue. Split core business types and interfaces in `global.d.ts` into independent ESM module files, and explicitly import them in components/hooks; refactored `IProxyConfig` interface definition to use discriminated unions and intersection types, eliminating overlapping type conflicts and successfully passing compilation.

## v1.3.3

### 🚀 Optimizations

- Optimize BUG-097: Resolve issue where asynchronous worker threads were blocked by synchronous disk I/O. Migrated configuration file reading to `tokio::fs` async APIs and updated large package file writes to be handled by `spawn_blocking`, eliminating transient freezes and latency spikes.
- Optimize BUG-098: Resolve issue where synchronous system process scanning blocked asynchronous threads. Executed `sysinfo` process scanning and force-killing inside `spawn_blocking` to avoid hanging Tokio scheduler worker coroutines.
- Optimize BUG-101: Optimize internal functions and state type safety in TS helper hooks. Wrapped exposed callbacks with `useCallback` to stabilize references, and established exact TS type definitions for core data structures like `proxies`.
- Optimize BUG-105: Optimize callback reference stability inside `use-profiles.ts`. Wrapped `mutateProfiles`, `patchProfiles`, and `patchCurrent` callback functions with `useCallback` to prevent redundant sub-component re-renders.

### 🐞 Fixed Bugs

- Fix BUG-069: Resolve issue where setting switch size="small" under Monochrome skin would freeze on the left side and fail to toggle or interact normally.
- Fix BUG-096: Resolve deadlock issue caused by backend resident threads holding `RwLock` read locks across await points. Acquired locks and completed Future computations inside independent local scopes, allowing the read lock to be dropped and released before the await point.
- Fix BUG-099: Resolve main UI thread white screen hanging and unresponsiveness caused by the synchronous `block_on` call in the Tauri Setup hook at client startup. Switched to `async_runtime::spawn` to load silent update check tasks asynchronously in the background.
- Fix BUG-100: Resolve truncation and overflow hazards of backend timestamp casting on 32-bit OS or embedded platforms. Standardized all timestamp fields in `PrfItem` and `IProfiles` to `Option<i64>` type, abolishing `as usize` type casting.
- Fix BUG-104: Fix casing inconsistency in `useWindowSnap.ts` file name. Renamed it to `use-window-snap.ts` and updated import paths accordingly, aligning with the project's kebab-case naming convention.
- Fix BUG-106: Fix type system failure caused by loose `any` type definitions for internal state variables and core data objects in `_layout.tsx`. Strictly typed `clientUpdateObj` (as `Update`) and `coreUpdateRelease` (as `GithubRelease`) and applied safe casting.

## v1.3.2

### 🚀 Optimizations

- Optimize BUG-082: Implement automatic background memory reclamation and optimization for WebView2 runtime on Windows. Actively send memory reclamation signals to the WebView2 process when the window is minimized or hidden in the system tray, minimizing physical memory usage during standby.

### 🐞 Fixed Bugs

- Fix BUG-093: Resolve significant delays in auto-activating and connecting nodes after Clash Mini starts or reloads configs. Concurrent-probed API ports and `DIRECT` node readiness to shorten cold start selection time, and applied fuzzy comparison to strip timestamp suffixes of historical selections after subscription updates to restore node selection.

## v1.3.1

### 🐞 Fixed Bugs

- Fix BUG-092: Resolve issue where clicking the "lightning cursor" at the top of the active outbound node list on the home page to trigger speed tests for all nodes failed to automatically select and switch to the fastest healthy node in the current filtered subset.
- Fix BUG-091: Resolve issue where the subscription link input field in production could not use right-click for paste, copy, cut, select all, etc.
- Fix BUG-090: Resolve issue where the text of "Subscription & Config", "Traffic Takeover Mode", "Basic Settings" cards on the left panel, and the active outbound node status bar on top in Trump-3D (retro-3d) dark mode rendered white by default, blending into the bright gold background and making reading difficult. Forced text color to high-contrast dark bronze black `#2C1F03` and provided natural hover feedback.

## v1.3.0

### 🚀 Optimizations

- Optimize BUG-087: Renamed the skeuomorphic 3D theme `Retro-3D` to `Trump-3D`, and redesigned it into a luxurious gold bar visual style. Applied linear high-reflection gold bar gradient backgrounds, dark bronze engraved lettering projections, heavy mechanical button press feedback, double-layer gold borders, and dark gold ambient lighting effects to enhance the skeuomorphic feel.
- Optimize BUG-086: Shortened the throttling interval of traffic data updates in narrow layout mode from 3000ms to 1000ms to eliminate traffic chart dropping to zero and jagged line breaks, and automatically suspended queries when the window is hidden to optimize resource consumption.

### 🐞 Fixed Bugs

- Fix BUG-089: Resolve issue where periodic health monitoring of active node latency in maximized mode triggered high-frequency speed test timeouts due to deprecated backend APIs, forcing frequent re-selection and switching of nodes. Refactored it to call the new `delayProxyByName` API provided by the plugin, and implemented a silent strategy (no notification popups) for background auto-selection of the same fastest node.
- Fix BUG-081: Completed promise chains for data refresh and added a delay before config polling, resolving the synchronization issue where home page node list failed to refresh immediately when importing/updating subscriptions and required waiting two minutes.
- Fix BUG-083: Resolve issue where auto-selection failed to read the correct Profile ID for subset filtering at startup due to the state closure not being ready.
- Fix BUG-084: Cleaned and removed redundant `!isDummyNode` filtering logic in auto-selection, delegating it to the data loading layer for a single clean interception.
- Fix BUG-085: Removed "All routes are busy" bubble notifications in background silent health checks to avoid distracting users with redundant error popups.
- Fix BUG-088: Fix issue in Trump-3D dark mode where default buttons with Contained style had light gold text blending into the bright gold background, making it hard to read. Adjusted text color to high-contrast dark bronze.

## v1.2.8

### 🐞 Fixed Bugs

- Fix BUG-081: Resolve slow response of auto-selecting the fastest node after starting or switching subscriptions, and the issue where advertising/placeholder nodes (such as remaining traffic, official website) occupied the top spots of the node list and falsely succeeded speed tests, interfering with background `NodeMonitor` checks. Implemented deep purification filtering in global data layers (`calcuProxies`, `calcuProxyProviders`, `fetchProxies`), and automatically triggered speed tests when `NodeMonitor` detects an ad node to correct the selection.

## v1.2.6

### 🚀 Optimizations

- Optimize BUG-079: Refined the tooltip description of "Rules Adjustable" in routing strategy to "arbitrarily adjust path controls on top of preset rules", making it more accurate and aligned with the "Path Control" feature, and updated translations for all 13 supported languages.

### 🐞 Fixed Bugs

- Fix BUG-080: Resolve unfriendly update notifications when versions match. When the kernel or client is already the latest version, replaced the error popups with info tooltips to improve interaction friendliness and robustness.

## v1.2.5

### 🚀 Optimizations

- Dead Code Cleanup: Removed redundant deprecated functions, dead code, and unused variables, streamlined the system tray menu, and restored the mechanism of automatically pulling the latest stable kernel.

### 🐞 Fixed Bugs

- Fix BUG-078: Resolve freeze issues in kernel upgrade and update downloads. Added 20-second timeout control for response stream chunk reads, and implemented multi-channel (Localhost, System, None) silent retries and auto-fallback, shortening TCP handshake timeout to 10 seconds.

## v1.2.4

### 🚀 Optimizations

- Performance Optimizations: Implemented batch buffered logging to reduce backend IPC event dispatch frequency; suspended redundant rule and policy group queries in mini-monitoring mode; implemented 1D incremental connection list updates to reduce data throughput; limited the list to display at most 100 most active physical nodes to improve React rendering performance.
- Asymmetric Window Wakeup: Wakes up WebSocket connections immediately when the window becomes visible, and disconnects with a 1-second delay when minimized or hidden.
- Adaptive Borders: Replaced dynamic animated borders with static, adaptive double/solid borders to reduce system CPU overhead.

### 🐞 Fixed Bugs

- Fix BUG-077: Fix update failure caused by incorrect regex matching of the Mihomo core auto-update filename.

## v1.2.3

### 🚀 Optimizations

- Optimize BUG-075: Completely resolve memory leak issues in backend connection list polling. Automatically suspend polling when the Connections drawer is closed; introduced client-side Traffic Accumulator to perform incremental memory calculation via low-frequency traffic SSE stream when the drawer is closed, ensuring accumulated data updates on the bottom control panel.

### 🐞 Fixed Bugs

- Fix BUG-076: Resolve issue where the update button remained clickable and triggered duplicate downloads when the kernel was already the latest version. Added static version comparison (filtering tag prefixes/suffixes) to disable the button and show "Up to date" when versions match.
- Fix Auto-update 404 error: Created `updater/app-update.json` and redirected Tauri auto-update URL to the GitHub repository Raw address, ensuring check-for-updates in client no longer throws errors.
- Refine Silent Release Guide: Added Phase 3 in `clash_mini_silent_release.md` to establish updater configuration file maintenance instructions.

## v1.2.2

### 🚀 Optimizations

- Pre-flight preparations for performance and update optimizations.

## v1.2.1

### 🐞 Fixed Bugs

- Fix BUG-065: Pin active outbound node real-time subset rotation filtering limits. When clicking the node name in the active outbound node status bar to rotate, dynamically extract and apply current search, filter, sorting, and hide-timeout conditions to ensure the rotation is restricted within the currently displayed candidate node subset.
- Fix BUG-074: Kernel update connection timeout and redundant version formatting. Refactored the backend to introduce a `NetworkManager` multi-downgrade automatic fallback mechanism (Local Port -> System Proxy -> Direct) for checking kernel updates, solving timeout connection failures caused by network environments; added universal `formatCoreVersion` formatter on frontend to eliminate duplicate `vv` prefix and unify to `Ver.X.Y.Z` format.

## v1.2.0

### 🐞 Fixed Bugs

- Fix BUG-072: Resolve issue where Allow LAN switch bounced back automatically and failed to stay enabled. Added lowercase deserialization config and `ts-rs` conversion annotations to the backend core `FindProcessMode` model, eliminating bouncing switches caused by Mihomo core field parsing errors.

### 🚀 Optimizations

- Refine Subscription Card Right-Click Menu: Added context menu (Edit, Edit File, Copy Link, Update, Delete) on subscription cards, and supported a simple config editing dialog, adapting seamlessly to all 6 visual themes.

## v1.1.3

### 🐞 Fixed Bugs

- Fix BUG-066: Optimize speed slider breathing/pulsing animation in Cyberpunk skin. Switched to inset box-shadow and border breathing, accelerated breathing frequency, and dynamically scaled aurora flow speed with slider value, solving the clipping and lack-of-effect issues.
- Fix BUG-067: Optimize shadow slider feedback in Modern Flat skin. Introduced adaptive base shadow density variables for light and dark modes (darker in dark mode) to unify button, card, input, and panel projections.
- Enhance Adjustment Freedom: Unified the maximum limit range of double sliders under all styles from `2.0` (or `3.0`) to `5.0` to provide users with stronger visual contrast and fine-grained control.

## v1.1.2

### 🚀 Optimizations

- Added 1-click loop switching of active nodes in the navigation bar.
- Streamlined development agreement `clash_mini_agreements.md`, removing redundant pitfalls and release SOP.

## v2.0.4

### 🐞 Fixed Bugs

- Fix BUG-041: Completely restructured home page node list rendering, removing React state `isMinimalHeight` and height listeners. The node list now renders unconditionally and clips naturally with window size, fixing the blank list issue caused by rendering timings at initial load.
- Optimize Agreement: Updated [Highest Design Baseline] to skeuomorphic 3D and physical texture aesthetics, and corrected development boundaries and Rust backend refactoring rules.
- Fix Doc Consistency: Corrected bug registration references in `clash_mini_pitfalls.md` and `clash_mini_silent_release.md` to `bug_list.md`.

## v2.0.3

### 🚀 Optimizations

- Upgrade Compile Environment: Updated project dependencies, supporting TypeScript type checks and backend Clippy static compilation.

## v2.0.2

### 🚀 Optimizations

- Narrow Layout Optimization: Right-aligned the protocol column, left-aligned the node name, and centered the latency column in the home page node list, and optimized the minimum window height to `163px` in minimalist mode.

## v2.0.1

### 🐞 Fixed Bugs

- Fix BUG-034: Optimize subscription activation auto-chain actions, restricting speed-tests and fastest node selection strictly to the current filtered home page node subset, preventing unavailable nodes from being incorrectly selected.
- Archive README_First: Updated and archived beginner user guides.

## v1.1.9

### 🐞 Fixed Bugs

- Fix BUG-073: Implemented static system tray design on Windows, avoiding COM `E_FAIL` (os error -2147467259) errors when dynamic update instructions sent to Explorer are blocked under Administrator privileges.
- Fix BUG-038: Fix setting page "Active/History" connection switch being blocked by the header, making parts of it unclickable.

### 🚀 Optimizations

- Optimize STYLE-001: Upgraded 23 core interactive components (including buttons, selectors, switches, sliders, inputs, and cards) in main scene and settings page to skeuomorphic 3D Bevel textures and neon glow, linked to adaptive depth and glow variables.

## v1.1.8

### 🐞 Fixed Bugs

- Fix BUG-071: Client and Mihomo core auto-update development and UI dropdown integration. Upgraded `tauri-plugin-mihomo` dependency to support newer kernels (v1.19.27+), resolving blank active node lists.
- Fix BUG-072: Fix Allow LAN switch bouncing back due to LogLevel serialization issues in newer plugin models.rs.

### 🚀 Optimizations

- Popover Menu Restructuring: Reconstructed the sidebar "Help" button into an upward Popover menu, adapting to all 6 visual skins with 3D bevel and glow effects.
- Adaptive Updates: Supports client silent background update polling and boot-ready detection, integrating kernel hot-swap and service release mechanisms.

## Upstream Release History (Clash Verge History)

## v2.5.2

### 🐞 Fixed Bugs

- Possible styling errors in macOS tray speed display.

<details>
<summary><strong> ✨ New Features </strong></summary>

- Added display support for TrustTunnel, OpenVPN, Tailscale, GostRelay nodes.

</details>

<details>
<summary><strong> 🚀 Optimizations </strong></summary>

- Close autofill popups.

</details>
