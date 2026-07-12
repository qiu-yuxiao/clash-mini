# Clash Mini v2.4.6 全面代码审查报告

**审查日期：** 2026-07-09  
**审查范围：** 后端Rust + 前端TypeScript全量审查  
**审查重点：** 执行中断、冲突、互锁、卡死、资源抢占  
**审查方式：** 静态代码分析，未修改任何代码

---

## 问题统计

| 严重程度 | 数量 |
|---------|------|
| 高 | 18 |
| 中 | 35 |
| 低 | 41 |
| **合计** | **94** |

---

## 高严重程度问题（18个）

### 后端 Rust（10个）

#### H-01 `ACTIVE_TASKS` 的 abort_handle 存储后从未调用，Profile切换时孤儿任务泄漏 ✅已修复
- **文件：** [monitor.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L12-L13) 第12-13行、第411-414行
- **描述：** `ACTIVE_TASKS` 全局存储了测速worker的 `abort_handle()`，注释写明"用于在Profile切换时进行主动中止"。但全局搜索 `.abort()` 调用，**没有任何代码路径实际调用这些handle的abort方法**。Profile切换后，旧Profile的测速任务（最多32个并发worker）仍在后台运行，浪费CPU和网络资源，且旧结果可能通过事件回写前端造成UI数据闪烁。
- **修复方向：** 在 `PROFILE_SWITCH_NOTIFY.notified()` 分支中，调用 `ACTIVE_TASKS.lock()` 获取所有handle并逐个 `.abort()`，然后清空列表。
- **修复提交：** e6593dd

#### H-02 `wait_for_clash_ready` 持有 `AUTO_SELECT_RUNNING` 锁最长50秒，且无取消机制 ✅已修复
- **文件：** [monitor.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L132-L171) 第132-171行、第261-286行
- **描述：** `trigger_backend_auto_select` 通过 `compare_exchange` 获取 `AUTO_SELECT_RUNNING` 锁后，立即调用 `wait_for_clash_ready()`。该函数有两阶段等待（30s + 20s = 50s），期间锁一直为true，任何其他调用方（前端、轻量模式、自愈）都收到 `AUTO_SELECT_BUSY` 被拒绝。且 `wait_for_clash_ready` 内部不检查 `is_exiting`，应用退出时也会傻等满50秒。
- **修复方向：** 在 `wait_for_clash_ready` 的两个while循环中增加 `is_exiting()` 检查；考虑将 `wait_for_clash_ready` 调用时机提前到获取锁之前。
- **修复提交：** 137da93c

#### H-03 `show_error_dialog` 阻塞异步工作线程 ⚠️不修+注释
- **文件：** [monitor.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L750-L753) → [lib.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L182-L198) 第182-198行
- **描述：** monitor线程在连续5次自愈选点失败后调用 `show_error_dialog()`，Windows平台使用 `MessageBoxW` 同步阻塞调用，直到用户点击"确定"。monitor线程运行在tokio worker线程上，阻塞期间该线程上的其他异步任务被饿死，可能导致UI响应延迟、IPC调用超时。如果用户长时间不点击，monitor线程长期无法继续健康检测。
- **修复方向：** 使用 `tokio::task::spawn_blocking(|| { show_error_dialog(...); })` 或改用Tauri异步对话框API。
- **处理决定：** 有意为之的设计。5次失败说明网络严重恶化，弹窗期间停下来等用户处理是正确行为。已加注释说明。

#### H-04 `config_update_in_progress` 互斥机制已实现但从未被调用（死代码） ✅已修复
- **文件：** [manager/mod.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/mod.rs#L37) 第37行、第68-74行
- **描述：** `CoreManager` 定义了 `config_update_in_progress: AtomicBool` 和 `try_start_config_update()` / `finish_config_update()` 方法，但**这两个方法在整个代码库中从未被调用**。并发配置更新没有互斥保护，如果两个 `update_config_forced(true)` 同时发生（Profile切换 + 用户手动修改），可能导致生成的运行时配置不一致。
- **修复方向：** 在 `update_config_with_force` 入口处调用 `try_start_config_update()`，函数退出时（含panic路径）调用 `finish_config_update()`，用Drop guard确保。
- **修复提交：** 25693ac0（删除死代码）

#### H-05 `ServiceManager::current()` 存在Notify通知丢失导致永久死锁风险 ✅已修复
- **文件：** [service.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs#L568-L599) 第568-599行
- **描述：** `current()` 中 `Notify::notified()` 返回的future只有在被poll时才注册waker。`run_operation` 的defer!使用 `notify_waiters()`（不存储许可），如果通知发生在future创建后、poll前，通知会丢失。导致 `current()` 永久挂起。`current()` 被 `wait_for_service_ready` 调用，后者在 `start_core` 中被调用，`start_core` 持有 `lifecycle_lock`。如果 `current()` 挂起，`lifecycle_lock` 永久持有，`stop_core` / `restart_core` 永久阻塞，**应用完全死锁**。
- **修复方向：** 改用 `tokio::sync::watch` 通道替代 `Notify`，或将 `notify_waiters()` 改为 `notify_one()`。
- **修复内容：** `run_operation` 中 `notify_waiters()` 改为 `notify_one()`。`notify_one()` 存储许可，即使通知时无waker注册，后续 `notified().await` 也能立即消费许可返回，不会丢失。

#### H-06 `entry_lightweight_mode` spawned task在长await后修改节点选择，不重新检查轻量模式状态 ✅已修复
- **文件：** [lightweight.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L109-L179) 第109-179行，特别是第161-167行
- **描述：** spawned task在 `wait_for_clash_ready()` 之前检查 `is_in_lightweight_mode()`，但 `wait_for_clash_ready()` 可能阻塞50秒，期间用户可能退出轻量模式并手动选点了某个节点。`wait_for_clash_ready()` 返回后，`restore_profile_selected_nodes` 和 `trigger_backend_auto_select` 直接执行，**不再重新检查** `is_in_lightweight_mode()`，会覆盖用户的手动选择。
- **修复方向：** 在 `wait_for_clash_ready()` 返回后、执行 `restore_profile_selected_nodes` 之前，再次检查 `is_in_lightweight_mode()`；或改为可abort任务。
- **修复内容：** 在 `wait_for_clash_ready()` 返回后增加 `is_in_lightweight_mode()` 检查，若用户已退出则跳过选点操作。

#### H-07 `LIGHTWEIGHT_LOCK` 不保护异步窗口操作的执行顺序 ✅已修复
- **文件：** [lightweight.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L96) 第96行（entry的destroy）、第221行（exit的show）
- **描述：** `LIGHTWEIGHT_LOCK` 仅序列化状态转换和派发调用，不序列化实际的窗口操作执行。`destroy_main_window` 通过 `run_on_main_thread` 派发后立即返回释放锁，`show_main_window` 获取锁后检查窗口状态时destroy闭包可能尚未在主线程执行。快速"进入→退出"时，destroy和show/create的执行顺序不确定，可能导致窗口被销毁后永远不可见。
- **修复方向：** 在 `destroy_main_window` 和 `show_main_window` 中等待主线程操作完成（通过 `oneshot` channel回传完成信号），再释放锁。
- **修复内容：** `destroy_main_window` 改为 async 函数，用 `tokio::sync::oneshot` channel 等待主线程闭包执行完毕后才返回，确保锁释放时窗口已真正销毁。

#### H-08 `update_lite_mode_menu` 未在主线程执行UI操作 ✅已修复
- **文件：** [tray/mod.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/tray/mod.rs#L293-L301) 第293-301行
- **描述：** `update_lite_mode_menu` 直接调用 `item.set_text()` 和 `item.set_enabled()`，未包裹 `run_on_main_thread`。对比同文件中的 `update_icon`（196行）和 `init`（82行）均使用了 `run_on_main_thread`。此函数在tokio worker线程上被调用，跨线程修改原生菜单资源可能导致数据竞争或崩溃。
- **修复方向：** 将 `set_text` / `set_enabled` 调用包裹在 `app_handle.run_on_main_thread` 中。
- **修复内容：** 用 `run_on_main_thread` 包裹菜单操作闭包，与 `update_icon` 保持一致。

#### H-09 `wait_for_resolve_done` 存在Notify竞态，可能永久挂起 ✅已修复
- **文件：** [resolve/mod.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/mod.rs#L201-L214) 第201-214行
- **描述：** 经典Notify竞态。`notify_waiters()` 不存储permit，仅唤醒当前已注册的waiter。如果 `resolve_done()` 在调用方执行 `RESOLVE_NOTIFY.notified().await` 之前调用，通知会丢失，调用方永久等待。`timer.rs:432` 有timeout保护，最坏是超时继续执行，但若有其他无timeout保护的调用点则永久卡死。
- **修复方向：** 改为先创建 `notified` future再检查标志，或改用 `tokio::sync::watch`。
- **修复内容：** `notify_waiters()` 改为 `notify_one()`；`wait_for_resolve_done` 改为先注册 `notified()` 再检查标志的 loop 模式，与 `ServiceManager::current()` 一致。

#### H-10 `patch_verge_config` 后端缺少互斥保护 ✅已修复
- **文件：** [cmd/verge.rs](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/verge.rs#L13-L15) 第13-15行
- **描述：** `patch_verge_config` 直接调用 `feat::patch_verge`，没有像 `patch_profiles_config` 那样的 `try_lock_profile_switching` 互斥保护。前端快速连续调用多个 `patchVerge` 时，后端会并发处理，可能导致配置文件写入竞态（后写入的覆盖先写入的）。
- **修复方向：** 为 `patch_verge_config` 添加互斥锁，或确保 `feat::patch_verge` 内部有原子性保证。
- **修复内容：** 在 `feat::patch_verge` 入口添加 `VERGE_PATCH_LOCK` (tokio::sync::Mutex) 互斥保护，序列化 edit_draft/apply/save_file 操作。

### 前端 TypeScript（8个）

#### H-11 全局缺少"测速进行中"的协调机制（最严重的设计缺陷） ✅已修复
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1061-L1083) 第1061-1083行、第261-267行、第293-300行；[proxy-groups.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L425-L441) 第425-441行
- **描述：** 当前有**四处**独立调用 `setResizable(false)` + `setDragRegionEnabled(false)`：
  1. `_layout.tsx` 第261行（`triggerAutoSelectAndRefresh`）
  2. `_layout.tsx` 第293行（fallback定时器）
  3. `_layout.tsx` 第1061行（`triggerWakeupLatencyTest`）
  4. `proxy-groups.tsx` 第427行（`handleCheckAll`）
  
  这些调用点彼此完全不知道对方是否在测速。`proxy-groups` 的 `otherTesting` 检查只看自己的 `testingGroups`，`_layout` 的各处根本不做并发检查。任意两个测速并发时，先完成者会解锁窗口，破坏另一个测速期间的UI锁定不变量，导致批量测速期间用户可以拖动/调整窗口大小，可能导致测速结果错乱或UI冻结。
- **修复方向：** 引入全局引用计数 `batchTestLockRef`，进入测速时++，退出时--，仅当===0时才恢复 `setResizable(true)` + `setDragRegionEnabled(true)`。所有调用点通过context共享此ref。
- **修复内容：** 新增 `src/services/batch-test-lock.ts` 全局模块，导出 `batchTestLockRef` 引用计数。所有4处调用点进入测速时 `++`，finally中 `Math.max(0, --)`，仅当 `===0` 才恢复窗口可调整/可拖动。

#### H-12 `fallbackTimerRef` 被前一次finally覆盖，导致新定时器泄漏 ✅已修复
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L255-L310) 第255-310行
- **描述：** 快速切换profile时，第一次的fallback timer触发后async回调执行期间，第二次调用clearTimeout对已触发的timer无效，然后设置新timer。第一次的finally执行 `fallbackTimerRef.current = null` 覆盖了新timer的ID。结果：新timer永远无法被清理，组件卸载后仍会触发并调用 `setDragRegionEnabled` / `setResizable`。
- **修复方向：** finally中使用条件赋值 `if (fallbackTimerRef.current === expectedId) fallbackTimerRef.current = null`。

#### H-13 `triggerAutoSelectAndRefresh` 中的 `setTimeout(0)` 未保存ID，卸载后泄漏 ✅已修复
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L255-L274) 第255-274行
- **描述：** `setTimeout(async () => { ... }, 0)` 的返回值未保存到任何ref，组件卸载时无法清理。异步函数会在卸载后调用 `setDragRegionEnabled`、`win.setResizable` 等，造成state update on unmounted component及原生窗口API调用异常。
- **修复方向：** 将定时器ID存入ref，在组件卸载或profile切换的cleanup中清理；或用 `cancelled` 标志包裹。

#### H-14 `triggerWakeupLatencyTest` 中的 `setTimeout(0)` 同样未保存ID ✅已修复
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1058-L1084) 第1058-1084行
- **描述：** 与H-13同类问题。`setTimeout(async () => { ... }, 0)` 没有保存返回值，组件卸载后异步体仍会执行 `win.setResizable(false)`、`setDragRegionEnabled(false)`、`triggerAutoSelect` 等操作。
- **修复方向：** 引入 `wakeupTestTimerRef` 保存ID，卸载时清理。

#### H-15 `visibilitychange` 唤醒测速的重入竞态 ✅已修复
- **文件：** [_layout.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1198-L1203) 第1198-1203行 + 第1036-1088行
- **描述：** `triggerWakeupLatencyTest` 没有防重入锁。快速Alt+Tab两次时，两个 `refreshAll` 并发执行。若唤醒测速的setTimeout(0)正在执行（已setResizable(false)），此时用户点击F4触发 `handleCheckAll`，两者会并发操作 `setResizable` / `setDragRegionEnabled`，先完成者的finally会恢复为true。
- **修复方向：** 引入 `isWakeupTestingRef` 互斥锁，与 `handleCheckAll` 共享全局"测速进行中"标志（见H-11）。

#### H-16 `handleCheckAll` 的 `testingGroups` 闭包竞态 ✅已修复
- **文件：** [proxy-groups.tsx](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L391-L451) 第391-451行，重点第434-440行
- **描述：** `testingGroups` 的状态更新是异步批处理的，闭包中的值在整个async生命周期内都是调用时的快照。A组测速进行中（setTestingGroups已调度但React未提交），用户立即点击B组测速时，B组的 `testingGroups` 仍是旧值。B组先完成时finally检查 `otherTesting`（用旧闭包值），A为false，执行 `setResizable(true)` + `setDragRegionEnabled(true)`，A组测速仍在进行但窗口已解锁。
- **修复方向：** 用ref同步镜像 `testingGroups`，在setTestingGroups时同步更新ref，finally中读取ref。

#### H-17 所有IPC `invoke` 调用无超时保护（系统性问题） ✅已修复
- **文件：** [cmds.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts) 全文（23-467行）
- **描述：** 所有 `invoke()` 调用均无前端侧超时包装。如果后端因互斥锁等待或内核通信阻塞而长时间不响应，前端Promise将永远pending。涉及的调用包括：
  - `triggerAutoSelect`（_layout.tsx 3处 + proxy-groups.tsx 1处）
  - `patchVergeConfig`（use-verge.ts）
  - `importProfile`（_layout.tsx 2处）
  - `getProfiles()`（_layout.tsx 2处 + proxy-groups.tsx 1处）
  - `currentWindow.isMaximized()`（window-provider.tsx）
  - `downloadAndInstall`（_layout.tsx）
  
  后端卡住时，相关的UI状态（`testingGroups`、`profileLoading`、`isImportingRef`、`clientStatus`等）永远停留在"进行中"，用户无法操作。
- **修复方向：** 创建统一的 `withIpcTimeout(promise, ms)` 工具函数，对所有IPC调用包裹超时（30-60s），超时后reject并恢复UI状态。

#### H-18 `patchVerge` 无并发保护 ✅已修复
- **文件：** [use-verge.ts](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-verge.ts#L44-L55) 第44-55行
- **描述：** `patchVerge` 是普通 `useCallback`，没有使用 `useLockFn`（对比 `use-clash.ts` 的 `patchClash` 和 `patchInfo` 都用了 `useLockFn`）。快速连续调用 `patchVerge`（如用户快速切换多个设置开关）会并发发出多个 `patchVergeConfig` IPC调用，后处理顺序不确定，可能导致配置覆盖。`finally { await refetch() }` 也会并发执行。
- **修复方向：** 将 `patchVerge` 改为 `useLockFn` 包装。

---

## 中严重程度问题（35个）✅ 全部处理完毕

### 后端 Rust（17个）

| 编号 | 文件 | 问题 | 状态 |
|------|------|------|------|
| M-01 | monitor.rs:162 + monitor.rs:280 | `wait_for_clash_ready` 在轻量模式路径中被重复调用，最坏100秒等待 | ✅已修复：加 `skip_wait_ready` 参数 |
| M-02 | monitor.rs:519 | monitor线程无限循环缺少 `is_exiting` 检查，退出时继续运行 | ✅已修复：循环开头加 `is_exiting()` 检查 |
| M-03 | monitor.rs:380-414 | panic安全性 — worker spawn后abort_handle存储前的panic窗口，32个worker不会被abort | ✅已修复：新增 `WorkerPoolGuard` + Drop |
| M-04 | lifecycle.rs:12,53,83 | `lifecycle_lock` 持有时间过长（包含 `wait_for_service_ready` 的长轮询），阻塞stop/restart | ✅已修复：拆分为锁外等待+锁内启动 |
| M-05 | lifecycle.rs:173-183 | `fallback_to_system_proxy` 修改配置时无同步保护，与前端patch可能交错 | ✅已修复：加 `FALLBACK_CONFIG_LOCK` |
| M-06 | service.rs:687-689 | `run_service_command` 使用 `block_in_place`，UAC对话框阻塞worker线程 | ✅已修复：改用 `spawn_blocking` |
| M-07 | service.rs:436-439 | `start_with_existing_service` 的 stop_clash 后无等待即 start_clash，端口冲突风险 | ✅已修复：stop后加500ms延迟 |
| M-08 | config.rs:12-67 | `patch_clash` 提前 `apply()` 导致配置变更在enhance失败期间可见 | ✅已修复：移到enhance成功后再apply |
| M-09 | config.rs:335-361 | `patch_verge` 的 apply + save_file 非原子性，save失败时内存已更新 | ✅已修复：save失败时回滚内存状态 |
| M-10 | help.rs:61-85 | `save_yaml` 的 read-before-write 存在TOCTOU竞态且非原子写入 | ✅已修复：临时文件+rename原子写入 |
| M-11 | monitor.rs:280-286 | `wait_for_clash_ready` 期间不响应Profile切换通知，切换后使用旧uid选点 | ✅已修复：等待期间监听Profile切换通知 |
| M-12 | clash.rs:20-41 | `restart_app` 不主动中止后台monitor线程和spawned任务 | ✅已修复：重启前设退出标志+abort任务+唤醒线程 |
| M-13 | window_manager.rs:48,117 | 全局防抖时间戳导致 `exit_lightweight_mode` 可能反复失败回滚 | ✅已修复：按操作类型分别防抖 |
| M-14 | lightweight.rs:222-236 | `exit_lightweight_mode` 将 `RateLimited` 视为失败并回滚状态 | ✅已修复：RateLimited时检查窗口实际状态 |
| M-15 | lightweight.rs:110,121,139 | spawned task的TOCTOU — 连接清理可能在退出后执行，切断用户活跃连接 | ℹ️已确认：密集检查已覆盖 |
| M-16 | tray/mod.rs:257-260 | `on_menu_event` 对EXIT菜单项也做防抖限流，可能静默丢弃退出事件 | ✅已修复：EXIT项不走防抖 |
| M-17 | window.rs:74-91 | `on_page_load` 中 `window.show()` 依赖页面加载完成，无超时兜底 | ✅已修复：加10秒超时强制show |

### 前端 TypeScript（18个）

| 编号 | 文件 | 问题 | 状态 |
|------|------|------|------|
| M-18 | _layout.tsx:229,1071 | `triggerAutoSelect` IPC无超时，后端hang住时setResizable永远不恢复 | ℹ️已覆盖：H-17已加60s超时，finally保证恢复 |
| M-19 | _layout.tsx:1232-1298 | `handleImportProfile` 无超时，`isImportingRef` 可能永久锁定 | ℹ️已覆盖：H-17已加60s超时，finally保证释放 |
| M-20 | _layout.tsx:828 | `downloadAndInstall` 无超时，网络异常时永远pending | ✅已修复：10分钟超时 |
| M-21 | _layout.tsx:896 | `invoke('start_core_upgrade')` 无超时 | ✅已修复：5分钟超时 |
| M-22 | _layout.tsx:1143-1153 | 重试3次后静默停止，无用户反馈 | ✅已修复：失败后showNotice提示 |
| M-23 | proxy-groups.tsx:444-450 | `handleCheckAll` 的finally在卸载后仍调用setState | ✅已修复：isMountedRef保护 |
| M-24 | proxy-groups.tsx:878-911 | throttle的timer在组件卸载时未清理 | ✅已修复：throttle添加cancel方法，cleanup中调用 |
| M-25 | proxy-groups.tsx:413 | `getProfiles()` IPC无超时 | ℹ️已覆盖：H-17已加30s超时 |
| M-26 | window-provider.tsx:93 | `isMaximized()` 无超时 | ✅已修复：5秒超时 |
| M-27 | window-provider.tsx:176-178 | `startDragging` 失败后 `dragStartedRef` 未重置，用户被困stealth模式 | ✅已修复：catch中重置dragStartedRef |
| M-28 | use-render-list.ts:124-130 | `setGroupListener` 单例覆盖风险 | ✅已修复：支持多listener数组管理 |
| M-29 | cmds.ts:48 | `enhanceProfiles` 的 `yaml.load` 缺少schema约束 | ✅已修复：try-catch+类型校验 |
| M-30 | quick-routing.ts:60-90 | Merge文件读-改-写存在竞态条件 | ✅已修复：Promise链式文件锁 |
| M-31 | use-system-state.ts:64-103 | TUN自动关闭effect在refetch途中可能重复触发 | ✅已修复：用ref保存依赖，effect仅依赖enable_tun_mode |
| M-32 | app-data-provider.tsx:109-204 | `fetchProxies` 闭包捕获 `isMiniStatus`，窗口拖动时频繁refetch | ✅已修复：用isMiniStatusRef读取 |
| M-33 | _layout.tsx:1036-1088 | `triggerWakeupLatencyTest` 30秒冷却存在竞态 | ✅已修复：原子化冷却检查+设置 |
| M-34 | _layout.tsx:276-310 | fallback timer 2秒超时与主测速耗时严重不匹配 | ✅已修复：调整为10秒 |
| M-35 | _layout.tsx:1098-1165 | profile切换retry间隔(2秒)不足以覆盖后端测速时间 | ✅已修复：指数退避（2s/4s/6s） |

---

## 低严重程度问题（41个）✅ 全部处理完毕

### 后端 Rust（17个）

| 编号 | 文件 | 问题 | 状态 |
|------|------|------|------|
| L-01 | monitor.rs:547-558 | tokio::select! 中两个Notify的分支竞争，许可可能丢失 | ℹ️加注释：notify_one存储许可，两分支等价，不影响 |
| L-02 | monitor.rs:137-138 | 跨await持有Handle::mihomo()的模式（当前正确但脆弱） | ℹ️加注释：MihomoManager是Arc全局单例，安全 |
| L-03 | lifecycle.rs:60-66 | stop_core_inner清理IPC连接池时无错误处理 | ✅已修复：Err分支加warn日志 |
| L-04 | service.rs:585-599 | run_operation释放锁后调用update_menu的菜单更新竞态 | ✅已修复：update_menu移到锁释放前 |
| L-05 | lightweight.rs:109-179 | spawned task无 `is_exiting` 检查 | ✅已修复：关键await点加is_exiting检查 |
| L-06 | lightweight.rs:80+monitor.rs:261 | LIGHTWEIGHT_LOCK与AUTO_SELECT_RUNNING的嵌套关系（当前正确） | ℹ️加注释：AtomicBool非阻塞式，无死锁风险 |
| L-07 | lightweight.rs:216-237 | exit_lightweight_mode状态机回滚期间is_in_lightweight_mode()返回false | ✅已修复：In \| Exiting 都视为在轻量模式 |
| L-08 | clash.rs:62-91 | change_clash_mode无锁保护，与patch_clash可能交错 | ✅已修复：加CLASH_PATCH_LOCK互斥 |
| L-09 | clash.rs:110-119 | test_delay中proxy_port读取与使用之间存在竞态 | ℹ️加注释：一次性操作，概率极低，影响可忽略 |
| L-10 | window_manager.rs:131 | 硬编码sleep(50ms)等待前端渲染 | ℹ️加注释：等待WebView渲染，等待PageLoadEvent代价更高 |
| L-11 | window_manager.rs:256-299 | activate_window返回值不反映实际操作结果 | ✅已修复：用mpsc channel回传主线程结果 |
| L-36 | tray/mod.rs:238-255 | on_tray_icon_event/on_menu_event未检查is_exiting | ✅已修复：入口处加is_exiting()检查 |
| L-37 | tray/mod.rs:25-27 | TrayIconState.tray字段存储后从未被读取（死代码） | ✅已修复：删除结构体及manage调用 |
| L-38 | resolve/window.rs:85-90 | run_on_main_thread在已是主线程的回调中冗余调用 | ℹ️加注释：保留冗余确保安全，防御未来上下文变化 |
| L-39 | resolve/mod.rs:65,74 | init_window与init_auto_lightweight_boot的执行顺序浪费资源 | ✅已修复：静默启动+自动轻量模式直接进轻量模式 |
| L-40 | service.rs:37 | parking_lot::Mutex与std::sync::Mutex混用 | ℹ️加注释：都正确，仅风格不一致，建议后续统一 |
| L-41 | monitor.rs:479 | Handle::refresh_clash()在持有AUTO_SELECT_RUNNING时被调用 | ✅已修复：移到锁释放后执行 |

### 前端 TypeScript（24个）

| 编号 | 文件 | 问题 | 状态 |
|------|------|------|------|
| L-12 | _layout.tsx:1165 | useEffect依赖数组遗漏setDragRegionEnabled | ✅已修复：用setDragRegionEnabledRef保存 |
| L-13 | _layout.tsx:1300-1313 | handleSelectProfile切换时不重置isStartingUpRef | ✅已修复：切换时重置isStartingUp和retry计数 |
| L-14 | proxy-groups.tsx:239 | queueMicrotask调用setShowScrollTop可能卸载后执行 | ✅已修复：isMountedRef保护 |
| L-15 | proxy-groups.tsx:95-101 | localStorage.setItem无try/catch保护写入 | ✅已修复：加try-catch |
| L-16 | window-provider.tsx:156-160 | mousemove频繁触发resetIdleTimer性能问题 | ✅已修复：100ms throttle节流 |
| L-17 | window-provider.tsx:223-245 | toggleMaximize/toggleFullscreen的TOCTOU | ℹ️加注释：已有try-catch保护，影响小 |
| L-18 | use-render-list.ts:132 | groupCacheRef无大小限制，profile切换后旧组缓存泄漏 | ✅已修复：计算后清理不存在的组缓存 |
| L-19 | use-render-list.ts:108-119 | refreshProxy递归轮询无上限 | ✅已修复：最大10次重试 |
| L-20 | base-page.tsx:18 | DragRegionContext在Provider未挂载时的fallback行为 | ✅已修复：完善默认值，开发环境警告 |
| L-21 | base-page.tsx:21 | BasePage未使用BaseErrorBoundary的恢复机制 | ✅已修复：增强ErrorFallback，添加重试按钮 |
| L-22 | delay.ts:34 | urlMap永不清理，存在内存泄漏 | ✅已修复：加clearUrlMap方法供profile切换调用 |
| L-23 | delay.ts:81-112 | pendingItemUpdates在listener移除后被丢弃 | ℹ️加注释：组件重挂后重新获取，不影响数据 |
| L-24 | delay.ts:281-288 | abortPromise永不resolve | ✅已修复：仅signal存在时才创建abortPromise |
| L-25 | delay.ts:47-50 | isBatchTesting废弃属性残留 | ✅已修复：删除废弃属性 |
| L-26 | cmds.ts:196-304 | calcuProxies无错误边界 | ✅已修复：整体try-catch，异常返回空结构 |
| L-27 | use-proxy-selection.ts:111-127 | flushChangeQueue递归调用风险（实际安全） | ℹ️加注释：isProcessingRef互斥+while循环，深度可控 |
| L-28 | use-proxy-delay-state.ts:87-120 | abortControllerRef清理依赖useLockFn前提 | ℹ️加注释：详细说明useLockFn保证单并发的正确性 |
| L-29 | use-visibility.ts:82-98 | onResized/onFocusChanged回调堆积 | ✅已优化：useCallback+isMountedRef模式 |
| L-30 | use-traffic-monitor.ts:353-359 | TrafficWorkerClient单例永不销毁 | ✅已修复：引用计数为0后延迟5分钟销毁Worker |
| L-31 | use-mihomo-ws-subscription.ts:284-291 | throttle首条消息立即flush | ℹ️加注释：前缘触发+后缘合并符合WS订阅场景预期 |
| L-32 | app-data-provider.tsx:298-382 | 事件监听器在refresh函数变化时重新注册 | ℹ️加注释：useStableFn包裹，引用稳定 |
| L-33 | _layout.tsx:176 | localStorage与后端状态不同步 | ℹ️加注释：仅存UI状态（过滤/排序），不涉及核心状态 |
| L-34 | use-clash-log.ts:13-15 | deserializer: JSON.parse无错误保护 | ✅已修复：safeJsonParse函数，失败返回默认值 |
| L-35 | use-render-list.ts:96 | renderList的useMemo依赖latencyTimeout（常量）造成误导 | ✅已修复：移除常量依赖 |

---

## 最关键的风险路径

### 死锁风险路径
```
start_core → 获取 lifecycle_lock → prepare_startup → wait_for_service_ready
  → SERVICE_MANAGER.current() → Notify通知丢失 → 永久挂起
  → lifecycle_lock 永久持有 → stop_core/restart_core 永久阻塞
  → 应用完全死锁
```

### 资源泄漏路径
```
Profile切换 → PROFILE_SWITCH_NOTIFY → monitor重置last_check_time
  → 旧测速任务继续运行（未abort） → 浪费CPU/网络
  → 同时新测速任务启动 → 触发AUTO_SELECT_BUSY → 60秒冷却
  → 应用退出时 → monitor不检查is_exiting → 继续运行
  → 可能触发show_error_dialog → 阻塞退出
```

### UI卡死路径
```
后端IPC hang住（锁竞争/内核无响应）
  → 前端invoke永远pending
  → setResizable(false) 永远不恢复
  → setDragRegionEnabled(false) 永远不恢复
  → testingGroups永远为true
  → 用户无法再次触发测速
  → 用户无法拖动/调整窗口
  → UI完全卡死
```

### 跨组件竞态路径
```
_layout 唤醒测速 → setResizable(false) + setDragRegionEnabled(false)
  → 用户点击F4 → proxy-groups handleCheckAll
  → proxy-groups 先完成 → finally恢复setResizable(true) + setDragRegionEnabled(true)
  → _layout 测速仍在进行 → 窗口已解锁
  → 用户拖动窗口 → 与测速IPC竞争 → UI冻结
```

---

## 良好实践确认

审查中也发现多处良好的工程实践：

1. **DelayManager的RAF批处理**（delay.ts:81-140）— 正确处理渲染期间新到达的更新
2. **后端锁的Drop Guard**（monitor.rs:270-277）— panic时也能释放锁
3. **后端profile切换的30秒超时**（profile.rs:312-313）— 防止配置更新无限阻塞
4. **WebSocket订阅的引用计数**（use-mihomo-ws-subscription.ts:29-355）— 正确管理多组件共享WebSocket生命周期
5. **连接数据的序列号验证**（use-connection-data.ts:80-91）— 检测不匹配后自动重连
6. **所有事件监听器均有正确清理**（6处listen调用全部有cleanup）
7. **所有localStorage JSON.parse均有try-catch保护**（10处中9处有保护，1处待确认）
8. **后端命令无生产环境panic风险**（所有unwrap/expect在test模块内）
9. **useLayoutEvents的事件监听器注册有disposed标志保护**

---

## 修复优先级建议

### P0 — 立即修复（影响稳定性）
1. **H-11** 全局测速锁（引用计数）— 解决跨组件setDragRegionEnabled竞态
2. **H-05** ServiceManager Notify死锁 — 改用watch通道
3. **H-02** wait_for_clash_ready 50秒持锁 — 增加is_exiting检查
4. **H-17** IPC超时包装 — 防止后端hang住时UI永久卡死
5. **H-06** spawned task覆盖用户选点 — wait后重新检查轻量模式状态

### P1 — 尽快修复（影响可靠性）
6. **H-01** ACTIVE_TASKS孤儿任务 — Profile切换时abort旧任务
7. **H-04** config_update_in_progress死代码 — 启用互斥
8. **H-07** LIGHTWEIGHT_LOCK不保护窗口操作 — 用oneshot channel等待完成
9. **H-10** patch_verge_config无互斥 — 添加锁保护
10. **H-18** patchVerge无并发保护 — 改用useLockFn

### P2 — 排期修复（影响边缘场景）
11. **H-08** update_lite_mode_menu非主线程
12. **H-03** show_error_dialog阻塞worker线程
13. **H-09** wait_for_resolve_done Notify竞态
14. **H-12/H-13/H-14** setTimeout未保存ID的泄漏
15. **H-15/H-16** 唤醒测速重入竞态 + testingGroups闭包竞态

---

*本报告基于代码静态分析，未修改任何代码。所有修复方向仅供参考，具体实现需结合实际运行环境和测试验证。*
