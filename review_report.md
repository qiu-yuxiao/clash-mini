# 代码审核报告 — v2.3.4 → v2.3.9 新提交

**审核范围**：`46e95505` (上次审核止于托盘图标+编译修复) → `bd141f62` (HEAD)
**提交数量**：34 commits（含 release/chore/docs 机械提交）
**实质代码提交**：约 16 个；纯版本 bump / Changelog / app-update.json 不计入实质审查
**审核模式**：仅静态审查，未修改任何代码
**审核日期**：2026-07-10

---

## 一、整体评价

本次迭代质量**总体良好（A-）**。核心目标——把选点（测速+择优+切换）统一收归后端单一引擎、统一延迟/探针阈值口径、托盘图标按接管模式自动切换、窗口尺寸阈值收口到单一常量、按窗口档位做资源梯度门控——均已完成且前后端一致。

大部分改动方向正确，与 `clash_mini_agreements.md` / working memory 既有约定吻合。发现 **1 个逻辑缺陷（🔴 建议修复）**、**3 个需确认的语义/行为变更（🟡）**、若干细节 nit（💭）。

---

## 二、逐主题审查

### 2.1 选点引擎统一收归后端（核心）
**涉及**：`src-tauri/src/module/monitor.rs`（+283/-…）、`src-tauri/src/cmd/proxy.rs`、`src/components/proxy/proxy-groups.tsx`、`src/pages/_layout.tsx`

`trigger_backend_auto_select` 签名从 `(profile_uid, sort_type)` 扩展为 `(profile_uid, node_names: Option<Vec<String>>, sort_type, select: bool)`：

- **子集语义正确**：`node_names = Some(非空)` 时仅用调用方子集（F4 传前端可见节点），剔除 dummy；`None/空` 时后端自取 PROXY 全量 + `proxy_head_state.json` 持久化 `filterText` 过滤（F1/F2、自动防断流）。与 working memory「选点限定可见子集内最快、绝不越界」一致 ✓
- **display / candidates 分离正确**：`display` 含全部节点（含 <30ms 假节点、≥2000ms 死节点、Err=1_000_000）用于展示；`candidates` 仅 `(30..2000)` 用于选点。选点用 `candidates.min_by_key(delay)`，修复了旧逻辑在 `sort_type=2`（按名称）时误选字母序首节点而非最快节点的 bug ✓
- **探针超时 = 判死阈值 = 2000ms**：`NODE_TEST_TIMEOUT_MS = NODE_DELAY_MAX_MS`，超过即按不可用，`Err` 上报 1_000_000 前端显 Error ✓
- **`auto_select_fail_count` 语义修正**：从「`!results.is_empty()` 即重置」改为「仅 `outcome.selected` 才重置」。避免「测速有结果但全死、未选中」时错误清零失败计数 ✓
- **`check_active_node_health` 修正**：判定从 `delay >= 30` 改为 `delay >= 30 && delay < 2000`。旧逻辑会把 ≥2000ms 的慢/超时节点误判为健康、不触发自愈；现已修复 ✓
- **`is_first_run` 初始化选点**：窗口不存在（纯托盘/轻量启动）时由后端执行一次初始化自动选点，且仅在 `is_first_run` 分支执行一次（**非每周期重选**，已核对主循环结构）✓
- **破坏性 API 变更已同步**：旧 `trigger_auto_select(_is_manual, sort_type)` 删除，新 4 参签名；`cmds.ts` 类型定义、3 处前端调用点（`proxy-groups.tsx:429`、`_layout.tsx:227`、`_layout.tsx:1068`）全部对齐；`lib.rs` 命令注册顺序调整无功能影响 ✓

**亮点**：选点逻辑彻底消除前后端重复测速与竞争切换，子集/全量两条路径语义清晰，互斥锁 `AUTO_SELECT_RUNNING` 贯穿所有调用方。

### 2.2 延迟/探针阈值统一 2000ms
**涉及**：`src/services/delay.ts`、`src/pages/_layout/utils/style-helpers.tsx`

- `delay.ts` 新增 `NODE_DELAY_MIN_MS=30` / `NODE_DELAY_MAX_MS=2000` 作为单一真源常量，`formatDelay`/`formatDelayColor` 默认 `timeout` 由 `10000` 改为 `NODE_DELAY_MAX_MS`；删除了冗余的 `if (delay >= 10000) return error.main`（已被 `delay >= timeout` 涵盖）✓
- `style-helpers.tsx` 的 `10000` → `NODE_DELAY_MAX_MS` 同步对齐 ✓
- 前端 `proxy-groups.tsx` 删除 `verge.default_latency_timeout` 读取，固定用 2000ms，与后端一致 ✓
- 删除了 `delay.ts` 的 `checkListDelay`（前端批量测速逻辑），已完全收归后端；`isBatchTesting` 改为恒返 `false`（仅保留 getter 兼容）。grep 确认无残留 `checkListDelay` 调用 ✓

### 2.3 托盘图标按接管模式自动切换
**涉及**：`src-tauri/src/core/tray/mod.rs`、`src-tauri/src/config/verge.rs`、`src/types/verge.ts`、`src-tauri/src/feat/{clash,config}.rs`

- `Tray::init` 由同步改为 `async`，创建托盘前提前读取 `verge` 配置，按 `enable_tun_mode`/`enable_system_proxy` 直接选 `tray-icon-tun/sys/default.png` 三张内置图标，消除「先建默认图标再竞态刷新」的静默启动 bug ✓
- `init` 调用方 `utils/resolve/mod.rs:54` 已加 `.await` 同步 ✓
- 删除了空函数 `update_menu_and_icon`（原 `pub async fn update_menu_and_icon(&self) {}` 本就不做事），`feat/clash.rs`、`feat/config.rs` 中的调用一并移除，无行为变化 ✓
- **功能削减（需确认）**：`config/verge.rs` 与 `types/verge.ts` 同步删除了 `common_tray_icon` / `sysproxy_tray_icon` / `tun_tray_icon` 三个字段及其 patch 处理。即不再支持用户自定义三套托盘图标，改为按接管模式自动切内置三图标。前后端一致、无配置破坏风险（verge 不 `deny_unknown_fields`，旧配置静默忽略），但属**有意的功能删除**，需产品确认意图（见 🟡-1）。

### 2.4 窗口尺寸阈值收口到 285/135
**涉及**：`src/constants.ts`、`src/utils/resolve/window.rs`、`src/pages/_layout.tsx`、`src/components/proxy/use-window-width.ts`、`src/providers/window/window-provider.tsx`、`src/components/layout/resize-handles.tsx`

- 后端 `DEFAULT_WIDTH` 270→285、`MINIMAL_WIDTH` 270→285、`MINIMAL_HEIGHT` 135（加注释"须与前端 `MINI_WIDTH_THRESHOLD`/`MINI_HEIGHT_THRESHOLD` 一致"），前端 `MINI_WIDTH_THRESHOLD=285`/`MINI_HEIGHT_THRESHOLD=135` 成为单一真源；重复常量 `MINIMAL_*` 已收口 ✓
- FEAT-003 stealth mode（`window-provider.tsx` idle 10s 隐藏标题栏）机制**未被破坏**，仅阈值常量统一 ✓
- 缩放手柄 `HANDLE_SIZE 6→10`：命中区变大（💭 见 nit）✓

### 2.5 窗口分级资源精简门控（perf 系列）
**涉及**：`src/providers/app-data-provider.tsx`、`src/pages/_layout/components/connections-panel.tsx`

- `getClashConfig` / `getSystemProxy` / `getRunningMode` 的 `enabled` 统一为 `isSettingsOpen && !isMiniStatus`：抽屉关闭或小窗即停 IPC。（补上了 working memory 记录的「`getClashConfig` 关抽屉未退」原漏点）✓
- `connections-panel.tsx`：窄窗口（`isMinimalWidth`）下 `<ConnectionsPanel/>` 不挂载，订阅 `useConnectionData({enabled: drawerOpen && !isMinimalWidth})`，挂载与数据订阅共用同一信号，符合「窄窗不挂载不订阅」约定 ✓
- 这些 perf 改动均先在 agreements/working memory 立规再动手，符合项目治理铁律 ✓

### 2.6 代码清理（bd141f62 + 各 cleanup）
- `feat/clash.rs` 删除未用 import；`cmd/network.rs` 把函数内 `use` 提到文件顶部消除重复；`cmd/save_profile.rs` 删除测试未用 import；`hooks/use-clash.ts` 删除未用 `invalidateClashConfig`；`window_manager.rs` 的 `create_window` 从 `Pin<Box<dyn Future>>` 改为原生 `async fn`（更清晰）。均为安全清理 ✓

---

## 三、问题清单

### 🔴 逻辑缺陷（建议修复）

> **【2026-07-10 复审更正】R1 为误报，已撤下。**
> 原判断"第 414 行 `if (!currentUid) return` 未复位 `testingGroups` → 按钮永久卡死"不成立。
> 实际代码里该 `return` 位于 `try` 内，而复位语句 `setTestingGroups((prev) => ({ ...prev, [groupName]: false }))` 在外层 `finally`（约第 445 行）。
> JavaScript 保证：只要进入 `try`，`finally` 必执行——即便 `try` 内 `return`/`throw` 也照跑。故 `currentUid` 为空时，flag 先置 `true` 随即被 `finally` 置回 `false`，**按钮不会被锁死**。
> 该提前 `return` 的真实效果仅为：无 profile 时跳过视觉占位与后端选点（本身是有意设计），且因 `setResizable(false)` 也在其之后，窗口拖拽不会被锁。唯一副作用是 flag 瞬闪一下"测速中"，无害。
> 结论：**本区间无 blocker 级逻辑缺陷。** 下方 S1/S2/S3 经用户确认均属设计意图。

### 🟡 需确认的语义/行为变更

**S1. 托盘三图标自定义字段被删除（功能削减）**
`common_tray_icon`/`sysproxy_tray_icon`/`tun_tray_icon` 前后端同步移除。技术无害，但意味着用户不再能自定义三套托盘图标。请确认这是产品既定意图（按接管模式自动切内置图标），而非误删。

**S2. 窗口高度阈值放宽带来的行为面扩大（需回归）**
- `isMiniStatus` 高度阈值 `100 → 135`：高 101–135 的窗口现被归为小窗，触发更激进的 DOM 卸载/通道关闭。
- `window-provider.tsx` 高度阈值 `130 → 135`：高 131–135 的窗口现在会进入 stealth 隐藏标题栏。
- 三处统一向「285×135」规格对齐，方向正确，但触发面扩大，建议回归验证小窗边界（尤其是 130–135 高度区间）的 UI 表现。

**S3. `use-window-width.ts` 返回值 `270 → 285` 的布局影响**
原返回 `270` 是历史真实渲染宽，改为 `MINI_WIDTH_THRESHOLD(285)` 后，下游以该值做宽度判断/布局的可能与遗留 270px CSS 产生错位。建议确认下游消费方是否需要同步调整。

### 💭 Nits

- **N1. `monitor.rs` `check_active_node_health` 的 else 分支注释过时**：现 else 同时覆盖 `<30ms`（假节点）与 `≥2000ms`（超时节点），但注释仍只写"<30ms 假节点"，易误导。建议更新注释。
- **N2. `resize-handles.tsx` `HANDLE_SIZE 6→10`**：边沿可拖拽命中区变大，狭小窗下可能与 stealth 点击恢复/拖拽判定轻微耦合，建议确认交互无冲突。
- **N3. `monitor.rs` 移除 `cancel_active_auto_select`**：原 profile 切换时强制中止测速任务的逻辑被删除，现统一依赖 `AUTO_SELECT_RUNNING` 互斥 + `current_uid` 双重校验。常规场景已覆盖，但极快的 profile 连切理论上仍可能让上一次测速结果作用于新 profile（双重校验会拦截切换，风险低）。

---

## 四、亮点（值得肯定）

1. **选点引擎彻底单点化**：F1/F2/F4/自动防断流四路全部走后端 `trigger_backend_auto_select` 单引擎，子集语义（F4 传可见子集、其余传 None 走全量+filterText）前后端完全一致，根除重复测速与竞争切换。
2. **阈值口径全链路统一**：`NODE_DELAY_MIN_MS/MAX_MS` 成为前后端单一真源，探针超时 = 判死阈值 = 2000ms，UI 展示（formatDelay/Color）与后端判定不再各自漂移。
3. **性能治理合规**：窗口分级资源精简均先立规（agreements / working memory）再改代码，且 `enabled` 门控与 DOM 挂载共用同一信号，无「挂载了却不订阅」或「订阅了却不挂载」的脱节。
4. **破坏性 API 变更零遗漏**：`trigger_auto_select` 4 参签名、`Tray::init` async、`create_window` async 的调用方全部同步，grep 验证无残留旧调用。

---

## 五、结论

- **可放行**：本区间经复审**无 blocker 级问题**（R1 已撤为误报）；S1/S2/S3 经用户确认均属设计意图，不阻断发版。
- **建议**：无需为 R1 改动代码。S2/S3 如仍有顾虑可做一次小窗边界手动回归，但已确认为设计意图。

> 本报告为静态审查产物，未对任何代码文件做修改。
