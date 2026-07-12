# Clash Mini v2.4.7 第二轮代码审计报告

**审计日期：** 2026-07-13
**审计范围：** 全量代码（后端 Rust + 前端 TypeScript + IPC 边界）
**审计重点：** 系统资源争夺、进程卡死/锁死、资源泄漏
**审计方法：** 4 个并行深度审计 agent，分别覆盖锁/同步、资源管理、前端资源、IPC 边界

---

## 审计结论

上一轮 94 个问题修复后，代码质量显著提升，但仍发现 **35 个新问题**（去重后）。核心风险集中在**退出路径不完整**——这是当前最大的系统性缺陷。

| 严重程度 | 数量 | 说明 |
|---------|------|------|
| Critical | 3 | 退出时资源不清理、后台线程不可取消 |
| High | 10 | detached task 泄漏、IPC 超时覆盖不足、错误传播断裂 |
| Medium | 14 | 竞态窗口、错误处理不完整、状态不一致 |
| Low | 8 | 性能优化、代码健壮性 |

---

## 一、Critical（3 个）

### C-01 `quit()` 退出路径不完整——不中止测速任务、不通知后台线程

- **文件：** [window.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/window.rs#L19-L39) 第19-39行
- **问题描述：** `feat::quit()` 仅做 `set_is_exiting()` + `clean_async()`，**完全不调用** `abort_all_active_tasks()`，也不发送 `MONITOR_WAKEUP_NOTIFY`/`PROFILE_SWITCH_NOTIFY`。而 `restart_app()` 有完整的 abort + notify + sleep(200ms) 流程。

  退出时残留的 32 路并发测速任务会继续执行 mihomo API 调用，但 `clean_async()` 中的 `stop_core` 会停止 mihomo 进程，导致测速任务的 API 调用失败并产生大量错误日志。后台 monitor 线程可能卡在 `trigger_backend_auto_select` 的 `select_node_for_group().await` 上，最坏要等 15 秒（NORMAL_CHECK_INTERVAL_SECS）才能检测到 `is_exiting`。

- **根因：** `quit` 和 `restart_app` 两条退出路径独立演化，未抽取公共清理逻辑。
- **修复方向：** 抽取 `prepare_exit()` 公共函数，`quit` 和 `restart_app` 都调用。

### C-02 后台 monitor 常驻线程无 AbortHandle，不可主动取消

- **文件：** [monitor.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L574-L856) 第574行
- **问题描述：** `start_background_monitor` 通过 `AsyncHandler::spawn` 启动，返回的 `JoinHandle<()>` **被直接丢弃**。没有任何机制保存 abort handle。

  `abort_all_active_tasks()` 只能 abort `ACTIVE_TASKS` 中的 worker pool（32个测速 worker），**无法 abort monitor 主任务自身**。当 monitor 正在执行 `trigger_backend_auto_select` 内的 `select_node_for_group().await` 或 `delay_proxy_by_name().await` 时，`MONITOR_WAKEUP_NOTIFY` 无法中断这些非 select 的 await，只能等 tokio runtime 强制 abort。

- **根因：** 设计为"协作式取消"，但缺少对 monitor 主任务本身的 abort 机制。
- **修复方向：** 用 `Mutex<Option<AbortHandle>>` 保存 monitor task handle，退出时主动 abort。

### C-03 退出/重启时未关闭 WebSocket 订阅、未停止 Timer 调度器

- **文件：** [window.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/window.rs#L41-L161) `clean_async`；[timer.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/timer.rs#L62-L120) 第62-120行
- **问题描述：** `clean_async` 仅做 3 件事：重置系统代理、关闭 TUN/停止 core、恢复 DNS。遗漏：
  1. **未关闭 WebSocket 订阅** — 进入轻量模式时调用 `mihomo.clear_all_ws_connections()`，但退出应用时不调用
  2. **未停止 Timer 调度器** — `Timer::run_scheduler` 是无限 loop，`command_tx` 永远不会被 drop（singleton 持有），调度器永远不退出
  3. **未关闭 AsyncLogger** — 缓冲日志可能丢失
  4. **未 abort 轻量模式 cleanup 任务** — `lightweight.rs:109` 的 spawned task

- **根因：** `clean_async` 设计于早期版本，未随新功能同步扩展。
- **修复方向：** 在 `clean_async` 中增加 WS 清理、Timer shutdown、Logger flush。

---

## 二、High（10 个）

### H2-01 轻量模式 cleanup 任务无 abort，多次开关累积 detached task

- **文件：** [lightweight.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L109-L179) 第109-179行
- **问题：** `entry_lightweight_mode` 内部 spawn 的 cleanup 任务（close_all_connections → clear_ws → restore_profile → trigger_auto_select → sleep(2s) → trim_working_set），JoinHandle 被直接丢弃。用户快速反复"进入→退出"轻量模式时，前一个 cleanup 任务可能仍在 sleep(2s) 中，新任务又启动。
- **修复：** 用 `Mutex<Option<AbortHandle>>` 保存 cleanup task handle，进入前 abort 上一个。

### H2-02 sidecar 日志消费任务无 abort 机制

- **文件：** [state.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/state.rs#L110-L162) 第110-162行
- **问题：** `start_core_by_sidecar` 中 spawn 的日志消费任务（`while let Some(event) = rx.recv().await`），JoinHandle 被丢弃。`stop_core_by_sidecar` 调用 `child.kill()` 后立即返回，未等待日志消费任务退出。如果 sidecar 进程被外部 kill，可能不发送 `Terminated` 事件，rx.recv() 永远阻塞，任务泄漏。
- **修复：** 保存 abort handle，stop 时主动 abort。

### H2-03 Timer 调度器无 shutdown 信号

- **文件：** [timer.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/timer.rs#L165-L195) 第165-195行
- **问题：** `run_scheduler` 是无限 loop，仅当 `command_rx.recv()` 返回 None 时退出。但 `command_tx` 由 Timer 单例持有永远不会被 drop。退出时 `DelayQueue` 中的定时任务（如订阅自动更新）可能 spawn 新任务。
- **修复：** 增加 `TimerCommand::Shutdown` 变体和 `shutdown()` 方法。

### H2-04 `abort_all_active_tasks` 只 abort 不 join

- **文件：** [monitor.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L16-L21) 第16-21行
- **问题：** abort 是异步的，`restart_app` 中只 sleep(200ms) 兜底，不保证任务完全退出。若任务持有 `AUTO_SELECT_RUNNING` 锁，abort 后 Drop Guard 是否执行取决于 tokio runtime 行为。
- **修复：** abort 后增加短暂 sleep(50ms) 等待，或保存 JoinHandle 并 await。

### H2-05 大量关键 IPC 调用仍缺少超时保护

- **文件：** [cmds.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts) 多处
- **问题：** `withIpcTimeout` 仅包裹了 4 个调用。以下关键调用全部无超时：
  - `enhanceProfiles`（3次 invoke）
  - `patchProfilesConfig`（Profile 切换核心命令）
  - `patchClashConfig` / `patchClashMode`
  - `restartCore`
  - `updateProfile` / `deleteProfile`
  - `installService` / `uninstallService`
  - 全部 ~25 个 tauri-plugin-mihomo IPC 调用（`getProxies`, `selectNodeForGroup`, `delayProxyByName` 等）
- **修复：** 统一包裹 `withIpcTimeout`，按操作性质设置超时。

### H2-06 `change_clash_mode` 错误不传播到前端

- **文件：** [clash.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L77-L108) 第77-108行；[cmd/clash.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L40-L43) 第40-43行
- **问题：** `change_clash_mode` 在 mihomo API 失败时仅 `logging!(error)`，不返回错误。命令层总是返回 `Ok(())`，前端无法知道模式切换是否成功。
- **修复：** `change_clash_mode` 返回 `Result<()>`，命令层 propagate。

### H2-07 Profile 切换无前端锁保护

- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1304-L1317) 第1304-1317行
- **问题：** `handleSelectProfile` 没有 `useLockFn` 或 ref 守卫。用户快速连续点击不同 Profile 时，会触发多个并发 `patchProfilesConfig` 调用。后端返回 Busy 但前端不区分。
- **修复：** 用 `useLockFn` 包裹。

### H2-08 `enhanceProfiles` 静默吞掉错误

- **文件：** [cmds.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts#L80-L179) 第80-179行
- **问题：** 三层 try-catch 中前两层的错误仅 `console.error` 不 throw，用户看不到任何反馈。最后的 `invoke('enhance_profiles')` 无 try-catch。
- **修复：** 统一错误处理，所有失败路径都应 throw 或返回明确结果。

### H2-09 `save_yaml` 在 Windows 上 rename 前 remove 存在数据丢失竞态

- **文件：** [help.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/help.rs#L100-L111) 第100-111行
- **问题：** Windows 分支 `remove_file(path)` 后到 `rename(tmp, path)` 之间有时间窗口。若 rename 失败，**原配置文件已被删除**，用户配置丢失。
- **修复：** 使用 `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`，或使用 `tempfile::NamedTempFile::persist()`。

### H2-10 `stop_core_by_sidecar` 未等待进程完全退出

- **文件：** [state.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/state.rs#L145-L162) 第145-162行
- **问题：** `child.kill()` 后立即返回，未 `child.wait()`。端口可能仍被占用，下次 `start_core` 时端口冲突。
- **修复：** kill 后 `tokio::time::timeout(2s, child.wait()).await`。

---

## 三、Medium（14 个）

### M2-01 resolve_setup_async 多个 detached 初始化任务无追踪
- **文件：** [mod.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/mod.rs#L45-L99) 第45-99行
- **问题：** 多个 `AsyncHandler::spawn` 的 JoinHandle 被丢弃，退出时无 abort。
- **修复：** 创建全局 `STARTUP_TASKS: Mutex<Vec<AbortHandle>>`。

### M2-02 core_updater 后台检查循环无 is_exiting 检查
- **文件：** [updater.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/updater.rs#L498-L510) 第498-510行
- **问题：** 无限 loop 无退出条件（当前被禁用，但代码仍在）。
- **修复：** 循环顶部加 `is_exiting()` 检查。

### M2-03 after_change_clash_mode 异步关闭连接无总超时
- **文件：** [clash.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L58-L74) 第58-74行
- **问题：** 逐个 `close_connection`，数千连接时可能阻塞数分钟。
- **修复：** 加 3 秒总超时，或直接调用 `close_all_connections`。

### M2-04 lightweight cleanup 任务的 is_in_lightweight_mode 检查仍有竞态
- **文件：** [lightweight.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L109-L179) 第109-179行
- **问题：** Atomic 检查是非事务性的，检查通过后状态可能立即变化。
- **修复：** 改用 AbortHandle 主动控制。

### M2-05 hotkey.rs 多个 detached 任务无 is_exiting 检查
- **文件：** [hotkey.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/hotkey.rs#L121-L200) 第121-200行
- **问题：** 退出时快捷键仍可能触发 spawn。
- **修复：** spawn 入口加 `is_exiting()` 守卫。

### M2-06 `delete_profile` 等命令不使用切换锁
- **文件：** [profile.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L195-L228) 第195-228行
- **问题：** 只有 `import_profile`、`update_profile`、`patch_profiles_config` 用了切换锁。`delete_profile` 可能在切换期间删除当前活跃 Profile。
- **修复：** 对所有修改 profiles 结构的命令统一加锁。

### M2-07 6 秒 Fallback 定时器与 Profile 切换竞态
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L278-L310) 第278-310行
- **问题：** fallback 回调内部已开始执行的异步操作不响应 Profile 切换。
- **修复：** 在 fallback 回调内增加 Profile UID 校验。

### M2-08 唤醒测速与 Profile 切换竞态
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1038-L1090) 第1038-1090行
- **问题：** `getFilteredNodeNames` 和 `triggerAutoSelect` 之间有 setTimeout(0) 间隔，Profile 可能变化。
- **修复：** setTimeout 回调内重新校验 Profile UID。

### M2-09 DelayManager 模块级 setInterval 未保存 ID
- **文件：** [delay.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L50-L64) 第50-64行
- **问题：** HMR 场景下 setInterval 累积。
- **修复：** 保存 timer ID，提供 destroy() 方法。

### M2-10 urlMap 的 clearUrlMap() 方法存在但从未被调用
- **文件：** [delay.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L36) 第36行
- **问题：** profile 切换时 urlMap 持续累积旧数据。
- **修复：** 在 profile 切换时调用 `clearUrlMap()`。

### M2-11 `patchVerge` 失败后 refetch 也失败时状态不一致
- **文件：** [use-verge.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-verge.ts#L44-L54) 第44-54行
- **问题：** 后端部分应用了变更，前端缓存仍是旧状态。
- **修复：** refetch 失败时显示"状态可能不一致"警告。

### M2-12 `is_port_in_use` 同步阻塞 IPC 线程
- **文件：** [network.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/network.rs#L101-L102) 第101-102行
- **问题：** `TcpListener::bind` 可能在 TIME_WAIT 或防火墙时阻塞。
- **修复：** 改为 `async fn`，用 `tokio::net::TcpListener::bind`。

### M2-13 tun.rs 中 macOS DNS 切换的 detached task 无去重
- **文件：** [tun.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/tun.rs#L37-L47) 第37-47行
- **问题：** 快速切换 TUN 模式时 DNS 操作交叉执行。
- **修复：** 用 `Mutex<Option<AbortHandle>>` 保存上一次任务。

### M2-14 `calcuProxies` 错误时返回空结构掩盖真实问题
- **文件：** [cmds.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts#L266-L399) 第266-399行
- **问题：** 调用方误以为"无代理组"而非"获取失败"。
- **修复：** 失败时 throw 让调用方决定降级策略。

---

## 四、Low（8 个）

### L2-01 test_delay 的 buf Vec 每次堆分配
- **文件：** [clash.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L142) 第142行
- **修复：** 改用栈数组 `let mut buf = [0u8; 1024];`

### L2-02 start_with_existing_service 中 500ms 硬编码等待
- **文件：** [service.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs#L443) 第443行
- **修复：** 改为轮询 `is_port_available` 直到超时。

### L2-03 lib.rs deep_link 的 detached task 无 is_exiting 检查
- **文件：** [lib.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L97-L104) 第97-104行
- **修复：** spawn 前检查 is_exiting。

### L2-04 window_script 超时强制显示窗口的 detached task
- **文件：** [window.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/window.rs#L121-L132) 第121-132行
- **修复：** 10s 后检查窗口是否已销毁再 show。

### L2-05 useVisibility 中 isMountedRef 跨 effect 共享
- **文件：** [use-visibility.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-visibility.ts#L24-L31) 第24-31行
- **修复：** 为第二个 effect 使用独立的 isMountedRef。

### L2-06 batchTestLockRef 的 ++ 在 try 块外
- **文件：** [batch-test-lock.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/batch-test-lock.ts#L13) 第13行
- **修复：** 将 `++` 也放入 try 块内。

### L2-07 ref 在 render 中直接赋值（非 useEffect）
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1027-L1033) 第1027-1033行
- **修复：** 统一使用 useEffect 赋值 ref。

### L2-08 `test_delay` 返回魔法数字 10000
- **文件：** [cmd/clash.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L79-L88) 第79-88行
- **修复：** 定义常量 `DELAY_ERROR_VALUE: u32 = 10000`，前端检测并显示"超时"。

---

## 五、系统性问题总结

### 问题 1：退出路径碎片化（最核心）

`quit()` 和 `restart_app()` 是两条独立实现，前者缺少 abort + notify。`clean_async` 未随新功能扩展。这是导致 C-01/C-02/C-03 的根因。

**统一修复方案：**
```
prepare_exit()
├── set_is_exiting()
├── abort_all_active_tasks() + sleep(50ms)
├── abort_monitor() + abort_lightweight_cleanup()
├── notify MONITOR_WAKEUP_NOTIFY + PROFILE_SWITCH_NOTIFY
├── Timer::shutdown()
├── clear_all_ws_connections()
├── Logger::flush()
├── sleep(200ms) 等待后台退出
└── clean_async()  // 现有的系统代理/TUN/core清理
```

### 问题 2：detached task 模式泛滥

全项目有 ~15 处 `AsyncHandler::spawn` / `tokio::spawn` 的 JoinHandle 被丢弃。虽然大部分任务会自然结束，但退出时无法主动取消。

**统一修复方案：** 创建 `TaskRegistry` 全局任务注册器，所有 spawn 的 handle 自动注册，退出时统一 abort。

### 问题 3：IPC 超时覆盖不足

仅 4/~60 个 IPC 调用有超时保护。后端 hang 住时前端永久等待。

**统一修复方案：** 在 `cmds.ts` 和 `tauri-plugin-mihomo/guest-js/index.ts` 中对所有 invoke 调用统一包裹 `withIpcTimeout`。

### 问题 4：错误传播链断裂

多处后端错误被 `logging!(error)` 或 `console.error` 吞掉，前端无法感知失败。

**统一修复方案：** 后端所有 feat 层函数返回 `Result`，命令层 propagate；前端所有 catch 块至少 showNotice 通知用户。

---

## 六、修复优先级

| 优先级 | 问题编号 | 修复复杂度 | 风险 |
|--------|---------|-----------|------|
| **P0-立即** | C-01, C-02, C-03 | 中（抽取 prepare_exit） | 高（退出时资源泄漏） |
| **P1-紧急** | H2-01~H2-04 | 中（保存 AbortHandle） | 高（task 泄漏） |
| **P1-紧急** | H2-05 | 中（包裹 IPC） | 高（前端 hang） |
| **P1-紧急** | H2-06~H2-08 | 小（返回 Result / 加锁） | 中（静默失败） |
| **P1-紧急** | H2-09, H2-10 | 小（原子写入 / wait） | 中（数据丢失 / 端口冲突） |
| **P2-尽快** | M2-01~M2-14 | 小~中 | 中 |
| **P3-计划** | L2-01~L2-08 | 小 | 低 |
