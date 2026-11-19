# Clash Mini 开发设计书

> **适用版本**：v3.5.6 及后续版本  
> **文档性质**：本文档是 Clash Mini 项目的**现行权威系统设计规范与工程开发宪章**，聚焦**当前系统做什么、为什么、架构边界、关键参数与不变式**（历史变更与缺陷演进记录见 [Changelog.md](Changelog.md)、[bug_list.md](bug_list.md) 及 Git 提交历史；代码标识映射见 [docs/implementation-notes.md](docs/implementation-notes.md)，架构总览见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)，Agent 操作避坑规范见 [clash_mini_pitfalls.md](clash_mini_pitfalls.md)）。

---

## 目录

- [🏗️ 一、项目定位与架构基石](#一项目定位与架构基石)
  - [1.1 核心定位与多实例隔离边界](#11-核心定位与多实例隔离边界)
  - [1.2 Windows x64 单一平台架构收敛](#12-windows-x64-单一平台架构收敛)
  - [1.3 系统服务模式与 TUN 模式隔离原则](#13-系统服务模式与-tun-模式隔离原则)
  - [1.4 进程管理与孤儿清理机制](#14-进程管理与孤儿清理机制)
  - [1.5 多语言支持与「英文保留原则」](#15-多语言支持与英文保留原则)
  - [1.6 运行内存基准与资源配额](#16-运行内存基准与资源配额)
  - [1.7 版权规范与合规署名](#17-版权规范与合规署名)
- [🎨 二、界面美学与交互体系](#二界面美学与交互体系)
  - [2.1 3D 实体拟物美学准则](#21-3d-实体拟物美学准则)
  - [2.2 遮挡式布局与响应式视口框架](#22-遮挡式布局与响应式视口框架)
  - [2.3 窗口尺寸常量与边界约束](#23-窗口尺寸常量与边界约束)
  - [2.4 Win32 消息子类化与 WM_SIZING 全向钳制钩子](#24-win32-消息子类化与-wm_sizing-全向钳制钩子)
  - [2.5 无死锁原子 Win32 指针缩放与几何同步持久化](#25-无死锁原子-win32-指针缩放与几何同步持久化)
  - [2.6 极窄隐匿模式（自动隐藏标题栏规范）](#26-极窄隐匿模式自动隐藏标题栏规范)
  - [2.7 6 套特色皮肤与双控制滑块体系](#27-6-套特色皮肤与双控制滑块体系)
  - [2.8 主页节点列表与活跃节点轮换](#28-主页节点列表与活跃节点轮换)
  - [2.9 极简静态菜单系统托盘规范](#29-极简静态菜单系统托盘规范)
  - [2.10 设置开关的乐观更新与权威数据源](#210-设置开关的乐观更新与权威数据源)
    - [2.10.1 开机自启：注册表方案与启动对账](#2101-开机自启注册表方案与启动对账)
  - [2.11 错误边界分级与局部降级](#211-错误边界分级与局部降级)
  - [2.12 首次使用引导、网络就绪反馈与防误杀入口](#212-首次使用引导网络就绪反馈与防误杀入口)
- [⚙️ 三、内核生态与运行时管控](#三内核生态与运行时管控)
  - [3.1 内核选择与安装包体积极致精简](#31-内核选择与安装包体积极致精简)
  - [3.2 Windows 命名管道与高并发退避重试](#32-windows-命名管道与高并发退避重试)
  - [3.3 端口冲突自适应探测与私有密钥保障](#33-端口冲突自适应探测与私有密钥保障)
  - [3.4 Geodata 与 GFWList 资源随包策略与按需加载](#34-geodata-与-gfwlist-资源随包策略与按需加载)
  - [3.5 Go Runtime 运行参数深度压制](#35-go-runtime-运行参数深度压制)
  - [3.6 内核在线平滑升级与多层代理降级链](#36-内核在线平滑升级与多层代理降级链)
  - [3.7 TUN 模式下 IPv6 流量全量接管（防泄漏铁律）](#37-tun-模式下-ipv6-流量全量接管防泄漏铁律)
- [💤 四、轻量级模式与窗口生命周期](#四轻量级模式与窗口生命周期)
  - [4.1 窗口关闭即入轻量模式](#41-窗口关闭即入轻量模式)
  - [4.2 不断网不断流铁律与后端连接维持](#42-不断网不断流铁律与后端连接维持)
  - [4.3 前端 WebSocket 熔断与自动重连自愈](#43-前端-websocket-熔断与自动重连自愈)
  - [4.4 唤醒禁止配置重置（跳过 force reload）](#44-唤醒禁止配置重置跳过-force-reload)
  - [4.5 窗口唤醒几何恢复与首帧轻量刷新](#45-窗口唤醒几何恢复与首帧轻量刷新)
- [📡 五、后台监控、健康探针与自愈体系](#五后台监控健康探针与自愈体系)
  - [🎯 本章最高方针 · 连续性优先](#-本章最高方针--连续性优先)
  - [5.1 恒定 15 秒健康监测与防抖自愈机制](#51-恒定-15-秒健康监测与防抖自愈机制)
  - [5.2 单一内核测量组（PROXY__METRICS）统一调度](#52-单一内核测量组proxy__metrics统一调度)
  - [5.3 统一记账、切换速率上限与 25 秒失败冷却保护](#53-统一记账切换速率上限与-25-秒失败冷却保护)
  - [5.4 订阅伪节点全链路拦截防线与节点过滤同源契约](#54-订阅伪节点全链路拦截防线与节点过滤同源契约)
  - [5.5 物理网络连通性快速探测与活跃流量免测（TCP:53）](#55-物理网络连通性快速探测与活跃流量免测tcp53)
  - [5.6 批量测速触发（F4/F1）与结果统一回传](#56-批量测速触发f4f1与结果统一回传)
  - [5.7 无人值守资源节约与静默门控](#57-无人值守资源节约与静默门控)
  - [5.8 测速失真复核与失败服务器避让](#58-测速失真复核与失败服务器避让)
- [🛡️ 六、安全防御与编译加固](#六安全防御与编译加固)
  - [6.1 Tauri 权限最小化与沙箱加固](#61-tauri-权限最小化与沙箱加固)
  - [6.2 用户输入 URL 严格校验与 SSRF 防护](#62-用户输入-url-严格校验与-ssrf-防护)
  - [6.3 安全解压机制（单文件直写暂存）](#63-安全解压机制单文件直写暂存)
  - [6.4 配置增强极简化与原生 Rust 卫兵](#64-配置增强极简化与原生-rust-卫兵)
  - [6.5 编译期加固、死代码门禁与源码保护](#65-编译期加固死代码门禁与源码保护)
- [📊 七、数据流中枢与协议解析](#七数据流中枢与协议解析)
  - [7.1 单一 PROXY 组模型架构](#71-单一-proxy-组模型架构)
  - [7.2 通用协议解析器（Universal Parser）规范](#72-通用协议解析器universal-parser规范)
  - [7.3 订阅导入等待与 Provider 刷新协同](#73-订阅导入等待与-provider-刷新协同)
  - [7.4 订阅管理与右键上下文菜单](#74-订阅管理与右键上下文菜单)
  - [7.5 Mihomo 插件 API 与序列化小写补丁](#75-mihomo-插件-api-与序列化小写补丁)
  - [7.6 远程订阅请求 User-Agent 版本解耦与兼容铁律](#76-远程订阅请求-user-agent-版本解耦与兼容铁律)
  - [7.7 路径控制精准规则生成与四态路由可逆闭环](#77-路径控制精准规则生成与四态路由可逆闭环)
- [🔧 八、工程发布体系与规范](#八工程发布体系与规范)
  - [8.1 Windows 测试清单与单测执行隔离](#81-windows-测试清单与单测执行隔离)
  - [8.2 静默发布全流程（Silent Release）](#82-静默发布全流程silent-release)
  - [8.3 令牌安全与代理环境防护红线](#83-令牌安全与代理环境防护红线)
  - [8.4 CI 四阶段死锁无关流水线架构](#84-ci-四阶段死锁无关流水线架构)
  - [8.5 本地制品验收与自动更新同步闭环](#85-本地制品验收与自动更新同步闭环)
  - [8.6 配置文件版本号与迁移框架](#86-配置文件版本号与迁移框架)
  - [8.7 可诊断性：日志保留与诊断包](#87-可诊断性日志保留与诊断包)
  - [8.8 前端单测基线与最小覆盖](#88-前端单测基线与最小覆盖)
  - [8.9 用户可见文案 i18n 硬约束](#89-用户可见文案-i18n-硬约束)
  - [8.10 安装程序全周期全局置顶铁律](#810-安装程序全周期全局置顶铁律)
- [附录：已知非缺陷（NOT A BUG，勿重复排查）](#附录已知非缺陷not-a-bug勿重复排查)

---

## 🏗️ 一、项目定位与架构基石

### 1.1 核心定位与多实例隔离边界

Clash Mini 是一款聚焦轻量、低内存、高连续性与 3D 拟物美学的 Windows 代理客户端，并在命名空间、端口、进程及存储上实施彻底隔离：

1. **软件名称**：窗口标题、对话框、托盘提示、安装包与可执行程序名称统一为 **`Clash Mini`**。
2. **本地数据隔离**：使用独立的应用数据目录，订阅、规则库与运行时数据不与其他 Clash 客户端混用。
3. **默认端口与凭据**：
   - 默认代理端口固定为 Mixed **`10801`**、SOCKS **`10802`**、HTTP **`10803`**。
   - 内核 Controller API 默认端口为 **`9098`**，默认控制密钥为 **`adapted-by-qiu-yuxiao`**（见 `src-tauri/src/constants.rs` 与 `src-tauri/src/config/clash.rs`）。
   - 单实例检测通信端口在 Release 模式为 **`33335`**，Dev 模式为 **`33336`**。
4. **单实例静默唤醒与命名管道隔离**：`server.rs::embed_server`（`127.0.0.1:33335`）常驻于独立守护 Tokio 运行时 `DAEMON_RUNTIME`，第二实例探测超时 **`1500ms`**；若端口已占用，仅向既有实例发送 `/commands/visible`（或携带标准 URL 查询百分号编码参数的 `clash-mini:` / `clash-verge:` 深链）并静默退出。当 `should_enter_silent_tray_on_launch()`（开启「静默启动」且存在真实订阅）为真时，既有实例保持托盘驻留且不弹窗，按需弹出 Win32 托盘气泡确认运行状态；仅当连接被拒绝或彻底超时时才显示单实例报错。

### 1.2 Windows x64 单一平台架构收敛

- **唯一支持平台**：**Windows 10 x64 及以上桌面操作系统**（`x86_64-pc-windows-msvc`）。
- **单平台纯化**：Rust 与 `Cargo.toml` 不保留 macOS/Linux 条件分支或 `objc2`/`libc`/`nix` 等非 Windows 依赖；CI 仅构建 `windows-latest`；前端 `getSystem()` 恒定返回 `'windows'`，仅渲染 Windows 原生窗口控制栏与样式。

### 1.3 系统服务模式与 TUN 模式隔离原则

为避免多客户端争夺 Windows SCM 导致断流，确立 **TUN-Service 严格隔离规范**：
- **服务启动门控**：仅在开启 TUN 模式时启用/接管 Windows 系统服务；非 TUN 模式（系统代理或混合端口）一律强制以本地独立子进程（Sidecar 旁路模式）运行内核，零 SCM 调用。
- **管理员直通与普通用户服务自举**：
  - 进程具备 Administrator 权限（`is_current_app_handle_admin`）时直接跳过服务等待并放行 TUN。
  - 普通用户开启 TUN 前由前端 `isTunModeAvailable` 检查权限或服务状态（`is_service_available`）；未就绪时通过 `installServiceAndSync()` 执行 `install_service`（UAC 提权安装，安装/启动就绪后若当前处于 Sidecar 模式立即停止服务自启残留的 Session 0 `mini-mihomo` 进程）并刷新系统状态，再由 `patchVerge` 一次性原子完成 `Sidecar -> Service` 切换，杜绝多连重启风暴与管理员管道抢占。
  - **命名管道跨运行时池隔离与零句柄泄漏**：`crates/tauri-plugin-mihomo` 的 `IpcConnectionPool` 按 `tokio::runtime::Id` 物理分桶隔离（`DAEMON_RUNTIME` 与 `tauri::async_runtime` 严禁跨 IOCP 借用 `NamedPipeClient`），`conn_driver` 绑定当前运行时的 `tokio::spawn` 并存入 `IpcConnection::drop` 自动 `abort()`，超时/报错/清池均 `0ms` 释放底层句柄；`reload_config` 实施前后双向清池，`disconnect` 在 `LocalSocket` 或 `force_timeout=0` 下直接丢弃读写端，`stop_core_inner` 停核前调用 `clear_all_ws_connections()` 同步释放常驻 WebSocket 与 HTTP 管道句柄。
  - 后端 `ServiceManager` 使用 `tokio::sync::Mutex<()>` 串行化状态刷新与启停，基于 `ServiceStatus`（`Ready` / `StartRequired` / `InstallRequired` / `ReinstallRequired` / `Unavailable(String)`）管理状态迁移；服务安装/启动就绪等待预算为 `SERVICE_WAIT_MAX = 30000ms`（轮询间隔 `200ms`），而 `ServiceManager::config().max_retries` 严格限定为单次 IPC 请求重试次数，二者职责不得混淆。
  - 若 `prepare_startup_mode` 发现服务被禁用（`DisabledOrBroken`）或二进制损坏，立即记录告警并持久回退至系统代理；若已启用服务在冷启动时瞬时超时，调用 `IVerge::enter_temp_tun_fallback` 进入会话级临时回退并派发 `Handle::refresh_verge()` 同步 UI，绝不覆写磁盘 `verge.yaml` 中的原始 TUN 意图。

### 1.4 进程管理与孤儿清理机制

- **独占进程前缀**：内核与核心辅助进程统一使用 **`mini-`** 前缀（如 `mini-mihomo.exe`）。
- **退出强杀与退出等待**：停止内核时记录 PID 并以 `50ms` 步进、上限 `2000ms` 轮询确认退出；退出应用流程（`clean_async` 及 panic 清理路径）在服务/TUN 模式下优先调用 `service::stop_core_by_service()` 停止 Session 0 内核，必要时回退 `CoreManager::stop_core()`，并无条件执行 `kill_all_mini_cores()` 清理用户态残留 `mini-` 进程。

### 1.5 多语言支持与「英文保留原则」

支持全部 13 种语言并执行以下保留规则：
1. **专业协议术语保持英文**：`Mixed Port`、`Allow LAN`、`TUN Mode`、`System Proxy`、`Direct`、`Rule`、`Global` 等在全语种下不意译。
2. **拟物滑块术语保持英文**：`Depth`、`Vibrancy`、`Radius`、`Accent`、`Glow`、`Contrast` 等保留英文原词。
3. **窄屏分段按钮 3 字母缩写**：西文语种分段控制器统一使用 `MAN`/`SYS`/`TUN`、`DIR`/`RUL`/`PRX`、`SYS`/`LIGHT`/`DARK`，通过悬浮 `Tooltip` 展示完整名称。

### 1.6 运行内存基准与资源配额

- **主窗常驻内存**：WebView2 开启态物理内存基线约 **35MB ~ 45MB**；禁止调用 Win32 `SetProcessWorkingSetSize` 强行修剪工作集（避免引发页面错误死锁）。
- **轻量模式释放**：关闭主窗并销毁 WebView2 后，外壳进程内存回落至 **~10MB**。
- **内核软堆上限**：Go runtime 通过 `GOMEMLIMIT`（默认 `128MiB`）约束内存占用。

### 1.7 版权规范与合规署名

- 遵循 **GPL-3.0-only** 开源协议，保留上游贡献者署名。
- 二次开发与重构界面版权统一展示：`© 2026 秋雨潇潇 (修改部分)`。

---

## 🎨 二、界面美学与交互体系

### 2.1 3D 实体拟物美学准则

- **100% 不透明实体渲染**：禁用整窗半透明与 Aero/Mica 模糊滤镜，主窗、弹窗、菜单及面板一律纯色高对比渲染。
- **标准实体线框**：列表表格使用 `2px solid` 凹槽网格线，模块间缝使用 `5px double` 双线，设置抽屉使用 `4px double` 双实线。

### 2.2 遮挡式布局与响应式视口框架

- **「遮挡式取景框」原则**：窗口缩放时不挤压、不折行缩放内部控件，而是扩大或裁切可视内容区。
- **285px 内容宽度断点与面板生命周期**：
  - 宽窗（内容宽 `>285px`）：节点列表呈 3 列卡片网格、顶部活跃节点栏展示 4 项指标；仅当侧边设置抽屉同步展开时挂载右侧 `ConnectionsPanel`（内聚 `useConnectionData` / `usePathControl` / `<ConnectionDetail />`），并在收起抽屉或缩回窄窗（`≤285px`）后完全卸载并通过 React Query `gcTime`（`30s`）回收缓存。
  - 窄窗（`≤285px`）：节点列表切换为 1 列卡片、顶部展示 2 项核心指标。`285px` 断点严格大于最小窗宽 `271px`，确保缩至最窄时进入单列排版。
  - 打开设置抽屉期间同步卸载被遮挡的底层 `ProxyGroups`（关闭后结合 `localStorage` 与 `useLayoutEffect` 原位恢复）；设置关闭按钮层级锁定 `SETTINGS_CLOSE_BUTTON_Z_INDEX = 250`（带 `data-close-settings="true"`）；`AddUrlDialog` 独立置于外层弹窗区并配合失焦端口保存守卫 `shouldSkipMixedPortBlurSave`。
- **271px 窄窗左侧锚定弹窗与通知规范**：
  - 所有模态弹窗（`AddUrlDialog`、`FirstRunGuideDialog`、`HelpMenuButton` 菜单、`LayoutDialogs`）与顶部通知（`NoticeManager`）宽度锁定为 `MIN_WINDOW_WIDTH`（`271px`）并锚定于左侧 `left: 0`，顶部经 `resolveNarrowOverlayTopOffset` 避让 `CUSTOM_TITLEBAR_HEIGHT`（`30px`；极窄隐匿模式隐藏标题栏时贴顶 `0px`），宽窗下绝不居中漂移；`NoticeManager` 层级设定 `zIndex = 2200 > 1500`（带 `px: 6px, pt: 4px`）避免被 Tooltip 遮挡。
  - 窄窗弹窗、`BaseStyledSelect`、全局/输入框右键菜单以及 `ConnectionDetail`/`ConnectionTable` 右键菜单统一复用 [popup-styles.ts](src/utils/popup-styles.ts)（`getPopupCardStyle` / `getPopupSkinTokens`）。
  - 系统运行日志弹窗严格按 `271px` 排版（固定 `95px` 级别下拉框、`overflowWrap: 'anywhere'`、底部吸底仅在贴底时采用 `behavior: 'auto'` 跟随），无横向滚动条。

### 2.3 窗口尺寸常量与边界约束

- **尺寸边界（逻辑像素）**：默认启动 **`271 × 680`**、最大允许 **`640 × 860`**、极限最小 **`271 × 126`**（`MINIMAL_WIDTH × MINIMAL_HEIGHT`）。
- **单一尺寸口径铁律**：全工程窗口尺寸一律使用**内尺寸（客户区）**（`setSize` / `innerSize` / `min_inner_size` / `max_inner_size`），位置使用**外位置**（`setPosition` / `outerPosition`）；严禁将 `outerSize()` 混入 `setSize()` 计算。
- **必须关闭无框系统阴影（`shadow(false)`）**：确保高 DPI 下 `outer == inner` 零客户区内缩，边框与光影由自绘皮肤背景承担。

### 2.4 Win32 消息子类化与 WM_SIZING 全向钳制钩子

- **原子主线程建窗与 `SetWindowSubclass`**：主窗口创建、去标题栏（`strip_caption_style_hwnd`）、外框对齐（`force_set_window_outer_size_hwnd`）与 `SetWindowSubclass` 全部收敛于 `run_on_main_thread`（带 `12s` 超时回滚 `cleanup_timed_out_main_window`），同步写入原子全局句柄 `MAIN_WINDOW_HWND` 并在 `WM_NCDESTROY` / `destroy_main_window` 时自动卸载子类化与清零句柄，确保轻量模式反复重建窗口零漏挂、零死锁。
- **全向宽高钳制**：`WM_ENTERSIZEMOVE` 置位 `NATIVE_WINDOW_RESIZING`、`WM_EXITSIZEMOVE` 复位并落盘；`WM_SIZING` 结合 `GetDpiForWindow` 对 `WMSZ_*` 八方向统一钳制在 `[271, 640] × [126, 860]` 并锁定非拖拽对边坐标（防边界回弹及 Y 轴漂移）。

### 2.5 无死锁原子 Win32 指针缩放与几何同步持久化

- **禁用系统模态拖拽 API**：禁止在网页端调用易卡死 UI 线程的 `startResizeDragging`。
- **8 方向自定义手柄 + OS 原子缩放**：边缘（`10px`）与四角（`20px`）句柄调用 `begin_window_resize(direction, seq)`、`step_window_resize(seq)`、`end_window_resize(seq)`，以单调 `gestureSeq` 隔离跨手势并发并在 `pointerup`/`pointercancel`/`onLostPointerCapture`/(buttons & 1 === 0) 及卸载时兜底收尾；外围 `~4px` 仍由 OS `TAURI_DRAG_RESIZE_BORDERS` 经 `WM_SIZING` 处理，双路同界。
- **防震荡与零撕裂原则**：`setPointerCapture` 必须在任意 `await` 之前同步调用；后端采集不可变起始矩形、`GetCursorPos` 与 `GetDpiForWindow`，经去重合并后在主线程执行单次 `SetWindowPos(..., SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOCOPYBITS)`，未拖拽维度保持原值不二次 clamp，西/北拖拽由不可变右/底边反推。
- **拖拽期冻结断点重排与有序落盘**：`NATIVE_WINDOW_RESIZING` / `isWindowResizing()` 激活期间，`lib.rs` 跳过逐像素 `Resized`/`Moved` 落盘、`WindowProvider` 暂停途中翻转 `isMinimalWidth`/`isMiniStatus`，直到松手触发 `RESIZE_ENDED_EVENT` 一次性对齐布局；非拖拽期通过 `1000ms` 节流 + 版本号校验 + `1px` 容差 + `>= MINIMAL` 守卫写入 `window_state.json`。

### 2.6 极窄隐匿模式（自动隐藏标题栏规范）

- **激活条件**：窄窗区间（内容宽 `≤285px`）且极简高度（`≤126px`），用户连续 **10 秒**无交互自动隐藏标题栏及外框装饰，保持极简纯背景。
- **物理像素域唯一判定**：必须直接比较 `onResized` / `innerSize()` 的内尺寸物理像素：`物理尺寸 ≤ 阈值 × scaleFactor + 1px`，禁止使用取整后的 `window.innerWidth/innerHeight`；极窄与极小状态由单一推导函数写入 state 与 ref。
- **无条件自续期**：10 秒空闲计时器在每次到期时无条件重排调度，交互、`onResized`（含拖拽防误触 `mousedown` 保护）与退出隐匿均执行「清除后重排」。

### 2.7 6 套特色皮肤与双控制滑块体系

- **6 套拟物皮肤**：`Retro 3D (Trump-3D)`（默认）、`Original`、`Modern`、`Frosted Glass`、`Cyberpunk`、`Monochrome`；各皮肤匹配独立中英字体组合与独立 `localStorage` 双参数（60fps CSS 变量预览 + 200ms 防抖/失焦/`pagehide` 落盘）。
- **直角工业小开关**：`Cyberpunk` 与 `Monochrome` 皮肤下的 `Switch`（[base-switch.tsx](src/components/base/base-switch.tsx)，`buildSquareSwitchStyle`）统一采用 `0px` 直角方形几何（常规 `28×14px`、`size="small"` `18×10px`，滑块 `12×12` / `8×8` 且 `borderRadius: '0px'`）。
- **三态分段控件对齐**：`ThemeSettingsCard`（`系统 / 浅色 / 深色`）与 `BasicSettingsCard`（`TUN 域名嗅探`）统一 `118 × 20px` 外框、`11px` 粗体、`marginInlineEnd: 4px` 右边界对齐，并带 `textOverflow: 'ellipsis'` 与 `Tooltip`。

### 2.8 主页节点列表与活跃节点轮换

- **网格与 0ms 切换**：默认 3 列网格（行高 `20px`），窄窗单列；卡片点击瞬间对 `['getProxies']` 执行 0ms 乐观更新并异步提交（失败回滚）同时按 §4.2 清理旧连接；卡片右侧延迟徽章设 `e.stopPropagation()` 独立点击热区，仅执行单节点实时测速（`select: false`）。
- **顶栏轮换、纯测速与过滤切点边界**：
  - 点击顶栏活跃节点卡片按当前过滤/排序子集顺位轮换可用节点并持久化 `profile.selected`；全超时或 `network_health::offline` 警示胶囊仅在离线时以 `select: false` 重测刷新延迟，在线态支持优选自愈。
  - 列表顶部闪电图标 `⚡` 与 `F4` 经共享函数 `resolveBatchCheckIntent('bolt' | 'f4')` 收敛为**纯延迟查看（`select: false, cleanStaleConnections: false`）**，仅刷新可见延迟，不切换节点、不切断任何连接。
  - 前台 4 类主动切点入口（列表点选、顶栏胶囊轮换、过滤词自动选点/`F1` 全量优选、在线警示胶囊重测）切入新节点后按 §4.2 精确清理停留在非目标节点上的 `PROXY` 存量长连接。
  - 过滤输入框（`700ms` 防抖/`Enter`）与 `F1` 执行 `select: true` 地域策略切换：`BaseSearchBox` 在 IME 组词期维持本地缓冲避免拼音中间态外发；`ProxyHead` 经 `isIncompleteFilterInput` 拦截悬空操作符前缀（`-`/`!`/`|` 等）；选点前调用 `flushHeadStatePersist()`；`ProxyGroups::handleCheckAll` 追踪 `activeTestSelectRef`/`pendingRecheckSelectRef`/`pendingRecheckGroupsRef` 与有限次 `AUTO_SELECT_BUSY` 重试，确保在途纯测速不吞掉后续过滤切点意图；零匹配安全短路，清空过滤框仅重置列表不触发全量测速。

### 2.9 极简静态菜单系统托盘规范

- **固定静态双项菜单**：右键仅固定包含①「轻量模式 / Lite mode」（通过 `item.set_text` 同步 `"✔ "` 勾选前缀）②「退出 (Exit)」，运行期严禁销毁重建菜单，规避高权限 Windows Shell `E_FAIL`。
- **左键双向切换与静态三图标**：`Tray::update_icon` 仅在代理模式变更时于三枚静态 PNG（`tray-icon-tun.png` / `tray-icon-sys.png` / `tray-icon.png`）间切换；左键结合 `MAIN_WINDOW_FOCUSED` 原子标志执行显隐切换；仅当静默启动启用期间收到手动冷启动/单实例激活时弹出 `2s` 防抖 Win32 `Shell_NotifyIconW(NIF_INFO)` 气泡，零后台轮询。

### 2.10 设置开关的乐观更新与权威数据源

- **数据源严格分离**：开关绑向持久化 `clashInfo.*`（`Config::clash().get_client_info()`），绝不绑定会被后端强制覆写（如 TUN 强开 `ipv6: true`）的内核运行时配置。
- **单一 `QueryClient` + 乐观锁闭环**：全应用唯一 `@/services/query-client` 单例；`patch_*` 先写缓存再请求，失败即时 `refetch` 回滚并配 `beginLongOperation`/`endLongOperation` 抑制 IPC 超时误报；`patchVerge` 使用 `useLockFn` 丢弃并发重入、返回 `boolean | undefined` 并支持 `{ silentError: true }` 去重报错；所有二/三选一开关及端口保存严格按「进行中（`MIN_LOADING_NOTICE_DWELL_MS = 320ms` via `markLoadingNotice`/`settleLoadingNotice`）→ 结果提示 → 单次报错」三段式呈现。

#### 2.10.1 开机自启：注册表方案与启动对账

- **注册表实现**：通过 `tauri-plugin-autostart` 写入 `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`（支持系统自带启动页管理），不再使用 Windows 计划任务。
- **启动无条件对账**：`resolve_setup_async` 在初始化配置后调用 `autostart::reconcile_on_startup()`，以 `verge.yaml` 为唯一真源对账重写（禁用前必须先检查 `is_enabled()` 避免误删报错）；UI 优先展示 `get_auto_launch_status` 真实注册表状态；首次启动静默执行 `cleanup_legacy_scheduled_tasks()` 并写入 `.autostart_task_migrated`（不提权不弹 UAC）。
- **静默托盘启动判定**：`lightweight::should_enter_silent_tray_on_launch()`（`enable_silent_start && has_real_profile_items()`）统一覆盖 `--silent` 开机自启、手动冷启动及 `/commands/visible` 重复唤起，仅首次无真实配置时开窗完成向导。

### 2.11 错误边界分级与局部降级

采用全局（`main.tsx`）、页面级（`BasePage`）与区域级（`AreaErrorFallback`）三级边界；重组件接入须挂区域边界且仅捕获渲染异常（异步错误由 `showNotice` + 日志自处理）。

### 2.12 首次使用引导、网络就绪反馈与防误杀入口

- **触发判定**：由 [profile-items.ts](src/utils/profile-items.ts) `hasRealProfileItem` 判定（严禁用 `items.length === 0`，因内置 `Merge` 项恒存在）；当 `localStorage.getItem('clash-mini-first-run-completed') !== 'true'` 且不存在真实 `remote`/`local` 配置时，延迟 `500ms` 居中（`271px` 全宽）弹出 `FirstRunGuideDialog`。
- **两步向导**：
  - **Step 1 / 2**：支持剪贴板静默嗅探、`http(s)://` 及 11 种单节点 URI 协议校验（`isSupportedSubscriptionOrNodeInput` / `SUPPORTED_NODE_URI_SCHEMES`）；`Dialog Paper` 与 `window` 级监听 `Enter` 导入（排除 IME 组词及按钮焦点）；`submitting` 期间禁用遮罩点击与 `Esc`。
  - **Step 2 / 2**：仅当活跃节点持有有效实测延迟（`0 < delay < NODE_DELAY_MAX_MS`）时显示「代理已就绪」（未测出时每 `1.5s` 刷新，超 `20s` 给出「🔄 重新测速并优选节点」按钮，严禁宣称 TUN 全就绪或伪造畅通）；仅首次真正向导导入成功且系统代理/TUN 均关闭时自动启用系统代理；底部展示金色 GitHub Star（`CLASH_MINI_GITHUB_URL`）与 Wiki 转化卡。「稍后手动配置」自动展开左侧抽屉。
- **常驻帮助菜单与纯文本防误杀指南**：顶部 `?` 帮助菜单（`HelpMenuButton`）在常规窗高与窄高（包括 `max-height: 830px`）均常驻可见，置顶加粗 `⭐ 去 GitHub 点亮 Star` 并提供 `🛡️ 轻量模式防误杀指南` 入口；点击后直接调用后端 `open_antivirus_guide` 使用系统默认文本查看器打开随包释放于安装/便携根目录及 `resources/` 的 **`轻量模式防误杀指南.txt`**（与 [docs/antivirus-whitelist.md](docs/antivirus-whitelist.md) 同源），不另弹 React 二级弹窗，完整说明 360 两套独立机制（`DoOptmGroupResidual` → `netmon\whiteinternal.ini`；`FastOptm`/`DoOptm` → `netmon\useroptm2.ini` 与 `netmon\useroptm_game2.ini` 中的 `check=0`）及「静默启动」减轻方案。所有文案全量覆盖 13 语种 i18n。

---

## ⚙️ 三、内核生态与运行时管控

### 3.1 内核选择与安装包体积极致精简

全应用仅集成单个官方稳定版内核（`mini-mihomo`），移除 Alpha 测试内核并关闭前端生产 Sourcemap，安装包体积极致收敛至 ~33.7MB。

### 3.2 Windows 命名管道与高并发退避重试

- **专用命名管道**：Rust 与内核的 HTTP/WS 通信 100% 经 Windows 命名管道 `\\.\pipe\mini-mihomo` 完成。
- **高并发弹性池化**：并发遇 `ERROR_PIPE_BUSY` 时使用指数退避（最大 `busy_retry_count = 24`）结合 Win32 `WaitNamedPipeW` 原生等待；池许可获取 `RejectPolicy::Timeout(2s)` 严格短于外层默认 `5s` 请求超时；对复用的半开失效连接，仅限幂等只读方法（`GET`/`HEAD`）自动驱逐重连重试 `1` 次。

### 3.3 端口冲突自适应探测与私有密钥保障

内核 Controller 默认监听 `127.0.0.1:9098`（密钥 `adapted-by-qiu-yuxiao`）：若端口被占用则先带私钥探测，鉴权通过直接平滑复用，被第三方占用则自动扫描顺位空闲端口动态迁移绑定。

### 3.4 Geodata 与 GFWList 资源随包策略与按需加载

- **随包预置列表**：内置释放 `Country.mmdb`、`geosite.dat` 与 `gfwlist.yaml -> ruleset/gfwlist.yaml`（构建期经 `scripts/prebuild.mjs::resolveGfwlist` 更新），启动 `init_resources` 在目标缺失/为空/`src_modified > dest_modified` 时释放，使首次安装第 0 毫秒具备完整分流能力；内核运行期按 `rule-providers.gfwlist`（`interval: 86400`，走 `PROXY`）自动定时更新，且本地更新后因时间戳更新而不被回滚覆盖。
- **冗余剔除与加载模式**：不随包 `geoip.dat`（节省 ~17MB）；`enhance` 自动剔除未在最终 `rules` 引用的上游 `rule-providers` 避免启动下载垃圾规则；保留 Mihomo 默认 geodata 加载策略（不注入 `geodata-loader: memconservative`）。

### 3.5 Go Runtime 运行参数深度压制

内核子进程默认注入受控 Go 运行环境（可被外部环境变量覆写）：`GOMEMLIMIT = 128MiB`、`GOGC = 100`、`GOMAXPROCS = 2`。

### 3.6 内核在线平滑升级与多层代理降级链

- **三级网络降级链**：访问 GitHub 查询与下载严格按 `127.0.0.1:10801 → 系统代理 → 直连 (Direct)` 自动降级，超时锁定为单次建连 `10s`、分块读取 `20s`、整包 `300s`（同版本号前置短路）。
- **原子替换校验**：下载目标 `mihomo-windows-amd64.exe.gz` 并流式写入 `.staging`，经 `inspect_mihomo_pe_binary` 校验文件尺寸（`>= 5 MB`）、DOS `MZ`、`e_lfanew`、PE32+ AMD64 头（`Machine=0x8664`、`Magic=0x020B`）与节表（区分 `Corrupted` 与 `IoUnavailable`）；替换前暂停服务并经 `wait_for_mini_cores_exit` 确认旧内核释放锁后原子覆盖。

### 3.7 TUN 模式下 IPv6 流量全量接管（防泄漏铁律）

- **防泄漏路由总原则**：开启 TUN 时无条件强制写入顶层 `ipv6: true` 与 `tun.inet6-address = fdfe:dcba:9876::1/126`，使 IPv6 默认路由必经 TUN 不从物理网卡泄漏；而 `dns.ipv6` 仅由 `enhance::tun::use_tun_with_sniffer` 作为唯一真源同步用户「Allow IPv6」开关（TUN / 系统代理 / 手动三模式严格一致）。
- **本地网段与 Fake-IP 防冲突**：规则尾部必含 IPv4 私网（含 `169.254.0.0/16`）、IPv6 本地（`::1/128`、`::/128`、`fc00::/7` 覆盖 TUN 地址、`fe80::/10`、`ff00::/8`）及逐段 IPv4-mapped 直连规则（严禁整体放行 `::ffff:0:0/96`）；`dns.fake-ip-range6` 固定 `2001:2::0/64`（严格避开 `fc00::/7`）；`dns-hijack` 显式写入 `["any:53", "tcp://any:53"]`。
- **GFWList 兜底与上游代理防直连规则**：默认模式尾部固定 `GEOIP,CN,DIRECT` + `MATCH,DIRECT`（严禁出现 Mihomo 不支持的 `GEOIP6` 或造成 IPv6 偷跑的 `IP-CIDR6,::/0,PROXY`；仅 `rule_fallback == "proxy"` 时使用 `MATCH,PROXY`）；`dns_upstream_proxy_rules_from_hosts` 为 `dns.fallback`/`nameserver`/`nameserver-policy` 非境内主机（含四大公共 DNS 对偶辅 IP）在 `RULE-SET,gfwlist` 后、`append-rules` 前注入 `PROXY` 规则。
- **三态 `sniffer` 与全链路 DNS 卫生**：UI 提供三态 `sniffer`（`null/true/false`，仅 TUN 模式可用；`自动` 尊重订阅显式 `false`，否则启用）；开启 `sniffer` 权威强制 `force-dns-mapping`/`parse-pure-ip`/`override-destination` 一致并补全 TLS/QUIC/HTTP 端口与 `skip-domain`/`skip-dst-address`。`sanitize_dns_upstreams` 统一治理 `nameserver`/`fallback`/`nameserver-policy`（保护 `default-nameserver` 与策略键）：以 `is_local_or_domestic_dns_host` 判定内外网，剥离 `#DIRECT`/`#PROXY` 伪后缀（保留 `#h3=true`），当 `respect-rules` 为假或含 `#DIRECT` 时将境外上游就地降级为可达境内 DoH，当 `respect-rules: true` 时升级境外明文公共 DNS 为 DoH 并由 `inject_dns_proxy_rules` 保障穿代理。

---

## 💤 四、轻量级模式与窗口生命周期

### 4.1 窗口关闭即入轻量模式

- **立即隐身销毁 WebView**：点击关闭按钮立即调用 `window.hide()` 并销毁 WebView2 实例释放 ~30–40MB 内存，仅保留 Rust 主线程、后台守护监测与托盘。
- **无条件阻止退出铁律**：`RunEvent::ExitRequested` 除非 `Handle::global().is_exiting()` 为真（仅托盘点击「退出」置位），否则无条件调用 `api.prevent_exit()`。
- **防误杀策略收敛**：不保留独立看门狗进程或屏外哨兵窗，防误杀指引通过 `轻量模式防误杀指南.txt` + `静默启动` 闭环交付，保留 `logger.rs`（兼容 `&str`/`String` panic 载荷）与 `monitor.rs` 状态恢复日志用于排障取证。

### 4.2 不断网不断流铁律与后端连接维持

- **连续性边界与前后台严格二分**：进入/退出轻量模式、正常冷启动与**后台故障自愈**坚持零断流热切换（绝不调用全局 `close_connection`），内核与 TUN 网卡全速保持在线；仅当用户通过前台 4 大显式切点（列表点选、顶栏胶囊轮换、过滤词/`F1` 优选、在线警示胶囊重测）主动更换节点，或因过滤规则/生命周期恢复（`ensure_proxy_group_selection`、`NodeMigration`）需要将流量迁出已被过滤词排除的地区节点时，才分别调用 `should_close_stale_proxy_connection` / `should_close_excluded_proxy_connection`（`monitor.rs`）与 `shouldCloseStaleProxyConnection`（`path-control.ts`）精准清理对应旧出口/越界出口上的 `PROXY` 存量代理连接，零误伤 `DIRECT`/`REJECT` 与合规连接。`⚡` 与 `F4` 为纯延迟查看，绝不切换节点或触碰存量连接。

### 4.3 前端 WebSocket 熔断与自动重连自愈

进入轻量模式或隐藏窗口超 `1s` 即发送关闭帧（超时 `1000ms`，下限 `500ms`）并调用 `clear_all_ws_connections` 切断前后端推送；前端 WS 重连采用 `15s` 封顶的随机抖动指数退避，监听 `verge://refresh-clash-config`（`250ms` 防抖 + `listenerEpoch`）清空退避并强制替换陈旧 socket。

### 4.4 唤醒禁止配置重置（跳过 force reload）

轻量唤醒仅重建 WebView 视图，内核始终健康运行；唤醒严禁触发 `reload_config(true)` 或规则全栈重建，通过持久化 Profile UID 比对零扰动衔接运行态。

### 4.5 窗口唤醒几何恢复与首帧轻量刷新

- **零阻塞 Win32 恢复**：唤醒一次性还原 `window_state.json` 几何与最大化状态；`is_position_on_screen` / `is_main_window_*` 直接在原子 `MAIN_WINDOW_HWND` 上调用零阻塞 Win32 API（`EnumDisplayMonitors` / `GetMonitorInfoW` / `GetDpiForMonitor` / `IsIconic` / `GetForegroundWindow`），必须在 `unminimize()` **之前**取样 `IsIconic` 跳过最小化坐标校验（`-32000,-32000`），避免跨屏失联且彻底杜绝 tokio 工作线程同步等待主线程死锁。
- **首帧零冗余协同**：唤醒首帧通过 `calcuProxies` 同步状态；轻量唤醒（`skipProbeIfFresh: true`）在当前活跃出口持有有效且未过期（`<= HISTORY_FALLBACK_MAX_AGE_MS`）延迟时**零拨测秒开**，仅在延迟缺失或陈旧时以过滤子集 `names` 复调后端（复用 30s 快照缓存）；浏览器窗口切回前台（`visibilitychange`）仅执行 `3000ms` 节流的 `refreshAll()` UI 刷新而不重复发起前端主动测速，测速统筹统一交由后端 `monitor.rs` 守护线程承担。

---

## 📡 五、后台监控、健康探针与自愈体系

### 🎯 本章最高方针 · 连续性优先

> **一切后台自动测速与自动选节点行为，其唯一目标是最大限度保持链路不断流，并在确已断流时以最短时间恢复连通。「选出最低延迟节点」仅在确有必要发生切换时作为候选排序依据，绝不构成主动切换的理由。非必要不主动切换节点，不主动切断既有连接。**

- **适用范围与豁免**：约束后台监测、健康探针、故障自愈及初始化自动选点；用户显式发起的过滤框地域约束即时切换（`700ms`/`Enter`）、`F1` 全量优选、列表点击及顶栏轮换属主动交互切换，`⚡`/`F4` 仅测延迟不切换。`PROXY` 为 `select` 组、`PROXY__METRICS`（`url-test` / `1800s` / `lazy: true`）不承载业务流量，切点权完全归属应用层。
- **必要性与优先级硬序**：**链路连续性 > 恢复时效 > 节点延迟**。仅当当前节点被确证不可用（`Dead`、伪节点、HTTPS 验真软隔离或已不在过滤子集）或维持现状必然导致/延长断流时才允许切换；初始化场景（冷启动/轻量唤醒/配置切换）由 `hasUsablePersistedExit`/`persisted_exit_is_usable` 门控，只要持久出口结构合法就只刷延迟、绝不切换。

### 5.1 恒定 15 秒健康监测与防抖自愈机制

- **守护运行时物理隔离**：`monitor.rs` 与单实例服务 `server.rs` 托管于 `async_handler.rs` 的 2 线程独立 `DAEMON_RUNTIME`，与 UI/命令默认池物理隔离；`NotificationSystem::send_event` 在 `!WindowManager::is_main_window_ready()` 时跳过窗口事件派发。
- **三档监控周期（`check_interval_secs`）**：在线稳态恒定 **15 秒**；初检失败进入重试态收紧至 **3 秒**；物理离线收紧至 **5 秒**。
- **统一裁决器 `decide_self_heal`**：主循环（`NodeConfirmedDead >= FAILOVER_CONFIRM_FAILS (2)`）、物理恢复（`NetworkRecovered` 恒判 `Wait` 安静等待并交主路径重测）与通道异常（`ApiUnreachable >= API_ERROR_HEAL_THRESHOLD (3)` 且经 `is_clash_api_alive()` 复核仍不通）必须经 `decide_self_heal` 裁决为 `Allow` 后方可进入 `self_heal_with_accounting`；`primary_url_distorted` 时恒返回 `NoAction` 并清零 `consecutive_fails`。
- **四态迟滞、死节点门检与保活**：`NodeStatus` 包含 `Unknown` / `Suspect` / `Dead` / `Alive`；`alive=false` 但上次成功仍在 `SUSPECT_WINDOW_SECS (30s)` 内判为 `Suspect` 不累计失败、不切点；`ConfirmedDead` 自愈切点前必经 `should_proceed_with_self_heal` 二次确认原节点确为 `Dead`（同时豁免速率上限卡死），`ApiChannelDown` 切点前若 `should_keep_alive_on_api_recovery` 复核现节点已恢复 `Alive` 则直接保活复位。
- **判死阈值与宽限期**：测速有效延迟区间锁定 **`[30ms, 5000ms)`**（`NODE_DELAY_MAX_MS = 5000`）；任何手动/自动切点后给予 **`20s`** 切换宽限期（`NODE_SWITCH_NOTIFY` 瞬时吸收）。
- **被动流量证活与分级探测**：基于本地 `/connections` 字节增量划分活跃连接状态：
  - 非 `Alive`（`Unknown`/`Suspect`/`Dead`）或时间戳缺失：强制 `allow_coalesce = false` 实时探测 `probe_metrics_group`；
  - `Flowing`（存在下行回包 `delta_download > 0` 且当前 `Alive`）：免拨测保鲜延展至 `PASSIVE_TRAFFIC_MAX_SKIP_SECS (1800s)`；
  - `Stalled`（有上行零回包 `delta_upload > 0 && delta_download == 0`）：受 `STALL_REPROBE_COOLDOWN_SECS (60s)` 限制在本轮立即实拨复核；
  - `Idle`（无活跃下行）：保鲜窗为 `IDLE_HEALTH_FRESH_SECS (900s)`，零闲时盲测风暴。

### 5.2 单一内核测量组（PROXY__METRICS）统一调度

- **内核按需统一测量**：废弃应用侧单节点 `/proxies/{name}/delay` 并发请求（避开 TUN 自指回环），所有批量、单点与后台拨测统一经 Rust 串口化调用 `delay_group("PROXY__METRICS")`；`PROXY__METRICS`（`interval: 1800, lazy: true`）与 `proxy-providers.*.health-check.lazy = true` 同步收敛，零内核自治盲测。
- **全局锁、单点免复核与异源隔离**：全仓 5 处 `delay_group`（主/备测速 + HTTPS 验真）统一串行持有 `METRICS_PROBE_LOCK`、统一 `NODE_TEST_TIMEOUT_MS (5000ms)` 与 `PROBE_COALESCE_WINDOW_MS (1500ms)`；`should_recheck_primary_with_fallback` 在单节点目标（`allow_single_target_skip`）且本轮主 URL 已有 `>= SINGLE_TARGET_SKIP_FALLBACK_MIN_HEALTHY_PEERS (3)` 个真实节点成功响应时跳过备用 URL 复核。真 HTTPS 验真结果只写 `node.rs` 状态机不入 `evaluate_failover` 判死（`Flowing` 态禁止即时 HTTPS 迁移，门槛为 `HTTPS_VERIFY_MIGRATE_MIN_FAILS = 2`，全败进入 `600s` 退避冷却）。
- **单点测速四契约**：① 仅测量不切点（`select: false`）；② 接受测量组全组并发探测以换取 TUN 无回环唯一真源；③ `DelayResultSource::NotExecuted` 返回空映射时前端调用 `DelayManager.restoreDelaySnapshot` 恢复原始快照与 `updatedAt`，严禁伪造 Timeout；④ 严禁使用陈旧 120s `history` 退化兜底——`len == 1` 强制实时探测不复用快照，批量子集复用 `< METRICS_REUSE_MAX_AGE_SECS (30s)` 快照（`fromProbe = false` 前端封顶 TTL 为 `30s`），自愈复用 `< SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS (10s)` 快照；仅 `fromProbe = true` 写入 30 分钟前端实时缓存。
- **并发互斥、过滤重检、错峰重连与双向 UI 同步**：共用 `AUTO_SELECT_RUNNING` 互斥锁；切点前 `select_fastest_candidate` 再次读取 `latest_filter_text` 过滤淘汰越界候选并写回 `profile.selected`；`Handle::refresh_clash()` 仅派发单一 `RefreshClash` 事件（前端 `use-layout-events.ts` 在 `250ms` 统一刷新 REST 查询，多路常驻 WebSocket 在 `REFRESH_CLASH_DEBOUNCE_MS = 400ms` 后按 `WS_RECONNECT_STAGGER_MS = 45ms` 步进错峰重连）；`MONITOR_WAKEUP_NOTIFY` 仅在退出轻量模式或核心升级/手动切换等必要场景唤醒；非轻量模式下经 `build_probe_ui_display` 与 `LAST_UI_ACTIVE_SYNC`（`UiActiveSyncState { node_name, history_time, reported_alive }`）保证 `covers_all_group_nodes` 真实同步、`Stalled` 全灭消除假绿、恢复必同步最新绿色延迟，前端通过 `preTestUpdatedAt` 保护 `-2` 流光占位不误砍原缓存寿命。

### 5.3 统一记账、切换速率上限与 25 秒失败冷却保护

- **手动抢占、速率上限与冷却**：选点或验真期间若 `MANUAL_SWITCH_EPOCH` 递增则返回 `AUTO_SELECT_MANUAL_PREEMPTED` 良性退让；`self_heal_subject_to_switch_rate_limit` 对 `NodeMigration`/`ApiChannelDown` 施加 `MIN_AUTO_SWITCH_INTERVAL_SECS (120s)` 最小实际切换间隔与 `AUTO_SWITCH_WINDOW_SECS (600s)` 最多 `MAX_AUTO_SWITCHES_PER_WINDOW (3)` 次上限（`ConfirmedDead` 豁免防卡死）；选点全败强制冷却 **25 秒**。
- **3 次重启与 5 次告警解耦**：`evaluate_self_heal_counters` 与 `complete_restart_core_attempt` 在 API 不可达满 `3` 次重启内核成功时保留断流失败计数继续累计至第 `5` 次弹出 Win32 `MessageBoxW` 告警；内核重启报错则立即作为终态弹窗并清零双计数（含原子单实例防叠窗门控）。

### 5.4 订阅伪节点全链路拦截防线与节点过滤同源契约

- **四层合规防线**：① **0ms 生成落盘层**——全链路强制 `profile.store-selected: false`（禁止 Mihomo `cache.db` 复活过滤前旧选点，唯一真源为 `profiles.yaml::selected` 与 `proxy_head_state.json`），且 `enhance::enforce_mini_agreements` 与 `Config::generate_file(Run)` 落盘前通过 `sync_runtime_proxy_group_head` / `promote_preferred_proxy_first` 将符合过滤条件的持久节点或合规子集首节点提前至 `PROXY.proxies[0]`；② **生命周期校验层**——`CoreManager` 启动（`start_core`）、重启（`restart_core`）、重载（`apply_config`）与内核升级统一经过 `resolve_desired_proxy_selection`（Snapshots 合规即同步落盘持久化）与带 `SELECTION_ENSURE_EPOCH` 自取消代际号的异步 `spawn_ensure_proxy_group_selection`（`120ms` 稳态缓冲、`250ms` 轮询、`12s`/`10s`/`15s` 有界预算，不阻塞排他命令锁）校验并在收尾清理 `close_excluded_proxy_connections`；③ **过滤子集兜底层**——`pick_subset_fallback_node` 严格禁止在非空 `filter_text` 下越界选出排除节点；④ **后台守护迁出层**——`evaluate_failover` 检测 `should_migrate_filtered_out_active` 在活跃节点落入排除区且存在合规候选时触发 `ShouldMigrate`。
- **前后端 1:1 伪节点与过滤词同源引擎**：[src-tauri/src/utils/node.rs](src-tauri/src/utils/node.rs)（`is_dummy_node`、`is_structural_banner_name`、`has_region_hint`、`match_node_filter`）与 [src/utils/node.ts](src/utils/node.ts)（`isDummyNode`、`isStructuralBannerName`、`hasRegionHint`、`matchNodeFilter`）逐条对齐：① 英文广告词/URL scheme 支持子串，中文广告短语剥离前缀后严格完全匹配或前缀匹配（严禁前端 `includes` 误伤真实节点）；② 无 `has_region_hint` 保护下的结构形态判定（内嵌非纯域名或冒号配额/日期键值，统一 `trim_start` 兼容全角空格 `\u{3000}`）；③ 多关键词过滤（空格 `AND`、`|`/`｜` `OR`、`-`/`!`/`！` 排除）4 处后端与前端筛选同源一致。
- **连接指纹连坐去重与 24h 可恢复 HTTPS 验真**：`proxy_endpoint_fingerprint` 计算归一化节点 SHA-256 指纹并经 `resolve_proxy_clone_topology` 裁决：同指纹无地区真节点且含横幅的端点整组连坐剔除并记入 `RUNTIME_TAINTED_BANNER_FPS`（前端通过 `get_confirmed_fake_proxy_names` 同步隐藏）；寄生真地区节点的组按优选等级归真。定期真 HTTPS 验真（`HTTPS_VERIFY_INTERVAL_SECS = 600s`，仅测 `Alive` 未验真节点且活跃已验真+`>= 3` 个已验真备用节点时跳过）采用内容拉取：过真 `1` 次永久 `verified_true`；有参照通过时其余连续失败 `> 5` 次进入 **24 小时软隔离（`HTTPS_VERIFY_BAN_TTL_SECS = 86_400`）**，期满自动复察或用户手动更新订阅调用 `clear_unverified_https_failures()` 立即解封。`prioritize_verified_candidates` 与 `utils::node::pick_subset_fallback_node` 优先已验真池并在非空 `filter_text` 下严禁越出过滤子集。

### 5.5 物理网络连通性快速探测与活跃流量免测（TCP:53）

若近 `ONLINE_CHECK_FLOWING_SKIP_SECS (60s)` 内存在活跃代理下行流量（`LAST_FLOWING_TRAFFIC_AT`），直接跳过公网握手判定物理在线；否则优先直连 `223.5.5.5:53`（`PRIMARY_NET_PROBE_TIMEOUT_MS = 250ms`），失败后再并发竞速 `119.29.29.29:53` / `1.1.1.1:53`（`600ms`）并以测速域名解析兜底（避开 TUN DNS 死锁）。在线每 `60s`、离线每 `5s` 探测；恢复离线→在线即刻清零失败计数并发起 `Wait` 稳态首测。

### 5.6 批量测速触发（F4/F1）与结果统一回传

- **快捷键可达性与增量渲染**：`disableWebViewShortcuts` 允许编辑框内直接响应 `F4`/`F1`（同时保护 `Ctrl+F` 与 `Alt+F4`）；后端经 Tauri 事件回传全组延迟并通过 RAF 增量注入前端缓存，默认延迟排序使用 `sort_display_delays`（`0ms` 超时转 `u32::MAX` 垫底）。
- **延迟呈现三级真源与共享状态同步**：按「前端缓存 → 内核 `alive` 判定 → `history` 回落（上限 `HISTORY_FALLBACK_MAX_AGE_MS = 45 分钟`，严格覆盖后端 `1800s`/`900s` 免测窗）」依次取值；前端通过 `DelayManager.beginBatchCheck` 统一管理 `-2` 测试态设置与 `restoreDelaySnapshot` 异常回滚，支持多订阅者（`Set`）精准注销与 `evictExpiredCache()` 主动推送重算，并在 `getDelayFix` 中凭严格更新的有效 `history` 淘汰滞留旧 `Timeout`、`injectBatchResults(..., { fromProbe: false })` 对 `Timeout` 封顶 `30s` 寿命。过滤与排序全局状态托管于 `useHeadStateNew`（`useSyncExternalStore` 模块单例）+ `100ms` 防抖 `save_proxy_head_state` + `flushHeadStatePersist`。

### 5.7 无人值守资源节约与静默门控

设置抽屉闭合或窄窗（`≤285px`）时彻底卸载 `ConnectionsPanel` 并在 `30s`（`gcTime`）回收连接缓存；宽窄窗口统一复用单一轻量 `TrafficGraph`（`1000ms` 采样、`10 FPS` 平滑滚动，窄窗呈极简 `30s` 视图，手动拖宽与最大化下自动切换为 `30s → 60s` 渐变面积 + 对数刻度网格视图，页面最小化或不可见时暂停重绘与流量 WS 订阅，零后台 Web Worker 开销）。

### 5.8 测速失真复核与失败服务器避让

- **主备双地址失真复核**：`probe_metrics_group` 主 URL 整体超时（或部分异常且未命中 §5.2 单点豁免）时立即以不同域名的备用 HTTPS 地址复核；仅主备均有效或明确主URL失真（`primary_url_distorted = true`）时由 `evaluate_failover` 返回 `NoAction` 防误切。
- **同服务器避让与安全阀**：活跃节点基于有效测速判死时，依据生成期 `clash-mini.yaml` 解析的节点→服务器映射将该服务器加入 `FAILED_SERVER_COOLDOWN_SECS (600s)` 冷却避让池；若剔除冷却服务器后候选为空则自动放行安全阀全量回退。

---

## 🛡️ 六、安全防御与编译加固

### 6.1 Tauri 权限最小化与沙箱加固

最小化 capabilities 声明：仅保留 `shell:allow-open`，移除命令行执行、动态窗口/WebView 创建、打印权限及 `tauri-plugin-http`，文件系统作用域收紧至应用数据目录与临时目录；外部 HTTP 请求统一经 Rust `NetworkManager` 收口。

### 6.2 用户输入 URL 严格校验与 SSRF 防护

外部链接唤起仅限 `http://`/`https://`；订阅请求实施三层 SSRF 防护：① `validate_url_no_ssrf` 拦截单标签主机、保留内网后缀、回环/私网/链路本地/ULA/IPv4-mapped IP（本地解析失败允许交由 `10801` 代理回退链解析）；② 直连路径挂载连接期异步 `SsrfSafeDnsResolver` 防 DNS Rebinding TOCTOU；③ 直连与代理双路对 `30x` 重定向每跳复验。

### 6.3 安全解压机制（单文件直写暂存）

内核在线升级（`core_updater`）直接定位压缩包唯一目标可执行文件流式解压至 `.staging` 并核验 PE64 格式与体积，不执行多文件目录展开；诊断包打包强制使用正斜杠 `/` 条目并跳过符号链接。

### 6.4 配置增强极简化与原生 Rust 卫兵

移除嵌入式 JS 运行时（`boa_engine`）与内置脚本，改为原生 Rust `apply_builtin_guards` 完成 `mode: script -> rule` 归一化与 `hysteria` 字符串 `alpn` 转数组；新建配置不再生成 `Script` 项，升级存量配置中的历史 `type: script` 项自动作为 no-op 安全兼容忽略。

### 6.5 编译期加固、死代码门禁与源码保护

- **全仓 Rust 死代码静态门禁**：`src-tauri/src/lib.rs` 严禁配置 `#![allow(dead_code)]`，由 `cargo check` 与 `cargo clippy -- -D warnings` 在编译期强制拦截未使用的 IPC、结构体、字段与函数。
- **溢出检查与关闭 Sourcemap**：Release profile 强制 `overflow-checks = true`；生产构建关闭前端 Sourcemap，压缩体积并保护前端源码。

---

## 📊 七、数据流中枢与协议解析

### 7.1 单一 PROXY 组模型架构

`enhance` 阶段将订阅所有可用真实节点平铺汇聚至唯一选择器组 **`PROXY`** 及同构隐藏测量组 **`PROXY__METRICS`**，彻底剔除多层嵌套策略组，前端展示、排序、过滤与激活 100% 绑定该唯一控制组；运行时配置通过 `sync_runtime_proxy_group_head` 将合规目标置为 `PROXY.proxies[0]` 并关闭 `store-selected`，确保任何冷启/重启瞬间零越界落点。

### 7.2 通用协议解析器（Universal Parser）规范

原生覆盖 `hysteria`、`hysteria2`、`anytls`、`vless`、`trojan`、`tuic`、`wireguard`、`socks5`、`http` 等协议族；共享预处理 `strip_fragment_and_query` 严格遵守 **`#fragment` → `?query` → 末位 `@` 拆解 `userinfo@host:port`** 顺序（防密码或参数含 `@` 截断主机），完整支持 IPv6 字面量、WireGuard CIDR 清洗及 VMess `sni -> servername` 映射。

### 7.3 订阅导入等待与 Provider 刷新协同

新订阅导入或重载后每 `1.5s` 轮询检查直到 `PROXY` 组出现真实有效节点，随即直写刷新前端本地缓存消除首屏延迟。

### 7.4 订阅管理与右键上下文菜单

卡片提供完整右键菜单与带 `Tooltip`/`aria-label` 的行内按钮（刷新/删除均统一弹出 `profiles.modals.confirmDelete` 3D 二次确认框），激活配置 3 次重试失败提示走 i18n Key；订阅弹窗与自定义输入框菜单全面联动 6 套拟物皮肤与原生/Web 双策略剪贴板。

### 7.5 Mihomo 插件 API 与序列化小写补丁

`crates/tauri-plugin-mihomo` 枚举反序列化统一使用 `#[serde(rename_all = "lowercase")]` 与内核 REST API 返回对齐，保障 `Allow LAN` 等设置读取零回弹。

### 7.6 远程订阅请求 User-Agent 版本解耦与兼容铁律

1. **单一事实源**：默认标识定义于 `src-tauri/src/constants.rs::constants::network::SUBSCRIPTION_COMPAT_USER_AGENT`（当前匹配内置 Mihomo `v1.19.x` 的 **`"clash-verge/v2.5.10"`**）。
2. **与 UI 版本号彻底解耦**：禁止动态拼接 `CARGO_PKG_VERSION`，常规 UI 发版不得更改此常量；仅当升级替换 `mini-mihomo` 内核大版本涉及新协议代际时同步审视更新。
3. **单订阅自定义逃生阀**：尊重 `PrfItem` 中的非空 `option.user_agent`，空值自动回退默认兼容标识。

### 7.7 路径控制精准规则生成与四态路由可逆闭环

- **精确目标匹配（禁止扩域）**：在「路径控制」表右键设为直连/代理/封锁时，主机域名统一生成精确 `DOMAIN,<host>,<TARGET>`（仅「添加网址」中显式输入 `*.` 或 `.` 通配前缀才生成 `DOMAIN-SUFFIX`），IPv4/IPv6/进程分别生成 `IP-CIDR`/`IP-CIDR6`/`PROCESS-NAME`；下发按「锁内 `saveProfileFile('Merge', ...)` 直通 `update_config_forced` 重载（仅保存未直通生效时兜底调用 `enhanceProfiles()`）→ 精确关闭命中活跃连接」执行；全局 `Merge` 对全部激活订阅恒生效且在 `enhance` 阶段仅加载一次（禁止当配置 `current_merge` 为空或 `'Merge'` 时重复二次合并）。
- **四态单表融合、YAML 备份与详情显式节点**：`composeActivePathControlConnections` 将实时流、本地手动断开记录、`Merge` 置顶规则以及近期已关闭连接（`_isClosed: true`）融为单表展示 **`直连 (DIRECT)` / `代理 (PROXY)` / `封锁 (REJECT)` / `已断开 (DISCONNECTED)`** 四态；表头提供一致的 3D `保存路由`（`exportCustomRoutingRules`）与 `导入路由`（`importCustomRoutingRules`）一键备份恢复；左键详情 [connection-detail.tsx](src/components/connection/connection-detail.tsx) 在 `主机` 下紧接显示 `代理节点`（`resolveConnectionNodeDisplay`）并对滞留于后台切换前旧节点的连接标示醒目的 `（旧节点连接）`（`staleNodeTag`）。

---

## 🔧 八、工程发布体系与规范

### 8.1 Windows 测试清单与单测执行隔离

- **统一测试命令**：Rust 测试必须执行 `pnpm cargo-test`（通过 `CLASH_MINI_TEST_LINK=1` 注入 `resource.lib` Windows 控件清单防止 `0xC0000139` 符号崩溃）；单测严禁共享全局 App Handle 或 Draft 状态。
- **编译防膨胀规范**：`src-tauri/Cargo.toml` `[lib]` 仅保留 `crate-type = ["rlib"]`；根 `Cargo.toml` `[profile.dev]` 与 `[profile.test]` 对齐 `codegen-units = 64`、内部自研包 `debug = 1`、第三方外部依赖 `debug = false`。

### 8.2 静默发布全流程（Silent Release）

发布通过唯一入口执行：
```powershell
.\scripts\release.ps1 <version>
```
自动完成：`github_token.txt` 注入 `$env:GH_TOKEN` → 检查 `dev` 分支干净及本地提交已推送 → 调用 `scripts/bump_version.py` 递增 4 处版本号（发版核对清单：常规迭代保持 `SUBSCRIPTION_COMPAT_USER_AGENT` 不变，内核大升级同步审视）→ 提交 Tag 触发 CI → 每 3 分钟轮询监控 → 自动拉取最新 `Clash.Mini_<version>_x64-setup.exe` 至 `portable_test/` 并校验完整性。

### 8.3 令牌安全与代理环境防护红线

> [!CAUTION]
> 1. **令牌零暴露**：Token 仅从 `github_token.txt` 读取并经 `$env:GH_TOKEN` 隐式传入 `gh`，严禁打印、落盘或硬编码明文。
> 2. **宿主机代理生命线保护**：测试与发布脚本严禁强杀、重启或探测宿主机正在运行的代理进程与服务（`clash-mini`、`clash_verge_service`、`mini-mihomo`、`verge-mihomo`、`clash-verge`）及其端口（`10801`–`10803`、`9098`、`33335`、`33336`）和命名管道（`\\.\pipe\mini-mihomo`）。

### 8.4 CI 四阶段死锁无关流水线架构

`release.yml` 采用单向无死锁四阶段流水线：
```mermaid
flowchart LR
  V[1. validate 校验] --> B[2. build 云端编译]
  B --> U[3. update_dev 更新配置]
  U --> P[4. publish 正式发布]
```
`validate` 校验一致性 → `build` 编译签名打包并上传 Draft Release → `update_dev` 计算体积签名回写 `updater/app-update.json` 至 `origin/dev` → `publish` 提取 `Changelog.md` 公开正式 Release。

### 8.5 本地制品验收与自动更新同步闭环

所有发行安装包统一下载保留于根目录 `portable_test/`（`git` 忽略，不得清理删除）；发版结束后及时 `git pull origin dev` 同步云端回写的 `updater/app-update.json`。

### 8.6 配置文件版本号与迁移框架

- `verge.yaml`、`profiles.yaml`、`clash.yaml` 顶层维护 `config_version: u32`（缺失视为 `v1`），纯函数逐级前向迁移，遇到更高未知版本只告警不降级写回；发生真实迁移前必须备份 `<文件名>.bak.v<from>`，备份失败中止迁移。
- `CURRENT_CLASH_CONFIG_VERSION = 2`：在 `template()` 合并之前调用 `migrate::detect_config_version(&map)`，自动将 `tun.stack: "system"` 归一化为 `"gvisor"`；`enhance::merge_default_config` 必须从运行时 YAML 中剥离 `config_version`。

### 8.7 可诊断性：日志保留与诊断包

- **日志留存**：运行时默认 `app_log_max_size = 128 KB` × `app_log_max_count = 40`（约 5MB / 24h+）。
- **一键诊断包与高亮定位**：`export_diagnostic_bundle` 将应用及 `service`/`sidecar` 日志与脱敏配置打入 `<app_home>/diagnostics/*.zip`，随后调用 Windows 原生 `explorer /select,"<path>"`（逗号后无空格）选中定位。
- **采集与脱敏硬约束**：① ZIP 条目名必须统一转为正斜杠 `/`；② 按修改时间 `mtime` 倒序采集最多 `MAX_DIAGNOSTIC_LOG_FILES = 60` 个文件、单文件尾部截断 `MAX_LOG_FILE_READ_BYTES = 2MB`；③ 一律跳过符号链接（防 `latest.log` 重复与目录环）；④ YAML 解析失败禁止落盘原文；⑤ `redact_log_text` 须先脱敏 `Authorization:` 整行与裸 `Bearer <token>` 再执行通用 KV 与 URL userinfo 正则，且必须放进 `spawn_blocking` 执行以免阻塞 tokio worker。

### 8.8 前端单测基线与最小覆盖

采用无 GUI `vitest`（`vitest.config.mts`，`environment: node`）运行 **`pnpm test`**，当前包含 13 个纯逻辑回归测试套件，锁定节点过滤、假节点/横幅识别、延迟真源与快照回滚、WS 指数退避、流量解析、窗口尺寸/快捷键、弹窗样式、订阅与路径控制不变式。

### 8.9 用户可见文案 i18n 硬约束

- **全面覆盖与严格类型**：除参与数据比较的内部哨兵串及语言选择器自身语种名称外，全部用户可见文案必须走 `t()`；禁止用 `@ts-expect-error` 掩盖 key 路径错误；相同含义仅保留唯一 key（通用动作词收归 `shared.actions.*`）。
- **13 语种同步与同构 UI**：新增或删除键须同时落地全部 13 个语种（占位符集合一致）并通过 `node scripts/cleanup-unused-i18n.mjs`、`pnpm i18n:check` 与 `pnpm i18n:types`；三处同构输入框右键菜单（`_layout.tsx`、`add-url-dialog.tsx`、`first-run-guide-dialog.tsx`）须同步维护。

### 8.10 安装程序全周期全局置顶铁律

在 `installer.nsi` 中于 GUI 初始化（`myGuiInit` / `un.myGuiInit`）、预检（`Section EarlyChecks`）、安装（`Section Install`）及卸载（`Section Uninstall`）全阶段持续执行 `BringToFront` 与 `System::Call "user32::SetWindowPos(i $HWNDPARENT, i -1, i 0, i 0, i 0, i 0, i 3)"`（`HWND_TOPMOST`），确保安全提示或前台窗口绝不遮挡安装/卸载交互界面；进程清理宏 `CheckAllVergeProcesses` 优先调用 `SimpleSC::StopService` 优雅停服（卸载段优先执行 `RemoveVergeService`），避免强杀触发 SCM Event ID `7031`。

---

## 附录：已知非缺陷（NOT A BUG，勿重复排查）

排查内核日志（`%APPDATA%/<appid>/logs/service/`）时，以下三类告警属于**网络现实或正常生命周期噪声**，核对判据成立即可结案：

| 现象 | 判据（怎么证伪「这是 bug」） | 为什么不改 | 处置 |
| --- | --- | --- | --- |
| `dial DIRECT (match GeoIp/cn) ... : i/o timeout`（境内目标，IPv4 与 IPv6 均可能出现） | ① 同批日志存在其他境内 IPv4 目标同时超时；② 事后直连同一目标可达 | 境内直连链路偶发丢包或站点限流，规则层无法预判瞬时抖动；绕代理更慢更不稳，阻断 IPv6 会误杀正常链路 | **不改规则**（应用层 Happy Eyeballs 自动 v4/v6 竞速重试） |
| `operation was canceled` 成批出现（含 `223.6.6.6` 等国内 DNS） | 发生时间点与内核重启、退出或 TUN 重建重合 | 内核 teardown 阶段统一取消在途 dial，属正常生命周期行为 | 不改 |
| `dial PROXY ... --> <节点server>:<端口> i/o timeout`（目标是**代理节点自身**） | `-->` 右侧为节点服务器 `IP:端口` 而非业务域名 | 属节点端故障或当前线路暂不可达，已由 `15s` 后台守护监测、`Flowing`/`Idle` 被动证活及 `Suspect`/2 次判死自愈闭环自动接管切换 | 不改规则，交由第 5 章节点健康状态机自动处置 |

**通用判据**：判断某条 `i/o timeout` 是否为配置缺陷，只看两点——目标是否为**代理节点自身**（是则属节点健康范畴），以及**同类直连目标换个时间点是否可达**（可达属网络抖动，持续不可达才属规则/路由问题）。
