# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：`BUG-057` ~ `BUG-316` 已通过用户验收并完整归档于 `clash_mini_agreements.md` 附录 A。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs: BUG-317 ~ BUG-367)

### BUG-317（对应规范章节：5.5）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Critical / 网络探测与自愈
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:289-308, 1298-1347`
- **根因与背景实证**：`is_physical_network_online` 顺序串行 `await` 三个公网 DNS `TCP:53` 地址且未设单连接超时，在静默丢弃 53 端口的企业/校园/酒店网络下首个连接耗尽全部 2s 预算，导致备选 DNS 与 `lookup_host` 域名解析兜底永不执行，后台健康巡检与断流自愈停摆。
- **修正与改良方案**：重构为并行竞争探测（每个 `TCP:53` 探针独立 `600ms` 超时预算），任一连通即刻返回 `true`；若 `TCP:53` 均不可达则在剩余预算内立即回退执行 `lookup_host` 域名解析兜底。

### BUG-318（对应规范章节：5.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 自愈记账与告警
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:1602-1960`
- **根因与背景实证**：`self_heal_with_accounting` 在 `auto_select_fail_count >= 3` 时无论内核 API 是否可达均先将计数器清零（且 `RestartCore` 分支同样清零告警计数），导致全节点不可达或内核重启后仍持续不可达时，协议 5.3 节规定的连续 5 次失败 Win32 告警弹窗永远无法触发。
- **修正与改良方案**：通过 `evaluate_self_heal_counters` 与 `complete_restart_core_attempt` 解耦内核 API 失活重启计数与全节点断流告警计数：API 不可达满 3 次触发 `RestartCore` 时，若内核重启成功（`Ok`）则保留 `auto_select_fail_count` 继续累计至第 5 次可靠触发系统告警；若内核重启自身报错（`Err`）则按不可恢复终态立即升级触发系统告警并重置本轮双计数（含原子弹窗防重入及成功/失败双分支单测锁定）。

### BUG-319（对应规范章节：3.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 内核在线升级
- **涉及物理路径与行号**：`src-tauri/src/core/core_updater.rs`, `src-tauri/src/core/manager/state.rs`, `src-tauri/src/core/service.rs`, `src-tauri/src/core/validate.rs`
- **根因与背景实证**：内核在线升级在解压前直接 `fs::File::create` 打开正式路径 `cores/mini-mihomo.exe` 将其截断为 0 字节；若下载中断或解压异常则残留损坏文件触发 `0xc000007b` 致命错误，且各消费点校验口径不一、瞬时文件锁易误删完好内核。
- **修正与改良方案**：改为写入 `.staging` 临时文件，经共享 `CoreUpdater::inspect_mihomo_pe_binary` 完整校验体积（`>= 5 MB`）、DOS `MZ`、`e_lfanew`、PE32+ AMD64 COFF/OptionalHeader（`Machine=0x8664`、`Magic=0x020B`）与节表边界，经共享 `wait_for_mini_cores_exit` 确认旧内核退出释放锁后再原子替换；`state.rs`、`service.rs`、`validate.rs` 三处统一接入 `resolve_usable_custom_core`，严格区分 `Corrupted`（删除并记日志）与 `IoUnavailable`（杀软扫描/瞬时锁占用时保留原文件并回退内置 sidecar）。

### BUG-320（对应规范章节：7.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 通用协议解析器
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/universal_parser.rs`
- **根因与背景实证**：`parse_ss` 与 `parse_wireguard` 在剥离 `?` 查询串前先按 `@` 切分导致含 `@` 查询参数截断主机地址；`parse_socks5` 漏剥离 `?` 查询串导致端口解析失败；`VMessJson` 缺失 `sni` 字段反序列化映射导致 CDN 域名前置节点 TLS 握手失败。
- **修正与改良方案**：统一遵循「先剥离 `#fragment` -> 再剥离 `?query` -> 最后按末尾 `@` 拆解 `userinfo@host:port`」顺序修复全部 9 类 URI 协议解析器，并在 `VMessJson` 中补齐 `sni` 映射至 `servername`。

### BUG-321（对应规范章节：2.8 / 5.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Medium / 过滤子集测速调度
- **涉及物理路径与行号**：`src/pages/_layout/utils/profile-coordination.ts:97-140`, `src/services/delay.ts`
- **根因与背景实证**：`batchTestWithFirstBatchSelect(names, select)` 将前端子集卡片置为 `-2` 测速态后，调用 `triggerAutoSelect(currentUid, undefined, 0, select)` 误传硬编码 `undefined`，导致子集参数丢失及未覆盖卡片转圈残留。
- **修正与改良方案**：将第二个参数修正为传入的 `names`（`triggerAutoSelect(currentUid, names, 0, select)`），并在后端返回空数组（未执行）或异常分支通过 `restoreDelaySnapshot` 回滚 `-2` 加载态且保留原始 `updatedAt`。

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
- **涉及物理路径与行号**：`src-tauri/src/config/prfitem.rs`, `src-tauri/src/utils/network.rs`
- **根因与背景实证**：`validate_url_no_ssrf` 静态阶段未拦截单标签主机名及 `.localhost`、`.local`、`.internal`、`.lan`、`.home.arpa` 等内网保留后缀，动态解析阶段漏检 IPv6 `fc00::/7`（ULA）与 IPv4-mapped 私网地址，且缺少直连建连期 DNS 重绑定防护与 30x 重定向逐跳校验。
- **修正与改良方案**：静态与预解析补齐单标签主机名、内网保留后缀、IPv6 ULA 与 IPv4-mapped 私网拦截；`NetworkManager::get_with_ssrf_guard` 在直连路径挂载连接期异步 `SsrfSafeDnsResolver` 消除 DNS Rebinding TOCTOU，在直连与代理双路径挂载 `30x` 重定向逐跳 `validate_url_no_ssrf` 复验；本地预解析 `Err` 放行以便被污染订阅域名交由 `127.0.0.1:10801` 代理回退链拉取，并明确登记远端代理隧道解析边界。

### BUG-328（对应规范章节：1.3 / 1.8）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 服务状态并发与 TUN 保护
- **涉及物理路径与行号**：`src-tauri/src/core/service.rs`, `src-tauri/src/core/manager/lifecycle.rs`, `src-tauri/src/config/verge.rs`, `src-tauri/src/feat/config.rs`
- **根因与背景实证**：`ServiceManager::run_operation` 使用单一 `AtomicBool` 防重入导致并发状态查询直接报错，且 `lifecycle.rs` 在已启用系统服务冷启动瞬时响应超时时改写内存/磁盘 `enable_tun_mode = false`，后续任意配置落盘均会覆盖用户持久化的 TUN 配置。
- **修正与改良方案**：将 `ServiceManager` 操作门控升级为异步互斥排队；通过非阻塞 `inspect_service_registration_async()` 区分 `NotInstalled`、`DisabledOrBroken`（`Start=4` 禁用或二进制缺失，跳过 30s 空等并持久回退）与 `InstalledAndEnabled`；已启用服务冷启动超时进入 `IVerge::enter_temp_tun_fallback` 会话锁定态，确保后续改语言/皮肤等任意落盘仍保留磁盘原始 `enable_tun_mode` / `enable_system_proxy`（用户显式切换代理开关时自动解锁）。

### BUG-329（对应规范章节：3.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 命名管道半开连接自愈
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/ipc.rs:274-277, 502-525`
- **根因与背景实证**：内核重载或空闲断开命名管道后，连接池 `sender.is_ready()` 仍可能为 `true`，首次复用即抛 `HttpParseError` 且无透明重连重试，引发前端查询偶发红框报错。
- **修正与改良方案**：在本地管道请求发送层针对池化复用连接且仅限幂等只读方法（`GET` / `HEAD`）失败时自动驱逐旧连接并新建物理管道透明重发 1 次；非幂等写操作绝不自动重发，杜绝内核侧重复执行指令。

### BUG-330（对应规范章节：3.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 命名管道繁忙退避互锁
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/ipc.rs:113-145, 500-503`, `crates/tauri-plugin-mihomo/src/mihomo.rs:32`
- **根因与背景实证**：命名管道 `ERROR_PIPE_BUSY` 24 次固定休眠退避总耗时 `3000ms` 与旧外层超时 `3000ms` 等长互锁，后半程建连成功亦来不及完成 HTTP 交互即被掐断。
- **修正与改良方案**：集成 Win32 `WaitNamedPipeW` 原生事件唤醒等待并收敛单次最大等待步长，配合 `5s` 外层超时彻底消除互锁掐断。

### BUG-331（对应规范章节：1.4 / 1.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 服务模式退出孤儿内核清理
- **涉及物理路径与行号**：`src-tauri/src/feat/window.rs:134-240`, `src-tauri/src/core/manager/state.rs:175-260`
- **根因与背景实证**：服务模式下退出清理若先走普通 `stop_core` 可能阻塞耗尽预算，且普通用户会话进程无法终止 Session 0 下 `SYSTEM` 权限的 `mini-mihomo.exe`，提权失败或模式漂移时易残留孤儿内核。
- **修正与改良方案**：退出前预先采样运行模式，当处于服务模式或开启 TUN 时**优先**通过 `service::stop_core_by_service()` 提权通道停止 Session 0 内核、经共享 `wait_for_mini_cores_exit` 确认进程退出并清理本地状态；提权超时/报错时回退 `CoreManager::stop_core()`（非服务模式若有残留且服务已安装亦补服务级兜底），且末尾始终无条件执行 `kill_all_mini_cores()`（对无权限 Session 0 进程优雅告警放行，同时兜底清理任意用户态残留内核）。

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

### BUG-336（对应规范章节：2.8 / 5.1）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 顶栏活跃节点轮换同步
- **涉及物理路径与行号**：`src/pages/_layout/components/active-node-card.tsx`, `src/hooks/use-proxy-selection.ts`, `src/services/cmds.ts`, `src-tauri/src/module/monitor.rs`
- **根因与背景实证**：顶栏活跃节点卡片点击快速轮换（`handleCycleNode`）切换内核成功后漏调 `['getProxies']` 缓存更新与 `patchCurrent` 持久化，且前端手动切点未第 `0ms` 唤醒后台监测吸收 `SWITCH_GRACE_PERIOD_SECS` 宽限期，存在轮询睡眠窗口隐患。
- **修正与改良方案**：切换成功后立即乐观更新并作废 `['getProxies']` 缓存、调用 `patchCurrent` 持久化写入 `selected`，并通过 `notify_node_switched`（带异常告警日志）置位时间戳且触发专属 `NODE_SWITCH_NOTIFY.notify_one()` 第 `0ms` 唤醒监测循环在所有早退分支前吸收宽限期（不触发额外全组拨测，且守护线程启动/重启时自动复位静态状态）。

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
- **当前状态**：`已撤销回退并收口全路径陈旧兜底防线，待用户确认`
- **类别与级别**：Improvement / 单节点专属微测速回退与全路径陈旧 history 防线加固
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs`, `src-tauri/src/core/notification.rs`, `src-tauri/src/core/handle.rs`, `src/services/delay.ts`, `src/pages/_layout/hooks/use-layout-events.ts`, `src/pages/_layout/utils/profile-coordination.ts`, `src/components/proxy/proxy-groups.tsx`
- **根因与背景实证**：原拟在非 TUN 模式下启用 `delay_proxy_by_name` 直测违背《唯一协议》§5.2 核心契约；且在单点、批量与后台自愈选点路径中，一旦测量组拨测未取得实时数据而退化读取 120s `history` 兜底，会导致陈旧延迟冒充实时结果甚至混入 `candidates` 切换出口，前端恢复或复用缓存时亦会误盖 `Date.now()` 30 分钟长寿命。
- **修正与改良方案**：完整撤回 `delay_proxy_by_name` 直测分流；后端重建 `DelayResultSource`（`LiveProbe` / `FreshMetricsCache` / `NotExecuted`）并彻底移除全路径 120s `history` 兜底；`verge://backend-delay-results` 携带 `fromProbe` 标记对 30s 复用数据强制封顶前端缓存寿命为 30s；新增 `restoreDelaySnapshot` 在未执行或异常回滚时严格保留原始 `updatedAt` 时间戳。

### BUG-345（对应规范章节：5.5）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 并行竞速物理网络探针
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs:289-308`
- **根因与背景实证**：串行物理网络探针在任一上游 DNS 抖动时增加数百毫秒至数秒无谓等待。
- **修正与改良方案**：使用 `futures::future::select_ok` 并行发起三路公网 DNS `TCP:53` 握手探针（单路 600ms 超时），最快一路握手成功即在 10~30ms 内判定在线，全败则秒级平滑衔接域名解析兜底。

### BUG-346（对应规范章节：3.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 内核升级原子暂存与真实 PE64 校验
- **涉及物理路径与行号**：`src-tauri/src/core/core_updater.rs`, `src-tauri/src/core/manager/state.rs`, `src-tauri/src/core/service.rs`, `src-tauri/src/core/validate.rs`
- **根因与背景实证**：内核在线升级缺乏完整 PE32+ 结构验证、旧内核进程句柄退出等待与瞬时 I/O 占用防误删保护。
- **修正与改良方案**：引入 `.staging` 原子暂存替换流水线，经 `inspect_mihomo_pe_binary` 校验二进制体积（`>= 5 MB`）、DOS `MZ`、`e_lfanew`、PE32+ AMD64 COFF/OptionalHeader 与节表边界，并严格区分结构损坏（`Corrupted`）与瞬时 I/O 不可读（`IoUnavailable`）；经共享 `wait_for_mini_cores_exit` 确认旧内核释放文件锁后原子替换，异常时自动清理临时文件并保留旧内核完好可用。

### BUG-347（对应规范章节：4.3）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 前端 WebSocket 指数退避重连
- **涉及物理路径与行号**：`src/hooks/use-mihomo-ws-subscription.ts`, `src/hooks/use-mihomo-ws-subscription.test.ts`
- **根因与背景实证**：内核重启或服务切换期间固定短周期重连产生高频无效 IPC/WS 风暴，且内核重启后旧 socket 句柄未主动关闭重连。
- **修正与改良方案**：将前端 WebSocket 断线重连升级为带随机抖动且严格以 `15s` 封顶的指数退避；监听 `verge://refresh-clash-config`（250ms 防抖合并 + `listenerEpoch` 防注册竞态）清空退避计数，并对仍持有陈旧 socket 句柄的活动通道主动关闭后立即重连，同时响应 `Close` 消息触发重连。

### BUG-348（对应规范章节：7.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 统一 URI 协议解析预处理管道
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/universal_parser.rs`
- **根因与背景实证**：各代理协议解析函数各自手工切分 `#` 与 `?`，容易在含 `@` 参数或尾斜杠边界下出现解析截断。
- **修正与改良方案**：封装共享纯函数 `strip_fragment_and_query(input)` 统一接入全部 9 类 URI 协议（`ss`/`trojan`/`vless`/`hysteria2`/`hysteria`/`anytls`/`tuic`/`wireguard`/`socks5`），并补齐全部 9 协议含 `@` 查询参数的 `server` 主机名与 `port` 端口双断言及 VMess `sni` 回归单元测试。

### BUG-349（对应规范章节：5.2 / 5.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 测速 0ms 超时置底全栈对齐与单测锁定
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs`
- **根因与背景实证**：后端展示矩阵与候选排序缺少独立纯函数封装与针对 `0ms` 超时节点的回归单测锁定。
- **修正与改良方案**：抽离 `sort_display_delays` 纯函数统一处理延迟升序（`0 -> u32::MAX` 垫底）与名称排序，并补充单元测试防止退化。

### BUG-350（对应规范章节：2.4）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 现代化 Win32 SetWindowSubclass 窗口钩子
- **涉及物理路径与行号**：`src-tauri/src/utils/resolve/window.rs:293-395`
- **根因与背景实证**：传统 `SetWindowLongPtrW(GWLP_WNDPROC)` 依赖全局静态旧过程指针，与其他窗口过程链及窗口销毁重建生命周期耦合脆弱。
- **修正与改良方案**：通过 `comctl32` 原生 `{SetWindowSubclass, DefSubclassProc, RemoveWindowSubclass}` 标准子类化架构绑定窗口生命周期，并在收到 `WM_NCDESTROY` 时自动解绑释放，原生支持任意次主窗口销毁与重建。

### BUG-351（对应规范章节：3.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 韧性命名管道连接池与 WaitNamedPipeW
- **涉及物理路径与行号**：`crates/tauri-plugin-mihomo/src/ipc.rs`
- **根因与背景实证**：`ERROR_PIPE_BUSY` 期间纯 `tokio::time::sleep` 盲轮询延迟偏高且无法感知管道即时空闲事件。
- **修正与改良方案**：在 Windows 平台引入 `WaitNamedPipeW` 阻塞线程池事件等待配合退避重试，并在连接池层记录连接复用标记对幂等查询实现陈旧断开连接的无感单次重建。

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
- **涉及物理路径与行号**：`src/pages/_layout/components/help-menu-button.tsx`
- **根因与背景实证**：`HelpMenuButton` 设置了 `@media (max-height: 830px) { display: none }`，导致默认窗口高度（680px）及主流 1080p/笔记本缩放屏下用户完全看不到 GitHub Star、Wiki、重新打开新手引导与导出诊断包入口。
- **修正与改良方案**：将 `max-height: 830px` 下的隐藏改为重定位至标题栏紧凑槽位，让默认窗口高度与常规屏幕用户随时可访问诊断导出、首次引导与项目主页。

### BUG-355（对应规范章节：2.8 / 5.2 / 5.6）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 补齐 F4 / F1 快捷测速择优与 Ctrl+F 搜索聚焦快捷键
- **涉及物理路径与行号**：`src/pages/_layout.tsx`, `src/utils/disable-webview-shortcuts.ts`, `src/components/proxy/proxy-groups.tsx`
- **根因与背景实证**：设计规范定义了 `F4`（可见子集测速择优）与 `F1`（全量测速择优），但主界面缺少统一收口且对称保护编辑态与全量弹窗遮挡的键盘快捷监听。
- **修正与改良方案**：并入 `disable-webview-shortcuts.ts` 单一 `document` 级 `keydown` 监听器，对 `Ctrl+F`、`F4`、`F1` 对称执行全量弹窗/菜单排除（覆盖抽屉、日志、编辑/导入/向导、客户端/内核升级、列管理与帮助菜单）、输入控件编辑态排除、IME 合成与长按连发保护（绝不误伤 `Alt+F4`/`Ctrl+F4`），并移除 `ProxyGroups` 硬编码 `'PROXY'` 兜底。

### BUG-356（对应规范章节：2.8 / 2.10）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 手动点击切换代理节点 0ms 乐观高亮
- **涉及物理路径与行号**：`src/hooks/use-proxy-selection.ts`
- **根因与背景实证**：手动点击节点卡片切换出口时需等待后端 IPC 与代理表重刷完成后高亮才移动，存在 200~800ms 视觉滞后。
- **修正与改良方案**：点击节点瞬间对 `queryClient` 的 `['getProxies']` 缓存执行乐观 `now` 更新实现 `0ms` 即时选中高亮，后台异步提交并在极少数失败场景下自动回滚缓存。

### BUG-357（对应规范章节：2.8 / 5.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 节点卡片延迟徽章独立点击单点测速热区
- **涉及物理路径与行号**：`src/components/proxy/proxy-item.tsx`
- **根因与背景实证**：节点卡片整行点击会直接切换当前出口节点，用户缺少在不中断当前活跃连接的前提下单独探测某一备用节点实时延迟的快捷入口。
- **修正与改良方案**：将节点卡片右侧延迟徽章设为独立点击热区（`e.stopPropagation()`），点击延迟数值仅对该节点执行单点实时测速（`select=false`）而不切换当前出口，点击卡片其余区域仍立即切换节点。

### BUG-358（对应规范章节：2.7）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 特色皮肤双滑块拖动渲染与持久化落盘解耦
- **涉及物理路径与行号**：`src/pages/_layout/hooks/use-skin-controls.ts`, `src/pages/_layout/components/theme-settings-card.tsx`
- **根因与背景实证**：拖动皮肤物理光影/色彩滑块时在高频 `onChange` 中同步反复读写 `localStorage` 并触发整树状态同步，造成快速拖动微卡顿。
- **修正与改良方案**：拖动过程 `onChange` 以内存态与 CSS 变量驱动 60fps 实时光影预览，同时结合 200ms 防抖去重落盘、`onChangeCommitted` 去重提交及窗口失焦 / `visibilitychange` / `pagehide` / 组件卸载多路冲刷，兼顾 60fps 丝滑手感与零配置丢失。

### BUG-359（对应规范章节：5.3 / 5.5）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Improvement / 全节点超时或物理断网时顶栏轻量状态胶囊与一键自愈
- **涉及物理路径与行号**：`src-tauri/src/module/monitor.rs`, `src/pages/_layout/components/active-node-card.tsx`, `src/pages/_layout/utils/notification-handlers.ts`
- **根因与背景实证**：当物理网络中断或后台测速发现全部节点超时不可达时，主界面顶栏缺乏直观的故障定性提示与一键重测恢复入口。
- **修正与改良方案**：后端监测在全节点不可达/物理离线及恢复（含选优成功）时派发状态事件，顶栏展示紧凑警示状态胶囊，点击重测成功后通过 `broadcastNetworkHealth` 同步全局模块态，且重测未执行、全超时或异常均给出明确 i18n 提示。

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

### BUG-362（对应规范章节：7.6 / 8.2）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 远程订阅默认 `User-Agent` 与 UI 版本号解耦及自定义逃生阀闭环
- **涉及物理路径与行号**：`src-tauri/src/constants.rs:4-12`, `src-tauri/src/utils/network.rs:78-92, 314-325`, `src-tauri/src/config/prfitem.rs:443-446, 596-606`
- **根因与背景实证**：`NetworkManager::create_request_with_tls_mode` 直接使用 `format!("clash-verge/v{}", env!("CARGO_PKG_VERSION"))`（当前发出 `clash-verge/v3.2.5`）作为默认请求头，将机场面板协议能力协商锚点与本分支高频递增的 UI 产品版本号强耦合，存在「外壳虚高、内核未升」诱发机场超前下发不兼容新协议、未来涨破 `v10.0.0` 触发粗糙面板字典序比较归零、偏离上游 `v2.x` 白名单遭反代拦截及暴露私有版本指纹四大隐患；同时 `PrfItem::from_url` 在构造返回 `PrfOption` 时漏存 `user_agent` 字段且未过滤空串，导致单订阅自定义 `option.user_agent` 逃生阀导入后落盘丢失。
- **修正与改良方案**：在 `constants::network` 定义单一事实源常量 `SUBSCRIPTION_COMPAT_USER_AGENT = "clash-verge/v2.5.10"`（已核实对应上游 `clash-verge-rev/clash-verge-rev` 2026-11 正式版 `v2.5.10` / 内置 `Mihomo v1.19.x` 同代基线，并在 `constants.rs` 注释写明选型依据与定值指纹权衡），与 `CARGO_PKG_VERSION` 及日常发版升号彻底解耦（随 `v3.2.6+` 合并发行）；`NetworkManager` 通过 `resolve_effective_user_agent` 对空白自定义值自动回退至该常量；补齐 `PrfItem::from_url` 对 `option.user_agent` 的清洗与持久化保留，并补充真实 HTTP 抓包与常量解耦回归单测，同步在 `§8.2` 发版清单补齐内核大版本升级核对项。

### BUG-363（对应规范章节：4.1 / 4.2 / 5.1）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Critical / 轻量模式防强杀四重闭环（异名物理隔离看门狗 + 双向互锁监督 + 双端工作区哨兵窗；原 120s 稳态代数复位已在 BUG-365 废除）
- **涉及物理路径与行号**：`src-tauri/src/process/guardian.rs:1-1436`, `src-tauri/src/module/lightweight.rs:99-107, 170-177`, `src-tauri/src/utils/window_manager.rs:447-460, 568-573, 617-624`, `src-tauri/packages/windows/installer.nsi:551-586, 944-947, 1132-1143`
- **根因与背景实证**：`v3.2.6` 现场运行 2 小时 30 分钟后（`21:05:24` ~ `23:34:59`），用户唤醒后再次关闭窗口进入轻量模式，叠加整机负载飙升至 `CPU 100% / 内存 94%` 触发 `360tray.exe`（`360perfoptm2.dll` `switch_smart_mode=1`, `cpu_exception=80`）高强度清理，主进程 `PID=12688` 与伴生看门狗 `PID=21012` 双双零日志猝死（Session 0 下 `clash-verge-service.exe` 与 `mini-mihomo.exe` 始终存活）。根因在于 `BUG-316` 初版看门狗直接以同名同路径 `clash-mini.exe --guardian` 无窗口（`CREATE_NO_WINDOW`）启动且主进程丢弃 `Child` 句柄不监控看门狗死活——导致无窗口看门狗要么在前期残留扫描中被先行单杀使主进程裸奔，要么在按 `clash-mini.exe` 应用组清理（`CGroupInfo` / `DoOptmGroupResidual` / `FastOptm`）时与主进程同组连坐双杀；同时原哨兵窗口采用 `(-32000, -32000)` 屏外最小化特征坐标、`LWA_ALPHA = 0` 与空标题，易在极限负载下被穿透。
- **修正与改良方案**：实施四重纵深闭环加固：① **看门狗异名异目录物理隔离**：按需经 `.staging` 原子同步至 `%LOCALAPPDATA%\<APP_ID>\guard\clash-mini-guard.exe`（占用锁定时平滑落入 `clash-mini-guard-alt.exe`），传入 `--target-exe` 并脱离父 Job 独立拉起，彻底免疫按 `clash-mini.exe` 应用组批量强杀且不占用安装目录文件锁，同步在 `installer.nsi` 安装/卸载流程补齐停机与清理；② **主进程与看门狗双向互锁监督**：主进程通过独立内核态阻塞线程（`WaitForMultipleObjects([child, exit_event], INFINITE)`，`0%` CPU）反向监控看门狗，主动退出 `0ms` 唤醒，看门狗遭外部误杀时记录退出码并带退避自动重建；③ **主进程与看门狗双端挂载工作区哨兵窗口**：哨兵窗迁入主屏工作区原点 `1×1`、标题非空、`LWA_ALPHA = 1`（`1/255` 肉眼不可见、`WM_NCHITTEST -> HTTRANSPARENT` 穿透、隐藏 Owner 不占任务栏/Alt+Tab），看门狗同步挂载哨兵窗并以 `MsgWaitForMultipleObjects(..., QS_ALLINPUT)` 即时响应防 `IsHungAppWindow` 判死，同时将隐藏至托盘态一并纳入守护信号；④ **稳态代数复位初版**（`v3.2.7/v3.2.8` 初版曾定 `120s` 复位，已在 `BUG-365` 复审中废除并替换为无时间清零的 OS 启动指纹+会话纪元累计熔断）。（注：初版已随 `v3.2.7` / `v3.2.8` 合并发行，但 `v3.2.8` 现场仍复现双进程消失并暴露可观测性、Job 隔离与门控缺口，后续系统性架构重构见 `BUG-365`）。

### BUG-364（对应规范章节：3.7）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Critical / TUN 模式域名嗅探与订阅 DNS 策略卫生
- **涉及物理路径与行号**：`src-tauri/src/enhance/tun.rs`, `src-tauri/src/enhance/mod.rs`, `src-tauri/src/config/verge.rs`, `src-tauri/src/cmd/verge.rs`, `src-tauri/src/feat/config.rs`, `src/pages/_layout/components/basic-settings-card.tsx`, `src/types/verge.ts`, `src/locales/*/settings.json`
- **根因与背景实证**：`v3.2.7` 现场取证发现系统代理模式与 Chrome 均可正常翻墙，唯常驻后台进程的 Edge 在 TUN 模式下全部外网超时（同机无痕/新启动 Edge 则可正常访问 Google）。日志与命名管道快照（`\\.\pipe\mini-mihomo`）证实双重原因：① **TUN 模式缺少 `sniffer` 域名嗅探配置**：Edge 后台网络进程（`--no-startup-window`）跨模式切换常驻期间持有了直连或路由器 IPv6 RA DNS（`fe80::1%3`）返回的真实/GFW 投毒 IP（如 `2001::1`、`69.171.235.22`），连接绕过 Fake-IP 且请求无 `Host`，由于未开启 `sniffer`，连接既无法命中 `RULE-SET,gfwlist,PROXY` 也未覆写投毒目标地址，全部直落 `MATCH,DIRECT` 刷 `i/o timeout`；② **机场订阅 `dns.nameserver-policy` 脏策略未做卫生清理**：如 `RtLvP3j0CQIN.yaml` 将 `+.google.com` 等 15 个海外域名固定指派给 `https://dns.cloudflare.com/dns-query` 且未开启 `respect-rules: true`，导致内核在大陆直连被墙 DoH 节点，非 A/AAAA 查询（实测 `TXT`）331ms 内直接返回 `SERVFAIL`（`DNS server failure`）。原 `v3.2.8` 首版修正存在默认分支删除策略键、`dns_host_proxy_rule` 丢失 BUG-302 对偶辅 IP、规则插入在 `GEOIP/MATCH` 前却晚于部分规则、`sniffer.enable` 无条件覆写等遗留缺陷。
- **修正与改良方案**：① **TUN `sniffer` 三态开关与权威字段锁定**：在 `basic-settings-card.tsx`、`cmd/verge.rs` 与 `feat/config.rs` 实现 `Auto / On / Off`（`null / true / false` + 非 TUN 模式禁用）三态控制及显式 `null` 复位，`enhance::tun::apply_tun_sniffer_policy` 在启用时**权威强制** `force-dns-mapping: true`、`parse-pure-ip: true` 并统一顶层与 `sniff.HTTP` 的 `override-destination`（杜绝订阅残留关闭值造成空壳嗅探），同时在 `Auto` 态尊重订阅显式 `sniffer.enable: false`（`insert_if_missing` 不强盖）、扩充 `skip-dst-address` 覆盖组播/广播/IPv4-mapped 私网与国内 DNS；② **DNS 三域统一治理与内外判据统一**：由 `sanitize_dns_upstreams` 统一处理 `dns.nameserver`、`dns.fallback` 与 `dns.nameserver-policy`（保留 `nameserver-policy` 全部策略键与 `default-nameserver` 明文），以 `is_local_or_domestic_dns_host` 作为国内外/局域网唯一判据（阻止国内 DNS 进入 `PROXY` 规则及 `fallback` 误判），并剥离 `#DIRECT`/`#PROXY` 伪后缀（保留 `#h3=true`）；在 `respect-rules: false` 或 `#DIRECT` 下就地重映射为境内 DoH，在 `respect-rules: true` 下经共享规则生成器于 `RULE-SET,gfwlist` 后、`append-rules` 前插入 `PROXY` 保通规则。

### BUG-365（对应规范章节：4.1 / 4.2 / 5.1 / 8.7）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：Critical / `v3.2.8` 轻量模式双进程静默消失 Session-0 会话隔离、可观测性与守护闭环系统性加固
- **涉及物理路径与行号**：`src-tauri/Cargo.toml`, `src-tauri/src/process/guardian.rs`, `src-tauri/src/module/lightweight.rs`, `src-tauri/src/module/monitor.rs`, `src-tauri/src/utils/resolve/mod.rs`, `src-tauri/src/utils/window_manager.rs`, `src-tauri/src/feat/window.rs`, `src-tauri/src/cmd/diagnostics.rs`, `src-tauri/src/lib.rs`
- **根因与背景实证**：`v3.2.8` 会话日志（归档 `2026-11-13_12-29-24.log`：于 `2026-11-13 10:39:56` 启动、`10:45:39` 再次进入轻量模式并在 `11:10:18` 最后一条日志后，原勘误曾误记为 `11:06:16`）主进程与 `clash-mini-guard.exe` 再次无声消失（Windows Application/System 事件日志无崩溃，WER 无转储）。现场取证确认：`360tray.exe` 智能加速（`switch_smart_mode=1`, `cpu-max=100`）激活、`ScanLog_2026-11-13_10_43_42.txt` 拦截过自启且 `useroptm2.ini` 缺失 `clash-mini` 豁免、`speedupopt.db` 已含 `clash-mini-guard` 优化记录、活跃主进程处于 Win32 Job 内（`IsProcessInJob=True`）。代码审查与复审进一步暴露：**原实现仅在可观测与死亡后判决着墨，未解决看门狗未脱离父 Job 遭 360 按进程组连坐双杀的防护根因**；同时存在带外凭证缺少 `pid && session_id` AND 匹配/TTL/两阶段（`prepared`/`confirmed`）、`3600s` 时间清零漏洞与写状态吞错、同名 `.tmp.<pid>` 竞争与 delete-then-rename 竞态、日志/错误静默出口及诊断包缺少 360/WER 事件采集等问题。
- **修正与改良方案**：按评审报告 `A1–A12` + 复审 `C1–C15` 落地全链路根因隔离与纵深加固：① **A6 Session-0 Broker 会话脱离隔离（核心防双杀）**：看门狗与复活主进程优先经 Session-0 **WMI (`Win32_Process.Create` → `WmiPrvSE.exe`)** → **Task Scheduler COM (`ITaskService`) 一次性任务** → **`CREATE_BREAKAWAY_FROM_JOB`** 级联拉起（彻底脱离父 Job，同 Job 回退显式标记 `same_job_degraded` 告警）；新增真实 `CreateJobObjectW` + `TerminateJobObject` 场景 6 端到端测试验证脱离父 Job 存活；② **A7 + C1–C2 两阶段严格带外退出双真源**：`prepare_exit()` 写 `phase="prepared"`（30s TTL），`RunEvent::Exit` 在退出窗口升级 `phase="confirmed"`（120s TTL），带重试写入并在退出取消时双路复位；看门狗强制校验 `proof.pid == parent_pid && proof.session_id == session_id`（AND 匹配 + TTL）并在 `GUARD_SAFETY_DELAY_ENTER = 3500ms` 前后双次复核；③ **A8 + C3–C4 无时间清零会话熔断（fail-closed）**：废除 `120s` 与 `3600s` 冷却清零，按 `boot_fingerprint + session_epoch_id` 持久化单会话累计计数（上限 `<= 3` 次），读写失败一律 fail-closed 停机熔断；④ **C5–C7/C15 原子 I/O 与非静默审计**：全量文件替换为递增 `ATOMIC_TEMP_SEQ` 唯一临时文件 + `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`；独立 `logs/guardian.log` 写失败触发 stderr/`tracing`/主日志回退，补齐 `GetExitCodeProcess`、降级、早期 `cancel`、`lightweight` 状态对齐等全分支日志；⑤ **A4 代码化取证**：`cmd/diagnostics.rs` 新增只读采集 360 进程/扫描豁免列表、CrashDumps 与有界超时 `wevtutil` WER/Application 事件输出至 `wer-crash-report.txt` 与 `system-info.txt`。

### BUG-366（对应规范章节：3.7 / 4.1 / 8.5）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / IPv6 全代理偷跑治理与审核遗留 Clippy / TOCTOU 清理
- **涉及物理路径与行号**：`src-tauri/src/enhance/mod.rs:694, 1296, 1495-1522, 1808-1860`, `src-tauri/src/process/guardian.rs:786-880, 1658-1666`, `updater/app-update.json:3`
- **根因与背景实证**：① `enforce_mini_agreements` 在 GFWList 模式（`rule_fallback != "proxy"`）末尾的 `GEOIP,CN,DIRECT` 与 `MATCH,DIRECT` 之间强行注入 `IP-CIDR6,::/0,PROXY`，导致未覆盖于 `GeoIP/CN` 或直连 CDN/下载/通讯应用的 IPv6 流量及非 GFWList 域名 AAAA 解析全部绕过默认直连策略进入付费机场节点计费；② 提交审核同时检出 `enhance/mod.rs:1296` 悬空 `///` 文档注释（触发 `clippy::empty_line_after_doc_comments` 阻断严格 Lint）、`enhance/mod.rs:694` 冗余 `.unwrap()`、`guardian.rs:1660` 冗余 `if !allow_degraded` 分支（触发 `clippy::needless_return`）及 `evaluate_and_record_respawn_in` 先 `exists()` 再 `fs::read()` 的窄 TOCTOU 窗口；③ `updater/app-update.json:3` 发布说明仍滞留 `"Clash Mini v3.2.9"`。
- **修正与改良方案**：① 移除 GFWList 模式下 `MATCH,DIRECT` 前的 `- IP-CIDR6,::/0,PROXY`（保留 `GEOIP,CN,DIRECT` + `MATCH,DIRECT` 对称兜底，仅当用户切换「代理全部流量」即 `rule_fallback == "proxy"` 时落入 `MATCH,PROXY`），并同步更新 `test_enforce_mini_agreements_logic` 断言与协议规范 `§3.7`；② 将 `enhance/mod.rs:1296` 改为普通 `//` 注释、用 `split_once('#')` 安全解包替换 `.unwrap()`；③ 将非干净冷启动缺失状态文件判定收敛进 `read_respawn_state_checked` 的单次 `fs::read`（`NotFound` 分支按模式裁决）消除 TOCTOU 并补齐单测，同时精简 `guardian.rs:1660` 单出口返回；④ 将 `updater/app-update.json` 的 `notes` 对齐为 `"Clash Mini v3.3.0"`。

### BUG-367（对应规范章节：2.2 / 7.7 / 8.9）
- **当前状态**：`代码已修正，待用户确认`
- **类别与级别**：High / 路径控制精准规则目标化与四态路由可逆闭环
- **涉及物理路径与行号**：`src/utils/quick-routing.ts`, `src/utils/path-control.ts`, `src/hooks/use-path-control.ts`, `src/components/connection/connection-table.tsx`, `src/components/connection/connection-detail.tsx`, `src/pages/_layout/components/connections-panel.tsx`, `src/pages/_layout.tsx`, `src/locales/*/connections.json`, `src/locales/*/profiles.json`, `src/types/generated/i18n-keys.ts`
- **根因与背景实证**：① `addQuickRoutingRule` 与 `buildProxyRule` 存在自动截断域名父级与强制后缀化缺陷（`parts.slice(-2/-3)` + `DOMAIN-SUFFIX`）：用户在「路径控制」表封锁 `play.googleapis.com` 时被擅自改写为 `DOMAIN-SUFFIX,googleapis.com,REJECT`（或宽泛后缀匹配），误杀同名或上/下级无关域，且 IPv6 字符判定缺少 `a-f` 并错误套用 `IP-CIDR`；② `ConnectionTable` 仅消费 Mihomo WebSocket 实时活跃/内存历史流：`路由` 列仅能映射 `直连 / 封锁 / 代理` 三者之一，手动「断开此连接」仅调用 `closeConnectionWithTimeout` 使连接从活跃表瞬间消失；手动「封锁」后置顶的 `REJECT` 规则未关闭旧 socket，而新触发的 `REJECT` 连接被内核瞬时关闭落入易失历史区；表格也缺少直接解除 `Merge` 封锁或重连已断开条目的「恢复默认路由」操作。
- **修正与改良方案**：① **精确目标匹配**：路径控制右键 `全局直连 / 代理分流 / 封锁它` 一律生成精确规则——域名 `DOMAIN,<exact_host>,<TARGET>`（禁止截断父域、禁止自动转 `DOMAIN-SUFFIX`）、IPv4 `IP-CIDR,<ip>/32,<TARGET>`、IPv6 `IP-CIDR6,<ip>/128,<TARGET>`、进程 `PROCESS-NAME,<proc>,<TARGET>`；「添加网址」中仅显式 `*.domain`/`.domain` 输入生成 `DOMAIN-SUFFIX`，纯域名同样保持精确 `DOMAIN`；② **先重载后切流 + 四态可逆呈现**：下发 `DIRECT/PROXY/REJECT` 先 `enhanceProfiles()` 再关闭命中该目标的存量活跃连接；前端融合持久化「手动已断开记录」（新活跃流到达时自动转正消除）与 `Merge` 覆写规则集条目，在「路径控制」表统一呈现 **直连 / 代理 / 封锁 / 已断开** 四态徽标与排序，且使已断开及被封锁链接留驻表内、右键直接支持重路由或一键「恢复默认路由」。

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
