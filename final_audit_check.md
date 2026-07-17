# Clash Mini 最终全盘审查报告

**检查时间**: 2026-07-17
**检查范围**: 全部 Rust 后端（src-tauri/src/，99 文件）+ 全部 TypeScript/React 前端（src/，82 文件）
**检查维度**: 正确性 · 安全性 · 可维护性 · 性能 · 死代码 · 交叉一致性 · 回归检查

---

## ✅ 已确认安全的领域

| 检查项 | 结论 |
|---|---|
| Rust `.unwrap()` 生产路径 | **零风险** — 全部在测试/脚手架代码 |
| Rust `unsafe` 块 | 仅 `show_error_dialog` 的 Win32 API，正确封装 |
| 前端静默错误吞咽 `.catch()` | **无** — 所有 catch 均有 console.warn/error 或 frontendLog |
| `hotkey`/快捷键系统残留 | **全部清除** — 前后端零引用 |
| `maximized`/`isMaximized`/`unmaximize` 残留 | **全部清除** — strontend zero, backend never had |
| 子集语义合规（选点不出 filterText 范围） | **通过** — F1/F2/F4/自动防断流/自愈全部在子集内；None 路径读 proxy_head_state.json filterText |
| 系统代理 Core 状态守卫 | **通过** — `resolve/mod.rs:117` 检查 running_mode ≠ NotRunning 不设代理，防止断网 |
| 节点恢复 filterText 回退 | **通过** — `lifecycle.rs:308-344` JSON 链式降级安全、最后回退 all.first() |
| 前端 `invoke` 调用 | 4 处：`frontend_log`（有 catch）、`save_proxy_head_state`（有 catch 但空体）、`open_devtools`（返回 Promise）、`start_core_upgrade`（try/catch）。除 save_proxy_head_state 外均正确处理。 |
| `let _ =` 丢弃 Rust 错误 | 全部最佳努力操作：文件清理、自动自愈选点、通道关闭 — 设计上可接受 |
| IPC 命令注册一致性 | **通过** — `lib.rs` 的 `generate_handler!` 与前端 `invoke` 一对一匹配，`hotkey` 命令已完全移除 |
| tauri.conf.json 窗口配置 | 窗口由 Rust `resolve/window.rs` 创建，DEFAULT=285×680、MAX=640×860，与前端 `isLargeMode` 对齐 |
| eslint（4 个改动文件） | EXIT=0，无类型/语法错误 |

---

## 🟡 建议修复的项

### 🟡 1. use-head-state.ts:103 — 空 catch {} 静默吞错误

**位置**: `src/components/proxy/use-head-state.ts` 第 103 行
**现状**:
```ts
try {
  localStorage.setItem(HEAD_STATE_KEY, JSON.stringify(state))
  await invoke('save_proxy_head_state', { state })
} catch {}  // ← 静默丢弃所有 IPC 失败
```
**影响**: 前端状态被 localStorage 兜底（主存储），后端持久化仅是镜像。若 IPC 持续失败，代理栏头部状态（排序/过滤/URL）将在重启后丢失，但运行时仍可用。 **不崩溃、不丢关键数据，但失败完全不可见。**

**建议**: 加 `console.warn`（一行改动）。
```ts
} catch (err) {
  console.warn('[useHeadState] 保存 head_state 到后端失败:', err)
}
```

### 🟡 2. window-provider.tsx — ensureWindowInScreen DPI 来源不一致（多屏混合 DPI 潜在坑）

**位置**: `src/providers/window/window-provider.tsx`
**问题**: `onResized` 用 `window.devicePixelRatio` 算 logical 尺寸（行 184），而 `ensureWindowInScreen` 用 `monitor.scaleFactor`（行 38）。单一屏无差异；多屏不同 DPI（如 100%+150%）时进入大尺寸模式那一刻：
- `setSize(640×860)` 触发 onResized
- 若 `devicePixelRatio` ≠ `monitor.scaleFactor`，`isLargeSize` 判定失效
- 导致 `lastNonLargeSizeRef` 被错误写入 → 退出大尺寸恢复错误尺寸（虽被后端 `max_inner_size` 钳住，但行为怪异）

**建议**: 在 `onResized` 中也读取 `currentMonitor()` 的 `scaleFactor`，或提取为 `const scaleFactor = (await currentMonitor())?.scaleFactor || window.devicePixelRatio`。
（单屏场景无影响，多屏混合 DPI 是边缘场景，优先级可低。）

### 🟡 3. macOS 双最大化行为不一致（已知设计取舍，待确认）

**位置**: `src/providers/window/window-provider.tsx` 行 388-390
**现状**: macOS 上 `setMaximizable` 不调用 → 原生绿色按钮 zoom 保留。但自定义最大化按钮也连 `toggleMaximize` → 640×860。两套行为并存：
- 原生 green zoom → 不更新 `isLargeMode`，图标不反映状态
- 自定义按钮 → 可能叠在原生 zoom 之上（640×860 覆盖 zoom 尺寸）

**建议**: 如果 macOS 只需要原生 green zoom，移除自定义 WindowControls 的最大化图标（或改为调用 native zoom）；如果 macOS 也需要统一大尺寸模式，则 `setMaximizable(false)` 也应用到 macOS。

### 🟡 4. 跨会话恢复大尺寸后退出跳回默认尺寸（体验问题）

**位置**: `src/providers/window/window-provider.tsx` init effect（行 396-413）
**现状**: 若程序在大尺寸模式关闭、窗口状态恢复插件还原 640×860 → init 标 `isLargeMode=true` 并重置 `lastNonLargeSizeRef={285, 680}` → 退出大尺寸时恢复默认 285×680，不是上次进入前的真实尺寸。

**建议**: 将 `lastNonLargeSizeRef` 持久化到 `localStorage`，或接受默认 285×680（即"总是退出到标准窄窗"）。

---

## 💭 吹毛求疵

### 💭 5. cmds.ts — openDevtools 死代码

**位置**: `src/services/cmds.ts` 第 426 行
**现状**: `openDevtools` 函数定义且导出，后端命令 `open_devtools` 也注册了（lib.rs:166），但前端无任何调用处。
**建议**: 若不再需要可删除（前后端）；若保留作为调试入口，加个注释说明用途。

### 💭 6. Rust window.rs — 窗口创建时未即时禁用 maximizable

**位置**: `src-tauri/src/utils/resolve/window.rs` 67-76
**现状**: 窗口 builder 未调 `.maximizable(false)`，依赖前端挂载后 `setMaximizable(false)`。存在短暂竞态窗口（创建→React 挂载→setMaximizable）。
**建议**: 在 Windows/Linux 构建时加一行：
```rust
#[cfg(not(target_os = "macos"))]
builder = builder.maximizable(false);
```
消除竞态，belt-and-suspenders。

### 💭 7. _layout.tsx 可用 debugLog 保留关键 trace

**现状**: 13 处调试 `console.log` 已被删除（`b7f2d001`）。项目中 `debugLog` 函数已广泛使用（proxy-groups、delay、use-profiles 等 30+ 处）且受 `isDebugLoggingEnabled` 开关统一控制。

**建议**: 若觉得"窗口唤醒"和"Fallback 10秒触发"这 2 条 trace 对卡死排查仍有价值，可改回 `debugLog(...)` 形式——生产零成本，可用 flag 一键开启。

### 💭 8. Rust TODO 注解统计

- 前端 3 个 TODO（error-boundary 粒度、i18n fallback、错误上报未集成）
- 后端 10 个 TODO（配置版本管理、LRU 缓存、日志轮转策略、re-export 清理等）
均为改进建议，非缺陷。无 🔴 项。

---

## 正面评价

- 🔴 hotkey 系统已**完全彻底**从前后端清除（12 个热键 + 配置 + 本地化 + 事件处理），零残留
- 🔴 frontendLog 热路径 INFO 转发已全部删除并接入 `isDebugLoggingEnabled` 统一开关——生产零成本，且支持运行时 flag 一键回开
- 窗口"大尺寸模式"替换原生 maximize 的概念转换**前后一致**：`window-context.ts`、`use-window.ts`、`resize-handles.tsx`、`window-controller.tsx`、`window-provider.tsx` 全部更新，无遗漏
- 子集语义在**所有选点路径**（F1/F2/F4/防断流/自愈/唤醒）合规，filterText 持久化 → 后端自动读取逻辑正确
- 系统代理 Core 状态守卫阻止了"Core 失败但设代理"的断网漏洞
- 节点恢复的 filterText 回退逻辑：JSON 链式 `and_then` → 键缺失自动退化 → `unwrap_or_default()` 空字符串 → `all.first()` 兜底——**无 panic、无边界溢出**
- 所有 Rust `unwrap` 均在生产安全范围内（测试/脚手架/assert）

---

## 建议优先级

| 优先级 | 项 | 改动量 |
|---|---|---|
| 🟡 应立即修 | #1 空 catch {} 加 console.warn | 1 行 |
| 🟡 建议修 | #3 macOS 双最大化一致（确认意图） | 讨论 |
| 🟡 可择期 | #2 DPI 一致性 | 10 行 |
| 🟡 可择期 | #4 跨会话尺寸持久化 | 15 行 |
| 💭 | #5 删 openDevtools 死代码 | 10 行 |
| 💭 | #6 后端 maximizable(false) | 1 行 |

---

**结论**: 代码质量基线良好，无 🔴 阻断项。4 个 🟡 建议中 #1 最轻量（1 行），其余取决于你的优先级和设计取舍。全部 🟡 修完后即可发版。
