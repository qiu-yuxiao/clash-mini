# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：`BUG-057` ~ `BUG-316` 已通过用户验收并完整归档于 `clash_mini_agreements.md` 附录 A。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs: BUG-317 ~ BUG-361)

### BUG-317（对应规范章节：5.5）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Critical / 网络探测与自愈
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:289-308, 1298-1347`
- **根因与背景实证**：`is_physical_network_online` 顺序串行 `await` 三个公网 DNS `TCP:53` 地址且未设单连接超时，在静默丢弃 53 端口的企业/校园/酒店网络下首个连接耗尽全部 2s 预算，导致备选 DNS 与 `lookup_host` 域名解析兜底永不执行，后台健康巡检与断流自愈停摆。
- **修正与改良方案**：重构为并行竞争探测（每个 `TCP:53` 探针独立 `600ms` 超时预算），任一连通即刻返回 `true`；若 `TCP:53` 均不可达则在剩余预算内立即回退执行 `lookup_host` 域名解析兜底。

### BUG-318（对应规范章节：5.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 自愈记账与告警
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:1547-1650`
- **根因与背景实证**：`self_heal_with_accounting` 在 `auto_select_fail_count >= 3` 时无论内核 API 是否可达均先将计数器清零，导致全节点不可达但本地内核 API 正常时计数器永远在 `0 -> 1 -> 2 -> 0` 循环，协议 5.3 节规定的连续 5 次失败 Win32 告警弹窗（`fire_self_heal_alert`）永远无法触发。
- **修正与改良方案**：解耦内核 API 失活重启计数与全节点断流告警计数：API 不可达满 3 次时执行内核重启恢复通道；全节点选点连续失败累计满 5 次时可靠触发 `fire_self_heal_alert()` Win32 告警并重置告警计数。

### BUG-319（对应规范章节：3.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 内核在线升级
- **涉及物理路径与行号**：`src-tauri/src/core/core_updater.rs:468-520`, `src-tauri/src/core/manager/state.rs:40-48`
- **根因与背景实证**：内核在线升级在解压前直接 `fs::File::create` 打开正式路径 `cores/mini-mihomo.exe`，瞬间将其截断为 0 字节；若下载中断或解压异常则残留 0 字节损坏文件，后续启动触发 `0xc000007b` 致命错误。
- **修正与改良方案**：改为写入 `.staging` 临时文件，完成解压并校验文件体积与 `MZ` PE 签名合法后再通过原子替换覆盖目标可执行文件；异常时清理 `.staging` 保留原可用内核；同时在 `state.rs` 选择自定义内核前校验最小合法体积，损坏时自动回退内置 sidecar。

### BUG-320（对应规范章节：7.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 通用协议解析器
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/universal_parser.rs:31-42, 208-210, 846-850, 1018-1025`
- **根因与背景实证**：`parse_ss` 与 `parse_wireguard` 在剥离 `?` 查询串前先按 `@` 切分导致含 `@` 查询参数截断主机地址；`parse_socks5` 漏剥离 `?` 查询串导致端口解析失败；`VMessJson` 缺失 `sni` 字段反序列化映射导致 CDN 域名前置节点 TLS 握手失败。
- **修正与改良方案**：统一遵循「先剥离 `#fragment` -> 再剥离 `?query` -> 最后拆解 `userinfo@host:port`」顺序修复 `parse_ss`、`parse_wireguard`、`parse_socks5`，并在 `VMessJson` 中补齐 `sni` 映射至 `servername`。

### BUG-321（对应规范章节：2.8 / 5.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 过滤子集测速调度
- **涉及物理路径与行号**：`src/pages/_layout/utils/profile-coordination.ts:97-117`
- **根因与背景实证**：`batchTestWithFirstBatchSelect(names, select)` 将前端子集卡片置为 `-2` 测速态后，调用 `triggerAutoSelect(currentUid, undefined, 0, select)` 误传硬编码 `undefined`，导致子集参数丢失及未覆盖卡片转圈残留。
- **修正与改良方案**：将第二个参数修正为传入的 `names`（`triggerAutoSelect(currentUid, names, 0, select)`），并在异常分支兜底复位 `-2` 加载态。

### BUG-322（对应规范章节：5.2 / 5.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 测速结果排序
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:1050-1053`
- **根因与背景实证**：后端批量测速展示矩阵中超时不可达节点延迟记为 `0`，默认升序排序 `display.sort_by_key(|r| r.1)` 将 `0ms` 死节点排在所有正常节点最前面。
- **修正与改良方案**：将默认延迟排序键改为 `if r.1 == 0 { u32::MAX } else { r.1 }`，确保超时节点恒定排在有效节点末尾。

### BUG-323（对应规范章节：6.2 / 7.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / Deep Link 订阅导入
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/scheme.rs:54-60`
- **根因与背景实证**：`extract_subscription_url` 采用字符串盲切至末尾，当 `clash://install-config?url=...&name=...` 含后续参数时将 `&name=...` 一并拼入订阅 URL 请求机场引发 HTTP 400。
- **修正与改良方案**：改为标准 URL 查询参数解析提取 `url` 字段并兼容百分号编码与未编码查询边界。

### BUG-324（对应规范章节：3.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / IPC 连接池超时层级
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/ipc.rs:474-504`, `crates/tauri-plugin-mihomo/src/mihomo.rs:32, 184`
- **根因与背景实证**：命名管道连接池排队获取许可超时（5s）大于外层默认请求超时（3s），高并发满载排队至 3s 时外层提前取消 Future 引发 Broken Pipe。
- **修正与改良方案**：重构超时层级：连接池许可获取超时收敛为 `2s`，外层默认请求超时提升至 `5s`，确保排队等待严格短于请求总预算。

### BUG-325（对应规范章节：4.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / WebSocket 断开保护
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/mihomo.rs:381-385`
- **根因与背景实证**：WebSocket `disconnect` 在 `force_timeout` 传入 `None` 或 `Some(0)` 时走无超时分支裸 `await` 发送关闭帧，若内核侧通道满载阻塞将导致调用方永久挂起。
- **修正与改良方案**：为关闭帧发送强制赋予保底超时（默认 `1000ms`，下限 `500ms`），杜绝断开流程无限期等待。

### BUG-326（对应规范章节：1.4）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Low / Sidecar 进程退出同步
- **涉及物理路径与行号**：`src-tauri/src/core/manager/state.rs:148-151`
- **根因与背景实证**：`stop_core_by_sidecar` 杀死子进程后仅固定 `sleep(100ms)` 而未确认进程真正退出，高连接负载下重启内核存在旧进程尚未释放端口的竞态。
- **修正与改良方案**：记录被停止子进程 PID 并轮询确认进程退出（步进 `50ms`，上限 `2000ms`），进程一退出即刻放行后续启动。

### BUG-327（对应规范章节：6.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Low / SSRF 内网域名防御
- **涉及物理路径与行号**：`src-tauri/src/config/prfitem.rs:828-851`
- **根因与背景实证**：`validate_url_no_ssrf` 在本地 DNS 解析失败时静默放行，未拦截 `.localhost`、`.local`、`.internal`、`.lan` 等内网保留主机后缀。
- **修正与改良方案**：在 DNS 解析前增加内网保留后缀（`.localhost`/`localhost.`/`.local`/`.internal`/`.home.arpa`/`.lan`）与单标签主机名拦截校验。

### BUG-328（对应规范章节：1.3 / 1.8）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 服务状态并发与 TUN 保护
- **涉及物理路径与行号**：`src-tauri/src/core/service.rs:416-455`, `src-tauri/src/core/manager/lifecycle.rs:200-257`
- **根因与背景实证**：`ServiceManager::run_operation` 使用单一 `AtomicBool` 防重入，并发状态查询直接报错 `service operation already running`，误触 `lifecycle.rs` 降级回退逻辑将用户开启的 `enable_tun_mode` 静默改写为 `false`。
- **修正与改良方案**：将 `ServiceManager` 操作门控升级为异步互斥排队，并在 `lifecycle.rs` 中严格区分瞬时繁忙与真实服务不可用，杜绝因并发查询篡改用户 TUN 配置。

### BUG-329（对应规范章节：3.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 命名管道半开连接自愈
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/ipc.rs:274-277, 474-503`, `crates/tauri-plugin-mihomo/src/mihomo.rs:183-203`
- **根因与背景实证**：内核重载或空闲断开命名管道后，连接池 `sender.is_ready()` 仍可能为 `true`，首次复用即抛 `HttpParseError` 且无透明重连重试，引发前端偶发红框报错。
- **修正与改良方案**：在本地管道请求发送层增加池化陈旧连接（复用连接）失败自动驱逐并新建物理管道透明重发 1 次的容错自愈机制。

### BUG-330（对应规范章节：3.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 命名管道繁忙退避互锁
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/ipc.rs:113-145, 500-503`, `crates/tauri-plugin-mihomo/src/mihomo.rs:32`
- **根因与背景实证**：命名管道 `ERROR_PIPE_BUSY` 24 次固定休眠退避总耗时 `3000ms` 与旧外层超时 `3000ms` 等长互锁，后半程建连成功亦来不及完成 HTTP 交互即被掐断。
- **修正与改良方案**：集成 Win32 `WaitNamedPipeW` 原生事件唤醒等待并收敛单次最大等待步长，配合 `5s` 外层超时彻底消除互锁掐断。

### BUG-331（对应规范章节：1.4 / 1.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 服务模式退出孤儿内核清理
- **涉及物理路径与行号**：`src-tauri/src/feat/window.rs:134-153`, `src-tauri/src/core/manager/state.rs:156-183`
- **根因与背景实证**：服务模式下退出清理若遇 2s 超时回退，普通用户会话进程调用 `Process::kill()` 终止 Session 0 下 `SYSTEM` 权限的 `mini-mihomo.exe` 会因 Win32 Error 5 拒绝访问静默失败，残留孤儿 TUN 内核。
- **修正与改良方案**：退出清理将 `stop_core` 预算放宽至 `3.5s`，且当处于服务模式或检测到服务在线时优先通过 `ipc::stop_core_by_service()` 提权通道强制停止 Session 0 内核后再执行本地兜底清理。

### BUG-332（对应规范章节：2.4）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 轻量唤醒窗口句柄复用
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/window.rs:293-350`, `src-tauri/src/utils/window_manager.rs:629-720`
- **根因与背景实证**：轻量模式销毁主窗口时未复位静态 `HOOKED_HWND`，若唤醒重建主窗口时 Windows 句柄表复用相同 `HWND` 值会误判已挂载而跳过 `WM_SIZING` 防下移钩子。
- **修正与改良方案**：迁移至绑定窗口生命周期的 Win32 `SetWindowSubclass` 并在 `WM_NCDESTROY` 时自动卸载清理句柄记录，彻底根除 `HWND` 复用失灵。

### BUG-333（对应规范章节：1.1）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 终端环境变量端口规范
- **涉及物理路径与行号**：`src-tauri/src/feat/proxy.rs:76-88`
- **根因与背景实证**：`copy_clash_env` 在 `verge_mixed_port` 未显式落盘时回退硬编码原版端口 `7897` 而非协议规定的 `10801`，导致复制出的终端代理命令连接被拒。
- **修正与改良方案**：统一回退至协议默认混合端口常量 `crate::constants::network::ports::DEFAULT_MIXED`（`10801`）。

### BUG-334（对应规范章节：8.7）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 定时订阅更新 Panic 防护
- **涉及物理路径与行号**：`src-tauri/src/core/timer.rs:268-283, 339-346`
- **根因与背景实证**：订阅自动更新异步任务未设置 Panic / Drop 复位保护，一旦任务中途异常展开将漏发 `TaskFinished`，导致 `state.running` 永久卡死在 `true`。
- **修正与改良方案**：引入 RAII Drop 守卫与 `catch_unwind` 双重屏障，确保异步任务无论正常返回、取消或 Panic 均可靠投递 `TaskFinished` 释放运行态。

### BUG-335（对应规范章节：4.1 / 5.4）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 内核异常重启选点恢复
- **涉及物理路径与行号**：`src-tauri/src/core/manager/lifecycle.rs:139-147, 301-319`
- **根因与背景实证**：内核卡死重启前 `snapshot_proxy_group_now()` 超时返回 `None`，重启后未回退读取本地 `profiles.yaml` 中持久化的上次有效选点，导致代理组掉回列表首项。
- **修正与改良方案**：当即时快照为 `None` 时自动回退读取当前激活 Profile 的 `selected` 持久化记录（过滤伪节点后）执行节点恢复。

### BUG-336（对应规范章节：2.8）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 顶栏活跃节点轮换同步
- **涉及物理路径与行号**：`src/pages/_layout/components/active-node-card.tsx:139-200`
- **根因与背景实证**：顶栏活跃节点卡片点击快速轮换（`handleCycleNode`）切换内核成功后漏调 `['getProxies']` 缓存更新与 `patchProfile` 持久化，导致界面停留旧节点且重启丢失选点。
- **修正与改良方案**：切换成功后立即乐观更新并作废 `['getProxies']` 缓存，同时调用 `patchProfile` 持久化写入 `selected` 并通知后端节点切换宽限期。

### BUG-337（对应规范章节：2.8）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 节点虚拟列表 Memo 稳定性
- **涉及物理路径与行号**：`src/components/proxy/proxy-groups.tsx:80-84`, `src/hooks/use-proxy-selection.ts:45, 110-165`, `src/components/proxy/proxy-render.tsx:27-61`
- **根因与背景实证**：`ProxyGroups` 每次渲染传入新匿名配置对象导致 `useProxySelection` 返回易变回调，击穿 `ProxyRender` 与 `ProxyItem` 的 `useMemo`/`React.memo`，大节点列表随流量跳动全量重绘。
- **修正与改良方案**：在 `useProxySelection` 内使用 `useRef` 稳定持有 `onSuccess`/`onError` 回调并固定返回函数引用，保护虚拟列表行级缓存命中率。

### BUG-338（对应规范章节：2.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 运行日志弹窗滚动抖动与劫持
- **涉及物理路径与行号**：`src/pages/logs.tsx:68-75`
- **根因与背景实证**：日志弹窗随 50ms 日志批次无条件触发 `behavior: 'smooth'` 平滑滚动动画，造成主线程动画队列堆积掉帧，并在用户向上翻阅历史日志时暴力拉回底部。
- **修正与改良方案**：移除高频 `smooth` 滚动冲突，改为仅在视口处于底部吸底状态时执行 `behavior: 'auto'` 即时跟随，用户上翻查阅时自动暂停拉拽。

### BUG-339（对应规范章节：5.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 节点延迟状态双重渲染
- **涉及物理路径与行号**：`src/hooks/use-proxy-delay-state.ts:18, 63-89`
- **根因与背景实证**：`useProxyDelayState` 惰性初始化计算延迟后挂载 `useEffect` 再次生成新对象触发重渲染，导致长列表滚动时每个可见节点卡片双倍渲染。
- **修正与改良方案**：在 reducer 中增加字段级浅相等短路比较（数值与状态完全相同时直接返回原引用），消除挂载与无变化通知带来的冗余重绘。

### BUG-340（对应规范章节：2.10 / 2.11）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 浮动配置 Promise 异常兜底
- **涉及物理路径与行号**：`src/pages/_layout/components/theme-settings-card.tsx`, `src/pages/_layout/components/basic-settings-card.tsx`, `src/pages/_layout.tsx`
- **根因与背景实证**：多处开关与主题切换直接触发浮动 `patchVerge(...)` Promise 而未附带 `.catch()`，当 IPC 繁忙抛出异常时产生 Unhandled Promise Rejection。
- **修正与改良方案**：为相关事件处理器中的浮动异步配置写入统一补齐 `.catch()` 错误吸收与日志记录。

### BUG-341（对应规范章节：1.5 / 2.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 271px 窄窗多语种设置排版
- **涉及物理路径与行号**：`src/pages/_layout/components/basic-settings-card.tsx:31-36, 116-121`
- **根因与背景实证**：271px 最小窗口宽度下，西文长标签在固定双列网格中配合 `whiteSpace: 'nowrap'` 无截断溢出并遮挡右侧开关控件。
- **修正与改良方案**：为网格单元赋予 `minWidth: 0` 与标签文本 `overflow: 'hidden'`, `textOverflow: 'ellipsis'` 及完整 `title` 悬浮提示，保证 13 语种在 271px 下零溢出遮挡。

### BUG-342（对应规范章节：8.9）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Low / 快捷分流规则通知词条勘误
- **涉及物理路径与行号**：`src/utils/quick-routing.ts:105-109`, `src/locales/*/profiles.json`
- **根因与背景实证**：连接面板添加快捷分流规则成功后复用了 `profiles.page.feedback.notifications.profileSwitched`，误向用户提示“订阅已切换”。
- **修正与改良方案**：在全部 13 个语种资源中新增专属 `quickRuleAdded` 通知词条并更新 `quick-routing.ts` 调用。

### BUG-343（对应规范章节：2.8）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Low / 搜索框双重广播去重
- **涉及物理路径与行号**：`src/components/base/base-search-box.tsx:95-106`
- **根因与背景实证**：`BaseSearchBox` 在输入事件回调与 `useEffect` 中对同一输入变动重复调用 `emitSearch` 两次，造成双倍过滤计算。
- **修正与改良方案**：收敛搜索匹配派发路径并记录上一次已广播键值，确保单次输入或筛选项变更仅触发一次广播。

### BUG-344（对应规范章节：5.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 单节点专属微测速与结果共享
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:940-1008`
- **根因与背景实证**：非 TUN（Sidecar）模式下用户单点测速一个节点仍触发全组上百节点并发拨测；且 TUN 模式下执行 `PROXY__METRICS` 组测后丢弃了同批测出的其他节点新鲜延迟。
- **修正与改良方案**：在非 TUN Sidecar 模式下对显式单节点测速启用 `delay_proxy_by_name` 专属微探针（毫秒级直达、节约 95% 带宽与连接开销，严守不选点/不复用旧缓存/未执行不伪造 Timeout 契约）；在 TUN 模式下保持 `PROXY__METRICS` 防回环组测并将同批新鲜延迟同步回填前端缓存。

### BUG-345（对应规范章节：5.5）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 并行竞速物理网络探针
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:289-308`
- **根因与背景实证**：串行物理网络探针在任一上游 DNS 抖动时增加数百毫秒至数秒无谓等待。
- **修正与改良方案**：使用 `futures::future::select_ok` 并行发起三路公网 DNS `TCP:53` 握手探针（单路 600ms 超时），最快一路握手成功即在 10~30ms 内判定在线，全败则秒级平滑衔接域名解析兜底。

### BUG-346（对应规范章节：3.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 内核升级原子暂存与 PE 校验
- **涉及物理路径与行号**：`src-tauri/src/core/core_updater.rs:460-530`
- **根因与背景实证**：内核在线升级缺乏下载产物结构完整性验证与失败无损回滚保护。
- **修正与改良方案**：引入 `.staging` 原子暂存替换流水线：解压后校验二进制体积（`>= 5 MB`）与 DOS/PE `MZ` 魔数头，校验通过再原子重命名替换正式文件，任何环节失败自动清理临时文件并保留旧内核完好可用。

### BUG-347（对应规范章节：4.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 前端 WebSocket 指数退避重连
- **涉及物理路径与行号**：`src/hooks/use-traffic-monitor.ts`, `src/pages/_layout/hooks/use-clash-ws.ts` (及相关 WS 订阅链路)
- **根因与背景实证**：内核重启或服务切换期间固定短周期重连产生高频无效 IPC/WS 风暴与错误日志刷屏。
- **修正与改良方案**：将前端 WebSocket 断线重连升级为带 `±20%` 随机抖动的指数退避（`1s -> 2s -> 4s -> 8s`，上限 `15s`），并监听内核重启/配置刷新事件瞬时清零退避计数立即重连。

### BUG-348（对应规范章节：7.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 统一 URI 协议解析预处理管道
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/universal_parser.rs`
- **根因与背景实证**：各代理协议解析函数各自手工切分 `#` 与 `?`，容易在复杂参数下出现边界不一致。
- **修正与改良方案**：封装共享纯函数 `strip_fragment_and_query(input)` 统一全协议 URI 预处理管道，并新增含 `@` 查询参数、Socks5 查询串与 VMess `sni` 的回归单元测试。

### BUG-349（对应规范章节：5.2 / 5.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 测速 0ms 超时置底全栈对齐与单测锁定
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs`
- **根因与背景实证**：后端展示矩阵与候选排序缺少独立纯函数封装与针对 `0ms` 超时节点的回归单测锁定。
- **修正与改良方案**：抽离 `sort_display_delays` 纯函数统一处理延迟升序（`0 -> u32::MAX` 垫底）与名称排序，并补充单元测试防止退化。

### BUG-350（对应规范章节：2.4）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 现代化 Win32 SetWindowSubclass 窗口钩子
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/window.rs:293-350`, `src-tauri/Cargo.toml`
- **根因与背景实证**：传统 `SetWindowLongPtrW(GWLP_WNDPROC)` 依赖全局静态旧过程指针，与其他窗口过程链及窗口销毁重建生命周期耦合脆弱。
- **修正与改良方案**：升级采用 `Win32::UI::Shell::{SetWindowSubclass, DefSubclassProc, RemoveWindowSubclass}` 标准子类化架构，并在收到 `WM_NCDESTROY` 时自动解绑释放，原生支持任意次主窗口销毁与重建。

### BUG-351（对应规范章节：3.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 韧性命名管道连接池与 WaitNamedPipeW
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/ipc.rs`, `crates/tauri-plugin-mihomo/Cargo.toml`
- **根因与背景实证**：`ERROR_PIPE_BUSY` 期间纯 `tokio::time::sleep` 盲轮询延迟偏高且无法感知管道即时空闲事件。
- **修正与改良方案**：在 Windows 平台引入 `WaitNamedPipeW` 阻塞线程池事件等待配合退避重试，并在连接池层记录连接复用标记实现陈旧断开连接的无感单次重建。

### BUG-352（对应规范章节：1.3 / 1.8）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / ServiceManager 异步互斥化状态门控
- **涉及物理路径与行号**：`src-tauri/src/core/service.rs:395-455`
- **根因与背景实证**：启动期多路异步任务并发查询/刷新系统服务状态时缺乏串行协调。
- **修正与改良方案**：将 `ServiceManager` 内部的 `operation_running: AtomicBool` 重构为 `tokio::sync::Mutex<()>` 异步互斥锁，使并发 `refresh`/`apply` 安全排队执行而不再相互报错。

### BUG-353（对应规范章节：5.1 / 8.7）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 后台常驻守护任务 spawn_supervised 监督器
- **涉及物理路径与行号**：`src-tauri/src/process/async_handler.rs`, `src-tauri/src/module/monitor.rs`
- **根因与背景实证**：后台长期运行协程若遭遇极端第三方库或数据边界 Panic 会静默终止，缺乏自动拉起防线。
- **修正与改良方案**：在 `AsyncHandler` 中新增 `spawn_supervised` 守护包装器（`AssertUnwindSafe(...).catch_unwind()` + 错误日志留痕 + 退避自重启，应用退出时自动终止），加固后台健康监测主循环。

### BUG-354（对应规范章节：2.12 / 8.7）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 解封常规屏幕高度下的顶栏帮助与诊断菜单
- **涉及物理路径与行号**：`src/pages/_layout/components/help-menu-button.tsx`, `src/pages/_layout/components/header-bar.tsx`
- **根因与背景实证**：`HelpMenuButton` 设置了 `@media (max-height: 830px) { display: none }`，导致默认窗口高度（680px）及主流 1080p/笔记本缩放屏下用户完全看不到 GitHub Star、Wiki、重新打开新手引导与导出诊断包入口。
- **修正与改良方案**：移除 `max-height: 830px` 隐藏封锁，保持紧凑图标常驻顶栏控制区，让全体用户随时可访问诊断导出、首次引导与项目主页。

### BUG-355（对应规范章节：2.8 / 5.2 / 5.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 补齐 F4 / F1 快捷测速择优与 Ctrl+F 搜索聚焦快捷键
- **涉及物理路径与行号**：`src/pages/_layout.tsx`
- **根因与背景实证**：设计规范定义了 `F4`（可见子集测速择优）与 `F1`（全量测速择优），但主界面缺少对应的窗口级键盘快捷监听及 `Ctrl+F` 快速定位过滤框能力。
- **修正与改良方案**：在主界面挂载全局键盘监听（排除弹窗输入冲突后）：`F4` 触发当前可见过滤子集测速择优、`F1` 触发全量节点测速择优、`Ctrl+F` 自动聚焦并全选节点过滤输入框。

### BUG-356（对应规范章节：2.8 / 2.10）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 手动点击切换代理节点 0ms 乐观高亮
- **涉及物理路径与行号**：`src/hooks/use-proxy-selection.ts`
- **根因与背景实证**：手动点击节点卡片切换出口时需等待后端 IPC 与代理表重刷完成后高亮才移动，存在 200~800ms 视觉滞后。
- **修正与改良方案**：点击节点瞬间对 `queryClient` 的 `['getProxies']` 缓存执行乐观 `now` 更新实现 `0ms` 即时选中高亮，后台异步提交并在极少数失败场景下自动回滚缓存。

### BUG-357（对应规范章节：2.8 / 5.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 节点卡片延迟徽章独立点击单点测速热区
- **涉及物理路径与行号**：`src/components/proxy/proxy-item.tsx`, `src/components/proxy/proxy-item-mini.tsx`
- **根因与背景实证**：节点卡片整行点击会直接切换当前出口节点，用户缺少在不中断当前活跃连接的前提下单独探测某一备用节点实时延迟的快捷入口。
- **修正与改良方案**：将节点卡片/紧凑卡片右侧延迟徽章设为独立点击热区（`e.stopPropagation()`），点击延迟数值仅对该节点执行单点实时测速（`select=false`）而不切换当前出口，点击卡片其余区域仍立即切换节点。

### BUG-358（对应规范章节：2.7）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 特色皮肤双滑块拖动渲染与持久化落盘解耦
- **涉及物理路径与行号**：`src/pages/_layout/components/theme-settings-card.tsx`
- **根因与背景实证**：拖动皮肤物理光影/色彩滑块时在高频 `onChange` 中同步反复读写 `localStorage` 并触发整树状态同步，造成快速拖动微卡顿。
- **修正与改良方案**：将拖动过程与持久化拆解：`onChange` 仅以内存态与 CSS 变量驱动 60fps 实时光影预览，`onChangeCommitted` 松手瞬间执行单次 `localStorage` 持久化写入。

### BUG-359（对应规范章节：5.3 / 5.5）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 全节点超时或物理断网时顶栏轻量状态胶囊与一键自愈
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs`, `src/pages/_layout/components/active-node-card.tsx`, `src/locales/*/layout.json`
- **根因与背景实证**：当物理网络中断或后台测速发现全部节点超时不可达时，主界面顶栏缺乏直观的故障定性提示与一键重测恢复入口。
- **修正与改良方案**：后端监测在全节点不可达/物理离线及恢复时派发轻量状态事件，顶栏活跃节点区在全量超时或离线时展示紧凑警示状态提示，点击即可立即触发重测自愈（13 语种 i18n 齐全）。

### BUG-360（对应规范章节：2.8）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 节点过滤输入框自动测速择优状态微反馈
- **涉及物理路径与行号**：`src/pages/_layout.tsx`, `src/locales/*/layout.json`
- **根因与背景实证**：节点过滤框输入后 700ms 防抖自动触发并发测速择优期间缺少视觉状态指示，用户不易感知后台正在自动筛选测速。
- **修正与改良方案**：在过滤自动测速防抖等待与执行期间为过滤框提供清晰的轻量加载/脉冲视觉指示及无障碍提示，测速择优完成后平滑收起。

### BUG-361（对应规范章节：2.9）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 系统托盘图标左键单击唤醒与隐藏双向切换
- **涉及物理路径与行号**：`src-tauri/src/core/tray/mod.rs:215-260`
- **根因与背景实证**：系统托盘左键单击仅单向执行激活恢复，当主窗口已处于前台可见且聚焦状态时再次点击托盘无法顺手收起窗口。
- **修正与改良方案**：在不触碰任何托盘动态图标/菜单更新 API（严守 BUG-073 静态托盘红线）的前提下，左键单击回调先判断主窗口状态：若不处于轻量模式且主窗口当前可见、未最小化且已获焦点，则平稳隐藏主窗口；否则执行唤醒恢复并置顶聚焦。

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

历史已确认关闭条目（`BUG-057` ~ `BUG-316`）完整追溯映射详见 `clash_mini_agreements.md`「附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表」。

| 缺陷编号 | 对应版本基线 | 状态 |
| :--- | :---: | :---: |
| **BUG-316** | v3.2.5 | 代码已修正，已确认 |
| **BUG-315** | v3.2.4 | 代码已修正，已确认 |
| **BUG-314** | v3.2.3 | 代码已修正，已确认 |
| **BUG-312** | v3.2.2 | 代码已修正，已确认 |
| **BUG-311** | v3.2.1 | 代码已修正，已确认 |
