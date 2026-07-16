# Clash Mini v2.5.8 发布前最终审计报告

**审计日期：** 2026-07-17  
**审计范围：** 全量代码（Rust 后端 + TypeScript 前端）  
**审计目的：** 正式发布前最后一轮全面检查

---

## 🔴 发布阻断项（必须修复）

| # | 文件 | 行号 | 标签 | 问题 | 说明 |
|---|------|------|------|------|------|
| B1 | `config/config.rs` | 113, 130 | 静默错误 | `let _ = tray::Tray::global().update_menu().await` + `let _ = profiles.cleanup_orphaned_files().await` | 托盘菜单更新失败无日志；孤立文件清理失败可能导致磁盘无限增长 |
| B2 | `core/core_updater.rs` | 469, 483, 535, 556 | 无退出检查 | `start_core()` / `stop_core()` 在核心升级路径中无 `is_exiting()` 守卫 | 退出过程中继续启动/停止核心，可能启动后在毫秒内又被终止 |
| F1 | `pages/_layout.tsx` | 525-527 | React 反模式 | `theme.controlSkin = controlSkin` 直接变更 hook 返回值 | 绕过 React 状态管理，永远不触发重新渲染，渲染与实际状态可能永久不同步 |
| F2 | `pages/_layout.tsx` | 1172 | 异步竞态 | `refreshProxyRef.current({ forceFull: true }).catch(() => {})` 即发即弃 | 唤醒时 refreshProxy 未 await，后续 IIFE 依赖已完成但可能过时的数据，且失败无感知 |

## 🟡 应修复项（建议发布前修）

| # | 文件 | 行号 | 标签 | 问题 | 说明 |
|---|------|------|------|------|------|
| B3 | 多处 | - | 静默错误 | 广泛使用 `let _ =` (hotkey, timer, file I/O, core ops) | 热键切换、定时任务、文件操作、核心生命周期操作失败完全无记录 |
| B4 | `lib.rs` / `hotkey.rs` | 101, 279, 132-206 | 分离任务 | `AsyncHandler::spawn` 返回的 `JoinHandle` 被丢弃 | 退出时无法中止，可能在资源已释放后执行 |
| B5 | `core/handle.rs` | 32 | Panic 风险 | `APP_HANDLE.get().expect("App handle not initialized")` | 若在句柄设置前/丢弃后调用则直接 crash |
| B6 | `core/manager/state.rs` | 39, 100 | 资源泄漏 | `libc::umask()` 恢复不在 RAII 守卫中 | spawn 过程中 panic 会导致 umask 无法恢复 |
| F3 | `hooks/use-head-state.ts` | 81-89 | 双层静默 | `.catch(() => { try {} catch {} })` 双层空捕获 | IPC 失败 + localStorage 兜底也失败 → 用户无感知 |
| F4 | `services/cmds.ts` | 27 | 静默丢弃 | `invoke('frontend_log', ...).catch(() => {})` | 前端诊断日志传输失败完全静默，调试时无法追踪 |
| F5 | `pages/unlock.tsx` | 207-221 | 无取消 | `useEffect` 内多个异步操作用 `void` 丢弃 Promise | deps 变化时旧操作仍运行，形成竞态导致过时数据覆盖当前数据 |
| F6 | `main.tsx` | 128-146 | 监听器泄漏 | `window.addEventListener` 全局监听器从未 `removeEventListener` | HMR 开发模式下重复监听器累积，生产环境无大影响 |

## 💭 低优项（延后处理）

| # | 文件 | 行号 | 标签 | 问题 |
|---|------|------|------|------|
| B7 | `config/config.rs` | 26 | TODO | 配置缺少版本号管理，未来字段变更时无法迁移 |
| B8 | `feat/icon.rs` | 73 | TODO | 图标缓存无 LRU 驱逐，长期运行可能无限增长 |
| B9 | `feat/proxy.rs` | 84 | 魔法数字 | 默认端口 7897 与 constants.rs 中的 DEFAULT_MIXED(10801) 不一致 |
| F7 | `components/layout/traffic-graph.tsx` | 112 | 非空断言 | `canvasRef.current!` 破坏类型安全，依赖后续 if check 兜底 |
| F8 | `pages/_layout.tsx` | 338-358 | 原地变更 | `.sort()` 在原地排序，目前安全因输入来自 `.filter()`，但模式脆弱 |

---

## 修复优先级建议

1. **B1** — 加 `logging!(warn)` 替代 `let _ =`
2. **F1** — `theme.controlSkin` 改为派生新对象而非直接变更
3. **F2** — 唤醒路径 await refreshProxy（或移到 IIFE 内）
4. **B2** — 加 `is_exiting()` 守卫
5. **F3/F4** — `catch {}` 替换为 `frontendLog`
6. **B3/B4** — 选关键路径加日志 / 保存 AbortHandle
7. **F5** — `useEffect` 内加 abort 逻辑
8. **🔴** 项修完即可发布，🟡 项建议修但可协商延后，💭 项不阻塞发布
