# 内存优化考察报告

**考察日期**：2026-06-28  
**考察范围**：全代码库（Rust 后端 + React 前端）  
**考察角度**：节约系统内存占用

---

## 🔴 高优先级（可量化 MB 级节省）

### H1. 前端：代理数据三重复制

**文件**：`src/services/cmds.ts:176-279` (`calcuProxies`) + `src/providers/app-data-provider.tsx:189-199`

**现状**：`calcuProxies()` 返回的数据结构中，同一个 `IProxyItem`（300+ 节点时每个节点 ~2-5KB）被复制了三遍：
```
global.groups[].all[]  ← IProxyItem 副本 1
records{}              ← IProxyItem 副本 2  
QueryClient 缓存       ← 整个响应体的副本 3
```

300 个节点的机场，这份数据轻松占到 3-8MB，三重复制就是 10-25MB。

**建议**：
1. `groups[].all` 中只存节点名称（`string[]`），不存完整 `IProxyItem`
2. 前端通过 `records[name]` 按需查找完整节点数据
3. 参考 normalizr 的数据标准化模式

**预估节省**：5-15MB（取决于订阅规模）

---

### H2. 前端：连接数据 gcTime: Infinity，永久累积

**文件**：`src/hooks/use-connection-data.ts:9, 231-232`

**现状**：
```typescript
const MAX_CLOSED_CONNS_NUM = 500
// ...
gcTime: 30_000  // 看似 30 秒过期
```
但 WebSocket 每秒通过 `queryClient.setQueryData` 推送新数据，**重置了 gcTime 计时器**。结果：只要连接面板打开，`closedConnections` 的 500 条记录**永远不被 GC**。

每条约 300-500 字节，500 条 = 150-250KB。单个不大，但加上 React 渲染树中的虚拟 DOM 副本，实际持有量可能翻倍。

**建议**：
- 将 `gcTime` 改为 `staleTime: Infinity, gcTime: 0`，或
- 前端不保留 closedConnections 缓存，改用后端分页查询
- 将 `MAX_CLOSED_CONNS_NUM` 降到 200

**预估节省**：200-500KB + 减少每秒的虚拟 DOM diff

---

### H3. 后端：`enhance()` 中 Mapping 每轮克隆 ~7 次

**文件**：`src-tauri/src/enhance/mod.rs`

**现状**：配置增强流水线中，`serde_yaml_ng::Mapping`（完整 Clash 配置 YAML AST）被反复 `.to_owned()` / `.clone()`：

```
process_profile_items (行 332-359):
  config = use_seq(rules, config.to_owned(), "rules")       ← 克隆 1
  config = use_seq(proxies, config.to_owned(), "proxies")    ← 克隆 2
  config = use_seq(groups, config.to_owned(), "proxy-groups") ← 克隆 3
  config = use_merge(&merge, config.to_owned())              ← 克隆 4
  use_script(script, config.clone(), ...)                   ← 克隆 5

process_global_items (行 301-306):
  config = use_merge(&merge, config.to_owned())              ← 克隆 6
  use_script(script, config.clone(), ...)                   ← 克隆 7
```

`enhance()` 入口 (行 93)：`clash_arc.0.clone()` — 克隆 8。

每次 enhance 运行，Mapping 被深拷贝 7-8 次。虽然 enhance 完成后临时内存会被释放，但在增强过程中峰值可达 **配置大小的 7-8 倍**。对于大订阅（30MB+ YAML），峰值可达 **200MB+**。

**建议**：
- `use_seq` 和 `use_merge` 改为 `&mut Mapping` 原地修改
- `use_script` 如需保持原始副本，用 `Arc<Mapping>` 共享而非 clone
- 优先改造 `use_seq`（纯追加操作，不需要 owned 副本）

**预估节省**：大订阅场景下峰值内存减半

---

### H4. Rust 后端日志无限增长风险

**文件**：`src-tauri/src/core/manager/state.rs:114` / `CLASH_LOGGER` (外部依赖)

**现状**：mihomo 核心的 stdout/stderr 通过 `CLASH_LOGGER.append_log()` 持续追加。`get_logs()` 返回**全部**累积日志的 `Vec<CompactString>`。侧载模式下，只要核心进程不重启，日志就无限增长。

每行日志 ~50-200 字节，10 万行 = 10-20MB。

**建议**：
- 在 `CLASH_LOGGER` 上设置环形缓冲区（如保留最近 5000 行）
- 或在 `get_clash_logs` 返回时截断

**预估节省**：长时间运行避免 10-50MB 额外占用

---

## 🟡 中优先级

### M1. 后端：`trigger_backend_auto_select` 中 32 个 String clone

**文件**：`src-tauri/src/module/monitor.rs:312-329`

**现状**：32 个并发测速任务，每个都 clone 一份 `test_url`（`String::clone()`）：
```rust
let test_url = test_url.clone();  // 32 次
```
每个 `String::clone()` 是 O(n) 的深拷贝。

**建议**：改为 `Arc<str>` 或 `Arc<String>` 共享引用。32 次 clone → 32 次 Arc 引用计数加一。

**预估节省**：每次并发测速少分配 32 个 String（~1-2KB）

---

### M2. 前端：`DelayManager` setInterval 永不清理

**文件**：`src/services/delay.ts:30-43`

**现状**：
```typescript
setInterval(() => { /* 清理过期缓存 */ }, 2 * 60 * 60 * 1000)
```
没有保存 `intervalId`，永远无法 `clearInterval`。即使应用"退出"（实际是 hide 窗口），这个定时器也会一直在后台跑。

**建议**：保存 interval ID，提供 `destroy()` 方法，在 `window.beforeunload` 时调用。

**预估节省**：原则性修复，实际内存消耗低（2 小时间隔）

---

### M3. 前端：Layout 组件 30+ useState 导致频繁全量重渲染

**文件**：`src/pages/_layout.tsx:420-1137`

**现状**：Layout 组件直接声明了 30+ 个 `useState`，任何一个更新都触发整个 Layout 重新渲染（虽然子组件有 memo，但 reconciliation 本身也有开销）。

**建议**：拆分为多个自定义 hook（`useSkinSettings`, `useUpdateState`, `useProfileEditor`），将不相关的状态分组，限制重渲染范围。

**预估节省**：减少不必要的组件树 reconciliation，降低 CPU 和 render-phase 临时内存

---

### M4. 前端：流量采样器 Web Worker 单例不释放

**文件**：`src/hooks/use-traffic-monitor.ts:353-359`

**现状**：`TrafficWorkerClient` 在模块级声明为 `let workerClient = null`（单例）。组件卸载后 Worker 和 sampler 实例继续持有 10+60 分钟的流量数据缓冲区。

**建议**：增加引用计数，最后一个订阅者退出时 `workerClient.terminate()` 并置 null。

**预估节省**：非流量面板场景下释放 1-2MB（取决于采样密度）

---

### M5. 后端：`get_merged_proxies` 无 `with_capacity` 预分配

**文件**：`src-tauri/src/enhance/mod.rs:788-855`

**现状**：`raw_proxies` 和 `all_proxies` Vec 通过 `Vec::new()` 创建，经历多次 reallocation。

**建议**：根据订阅数量预估容量：
```rust
let mut raw_proxies = Vec::with_capacity(estimated_count);
```

**预估节省**：减少 2-3 次 reallocation，少分配几十 KB 临时内存

---

### M6. 前端：`filterConn` useMemo 每 tick 全量遍历

**文件**：`src/pages/_layout.tsx:1070-1084`

**现状**：连接 WebSocket 每秒推送 delta 更新，但 `filterConn` 的 `useMemo` 依赖 `connectionsData` 整体引用——每次 delta 变化都触发全量 `conns.filter()` + 排序，即使只改了一条连接。

**建议**：使用 selector 模式或 `useMemo` 的 `equals` 比较函数，仅在排序或过滤参数变化时重算。

**预估节省**：每秒减少 200 条连接的全量 filter/sort 遍历

---

## ⚪ 低优先级

| # | 项目 | 文件 | 建议 |
|---|------|------|------|
| L1 | ProxyVirtualList/ChainRuleHeader 未 memo | `proxy-groups.tsx:629,760` | 添加 `React.memo` |
| L2 | ProxyItem 大型内联 sx 回调 | `proxy-item.tsx:111-168` | 提取为模块级函数 |
| L3 | `test_url.to_string()` 在健康检查中 | `monitor.rs:312` | 缓存为 `Arc<str>` |
| L4 | `format!()` 在日志宏中 156 处 | 全局 | 使用惰性求值（`if log_enabled!()` 守卫） |
| L5 | 7 层 React Context 嵌套 | `app-data-provider.tsx:472-489` | 合并或换 zustand/jotai |
| L6 | `filterSort` 中 group cache 可能膨胀 | `use-render-list.ts:386-401` | 对 `groupCacheRef` 设置上限 |

---

## 📊 预估总体收益

| 优化项 | 场景 | 预估节省 |
|--------|------|----------|
| H1 代理数据去重 | 300+ 节点机场 | **5-15 MB** |
| H3 enhance 克隆链 | 大订阅增强时 | **峰值减少 50%** |
| H4 日志环形缓冲 | 长时间运行 | **10-50 MB** |
| H2 连接数据 GC | 常驻运行 | **0.5-2 MB** |
| M1-M6 合计 | 常态运行 | **2-5 MB** |

**如果全部落地，在典型用户场景下可节省 20-70MB 内存**，大订阅场景下峰值节省更多。

---

## ✅ 已经做对的设计

- `smartstring::alias::String` 内联优化（Rust 端大量使用）
- `CompactString` 用于日志行存储
- `useVirtualizer` 虚拟列表（ProxyGroups + ConnectionTable）
- `useStableFn` ref 模式避免闭包陷阱
- WebSocket 共享订阅引用计数
- `MIHOMO_WS_STREAM_BUFFER_SIZE = 8` 有界通道
- `Draft<IVerge>` 使用 `Arc<RwLock>` COW 模式
- `with_data_modify` 的读写锁分离设计
- `useVisibility` 的 1 秒防抖
