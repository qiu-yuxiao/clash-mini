# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：历史已验收条目详见 [Changelog.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Changelog.md) 与 Git 提交历史。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

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
