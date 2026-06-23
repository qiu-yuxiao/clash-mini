## v1.6.9

### 🐞 Fixed Bugs

- **修复 Rust 编译错误**：在 `window_manager.rs` 添加缺失的 `use std::pin::Pin`，清理多余 import。
- **修复 TypeScript import 类型错误**：`app-data-context.ts` 中 `BaseConfig`/`ProxyProvider`/`Rule`/`RuleProvider` 改为 `import type`，防止运行时因无法解析外部包类型导致前端白屏。
- **滚动条样式修正**：恢复 `proxy-groups.tsx` 中 `scrollbarWidth: 'none'` 样式稳定性。

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
