# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> > **🤖 AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES FOR AI AGENTS)**:
> > 
> > 本文件使用 **「结构化卡片 + 极简索引表」** 的双轨设计。所有参与本项目的 AI 协同助理在读取 and 更新本文件时，必须严格遵守以下书写与维护规范，严禁格式退化：
> > 
> > 1. **严禁在表格中堆叠排查细节**：只有已被 Master 验证确认关闭的 Bug 才能放入「已解决的历史 Bug 索引」表格中，且状态统一写为 `代码已修正，已确认`。表格中禁止包含任何 `设计要求`、`代码状态` 等冗余的后台开发过程细节。
> > 2. **待验证/排查中 Bug 强制卡片化**：任何状态为 `排查中` 或 `代码已修正，待确认` 的 Bug，必须在顶部的「待验证与活动中 Bug 详情」区以独立标题 and 结构化字段登记。
> > 3. **强制记录排查记忆，防止重复劳动**：在卡片中，必须详实搜集、继承并持久化记录以下三个协同要素：





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

### 🚨 **BUG-043** 全自动订阅链接格式智能识别与并联订阅合并
* **目标版本**：`v2.0.6`
* **当前状态**：`代码已修正，待确认`
* **【现象与复现路径】**：
  * 缺乏全自动链接格式识别（YAML与Base64）与默认并联订阅合并。
* **【当前修正方案 & 设计要求】**：
  * **修改文件**：
    * [**`prfitem.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/profile/prfitem.rs) ( 后端订阅处理)
    * [**`profiles.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/profiles.rs) ( 导入/追加订阅去重处理)
    * [**`enhance/mod.rs`**](file:///C:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)
  * **设计要点**：
    1. 从 UI 移除兼容模式开关，将输入框及按钮文字恢复为标准文案；
    2. 后端 `prfitem.rs` 移除开关配置判断，YAML 校验失败时直接尝试 Base64/URI 解析并对垃圾数据输出自定义中文报错；
    3. `enhance/mod.rs` 中默认开启并联订阅合并编译。
    4. **【去重与复用】**：机场订阅链接去重与复用。在 `append_item` 时自动扫描并匹配已有的远程订阅 URL。若发现相同的远程订阅已存在，则自动复用原订阅的 `uid` 与 `file` 文件名，将新增操作转换为就地刷新更新，彻底防备同一机场订阅链接重复导入生成多个配置文件的问题。

---


## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

所有已通过 Master 验证并确认关闭 of Bug，在此进行极简化表格索引。

| Bug 编号 | 缺陷描述与现象 | 解决版本 | 目前状态 |
| :--- | :--- | :---: | :--- |
| **BUG-042** | 支持本地通用非 YAML 订阅链接解析（支持 VMess, SS, Trojan, VLESS, Hysteria2 及 HTTP 格式）。 | v2.0.4-full | 代码已修正，已确认 |
| **BUG-028** | 开机启动时因网络未就绪自动更新失败后增加重试机制，且缺省更新间隔兜底为 24 小时。 | v1.1.4 | 代码已修正，已确认 |

| **BUG-057** | 导入新订阅后节点列表加载显示空白。 | v4.1.0 | 代码已修正，已确认 |
| **BUG-053** | 首次导入或更新订阅自动切换激活及防空值误报。 | v3.0.4 | 代码已修正，已确认 |
| **BUG-051** | 流量小卡片上传下载总量数值常驻为 0。 | v3.0.4 | 代码已修正，已确认 |
| **BUG-049** | 活性监控探活定时器在 proxies 变动时反复重置。 | v3.0.2 | 代码已修正，已确认 |
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
