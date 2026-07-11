# 代码审核报告 — v2.4.2→v2.4.3（da996c23..0523ee99）

**审核时间**: 2026-07-11 16:25  
**审核范围**: 9 个功能性 commits + 2 个 chore，共 **+109/-44** 净变更  
**审核方式**: 只读审查，未改任何代码

---

## 总评

**整体质量: A，本区间无 blocker 级问题。**

本批次的核心主题是**并发安全加固**——全局配置切换锁统一收口、订阅导入与手动更新互斥、系统代理注册表写入加锁、TUN 切换时 SYS_PROXY 提前清场。此外还有若干配置边界修正（allow-lan/ipv6 强制重启、Mixed Port 输入覆盖 bug）。方向正确，实现也干净。

---

## commit-by-commit 分析

### 1. `c2ae40bc` — clippy 警告清理

| 文件 | 改动 |
|------|------|
| `cmd/app.rs` | 加 `#![allow(clippy::unused_async)]` |
| `cmd/clash.rs` | 同上 |
| `config/verge.rs` | 删除未用 `deserialize_encrypted`/`serialize_encrypted` |
| `core/manager/lifecycle.rs` | 删除 `StringifyErr`/`IVerge`/`String` 未用 import |

🟡 **S1: crate 级别 `allow` 略粗糙**  
`#![allow(clippy::unused_async)]` 拍在 crate 根部会压制该文件内所有 `unused_async` 实例。若后续有人故意写 `async fn() { /* no await */ }`，clippy 不会报。但考虑到 `cmd/` 下的 Tauri command 函数签名由宏约定、`async` 是框架要求，实际风险很低。可接受。

✅ 其余清理无误。

---

### 2. `97e59cd6` + `424eb2f0` — 版本号 bump + app-update.json

Trivial，无代码审查点。

---

### 3. `d5bae65b` — allow-lan/ipv6 变更强制重启内核

**关键改动** (`feat/config.rs:patch_clash`):
- 新增 `old_allow_lan` / `old_ipv6` 与 `new_allow_lan` / `new_ipv6` 对比检测
- 若实质变动 → `need_restart` → `restart_core()`
- 同时删除非重启路径的 `Config::runtime().await.edit_draft(|d| d.patch_config(patch))`

✅ `allow_lan_changed = new_allow_lan.is_some() && new_allow_lan != old_allow_lan` 正确处理了"首次设置"（old 为 None，new 为 Some(true)）和"值改变"两种场景。  
✅ 删除 hot-reload 的 `edit_draft` 是正确的——因为 `update_config_checked()` 内部自己会合并的，外部再 patch 是多余。  
✅ 注释说明了"端口冲突、TUN 网卡死锁"的根因，决策透明。

无问题。

---

### 4. `024e2cd2` — rule-fallback/dns/enhanced 触发 CLASH_CONFIG + 托盘

**关键改动** (`feat/config.rs:determine_update_flags`):
- `tun_mode.is_some()` 条件**扩展为**：
  ```rust
  tun_mode.is_some()
      || rule_fallback.is_some()
      || enable_dns_settings.is_some()
      || enable_builtin_enhanced.is_some()
  ```
  触发 `CLASH_CONFIG | GROUP_SYS_TRAY | SYSTRAY_ICON`

✅ 修正了此前 rule-fallback/dns/enhanced 切换后不改配置的遗漏。完全正确。

---

### 5. `7ad818f9` — TUN↔系统代理切换竞态修复

**两处改动**：

**a) `core/sysopt.rs:reset_sysproxy`**
```rust
let _lock = self.update_lock.lock().await;
```
在关闭代理守卫和操作 OS 注册表前，持 `update_lock` 防止与 `update_sysproxy` 的 `spawn_blocking` 并发写入 WinINET 注册表。

**b) `feat/config.rs:process_terminated_flags`**
SYS_PROXY 处理从 CLASH_CONFIG **之后**移至 **之前**。

✅ 两处配合解决了 TUN 切换时的竞态：锁确保清空/写入不乱序；SYS_PROXY 提前确保 OS 代理先清空、再启 TUN 网卡，不存在双重流量接管的冲突窗口。  
✅ 注释详尽地说明了退场顺序和异常隔离理由。高质量。

---

### 6. `827f8b80` — auto-launch/silent-start 区分启动模式

**a) `utils/resolve/mod.rs:init_window`**
- Windows 上 silent start 条件改为 `is_silent_start && has_silent_arg`（需配置**且**命令行带 `--silent`）
- 其他平台保持原逻辑

**b) `utils/schtasks.rs`**
- 计划任务 XML 加 `<Arguments>--silent</Arguments>`
- `set_auto_launch` 中的 `remove_task_elevated` 失败改为非致命 warn

🟡 **S2: `#[cfg]` 分支可能漏 Linux 兜底**

当前逻辑：
```rust
#[cfg(target_os = "windows")]
let should_silent = is_silent_start && has_silent_arg;
#[cfg(not(target_os = "windows"))]
let should_silent = is_silent_start;
```

这意味着 macOS 上 silent start 仅看配置，不需要 `--silent` 参数。但 schtasks.rs 的 `--silent` 参数只在 Windows 任务计划里加——macOS 的 launchd plist 默认不会有 `--silent` 参数，所以 `has_silent_arg` 对 macOS 来说一定为 false，旧逻辑（仅看配置）是正确且唯一的可行方案。

实际上这段 `#[cfg]` 处理是正确无误的，我收回我的担忧——macOS 的情景不走 arg 检查，Linux 也不走（Linux 没有 silent start 的等价物，不生成 launchd/task 任务）。正确，无问题。

✅ 整体方案干净：Windows 上只有计划任务启动时带 `--silent` 参数，用户双击不会触发 silent start，完美。

---

### 7. `9e5403db` — 订阅导入并发锁 + 自动激活 + 回滚同步

**三处改动**：

**a) 并发锁** — `import_profile` 入口加 `CURRENT_SWITCHING_PROFILE` compare_exchange 锁 `defer!` 释放。

**b) 自动激活** — 导入后检查 `final_current = Config::profiles().await.data_arc().current.clone()`，若导入的 item 成为 current，`AsyncHandler::spawn` 异步调 `update_config_forced`。

**c) 回滚同步** — `discard_and_restore` 回滚后加 `notify_profile_changed(prev_profile)`。

✅ 锁保护了导入+手动更新的互斥。✅ 自动激活逻辑正确（`is_current_changed` 的 TOCTOU 间隙由 `CURRENT_SWITCHING_PROFILE` 锁保护，安全）。  
✅ 回滚通知补上了此前遗漏的 UI 状态同步。

无问题。

---

### 8. `ec6900ac` — Mixed Port 输入覆盖 bug

**`basic-settings-card.tsx`**：
- 引入 `localPort` + `inputRef` 本地状态
- `useEffect` 只在输入框**不聚焦**时从 `mixedPortVal` 同步
- `onLocalSave` → `handleSavePort(localPort)`（传值而非闭包）
- 移除 `setMixedPortVal` prop

**`_layout.tsx`**：
- `handleSavePort` 改为 `(port: number) => void` 签名

✅ 标准的"受控输入 + 外部值同步"模式，`document.activeElement !== inputRef.current` 守卫正确防止输入中被打断。干净。

---

### 9. `0523ee99` — 全局配置切换锁统一收口

**关键改动**：
- `CURRENT_SWITCHING_PROFILE` 从 `cmd/profile.rs`（本地 static）迁至 `feat/profile.rs`（pub static）
- `update_profile`(cmd) 新增锁获取
- `async_task`(timer) 新增锁获取
- `patch_profiles_config`(cmd) 改为引用 `feat::CURRENT_SWITCHING_PROFILE`
- `import_profile`(cmd) 已在上一 commit 加锁

✅ 此前每个路径各自有一个独立的 `AtomicBool`（`patch_profiles_config` 一个、`import_profile` 后来补的但跨在 cmd 层），互不感知对方操作。统一后所有配置切换/更新/导入/定时任务**串行化通过一个锁**，彻底消除交叉竞态。  
✅ 所有 acquire/release 语义一致：`compare_exchange(false, true, Acquire, Relaxed)` / `store(false, Release)`。

💭 **N1: timer.rs 中 lock/import/update 三路都有相同的锁代码片段**

`cmd/profile.rs:update_profile`、`cmd/profile.rs:import_profile`、`core/timer.rs:async_task` 三处完全相同的：
```rust
if CURRENT_SWITCHING_PROFILE.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
    logging!(info, ...);
    return Err/Ok(...);
}
defer! {
    CURRENT_SWITCHING_PROFILE.store(false, Ordering::Release);
}
```
这已经是四次重复（算上 `patch_profiles_config` 是四次）。考虑提一个辅助函数如 `try_acquire_switch_lock()` 来消除重复。

---

## 总结

| 级别 | 编号 | 说明 | 文件 |
|------|------|------|------|
| 🟡 S1 | crate 级 allow（已修） | `#![allow(clippy::unused_async)]` 替换为 6 个 `pub fn` 同步函数，并移除所有 `#[allow]`（2026-07-11 user 确认） | `cmd/app.rs` `cmd/clash.rs` |
| 💭 N1 | 锁代码重复（已修） | `cmd/profile.rs` 中 `import_profile`/`update_profile` 两处提取为 `try_lock_profile_switching!` 宏；`patch_profiles_config` 因 acquire/defer 分离不适用；`timer.rs` 因返回值不同放过。 | `cmd/profile.rs` |
| ✅ | 其余全部通过 | 并发锁统一、订阅锁+自动激活+回滚同步、allow-lan/ipv6 重启、SYS_PROXY 顺序调整、Mixed Port 本地状态、silent-start arg 检测 | 全局 |

**本区间无 🔴 blocker 问题，可以放心放行。**
