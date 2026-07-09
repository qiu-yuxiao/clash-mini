# 窗口尺寸分级精简 Checklist（以小窗口模式为蓝本）

> 设计基线：Clash Mini 的窗口布局采用**遮挡模式（Occlusion Model）**——
> 元素不随窗口缩放而缩放/重排，窗口变小只是"取景框"变小，被框住的内容之外的部分被遮挡。
> 因此"窗口越小 → 需要挂载的 DOM 越少、需要订阅的数据通道越少 → 越省资源"。
>
> 用户把窗口从**小 → 默认 → 窄 → 大**逐级定义，正是这个"由小到大、细节递增"的资源梯度。
> 本 checklist 先把**小窗口（285×135）**这一最省资源的形态拆透，作为后续窄/大窗口的参照系。
>
> 标注说明：
> - ✅ DONE = 已实现（包括"轻量模式"及既有代码已做好的精简，列在此供举一反三）
> - 🔲 TODO = 仍可进一步精简的候选点，附收益/风险/建议，待确认后实施
> - ❌ REJECTED = 经你裁定不实施（会伤 UX 或纯属信息不全的误判）
> - ♻️ REDUNDANT = 已被既有机制覆盖，无需再做

> **⚠️ 工作铁律（2026-07-09 裁定）**：
> 此后凡是我认为"还能进一步精简"的点，**必须先把点子拿出来跟你核对、经你同意后才动手**。
> 不得自行实施。原因：有些点实际无效/收益极小，有些是我没掌握既有机制（如 FEAT-003）而
> 提出的误判。下面的 A/B/C–G 即首轮核对结果。

---

## 一、小窗口（285×135）当前渲染树

按遮挡模式，小窗口可见区只应有：

| 区域 | 高度 | 小窗口是否挂载 | 说明 |
|------|------|----------------|------|
| 自定义标题栏 (customTitlebar) | 30px | ⚠️ 仍挂载（Win/Linux） | 仅含图标+标题+最小化/关闭按钮 |
| 顶部控制栏（活跃出口节点栏 + 置顶 + 设置） | 30px | ✅ 挂载 | 小窗口核心信息，必须可见 |
| 代理节点表（ProxyGroups 虚拟列表） | 撑满 | ❌ **不挂载** | 被遮挡，无需渲染 |
| 设置抽屉（左栏+连接面板） | 撑满 | ❌ **不挂载** | 被遮挡，无需渲染 |
| 底部流量条（MiniTrafficPanel） | 100px（小窗） | ✅ 挂载 | 用轻量 SVG 图 |

> 观察：标题栏(30)+控制栏(30)+流量条(100)=160px > 可用 135px，超出的 25px 被 `overflow:hidden`
> 裁掉（落在流量条底部）。活跃节点栏在最上、可见；符合遮挡预期。若想让流量条完整可见，
> 收回标题栏 30px 即可（见 TODO-B）。

---

## 二、数据通道（Data Channels）

### ✅ DONE
- [x] **代理全量数据走精简路径**：`app-data-provider.fetchProxies` 在 `isMiniStatus` 下只取
  `PROXY` 组 + 当前活跃节点，不再 `calcuProxies()` 拉全量组/节点/records。
- [x] **代理供应商 / 规则供应商 / 规则列表 关闭订阅**：`proxyProviders / ruleProviders / rulesData`
  的 `useQuery` 均 `enabled: !isMinimalWidth`，小窗不拉。
- [x] **连接（Connections）数据关闭**：`useConnectionData({ enabled: drawerOpen && isPanelVisible })`，
  小窗抽屉不打开 → 不订阅连接流。
- [x] **运行时长（uptime）查询 `enabled:false`**：永不拉取。
- [x] **系统代理 / 运行模式 仅在设置打开时拉**：`enabled: isSettingsOpen`，小窗不触发。
- [x] **流量 WS 在窗口隐藏时暂停**：`useTrafficData` 的 `active = enabled && isVisible`。
- [x] **轻量模式（终极精简，参照系）**：Rust 侧 `entry_lightweight_mode` 直接销毁主窗口 +
  断所有 WebSocket 订阅 + 连接 GC，前端 DOM/订阅全部归零。这是"已经精简很好"的范本。

### 🔲 TODO（候选）
- [x] **TODO-A `clashConfig`（getBaseConfig）小窗仍常驻订阅 — ✅ 已实施（2026-07-09 你裁定通过）**
  - 现状（改前）：`useQuery(['getClashConfig'], getBaseConfig)` 无窗口尺寸门控，小窗每次刷新都拉。
  - 事实：小窗可见区（活跃节点栏、流量条）都不读 `clashConfig`；它只服务于设置抽屉/接管模式
    （`_layout.tsx:1801/1814` 均为 `{!isMiniStatus && (...)}`，小窗不挂载）。
  - 改动：`app-data-provider.tsx` 给该 `useQuery` 加 `enabled: !isMiniStatus`。
  - 已核实的安全边界：
    - `isCoreDataPending = isProxiesPending || isClashConfigPending` 无人 UI 消费（`useAppData`
      仅打包导出，组件无读取），故 `enabled:false` 不会让小窗卡加载态。
    - `systemValue.systemProxyAddress` 仅在 `isSettingsOpen` 时用 `clashConfig`，小窗抽屉不开，无影响。
    - `refreshAll` 经 `forceFullProxiesRef` 绕过精简路径，但 settings 在小窗不可达，不会误触发。
  - 收益：小窗省一次 GetBaseConfig IPC + 内核 config 解析；`refreshAll`/profile 切换时少一笔。
- [ ] **TODO-D `verge://refresh-proxy-config` 重拉可更激进去抖 — ❌ REJECTED（你裁定：伤 UX）**
  - 现状：内核每次代理变更事件都触发 `refreshProxy()`，小窗走精简 fetch（仍含 2 次
    `getProxyByName` IPC 往返）。
  - 裁定理由：更粗去抖会拖慢活跃节点切换的即时反馈，影响操作体验。小窗首要诉求是"切得快"，
    不是"省 IPC"，故不做。
- [ ] **TODO-F `profile-changed` 监听在小窗可短路 — ❌ REJECTED（你裁定：伤 UX/操作）**
  - 现状：监听常驻，触发 `refreshRules/refreshRuleProviders`（小窗被 `enabled` 门控为 no-op）
    但仍 `invalidateQueries(['getProfiles'])`。
  - 裁定理由：短路监听会改变 profile 切换的刷新语义，存在回归风险且收益极小，不做。

---

## 三、渲染画面内容（Rendered Content）

### ✅ DONE
- [x] **代理节点表（虚拟列表）不渲染**：`_layout.tsx` 中 `{!isMiniStatus && <ProxyGroups/>}`。
- [x] **设置抽屉不渲染**：`{drawerOpen && !isMiniStatus && <设置抽屉>}`。
- [x] **流量图选用轻量组件**：`MiniTrafficPanel` 在 `isMinimalWidth` 时用 `TrafficGraph`（SVG）
  替代 `EnhancedCanvasTrafficGraph`（Canvas），小窗不跑 Canvas 绘制。
- [x] **宽/窄窗口列数自适应**：`calculateColumns` 中 `width<=285 → 1 列`，`>285 → 3 列`
  （依据 agreements 第九条），小窗只渲染 1 列（即便表渲染也不会铺 3 列）。

### 🔲 TODO（候选）
- [ ] **TODO-B 标题栏在小窗可收回 — ♻️ REDUNDANT（你裁定：既有机制已覆盖）**
  - 原提议：收标题栏 30px 让流量条完整可见。
  - 裁定理由：**已有 FEAT-003 空闲自动隐藏标题栏机制** —— `window-provider.tsx` 的
    `idleTimerRef`（`IDLE_HIDE_DELAY_MS = 10_000`）：当窗口满足
    `宽≤285 且 高≤135`（即小窗口）且用户无操作 **10 秒后**，`setIsDecorationsHidden(true)`，
    `_layout.tsx:1522` 据此隐藏整个自定义标题栏、改由 `GlowBorder` 替代；活跃节点栏随即
    获得空位完整显示。**所以"小窗标题栏腾位"已由现有 stealth 机制实现，无需再改。**
  - 教训：这是我没掌握既有机制时提出的误判，印证了"先核对再动手"的必要性。
- [ ] **TODO-C 流量 WS 采样率在小窗可减半 — ❌ REJECTED（你裁定：伤 UX）**
  - 现状：`useTrafficData` 节流 `throttleMs: 1000`。
  - 裁定理由：1s→2s 会让流量图视觉变卡顿，影响观感，不做。

---

## 四、DOM（已挂载但未用的节点）

### ✅ DONE
- [x] **代理表 DOM 整体卸载**（见上），不是隐藏而是**不挂载**，无虚拟列表开销。
- [x] **设置抽屉 DOM 整体卸载**。

### 🔲 TODO（候选）
- [ ] **TODO-E 缩放热区（ResizeHandles，8 个透明 div）小窗可精简 — ❌ REJECTED（你裁定：伤 UX/操作）**
  - 现状：`ResizeHandles` 在任意模式下都挂载 8 个透明 div 模拟边/角缩放。
  - 裁定理由：小窗仍需能拖拽缩放（尤其从 stealth 态放大回去），去掉热区会牺牲操作便利，不做。
- [ ] **TODO-G 三处重复的 resize/visibilitychange 监听可合并（横切项，非小窗专属）— ❌ REJECTED（你裁定：伤 UX/操作）**
  - 现状：`window-provider`、`_layout`、`app-data-provider` 各加一套 resize/focus/visibilitychange
    监听，三套并存。
  - 裁定理由：合并属较大重构，动到尺寸判定与事件链路，易引入回归、影响操作稳定性，不做。

---

## 五、小结与后续

- **小窗口当前已做到的精简**：代理表 DOM 不挂载、设置抽屉不挂载、代理数据走精简 fetch、
  供应商/规则/连接/uptime 订阅全关、流量图用 SVG 轻量组件、流量 WS 隐藏即停。
  这已是一个相当"瘦"的小窗形态，与遮挡模式原则一致。
- **首轮核对结果（2026-07-09，逐条经你裁定）**：
  - ✅ **TODO-A 已实施**：`clashConfig` 查询加 `enabled: !isMiniStatus`，小窗不再拉取。
  - ♻️ **TODO-B 冗余**：标题栏"10 秒空闲自动消失"已由 FEAT-003 实现，无需再改。
  - ❌ **TODO-C / D / E / F / G 全部否决**：做了都会伤 UX（流量图变卡、切节点反馈变慢、
    缩放热区缺失、监听语义改动等），不动。
- **后续规则**：任何新的"还能再精简"点子，先拿出核对、经你同意再改。
- **举一反三的下一步**：把同一张三轴 checklist 套到**窄窗口（285×(135,860]）**与
  **大窗口（(285,640]×(135,860]）**，看哪些数据通道/渲染可在"比小窗大、比默认瘦"的区间
  进一步分级（例如窄窗是否也可不挂设置抽屉、大窗是否常驻连接面板等）。本次先聚焦小窗。

---

_注：本文件为工程内部 checklist（非用户需求文档），仅记录可精简点与状态；窗口尺寸的
用户侧定义以 `clash_mini_agreements.md` 为准。_
