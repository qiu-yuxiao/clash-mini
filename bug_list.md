# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：历史已验收条目详见 [Changelog.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Changelog.md) 与 Git 提交历史。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-419 「添加规则」缺失直连/封锁入口且路径控制右键缺乏一键整站根域名作用域切换

- **对应模块**：[quick-routing.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/quick-routing.ts)、[routing-preference-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/routing-preference-card.tsx)、[add-url-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/add-url-dialog.tsx)、[connection-table.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx)、[path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/path-control.ts)、[use-path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-path-control.ts)
- **问题描述**：`BUG-367` 为防止封锁 `play.googleapis.com` 误扩散至 `*.googleapis.com` 而将右键菜单收紧为仅生成精确 `DOMAIN` 规则，且左侧代理方式第三档「添加网址」写死只能生成 `PROXY` 规则而无法选择 `DIRECT` 或 `REJECT`；这导致用户需要为 Oracle 等多子域名站点整站直连时无界面入口可走，只能逐个子域名反复右键试错。
- **修复方案**：
  1. 在 [routing-preference-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/routing-preference-card.tsx) 与 13 语种资源中将代理方式第三档更名为 **`添加规则`**（保留 `verge.yaml` 配置枚举 `'addurl'` 兼容），并在 [add-url-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/add-url-dialog.tsx) 顶部新增 3D 三选一滑块 **`[ 代理 | 直连 | 封锁 ]`**（默认 `代理`），提交后自动按新规则关闭匹配活跃连接；
  2. 在 [quick-routing.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/quick-routing.ts) 新增 `extractRegistrableDomain`（内置 `.co.uk`/`.com.cn`/`.github.io` 等公共后缀识别与裸后缀拦截）与 `buildQuickRuleFromInput`：输入裸根域名（如 `oracle.com`）或显式通配（`*.oracle.com`）生成 `DOMAIN-SUFFIX`，具体子域名（如 `play.googleapis.com`）严格生成精确 `DOMAIN`；`upsertPrependRule` 在写入 `DOMAIN-SUFFIX` 时自动收编下层冗余子域名规则并保留后续精确例外置顶覆写能力；
  3. 在 [connection-table.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx) 右键菜单顶部新增 **`[ 仅此域名 | *.根域名 ]`** 切换条（默认锁定 `仅此域名`，零误伤单子域封锁），切换后可一键下发整站 `DOMAIN-SUFFIX` 并联动 [path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/path-control.ts) / [use-path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-path-control.ts) 清理整站已断开记录与匹配活跃流，同步补齐 13 语种文本及单测。
- **状态**：代码已修正，待用户确认

### BUG-418 路径控制表活跃连接与已关闭/已封锁链接混排，需在「链接目标」表头同排右侧增加二选一开关隔离

- **对应模块**：[path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/path-control.ts)、[use-path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-path-control.ts)、[connections-panel.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/connections-panel.tsx)、[connection-table.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx)、[path-control.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/path-control.test.ts)
- **问题描述**：当前设置页右侧「路径控制」表格将实时活跃连接、近期已关闭连接以及 `Merge` `REJECT` 封锁/置顶规则混合渲染在同一列表内，长流实时连接与已结束死链/封锁项混杂在一起导致日常排查不够直观清爽；若在结果为空时整体替换为 `<BaseEmpty />` 还会令嵌入式表头开关随空表一起消失。
- **修复方案**：
  1. 将 [path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/path-control.ts) 合表函数重构为 `partitionPathControlConnections` 与 [use-path-control.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-path-control.ts) 双视图输出：**左侧「活跃」** 仅显示当前实时 `直连 / 代理` 连接；**右侧「已关闭」** 顶部固定置顶 `封锁 (REJECT)` 规则（重启后仍可一眼定位解封），其后依次为手动 `已断开` 记录、无活跃流的自定义规则及按目标去重的近期已关闭连接（自动排除活跃视图中已在线的目标）；
  2. 在 [connection-table.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx) 「链接目标」列头同排右侧嵌入紧凑 3D 二选一开关（`活跃 / 已关闭`，点击阻止冒泡隔离排序），由 [connections-panel.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/connections-panel.tsx) 始终保持表格挂载（空态在表头下方呈现 `<BaseEmpty />`），并补齐 13 语种 `active` / `closed` 翻译与 [path-control.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/path-control.test.ts) 单测覆盖。
- **状态**：代码已修正，待用户确认

### BUG-417 备用测速 URL 误赦免坏节点、服务模式启动竞态未对账、首次观测伪宽限期与客户端升级单路断流无日志

- **对应模块**：[probe.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/probe.rs)、[verdict.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/verdict.rs)、[daemon.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/daemon.rs)、[service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs)、[lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)、[use-client-update.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-client-update.ts)、[client-update.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/client-update.ts)
- **问题描述**：
  1. [probe.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/probe.rs) 在主测速 URL 全超时后对整组打备用 URL 时，只要任意其他节点返回响应即无条件置 `primary_url_distorted = true`，而 [verdict.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/verdict.rs) 未核对当前活跃节点自身是否通过备用 URL 便直接返回 `NoAction` 并清零失败计数，导致主/备双超时的坏节点被同组其他健康节点「替考免死」，长时间锁死 Antigravity 等长流请求；
  2. 升级重装或冷启动期间若 `clash_verge_service` IPC 瞬时繁忙超时，[lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs) 将 `CoreManager::running_mode` 回滚为 `NotRunning` 后即使服务 1 秒后成功拉起内核也无运行期自动对账恢复机制，致使 `BUG-411`（缺管快重启）、`BUG-415`（`service_latest.log` 建连失败突发监控）与 `BUG-416`（网卡事件增量读取）整场失效；
  3. [daemon.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/daemon.rs) 在守护循环首次观测到活跃节点（`None -> Some`）时误当作节点切换开启 `20s` 切换宽限期吞掉首轮异常；且启动期 `select=false` 纯测速若已实证持久节点超时不可达而同组存在可达候选，未进入快速复核；
  4. [use-client-update.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-client-update.ts) 使用默认单路裸下且未在 `check()` / `downloadAndInstall()` 异常时调用 `frontendLog` 落盘，代理节点短暂抖动即中断报错且日志无可追溯记录。
- **修复方案**：
  1. 收紧 [probe.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/probe.rs) 备用 URL 有效真实节点响应判定，并在 [verdict.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/verdict.rs) 新增 `is_active_excused_by_primary_distortion`：仅当当前活跃节点自身通过备用 URL 时方免于判死；若仅其他节点通而活跃节点仍超时，同步向 UI 标红当前节点并照常推进故障裁决与切换；
  2. 在 [service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs)`::start_with_existing_service` 为 `start_clash` IPC 补充 3 次短退避重试，并在 [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs) 新增 `CoreManager::reconcile_running_mode_if_needed`，于守护就绪及周期循环探测到服务内核管道在线而状态仍为 `NotRunning` 时自动对账恢复 `RunningMode::Service`；
  3. 在 [daemon.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/daemon.rs) 仅对真实节点切换（`Some(old) -> Some(new)`）启用 `20s` 宽限期，且启动纯测速实证持久节点超时而同组另有可达节点时预置第 1 次失败并按 3 秒间隔二次复核（恢复即保活、再败即切换），同步补齐 [monitor/tests.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/tests.rs) 单测；
  4. 新增 [client-update.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/client-update.ts) 及 [client-update.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/client-update.test.ts) 接入 [use-client-update.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-client-update.ts)：检查阶段优先本地混合端口代理并在失败后自动回退直连/系统代理，下载流中断（`Finished` 前）自动退避重试最多 3 次，且全部失败/重试通过 `frontendLog` 落盘；
  5. 为 `[TUN] default interface lost/changed` 设立独立 `300s`（`TUN_EVENT_MAX_AGE_SECS`）回溯窗与 `60s`（`TUN_INTERFACE_CHANGED_TTL_SECS`）信号自动过期 TTL，将 `NodeStatus` 上收至 [monitor/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/mod.rs) 消除 `observer ↔ verdict` 双向依赖，统一订阅三阶段下载失败前后端 13 语种 `withClashProxyFailed` 提示路径（激活 `update_failed_even_with_clash` 通知处理），并校准 `Changelog.md`、架构/契约文档与前端注释中陈旧 `monitor.rs` 路径引用。
- **状态**：代码已修正，待用户确认

### BUG-416 订阅更新失败误判成功与快重试失效、诊断包漏采现行内核崩溃、网卡切换/断开免测盲区及前端陈旧注释收口

- **对应模块**：[profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/profile.rs)、[timer.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/timer.rs)、[diagnostics.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/diagnostics.rs)、[monitor/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/mod.rs)、[monitor/observer.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/observer.rs)、[monitor/verdict.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/verdict.rs)、[monitor/daemon.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/daemon.rs)、[delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts)
- **问题描述**：
  1. [profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/profile.rs)`::perform_profile_update` 在订阅三阶段下载全部失败时仅发送后端通知却返回 `Ok(is_current)`，[update_profile](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/profile.rs) 对配置校验失败分支亦同，导致：[timer.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/timer.rs) 误判更新成功而无法触发 `2` 分钟快速重试，前端手动更新误走成功分支重载未变配置并弹出绿色 `subscriptionUpdated` 提示；
  2. [diagnostics.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/diagnostics.rs) 收集 Windows WER Application 崩溃事件与 `%LOCALAPPDATA%\CrashDumps` 转储清单时仅匹配旧内核 `verge-mihomo`，漏采当前运行内核 `mini-mihomo` 与系统服务 `clash-verge-service` 崩溃记录；
  3. 后台守护线程增量消费内核日志时未识别 `[TUN] default interface lost by monitor` 与 `[TUN] default interface changed by monitor`，发生物理断网或默认网卡切换后仍可能被近 `60s` 的 `ONLINE_CHECK_FLOWING_SKIP_SECS` 或 `Flowing`/`Idle` 健康缓存继续免测滞留；
  4. [delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts) 第 394 行注释仍残留陈旧的 `PROXY__METRICS` `5` 分钟（`300s`）周期旧假定（实际为 `HISTORY_FALLBACK_MAX_AGE_MS` `45` 分钟容忍窗）。
- **修复方案**：
  1. 在 [profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/profile.rs) 中令三阶段下载全败与配置校验失败严格向上返回 `Err(...)`（仅在后台定时触发且非轻量模式下发送后端失败通知，手动更新由前端统一弹出唯一错误提示，不重载未变配置），并在 [timer.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/timer.rs) 提炼 `compute_next_schedule_minutes` 并补齐 `2` 分钟快重试单测；
  2. 在 [diagnostics.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/diagnostics.rs) 新增 `is_clash_mini_wer_target` / `is_clash_mini_dump_target`，统一覆盖 `clash-mini`、`mini-mihomo`、`clash-verge-service`、`verge-mihomo` 与 `msedgewebview2` 并更新单元测试；
  3. 在 [observer.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/observer.rs)、[mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/mod.rs)、[daemon.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/daemon.rs) 与 [verdict.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/verdict.rs) 新增 `KernelLogEvent` / `KernelReprobeReason` 与 `FORCE_ONLINE_RECHECK`：`lost` 与 `changed` 均立即清零 `LAST_FLOWING_TRAFFIC_AT` 并置位 `FORCE_ONLINE_RECHECK` 跳过 `60s` 免探窗口；`changed` 额外在 `60s` 冷却外触发一次活跃节点实拨复核，补齐 [monitor/tests.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor/tests.rs) 单元测试及全局静态并发串行锁；
  4. 校准 [delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts) 陈旧注释。
- **状态**：代码已修正，待用户确认

### BUG-415 代理节点建连握手超时盲区突破免测缓存、启动纯测速及 10 分钟稳态摘要可观测性补齐与陈旧注释校准

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[state.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/state.rs)
- **问题描述**：
  1. [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)`::sample_active_node_traffic` 仅统计 `/connections` 中已成功建连的活跃隧道，节点建连/握手阶段即超时的失败请求（如 `dial PROXY ... failed to create session: context deadline exceeded` / `i/o timeout`）不会进入 `/connections`；若同时存在存量长连接维持下行字节（`Flowing` 免测窗 `1800s`）或处于 `Idle` 免测窗（`900s`），新连接持续失败的半死状态最长可滞后 15~30 分钟才触发实拨复核；
  2. 轻量模式下启动期 `select: false` 纯测速刷新不输出完成结果，且后台守护线程稳态心跳仅为 `debug` 级别，导致正常运行 1 小时以上 `latest.log` 零日志输出，难以区分“稳态健康免测”与“任务卡死”；
  3. [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 多处注释仍残留 `PROXY__METRICS` 每 `300s` 自治轮询的旧假设（实际在 [enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs) 已收敛为 `interval: 1800` + `lazy: true` 且无业务规则引用）。
- **修复方案**：
  1. 在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 新增零常驻 WebSocket、零管道占用的代理建连失败突发追踪器（`Service` 模式按字节偏移增量读取 `service_latest.log` 完整行并兼容轮转重置，首次挂载与切换节点时锚定文件末尾及切换时刻；`Sidecar` 模式由 [state.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/state.rs) 标准输出/错误消费循环直喂），精准匹配 `dial PROXY ` 握手/会话/超时错误（排除 `DIRECT`、`PROXY__METRICS`、`operation was canceled` 与 DNS 无记录噪声）；当 `DIAL_FAIL_WINDOW_SECS (30s)` 内累计 `>= DIAL_FAIL_REPROBE_THRESHOLD (3)` 次且不在 `DIAL_FAIL_REPROBE_COOLDOWN_SECS (60s)` 冷却期时，突破 `Flowing`/`Idle` 免测缓存立即触发一轮 `probe_metrics_group` 体检复核（最终切点仍严格遵守连续 2 次判死 / 间歇黑洞统一裁决）；
  2. 为 `trigger_backend_auto_select_inner` 的 `select == false` 纯测速路径补齐 `info` 级完成摘要日志（待测数、可达数、数据来源、当前节点及其实测延迟），并在 `start_background_monitor` 新增每 `MONITOR_SUMMARY_INTERVAL_SECS (600s)` 一次的零额外 IPC `info` 级周期状态摘要；
  3. 同步修正 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 中 5 处关于 `PROXY__METRICS` `300s` 自然周期的陈旧注释，并补齐日志解析、增量文件偏移读取（含截断轮转与半行保留）及免测突破门控的单元测试。
- **状态**：代码已修正，待用户确认

### BUG-414 自启计划任务判存迁移、单实例端口独占与唤醒验真、选点任务可中止及构建告警/陈旧注释收口

- **对应模块**：[autostart.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/autostart.rs)、[installer.nsi](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi)、[server.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/server.rs)、[lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)、[state.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/state.rs)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[ipc.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/ipc.rs)、[config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/config.rs)、[index.scss](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss)、[_theme.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_theme.tsx)
- **问题描述**：全项目验收审核发现四处工程健壮性与可维护性瑕疵：① [autostart.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/autostart.rs) 对管理员权限历史计划任务删不掉时仍无条件落盘 `.autostart_task_migrated` 导致永久跳过，且安装器未顺带清理历史任务；② [server.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/server.rs) 残留禁用但误导的 `SO_REUSEADDR` 注释、唤醒端口未校验响应状态与 `--silent` 探活、且瞬时释放期缺少短退避重绑；③ [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs) `spawn_ensure_proxy_group_selection` 丢弃 `JoinHandle` 且未监听 `MANUAL_SWITCH_EPOCH` 手动切点退让；④ `font.scss` 引用不存在的 `Twemoji.Mozilla.ttf` 触发构建告警、[ipc.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/ipc.rs) `max_connections` 注释过时、`config.rs` 残留已实现的 `TODO(config-version)`，以及 `ALERT_DIALOG_OPEN` 复位缺少异常守卫。
- **修复方案**：
  1. 在 [autostart.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/autostart.rs) 基于 `%SystemRoot%\System32\Tasks\<name>` 判存与 `done` / `pending_admin` 双态标记避免永久跳过提权任务，并在 [installer.nsi](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi) 安装/卸载段静默清理历史计划任务；
  2. 在 [server.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/server.rs) 修正 Windows 独占绑定注释、新增 `/commands/ping` 探活路由并校验 HTTP 响应状态后才静默退出，遇 `ConnectionRefused` 瞬时释放窗口执行 `3 × 150ms` 短退避重绑；
  3. 在 [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs) 与 [state.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/state.rs) 托管 `SELECTION_ENSURE_HANDLE`（新派发及停核时 `abort()`）并接入 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) `current_manual_switch_epoch()` 手动切点退让，同时用 `scopeguard::defer!` 守护 `ALERT_DIALOG_OPEN` 复位；
  4. 移除孤立 `font.scss` 与失效 `Twemoji.Mozilla.ttf` 引用，回退原生 `"Segoe UI Emoji"`，校准 `ipc.rs` 注释并清理 `config.rs` 陈旧 TODO。
- **状态**：代码已修正，待用户确认

### BUG-413 诊断包 WireGuard 密钥脱敏遗漏、内核升级强制 SHA-256 校验及运行核/沙箱权限收口

- **对应模块**：[diagnostics.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/diagnostics.rs)、[universal_parser.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/universal_parser.rs)、[core_updater.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/core_updater.rs)、[types.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/types.ts)、[verge.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/verge.rs)、[migrated.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/capabilities/migrated.json)、[desktop.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/capabilities/desktop.json)
- **问题描述**：全项目验收审核发现三项安全边界缺陷：① [diagnostics.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/diagnostics.rs) `SENSITIVE_KEYS` 仅包含 `psk` 而遗漏 WireGuard 标准键名 `pre-shared-key`（[universal_parser.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/universal_parser.rs) 写入键名）及 `short-id` 等字段，致导出诊断包外泄预共享密钥；② 上游 `MetaCubeX/mihomo` Release 不附带 `checksums.txt`（[core_updater.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/core_updater.rs) 总是走 `Ok(())` 跳过 SHA-256 校验），且未消费 GitHub API 自带 `asset.digest`，下载/解压亦缺少源域名白名单与大小上限；③ [verge.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/verge.rs)::`get_valid_clash_core` 未在运行期强校验 `VALID_CLASH_CORES` 白名单（可被 IPC 写入后污染 `LocalSystem` 服务路径），且 [migrated.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/capabilities/migrated.json) `fs:scope` 对 `$APPDATA/**` 授权偏宽。
- **修复方案**：
  1. 在 [diagnostics.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/diagnostics.rs) `SENSITIVE_KEYS` 与 `LOG_SENSITIVE_KV` 补全 `pre-shared-key`、`preshared-key`、`private-key-passphrase`、`short-id`、`obfs-param`、`protocol-param` 及 `private-key`/`psk` 脱敏规则与单测；
  2. 在 [core_updater.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/core_updater.rs) 与 [types.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/types.ts) 解析 GitHub Release `asset.digest`（`sha256:...`）与 `size`，缺失摘要或校验不符一律 Fail-Closed 拒装；锁定 `https://github.com/MetaCubeX/mihomo/releases/download/` 下载源，设置 `100 MiB` 下载上限与 `150 MiB` 解压上限，并移除 `stop_core` 前误置的恢复启动分支；
  3. 在 [verge.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/verge.rs) 运行时强制收敛 `clash_core` 为 `VALID_CLASH_CORES`（仅 `"mini-mihomo"`），同时将 [migrated.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/capabilities/migrated.json) 与 [desktop.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/capabilities/desktop.json) 平台收口为 `windows`，收紧 `fs:scope` 至 `[$TEMP/**]` 并显式 `deny` `$APPDATA/cores/**`。
- **状态**：代码已修正，待用户确认

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

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

历史已确认关闭条目（`BUG-057` ~ `BUG-410`）历史演进记录详见 [Changelog.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Changelog.md) 与 Git 提交历史。

| 缺陷编号 | 对应版本基线 | 状态 |
| :--- | :---: | :---: |
| **BUG-410** | v3.5.9 | 代码已修正，已确认 |
| **BUG-409** | v3.5.8 | 代码已修正，已确认 |
| **BUG-408** | v3.5.8 | 代码已修正，已确认 |
| **BUG-407** | v3.5.8 | 代码已修正，已确认 |
| **BUG-406** | v3.5.8 | 代码已修正，已确认 |
| **BUG-405** | v3.5.7 | 代码已修正，已确认 |
| **BUG-404** | v3.5.7 | 代码已修正，已确认 |
| **BUG-403** | v3.5.6 | 代码已修正，已确认 |
| **BUG-402** | v3.5.6 | 代码已修正，已确认 |
| **BUG-401** | v3.5.6 | 代码已修正，已确认 |
| **BUG-400** | v3.5.6 | 代码已修正，已确认 |
| **BUG-399** | v3.5.6 | 代码已修正，已确认 |
| **BUG-398** | v3.5.5 | 代码已修正，已确认 |
| **BUG-397** | v3.5.4 | 代码已修正，已确认 |
| **BUG-396** | v3.5.3 | 代码已修正，已确认 |
| **BUG-395** | v3.5.2 | 代码已修正，已确认 |
| **BUG-394** | v3.5.2 | 代码已修正，已确认 |
| **BUG-393** | v3.5.1 | 代码已修正，已确认 |
| **BUG-392** | v3.5.1 | 代码已修正，已确认 |
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
