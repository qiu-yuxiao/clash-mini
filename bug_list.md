# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> > **🤖 AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES FOR AI AGENTS)**:
> > 
> > 本文件使用 **「结构化卡片 + 极简索引表」** 的双轨设计。所有参与本项目的 AI 协同助理在读取 and 更新本文件时，必须严格遵守以下书写与维护规范，严禁格式退化：
> > 
> > 1. **严禁在表格中堆叠排查细节**：只有已被 Master 验证确认关闭的 Bug 才能放入「已解决的历史 Bug 索引」表格中，且状态统一写为 `代码已修正，已确认`。表格中禁止包含任何 `设计要求`、`代码状态` 等冗余的后台开发过程细节。
> > 2. **待验证/排查中 Bug 强制卡片化**：任何状态为 `排查中` 或 `代码已修正，待确认` 的 Bug，必须在顶部的「待验证与活动中 Bug 详情」区以独立标题 and 结构化字段登记。
> > 3. **强制记录排查记忆，防止重复劳动**：在卡片中，必须详实搜集、继承并持久化记录以下三个协同要素：




### 🚨 **BUG-057** 导入订阅后内核节点已展开但前端表格显示空白
* **目标版本**：`v4.1.0`
* **当前状态**：`代码已修正，已确认`
* **【现象与复现路径】**：
  * 导入订阅链接后，节点数据在后台内核（Mihomo/Clash core）中已正确解开与加载，但前端主页的节点表格（`ProxyGroups`）依然显示空白，没有任何节点数据渲染出来。首次导入后永远空白，不会自动恢复。
* **【根因分析】**：
  * **根因1**：双重 `enhanceProfiles()` 竞态调用 —— `handleImportProfile` 显式调用 `enhanceProfiles()`，同时 `useEffect` 检测到 `currentProfileUid` 变化后再次调用，导致内核配置被双重重载，第二次重载清空了第一次已加载的 proxy-provider 数据。
  * **根因2**：`use-render-list.ts` 的空数据 re-fetch 机制失效 —— 原有逻辑仅检测 `!groups.length`（完全无组），但实际场景是 PROXY 组已存在、其 `all` 数组为空（proxy-provider 尚未异步下载完成），此场景未被覆盖，导致前端永远不重试刷新。
  * **根因3**：`calcuProxies()` 空值崩溃 —— 当 `proxyResponse.proxies` 为 `null` 或 `providerRecord` 结构不完整时，后续 `.flatMap()` / `.reduce()` 直接抛出异常，阻断数据流。
* **【v4.1.0 修正方案】**：
  * **修改文件**：
    * [**`_layout.tsx`**](file:///C:/Users/sun_y/Documents/trae_projects/Clash_Mini/src/pages/_layout.tsx) (布局/导入逻辑)
    * [**`use-render-list.ts`**](file:///C:/Users/sun_y/Documents/trae_projects/Clash_Mini/src/components/proxy/use-render-list.ts) (前端节点渲染列表)
    * [**`cmds.ts`**](file:///C:/Users/sun_y/Documents/trae_projects/Clash_Mini/src/services/cmds.ts) (数据计算服务)
  * **设计要点**：
    * 1. **`_layout.tsx`**：在 `handleImportProfile` 的 try 和 catch 分支中，`mutateProfiles()` 之后立即设置 `lastEnhancedProfileRef.current = targetUid`，防止后续 `useEffect` 重复调用 `enhanceProfiles()`；同时在两个分支的 `enhanceProfiles()` 之后增加 `patchClashMode('rule')`，确保导入后切换到 rule 模式显示节点列表。
    * 2. **`use-render-list.ts`**：替换原有仅检测 `!groups.length` 的死代码，新增智能轮询恢复机制 —— 当 PROXY 组存在但 `all` 中无真实节点时（proxy-provider 异步加载中），启动 1s 间隔轮询 `refreshProxy()`，直到节点数据到来后自动停止。
    * 3. **`cmds.ts`**：`calcuProxies()` 加入 `proxyResponse?.proxies || {}` 和 `providerResponse || {}` 空值防御；`providerMap` 构造加入 `item?.proxies` 安全检查；`generateItem` 对无名称节点返回安全的 unknown 对象。



### 🚨 **BUG-056** 程序退出时残留内核/服务孤儿进程
* **目标版本**：`v4.0.1`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 程序在退出时没有确保能把自己相关的所有进程（如内核、服务或守护进程）完全杀死，导致后台残留孤儿进程。
* **【历史诊断与物理实证】**：
  * **已确认事实**：
    * [x] **官方同名进程冲突**：官方客户端也使用 `verge-mihomo` 等名称，如果直接强杀该名称会影响宿主机上正常运行 of 官方版内核。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`Cargo.toml`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Cargo.toml) (根依赖)
    * [**`src-tauri/Cargo.toml`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/Cargo.toml)
    * [**`state.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/state.rs) ( 核心进程强杀逻辑)
    * [**`window.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/window.rs) ( 退出清理事件集成)
    * [**`verge.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/verge.rs) ( 内核配置更名)
    * [**`chain.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/chain.rs) ( 支持映射更名)
    * [**`tauri.conf.json`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.conf.json)
    * [**`tauri.linux.conf.json`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.linux.conf.json)
    * [**`prebuild.mjs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/scripts/prebuild.mjs)
    * [**`portable.mjs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/scripts/portable.mjs)
    * [**`portable-fixed-webview2.mjs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/scripts/portable-fixed-webview2.mjs)
    * [**`installer.nsi`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/packages/windows/installer.nsi)
  * **设计要点**：
    1. 将 sidecar 核心二进制及运行进程由 `verge-mihomo`/`verge-mihomo-alpha` 彻底更名为 `mini-mihomo`/`mini-mihomo-alpha` (增加 `mini-` 前缀)，实现与官方客户端的物理隔离；
    2. 后端引入 `sysinfo` 库，在主程序退出事件 `clean_async` 和侧边栏停止核心 `stop_core_by_sidecar` 中，扫描系统进程并强杀所有名称包含 `mini-mihomo` 的残留进程，确保零残留且不误伤官方客户端。

---


### 🚨 **BUG-054** 多套控件皮肤风格（Skin Swapping）切换及滑块参数重定义
* **目标版本**：`v3.0.4`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 多套控件皮肤风格切换支持，各皮肤下的滑动条对应底层参数在不同皮肤下需要完全重定义且独立保存，且 270px 窄窗口下换肤选择器需要支持物理裁剪。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`_layout.tsx`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)
    * `use-custom-theme.ts` ( 样式管理)
  * **设计要点**：
    1. 支持五套风格：`Retro 3D`、`Modern Flat`、`Frosted Glass`、`Cyberpunk`、`Monochrome`；
    2. 增加 Excel 风格单行选择器，排列尺寸固定 `462.5px`，左定位 `177.5px`，实现窄窗口物理裁剪，选中单元格渲染真实皮肤控件外观；
    3. 两个滑块控制参数在不同皮肤下重定义，且在 `localStorage` 中独立存储；
    4. 切换风格时中英文字体共鸣对齐；
    5. 按钮有 `0.15s` 过渡动画，大卡片/背景/分栏瞬切；
    6. 保持中缝双线、网格线和 GlowBorder 呼吸灯样式不变；
    7. 初始默认启动尺寸 270x680 并完好显示日志按钮。

---

### 🚨 **BUG-053** 首次导入/更新订阅自动切换激活及超时防误报繁忙逻辑
* **目标版本**：`v3.0.4`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 首次导入或更新订阅后，未能自动切换/激活该配置文件，且由于 Clash 核心下载/解析 Provider 耗时，导致容易在 5 秒内因无可用节点而误报“所有线路都繁忙”。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`_layout.tsx`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) ( `handleImportProfile` 与测速轮询逻辑)
  * **设计要点**：
    1. 首次导入配置文件后自动将 `current` 切换为该新配置 of UID 并激活；
    2. 延长前端等待 Clash 内核就绪并获取可用节点的超时时间至 20 秒；
    3. 轮询期间若前 6 秒仍无可用节点，后台触发前端的直接延迟测速（`checkListDelay`）作为 fallback，避免误报繁忙。

---

### 🚨 **BUG-052** 自动连切背景运行时弹窗打扰与存活检测阈值过低
* **目标版本**：`v3.0.4`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 后台自动连切功能频繁弹窗通知已切换至最快节点（但实际并没有改动节点），且后台健康检测阈值过低（1500ms），导致稍微有点延迟 of 节点被误判为坏掉并反复拉起全量测速。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`_layout.tsx`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) ( `triggerAutoSelectFastestNode` )
  * **设计要点**：
    1. 在 `triggerAutoSelectFastestNode` 中引入 `isBackground` 判定；
    2. 当为背景运行且最快节点未改变（当前已是最快）时，完全静默不弹窗；
    3. 后台 `checkNode` 的存活检测健康阈值放宽至 `3000ms`，且移除前置加载通知。

---

### 🚨 **BUG-051** 流量小卡片上传下载总量数值常驻为 0
* **目标版本**：`v3.0.4`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 窗口下方的表示流量的小卡片里，上传下载总量的数值一直是 0。
* **【历史诊断与物理实证】**：
  * **已确认事实**：
    * [x] **缓存键丢失**：useConnectionData 禁用 WebSocket 后缓存键为 null，轮询获取的总量数据在 `queryClient.setQueryData` 找不到存储地址而被丢弃。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`use-mihomo-ws-subscription.ts`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-mihomo-ws-subscription.ts)
    * [**`use-connection-data.ts`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-connection-data.ts)
  * **设计要点**：
    1. 为 `useMihomoWsSubscription` 增加 `buildCacheKey` 选项；
    2. 优化 `responseCacheKey` 计算逻辑，当 WebSocket 禁用时 fallback 使用 `buildCacheKey` provide 静态缓存键；
    3. `useConnectionData` 传入对应的 `buildCacheKey`。

---

### 🚨 **BUG-049** 节点活性监控 useEffect 定时器反复重置失效
* **目标版本**：`v3.0.2`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 活跃出口节点活性监控定时检测（自动测速与切换）在运行中失效。
* **【历史诊断与物理实证】**：
  * **已确认事实**：
    * [x] **依赖变动重置**：useEffect 依赖了频繁变动的 `proxies` 状态与 `triggerAutoSelectFastestNode`，导致定时器在倒计时完成前反复被重置清除，无法正常在后台运行测速调度。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`_layout.tsx`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)
  * **设计要点**：
    * 将活性监控 `useEffect` 中的外部状态依赖解耦，仅依赖于 `currentProfileUid`；使用 stable Refs 缓存 `proxies` 和 `triggerAutoSelectFastestNode`，使后台定时器能在长周期内平稳不间断运行，达到真正的 60 秒定期测速与失效自动连切。

---

### 🚨 **BUG-043** 全自动订阅链接格式智能识别与并联订阅合并
* **目标版本**：`v2.0.6`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 缺乏全自动链接格式识别（YAML与Base64）与默认并联订阅合并。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`prfitem.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/profile/prfitem.rs) ( 后端订阅处理)
    * [**`enhance/mod.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)
  * **设计要点**：
    1. 从 UI 移除兼容模式开关，将输入框及按钮文字恢复为标准文案；
    2. 后端 `prfitem.rs` 移除开关配置判断，YAML 校验失败时直接尝试 Base64/URI 解析并对垃圾数据输出自定义中文报错；
    3. `enhance/mod.rs` 中默认开启并联订阅合并编译。

---

### 🚨 **BUG-042** 非 YAML 格式订阅链接本地通用解析机制
* **目标版本**：`v2.0.4-full`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 缺乏对非 YAML 格式订阅链接（如 Base64 编码的 ss://, trojan://, vmess:// 链接列表）的直接支持，无法进行兼容转换。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`universal_parser.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/universal_parser.rs) ( 本地通用解析器)
  * **设计要点**：
    * 在“兼容模式”下提供通用 URI 解析管道与多订阅合并冲突处理。支持自动对 `ss://`、`trojan://`、`vmess://` 的单行 URI 或 Base64 编码的订阅源进行 robust 还原解析，并转换为 Clash YAML proxy 列表，同时解决同名节点合并时的命名碰撞，并在前端修剪还原显示。

---

### 🚨 **BUG-028** 开机自动启动订阅自动更新重试与默认更新周期
* **目标版本**：`v1.1.4`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 客户端在开机自动启动后，因为网络尚未就绪，导致自动更新订阅失败，且之后不会再尝试自动更新，代理功能瘫痪。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`timer.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/timer.rs) ( 定时器任务模块)
  * **设计要点**：
    1. 开机自启动客户端时，如果因为网络尚未加载成功而导致第一次订阅更新刷新失败，不应跳过更新，而应延迟一定时间后进行重试，直至网络通畅并成功刷新。
    2. 当本地订阅配置文件中缺失 `update_interval` 时，定时器自动兜底采用 1440 分钟（24 小时）默认更新间隔以注册和运行任务。

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

所有已通过 Master 验证并确认关闭 of Bug，在此进行极简化表格索引。

| Bug 编号 | 缺陷描述与现象 | 解决版本 | 目前状态 |
| :--- | :--- | :---: | :--- |

| **BUG-055** | 托盘 `proxy_cache` 锁生命周期过长被拦截及 Git 构建失败。 | v3.0.6 | 代码已修正，已确认 |
| **BUG-050** | CPU 资源消耗显著高于原版（常驻组件后台渲染耗能，Traffic可见性响应缺失）。 | v3.0.3 | 代码已修正，已确认 |
| **BUG-048** | 无边框模式下窗口边缘磁吸及自适应吸附与脱离功能丢失。 | v3.0.1 | 代码已修正，已确认 |
| **BUG-047** | 本地 pre-push 钩子运行 cargo clippy 遇到测试模块中的 `.unwrap()` 报错拦截推送。 | v3.0.1 | 代码已修正，已确认 |
| **BUG-046** | 隐身模式下，单击窗口非避让区域无法正常恢复原生标题栏（点击唤醒失效）。 | v3.0.0 | 代码已修正，已确认 |
| **BUG-045** | 最窄窗口下，底部的流量图和数据卡片不够紧凑，且 Canvas 折线图缩放模糊。 | v2.0.7 | 代码已修正，已确认 |
| **BUG-044** | 本地导入节点卡片高亮激活逻辑未对齐，第二行元数据格式不统一。 | v2.0.7 | 代码已修正，已确认 |
| **BUG-041** | 启动或加载页面时，主页节点的表格（ProxyGroups）空白，需手动拉伸窗口方显现。 | v2.0.4 | 代码已修正，已确认 |
| **BUG-039** | 出口节点延迟变慢/超时，无法自动连切，且背景测速间隔不够自适应。 | v2.0.0 | 代码已修正，已确认 |
| **BUG-040** | 发行预检时强杀冲突进程，导致 host 上的原版客户端被误杀。 | v2.0.0 | 代码已修正，已确认 |
| **BUG-038** | 设置页展开后遮挡了右上角关闭按钮，导致无法关闭设置窗口而被锁死（v1.1.9 修复）。 | v1.1.9 | 代码已修正，已确认 |
| **BUG-036** | 设置页展开后遮挡了右上角关闭按钮，导致无法关闭设置窗口而被锁死（v1.1.6 修复）。 | v1.1.6 | 代码已修正，已确认 |
| **BUG-037** | 窗口始终置顶功能移除设置页开关，且设置页展开时隐藏顶部栏置顶图钉。 | v1.1.7 | 代码已修正，已确认 |
| **BUG-035** | 置顶图钉引入后，活跃出口状态条发生遮挡，且无法自适应定位和右对齐。 | v1.1.4 | 代码已修正，已确认 |
| **BUG-034** | 激活/切换订阅时无法自动触发测速、排序并选中最快可用节点。 | v2.0.1 | 代码已修正，已确认 |
| **BUG-033** | 导入 3 个或更多订阅时，侧边栏订阅列表没有滚动条。 | v1.1.4 | 代码已修正，已确认 |
| **BUG-032** | 流量接管模式切换时滑块先跳回“手动”再到目标模式，产生视觉闪烁与提权打扰。 | v1.1.4 | 代码已修正，已确认 |
| **BUG-031** | 开启系统代理时修改 Mixed Port，保存后滑块异常跳回手动，且系统代理配置未同步。 | v1.1.4 | 代码已修正，已确认 |
| **BUG-030** | 开机启动若默认是系统代理，滑块 UI 依然异常显示为“手动模式”。 | v1.1.4 | 代码已修正，已确认 |
| **BUG-029** | 当前活跃出口节点状态栏排列混乱，缩小时文本易错位折行。 | v1.1.4 | 代码已修正，已确认 |
| **BUG-027** | 路径控制下右键选择设为全局直连或代理分流，无实际规则写入和分流改变。 | v1.1.4 | 代码已修正，已确认 |
| **BUG-017** | 置顶当前活跃出口节点状态条无法正确获取并渲染节点的协议与 IP:port。 | v1.1.7 | 代码已修正，已确认 |
| **BUG-024** | Windows 标题栏数学粗体字符在部分 Windows 系统下显示为方块或乱码。 | v1.1.2 | 代码已修正，已确认 |
| **BUG-025** | 滑块选项文本只有在被选中时才是粗体，与普通按钮常驻粗体不统一。 | v1.1.2 | 代码已修正，已确认 |
| **BUG-026** | 界面控件（主按钮、双滑块、三选一滑块）尺寸显得过于偏大。 | v1.1.2 | 代码已修正，已确认 |
| **BUG-018** | 底部指标卡片整体偏大，且在折行及小屏状态下字体比例失调。 | v1.1.2 | 代码已修正，已确认 |
| **BUG-016** | 代理模式在非规则模式下隐藏表头导致定位、延迟排序按钮丢失，本地联网死锁。 | v1.1.1 | 代码已修正，已确认 |
| **BUG-015** | 配置文件激活时与未缓存的 activateSelected 发生 useEffect 联动，导致无限循环更新。 | v1.0.9 | 代码已修正，已确认 |
| **BUG-014** | 在全局代理（Global）模式下，用户选择节点无法控制流量出口。 | v1.0.4 | 代码已修正，已确认 |
| **BUG-002** | 默认开启 IPv6 且缺乏界面控制，可能导致某些网络环境下 DNS 泄露或代理分流异常。 | v1.0.3 | 代码已修正，已确认 |
| **BUG-003** | DNS 覆写默认未开启，导致国内直连 and 防泄露体验不佳。 | v1.0.3 | 代码已修正，已确认 |
| **BUG-011** | 默认未开启代理守护（Proxy Guard）导致系统代理易被篡改或静默失效。 | v1.0.3 | 代码已修正，已确认 |
| **BUG-012** | 默认未开启自动轻量化模式（Memory Optimization）导致后台挂机占用内存较高。 | v1.0.3 | 代码已修正，已确认 |
| **BUG-013** | 自动检查更新选项开启（auto_check_update）引发的冗余后台行为。 | v1.0.3 | 代码已修正，已确认 |
| **BUG-001** | 开机自启非管理员权限下修改后台静默失败且前端 Switch 状态脱节。 | v1.0.2 | 代码已修正，已确认 |
| **BUG-004** | 代理模式在非规则模式下隐藏表头导致定位、延迟排序按钮丢失。 | v0.2.0 | 代码已修正，已确认 |
| **BUG-005** | 表格最外侧左、右竖边框线非 5px double 且左侧表格线缩进空缺十几个像素。 | v0.2.0 | 代码已修正，已确认 |
| **BUG-006** | 界面清除毛玻璃不彻底，部分 Aero 样式变量残留导致滚动漏光重叠。 | v0.2.0 | 代码已修正，已确认 |
| **BUG-007** | 订阅管理中偶现没有名字的空行订阅项，重启后仍反复出现。 | v0.2.0 | 代码已修正，已确认 |
| **BUG-008** | 设置页面右上角设置齿轮按钮与底部的节点名称/延迟重合且无法点击。 | v0.2.0 | 代码已修正，已确认 |
| **BUG-009** | 启动时提示有 Clash Verge 新版本并引导升级从而覆盖客户端的问题。 | v0.0.1 | 代码已修正，已确认 |
| **BUG-010** | 便携版绿色压缩包在 Release 时丢失或包含旧版本二进制的问题。 | v0.3.2 | 代码已修正，已确认 |

---

## 📌 待优化与设计微调 (UI/UX Enhancements)

| 优化编号 | 改进项与现象描述 | 目标版本 | 目前状态与设计方案 |
| :--- | :--- | :---: | :--- |
