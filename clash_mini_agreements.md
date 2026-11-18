# Clash Mini 开发设计书

> **适用版本**：v3.5.4 及后续版本  
> **文档性质**：本文档是 Clash Mini 项目的**权威系统设计规范与工程开发宪章**。所有功能开发、重构优化与代码审核必须严格遵循本文档中定义的架构约定、业务逻辑与安全红线。  
> **维护说明**：本文档聚焦于**系统设计意图（做什么、为什么、架构边界与关键参数）**；具体代码实现见配套索引 `docs/implementation-notes.md`，架构全局图谱见 `docs/ARCHITECTURE.md`，历史避坑指南见 `clash_mini_pitfalls.md`。

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
    - [2.10.1 开机自启：注册表方案与启动对账](#2101-开机自启注册表方案与启动对账2026-10-22-定)
  - [2.11 错误边界分级与局部降级](#211-错误边界分级与局部降级)
  - [2.12 首次使用引导与网络就绪反馈（含 GitHub Star 转化设计）](#212-首次使用引导与网络就绪反馈含-github-star-转化设计)
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
  - [4.5 窗口唤醒几何恢复与首帧测速刷新](#45-窗口唤醒几何恢复与首帧测速刷新)
- [📡 五、后台监控、健康探针与自愈体系](#五后台监控健康探针与自愈体系)
  - [🎯 本章最高方针 · 连续性优先](#-本章最高方针--连续性优先)
  - [5.1 恒定 15 秒健康监测与防抖自愈机制](#51-恒定-15-秒健康监测与防抖自愈机制)
  - [5.2 单一内核测量组（PROXY__METRICS）统一调度](#52-单一内核测量组proxy__metrics统一调度)
  - [5.3 统一记账与 25 秒失败冷却保护](#53-统一记账与-25-秒失败冷却保护v324--v332-优化)
  - [5.4 订阅伪节点全链路拦截防线与节点多关键词过滤同源契约](#54-订阅伪节点全链路拦截防线与节点多关键词过滤同源契约noders--nodets)
  - [5.5 物理网络连通性双频快速探测（TCP:53 探针）](#55-物理网络连通性双频快速探测tcp53-探针v324-优化)
  - [5.6 批量测速触发（F4/F1）与结果统一回传](#56-批量测速触发f4f1与结果统一回传)
  - [5.7 无人值守资源节约与静默门控](#57-无人值守资源节约与静默门控)
  - [5.8 测速失真复核与失败服务器避让](#58-测速失真复核与失败服务器避让)
- [🛡️ 六、安全防御与编译加固](#六安全防御与编译加固)
  - [6.1 Tauri 权限最小化与沙箱加固](#61-tauri-权限最小化与沙箱加固)
  - [6.2 用户输入 URL 严格校验与 SSRF 防护](#62-用户输入-url-严格校验与-ssrf-防护)
  - [6.3 安全解压机制（Zip Slip 防护）](#63-安全解压机制zip-slip-防护)
  - [6.4 配置增强极简化与原生 Rust 卫兵（`BUG-381`）](#64-配置增强极简化与原生-rust-卫兵bug-381)
  - [6.5 编译期加固与源码保护](#65-编译期加固与源码保护)
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
- [附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表](#附录-a-历史缺陷与工程修复bug-id追溯映射表)
- [附录 B: 已知非缺陷（NOT A BUG，勿重复排查）](#附录-b-已知非缺陷not-a-bug勿重复排查)

---

## 🏗️ 一、项目定位与架构基石

### 1.1 核心定位与多实例隔离边界

Clash Mini 是一款高度聚焦于极致轻量、超低内存占用、高稳定运行与 3D 拟物美学的 Windows 代理客户端。为确保与原版 Clash Verge（或其他同类工具）在同一台 Windows 设备上**完美并存、互不干扰、流畅运行**，系统在命名空间、网络端口、进程命名及本地存储上实施彻底物理隔离：

1. **软件名称精确性**：面向用户界面的所有窗口标题、对话框、托盘提示均精确展示为 **`Clash Mini`**；安装包与可执行程序名称精确使用 `"Clash Mini"`。
2. **本地配置与数据隔离**：配置目录与原版相互隔离，拥有独立数据存储路径（位于独立的应用数据目录），订阅配置、规则库与运行时数据库互不污染。
3. **网络端口避让与默认凭据铁律**：
   - 默认代理端口固定为：Mixed **`10801`**、SOCKS **`10802`**、HTTP **`10803`**（避开原版默认的 7890/7897）。
   - 内核 Controller API 默认端口固定为 **`9098`**（避开原版默认的 9090/9097），默认控制器密钥为 **`adapted-by-qiu-yuxiao`**（见 `src-tauri/src/constants.rs` 与 `src-tauri/src/config/clash.rs`）。
   - 单实例检测通信端口在 Release 模式锁定为 **`33335`**，Dev 模式锁定为 **`33336`**。
4. **单实例静默唤醒规范（v3.5.2 / BUG-394 / BUG-395 加固）**：单实例 HTTP 服务（`server.rs::embed_server`，`127.0.0.1:33335`）常驻于物理隔离的专属守护 Tokio 运行时 `DAEMON_RUNTIME`，第二实例探测超时为 **`1500ms`**；若检测到端口已被占用，向既有实例发送 `/commands/visible` 通知后**静默退出**，禁止弹出错误弹窗；当 `should_enter_silent_tray_on_launch()`（开启「静默启动」且已有真实订阅配置）为真时，既有实例接收 `/commands/visible` 后**不创建/不弹出主窗口**，维持轻量托盘驻留并通过 Win32 托盘气泡确认运行状态；仅当端口占用但请求彻底失败（超时/连接拒绝）时才提示单实例错误。

### 1.2 Windows x64 单一平台架构收敛

自 v2.9.1 起，Clash Mini 正式完成**平台架构收敛**，项目聚焦于单一目标平台：
- **唯一支持目标**：**Windows 10 x64 (64-bit) 及以上桌面操作系统**。
- **彻底剔除跨平台历史分支**：
  - 后端 Rust 代码全面移除非 Windows 平台的 `#[cfg(target_os = "...")]` 宏与平台专属实现（彻底清理 macOS 托盘富文本/DNS hack/Linux 包装器/Unix 套接字等）。
  - `Cargo.toml` 纯化依赖，移除 `objc2`、`libc`、`nix` 等非 Windows 依赖项。
  - 构建矩阵全面精简：CI 构建矩阵仅保留 `windows-latest`（`x86_64-pc-windows-msvc`），移除非 Windows 编译作业与资源。
  - 前端判定收敛：`getSystem()` 恒定返回 `'windows'`，窗口控制条纯化渲染原生 Windows「最小化、最大化/还原、关闭」控件，前端样式清除所有 macOS/Linux 适配分支。

### 1.3 系统服务模式与 TUN 模式隔离原则

为杜绝多客户端争夺 Windows 服务控制管理器（SCM）导致的代理断流，确立 **TUN-Service 严格隔离规范**：
- **服务启动门控**：**仅当用户明确开启 TUN 模式时，才允许启动/接管 Windows 系统服务**。
- **旁路模式强制优先**：当 TUN 模式未开启（仅使用系统代理或混合端口代理）时，无论系统后台是否存在已安装的服务，Clash Mini 均强制以**本地独立子进程（旁路模式）**运行内核，完全不调用 SCM 服务管线，实现零服务竞争。
- **免 UAC 快速通道与普通用户服务自举闭环**：
  - 当前进程以 Administrator 管理员权限运行时，`wait_for_service_ready` 在 `is_current_app_handle_admin` 为真时直接放行，跳过服务等待，秒级就绪。
  - 普通用户开启 TUN 前由前端 `isTunModeAvailable` 判定权限或服务就绪（`is_service_available`）；未就绪时触发 `installServiceAndRestartCore()`（依次调用 `install_service` 执行 UAC 提权安装 → `restart_core` 重启切换至服务模式），再写入 `enable_tun_mode`（卸载入口为 `uninstall_service`）。
  - 后端 `ServiceManager` 基于 `ServiceStatus`（`Ready`、`StartRequired`、`InstallRequired`、`UninstallRequired`、`ReinstallRequired`、`ForceReinstallRequired`、`Unavailable(String)`）管理状态迁移；安装/启动等待预算为 `SERVICE_WAIT_MAX = 30000ms`（轮询间隔 `SERVICE_WAIT_INTERVAL = 200ms`）。若 `prepare_startup_mode` 检测服务已被禁用或二进制损坏，立即记录告警并持久回退至系统代理；若已启用服务在冷启动时瞬时不可达或等待超时，则通过 `IVerge::enter_temp_tun_fallback` 进入会话级临时回退并刷新前端提示，保护用户磁盘 `verge.yaml` 的原始 TUN 意图不被无关写盘覆写。

### 1.4 进程管理与孤儿清理机制

- **独立命名空间**：所有 Sidecar 内核二进制文件与主进程均添加 **`mini-`** 前缀（如 `mini-mihomo.exe`），在操作系统进程树与任务管理器中建立独占进程空间。
- **退出时孤儿强杀防线**：在主程序退出处理函数（`clean_async`）及崩溃钩子中，异步并发检索并杀灭所有携带 `mini-` 前缀的残留进程，彻底释放网络端口与文件锁，绝不误伤系统上其他 Clash/Verge 客户端。

### 1.5 多语言支持与「英文保留原则」

系统完备支持包括中文、繁体中文、英文、日文、韩文等全部 13 种语言，并严格落实**「英文保留原则」**：
1. **行业标准词汇与协议名一律保持英文**：如 `Mixed Port`、`Allow LAN`、`TUN Mode`、`System Proxy`、`Direct`、`Rule`、`Global` 等，在所有非英文语言包中不得硬性翻译为易混淆的词汇。
2. **拟物风格控制变量一律保持英文**：各皮肤专属的双控制滑块标签（如 `Depth`、`Vibrancy`、`Radius`、`Accent`、`Glow`、`Contrast` 等）在任何语言下均保留原始英文词汇。
3. **分段切换按钮工业宽度适配**：设置侧栏 240px 宽度限制下，西文语言一律使用 3 字母大写工业缩写（`MAN`/`SYS`/`TUN`、`DIR`/`RUL`/`PRX`、`SYS`/`LIGHT`/`DARK`），悬浮 Tooltip 呈现完整名称，彻底防止 UI 折行撑爆。

### 1.6 运行内存基准与资源配额

- **常驻物理内存基准**：主界面开启状态下壳进程物理内存占用约为 **35MB ~ 45MB**，此为正常基准表现（已剔除旧版调用 Windows 暴力修剪工作集 API 导致的二次死锁）。
- **轻量模式内存释放**：进入轻量模式销毁 Webview2 后，外壳内存回落至微量（约 10MB 左右）。
- **内核内存配额约束**：通过 Go runtime 默认环境变量约束内核软堆目标上限为 **`128MiB`**（`GOMEMLIMIT`，支持环境变量覆写），常驻运行内存极度受控。

### 1.7 版权规范与合规署名

- 整体项目遵循 **GPL-3.0-only** 开源协议。
- 保留原开源项目之署名与贡献者列表。
- 开发者对 2026 年以来的二次开发与架构重构享有著作权，界面版权页脚统一展示：`© 2026 秋雨潇潇 (修改部分)`。

## 🎨 二、界面美学与交互体系

### 2.1 3D 实体拟物美学准则

Clash Mini 坚守 **3D 实体拟物与物理光影美学**，彻底摒弃扁平寡淡的扁平化界面，赋予控件物理厚度、真实倒角凹凸感、高光反射与动态光泽：
- **实体纯色无虚化**：禁用整窗半透明与 Aero/Mica 模糊滤镜，主背景、弹窗、菜单与面板 100% 不透明纯色渲染，消除渲染撕裂与跨显卡卡顿。
- **经典几何分割线**：表格采用 `2px solid` 拟物凹槽网格线；模块间缝采用 `5px double` 双线；设置抽屉采用 `4px double` 双实线结构，构建紧凑工业质感。

### 2.2 遮挡式布局与响应式视口框架

窗口采用**「遮挡式取景框」布局**，严格区别于传统自适应的「挤压式」布局：
- **缩放几何恒定**：窗口缩放拖拽时，界面内部元素不缩小、不重新折行排版、不发生挤压变形；窗口宛如物理取景框，框大露出更多内容，框小仅露出最核心关键内容。
- **双宽窄结构临界点（内容宽度 285px）**：
  - 宽窗（>285px）：节点列表呈现 3 列立体卡片表格；顶部活跃节点信息栏展示 4 项详细指标。
  - 窄窗（≤285px）：节点列表自动切换为 1 列纵向信息行；顶部活跃节点栏紧凑展示 2 项核心指标。
  - 右侧连接与路由监控表遵循大窗（>285px）且设置抽屉展开时挂载观测、设置关闭或窄窗（≤285px）彻底卸载不渲染、不订阅且经 `gcTime`（`30s`）自动回收连接缓存的统一生命周期（BUG-385：`useConnectionData` / `usePathControl` 与 `<ConnectionDetail />` 内聚于 `ConnectionsPanel` 自身生命周期）；打开设置期间底层主页节点列表 `ProxyGroups` 同步卸载（关闭后经 `localStorage` + `useLayoutEffect` 原位瞬时恢复）；设置关闭按钮层级锁定 `SETTINGS_CLOSE_BUTTON_Z_INDEX (250)` 避免越过模态弹窗，`AddUrlDialog` 独立置于外层弹窗区并配合关闭抽屉清理菜单锚点与端口框未提交失焦保护（`shouldSkipMixedPortBlurSave`）。
  - **该断点与窗口最小宽度（271px）不是同一个数**：断点是排版门槛（比较内容宽度），最小宽度是几何下限。最小宽度必须小于断点，保证窗口缩到最小时必然进入窄窗模式，且内容宽度 272–285 区间行为与历史版本一致（否则三列卡片会被挤进 272px）。
- **窄窗 `271px` 弹窗/通知区域锁定与视觉风格统一规范**：
  - 所有模态弹窗（`AddUrlDialog`、`AntivirusWhitelistDialog`、`FirstRunGuideDialog`、`HelpMenuButton` 一/二级弹窗、`LayoutDialogs`）与顶部反馈通知（`NoticeManager`）宽度均严格锁定为 `MIN_WINDOW_WIDTH`（`271px`）并锚定于左侧窄窗区域（`left: 0, width: 271px`），顶部按 `resolveNarrowOverlayTopOffset` 避让 `CUSTOM_TITLEBAR_HEIGHT`（`30px`；原生装饰栏或极窄隐匿隐藏标题栏时贴顶 `0px`），即使在宽窗口（`>285px`）下也绝不居中漂移或撑满宽窗；且 `NoticeManager` `zIndex` 必须高于 MUI `Tooltip`（`2200 > 1500`）并带内边距（`px: 6px, pt: 4px`），防止操作反馈通知被悬浮 Tooltip 遮挡（BUG-371）；
  - 所有窄窗弹窗、`BaseStyledSelect` 下拉选单、全局/输入框右键菜单以及右侧连接表的详情/右键小菜单（`ConnectionDetail` 与 `ConnectionTable` 右键菜单，保留跟随鼠标的小尺寸及宽窗就地展示定位）均统一复用 `src/utils/popup-styles.ts`（`getPopupCardStyle` / `getPopupSkinTokens`）的 3D 实体拟物卡片表面与皮肤高对比语义配色；
  - 「系统调试运行日志」弹窗内部工具栏（日志暂停/播放、升降序、一键清空）与日志级别选择下拉框（固定 `95px`）采用紧凑排版，配合 `overflowWrap: 'anywhere'` 自动换行，彻底消除 `271px` 视口下的任何横向滚动条。

### 2.3 窗口尺寸常量与边界约束

为保证各组件布局绝对严丝合缝，定义全工程严格通用的尺寸边界常量：
- **默认启动尺寸**：`271px × 680px`（逻辑像素）。
- **最大允许尺寸**：`640px × 860px`。
- **极限最小尺寸**：**`271px × 126px`**（`MINIMAL_WIDTH × MINIMAL_HEIGHT`）。
- **为什么是 271**：v3.0.1 及更早版本的无框窗口带系统阴影，客户区比窗口外框窄 22 物理像素（150% DPI 下），外框 285 时内容实际只有 270.67 CSS px。关闭阴影后 outer == inner，若沿用 285 会让窄窗内容凭空变宽约 14px、既有排版变松。故把宽度定死为 271（= 285 − 14），让窄窗内容宽度与历史版本视觉一致。
- **边界防御**：任何情况下（包括高速鼠标拖拽、外部窗口吸附、DPI 切换等），窗口物理与逻辑尺寸均不得缩小至最小尺寸以下。
- **尺寸口径唯一化（铁律）**：全工程的窗口尺寸一律指**内尺寸（客户区）**，位置一律指**外位置**。落地 API 对应关系固定为：尺寸 = `setSize()` / `innerSize()` / `min_inner_size` / `max_inner_size`，位置 = `setPosition()` / `outerPosition()`。**严禁把 `outerSize()` 与 `setSize()` 混用**——二者差一个外框内缩量，混用会让每次缩放多放大一圈（见 BUG-297）。
- **无框阴影必须关闭（`shadow(false)`）**：开启无框阴影时 tao 会给客户区加不可见内缩（实测 150% DPI 下左右各 11、上 2 下 11 物理像素），outer ≠ inner，上述口径铁律被物理破坏。关闭后内缩归零、outer == inner，`force_set_window_outer_size` 与 `min_inner_size` 不再互相矛盾。代价是失去系统投影与 Win11 圆角/1px 描边，由自绘外框（皮肤背景）覆盖。**重新开启阴影将同时破坏缩放几何与最小尺寸声明，不得开启。**

### 2.4 Win32 消息子类化与 WM_SIZING 全向钳制钩子

针对 Windows 系统在无边框（Frameless）窗口边缘缩放时突破尺寸边界或 Y 轴下移飘移的底层缺陷，实施 **Win32 窗口过程子类化防御**：
- **钩子安装与原子句柄主线程闭环（现代化 `SetWindowSubclass` 架构，BUG-332 / BUG-350 / BUG-386 / BUG-394）**：主窗口构建、样式剥离（`strip_caption_style_hwnd`）、外框对齐（`force_set_window_outer_size_hwnd`）与 `SetWindowSubclass` 钩子挂载全部收拢于 UI 主线程 `run_on_main_thread` 闭包内就地一次完成（带 **`12s`** 异步等待超时及失败销毁回滚 `cleanup_timed_out_main_window`），同步登记原子全局句柄 `MAIN_WINDOW_HWND` 并在 `WM_NCDESTROY` / `destroy_main_window` 时自动清零；子类化过程拦截 `WM_ENTERSIZEMOVE`、`WM_SIZING` 与 `WM_EXITSIZEMOVE`，并在收到 `WM_NCDESTROY` 时自动调用 `RemoveWindowSubclass` 解绑释放，彻底根除轻量模式销毁重建主窗口时因 Windows 复用 `HWND` 数值或后台线程阻塞导致的钩子漏挂与死锁。
- **全向宽高钳制与会话状态同步（BUG-386）**：在 `WM_ENTERSIZEMOVE` 置位全局原生拖拽标志 `NATIVE_WINDOW_RESIZING`、在 `WM_EXITSIZEMOVE` 复位并持久化最终逻辑几何；在 `WM_SIZING` 中结合 `GetDpiForWindow` 统一把 `WMSZ_LEFT/RIGHT/TOP/BOTTOM/TOPLEFT/TOPRIGHT/BOTTOMLEFT/BOTTOMRIGHT` 八方向提议矩形钳制在 `[MINIMAL_WIDTH, MAX_WIDTH]` × `[MINIMAL_HEIGHT, MAX_HEIGHT]`，并按非移动边反向锁定对边坐标（如 `rect.top = rect.bottom - target_h`、`rect.left = rect.right - target_w`），在系统正式布局前同时防漂移、防越界。
- **重入与睡眠唤醒防御**：按当前有效窗口句柄判重并在销毁消息中精确清零，在系统睡眠恢复（Resumed）或多次激活触发时自动短路，避免重复子类化。

### 2.5 无死锁原子 Win32 指针缩放与几何同步持久化

- **禁用系统模态 Resize API**：绝对禁止网页端调用阻塞式系统模态 `startResizeDragging`（该 API 在 UI 线程内直接触发阻塞式 COM 调用，极易诱发窗口死锁未响应）。
- **8 方向透明指针手柄与原生原子缩放管线（BUG-386）**：通过位于窗口四周（10px 宽）及四角（20px 宽）的 8 个自定义指针手柄驱动后端原生原子缩放命令 `begin_window_resize(direction, seq)`、`step_window_resize(seq)`、`end_window_resize(seq)`；通过 `pointerup`、`pointercancel`、`onLostPointerCapture` 与 `(e.buttons & 1) === 0` 及组件卸载清理多重保障，在极端卡顿场景下也能绝对收尾并带单调手势序号 `gestureSeq` 隔离跨手势乱序 IPC。
- **全局屏幕坐标系 + 单次原子 `SetWindowPos`（根除震荡与双 IPC 撕裂）**：
  - 严禁前端用相对窗口客户区的 `e.clientX / e.clientY` 计算左/上/斜角拖拽位移（窗口原点左移或上移后会反向改变 `clientX/Y`，在鼠标静止下也会触发 60Hz 正反馈震荡），亦严禁拆分为 `setSize + setPosition` 双次 IPC（会产生帧间撕裂并触发 `tao::set_inner_size` 缓存边框偏差）。
  - 后端 `begin_native_window_resize` 在单次调用内原子采集 `GetWindowRect`、`GetClientRect`、`GetCursorPos` 与 `GetDpiForWindow`；`step_native_window_resize` 直接读取 OS 全局物理光标 `GetCursorPos` 计算绝对位移（对超界边界仅滑动全局参考游标消除反向拖拽死区，绝不篡改起始矩形锚边），去重后经 `PENDING_SWP_RECT` + `RESIZE_SWP_QUEUED` 合并投递 UI 主线程调用**单次** `SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOCOPYBITS)`。
- **指针捕获必须在任何 `await` 之前建立**：`React` 在事件派发结束后会把 `SyntheticEvent.currentTarget` 置空，`await` 之后再读取只能得到 `null`，导致捕获从未建立；按下时同步登记当前手势序号与清理态，保证快速点击/松手不留残余拖拽标志。
- **拖拽期间冻结断点重排与内核事件降噪（BUG-386）**：遵循 §2.2「遮挡式取景框」设计——当原生/指针拖拽激活（`NATIVE_WINDOW_RESIZING` / `isWindowResizing() === true`）时：
  1. Rust `on_window_event` 跳过逐像素 `Resized`/`Moved` 的 Win32 几何查询与落盘排队，改在 `end_native_window_resize`（或 OS `WM_EXITSIZEMOVE`）收尾时一次性持久化最终尺寸与位置；
  2. `WindowProvider` 仅更新最新 `onResized` 物理像素记录及空闲计时器续期，**暂停 `checkMaximized` 在拖拽中途翻转 `isMinimalWidth` / `isMiniStatus`**，彻底消除反复拖跨 `285px`/`126px` 断点引发的 `ConnectionsPanel` / `TrafficGraph` / `ProxyGroups` 反复挂载卸载与 WS/Worker 震荡卡死；
  3. 拖拽结束触发 `RESIZE_ENDED_EVENT`（携带后端返回的权威客户区物理像素 `width / height / scaleFactor`）一次性精准对齐布局。
- **每帧只按内尺寸基准与锚定边反推**：几何重算对未拖拽维度直接透传原尺寸不做二次 clamp（避免 150% DPI 下最小宽度拖上下边被顶大 1px），西/北拖拽由不可变起始矩形的固定右边/底边反推 `left/top`，`shadow(false)` 下保持 `outer == inner` 零累计误差。
- **与系统原生缩放带共存**：窗口外圈约 4 CSS px（`SM_CXFRAME`）由 Tauri 的 `TAURI_DRAG_RESIZE_BORDERS` 交由 OS `WM_SIZING` 路径处理，内侧归网页手柄驱动的 Rust 原子路径，两条路径均统一受 §2.4 尺寸钳制。
- **节流写盘与版本号有序保护**：非拖拽期几何变动通过 1000ms 节流 + 尾随补写 + 递增版本号互斥写入保护，带 1px 容差守卫，严防 0×0 销毁噪声或旧值倒灌覆盖终值。

### 2.6 极窄隐匿模式（自动隐藏标题栏规范）

- **极窄模式激活条件**：当窗口处于窄窗区间（内容宽度 ≤285px）且高度处于极简高度（≤126px），且用户连续 10 秒无交互时，自动进入隐藏标题栏极窄模式。
- **判定必须在物理像素域做，禁止读页面视口**：宽/高门槛一律用**窗口内尺寸的物理像素**（`onResized` 事件 payload 或 `innerSize()`）比较，判定式为 `物理尺寸 ≤ 阈值 × 缩放因子 + 1 物理像素`。
  理由：页面 `window.innerWidth/innerHeight` 上报的是**整数 CSS 像素**，125%/175% 等非整数档存在取整方向问题（如 125% 档 158 物理像素 → 126.4 CSS → 上报 **127**，而门槛是 **126**，差这 1 像素就会判不成立；150% 档 $126 \times 1.5 = 189$ 虽恰为整数，但加 1 物理像素容差可兜底全档位）。
  历史教训：v3.0.2 / v3.0.3 两次「修了没效果」都栽在这条链上——3.0.3 给视口数值加 `Math.round`，但页面拿到的是整数，取整对整数是空操作，故零改善。
- **判定入口唯一**：极小窗 / 窄窗状态只能由同一个计算函数从物理尺寸派生后写入 state 与 ref，禁止在计时器回调、事件监听里各自重算一遍阈值比较（旧实现为此有两份判定，任何阈值调整必然漏改一处）。
- **空闲计时器必须自续期**：10 秒定时器每次触发后**无条件重新排程**，禁止「条件不满足就 `return`」——那样启动 10 秒后窗口还不是极小窗，计时器即永久死亡，功能整场会话失效。计时器的续命来源必须覆盖全：页面内用户输入、**窗口几何变化（`onResized`）**、退出隐匿。续期用「清除后重排」即可，不额外引入固定节拍轮询（不增加唤醒频率）。
- **极简纯净边缘（移除外框装饰）**：移除极窄隐匿模式下的 GlowBorder 外发光与自绘边框装饰，窗口边缘保持原汁原味的极简纯净背景，杜绝任何视觉杂质与多余渲染开销。

### 2.7 6 套特色皮肤与双控制滑块体系

系统内置 6 套完整的 3D 拟物与物理质感视觉主题：
1. **Retro 3D (Trump-3D)**：默认皮肤，以金条物理质感、高浮雕按键与深邃暗底为特色。
2. **Original**：致敬经典工业风，灰白实体与硬朗刻线。
3. **Modern**：深色现代立体，兼具简约与倒角层次。
4. **Frosted Glass**：实体暗黑磨砂玻璃质感，高对比度立体轮廓。
5. **Cyberpunk**：暗夜霓虹，高饱和电光蓝与荧光品红。
6. **Monochrome**：黑白极简工业风，高反差实体黑白按键。

- **双控制滑块解耦与独立存储**：在不同皮肤下，两个控制滑块对应不同的底层物理参数（如 Depth/Vibrancy、Radius/Accent 等），各参数在 LocalStorage 中使用独立字段存储，切换皮肤不产生数值污染。
- **字体族共鸣对齐**：各皮肤严格匹配中英文字体组合（如 Retro 3D 匹配 Trebuchet MS / SimHei；Cyberpunk 匹配 Consolas / NSimSun 等），确保跨语言视觉一致。
- **硬朗直角皮肤小开关尺寸几何统一规范（BUG-384）**：`Cyberpunk` 与 `Monochrome` 两套锐利直角工业风皮肤下的 `Switch`（[base-switch.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-switch.tsx)）统一采用 `0px` 直角方形几何（常规 `28 × 14px`、`size="small"` 收敛为 `18 × 10px`、轨道 `borderRadius: 0`、滑块 `8 × 8px` / `12 × 12px` 且 `borderRadius: '0px'`）；`Monochrome` 以纯黑白灰高反差边界区分开闭态，与同卡片内的直角分段控件及按钮保持视觉统一。
- **三态分段开关尺寸与纵向对齐规范**：`ThemeSettingsCard` 中的主题模式三态开关（`系统 / 浅色 / 深色`）统一与 `BasicSettingsCard` 中的 `TUN 域名嗅探` 三态开关保持完全一致的 `118 × 20px` 外框尺寸与 `11px` 粗体字号，并通过 `marginInlineEnd: 4px` 对齐两张同列设置卡片内部开关的左右边界（标题文字仍保持最左对齐），同时对标题与选项文字统一施加 `textOverflow: 'ellipsis'` 与 `title` 悬浮提示以兼容多语种长文案。

### 2.8 主页节点列表与活跃节点轮换

- **三列网格布局**：默认模式下每行 3 个节点卡片，单节点高度固定 20px；窄窗模式平滑退化为单列。
- **整行 0ms 乐观切换与延迟徽章独立单测热区（BUG-356 / BUG-357 / BUG-396）**：节点卡片主体区域点击瞬间对 `['getProxies']` 缓存执行 `0ms` 乐观高亮切换并异步同步内核（失败自动回滚），且随后立即触发精准存量连接清理（详见 §4.2）；卡片右侧延迟徽章设为独立点击热区（阻止冒泡），点击仅对该节点触发实时单点测速（`select: false`），不中断当前活跃出口连接。
- **活跃节点快速轮换、纯测速边界与前台即时切换存量清理（FEAT-006，BUG-336 / BUG-390 / BUG-396 / BUG-397）**：点击顶部导航栏活跃节点卡片，基于用户当前搜索/过滤/排序状态实时计算可用子集，直接轮换至下一顺位可用节点、即时刷新 `['getProxies']` 缓存并同步写回 `profile.selected` 持久化；当全节点超时或物理断网时顶栏展示轻量警示胶囊供一键重测自愈（BUG-359），且在物理断网（`networkHealth === 'network_health::offline'`）场景点击重测仅刷新延迟（`select: false`），严禁离线状态下触发无意义切点（BUG-390）。列表顶部闪电图标 `⚡`（`BoltOutlined`）与 `F4` 快捷键经共享意图裁决函数 `resolveBatchCheckIntent('bolt' | 'f4')` 收敛为**纯延迟查看（`select: false, cleanStaleConnections: false`）**，仅刷新当前可见列表延迟数字，绝不更换当前节点、绝不断开任何存量连接；而全部 4 类前台主动换节点入口（节点列表点选、顶栏活跃节点胶囊轮换、过滤框输入自动选点与 `F1` 全量优选、异常胶囊在线重测选点）成功切换后，均调用精准清理逻辑关闭仍挂在非当前目标节点上的 `PROXY` 存量长连接（见 §4.2）。
- **节点过滤与自动并发测速择优接管（地域策略即时切换约束，特例豁免，BUG-392 补强）**：在 Clash Mini 的单一 `PROXY` 组架构下，节点列表顶部的过滤输入框不仅用于视图搜索，更承担**地域策略约束开关**职责（典型场景：为应对如 Antigravity 拒绝香港 IP 等服务区域限制，用户输入 `-香港` 或 `美国|新加坡` 即表达「撤出受限地区并切入合规节点子集」的明确意图）。因此当用户完成过滤词输入（实体输入停止 700ms 防抖，或按 `Enter` 键即时生效）时，系统自动对当前过滤子集触发一次多路并发测速与择优切换（`triggerAutoSelect`，`select: true`），所见即所选。该交互属于**用户主动发起的地域策略切换**，与后台无感知自动切点严格区分，受 §5 显式豁免；为根治输入法拼音/算子半输入态抢跑与在途竞态（BUG-392）：① `BaseSearchBox` 在中文 IME 组词期（`onCompositionStart`~`onCompositionEnd`）仅维护本地显示缓冲而不外发拼音中间态；② `ProxyHead` 通过 `isIncompleteFilterInput` 跳过纯操作符或末尾悬空算子（如 `"-"`、`"!"`、`"日本 -"`、`"日本|"`）；③ `ProxyGroups::handleCheckAll` 与 `handleCheckFull` 选点前立即 `flushHeadStatePersist()` 同步 `filterText` 至后端，且 `handleCheckAll` 同步追踪 `activeTestSelectRef` / `pendingRecheckSelectRef`：当在途测速（含闪电/`F4` 纯测速）尚未结束而 `filterText` 已更新或由过滤提交升级为 `{ select: true }` 时登记尾部重测队列（`pendingRecheckGroupsRef`）及有限次 `AUTO_SELECT_BUSY` 续试，绝不静默丢弃；④ 若过滤词未匹配到任何可见节点则安全短路跳过，清空过滤框时仅重置列表而不触发全量测速。

### 2.9 极简静态菜单系统托盘规范

为避开 Windows 高权限与 UAC 提权环境下 Win32 托盘 Shell API 高频重建或轮询刷新导致的系统级 `E_FAIL` (0x80004005) 错误，托盘实施**静态固定菜单 + 仅模式切换换标**规范（`src-tauri/src/core/tray/mod.rs`）：
- **固定双项菜单与左键双向切换（BUG-361）**：
  - 左键单击执行双向窗口切换——若不处于轻量模式且主窗口当前可见、未最小化且处于前台聚焦状态，则单击隐藏收起主窗口，否则激活恢复并置顶主窗口。
  - 右键弹出固定结构双项静态菜单，禁止运行期销毁或重建菜单：①「轻量模式 / Lite mode」（切换时仅通过 `item.set_text` 同步前缀 `"✔ "` 勾选标识）；②「退出 (Exit)」。
- **图标状态映射与按需静默气泡（无轮询、无动态重绘，BUG-395）**：`Tray::update_icon` 仅在代理模式变更事件时按需调用 `tray.set_icon`，在内置三枚静态 PNG 图标间切换：`tray-icon-tun.png`（TUN 模式）、`tray-icon-sys.png`（系统代理模式）与 `tray-icon.png`（仅本地混合代理）；`MAIN_WINDOW_FOCUSED` 以原子布尔值缓存窗口焦点以供托盘左键无阻塞判定；仅当用户启用「静默启动」后**手动非 `--silent` 启动或再次点击启动图标**时，按需通过原生 Win32 `Shell_NotifyIconW(NIM_MODIFY, NIF_INFO)` 弹出带 `2s` 防抖的托盘气泡提示（零后台轮询、零创建 WebView 窗口）。严禁在后台定时器中高频刷新图标或更新 ToolTip。

### 2.10 设置开关的乐观更新与权威数据源

- **权威数据源分离（不得混淆）**：设置界面的开关**必须绑定「用户持久化配置」**，而不是内核 REST API 返回的**运行时配置**。原因：运行时配置会被后端按场景强制改写（如 TUN 模式下顶层 `ipv6` 恒被强制为 `true` 以接管 IPv6 路由防泄漏），若 UI 直接绑运行时值，开关会被强制弹回、无法切换。落地：`ClashInfo` 额外暴露用户在 `clash.yaml` 中持久化的 `ipv6` / `allow_lan`，开关统一绑 `clashInfo.*`（`get_clash_info` 取 `Config::clash().get_client_info()`，读用户存储配置而非运行时）。
- **乐观更新契约**：开关切换时**先**把补丁写入 `queryClient` 缓存（`setQueryData`）以获得 0ms 手感，**再**发后端请求；请求**成功**后重取权威值同步、**失败**则立即回滚（`refetch`）——保证「UI 状态 ≠ 后端状态」的窗口尽可能短。开关触发的 `patch_*` 属长操作（可能重启内核、阻塞 IPC 达 60s），必须配 `beginLongOperation` / `endLongOperation` 抑制 IPC 超时提示风暴。
- **queryClient 必须同源**：全应用**只能存在一个** `QueryClient` 实例（`main.tsx` 的 `QueryClientProvider` 与各处 `import { queryClient }` 必须指向同一 `@/services/query-client` 单例）；`useQueryClient()` 与直接 import 不可各自持有不同实例，否则乐观更新会写进另一个缓存、UI 不刷新。
- **并发点击语义**：`patchVerge` 使用 `useLockFn`，其语义是**丢弃**并发调用（而非排队）——这是**有意为之**，用于防止并发 patch 引发内核反复 reload 与 IPC 风暴。因乐观更新写在锁内，被丢弃的调用不会改 UI，故缓存与「未执行」的事实保持一致；成功执行时返回 `true`（并发丢弃时 `useLockFn` 解析为 `undefined`），调用方据此判断是否跳过成功提示，且可传 `{ silentError: true }` 由外层统一展示单次错误弹窗，避免 `useVerge` 与调用方重复弹窗（BUG-371）。
- **设置开关操作反馈三段式契约（BUG-371）**：流量接管模式与基础设置中的所有二选一 / 三选一开关与端口保存操作，必须严格按「进行中提示（`showNotice.info(..., 0)` + `markLoadingNotice(waitId)`）→ 结果提示（`showNotice.success` / `warning`）→ 失败单弹窗（`showNotice.error`）→ `finally { settleLoadingNotice(waitId) }`」闭环实现；`settleLoadingNotice` 保证快速完成（`<320ms`）的后端 IPC 调用也能让用户看清"正在调整"状态条后再自然退场。

### 2.10.1 开机自启：注册表方案与启动对账（2026-10-22 定）

- **实现介质：注册表 `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`**，由已注册的 `tauri-plugin-autostart`（底层 `auto-launch`）写入；**不再使用计划任务**（`utils/schtasks.rs` 已整体删除）。
  - **为什么不用计划任务**：它唯一的独有能力是 `RunLevel=HighestAvailable` 可「免 UAC 提权自启」，而本项目**用不上** —— TUN 走独立安装的 Windows 服务（`clash_verge_service`，`StartMode=Auto`，开机自启），app 本身不需提权；是否建 Admin 任务原本也只取决于「进程当前已提权」，实测现网任务都是 `RunLevel: Limited`。
  - 换回注册表的收益：删掉约 400 行 `schtasks` 逻辑（含仅为解析 `schtasks.exe` 中文输出而存在的 OEM/ANSI 代码页解码）、无子进程、无 XML 落盘，用户在任务管理器「启动」页可直接看到并禁用。
  - 自 `v3.5.2`（BUG-395）起，静默启动统一由 `lightweight::should_enter_silent_tray_on_launch()` 判定：当 **`enable_silent_start == true` 且存在至少一条真实配置（`remote`/`local`）** 时，不仅开机自启（带 `--silent`），包括用户手动双击图标冷启动以及轻量/运行态再次点击启动图标（`/commands/visible`）均直接静默进入系统托盘而不露主窗（仅首次无配置阶段仍开窗完成新手引导；手动启动时弹出托盘气泡提示，左键点击托盘图标可随时打开主界面）。
- **启动必须对账（铁律）**：`resolve_setup_async` 在 `init_verge_config` 之后无条件调用 `autostart::reconcile_on_startup()`，以**配置为准**重写真实自启状态。
  - 旧实现只在用户拨开关时才写状态（`UpdateFlags::LAUNCH`），启动流程没有任何对账步骤；叠加「卸载脚本从不清理计划任务（installer.nsi 全文无 schtasks）」，老用户升级后残留任务继续拉起 —— 即 v3.0.0 实测的「开关显示关、开机却照样启动」。
  - **配置胜出，包括用户在任务管理器里手动禁用的情形**：`enable()` 会同时把 `StartupApproved\Run` 覆盖写回「启用」，所以只要配置是开，下次启动就会重新启用。这是刻意的单一真源设计（避免又一种「配置与事实不一致」），但要在文档里说清楚。
  - **禁用前必须先 `is_enabled()`**：底层 `disable()` 是 `delete_value`，值不存在会报错；对账每次启动都跑，不加判断就会每次刷一条错误日志。
- **开关必须显示真实状态**：自启开关优先显示 `get_auto_launch_status`（读注册表）的结果，配置值仅作回退。否则用户在任务管理器禁用、或注册表被改写后，UI 仍显示错误状态。
- **迁移遗留一次**：`cleanup_legacy_scheduled_tasks()` 在新版首次启动时删除历史任务 `Clash Mini` / `Clash Mini (Admin)`，并落标记文件 `.autostart_task_migrated` 保证只做一次。**不提权、不弹 UAC**：用户级任务普通权限即可删，Admin 级删不掉只记日志，绝不为清理打断启动流程。

### 2.11 错误边界分级与局部降级

- **分级策略**：全局（`main.tsx`）只作最后兜底；页面级（`BasePage`）单页崩溃不影响其他区域；区域级（活跃节点栏、节点列表、连接表、流量图、弹窗组）用 `AreaErrorFallback` 局部降级为一行提示，**严禁因单组件异常整页白屏**。
- **新增重组件必须挂边界**：列表 / 画布 / 依赖第三方渲染的重组件，接入时同步包一层区域边界。
- **边界只捕获渲染期异常**：事件处理与异步代码中的错误不走边界，必须用 `showNotice` 与日志记录自行处理。
- **边界不宜过密**：优先高频出错、依赖第三方渲染的组件；过度拆分只增加复杂度与开销。

### 2.12 首次使用引导与网络就绪反馈（含 GitHub Star 转化设计）

- **触发条件（判定语义，错不得）**：`localStorage.getItem('clash-mini-first-run-completed') !== 'true'` **且列表中没有任何真实订阅**（`type === 'remote' || type === 'local'`）时，启动 500ms 后自动居中弹出引导弹窗（`FirstRunGuideDialog`）。
  - **严禁用 `profiles.items.length === 0` 判断**：后端 `Config::init_config()` 每次启动都会调用 `ensure_default_profile_items()`，**无条件**确保存在 `Merge`（`type: merge`）内置项（从旧版升级的配置还可能保留历史 `Script` 项），故 `items.length` 始终非零、该判定永不成立（v2.9.9 曾因此让弹窗完全变成死代码）。
  - 判定逻辑固定在 `src/utils/profile-items.ts`（`hasRealProfileItem`），并有回归单测；存量老用户绝不被打扰。
- **两步式价值感知与转化向导（`STEP 1 / 2` → `STEP 2 / 2`）**：
  - **横向全宽铺满默认 `271px` 窄视口**：由于初次启动默认窗口宽度为 `271px`（见 §2.3），引导弹窗必须显式声明全宽约束（破除 MUI `Dialog` 默认左右外边距挤压）并配合紧凑内边距，严禁在瘦长窗口内再嵌套一条更窄的瘦长弹窗。
  - **Step 1（一键导入订阅或单/多行节点 URI，BUG-389）**：顶部配有步骤胶囊（`STEP 1 / 2`）与三枚核心特性徽章（智能优选低延迟节点 / 防 DNS 泄漏保护 / 轻量静默驻留）；`isSupportedSubscriptionOrNodeInput` 与 `SUPPORTED_NODE_URI_SCHEMES`（位于 `src/utils/node.ts` 并由 `FirstRunGuideDialog` 调用）同时校验单行 `http(s)://` 订阅链接及单/多行节点 URI（`vmess://` / `vless://` / `ss://` / `trojan://` / `hysteria2://` / `hysteria://` / `anytls://` / `tuic://` / `wireguard://` / `socks5://` / `socks://`）；弹窗打开时自动静默嗅探系统剪贴板并复用同源校验展示智能横幅（`clipboardDetected`），同时保留剪贴板按钮与右键菜单；导入成功后原地切换至第二步。
  - **Step 2（实时连通性仪表盘 + GitHub Star 激励卡）**：
    - **卡片 1（实时连通性面板）**：展示订阅导入成功状态、代理就绪探测结果，并在就绪时实时展示**当前最优出口节点名称**（`activeNodeName`）与**绿色发光延迟徽章**（`⚡ XX ms`）。
    - **卡片 2（金色 GitHub Star 转化卡 + Wiki 入口）**：采用暖金渐变卡片展示开源支持文案（`starTitle` + `wikiHint`），附带「📖 查看 Wiki 进阶指南 ↗」次级链接，底部主 3D 按钮升级为「⭐ 打开 GitHub 测速并点 ⭐ Star」（`CLASH_MINI_GITHUB_URL`；由 i18n 文案自身携带 `⭐`，JSX 不重复硬编码前缀），在用户刚体验到导入成功与节点低延迟的愉悦时刻自然引导浏览器打开仓库主页测试国际网络并顺手点亮 Star。
    - *(注：`v3.5.2` / `BUG-395` 已从引导 Step 2 彻底移除冗余的安全软件卡片；轻量模式防误杀原因与两套方案收敛至 `?` 帮助菜单专属入口「🛡️ 轻量模式防误杀指南」。)*
- **就绪判定必须是「测出来」而不是「假定」**：第二步只有在**活跃出口节点存在有效延迟**（`0 < delay < NODE_DELAY_MAX_MS`，数据取自既有代理上下文，不另造测量链路）时才显示「代理已就绪」。未测出则显示「正在确认节点连通性…」并每 1.5s 刷新一次；超过 20s 仍无有效延迟则改成如实提示「暂未测出节点延迟」，**不得宣称网络畅通**。
  - 同时按 `sysproxy.enable` 的真实状态提示「系统代理未开启」。**严禁再写「TUN 已全量就绪」**——`init_config` 在非管理员且未装服务时会把 `enable_tun_mode` 强制置为 `false`，首次安装用户绝大多数属于这种情形。
- **帮助菜单常驻 Star 与防误杀指南入口**：顶栏 `?` 帮助菜单（`HelpMenuButton`）首项固定为加粗的 `⭐ 去 GitHub 点亮 Star`（`layout.components.helpMenu.github`），为错失首次引导的老用户提供常驻支持通道；菜单内设专属入口 `🛡️ 轻量模式防误杀指南`（`layout.components.antivirus.menuLabel`，BUG-395），点击弹出 `AntivirusWhitelistDialog`（全 13 语种对齐 `docs/antivirus-whitelist.md`），说明 360 在高负载下对「开窗→关窗」残留进程的处理机制，并提供① 弹窗内一键开启「静默启动」避开露窗 + 误杀重开静默进托盘的省心方案，与② Windows 安全模式写入 `whiteinternal.ini` 硬白名单的永久根治方案（含路径与配置片段一键复制按钮）。
- **导入不可被误中断**：导入进行中（`submitting`），点击遮罩与按 **Esc**（`reason === 'escapeKeyDown'`）都必须被拦截，不允许关闭弹窗。
- **优雅稍后配置与再次唤起保护**：所有出口（完成 / 打开 GitHub Star / 打开 Wiki / 稍后手动配置）统一记录 `clash-mini-first-run-completed: true`；仅当确属首次引导（`localStorage.getItem(FIRST_RUN_GUIDE_STORAGE_KEY) !== 'true'`）导入成功且未启用系统代理/TUN 时才自动开启系统代理，从帮助菜单再次打开引导时尊重用户当前代理开关偏好（BUG-389）；「稍后手动配置」还会自动滑开左侧抽屉指引订阅管理卡片位置。
- **多语言必须齐全**：本弹窗文案走 i18n（`layout.components.firstRun.*`），**13 个语种资源全部提供**。理由：应用已按系统语言自动初始化（`system_language()` + `clash_verge_i18n`），支持 zh/zhtw/en/ru/fa/tt/id/ar/ko/tr/de/es/jp；首次引导是用户看到的第一屏，单独硬编码中文等于给非中文用户「一上来就是外语」的糟糕第一印象。新增 key 后须执行 `pnpm i18n:types` 重新生成类型。

---

## ⚙️ 三、内核生态与运行时管控

### 3.1 内核选择与安装包体积极致精简

- **单一内核发布战略**：自 v2.9.1 起彻底移除预装 Alpha 内核（`mini-mihomo-alpha`），全局统一搭载官方最新经过充分验证的稳定版内核（`mini-mihomo`）。
- **体积瘦身成果**：通过剥离冗余内核与关闭前端 sourcemap，安装包体积由 45MB 直降至 **~33.7MB**（缩减 25% 以上），大幅降低网络分发与冷启动磁盘解压开销。

### 3.2 Windows 命名管道与高并发退避重试

- **专用 IPC 管道通信**：Rust 后端与内核之间的 HTTP API 及 WebSocket 通信，100% 依托 Windows 原生命名管道（`\\.\pipe\mini-mihomo`）建立，杜绝开放网络 TCP 控制端口带来的端口抢占与安全隐患。
- **指数退避重试机制（busy_retry_count = 24）**：在批量测速、自愈选点与状态上报突发高并发场景下，针对命名管道可能出现的 `ERROR_PIPE_BUSY`（管线繁忙）错误，内置指数退避重试引擎，最大重试次数锁定为 **24 次**，配合 Win32 `WaitNamedPipeW` API 形成弹性质询，100% 杜绝并发撞管引发的请求丢失或假死。

### 3.3 端口冲突自适应探测与私有密钥保障

- **动态端口避让与自愈**：内核 Controller 默认监听 `127.0.0.1:9098`。在启动初始化阶段，若探测到端口被占用，先向该端口发送鉴权探测；若带私有密钥校验成功，则判定为异常退出的前代残留进程予以平滑复用；若鉴权失败（被第三方程序占用），自动向上逐个扫描空闲端口，并动态重写内核启动配置完成无感迁移绑定。
- **控制密钥私有化**：废弃所有硬编码公共密钥，外部控制密钥采用全局强随机散列固定值保护，阻断本地恶意软件利用内核 API 篡改网络路由。

### 3.4 Geodata 与 GFWList 资源随包策略与按需加载

- **精简随包清单**：
  - **随包并释放至运行目录**：`Country.mmdb`（GEOIP 与 fallback-filter 规则必备基础库）、`geosite.dat`（GEOSITE 分流规则库）以及 `gfwlist.yaml -> ruleset/gfwlist.yaml`（GFWList 境外域名分流规则集，约 95KB）。
  - **GFWList 发版拉取 + 随包预置 + 定时更新闭环（BUG-309）**：每次构建/发版阶段（`scripts/prebuild.mjs::resolveGfwlist`）主动从多镜像源拉取最新 `Loyalsoldier/clash-rules` 的 `gfw.txt` 并校验打入 `resources/gfwlist.yaml`；应用启动早期（`init_resources`）若运行目录 `ruleset/gfwlist.yaml` 缺失、为空或旧于随包版本则立即释放到位，使全新安装第 0 毫秒即具备完整境外域名分流能力，彻底省去首次启动联网等待拉取规则的逻辑；运行期仍由内核 `rule-providers.gfwlist` 按 `interval: 86400`（24 小时）经 `PROXY` 通道自动定时更新，且更新后的文件因 `dest_modified > src_modified` 在后续日常重启时自动保留不被旧随包文件覆盖。
  - **移除随包**：移除体积庞大的 `geoip.dat`（节省约 17MB 磁盘与包体积），满足 99.9% 订阅规则之需。
- **内核默认加载语义**：运行时配置不注入 `geodata-loader: memconservative`，沿用 Mihomo 内核默认地理数据库加载机制（与架构总览 §5.7 一致）。

### 3.5 Go Runtime 运行参数深度压制

内核作为后台子进程运行时，通过启动环境变量实施深层运行时参数压制，限制其在桌面端轻量运行：
- **`GOMEMLIMIT = 128MiB`（可由环境变量覆写）**：约束 Go GC 软堆内存目标上限为 128MiB，触发积极回收，兼顾突发并发稳定与低内存占用。
- **`GOGC = 100`（可由环境变量覆写）**：维持标准回收频次，降低拥塞突发时的 GC CPU 抖动。
- **`GOMAXPROCS = 2`（可由环境变量覆写）**：限制内核最大并行线程数，契合轻量桌面端代理定位，杜绝空闲时占用多核 CPU 资源。

### 3.6 内核在线平滑升级与多层代理降级链

- **全流程网络降级机制**：所有访问 GitHub API 查询最新内核版本及下载 Release 包的逻辑，统一托管于后端网络管理器，严格按如下链路顺序自动重试与降级：
  $$\text{Localhost 代理 (10801)} \longrightarrow \text{Windows 系统代理} \longrightarrow \text{直连 (Direct)}$$
- **升级安全与写锁规避**：下载与校验采用 `mihomo-windows-amd64.exe.gz` 纯净版本；替换时若系统服务运行中，先优雅断开服务以规避 Windows 可执行文件写入占用锁，安全解压替换后再行恢复。
- **超时与相同版本前置过滤**：单次建连与握手超时 10 秒，流式读取单块超时 20 秒，整包读取上限 300 秒；若比对版本号相同，前置阻断下载并给出静默提示。

### 3.7 TUN 模式下 IPv6 流量全量接管（防泄漏铁律）

- **问题定性**：mihomo 官方文档明确 `tun.inet6-address`「需同时将顶层配置中的 `ipv6` 设为 `true` 才生效」。此前 TUN 启用时若用户关闭「Allow IPv6」（顶层 `ipv6: false`），内核不具备 IPv6 收包能力，**不会把 IPv6 默认路由导入 TUN**——系统 `::/0` 仍指向物理网关，导致**所有 IPv6 流量从物理网卡直连、100% 绕过代理泄漏**（实测：到 Google 的 IPv6 请求立即失败，且不经过代理）。这属「内核禁 IPv6 + 系统开 IPv6」的最差组合。
- **铁律**：**TUN 启用时，内核必须始终具备 IPv6 接管能力**（顶层 `ipv6: true` + 显式 `tun.inet6-address`），使 IPv4 与 IPv6 的默认路由**一律由 TUN 接管**，IPv6 流量必须进入内核按规则处理，严禁从物理网卡泄漏。
- **开关语义二分（不得混淆）**：
  - **顶层 `ipv6`** 管「TUN 是否接管 IPv6 路由（防泄漏）」——TUN 启用时恒为 `true`，不由用户开关决定；
  - **`dns.ipv6`** 管「是否真正使用 IPv6」——由用户「Allow IPv6」开关决定；关闭时内核 DNS 不返回 AAAA（回空解析），应用拿不到 IPv6 地址、实际走 IPv4。
- **`dns.ipv6` 在三模式下语义统一（2026-10-16 追加）**：`dns.ipv6` 必须**同时**跟随「Allow IPv6」开关，**TUN / 系统代理 / 手动三种模式一致**，不得只在 TUN 模式下同步。
  - **非 TUN 模式无需防泄漏**：非 TUN 时系统流量默认不经过内核（仅显式指向代理端口或经系统代理的流量入内核），不存在「全局接管被绕过」的泄漏面，故**不强制** `ipv6: true`、**不写** `inet6-address`。
  - **但 `dns.ipv6` 仍须跟随开关**：否则订阅若自带 `dns.ipv6: true`，用户关闭开关后非 TUN 模式下 DNS 仍返回 AAAA（拿到 IPv6 地址却连不通，多一次失败尝试），造成两模式行为不一致。
- **落地位置**：`src-tauri/src/enhance/tun.rs::use_tun_with_sniffer`（开关值 `user_ipv6` 在 `if enable` 之前读取，供两分支共用；TUN 分支末尾做路由接管）；TUN 的 IPv6 地址常量为 `constants::tun::TUN_INET6_ADDRESS`（`fdfe:dcba:9876::1/126`）。
- **`dns.ipv6` 单一真源（2026-10-17）**：`dns.ipv6` 的**唯一写入点**是 `use_tun_with_sniffer`（TUN / 非 TUN 两分支均**无条件**写入，值恒为用户「Allow IPv6」开关）。**严禁**在 `apply_mandatory_dns_settings` 等处读顶层 `ipv6` 去兜 `dns.ipv6` —— TUN 模式下该值已被 `use_tun_with_sniffer` 强制为 `true`（防泄漏接管所需），那样兜会把用户关闭的 IPv6 静默写回开启。
- **IPv6 本地直连规则（2026-10-17 补充）**：兜底规则集必须同时覆盖 IPv4 与 IPv6 的本地网段，防止 `rule_fallback = "proxy"`（即 `MATCH,PROXY`，UI 可切）时本地/局域网流量被误转远端代理：
  - IPv4：`127.0.0.0/8`、`172.16.0.0/12`、`192.168.0.0/16`、`10.0.0.0/8`、`100.64.0.0/10`、`169.254.0.0/16`（链路本地，2026-10-17 补齐）
  - IPv6：`::1/128`、`::/128`、`fc00::/7`（ULA，**覆盖 TUN 自身段** `fdfe:dcba:9876::1/126`）、`fe80::/10`、`ff00::/8`
  - IPv4-mapped IPv6（`::ffff:a.b.c.d`）按 IPv4 私网段**逐一映射**直连（`::ffff:127.0.0.0/104`、`::ffff:10.0.0.0/104`、`::ffff:172.16.0.0/108`、`::ffff:192.168.0.0/112`、`::ffff:100.64.0.0/106`、`::ffff:169.254.0.0/112`）。**严禁**整体把 `::ffff:0:0/96` 设为 DIRECT —— 那会把国外目标一并直连、绕过代理。
- **`dns.fake-ip-range6` 与 `fc00::/7` 的硬约束**：`fake-ip-range6` 必须落在 `fc00::/7` **之外**（当前 `2001:2::0/64`，见 `src-tauri/src/enhance/tun.rs`，安全）。因 `fc00::/7` 已被设为直连，一旦改成社区常见的 ULA 取值（如 `fdfe:dcba:9877::/64`），fake IPv6 就会撞上该直连规则。改此值前必须先核对本约束。
- **DNS 劫持必须显式覆盖 TCP**：mihomo 的 `dns-hijack` **不书写协议时默认按 `udp://` 处理**，故必须写为 `["any:53", "tcp://any:53"]`；否则 TCP 53 的 DNS 查询不被劫持、改按规则转发，形成 DNS 泄漏面。
- **GFWList 模式兜底策略与禁止 `IP-CIDR6,::/0,PROXY` 全量代管（BUG-366 / v3.3.0 修正，替代 2026-10-21 旧兜底）**：
  - **移除 `MATCH,DIRECT` 前的 `IP-CIDR6,::/0,PROXY`**：在默认 GFWList 模式（`rule_fallback != "proxy"`）下，尾部仅固定注入 `GEOIP,CN,DIRECT` + `MATCH,DIRECT`，**严禁重新加入 `IP-CIDR6,::/0,PROXY`**。原 `IP-CIDR6,::/0,PROXY` 会将所有未命中 GeoIP CN 或新分配的国内 CDN/通讯/下载工具 IPv6 流量以及非 GFWList 域名的 IPv6 会话一并送入机场付费节点，造成巨额无效流量偷跑；仅当用户主动切换「代理全部流量」（`rule_fallback == "proxy"`）时，才在尾部使用 `MATCH,PROXY`。
  - **必须用 `GEOIP` 而非 `GEOIP6`**：mihomo（Clash Meta）**不支持 `GEOIP6` 规则类型**（老 Clash Premium 语法），写入即 `unsupported rule type: GEOIP6`，配置校验失败、内核拒绝启动、全部流量回落直连（v3.0.0 现场实证）。mihomo 的 `GEOIP,CN,DIRECT` 自动覆盖 IPv4/IPv6（查 mmdb 对应段）且保持**不带 `no-resolve`**（对域名先解析再判定境内 IP 直连），并须置于私网/TUN 本地直连规则之后、`MATCH` 之前。
- **`dns.fallback` 的上游必须强制走代理（2026-10-21 铁律，2026-10-23 辅 IP 对偶补齐）**：`dns.respect-rules: true` 时，内核为解析域名发出的上游连接**同样要过规则匹配**；而默认 `rule_fallback = "direct"`，于是 fallback 里的境外 DoH（`1.1.1.1` / `8.8.8.8` / `8.8.4.4`）会一路漏到 `MATCH,DIRECT`。这三家在大陆直连被墙 → fallback 整链失效 → `fallback-filter.geoip` 防投毒能力等于关闭 → 投毒应答（如 Teredo 段的 `2001::1`）被采信 → 客户端反复连不存在的地址刷 `i/o timeout` → **TUN 断流**。
  - **落地方式**：由 `enhance::dns_upstream_proxy_rules_from_hosts` 对 `dns.fallback`、`dns.nameserver` 与 `dns.nameserver-policy` 的非境内主机统一推导，生成 `IP-CIDR,<ip>/32,PROXY,no-resolve`（IPv6 则 `IP-CIDR6,<ip>/128`）或 `DOMAIN,<host>,PROXY`；同时通过 `is_local_or_domestic_dns_host` 跳过私网与国内公共 DNS（如 `223.5.5.5`、`doh.pub` 等），对已知四大公共 DNS（Google / Cloudflare / Quad9 / OpenDNS）额外补齐 BUG-302 对偶辅 IP 及已知域名落地 IP，其余自定义境外解析器按实际主机名/IP 动态跟随，不写死唯一订阅上游。
  - **插入位置**：`RULE-SET,gfwlist` 之后、用户 `append-rules` 之前。既不抢用户规则最终决定权，又早于全部私网与 `MATCH` 规则。
- **TUN 模式域名嗅探（`sniffer`）与三态可配置开关（BUG-364，v3.2.8+ 对齐）**：后台常驻浏览器持有历史直连/投毒裸 IP 绕过 Fake-IP 时，需依靠内核 `sniffer` 通过 TLS SNI / HTTP Host / QUIC SNI 还原真实域名并覆写目标地址。
  - **三态开关与非破坏合并**：UI 在基础设置卡片提供三态控制器（`自动 / 开启 / 关闭` 对应 `null / true / false`，仅在 TUN 模式下启用、切回 `自动` 时通过显式 `null` 复位字段）。处于 `自动` 态时：若订阅未显式关闭则默认注入 `sniffer.enable: true`；若订阅已显式设置 `sniffer.enable: false`，必须尊重其选择、**禁止 `revise!` 无条件覆写为 `true`**。用户显式选 `开启` 时强制启用；用户显式选 `关闭` 时写入 `sniffer.enable: false`。
  - **权威字段一致性**：开启 `sniffer` 时**权威强制写入** `force-dns-mapping: true`、`parse-pure-ip: true`，并**权威统一**顶层 `sniffer.override-destination` 与 `sniff.HTTP.override-destination`（杜绝订阅残留 `enable: true` 却关闭三项关键字段导致的空壳嗅探）；补缺 `TLS`/`QUIC`（`443`、`8443`）与 `HTTP`（`80`、`8080-8880`）嗅探端口，并扩充 `skip-domain` 与 `skip-dst-address`（覆盖回环、私网、CGNAT、ULA、链路本地、广播/组播、IPv4-mapped 私网映射段及国内明文公共 DNS，降低纯 IP 嗅探副作用）。
- **订阅 DNS 全链路统一卫生治理（`nameserver` / `fallback` / `nameserver-policy`，BUG-364 / BUG-308，v3.2.8+ 对齐）**：
  - **三域统一与保键重映射原则**：`sanitize_dns_upstreams` 统一治理 `dns.nameserver`、`dns.fallback` 与 `dns.nameserver-policy`（绝不触碰必须保留明文的 `default-nameserver`，且严禁删除 `nameserver-policy` 策略键）；是否属于境内/局域网统一以 `is_local_or_domestic_dns_host` 判定（不再以 `fallback` 列表反推境外）。当 `dns.respect-rules` 为 `false`（默认）或条目携带废弃 `#DIRECT` 标记时，将直连不可达的海外明文/加密 DNS 上游一律就地降级为从真实 `dns.nameserver` 推导的可达境内 DoH（缺省回退 `https://doh.pub/dns-query`）并去重，避免 `SERVFAIL` 或直连超时。
  - **共享推导与伪后缀剥离**：当 `dns.respect-rules: true` 时，明文境外公共 DNS 按 BUG-308 升级为 DoH，并由 `inject_dns_proxy_rules` 调用 **`dns_upstream_proxy_rules_from_hosts`** 为合并收集的境外解析器主机列表生成前置 `PROXY` 规则（插入 `RULE-SET,gfwlist` 之后、`append-rules`/私网/`MATCH` 之前）。`#DIRECT` / `#PROXY` 属内核不识别的历史伪后缀：`parse_dns_upstream` 在识别 `#DIRECT` 不可达语义后将其与 `#PROXY` 统一剥离，仅保留 `#h3=true` 等合法 Mihomo 原生参数。

---

## 💤 四、轻量级模式与窗口生命周期

### 4.1 窗口关闭即入轻量模式

当用户点击主窗口右上角的「关闭」按钮时，系统严格执行如下轻量级转换流程：
1. **立即隐藏主窗口**：拦截系统默认的关闭注销事件，第一时间调用 `window.hide()`，视觉上瞬时消失。
2. **彻底销毁 Webview2 实例**：异步调用轻量模式入口，安全释放 Webview 句柄并彻底销毁 WebView2 前端进程，立即释放其所占用的 **30MB ~ 40MB** 内存与 GPU 渲染句柄，系统仅保留 Rust 后台主线程与常驻托盘。
3. **Tauri 事件循环阻止退出铁律（v3.2.4 定型）**：隐身轻量模式下窗口已销毁，Windows 系统或 Tao 框架在 session 变动或事件投递时会分发 `RunEvent::ExitRequested`。除非用户在系统托盘菜单明确点击「退出」（`Handle::global().is_exiting() == true`），否则 Tauri 事件循环必须**无条件调用 `api.prevent_exit()`**。严禁附加诸如 `code.is_none()` 或分支状态校验，彻底杜绝进程意外退出、托盘图标消失导致后台防断流葛优守护失效。
4. **防误杀策略收敛与可观测性保留（v3.3.2+ 清理定型，v3.5.2 / BUG-395 方案升级）**：早先引入的伴生看门狗（`guardian.rs`）、`1×1` 隐形哨兵窗口（`resident_host` / `guard_host`）、Session-0 `Breakaway` 拉起链路及独立心跳/退出凭证文件已证实无效且反增误报风险，现已**彻底移除**；`v3.5.2`（BUG-395）结合 360 性能引擎机制实证升级为双路径防护：帮助菜单「🛡️ 轻量模式防误杀指南」（`AntivirusWhitelistDialog` & `docs/antivirus-whitelist.md`）提供「一键开启静默启动（全程不露主窗避开开窗残留记录 + 误杀重开静默进托盘）」与「Windows 安全模式修改 `whiteinternal.ini` 硬白名单」双方案，同时保留 `logger.rs` 兼容 `&str`/`String` 的 panic 钩子与 `monitor.rs` 事件驱动恢复日志。

### 4.2 不断网不断流铁律与后端连接维持

- **网络连接连续性铁律**：**进入轻量模式、退出轻量模式或在正常状态下自愈/手动切换节点，绝对禁止全量切断已建立的用户网络连接**。本条是 §5 章首「🎯 本章最高方针 · 连续性优先」在生命周期场景的具体落地；方针中的优先级硬序与「非必要不主动切换」同样适用于本节涉及的一切自动行为，且方针效力高于本条。
- **前后台严格二分：后台平滑保活不断连接 vs. 前台主动切换精准对齐新节点（BUG-396）**：
  - **后台自愈与冷启动/唤醒初始化**（`monitor::self_heal_with_accounting`、`trigger_backend_auto_select` 无清理标记路径）：坚持零断流热切换，绝对不关闭任何存量长连接，确保未中断的下载与 HTTP/2 / gRPC 长流不受后台偶发切换影响。
  - **前台用户显式下达换节点指令**（节点列表点选、顶栏胶囊轮换、过滤词自动选点 / `F1` 全量优选、顶栏异常胶囊在线重测）：由 `should_close_stale_proxy_connection`（Rust `monitor.rs`）与 `shouldCloseStaleProxyConnection`（前端 `path-control.ts`）精准清理满足 **`chains` 含 `"PROXY"` 且不含新 `targetNode`** 的存量代理连接（彻底清除先粘滞旧节点、后多跳切换残留的 HTTP/2 连接池粘连），同时严格保护 `DIRECT` / `REJECT` 流量以及已位于 `targetNode` 上的连接（零误伤）。点击列表闪电图标 `⚡` 与按 `F4` 属「纯延迟查看」（`select: false, cleanStaleConnections: false`），绝不切换节点亦不触碰任何存量连接（BUG-397）。
- **内核进程与虚拟网卡不间断**：内核子进程与 TUN 网卡始终保持正常全速运行，正常流量与健康长连接绝不发生瞬间闪断。

### 4.3 前端 WebSocket 熔断与自动重连自愈

- **主动切断前端数据流**：进入轻量模式的同时，后端主动熔断所有面向前端的 WebSocket 数据推送流（流量统计、运行日志、连接列表、节点状态），关闭分发协程，阻断后台空跑消耗。
- **唤醒自愈瞬连**：当用户点击托盘唤醒重新创建主窗口时，前端布局组件在挂载阶段（`useEffect`）会自动发起全新的 WS 订阅握手，数据流瞬时无感重建，无需额外的手动恢复脚本。

### 4.4 唤醒禁止配置重置（跳过 force reload）

- **轻量唤醒不是配置重载**：轻量模式唤醒仅为前端视图（Webview）的重建与呈现，内核一直处于健康工作状态。
- **禁止触发全栈重建**：唤醒流程**严禁调用任何配置重载（`reload_config(true)`）或规则全栈重建指令**，杜绝因重新加载规则导致的 5 ~ 8 秒代理中断；前端通过 LocalStorage 持久化已增强 Profile UID，比对一致时仅刷新节点延迟与高亮选中，绝不扰动内核运行态。

### 4.5 窗口唤醒几何恢复与首帧测速刷新

- **几何坐标严密还原**：唤醒创建窗口时，原子读取 `window_state.json` 中保存的最后已知宽度、高度、X、Y 坐标与最大化状态，一次性套用，杜绝窗口恢复时闪烁或回退为默认尺寸。
- **多屏拔插与脱屏保护（BUG-394 零阻塞 Win32 直查）**：窗口唤醒与激活阶段（`activate_window` / `restore_window_state`）中，`is_position_on_screen` 改为直调 Win32 `EnumDisplayMonitors` + `GetMonitorInfoW` + `GetDpiForMonitor`，`get_main_window_with_state` 与 `is_main_window_visible`/`focused`/`minimized` 改为基于原子句柄 `MAIN_WINDOW_HWND` 直调 `IsWindow` / `IsWindowVisible` / `IsIconic` / `GetForegroundWindow`（叠加 `MAIN_WINDOW_FOCUSED` 原子缓存），完全消除 `tokio` 工作线程调用 Tauri `window_getter!` 跨线程同步等待主线程的连环死锁。若因拔掉副屏等导致坐标落入盲区，立即自动 `center()` 居中重置；**该校验必须跳过「最小化状态」**：Windows 上最小化窗口矩形位于 `(-32000, -32000)`，而 `unminimize()` 未必在同一轮消息生效，故须在 `unminimize()` **之前**取样 `IsIconic`，最小化过即跳过本次校验。
- **首帧优先全量刷新**：唤醒第一微秒无条件发起代理数据全量刷新，绕过繁琐的就绪等待链，确保前端节点列表在窗口显现的第一时间即可见可用，杜绝首屏空白。

---

## 📡 五、后台监控、健康探针与自愈体系

### 🎯 本章最高方针 · 连续性优先

> **一切后台自动测速与自动选节点功能，其唯一目的是尽可能保持链路不断流，并在确已断流时把断流时间压到最短。「选出最小延迟节点」只是切换发生时顺带取得的附随收益，不构成切换的理由，更不是功能目标。非必要不主动切换节点，不主动触碰已建立的连接。**

- **适用范围**：后台监测线程、健康探针、故障自愈、初始化自动选点等**无需用户逐次确认**的自动行为。用户显式触发的 `F1` 全量优选、手动选点以及**过滤框地域约束即时切换（`700ms` 防抖/`Enter` 触发 `select: true`，见 §2.8）**不受本方针约束（此时选出合规快节点就是用户的直接意图）；而点击列表闪电图标 `⚡` 与按 `F4` 仅刷新当前可见列表延迟（`select: false`），天然不换节点、不断连接（BUG-397）。`PROXY` 控制组为 `select` 类型、`PROXY__METRICS` 测量组虽为 `url-test` / 300s 周期但不承载用户流量，故内核侧不存在自动择优切换出口（见 §5.2 / §7.1）；本方针约束的是应用层 `select_node_for_group("PROXY", ...)` 及一切后台拨测。
- **「必要」的判定标准（穷举）**：仅当 ① 当前出口节点已被确证不可用（`NodeStatus::Dead`、假/广告节点、真 HTTPS 验真失败或处于 24h 软隔离期内），或 ② 有明确证据表明维持现状将导致断流、或显著延长断流时间，才构成切换的必要性。**下列情形一律不构成必要**：别的节点延迟更低、周期测速结果波动、测速地址失真、物理网络由离线转在线、内核 Controller API 瞬时异常、窗口/轻量模式/配置生命周期变化。
- **优先级硬序**：**链路连续性 > 恢复时效 > 节点延迟**。三者冲突时一律按此顺序裁决；任何以「择优」为目的的后台切换均属违约。延迟优化只能发生在「已因上述必要性决定切换之后」的候选选择环节（现有 `min_by_key` 选点即属此类附随收益，不构成触发理由）。
- **初始化场景的具体约束**：冷启动、轻量模式唤醒、Profile 切换时的初始化自动选点，**仅当当前 `PROXY.now` 结构性失效**（空 / `DIRECT` / `REJECT` / 伪节点 / 真 HTTPS 验真软隔离节点 / 已不在新配置或当前过滤子集内）才允许切点；出口有效时只做全组测速刷新延迟、不换节点。刻意不以「没有健康延迟」「别的节点更快」为切点理由——冷启动时内核尚未拨测，`alive` / `history` 不可靠；真死节点交由后台监测经连续 2 次确认 + 死节点门检自愈，不在初始化阶段抢占判断。F1/过滤框手动选点与后台故障自愈不受此约束（⚡ / F4 仅刷新可见列表延迟，不切换节点，BUG-397）。
- **切换动作不得制造断流**：切点必须是 `select_node_for_group` 干净热切换；默认路径严禁 `close_connection`、配置重载、内核重启。确需清理存量连接时，须先确证旧节点已死且连接确已无流量，见 §4.2。
- **测量行为不得成为断流源**：后台自动测速只用于取得判活证据，不得因拨测本身扰动链路、污染 `alive` / `history` 或诱发误判；全组拨测须统一超时与互斥，判死只认本轮主/备测速 URL 实拨结果，异源测量（真 HTTPS 验真等）结果只进各自的用途状态机（见 §5.2）。
- **举证责任与上位效力**：新增或调整任何后台自动切换逻辑，必须先说明「若不切换会发生何种断流、切换如何缩短它」；无法举证者一律默认不切换。本方针是本章全部条款、参数与实现的最高裁决依据，与本章其他条款冲突时以本方针为准（早期草案 `docs/node-health-architecture.md` 已于 v3.3.4 清理移除，本文档为唯一权威规范）。

### 5.1 恒定 15 秒健康监测与防抖自愈机制

为确保在无人值守、前台关闭或轻量模式下 24/7 网络的高可用性，后台维护着高可靠守护监测线程：
- **专属守护 Tokio 运行时物理隔离（v3.5.2 / BUG-394）**：`async_handler.rs` 维护独立 2 线程守护运行时 `DAEMON_RUNTIME`（`"clash-mini-daemon"`），`monitor.rs` 通过 `AsyncHandler::spawn_daemon_supervised` 托管、`server.rs`（`127.0.0.1:33335` 单实例/PAC HTTP 服务）通过 `AsyncHandler::spawn_daemon` 托管，与 Tauri UI/命令默认 Tokio 工作池彻底物理隔离；即使极端高 CPU 下前端建窗饥饿卡死，15 秒后台监测自愈与单实例 IPC 仍独立稳健推进；同时 `NotificationSystem::send_event` 仅在 `!WindowManager::is_main_window_ready()` 时跳过窗口事件派发，避免建窗未就绪前触发跨线程等待。
- **恒定 15 秒基线周期（在线正常态）**：监测周期在轻量模式与前台窗口开启时**严格恒定为 15 秒**，不做任何放慢（代理运行与前台窗口开闭无关，节点假死应在 15 秒内被捕获）。这是三档周期中的基线档：节点异常进入重试态时收紧为 `RETRY_CHECK_INTERVAL_SECS (3s)`（见下一条），物理网络离线时为 `OFFLINE_CHECK_INTERVAL_SECS (5s)`；离线档与主循环对齐，保证 5 秒快速探测不被 15 秒主循环限制。三档统一由 `check_interval_secs(is_retry_mode, was_online)` 裁决，不存在第四种周期，标题中「恒定 15 秒」特指在线正常态。
- **重试加速（3 秒）**：当探测到当前活跃节点异常或初次失败时，周期立即动态收紧为 **3 秒**，执行高频复检。
- **防抖自愈判定与统一裁决器**：节点侧自愈严格执行**连续 2 次独立检测失败**才正式触发全节点批量选点自愈，彻底滤除网络微抖动造成的过度切换。后台监测**所有**自愈请求（主路径 / 网络恢复 / API 异常）必须构造 `SelfHealTrigger` 并交由 `decide_self_heal` 纯函数裁决，仅 `SelfHealDecision::Allow` 时方可调用 `self_heal_with_accounting`，严禁任一分支各自直接触发：① `NodeConfirmedDead` 要求 `consecutive_fails >= FAILOVER_CONFIRM_FAILS (2)`；② `NetworkRecovered` **恒判 `Wait`**——物理网络恢复只说明链路可用，不构成节点故障证据，必须安静等待并交主路径重新测量（旧实现凭一次 `evaluate_failover != NoAction` 立即自愈、且同周期主路径重复拨测，已删除）；③ `ApiUnreachable` 属**内核 API 通道故障**、不是节点侧故障，故适用独立门控：连续 `API_ERROR_HEAL_THRESHOLD (3)` 次异常后须再经 `is_clash_api_alive()` 复核，仅当 API 仍不可达才放行，复核时已恢复则清零计数并交下一周期测量。**两类故障边界明确**，不再出现「规范写连续 2 次、实现却留三条后门」的自相矛盾；BUG-318 的连续失败 / 内核重启 / 系统告警记账链路不变。
- **测速失真不记失败**：体检得出「主测速地址整体失败、备用地址可用 ⇒ 测速地址失真、节点侧正常」时，`evaluate_failover` 直接返回 `NoAction`，主循环清零 `consecutive_fails` 并复位健康态；该结论必须由 `probe_metrics_group` 以 `MetricsProbeOutcome.primary_url_distorted` 显式传回裁决层，**严禁只写日志不进裁决**。
- **三态健康迟滞与自愈门检**：`NodeStatus` 由二值健康判定扩为 `Unknown / Suspect / Dead / Alive` 四态；`alive=false` 但上一次成功拨测仍在 `SUSPECT_WINDOW_SECS (30s)` 内时判 `Suspect`（Mihomo 对失败节点只写 `alive=false`、不追加 `history`），`evaluate_failover` 直接 `NoAction`、不累计失败，只有 `Dead` 才构成自愈证据；超过窗口或时间戳不可解析（保守）才判死。自愈切点前须经 `should_proceed_with_self_heal` 用**同一权威来源**（`get_proxies() + node_status()`）复核原节点确已死，非 `Dead` 一律放弃——自愈仅兜底，不因「别的节点更快」切点（择优属 F1 手动选点）。门检仅适用于 `SelfHealOrigin::ConfirmedDead`：`FailoverVerdict::ShouldMigrate`（假/拉黑节点重选、真 HTTPS 验真失败迁移）与内核 API 通道故障由 `SelfHealOrigin` 显式隔离，避免误挡既有迁移链路；同理，经确认已死的 `ConfirmedDead` 由 `self_heal_subject_to_switch_rate_limit` 豁免切换速率上限截断（避免被迫卡死在死节点），而 `ApiChannelDown` 进入切点前须经 `should_keep_alive_on_api_recovery` 复核——若此时 API 已可读且现节点 `Alive`，直接保活现有出口并复位计数器（BUG-388）。真死节点自愈最多推迟 30 秒，由既有 20 秒宽限 + 25 秒冷却覆盖。
- **严格判死阈值（5000ms）**：测速超时硬限为 **5000ms**；延迟有效区间锁定为 `[30ms, 5000ms)`；超过 5000ms 或产生握手错误一律判死。**该值历史上在 2000ms 与 5000ms 之间反复调整过不止一次，2026-10 定型为 5000ms（BUG-304 现场双路对照监控实证）：链路拥塞簇内仅 TCP 握手即需 1.1~2.1s（SYN 丢包按 1s RTO 重传的整秒台阶指纹），叠加 TLS 握手与 204 探测往返后，「慢而可用」的活节点总耗时稳定超过 2s——2000ms 预算会把拥塞期的整组活节点误判为断流；而真死节点（机场集群故障）5s 内同样超时，不会产生误活。除非有同等量级的现场实证，禁止回调至 2000ms。**
- **切换宽限期（20 秒，v3.2.4 优化）**：任何节点切换（含自动选点与手动切换）后 20 秒内，失败判定只记日志、不计入连续失败次数，防止「切到尚未拨测的新节点 → 缓存陈旧误判 → 再次切走」的乒乓震荡。v3.2.4 从原 60 秒缩短至 20 秒，兼顾新节点状态稳定与切到不可用节点时的二次自愈时效。
- **探测按需降频与被动流量证活（BUG-379）**：体检不再固定每 60 秒对测量组全组盲测，改为基于本地管道 `/connections` 字节增量的**被动流量证活 + 停滞秒级复核 + 本轮实拨证据**分层裁决：① 任何非 `Alive`（`Suspect` / `Dead` / `Unknown`）或时间戳缺失一律强制发起 `probe_metrics_group` 全组实时拨测，且 `allow_coalesce=false` 禁用 1500ms 短窗合并，保证判死仍严格建立在本轮主/备测速 URL 实拨结果上（保留 BUG-377 异源隔离不变式）；② 当活跃节点 `NodeStatus::Alive` 且近一周期连接有真实下行回包（`Flowing`：`delta_download > 0`）时，免拨测保鲜上限延展至 `PASSIVE_TRAFFIC_MAX_SKIP_SECS (480s)`（严格小于前端 `HISTORY_FALLBACK_MAX_AGE_MS = 600s`，杜绝 UI 误显 `Timeout`），稳态拨测风暴直降 **87.5%**；③ 当存在上行请求但零下行回包（`Stalled`：`delta_upload > 0 && delta_download == 0`）时，在 `STALL_REPROBE_COOLDOWN_SECS (60s)` 允许下立即突破缓存于本轮 15s 周期内实拨复核，突发断流响应从最慢 63s 收敛至 ~18s；④ 完全空闲（`Idle`）时保鲜窗设为 `IDLE_HEALTH_FRESH_SECS (300s)` 对齐测量组自然周期（降频 80%）。严禁为追求单点测速启用 `/proxies/{name}/delay`（会破坏 §5.2 无回环铁律）。

#### 5.1.1 诊断探针（已移除，本节仅保留溯源）

v2.9.3 诊断版曾为定位「轻量模式下体检在特定时点后停止推进」（2026-10-13，存疑点为主循环卡死 / 体检门控被挡 / 拨测挂起）在主循环与体检链路注入统一前缀 `[探针]` 的 INFO 级轨迹日志，覆盖主循环轮次、体检门控、`evaluate_failover` 出入口、`delay_group` 耗时、物理探活与自愈冷却判定，仅记录轨迹、不改变控制流。**该次诊断结束后探针已全部移除**：现行 `src-tauri/src/` 与 `src/` 中不存在任何 `[探针]` 前缀日志，本节描述的注入行为已清理核验不再存在。后续临时排查仍遵循「一次性诊断物」原则——另起独立提交、定位后随修复一并移除，不得把它写回常规规范条款或长期驻留正式版本。

### 5.2 单一内核测量组（PROXY__METRICS）统一调度

- **彻底废除应用侧并发池**：废弃旧版由前端/应用层维护并发 Worker 池逐个测速的架构（不仅开销大，且在 TUN 模式下对活跃节点调 `/proxies/{name}/delay` 会引发自指回环假超时，是早期「断流 + 全 Timeout」的根因；该单代理直测路径已彻底废弃、应用侧不保留）。
- **委托内核统一测量**：全量测速、**单点测速**与选点一律委托内核内部隐式创建的 `PROXY__METRICS` 测量组（`url-test` 类型、`interval: 300`、`lazy: false`，与 `PROXY` 组完全同构；`lazy: false` 保证即使没有直连流量走过该测量组，后台探测与状态维持也能稳定生效），单次调用触发内核并发调度，由内核核心直接执行高效底层拨测。群发测速与单点测速必须取自**同一测量组、同一测速 URL、同一探针超时**（`NODE_TEST_TIMEOUT_MS` 恒等于全局统一判死阈值 `constants::timing::NODE_DELAY_MAX_MS` = 5000ms，见 5.1 节 BUG-304 定型依据，禁止回调 2000ms）。
- **单一权威健康源与异源测量隔离（BUG-377 / BUG-388）**：全仓对 `PROXY__METRICS` 的 5 处 `delay_group`（主/备测速 3 处 + 真 HTTPS 验真 2 处）必须统一取 `METRICS_PROBE_LOCK` 串行化、统一使用 `NODE_TEST_TIMEOUT_MS (5000ms)` 超时，删除验真单独设定的 6000ms。两类测量的**用途严格分离**：主/备测速 URL 结果用于健康裁决（判活 / 判死）与 UI 延迟显示；真 HTTPS 内容拉取（非 `204` 地址）只用于识别「对 generate_204 放行但真网页卡死」的假节点，结果仅写入 `node.rs` 的 verify 状态机，**不得**成为 `evaluate_failover` 的判死依据。活跃节点存在真实下行流量（`ActiveTrafficState::Flowing`）时，由 `should_allow_https_migration` 直接跳过活跃节点即时 HTTPS 验真与迁移，且迁移门槛收紧为 `HTTPS_VERIFY_MIGRATE_MIN_FAILS (2)` 次连续失败；若主/备验证 URL 均无响应或本批 `0` 个节点通过，立即进入 `HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS (600s)` 全组退避冷却避免无效探测风暴（BUG-388）。`alive` / `history` 是共享可变状态，任何调用方都会改写它，故判死只允许发生在本轮主/备 URL 实拨后重读的结果上，免拨测轮不得据其判死（见 §5.1）。验真 URL 与主测速 URL 不同属于功能需要，不违反「同一测速 URL」契约——该契约约束的是**参与健康裁决的测速**；统一的是超时、互斥与「哪一路结果拥有裁决权」。
- **单点测速四条契约**（2026-10-08 确立三条，2026-10-17 补第 4 条）：
  1. **仅测量、不切换**：单点测速必须以「不选点」方式调用，严禁改写用户当前所选节点，也绝不回写 `profile.selected`。
  2. **全组拨测属既定代价**：内核只提供「组级拨测」接口，故单节点测速一次**同样会触发对该测量组全量成员的拨测**，耗时由最慢成员决定（拥塞期可达 ≈5000ms），而结果只回报所请求的那一个节点。此为换取「TUN 下无回环、全链路数据源唯一」的既定取舍，不予优化掉。
  3. **不得伪造超时**：测速**未真正执行或未取得任何实时拨测结果**时（内核未就绪 / 无可测节点 / 读不到 PROXY 组 / 主备测速地址均整体失败 `DelayResultSource::NotExecuted`，即后端返回空结果），前端必须通过 `restoreDelaySnapshot` 原样恢复测速前的缓存快照（严格保留原始 `updatedAt` 时间戳，无原缓存则清除 `-2` 占位）并提示「测速未执行」，**严禁呈现为 Timeout，亦严禁重新盖章 `Date.now()` 虚增 30 分钟缓存寿命**；仅当测速确已执行（`LiveProbe`）而该节点未测得时，才允许以 Timeout 呈现。
  4. **禁止陈旧 history 兜底、数据来源标记与并发拨测风暴抑制（2026-10-17 确立，BUG-344 / BUG-368 收口）**：`valid_nodes.len() == 1`（用户显式单点测速）时**禁止复用任何测量组缓存/快照并禁用并发合并**，必须发起实时拨测；批量子集场景（`len > 1`）优先在 `< METRICS_REUSE_MAX_AGE_SECS`（30s）内复用最近一次完整 `probe_metrics_group` 实时快照（解决 Mihomo 对死节点不追加 `history` 导致按节点逐一判断 `all_fresh` 失效并误发整组拨测的问题），否则再校对各节点 `history` 新鲜度，命中复用时标记 `DelayResultSource::FreshMetricsCache`。故障自愈链（`self_heal_with_accounting`）允许在 `SELF_HEAL_PROBE_REUSE_MAX_AGE_SECS`（10s）内直接复用 `evaluate_failover` 二次确认刚产出的全组快照（若尚未上屏则保留首个消费方 `LiveProbe` 语义），同时全量全组拨测统一由 `METRICS_PROBE_LOCK` 串行化并在非强制复核/非单点调用上启用 `PROBE_COALESCE_WINDOW_MS`（1500ms）短窗口并发合并，彻底消除多调用方同时或背靠背向同一机场节点服务器发起多轮全组握手风暴。**全路径一律彻底废除 120s `history` 退化兜底**：一旦 `probe_metrics_group` 未取得实时数据，立即返回空结果 `NotExecuted`。同时 `verge://backend-delay-results` 事件携带 `fromProbe: bool` 语义标记：仅 `LiveProbe`（`fromProbe=true`）赋予 30 分钟实时缓存，缓存复用（`fromProbe=false`）在前端强制封顶剩余保鲜期为 30 秒。**【作用域澄清】** 本节废除的是**后端测速取数路径**的陈旧 `history` 退化兜底；前端延迟展示仍保留合法的 `history` 回落（§5.6 三级真源、`HISTORY_FALLBACK_MAX_AGE_MS = 10 分钟`），二者作用域不同，不构成规范冲突。
- **互斥与可读反馈**：单点测速、群发测速与后台自愈共用同一互斥锁（`AUTO_SELECT_RUNNING`）。互斥抢占失败（`AUTO_SELECT_BUSY`）属正常竞争，前端**必须给出可读提示**，禁止静默无响应。
- **所见即所选（纯测速与子集切换边界、切换前过滤门检，BUG-392 / BUG-397）**：点击列表闪电图标 `⚡` 或按 `F4` 仅刷新当前可见列表延迟（`select: false`），绝不改写选中节点或写回 `profile.selected`；通过过滤框输入（700ms 防抖 / Enter）自动触发测速切换（`select: true`）时，测速由内核全量执行，但最终挑出最优节点时**严格限制在用户当前界面的过滤/搜索子集内部**；全量自动优选（`F1`，`select: true`）则从全量成员中挑选。此外，`monitor::trigger_backend_auto_select_inner` 在执行 `select_node_for_group("PROXY", ...)` **切换之前**通过 `select_fastest_candidate` 实时重读 `latest_filter_text` 过滤并淘汰所有不匹配当前过滤词的候选（若整批候选均不符合最新过滤条件则安全中止切换），严禁陈旧在途测速（如抢跑前缀）先把运行时出口切往被排除节点（如 `-香港` 下的香港节点）再事后仅跳过持久化回写。选点完成后自动持久化写回 `profile.selected`。
- **后台体检实时延迟同步与 `-2` 状态寿命保护（BUG-369，v3.3.2）**：
  - `monitor::evaluate_failover` 触发 `probe_metrics_group` 取得新鲜实时延迟后，非轻量模式下通过 `Handle::notify_delay_results("PROXY", ..., true)` + `Handle::refresh_proxies()` 同步广播至前端并置 `reported_live_to_ui = true`，使断网恢复或后台体检后列表立即更新延迟而不长期显示 `-`；`lifecycle::restart_core` 完成后立即调用 `MONITOR_WAKEUP_NOTIFY.notify_one()` 唤醒首轮检测。
  - `DelayManager::setDelay(..., -2)` 在已有未过期缓存时暂存 `preTestUpdatedAt`，后续 `injectBatchResults(..., { fromProbe: false })` 据此延续测试前合法缓存寿命，避免先置流光占位的节点被误砍至 `30s` 过期并退化为 `-`；`_layout.tsx::triggerWakeupLatencyTest` 对 `-2` 测试占位补全测前快照捕获与空/异常返回回滚机制。
- **部分测速失真复核与双向 UI 延迟同步闭环（BUG-370 / BUG-387，v3.3.3 / v3.4.2）**：
  - `probe_metrics_group` 在主测速地址返回 `Ok(map)` 时通过 `should_recheck_with_fallback` 检查当前目标节点（`expected_nodes`；空集时检查非假节点集合）是否均未取得 `[30ms, 5000ms)` 有效延迟；命中“部分失真”时立即切 `secondary_test_url` 复核并由 `merge_probe_delays` 合并主备有效延迟。**【门控方向修正】** 快照只要对当前目标节点全灭，一律禁止被 `try_reuse_probe_snapshot` / `PROBE_COALESCE_WINDOW_MS` 短窗合并复用，与它是否经过主/备双地址复核无关；原 `dual_url_verified` 字段会放行「备用地址整体可用但目标节点无有效延迟」的残缺快照，已随该门控一并移除。
  - `evaluate_failover` 在非轻量模式下遵循双向显示同步不变式（**严禁改变任何切点/自愈门限**）：① `need_probe` 实拨且 `!primary_url_distorted` 时即使 `probed_delays` 为空（全员超时 `Ok({})`）也通过 `build_probe_ui_display` 推送全组 `0 (Timeout)` 并同步清空 `LAST_METRICS_PROBE`，当且仅当 `from_probe && outcome.covers_all_group_nodes` 才置位 `reported_live_to_ui = true`；② 若活跃流量呈 `ActiveTrafficState::Stalled`（有上行请求、零下行回包黑洞）且 `probe_metrics_group` 因全灭返回 `None`，在维持 `NoAction` 不乱切点前提下立即以 `from_probe = false`（`30s` 短保鲜）推送 `(active_node, 0)` + `refresh_proxies()` 消除顶栏假绿残留；③ `primary_url_distorted = true` 时仅同步真正取得有效延迟的节点，严禁将未响应节点漆成 `0 (Timeout)` 写入 30 分钟长缓存；④ 原 `LAST_SYNCED_ACTIVE_HISTORY_TIME` 升级为 `LAST_UI_ACTIVE_SYNC`（`UiActiveSyncState { node_name, history_time, reported_alive }`）：任何推向 UI 为 `Timeout (0)` 的活跃节点绝不写入旧成功 `history.last().time` 而强制记 `reported_alive = false`，使后续免拨测轮或节点复检恢复 `NodeStatus::Alive` 时哪怕 `history.last().time` 与 `LAST_NOTIFIED_HEALTH` 均未变化也能无条件回传恢复绿色延迟。
  - **主测速地址整体失败的裁决契约**：`probe_metrics_group` 返回 `MetricsProbeOutcome { delays, primary_url_distorted }`；主 URL 整体失败且备用 URL 成功时置 `primary_url_distorted = true`，`evaluate_failover` 在完成上述有效节点 UI 延迟同步与 `LAST_UI_ACTIVE_SYNC` 记账后直接返回 `NoAction`，**不得**再用重读 `get_proxies()` 的 `alive` / `history` 判死（此刻内核已将全组成员标为 not-alive）。

### 5.3 统一记账与 25 秒失败冷却保护（v3.2.4 / v3.3.2 优化）

- **自愈路径统一裁决与记账**：后台监测的全部触发分支（主路径连续 2 次失败、网络离线转在线恢复、API 连续 3 次异常）**必须先经 `decide_self_heal` 统一裁决**（规则见 §5.1），仅 `SelfHealDecision::Allow` 时才汇聚至 `self_heal_with_accounting` 统一记账函数；网络离线转在线恢复恒判 `Wait`，不再进入自愈路径，也不再于该分支额外拨测一次。自愈成功切换后**不触碰、不关闭任何存量连接**（现行规范见 §4.2），仅输出已切换与跳过清理日志，25 秒冷却与统一记账契约不变；若 `trigger_backend_auto_select_inner` 测速或验真期间用户手动切换节点（`MANUAL_SWITCH_EPOCH` 递增），切换落盘前返回 `AUTO_SELECT_MANUAL_PREEMPTED` 放弃写回，记账层与 `AUTO_SELECT_BUSY` 一同按良性让路处理（不累计失败、不加冷却惩罚）（BUG-388）。
- **切换速率上限**：`self_heal_with_accounting` 除 25 秒「**发起选点**冷却」外，新增以「**实际切换**」为单位的速率约束（通过 `self_heal_subject_to_switch_rate_limit` 作用于 `NodeMigration` / `ApiChannelDown`，而对已确证已死的 `ConfirmedDead` 豁免截断以防止卡死在死节点上，BUG-388）：两次实际切换间隔不得小于 `MIN_AUTO_SWITCH_INTERVAL_SECS (120s)`，`AUTO_SWITCH_WINDOW_SECS (600s)` 滑动窗口内最多 `MAX_AUTO_SWITCHES_PER_WINDOW (3)` 次，由纯函数 `auto_switch_permitted` 裁决。主循环维护 `recent_auto_switches: Vec<Instant>`，仅在 `outcome.selected` 且真正换到不同节点时追加、按 600 秒裁剪；`dead_node` 为空的 API 通道故障路径若经 `should_keep_alive_on_api_recovery` 检查仍不可达且执行切点，按「执行了切点」计入。超限只输出告警并跳过本次切点，不改变连续失败计数、统一记账与 BUG-318 告警链路语义。
- **25 秒失败冷却防线**：若自愈选点失败或所有节点均不可达，强制进入 **25 秒冷却期**（v3.2.4 由原 60 秒缩短至 25 秒，既杜绝风暴式并发高频测速，又显著加快连续异常时的恢复时效），冷却期内严禁重复发起自愈冲击。
- **连续 5 次系统级告警与重启终态契约**：`evaluate_self_heal_counters` 与 `complete_restart_core_attempt` 将「API 假死 3 次重启计数」与「全节点断流 5 次告警计数」解耦——API 不可达满 3 次触发 `RestartCore` 时仅重置 `api_dead_heal_fail_count`：若内核重启成功（`Ok`），保留 `auto_select_fail_count` 继续累计至第 5 次可靠触发 Win32 原生 `MessageBoxW` 系统告警并重置双计数；若内核重启自身报错（`Err`），则定义为不可恢复终态，立即升级弹出内核重启失败系统告警并重置本轮双计数（内置原子单实例门控防止模态弹窗堆叠）。

### 5.4 订阅伪节点全链路拦截防线与节点多关键词过滤同源契约（node.rs / node.ts）

针对订阅源经常恶意混入的广告与公告假节点（如包含「官网」「剩余流量」「过期时间」「续费」「公告」「购买」等条目），以及前后端节点搜索/自愈过滤子集，构建全链路共享防护与同源匹配引擎：
- **统一判定函数**：在 `src-tauri/src/utils/node.rs` 维护统一共享的 `is_dummy_node` 过滤引擎。
- **三道铁锁拦截**：
  1. **配置生成层**（`enhance::enforce_mini_agreements`）：在将订阅节点注入 `PROXY` 组时强制剥离所有伪节点，防止其进入候选列表。
  2. **状态恢复层**（`lifecycle::restore_proxy_group_now`）：回退恢复节点选择时，强行避开伪节点。
  3. **自愈决策层**（`monitor::evaluate_failover`）：挑选最优节点时，自动跳过任何伪节点，彻底斩断「内核重载默认选中伪节点 $\rightarrow$ 后台自愈切回真节点 $\rightarrow$ 重载再次切回伪节点」的死循环。
- **前后端判定必须同源（2026-10-20 确立，BUG-383 补强结构识别）**：前端 `src/utils/node.ts::isDummyNode` 必须与后端 `utils/node.rs::is_dummy_node` **逐条对齐**，且 `node.test.ts` 与后端测试用例一一对应。
  - 英文广告词（expire / traffic / website / subscribe）与 URL scheme：允许**子串**匹配。
  - 中文广告短语：必须**归一化后「完全匹配或以该短语开头」**（先剥两端包裹符号与「节点-」类通用前缀）。
  - **结构形态横幅识别（`is_structural_banner_name` / `isStructuralBannerName`，BUG-383，非穷举兜底）**：当且仅当名称**不含任何地区/专线暗示**（`!has_region_hint(name)`，涵盖国旗 Emoji、中文地区城市名及独立边界/大小写消歧的 IATA/国家地区代码与 `bgp`/`iplc`/`iepl`）时，命中以下任一结构形态即判定为伪节点：① **非纯节点域名语境的内嵌裸域名**（`has_embedded_banner_domain`：含 `:`/`：` 或非纯 ASCII 且出现不含地区后缀混写的 `<label>.<tld>`，不误杀纯 ASCII 服务器域名如 `vps1.myserver.com`）；② **冒号键值配额或日期倒计时格式**（`has_colon_quota_or_date`：首个 `:`/`：` 右侧包含 `20xx-xx`/`20xx年xx` 日期、数字配额单位 `GB/TB/MB/PB/GiB/TiB` 后不接 `[a-z]`，或 `天`/`小时`/`day`/`days` 倒计时；后端统一通过 `str::trim_start` 支持含全角空格 `\u{3000}` 在内的全 Unicode 空白，与前端 `\s*` 1:1 对齐，BUG-388）。
  - **严禁前端对中文短语用 `includes` 子串匹配**：会把 `日本群组-A`、`美国-网址专线`、`上海-群节点-01`、`HK-流量优化` 这类真实节点误判为伪节点而从前端列表隐藏，造成「**后端能选中、前端却看不到**」的高危不一致。
  - 取舍：前缀匹配会漏判「广告短语不在开头且无冒号键值/域名形态」的孤立伪节点（如 `【剩余流量】80GB`、`最新官网发布页`）。这是**有意为之**——单条目不靠穷举词表硬拆，绑在同一展示端口的多行说明则交由下方「指纹连坐分组裁决」兜底整组清理，误杀真实节点的风险降到最低。
- **节点多关键词包含与排除同源过滤（`match_node_filter` / `matchNodeFilter`，BUG-368，v3.3.2）**：
  - 后端 `src-tauri/src/utils/node.rs::match_node_filter` 与前端 `src/utils/node.ts::matchNodeFilter` **1:1 逐条同源**，分别统一接入 `monitor.rs`（恢复、判定、选点、写回 4 处）、`lifecycle.rs::select_fallback_in_subset`、`src/components/proxy/use-filter-sort.ts` 与 `src/hooks/use-profiles.ts`，严禁任何模块保留私有 `.contains` / `.includes` 分叉；
  - 语法语义（大小写不敏感）：空格分隔多个 `AND` 子句；子句内支持半角 `|` 或全角 `｜` 分隔多关键词 `OR` 匹配（自动折叠符号两侧空白，兼容 `日本 | 新加坡 | 台湾`）；以 `-`、`!` 或 `！` 开头的子句表示排除（如 `-香港`、`日本|新加坡 -倍率`）；空串、仅空白及输入态孤立符号（`-`、`!`、`|`）放行返回 `true`。
- **同配置克隆去重、指纹连坐清剿与真 HTTPS 验真状态机防线（BUG-369 / BUG-380 / BUG-383）**：
  - **规范化连接指纹、全量拓扑连坐与克隆优选（`utils/node.rs` + `enhance/mod.rs`）**：`proxy_endpoint_fingerprint` 剥离 `name` 字段后对包含合法 `server` 与 `port` 的规范化 YAML 连接字段计算 SHA-256 指纹；`enforce_mini_agreements` 在剔除伪节点**之前**先对全部有效 `server` 的原始 `proxies:` 构造 `RawProxyEntry` 交由 `resolve_proxy_clone_topology` 分组裁决：① **共享横幅端点连坐全灭**：对 `>= 2` 条共用同一指纹的分组，若组内存在任一 `is_dummy_node` 成员（或全组成员均为含 `:`/`：` 键值结构），且没有任何非伪节点成员带地区暗示（`!has_region_hint`），说明该后端专用于承载订阅横幅、无论另起何种新广告文案均整组 `0` 保留全部剔除，并将其指纹写入内存级 `RUNTIME_TAINTED_BANNER_FPS`（供 `is_confirmed_fake_fp` / `confirmed_fake_node_names` 与冷启动 `ensure_runtime_node_fingerprints_loaded` 全链路同步隐身）；② **借壳寄生真节点归真**：若同指纹组内存在带地区暗示的真节点（如横幅直接挂靠 `香港1`），则吞掉全部横幅并按优选等级 `(!colon && has_region) > has_region > !colon > fallback` 唯一保留真地区节点名称，确保用户 `-香港` 等地域排除过滤稳定生效。维持 `[PROXY, PROXY__METRICS]` 双组架构不变。
  - **单向粘滞真值 + 24h 可恢复软隔离与手动订阅解封（`node_https_verify.json`，BUG-380）**：后台 `run_https_verification_pass` 仅在存在待观察节点（既非 `verified_true` 也未处于 24h 软隔离期内）且物理网络在线时，使用非 `204` HTTPS 内容地址（主 `github.githubassets.com` favicon，备 `wikipedia.org` favicon）并发验真；**通过 1 次即永久置 `verified_true = true`**（永不降级也不再重测）；**仅当同批至少 1 个参照节点通过真 HTTPS 验真时**才对其余未通过的未验真指纹累加 `fail_count`，连续失败超过 `HTTPS_VERIFY_MAX_FAILS`（`5` 次，即 `>= 6` 轮）进入 **24 小时软隔离（`HTTPS_VERIFY_BAN_TTL_SECS = 86_400`，复用 `last_verified_at` 判定，零格式迁移）**。隔离期满后自动解除隐藏并清零重新观察；此外，用户在 UI **手动触发更新订阅**（`is_mannual_trigger == true`）时调用 `clear_unverified_https_failures()` 立即重置未验真失败记录与软隔离标记（保留 `verified_true` 白名单）。
  - **全链路择优、严格子集兜底与软隔离期间隐身**：`prioritize_verified_candidates` 与 `lifecycle::pick_subset_fallback_node` 优先从 `verified_true` 池择优；当显式设置非空 `filter_text` 时，`pick_subset_fallback_node` 仅在匹配子集内回退而严禁越界切出子集（仅当 `filter_text.trim().is_empty()` 才允许全局兜底，BUG-388）；`enhance`、`monitor`、`lifecycle` 全链路在 24h 软隔离期内跳过被隔离节点；前端通过 `get_confirmed_fake_proxy_names` 命令在 `calcuProxies` 中过滤隐藏，新进入软隔离或手动更新解封时后端同步触发 `Handle::refresh_proxies()` 刷新可见性。

### 5.5 物理网络连通性双频快速探测（TCP:53 探针，v3.2.4 优化）

- **双频独立探针**：后台常驻网络在线状态感知引擎。优先通过直连公网高可用 DNS IP（`223.5.5.5:53`、`119.29.29.29:53`、`1.1.1.1:53`）的 TCP 握手完成连通性感知，避开 TUN 模式下的域名解析死锁与代理环路，10~30ms 内完成探测；若均不通则回退到当前测速 URL 进行域名查询兜底。
- **自适应周期**：网络正常**在线时每 60 秒**探测一次；检测到**离线时收紧至每 5 秒**探测一次。
- **离线转在线瞬时唤醒**：网络由离线转为在线的第一时间清零 `consecutive_fails`、解除重试态、派发 `network_health::ok`，并把 `last_check_time` 前拨以强制本周期立即走主路径统一体检；**不再一次判定即自愈**。`SelfHealTrigger::NetworkRecovered` 在 `decide_self_heal` 中恒判 `Wait`——物理链路恢复只说明链路可用，不构成节点故障证据；即便本周期体检判死也只累计第 1 次失败，3 秒后第二次独立物理拨测确认才可能进入自愈（见 §5.1 / §5.3）。

### 5.6 批量测速触发（F4/F1）与结果统一回传

- **统一测速通道与快捷键可达性**：主界面闪电图标 `⚡` / `F4`（仅测当前可见列表延迟、不换节点、不断连接，BUG-397）、`F1`（全量测速并优选切换）以及后台自愈测速完成后，由后端以统一的 Tauri 事件向前端实时广播全量测速矩阵数据（包含成功延迟、Timeout、Error 等状态）；`disableWebViewShortcuts` 允许用户在 `<input>`/`<textarea>` 等编辑焦点中直接按下 `F4`/`F1` 触发测速与优选，同时继续保护输入框内的 `Ctrl+F` 与系统 `Alt+F4`（BUG-389）。
- **逐节点增量刷新**：前端监听接收数据后，通过动画帧增量注入缓存并驱动列表流光动效收尾，不再自行调用前端并发测速，保障全链路数据源绝对唯一。
- **延迟呈现的三级真源（严禁陈旧成功值冒充当前延迟）**：节点延迟按「本次测速结论（前端缓存）→ 内核存活判定 → history 回落」三级取值，取到即止。
  - 内核拨测失败时**只把节点标记为 `alive=false`，不追加 history**，history 会永久停在最后一次成功的那个值。故回落 history 前必须校验存活状态与记录新鲜度：已判死、或记录早于测量组 2 个 `interval`（`HISTORY_FALLBACK_MAX_AGE_MS` = 10 分钟）的，一律按 Timeout 呈现，不得显示为绿色延迟。
  - **内核 `alive` 不得覆盖已有的本次测速结论**：依 5.8，测速链路失真时内核会把全组置为 not-alive，若以它优先会把全部节点误显示为 Timeout。
  - 起因：2026-10-24 实测事故——活跃节点栏已显示 Timeout，节点列表同一节点仍显示绿色 106ms（history 里的陈旧成功值）。
- **同一节点的延迟订阅必须支持多订阅者**：活跃节点栏与节点列表项会同时订阅同一个 `PROXY::节点名`，任一订阅者的注册/注销都不得影响其它订阅者，注销必须精确到具体订阅者。此前单订阅模型下后注册者顶掉先注册者，被顶掉者永久停留在挂载时的旧值，导致「上面已 Timeout、下面还是绿的」。
- **自愈失败也必须刷新前端代理缓存**：选点成功与失败（所有节点不可达）都要向前端广播代理数据刷新，否则前端长期握着陈旧的 `alive`/`history`，界面延迟与真实连通状态脱节。
- **组头过滤/排序状态全局单例同步（BUG-369，v3.3.2）**：`useHeadStateNew` 重构为基于 `useSyncExternalStore` 的模块级单一事实源共享状态库并导出 `getGroupHeadState`，配合无实质变更 no-op 短路、本地脏键合并与即时 `localStorage` 同步及 100ms 防抖落盘（`save_proxy_head_state`），彻底消除 `_layout.tsx` 写入 `sortType` 与节点页组件写入 `filterText` 时双份独立 `useReducer` 实例互相覆盖导致过滤词（如 `-香港`）丢失的竞态。
- **过期延迟缓存主动解冻、超时缓存自新淘汰与顶栏活跃卡片响应式联动（BUG-370 / BUG-387，v3.3.3 / v3.4.2）**：`DelayManager.evictExpiredCache()` 在淘汰过期缓存条目（严禁淘汰进行中的 `-2` 测试态）时主动向对应节点/分组订阅者推送重置通知（`{ delay: -1, updatedAt: 0 }`），驱动 `useProxyDelayState` 与 `ActiveNodeStatusCard` 即时按当前 `proxy` 快照回落重算；`DelayManager.getDelayFix()` 当命中缓存中的 `Timeout`（`0` 或 `>= NODE_DELAY_MAX_MS`）但内核快照 `proxy.alive === true` 且 `proxy.history.last()` 存在严格新于 `update.updatedAt`（且未超 `HISTORY_FALLBACK_MAX_AGE_MS`）的有效绿色延迟时立即淘汰该超时缓存并直出新延迟（健康绿缓存仍严格优先于 `alive` 标记）；`injectBatchResults(..., { fromProbe: false })` 仅在注入健康有效延迟时继承既有缓存的 `updatedAt`/`preTestUpdatedAt`，注入 `Timeout` 一律封顶 `REUSED_METRICS_MAX_TTL_MS`（`30s`）；`ActiveNodeStatusCard` 同步直接响应 `activeNodeRecord` 引用变化刷新延迟展示，杜绝同名活跃节点及节点列表永久滞留假 `Timeout`。

### 5.7 无人值守资源节约与静默门控

- **连接列表静默门控与及时回收（BUG-385）**：侧边设置抽屉未展开或处于极窄窗口（`≤285px`）时，连接面板整体不挂载且连接数据订阅（`useConnectionData` / `usePathControl` 与 `<ConnectionDetail />` 内聚于 `ConnectionsPanel` 内部）彻底注销；Observer 归零后由 React Query `gcTime`（`30s`）自动回收连接缓存（不再滞留常驻 `Layout` 顶层），宽窗开启期间亦不再每秒带动整页重绘。
- **折线图渲染与 Worker 回收节流**：流量统计采样周期恒定控制为 1000ms 一次快照，在窗口最小化或不可见时挂起重绘，CPU 占用率无限趋近于 0%；宽窗流量统计 Web Worker 在缩回窄窗或隐匿停采后按 `TRAFFIC_WORKER_DESTROY_DELAY_MS`（`30s`，与 React Query `gcTime` 对齐）自动 `terminate()` 释放线程资源。

### 5.8 测速失真复核与失败服务器避让

两条针对「机场服务器抖动」的健壮性铁律。起因是 2026-10-18 实测事故：订阅 61 个节点中 **36 个挤在同一台服务器** `212.189.51.64`（仅端口不同），该机间歇性不可达（单窗口 222 次 `dial i/o timeout`），叠加机场劫持 HTTP 测速地址，导致 app 连续十三分钟反复断流。

- **测速失真复核（必做）**：整组拨测必须带「备用地址复核」——主测速地址整体失败（mihomo 返回 `all proxies timeout` 等）时，用**另一个不同域名的 HTTPS 地址**再拨测一次。
  - 只有**主备双双失败**才可认定「测速链路不可用」；此时**严禁**把「所有节点超时」当作「节点已死」——那会触发无意义的切节点抖动与内核重启。
  - **测速链路不可用时，`evaluate_failover` 必须返回 `NoAction`**（不判死、不触发自愈），等下一周期重试。依据：`delay_group` 整体失败时 mihomo 已把全组成员标记为 not-alive，据此判死即为误判。
  - **主失败、备成功时同样必须返回 `NoAction`**：此时日志判定为「主测速地址失真、节点侧正常」，该结论由 `MetricsProbeOutcome.primary_url_distorted` 传回裁决层，`evaluate_failover` 保留 UI 延迟同步后直接跳过健康裁决，主循环清零 `consecutive_fails`。原先「日志说节点正常、裁决仍按 not-alive 累加失败并触发自愈」的语义断层必须杜绝。
  - 落地：`monitor::probe_metrics_group`（主 URL = `get_test_url()`，备用 = `FALLBACK_TEST_URL`）。
- **失败服务器避让（必做）**：自愈选点**必须按「服务器」而非「节点」避让**——同一台机器常挂同一机场的多个端口节点，只避让单节点会让自愈立刻又选中同机的兄弟节点，形成死循环。
  - 活跃节点被判死（且判定基于**有效测速数据**）时，把**其所在服务器**记入冷却表（`FAILED_SERVER_COOLDOWN_SECS`，默认 10 分钟）；选点时剔除冷却表内服务器上的候选。
  - **安全阀**：若剔除后无任何候选（全在失败服务器上），必须放弃避让、回退全量——宁可切一个可疑节点，也不能无点可选。`display`（前端展示）始终是全量，不受避让影响。
  - 节点→服务器映射取自生成的运行时配置 `clash-mini.yaml` 的 `proxies` 段（mihomo `/proxies` API **不暴露 server 字段**）；解析失败则自动退化为不避让，不影响选点正确性。
  - 落地：`monitor::FAILED_SERVERS` / `record_server_failure` / `snapshot_failed_servers` / `load_node_server_map`。

---

## 🛡️ 六、安全防御与编译加固

### 6.1 Tauri 权限最小化与沙箱加固

严格遵循最小权限安全原则：
- **移除危险 Shell 与冗余窗口/WebView 权限**：仅保留 `shell:allow-open` 唤起外部链接，彻底剔除 `shell:default`、命令执行权限以及无用的窗口/WebView 创建与打印权限（`core:window:allow-create`、`core:webview:allow-create-webview*`、`core:webview:allow-print`），并清理 `migrated.json` 重复权限声明，前端环境无法执行本地指令或派生未授权窗口。
- **FS 作用域极致收紧**：文件系统插件的作用域严格限定于当前应用程序的数据目录与临时目录，严禁通过通配符跨目录读取用户私有文件。
- **彻底移除前端 HTTP 插件暴露**：已移除 `tauri-plugin-http` 与 `capabilities/desktop.json` 中全部 `http:*` 权限，WebView 侧不开放任何直接插件网络通道，外部 HTTP 请求统一收口至受 SSRF 防护的 Rust `NetworkManager` 后端。

### 6.2 用户输入 URL 严格校验与 SSRF 防护

- **网页唤起协议限制**：调用系统浏览器打开外部链接时，严格限制仅允许 `http://` 与 `https://` 协议，彻底阻断 `file://`、`javascript:`、`data:` 等危险伪协议。
- **订阅导入与更新多层 SSRF 防线**：远程订阅请求执行三层防护：① 入口静态与本地预检（`validate_url_no_ssrf` 拦截单标签主机名、`.localhost`/`.local`/`.internal`/`.lan`/`.home.arpa` 等保留后缀及回环/私网/链路本地/IPv6 ULA `fc00::/7`/IPv4-mapped 私网 IP；本地 `ToSocketAddrs` 解析返回 `Err` 时放行以便被污染订阅域名交由 `127.0.0.1:10801` 代理回退链拉取）；② 直连请求（`ProxyType::None`）挂载连接期异步 `SsrfSafeDnsResolver` 消除 DNS Rebinding TOCTOU 窗口；③ 直连与代理客户端统一挂载自定义重定向策略对每一跳 `30x` 目标再次执行 `validate_url_no_ssrf`。已知架构边界：经本地/系统 HTTP 代理隧道交由远端节点解析域名且未发生 HTTP 重定向时，客户端套接字层不可见远端最终解析 IP，由代理内核及出站节点侧隔离。

### 6.3 安全解压机制（Zip Slip 防护与单文件直写暂存）

- **单目标安全暂存策略**：内核在线升级模块（`core_updater`）从压缩包按名称精准定位唯一可执行二进制后，经 `zip::ZipArchive` 或 `flate2::read::GzDecoder` 流式直写至受控 `.staging` 临时可执行路径并校验体积与 DOS/PE 签名头，不执行多文件目录展开，从机制上杜绝路径穿越落盘（BUG-391 对齐实际实现）；诊断包打包模块（`diagnostics`）按 §8.7 强制将 ZIP 条目名规范化为正斜杠 `/` 并跳过符号链接。

### 6.4 配置增强极简化与原生 Rust 卫兵（`BUG-381`）

- **JS 扩展引擎彻底退场**：Clash Mini 坚持零手工脚本编辑器的极简架构，自 v3.3.4 起移除嵌入式 `boa_engine` JavaScript 运行时及内置 `.js` 脚本文件，原 `meta_guard.js`（`mode: script` → `rule`）与 `meta_hy_alpn.js`（`hysteria` 字符串 `alpn` → 数组）全部由原生 Rust 函数 `apply_builtin_guards` 在毫秒级无分配完成。
- **存量兼容承诺**：不再在新建配置时自动生成 `Script` 增强文件；若升级用户的 `profiles.yaml` 中仍遗留历史 `type: script` 项或旧 `.js` 文件，后端自动作为无操作（no-op）安全忽略并放行校验，绝不破坏已有配置读取。

### 6.5 编译期加固与源码保护

- **强制溢出检查（overflow-checks = true）**：在 `Cargo.toml` 的 release 编译 profile 中明确声明 `overflow-checks = true`，严禁关闭整数溢出检查，保障低层指针运算绝对内存安全。
- **关闭前端生产 Sourcemap**：在生产打包（`pnpm web:build`）阶段彻底关闭 sourcemap 生成，不仅大幅压缩安装包体积，更能防止前端源代码泄露。

---

## 📊 七、数据流中枢与协议解析

### 7.1 单一 PROXY 组模型架构

不同于原版 Clash Verge 维护的多层嵌套策略组体系，Clash Mini 在数据架构上彻底执行**单一 `PROXY` 组收敛原则**：
- **生成期单组折叠**：在配置增强阶段（`enhance`），将上游订阅中定义的所有复杂节点统一平铺提取至唯一的 `PROXY` 选择器（Selector）组，并附设隐藏的 `PROXY__METRICS` 测量组。
- **前端数据流极度纯化**：前端展示、筛选、排序、激活、延迟呈现均 100% 绑定该唯一 `PROXY` 组，彻底废除多层组的动态查找与层级维护，从根本上消除了数据不同步与渲染竞态。

### 7.2 通用协议解析器（Universal Parser）规范

为解决各订阅转换服务格式不规范、编码混乱以及新型协议的解析诉求，建立健壮的通用协议解析体系：
- **支持全协议族**：原生覆盖 `hysteria`、`hysteria2`、`anytls`、`vless`、`trojan`、`tuic`、`wireguard`、`socks5`、`http`。
- **URL 分解核心铁律（先 Query 后 Userinfo）**：
  > [!IMPORTANT]
  > **解析含有鉴权信息的 URL 时，必须遵循「先剥离 Query 参数（`?` 之后），再拆解 Userinfo（`@` 之前）」的绝对处理顺序。**
  > 若先按 `@` 拆分，当 Query 参数或密码自身含有 `@`（例如 `?sni=example.com@sni&...` 或密码含有 `@`）时，会导致主机地址被截断截失，节点解析全面崩溃。该铁律通过了严密的单元测试验证。
- **完备的 IPv6 与 CIDR 支持**：支持 IPv6 目标格式（如 `[2001:db8::1]:port`）的规范拆解与默认端口补齐；支持 WireGuard 地址中的 CIDR 格式（如 `/32`、`/128`）自动清洗。

### 7.3 订阅导入等待与 Provider 刷新协同

- **异步初始化轮询门控**：用户导入新订阅后，内核需要数秒至十几秒初始化并下载 Proxy Provider 数据；系统在此阶段启动每 1.5 秒一次的智能轮询，直至 `PROXY` 组确认包含有效真实节点才判定导入完成并刷新 UI。
- **本地直写规避节流**：就绪后将最新节点数据直接写入本地缓存中枢，规避常规请求缓存节流带来的首屏延迟。

### 7.4 订阅管理与右键上下文菜单

- **原生右键上下文菜单与误删二次确认（BUG-390）**：订阅卡片支持自定义右键菜单，提供「编辑配置」、「系统编辑器打开本地 YAML」、「复制订阅链接」、「强制更新」、「删除配置」完整交互；卡片行内的刷新/删除图标按钮统一补全 `Tooltip` 与 `aria-label`，且无论行内删除图标还是右键「删除配置」均统一经窄窗 3D 确认弹窗（`profiles.modals.confirmDelete`）二次确认后再执行物理删除；启动配置激活 3 次重试失败提示走 i18n Key（`layout.notices.profileActivationFailed`）。
- **编辑对话框换肤协同**：订阅管理与编辑弹窗（MUI Dialog）的样式深度集成 6 套拟物皮肤，边框、高光、倒角与文字反差随主题滑块实时联动。
- **剪贴板健壮交互**：订阅输入框支持全选、剪切、复制与多策略粘贴（优先使用系统剪贴板原生 API，失败平滑降级至标准 Web Clipboard），解决特殊环境下粘贴失效的问题。

### 7.5 Mihomo 插件 API 与序列化小写补丁

- **Rust 枚举字段小写反序列化补丁**：针对内核 API 将部分枚举参数以全小写形式返回导致的反序列化崩溃问题，在本地插件中维护 `#[serde(rename_all = "lowercase")]` 补丁，保证前后端类型双向严密对齐，彻底解决 Allow LAN 等基础设置开关刷新时无故回弹的顽疾。

### 7.6 远程订阅请求 User-Agent 版本解耦与兼容铁律

主流机场面板解析客户端 `User-Agent` 的唯一技术目的是**格式路由**（识别 `clash-verge` 返回完整 Clash/Mihomo YAML）与**协议代际门槛协商**（按上游官方 Clash Verge Rev 外壳版本映射推定客户端内置 `mihomo` 内核支持的新协议族）。为彻底消除自研分支版本号高频演进带来的兼容性与隐私风险，确立以下四条铁律（BUG-362）：

1. **代码单一事实源**：默认订阅兼容标识统一定义于 `src-tauri/src/constants.rs` 的 `constants::network::SUBSCRIPTION_COMPAT_USER_AGENT`（当前内置 `Mihomo v1.19.x`，对应固定值 **`"clash-verge/v2.5.10"`**），由 `src-tauri/src/utils/network.rs` 统一引用。
2. **与客户端 UI 版本号彻底解耦**：严禁在默认请求头中动态拼接 `env!("CARGO_PKG_VERSION")`（如 `clash-verge/v3.2.5`），从根源上杜绝：①「外壳虚高、内核未升」诱发机场超前下发不兼容新协议导致内核启动崩溃；② 未来涨破 `v10.0.0` 触发粗糙面板 ASCII 字典序比较归零 Bug；③ 偏离上游真实存在的主版本段（`^clash-verge/v2\.`）遭 WAF / 反代白名单误杀；④ 暴露私有分支独有版本指纹。日常 UI 迭代与发版升号**一律不改动** `SUBSCRIPTION_COMPAT_USER_AGENT`。
3. **纯净单段格式、核实依据与唯一更新触发条件**：默认标识必须保持与上游主流桌面端逐字节一致的纯净单段 `clash-verge/vX.Y.Z`（依据：已核实 GitHub 官方仓库 `clash-verge-rev/clash-verge-rev` 2026-11 正式发行版 `v2.5.10`，同代内置 `Mihomo v1.19.x` 内核；主流订阅转换及机场面板依 `clash-verge/v2.x` 下发 Mihomo YAML；权衡定值可聚类指纹与未知 `v3.x` 分支兼容性风险后，固定采用上游同代基线并以单订阅 `option.user_agent` 作为自定义逃生阀）。严禁使用易被拦截的裸 `clash.meta/...` 或追加空格后缀；**仅当未来升级替换内置 `sidecar/mini-mihomo` 内核大版本并引入新协议代际时，才同步审视调整为同代上游官方正式版号（见 §8.2 发版前检查清单与 `src-tauri/src/constants.rs` 文档注释）**。
4. **保留单订阅自定义逃生阀**：支持在单个远程订阅配置项 `option.user_agent` 中按需覆盖请求头；空串或纯空白值自动回退至 `SUBSCRIPTION_COMPAT_USER_AGENT`，且 `PrfItem::from_url` 在导入与更新时完整持久化保留非空自定义 `user_agent`。

### 7.7 路径控制精准规则生成与四态路由可逆闭环

针对主窗口右侧「路径控制」监控表与「流量接管模式 -> 添加网址」的规则下发与状态可视化，确立以下工程契约（BUG-367）：

1. **精确目标匹配铁律（严禁擅自截断域名或隐式扩域）**：
   - 在「路径控制」表右键执行「全局直连」、「代理分流」或「封锁它」时，必须严格按用户选中条目的精确目标生成 `Merge` `prepend-rules` 规则：
     - 主机域名统一生成 `DOMAIN,<exact_host>,<TARGET>`，**严禁调用 `slice(-2/-3)` 剥离子域、严禁隐式改写为 `DOMAIN-SUFFIX`**（杜绝封锁 `play.googleapis.com` 误生成 `DOMAIN-SUFFIX,googleapis.com,REJECT` 扩大打击面）；
     - 纯 IPv4 地址生成 `IP-CIDR,<ip>/32,<TARGET>`；纯 IPv6 地址（兼容方括号与 `%zone` 剥离）生成 `IP-CIDR6,<ip>/128,<TARGET>`；进程目标生成 `PROCESS-NAME,<proc>,<TARGET>`；
     - 「添加网址」批量导入中，仅当用户显式填写通配语法（`*.example.com` 或 `.example.com`）时生成 `DOMAIN-SUFFIX,example.com,PROXY`；无通配前缀的域名一律生成精确 `DOMAIN,<host>,PROXY`。
2. **先重载后切流、四态独立表达与表内可逆解封**：
   - 规则下发必须按「写入 `Merge` → `enhanceProfiles()` 内核重载 → 精确关闭当前命中该目标的活跃连接」顺序执行，保证存量长连接即刻生效切换或阻断。
   - 表格 `路由`（`chains`）列必须完整区分并展示四种状态：**`直连`（`DIRECT`）**、**`代理`（`PROXY`）**、**`封锁`（`REJECT`）**、**`已断开`（`DISCONNECTED`）**。
   - **技术边界与状态合成机制**：当连接被手动「断开此连接」或命中 `REJECT` 规则瞬时阻断后，Mihomo 活跃连接表（`/connections` WS）会移除该 socket 句柄（且客户端未主动重试前不会重建 TCP 连接）。为此前端在「路径控制」表将 Mihomo 实时连接与本地持久化的「手动已断开记录」以及 `Merge` `prepend-rules` 置顶管控条目（含历史手工写入的 `DOMAIN-SUFFIX` / `REJECT` / 置顶直连或代理规则）合成统一视图：被封锁或手动断开的链接保留在表中并显示清晰状态；当目标重建新活跃流时自动洗去「已断开」标记；用户在表内右键即可直接将其重设为 `PROXY`/`DIRECT` 或点击「恢复默认路由」删除置顶覆写规则，无需离开「路径控制」表跳转「添加网址」。
3. **单表融合近期已关闭连接（废除「活跃 / 历史」二选一开关）**：
   - `ConnectionsPanel` 移除 `活跃 / 历史` 分段切换与「清空历史」按钮；`composeActivePathControlConnections` 将 `closedConnections` 按逆序（最新关闭优先）对目标去重后合入同一路径控制表（自动排除已被实时活跃连接、手动已断开记录或 `Merge` 托管规则覆盖的目标，实时速率置零并标记 `_isClosed: true`），排于实时流与置顶托管规则之后（`_isClosed` 行对详情按关闭态展示并在右键菜单禁用「断开连接」），解决短连接一闪而过无法在主表就地右键管控的问题。
4. **自定义路由规则可移植备份与重装恢复**：
   - 在 `路径控制（右键点击链接）` 右侧提供与 `断开全部` 风格完全一致的紧凑 3D 按钮 `保存路由`（`exportCustomRoutingRules`）与 `导入路由`（`importCustomRoutingRules`），通过 `@tauri-apps/plugin-dialog` 与 `@tauri-apps/plugin-fs` 将 `Merge` `prepend-rules` 导出为带版本元数据的 UTF-8 YAML 备份文件；全新重装后可一键从指定备份文件合并导入至 `Merge` `prepend-rules`，并在操作期间通过 `beginLongOperation`/`endLongOperation` 护航完成 `enhanceProfiles()` 重载与 `clash-mini-merge-rules-changed` 界面同步刷新（13 语种全量同步）。
5. **左键详情弹窗直显实际出口节点与旧节点滞留提示（BUG-396）**：
   - 在 `ConnectionDetail`（左键点击「路径控制」表条目弹出）内，于 `主机`（`host`）正下方紧接新增 **`代理节点`（`connections.components.fields.proxyNode`）** 字段，由纯函数 `resolveConnectionNodeDisplay` 精确解析并展示 `chains[0]` 真实出口节点名（直连显 `DIRECT`、封锁显 `REJECT`、合成规则行显 `--`）；当该活跃代理连接仍驻留于后台平滑切换前的历史节点（即 `nodeName !== PROXY.now`）时，以醒目警示色追加 `（旧节点连接）`（`staleNodeTag`）标签，方便用户配合弹窗底部的「关闭连接」按需单条切断。

---

## 🔧 八、工程发布体系与规范

### 8.1 Windows 测试清单与单测执行隔离

- **Windows 应用清单与通用控件注入**：为解决 Windows 环境下直接运行 `cargo test` 因测试二进制缺失清单导致 `TaskDialogIndirect` 等 Win32 符号找不到触发 `0xC0000139` 异常崩溃的问题，工程规范确立测试命令必须使用：
  ```bash
  pnpm cargo-test
  ```
  底层通过 `CLASH_MINI_TEST_LINK=1` 注入已编译好的 Windows 资源清单库（`resource.lib`）。
- **单元测试并发与环境隔离**：所有单元测试（特别是 `save_profile`、`agreements` 等涉及配置读写与全局状态的测试）必须使用独立的测试上下文或临摹环境，严禁在并发单测中共享全局 App Handle 或共享全局 Draft 状态，保障全套主库及工作区子 crate 单元测试随时执行 100% 稳定全绿。
- **开发与单测编译防膨胀规范**：Clash Mini 为纯 Windows 桌面端，`src-tauri/Cargo.toml` 的 `[lib]` 仅保留 `crate-type = ["rlib"]`（禁止恢复移动端遗留的 `staticlib` / `cdylib` 双份链接产物）；根 `Cargo.toml` 的 `[profile.dev]` 与 `[profile.test]` 统一对齐 `codegen-units = 64`、一等自研 crate（工作区成员与本地 `tauri-plugin-mihomo`）设置 `debug = 1` 保留完整行号栈回溯、第三方外部依赖（`package."*"`）设置 `debug = false` 剥离巨型冗余 PDB 符号，确保 `cargo check` / `clippy` 与 `pnpm cargo-test` 共享同一份依赖编译缓存。

### 8.2 静默发布全流程（Silent Release）

项目发布严格执行一键化、无交互干扰的**静默发布标准作业程序（SOP）**：
- **唯一发布命令**：
  ```powershell
  .\scripts\release.ps1 <version>
  ```
- **自动化执行阶段**：
  1. **环境与令牌自检**：从 `github_token.txt` 读取 Token 注入环境变量；校验 dev 分支与工作区干净状态；校验本地 commit 全部推送完毕。
  2. **版本号原子递增与内核 UA 门禁核对**：调用 `scripts/bump_version.py` 同步递增 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 与 `updater/app-update.json`（由 `cargo check` 按需维护 `Cargo.lock` 内部包版本）；发版检查清单显式包含以下两项核对：
     - `[ ]` **UI 常规迭代发版**：确认未改动 `src-tauri/src/constants.rs` 中的 `SUBSCRIPTION_COMPAT_USER_AGENT`（详见 §7.6）；
     - `[ ]` **内置 `sidecar/mini-mihomo` 内核大版本升级发版**：同步核对并按需升级 `SUBSCRIPTION_COMPAT_USER_AGENT` 至对应协议代际的上游正式版标识。
  3. **Git 自动化流水**：原子提交 `release: bump version to <version>`，推送 `origin dev`，打 Tag `v<version>`，推送 `origin v<version>` 触发 CI。
  4. **云端 CI 轮询监测**：每 3 分钟轮询一次 Actions 任务状态，输出进度日志。
  5. **正式 Release 验收与下载**：监测到 Release 正式发布后，自动下载最新 `Clash.Mini_<version>_x64-setup.exe` 至本地 `portable_test/` 目录，验证文件大小正常（>0 MB）并输出汇总报告。

### 8.3 令牌安全与代理环境防护红线

> [!CAUTION]
> **以下为执行脚本与自动化工作流中的最高安全红线，严禁违反：**
> 1. **令牌泄露零容忍**：GitHub API Token **必须且仅能**从项目根目录 `github_token.txt` 读取，通过环境变量 `$env:GH_TOKEN` 隐式传递给 `gh` 命令行工具；**严禁将令牌内容打印至控制台、写入日志文件或作为明文字符串拼接入命令中**。
> 2. **本机代理服务保护红线**：本地测试环境处于 Clash Mini 代理接管之下，发版与测试脚本**严禁**执行任何强杀运行中代理及服务进程（`clash-mini`、`clash_verge_service`、`mini-mihomo`、`verge-mihomo`、`clash-verge`）或探测/占用其控制端口（`10801`–`10803`、`9098`、`33335`、`33336`）及命名管道（`\\.\pipe\mini-mihomo`）的操作。

### 8.4 CI 四阶段死锁无关流水线架构

GitHub Actions 官方发布流水线（`release.yml`）采用无死锁的四阶段单向依赖设计：
```mermaid
flowchart LR
  V[1. validate 校验] --> B[2. build 云端编译]
  B --> U[3. update_dev 更新配置]
  U --> P[4. publish 正式发布]
```
1. **validate**：轻量级 runner 校验 Tag 来源与版本一致性（耗时 ~1 分钟）。
2. **build**：在 Windows x64 云端环境中完整编译 Rust 与 React 前端，完成二进制签名，打包生成 NSIS 安装包并作为草稿 Release 上传（耗时 ~25-30 分钟）。
3. **update_dev**：自动获取构建产物的真实大小与 `.sig` 签名，调用脚本生成 `updater/app-update.json` 并强制推送回 `origin/dev` 分支，完成更新链闭环（耗时 ~1 分钟）。
4. **publish**：从 `Changelog.md` 中提取对应版本的更新日志，将草稿 Release 正式发布为全网公开 Release（耗时 ~1 分钟）。

### 8.5 本地制品验收与自动更新同步闭环

- **本地交付目录统一**：所有成功发布的安装包统一交付存放于根目录 **`portable_test/`** 目录（如 `portable_test/Clash.Mini_2.9.1_x64-setup.exe`），方便开发者即刻进行真机安装验证。
- **远程自动更新联动**：本地代码库随后执行 `git pull origin dev`，同步由 CI `update_dev` 任务回写的 `updater/app-update.json`，确保下一次开发迭代始终建立在最新的升级配置之上。

### 8.6 配置文件版本号与迁移框架

- **版本号独立于应用版本**：`verge.yaml` / `profiles.yaml` / `clash.yaml` 顶层带 `config_version: u32`，**缺失视为 v1**；仅在配置格式发生破坏性变更（字段重命名 / 类型变更 / 结构调整 / 默认值语义变化）时 +1。
- **读取即迁移，写入即落版**：加载时按版本**逐级**执行迁移，写回时始终写当前版本。
- **禁止降级写回**：配置版本**高于**本程序支持的版本（用户降级运行）时，只告警、尽量兼容读取，**不迁移、不写回降级**，避免把高版本配置改坏。
- **迁移函数必须是纯函数**：只改内存结构、不做 IO，且每个迁移都要有对应单测。
- **迁移前自动备份**：真正发生迁移时先备份为 `<文件名>.bak.v<from>`（同一版本只备份一次，保留最早那份原始配置）。**备份失败则中止本次迁移**（配置保持旧版本原样，下次启动重试），绝不在无备份时改写用户配置。
- **`clash.yaml` 纳入迁移框架（v2）**：`CURRENT_CLASH_CONFIG_VERSION = 2`。加载 `clash.yaml` 时同样执行对称的检查与备份；`migrate_clash_v1_to_v2` 将破坏性的 `tun.stack: "system"` 自动归一化为 `"gvisor"`（避免在某些系统环境下引起崩溃），迁移后落盘并持久化 `config_version: 2`；`template()` 默认写入当前版本。
- **`config_version` 严禁混入运行时配置**：它是 Clash Mini 自己的配置格式版本号，**不是 mihomo 配置项**。`enhance::merge_default_config` 必须在兜底分支显式跳过该键（与 `redir-port` / `tproxy-port` 同等处理）。实测内核会忽略未知顶层键（`-t` 自校验通过），但运行时配置只应含内核认识的键——否则将来内核一旦改严格解析，就会重演「配置校验失败、内核起不来」的事故。
- **版本检测必须早于模板合并（顺序铁律，勿改）**：`clash.yaml` 的加载路径是「读 Mapping → 合并 `template()` 缺失键 → 迁移」，而 `template()` 自带 `config_version = 当前版本`。若在合并**之后**才读 `config_version`，存量配置（无该键）会被合并注入成最新版，`from_version < CURRENT` 永久为假，**所有迁移静默失效**。因此版本检测统一走 `migrate::detect_config_version(&map)`，且**必须在 `template()` 合并之前调用**（`IClashTemp::new` 已按此实现，并由 `detection_must_precede_template_merge` 单测锁定）。`verge.yaml` / `profiles.yaml` 直接反序列化读取该字段、不经过模板合并，天然免疫此坑。
  - 该缺陷曾长期潜伏（v2.9.9 引入迁移框架时即存在）：`tun.stack: "system" → "gvisor"` 的 v1→v2 迁移实际从未执行过。所幸旧逻辑在 `migrated == from_version` 时不落盘，被污染的版本号**没有写回文件**，故修复后存量配置下次启动即自愈，无需数据修复。
- **落地**：`src-tauri/src/config/migrate.rs`（版本常量、`detect_config_version`、`migrate_verge` / `migrate_profiles` / `migrate_clash`、`backup_before_migration`）及 `src-tauri/src/config/clash.rs`。

### 8.7 可诊断性：日志保留与诊断包

- **日志保留约 24 小时**：`app_log_max_size`（运行时默认 `128 KB`）× `app_log_max_count`（运行时默认 `40`）≈ 5MB ≈ 26 小时（早先默认值 `8` 已由配置迁移 `v2→v3` 与运行时默认配置同步对齐至 `40`，**用户自定义的其它值一律尊重**）。
- **一键导出诊断包**：`export_diagnostic_bundle`（入口在「帮助 → 导出诊断包」）把全部轮转日志（应用主日志 + `service`/`sidecar` 内核日志）+ 脱敏配置 + 版本与运行摘要打成 zip，输出到 `<app_home>/diagnostics/`；`latest.log` 符号链接跳过不重复入包。
- **原生高亮定位与交互反馈**：诊断包导出成功后，前端通过 Tauri 命令 `show_in_folder`（Windows 原生执行 `explorer /select,"<path>"`，参数逗号后严禁有空格；已彻底收敛至 Windows 单一平台）在文件管理器中**直接选中并高亮生成的 zip 文件**，免去用户在目录中翻找；导出生成过程中前端弹出 loading 提示反馈，防止用户重复点击。
- **日志大文件尾部截断保护**：日志采集设定单文件 2MB 硬上限（`MAX_LOG_FILE_READ_BYTES`）以及 60 个日志文件采集上限（`MAX_DIAGNOSTIC_LOG_FILES`）。当日志超过 2MB 时，`read_log_file_with_tail_cap` 从尾部 seek 读取最新内容，并自动向后扫描跳过首个不完整行，在保留最关键排障日志的同时确保文件行结构完整，彻底消除内存与磁盘爆满风险。
- **脱敏是硬要求**：节点 `password` / `uuid` / `psk` / `auth` / `obfs-password`、内核 `secret` / `token`、订阅 `url` 一律替换为 `***REDACTED***`；**YAML 解析失败时只写错误说明、绝不写原始正文**（已由单测覆盖）。**日志正文同样脱敏**：按敏感字段名（`token=` / `password:` / JSON 形态等）与 URL userinfo 正则替换，无害的测速/网站 URL 原样保留排查价值（已由单测覆盖）；README 同时明示「发送前可自行解压检查」。
- **探针打点已清理**（2026-10-20）：源自断流排查的 18 处 `[探针]` INFO 打点已整体移除；关键状态变化仍由正式的 `[后台监测]` warn/info 日志覆盖。日后如需类似排障打点，用完即清，不留入生产。
- **日志采集三条硬约束（2026-10-20 补齐）**：
  1. **条目名必须用正斜杠**：Windows 下 `strip_prefix` 得到的是反斜杠路径，直接入包会生成 `service\xxx.log` 这类畸形条目（ZIP 规范要求 `/`，部分解压工具会解成字面文件名）。
  2. **条数上限按 mtime 倒序取**：`read_dir` 顺序不确定，若按任意顺序取满上限，会先塞满主日志、把最需要的 `service/` 内核日志挤掉。
  3. **符号链接一律跳过**：`latest.log` 与最新轮转文件内容重复；更关键的是**链接目录**会形成遍历环，靠条数上限兜不住（会无限压栈）。
- **认证头脱敏**：`Authorization: xxx` 整行抹除、裸 `Bearer <token>` 单独抹除，且必须在通用 KV 规则**之前**执行——否则 KV 规则只吃掉 `Bearer` 一个词，后面真正的令牌会漏出。
- **脱敏必须放阻塞线程池**：`redact_log_text` 是 CPU 密集同步正则（最坏 60 个 × 2MB），须经 `spawn_blocking` 执行，不得直接占用 tokio worker。

### 8.8 前端单测基线与最小覆盖

- **框架**：vitest（`vitest.config.mts`，`environment: node`，**刻意不加载 Tauri/Vite 插件**，配置 `@/` 路径别名映射，保证无 GUI 环境与 CI 可直接跑）。命令：`pnpm test`。
- **只测纯逻辑**：当前共 **13 个前端 Vitest 测试文件（共 126 项用例）**，覆盖节点过滤与状态（`node.test.ts`、`search-matcher.test.ts`、`use-head-state.test.ts`）、延迟计算与 WS 订阅（`delay.test.ts`、`use-mihomo-ws-subscription.test.ts`、`parse-traffic.test.ts`）、窗口几何/全局快捷键/视觉样式（`mini-window.test.ts`、`window-resizing.test.ts`、`popup-styles.test.ts`）以及订阅/路由协调（`profile-coordination.test.ts`、`profile-items.test.ts`、`path-control.test.ts`、`quick-routing.test.ts`），全面锁定纯逻辑不变式与前后端同源算法。
- **刻意不做**：暂不引入组件 / DOM / 快照测试；组件行为靠 `tsc` 与真机验证，避免为测试而测试。

### 8.9 用户可见文案 i18n 硬约束

- **一切用户可见文案必须走 `t()`**：按钮、标签、菜单项、`Tooltip`、`aria-label`、`placeholder`、`showNotice` 提示、错误边界兜底文案都算。只有两类例外：① 参与数据比较的哨兵字符串（`profile-import-card` 的「本地手动导入的代理节点」、`utils/node.ts` 的假节点关键词）；② 语言选择器里的语言自身名称（简体中文 / 日本語 …）。`console.*` 与 `debugLog` 的排障日志不在此列。
- **key 路径必须与资源层级一致**：`src/locales/<module>/<file>.json` 的根节点会拼进 key。例如 `proxies.json` 根为 `page`，资源里的 `page.a11y.nodelist` 对应的真实 key 是 `proxies.page.a11y.nodelist`。**严禁凭印象猜 key**，拿不准就看 JSON 层级或 `src/types/generated/i18n-keys.ts` 的联合类型。
- **严禁用 `@ts-expect-error` 掩盖 i18n 类型错误**：生成类型是字面量联合，key 写错会立刻报错；一旦用 `@ts-expect-error` 压掉，错误 key 会静默上线，界面直接显示 `proxies.a11y.nodelist` 这种原始 key 串（2026-10-20 实测案例：`proxy-groups.tsx` 的 `aria-label` 同时踩了「key 少一层 `page.`」和「`@ts-expect-error` 压掉类型报错」两个坑）。确属第三方类型缺陷时才允许压制，且注释必须写明「报的是哪一条、为什么不是 key 的问题」。
- **新增 key 必须 13 语种同时落地**：zh/zhtw/en/ru/fa/tt/id/ar/ko/tr/de/es/jp 一个不能少，`{{placeholder}}` 集合必须与基准语种完全一致（单位可本地化，如 ru 用 `МБ`，但占位符名不得改）。改完依次执行 `pnpm i18n:check`（missing / extra / missingSource 必须全 0）与 `pnpm i18n:types`。
- **同一句话只留一个 key**：功能与语境相同的文案不得在多模块各写一份（反例：`layout.notices.adjusting` 与已删除的 `settings.mini.adjusting` 曾同值并存）。跨模块复用的通用动作词统一收敛到 `shared.actions.*`。
- **同款 UI 必须整体覆盖**：项目里存在三处结构完全相同的「剪切/复制/粘贴/全选/清空」输入框右键菜单（`_layout.tsx`、`add-url-dialog.tsx`、`first-run-guide-dialog.tsx`），改一处必须同步改全部，禁止只改当前正在动的那一个。

### 8.10 安装程序全周期全局置顶铁律

- **置顶背景与安全目标**：在 Windows 宿主机安装或升级 Clash Mini 时，杀毒软件弹出的拦截确认窗口、系统级通知或用户其他处于激活态的前台窗口极易将安装器遮挡至后台。外行用户常因「窗口消失」误以为安装卡死而反复双击，或错过安全软件的放行确认。因此，**保证安装程序在所有过程中一直保持在窗口最上层，不得被其他窗口遮挡**，作为系统级绝对铁律。
- **全生命周期置顶防护链路**：
  1. **GUI 初始展现（`myGuiInit` / `un.myGuiInit`）**：在 Modern UI 2 界面构造第一时间，调用 `BringToFront` 并执行 Win32 API：
     ```nsis
     System::Call "user32::SetWindowPos(i $HWNDPARENT, i -1, i 0, i 0, i 0, i 0, i 3)"
     ```
     赋予 `$HWNDPARENT` 顶层窗口扩展样式（`WS_EX_TOPMOST`，`-1` = `HWND_TOPMOST`，`3` = `SWP_NOSIZE | SWP_NOMOVE`）。
  2. **卸载器对等置顶（`MUI_CUSTOMFUNCTION_UNGUIINIT`）**：卸载程序运行同样全量绑定 `un.myGuiInit`，保证反安装交互一致性。
  3. **分阶段持续强化防抢焦**：在预检（`Section EarlyChecks`）、文件解压写入（`Section Install`）以及卸载执行（`Section Uninstall`）核心代码段前置插入 `BringToFront` 与 `SetWindowPos` 重申命令，即便子进程调用（如驱动安装、VC 运行库检测等）或用户点击其他屏幕区域，安装器与卸载器始终居于最前排可见。

---

## 附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表

| 缺陷编号 | 对应章节 | 修复性质与关键技术点 |
| :--- | :---: | :--- |
| **BUG-057** | 7.3 | 导入新订阅后内核重启的智能轮询等待机制（避免前端节点列表瞬间空白） |
| **BUG-065** | 2.8 / 5.2 | 顶部活跃出口节点基于当前过滤/排序子集的实时动态循环轮换 |
| **BUG-070** | 1.3 / 1.4 | 管理员运行模式下避免异步服务重试导致的双内核进程泄漏与阻塞 |
| **BUG-071** | 3.6 | 客户端与内核自动更新的多层代理降级与私有 Release 路径定向 |
| **BUG-072** | 7.5 | 修复插件枚举反序列化缺少小写注解导致 Allow LAN 开关无故回弹的问题 |
| **BUG-073** | 2.9 | 根除高权限 Windows 下频繁更新托盘 API 引发的系统级 `E_FAIL` 崩溃（托盘静态化） |
| **BUG-074** | 3.6 | 内核在线下载与 API 请求的 Localhost → System → Direct 三级代理降级网关 |
| **BUG-075** | 5.7 | 侧边栏关闭与小窗状态下连接列表轮询彻底卸载与数据流完全静默 |
| **BUG-078** | 3.6 | 内核下载网络请求 20 秒分块读取超时与全局 300 秒防护中断机制 |
| **BUG-080** | 3.6 | 内核检查更新时的相同版本前置比对与静默提醒拦截 |
| **BUG-082** | 4.1 | 彻底废除工作集暴力修剪 API，消除 WebView2 恢复时 page fault 引发的死锁 |
| **BUG-083** | 5.1 | 后端常驻守护线程 15 秒恒定健康监测与防抖自愈机制 |
| **BUG-090** | 2.7 | Retro-3D (Trump-3D) 主题深色模式下文字与卡片反差对比度校正 |
| **BUG-091** | 7.4 | 订阅链接输入框的右键上下文菜单拦截与系统级剪贴板读写保障 |
| **BUG-092** | 5.6 | 主界面表头闪电按钮的批量测速触发与并发互斥保护 |
| **BUG-093** | 3.3 | 内核 Controller API 端口冲突自动扫描避让与密钥安全绑定 |
| **BUG-102** | 7.4 | 主布局单体大组件按单一职责原则进行模块化重构与状态下发规范 |
| **BUG-138** | 5.2 | 闪电批量测速完成后在可见子集内自动选择最快健康节点 |
| **BUG-174** | 2.7 | Retro-3D 主题中底部数据卡片文字与图标的拟物排版与反差修正 |
| **BUG-175** | 4.1 | 窗口点击关闭按钮立即隐藏并同步销毁 Webview2 进入轻量模式 |
| **BUG-176** | 5.6 | 后台测速结果统一通过 Tauri 事件实时回传前端渲染延迟数据 |
| **BUG-240** | 2.2 / 5.7 | 设置抽屉小窗降级与连接面板大窗挂载/窄窗卸载的统一尺寸门控 |
| **BUG-256** | 4.1 / 8.3 | 窗口生命周期事件通过框架统一接口拦截，规避销毁期 Panic |
| **BUG-257** | 5.1 | 确立轻量模式下后台监测周期严格恒定为 15 秒的设计规范 |
| **BUG-258** | 4.2 | 轻量模式进入时切断前端数据流，但严格维持内核长连接与代理不断流 |
| **BUG-259** | 2.5 / 4.1 | 所有窗口显示、隐藏与几何操作必须强制调度至 UI 主线程执行 |
| **BUG-262** | 4.5 | 轻量模式唤醒后主界面代理节点列表第一微秒优先全量刷新 |
| **BUG-269** | 4.3 / 8.2 | 窗口销毁时主动切断并清空前端常驻 WebSocket 协程与连接池句柄 |
| **BUG-273** | 5.3 | 后台自愈三大触发路径统一收敛至 `self_heal_with_accounting` 记账与 60s 冷却 |
| **BUG-274** | 2.5 | 窗口 Resize 几何持久化采用同步写盘并加 `>= MINIMAL` 守卫，杜绝 0×0 竞态覆写 |
| **BUG-275** | 5.4 | 订阅伪节点（广告/公告）全链路识别剔除（`node.rs`），彻底根除自愈死循环 |
| **BUG-276** | 2.4 | Win32 消息子类化挂载 `WM_SIZING` 钩子，钳制上边缘缩放至最小高度时的 Y 轴下移 |
| **BUG-277** | 7.2 | 通用协议解析器 URL 拆解顺序修正：先分离 Query 再拆 Userinfo，根治密码含 `@` 解析崩溃 |
| **BUG-278** | 3.2 | Windows 命名管道并发争用引入 `busy_retry_count = 24` 指数退避重试 |
| **BUG-279** | 8.1 | 测试命令配置 `CLASH_MINI_TEST_LINK=1` 注入 Windows 应用清单，解决单测符号缺失崩溃 |
| **BUG-280** | 8.1 | 单元测试环境隔离化改造，消除全局 App Handle 与草稿状态竞争，实现单测 100% 稳定通过 |
| **BUG-281** | 1.2 / 3.1 | 移除 Alpha 测试内核与关闭前端 sourcemap，实现工程单一 Windows 平台收敛与包体骤降 ~10MB |
| **BUG-282** | 5.2 | 单点测速改走后端统一测量组（PROXY__METRICS），消除 TUN 活跃节点回环假 Timeout 与本地 1000ms 截断 |
| **BUG-283** | 8.7 | 导出诊断包文件定位高亮（`show_in_folder`）与 2MB 尾部截断保护 |
| **BUG-284** | 8.6 | `clash.yaml` 纳入版本迁移框架（v1 → v2 自动归一化 tun.stack 为 gvisor） |
| **BUG-285** | 8.8 | 前端 Vitest 纯逻辑单测基线扩充（`node.test.ts` 与 `delay.test.ts`） |
| **BUG-286** | 2.12 | 首次安装引导向导弹窗（直接填入订阅）与就绪打开项目 Wiki 测试验证通道 |
| **BUG-287** | 8.9 | 剩余硬编码文案 i18n 收口；修正节点列表 `aria-label` 错误 key（`proxies` → `proxies.page.a11y.nodelist`）并为帮助按钮补可读名称 |
| **BUG-288** | 8.6 | `clash.yaml` 版本检测提到模板合并之前，修复 v1→v2 迁移（`tun.stack` 归一化）长期静默失效 |
| **BUG-289** | 8.7 | 诊断包脱敏补齐 `authorization` 键，覆盖 `ws-opts.headers.Authorization` 节点级令牌泄露 |
| **BUG-290** | 3.7 | IPv6 兜底去掉 `no-resolve`，修复域名目标绕过 IPv6 规则漏回 `MATCH,DIRECT` 造成的 i/o timeout |
| **BUG-291** | 3.7 | `dns.fallback` 的境外 DoH 强制走代理，修复 `respect-rules` 下 fallback 直连被墙导致防投毒链路整体失效 |
| **BUG-292** | 4.1 | 节点选择快照/还原跳过内置出站（DIRECT/REJECT），消除冷启动 `proxy not exist` 告警 |
| **BUG-293** | 2.10.1 | 开机自启改注册表方案并加启动对账/真实状态回读，修复「开关显示关却仍随系统启动」 |
| **BUG-294** | 3.7 | IPv6 兜底误用 mihomo 不支持的 `GEOIP6` 规则类型导致配置校验失败、内核拒绝启动、全部流量回落直连；改用 `GEOIP`（自动覆盖 IPv4/IPv6） |
| **BUG-295** | 2.10.1 | `enable_silent_start` 补齐 `--silent` 命令行校验，修复手动双击桌面启动被误判静默跳过创建主窗口；自启开关补齐立即乐观更新 |
| **BUG-296** | 3.7 / 4.1 | 节点快照保留内置出站传递以激活还原期子集兜底；`dns.fallback` 代理规则排除私网/环回 IP；全局代理模式下跳过 GEOIP 直连拦截保持 `MATCH,PROXY` 语义 |
| **BUG-297** | 2.3 / 2.5 | 缩放拖拽基准用 `outerSize()` 而落地用 `setSize()`（内尺寸），每帧多出外框内缩量（实测 150% DPI 为 22×13 物理像素），最小宽度下按一下左键窗口即跳宽；`setPointerCapture` 在 `await` 后取 `e.currentTarget` 得 `null` 导致捕获从未建立、拖拽一帧即失效。修复：尺寸统一内尺寸口径 + 关闭无框阴影（`shadow(false)`）+ 捕获前置同步建立 + 手势提前结束可收尾 |
| **BUG-298** | 2.10.1 | TUN 全新安装三层失效：① 安装服务后 IPC 等待窗口仅 5s（安装器收尾 + SCM 首次启动常超窗）导致首次误报失败；② 失败后 `SERVICE_MANAGER` 缓存停在 Unavailable 且 `restart_core` 不经过等服务流程，`prepare_startup_mode` 读陈旧缓存把内核静默降级 Sidecar——TUN 开关显示成功但流量不被接管；③ 降级与回退全程无日志无通知。修复：等待窗口放宽至 30s + restart 路径补锁外等服务 + 降级 WARN 留痕 + 回退前端通知 + 服务操作失败补 error 日志。**配套铁律**：`timing::SERVICE_WAIT_MAX` 是「等服务就绪的预算」（由等待循环自行换算次数），`ServiceManager::config().max_retries` 是「单次 IPC 请求失败重试次数」，二者语义不同、不得互相顶替——放宽就绪窗口必须改前者，改后者会把服务挂掉时每一次普通调用都拖成 30s。回退路径绕过 `patch_verge` 时**必须自行补发 `Handle::refresh_verge()`**，否则前端 TUN / 系统代理开关停在旧状态（显示与实际相反） |
| **BUG-299** | 2.12 | 首次引导「按回车键立即导入」仅监听输入框 `onKeyDown`，焦点一旦离开输入框（如点击提示文字）Enter 事件无人接听，表现为按了没反应。修复：Enter 监听提升到 Dialog Paper（弹窗内任意位置生效），排除 `isComposing`（IME 组词确认）与按钮焦点（放行按钮自身行为），输入框尾部补装饰性回车图标，提示文案改说明语气 |
| **BUG-300** | 2.3 | 最小高度 135 沿用带系统阴影时代的余量，关闭阴影（`shadow(false)`）后未像宽度（285→271）那样做 −14 对称修正，最小高度下底部凭空多一段空白。修复：`MIN_WINDOW_HEIGHT` / `MINIMAL_HEIGHT` / `MINI_HEIGHT_THRESHOLD` 统一 135→121。3.0.5 再 121→126：121×1.5=181.5 物理落点有取整歧义，126×1.5=189 恰为整数（主流 150% 档不再依赖容差兜底），且 125%/175% 档 MulDiv 半进位落点（158/221）回算恒 ≥ 阈值，消除「回算略小于阈值」的边缘场景 |
| **BUG-301** | 2.10.1 / 3.7 | TUN 开关显示安装成功但实际不可用（现场 80 秒全系统断网）：订阅占位假节点 `server: '1'` 名字伪装成 URL（"v0-网址:xxx"）绕过 `is_dummy_node` 纯名字过滤 → 入选 PROXY 组成员（订阅排序在首位）→ 内核重启后 Selector 落回该死节点 → TUN 下全系统流量打进死地址触发 `1:443 connect error` 风暴打满核心 CPU → 节点恢复一次性 4s IPC 超时即放弃 + 自愈监控 87s 后才救场。修复：① `enhance` 收集组成员时按 `is_valid_node_server` 校验 server 字段（IP 字面量或含点域名），无效节点不入组（定义保留在 `proxies:`，与 dummy 过滤同策略）；② `restore_proxy_group_now` 组填充等待期顺带校验目标节点在组内（不在组内属持久性错误直接兜底），select 失败改 90 秒预算重试（3s 间隔），预算耗尽再走子集兜底。**铁律**：节点名字可任意伪装，server 字段藏不住——拦占位节点必须看 server，不能只看名字 |
| **BUG-302** | 3.7 | `dns.fallback` 配置 `8.8.8.8` 或 `dns.google` 时，内核 DoH 解析并发轮询至其对偶辅 IP `8.8.4.4:443`，因缺少规则漏入 `MATCH,DIRECT` 触发直连 i/o timeout 告警。修复：现由 `dns_upstream_proxy_rules_from_hosts` 统一替代早先 `dns` 代理规则推导，针对知名公共 DNS（Google 8.8.8.8 ↔ 8.8.4.4、Cloudflare 1.1.1.1 ↔ 1.0.0.1）自动对偶注入辅 IP 规则，域名型上游同步注入 `DOMAIN,<host>,PROXY` 与已知固定落地 IP，彻底消灭 DoH 上游漏入直连超时 |
| **BUG-303** | 5.6 | 延迟呈现三级真源与节点延迟多订阅：根除「已断流节点仍显示绿色历史延迟」与「列表收不到更新永久卡旧值」 |
| **BUG-304** | 5.6 | 节点判死阈值与批量测速超时 2000ms → 5000ms：链路拥塞簇内 TCP 握手即需 1.1~2.1s（现场双路对照实证），叠加 TLS 与 204 往返后活节点总耗时超 2s 被整组误判断流；5s 预算覆盖拥塞期「慢而可用」节点，真死节点 5s 内同样超时。单一真源 `NODE_DELAY_MAX_MS` 同步驱动测速超时、判死阈值与有效延迟区间 |
| **BUG-305** | 1.3 / 2.4 / 3.1 / 5.1 | 全栈收尾审计治理（F-01~F-07, FE-01~FE-10）：① 关闭 TUN 时附加 `UpdateFlags::RESTART_CORE` 彻底释放系统服务切回 Sidecar（消除前端开启 TUN 时重复 `restartCore`）；② `setup_wm_sizing_hook` 迁入 `build_new_window` 并按 `HOOKED_HWND` 判重，根治冷启动时序竞态与轻量模式重建窗口失灵；③ 单例唤醒补齐 `clash-mini:`/`clash-verge:` 匹配与 `.query()` 百分比编码；④ `get_proxy_addr` 改用强类型派生结构体单次反序列化；⑤ 剥离零引用的 Monaco 编辑器子系统（前端 `dist` 缩减 >4.4MB）及 16 个无用 npm 依赖，延迟 Hook 惰性初始化消除卡片首帧双重重绘，清理 52 处孤立后端 i18n 键 |
| **BUG-306** | 2.2 / 5.7 | 宽窗口路径控制表格改良与首次拉宽不显示竞态修复（v3.1.6）：① `ConnectionTable` 的 `chains`（路由）列补齐 `accessorFn` 以启用 `@tanstack/react-table` 升序/降序/取消循环点击排序（活跃与历史连接通用），列宽由 50px 扩至 64px 容纳排序箭头；② 修复 `ConnectionsPanel` 外层容器 `display: block` 导致内部 `flex: 1` 失效、虚拟列表被撑高至全量高度而无法纵向滚动的布局断裂，补齐 `display: 'flex', flexDirection: 'column', height: '100%'` 滚动链路（保持隐藏滚动条、仅通过鼠标滚轮上下滚动）；③ 移除 `WindowProvider.checkMaximized` 中误丢弃末帧 `onResized` 事件的 `isWindowResizing()` 拦截（`syncGeometryAndLargeState` 纯内存计算无 IPC），并新增 `clash-mini-resize-ended` 收尾对账与 `toggleMaximize` 即时 `applyGeometry`，根治首次在设置页将窗口拉至最宽时 `isMinimalWidth` 卡在 `true` 导致右侧 `ConnectionsPanel` 不挂载的竞态 |
| **BUG-307** | 2.12 | 首次使用向导弹窗（`FirstRunGuideDialog`）针对 `271px` 极窄竖屏窗口实施主从纵向层级重构（v3.1.7）：① 弹窗标题（「欢迎使用 Clash Mini」/「订阅已就绪」）与步骤胶囊（`STEP 1 / 2` / `STEP 2 / 2`）由左右并列改为上下主从两行，消除标题被挤折行；② 输入框下方的「一键粘贴剪贴板链接」与「按 Enter 键即可导入」改为上下两行居中排布，剪贴板智能探测横幅取消单行省略截断，三大特性胶囊自适应均分撑满；③ 第二步「当前最优节点」标签（含延迟胶囊）与节点全称改为上下分行展示，彻底释放长节点名称显示空间；④ 第一步与第二步底部操作栏（`DialogActions`）统一改为 `disableSpacing` 纵向双行居中满宽排布（主操作按钮在上、次操作「稍后手动配置」/「进入主界面」在最下方一行），根除窄窗下双按钮横向挤压 |
| **BUG-308** | 3.7 / 5.2 | 订阅自带 DNS 境外明文 UDP 上游的 DoH 卫生改写（3.1.7 现场实证，v3.1.8 发布）：机场订阅在 `dns.nameserver` 自带明文 `8.8.8.8`，`respect-rules: true` 下内核自身 DNS 查询命中 `IP-CIDR,8.8.8.8/32,PROXY` 穿代理；节点为 anytls 等 TCP 承载协议时 UDP 需 UDP-over-TCP（每个查询先向节点新建 TCP 会话），该会话建立慢于境内 DoH，在内核 DNS 并发赛跑中必被取消，刷出 `failed to create session: dial tcp <node>: operation was canceled` 警告（7 条全部落在 anytls 被选中窗口内，切 ss 节点后零新增；实测确认该节点本身可翻墙，节点无责）。修复：enhance 流水线新增 `sanitize_foreign_plain_dns_upstreams`，仅将 `nameserver` 中四家境外公共 DNS（Google 8.8.8.8/8.8.4.4、Cloudflare 1.1.1.1/1.0.0.1）的明文形式（裸 IP / `IP:53`）改写为 `https://<ip>/dns-query`；`default-nameserver` 按协议定义必须明文不动、境内明文上游直连可达不动、已加密上游不动。诊断期间新增管道 API 调试脚本约定：命名管道返回 chunked 编码，需按 chunk 长度行重组后再 JSON 解析 |
| **BUG-309** | 2.12 / 3.4 | GFWList 规则集随包预置与 24h 定时更新闭环 + 新手向导步骤胶囊居中对齐（3.1.8 现场实证）：全新安装时本地 `ruleset/gfwlist.yaml` 尚未生成，需等待后台 29 节点并发测速完成切换后才拉取成功，期间用户提前点击打开 GitHub / YouTube 会因 `gfwlist` 规则数为 0 而全部漏入 `MATCH,DIRECT` 直连超时（日志实证 `dial DIRECT (match Match/) --> 20.205.243.166:443 / 172.217.112.4:443 i/o timeout`）。修复：① `scripts/prebuild.mjs` 新增 `resolveGfwlist` 任务，每次构建/发版时主动从多镜像源拉取最新 `Loyalsoldier/clash-rules` 的 `gfw.txt`（~95KB）并校验 `payload:` 后写入 `src-tauri/resources/gfwlist.yaml`；② `init_resources()` 在启动早期（内核启动前）将 `resources/gfwlist.yaml` 释放至 `<app_home_dir>/ruleset/gfwlist.yaml`（缺失/空文件或 `src_modified > dest_modified` 时复制），使全新安装第 0 毫秒即具备 6,500+ 条境外域名分流规则，彻底省去首次启动拉取等待与锁按钮交互；③ 保留 `enforce_mini_agreements` 中 `rule-providers.gfwlist` 的 `interval: 86400`（24h）+ `proxy: PROXY` 后台定时自动更新机制，且更新后的文件因 `dest_modified > src_modified` 在后续重启时自动保留；④ `FirstRunGuideDialog` 弹窗标题与 `STEP 1 / 2`、`STEP 2 / 2` 胶囊统一居中对齐 |
| **BUG-310** | 2.12 / 3.4 | 机场未引用 `rule-providers` 剥离 + 新手引导回车即导入与系统代理误报修复（3.1.9 现场实证）：① 机场原始订阅自带 `rule-providers`（如 `Local-IP.mrs`、`China-Site.mrs`、`China-IP.mrs`），虽已被 `enforce_mini_agreements` 覆写 `rules` 而零引用，但保留在 `rule-providers` 映射中仍会触发 Mihomo 首次启动联网下载 ~700KB 无用规则集；现按 `final_rules` 中 `RULE-SET,<name>` 实际引用集合自动剔除未引用的 `rule-providers`，确保 Clash Mini 自身分流所需的 `gfwlist.yaml`、`Country.mmdb`、`geosite.dat` 100% 随包预置、首次启动零外部规则拉取；② `FirstRunGuideDialog` 第一步新增 `window` 级 `Enter` 兜底监听（解决焦点落在 `document.body` 时 `Paper.onKeyDown` 收不到回车事件）与剪贴板订阅自动回填（输入框为空时按 `Enter` 或点击「立即导入并开启」自动采用已探测或实时剪贴板中的订阅 URL 并填入输入框），且输入框右侧回车图标支持点击触发导入；③ 修复 `FirstRunGuideDialog` 第二步因 `sysproxy` 仅在设置抽屉打开时才查询（引导期恒为 `undefined`）以及 TUN 模式下 `sysproxy.enable` 为 `false` 导致 100% 误报「⚠️ 系统代理未开启，请在设置中开启后再试」的缺陷，改以 `verge.enable_system_proxy || verge.enable_tun_mode || sysproxy?.enable` 判定流量接管状态，并在导入成功时若二者均未开启自动开启系统代理 |
| **BUG-311** | 2.12 / 4.2 | 新手引导（Step 2/2）主动测速择优与超时交互「🔄 重新测速并优选节点」闭环（v3.2.1）：① 修复 `handleImportProfile` 在调用 `await mutateProfiles()` 时 `isImportingRef.current` 仍为 `true`（直到 `finally` 才释放），导致新订阅 `currentProfileUid` 触发的 `useEffect` 在首行被 `if (isImportingRef.current) return` 误拦截、首次导入后跳过全节点并发测速优选的时序竞态；② `FirstRunGuideDialog` 第二步打通 `DelayManager` 的 `PROXY` 组监听与缓存直读（`getDelayUpdate` + `node.history` 双源），并在进入 Step 2/2 后若 2.5s 仍无有效延迟自动后台触发一次 `triggerAutoSelect(..., select=true)` 全节点并发测速优选；③ 当检测超时或当前节点测出超时（`0 ms`）时，将消极提示升级为建设性引导文案，并在状态卡片内展示满宽「🔄 重新测速并优选节点」按钮，点击即可当场触发全量节点并发测速并自动切换至最快节点 |
| **BUG-312** | 2.10.1 / 2.12 | 开机自启开关首点回弹竞态修复 + 新手引导 0ms 全节点选优与未测速不显节点占位（v3.2.2）：① 修复基础设置「开机自启」开关在用户首次点击时必然回弹关闭的问题：`useEffect` 监听了 `[verge?.enable_auto_launch]`，在 `patchVerge` 乐观更新瞬时触发副作用，早于异步 IPC 写入 Windows 注册表，导致 `getAutoLaunchStatus()` 读到旧注册表值 `false` 将乐观状态回滚；现增加操作互斥锁并在副作用中剥离配置依赖、仅在写入完成对账确认；② 修复新手引导 Step 2/2 一开始显示未测速首节点并报超时的问题：进入 Step 2/2 零延迟立即触发全量节点并发测速与择优（`triggerAutoSelect` `select=true`），在测速择优完成前不展示未测速的默认首节点（卡片位置保持空置，提示「正在并发测速并自动选优最快节点…」），待真实测通跑赢后才挂载「当前最优节点」与毫秒延迟胶囊 |
| **BUG-313** | 1.1 / 3.3 | [历史记录：早期多实例联调期间的端口错开特例已终止，全面回归统一实例端口与密钥标准：Mixed `10801`、SOCKS `10802`、HTTP `10803`、Controller API `9098`（`127.0.0.1:9098`）、单实例端口 `33335`（dev `33336`）、控制器密钥 `adapted-by-qiu-yuxiao`] |
| **BUG-314** | 2.2 / 2.8 / 8.10 | 节点过滤自动并发测速择优与活跃接管 + 调试日志弹窗对齐极窄窗口（271px）+ 安装程序全周期全局置顶铁律（v3.2.3）：① 节点列表过滤输入框输入完成（物理按键停止输入 700ms 防抖或 Enter 即时确认，排除 IME 组词回车与清空动作）自动触发当前过滤子集的多路并发测速与择优选择（`triggerAutoSelect` `select=true`），测速完成后最优节点自动成为当前活跃节点，且匹配结果为 0 时安全短路不触发请求；② 系统调试运行日志弹窗宽度由 800px 响应式重构为严格对齐最窄视口常数 `MIN_WINDOW_WIDTH`（271px），并在 271px 下重构标题栏、日志级别选择与清空按钮排版，实现全场景无横向滚动条自适应换行；③ 安装程序全局置顶铁律：在 `installer.nsi` 中全链路强化 `SetWindowPos(HWND_TOPMOST)` 与 `BringToFront`，覆盖 GUI 初始化（`myGuiInit` 与 `un.myGuiInit`）、安装前环境检查（`Section EarlyChecks`）、文件部署（`Section Install`）与卸载（`Section Uninstall`），确保安装/卸载全程始终保持在所有窗口最上层，绝不被任何第三方窗口遮挡 |
| **BUG-315** | 4.1 / 4.2 / 5.1 / 5.3 / 5.5 | 隐身轻量模式进程异常退出根治与后台断流自愈加速治理（v3.2.4）：① 根治隐身轻量模式下 `clash-mini.exe` 进程终止与托盘图标消失：在 Tauri 事件循环 `RunEvent::ExitRequested` 建立无条件阻止退出铁律，除非显式托盘点击「退出」（`Handle::global().is_exiting()`），否则绝对调用 `api.prevent_exit()`，消除窗口销毁或系统 session 事件意外触发退出杀死主进程与后台断流葛优守护；② 物理网络探测根治断流死锁：将 `is_physical_network_online` 从域名解析（`lookup_host`）重构为优先直连公网高可用 DNS IP（`223.5.5.5:53`、`119.29.29.29:53`、`1.1.1.1:53`）进行毫秒级 TCP 握手，避开 TUN 模式下因节点断流导致的 DNS 域名解析失败，彻底打破「节点断流 → DNS 失败 → 误判网络离线 → 暂停健康检测 → 无法自愈切点」的死锁环；③ 缩短断流自愈感知与响应延迟：节点健康新鲜度阈值（`HEALTH_DATA_FRESH_SECS`）从 60s 缩短至 20s，节点切换宽限期（`SWITCH_GRACE_PERIOD_SECS`）从 60s 缩短至 20s，自愈失败冷却保护从 60s 缩短至 25s，突发断流响应时间大幅缩短至 15~20 秒 |
| **BUG-316** | 4.1 / 4.2 / 5.1 | [已于 v3.3.3 废止移除伴生看门狗与屏外哨兵窗，仅保留白名单指引及日志恢复可观测性] 早期针对第三方内存清理软件轻量模式强杀的伴生看门狗方案（v3.2.5）：① 现场定位 `v3.2.3`/`v3.2.4` 进入轻量模式 15~20 分钟后进程零日志消失、托盘灭失的真凶为本机 `360tray.exe`（`360perfoptm2.dll` `DoOptmGroupResidual` / `FastOptm`）在整机 CPU 满载越过 `cpu_exception=80` 触发智能加速时，因主窗口 `Tauri Window` 已销毁、托盘消息窗带 `WS_EX_TOOLWINDOW` 而不被识别，将无签名 `clash-mini.exe` 判定为关窗残留进程调用 `TerminateProcess(hProcess, 99)` 外部强杀；② 新增原生零 WebView 哨兵宿主窗口（隐藏 Owner + 屏外 `1×1` 全透明穿透 `WS_POPUP | WS_VISIBLE` 归属窗口，不占任务栏与 Alt+Tab，严守 `BUG-082` 不触碰工作集修剪），使 `EnumWindows` 恒满足可见非工具窗判据，从源头阻断误杀；③ `main.rs` 前置分流 `< 1.5MB`、`0%` CPU 内核事件阻塞的 `--guardian` 伴生看门狗，正常退出 `0ms` 同步退场，遭外部强杀时同步向 `latest.log` 补写 `PID` 与 `ExitCode` 死因证据，并在轻量模式下经 `1.5s` 避让安装/卸载器后以 `--guardian-respawn` 纯静默托盘态拉起，直连运行中的系统服务内核零断流恢复防断流守护；④ `monitor.rs` 补齐 `consecutive_fails > 0 -> 0` 事件恢复闭环日志，`logger.rs` 补齐 `String` 类型 panic 载荷解析 |
| **BUG-317** | 5.5 | 网络探测与自愈：重构为并行竞争探测（每个 `TCP:53` 探针独立 `600ms` 超时预算），任一连通即刻返回 `true`；若 `TCP:53` 均不可达则在剩余预算内立即回退执行 `lookup_host` 域名解析兜底。 |
| **BUG-318** | 5.3 | 自愈记账与告警：解耦 API 失活重启计数与全节点断流告警计数（`evaluate_self_heal_counters` + `complete_restart_core_attempt`）：API 不可达满 3 次触发内核重启时若重启成功则保留告警计数继续累计至第 5 次触发系统告警；若内核重启自身失败（`Err`）则按终态立即升级触发系统告警并重置本轮双计数（含原子弹窗防重入与双分支单测锁定）。 |
| **BUG-319** | 3.6 | 内核在线升级：写入 `.staging` 临时文件，经共享 `CoreUpdater::inspect_mihomo_pe_binary` 完整校验体积（`>= 5 MB`）、DOS `MZ`、`e_lfanew`、PE32+ AMD64 COFF/OptionalHeader（`Machine=0x8664`、`Magic=0x020B`）与节表边界，并经共享 `wait_for_mini_cores_exit` 确认旧内核退出释放锁后原子替换；`state.rs`、`service.rs`、`validate.rs` 三处选核统一接入 `resolve_usable_custom_core`，严格区分结构损坏（删除回退并记日志）与瞬时 I/O 占用（保留文件回退内置 sidecar）。 |
| **BUG-320** | 7.2 | 通用协议解析器：统一遵循「先剥离 `#fragment` -> 再剥离 `?query` -> 最后按末尾 `@` 拆解 `userinfo@host:port`」顺序修复全部 9 类 URI 协议解析器，并在 `VMessJson` 中补齐 `sni` 映射至 `servername`。 |
| **BUG-321** | 2.8 / 5.2 | 过滤子集测速调度：将第二个参数修正为传入的 `names`（`triggerAutoSelect(currentUid, names, 0, select)`），并在返回空结果（未执行）或异常分支通过 `restoreDelaySnapshot` 回滚 `-2` 加载态且保留原始 `updatedAt`。 |
| **BUG-322** | 5.2 / 5.6 | 测速结果排序：将默认延迟排序键改为 `if r.1 == 0 { u32::MAX } else { r.1 }`，确保超时节点恒定排在有效节点末尾。 |
| **BUG-323** | 6.2 / 7.3 | Deep Link 订阅导入：改为标准 URL 查询参数解析提取 `url` 字段并兼容百分号编码与未编码查询边界。 |
| **BUG-324** | 3.2 | IPC 连接池超时层级：重构超时层级：连接池许可获取超时收敛为 `2s`，外层默认请求超时提升至 `5s`，确保排队等待严格短于请求总预算。 |
| **BUG-325** | 4.3 | WebSocket 断开保护：为关闭帧发送强制赋予保底超时（默认 `1000ms`，下限 `500ms`），杜绝断开流程无限期等待。 |
| **BUG-326** | 1.4 | Sidecar 进程退出同步：记录被停止子进程 PID 并轮询确认进程退出（步进 `50ms`，上限 `2000ms`，阻塞任务异常按未确认退出保守等待），进程一退出即刻放行后续启动。 |
| **BUG-327** | 6.2 | SSRF 内网域名防御：静态与预检拦截单标签主机名、内网保留后缀及 IPv6 ULA（`fc00::/7`）/IPv4-mapped 私网；`NetworkManager::get_with_ssrf_guard` 在直连路径挂载连接期异步 `SsrfSafeDnsResolver` 消除 DNS Rebinding TOCTOU，在直连与代理双路径挂载 `30x` 重定向逐跳复验；本地预解析 `Err` 放行交由 `127.0.0.1:10801` 代理回退链拉取，并明确登记远端代理隧道解析边界。 |
| **BUG-328** | 1.3 / 1.8 | 服务状态并发与 TUN 保护：`ServiceManager` 操作门控升级为异步互斥排队；`inspect_service_registration_async` 非阻塞区分 `NotInstalled`、`DisabledOrBroken`（`Start=4` 禁用或二进制缺失，直接跳过 30s 空等并持久回退）与 `InstalledAndEnabled`；已启用服务冷启动瞬时超时进入 `IVerge::enter_temp_tun_fallback` 会话锁定态，保护后续任意无关配置落盘均不覆写磁盘 `verge.yaml` 的原始 `enable_tun_mode` / `enable_system_proxy`。 |
| **BUG-329** | 3.2 | 命名管道半开连接自愈：在本地管道请求发送层针对池化复用连接且仅限幂等只读方法（`GET` / `HEAD`）失败时自动驱逐旧连接并新建物理管道透明重发 1 次；非幂等写操作绝不自动重发，杜绝重复执行。 |
| **BUG-330** | 3.2 | 命名管道繁忙退避互锁：集成 Win32 `WaitNamedPipeW` 原生事件唤醒等待并收敛单次最大等待步长，配合 `5s` 外层超时彻底消除互锁掐断。 |
| **BUG-331** | 1.4 / 1.3 | 服务模式退出孤儿内核清理：退出清理在服务模式或开启 TUN 时**优先**通过 `service::stop_core_by_service()` 提权通道停止 Session 0 内核、轮询确认进程退出并同步本地状态；提权超时/报错时回退 `CoreManager::stop_core()`，且末尾始终无条件执行 `kill_all_mini_cores()`（对无权限 Session 0 进程优雅告警放行，同时清理任意用户态残留内核）。 |
| **BUG-332** | 2.4 | 轻量唤醒窗口句柄复用：迁移至绑定窗口生命周期的 Win32 `SetWindowSubclass` 并在 `WM_NCDESTROY` 时自动卸载清理句柄记录，彻底根除 `HWND` 复用失灵。 |
| **BUG-333** | 1.1 | [`copy_clash_env` 因全栈零调用已随收尾清理连同 `feat/proxy.rs` 移除；`DEFAULT_MIXED = 10801` 默认混合端口规范现行生效] 终端环境变量端口规范：统一回退至协议默认混合端口常量 `crate::constants::network::ports::DEFAULT_MIXED`（`10801`）。 |
| **BUG-334** | 8.7 | 定时订阅更新 Panic 防护：引入 RAII Drop 守卫与 `catch_unwind` 双重屏障，确保异步任务无论正常返回、取消或 Panic 均可靠投递 `TaskFinished` 释放运行态。 |
| **BUG-335** | 4.1 / 5.4 | 内核异常重启选点恢复：当即时快照为 `None` 时自动回退读取当前激活 Profile 的 `selected` 持久化记录（过滤伪节点后）执行节点恢复。 |
| **BUG-336** | 2.8 / 5.1 | 顶栏活跃节点轮换同步：切换成功后立即乐观更新并作废 `['getProxies']` 缓存、持久化写入 `selected`，并通过 `notify_node_switched` 置位时间戳且触发专属 `NODE_SWITCH_NOTIFY.notify_one()` 第 0ms 唤醒监测循环吸收宽限期（前置在所有早退分支之前，且守护线程重启时自动复位静态状态）。 |
| **BUG-337** | 2.8 | 节点虚拟列表 Memo 稳定性：在 `useProxySelection` 内使用 `useRef` 稳定持有 `onSuccess`/`onError` 回调并固定返回函数引用，保护虚拟列表行级缓存命中率。 |
| **BUG-338** | 2.2 | 运行日志弹窗滚动抖动与劫持：移除高频 `smooth` 滚动冲突，改为仅在视口处于底部吸底状态时执行 `behavior: 'auto'` 即时跟随，用户上翻查阅时自动暂停拉拽。 |
| **BUG-339** | 5.6 | 节点延迟状态双重渲染：在 reducer 中增加字段级浅相等短路比较（数值与状态完全相同时直接返回原引用），消除挂载与无变化通知带来的冗余重绘。 |
| **BUG-340** | 2.10 / 2.11 | 浮动配置 Promise 异常兜底：为相关事件处理器中的浮动异步配置写入统一补齐 `.catch()` 错误吸收与日志记录。 |
| **BUG-341** | 1.5 / 2.2 | 271px 窄窗多语种设置排版：为网格单元赋予 `minWidth: 0` 与标签文本 `overflow: 'hidden'`, `textOverflow: 'ellipsis'` 及完整 `title` 悬浮提示，保证 13 语种在 271px 下零溢出遮挡。 |
| **BUG-342** | 8.9 | 快捷分流规则通知词条勘误：在全部 13 个语种资源中新增专属 `quickRuleAdded` 通知词条并更新 `quick-routing.ts` 调用。 |
| **BUG-343** | 2.8 | 搜索框双重广播去重：收敛搜索匹配派发路径并记录上一次已广播键值，确保单次输入或筛选项变更仅触发一次广播。 |
| **BUG-344** | 5.2 | **【直测路径已回退，并收口全路径陈旧兜底防线】** 撤回违背 §5.2 的 `delay_proxy_by_name` 直测分流；后端重建 `DelayResultSource` 语义标记并彻底移除单点/批量/后台自愈全路径的 120s `history` 兜底，经 `fromProbe` 标记约束前端 `injectBatchResults` 对 30s 复用数据的缓存寿命（封顶 30s），配合 `restoreDelaySnapshot` 保留原始 `updatedAt`。 |
| **BUG-345** | 5.5 | 并行竞速物理网络探针：使用 `futures::future::select_ok` 并行发起三路公网 DNS `TCP:53` 握手探针（单路 600ms 超时），最快一路握手成功即在 10~30ms 内判定在线，全败则秒级平滑衔接域名解析兜底。 |
| **BUG-346** | 3.6 | 内核升级原子暂存与真实 PE64 校验：引入 `.staging` 原子暂存替换流水线，经 `inspect_mihomo_pe_binary` 校验完整 PE32+ AMD64 头与节表并严格区分 `Corrupted` 与 `IoUnavailable`，经共享 `wait_for_mini_cores_exit` 确认旧内核释放文件锁后原子替换，异常自动清理 `.staging` 保留旧内核。 |
| **BUG-347** | 4.3 | 前端 WebSocket 指数退避重连：升级为带随机抖动且严格以 `15s` 封顶的指数退避；监听 `verge://refresh-clash-config`（250ms 防抖合并对齐全局契约 + `listenerEpoch` 防竞态）清空退避并对仍持有陈旧 socket 句柄的活动通道主动关闭重连，且无活动 owner 时收到 `Websocket error` 仍兜底触发重连。 |
| **BUG-348** | 7.2 | 统一 URI 协议解析预处理管道：封装共享纯函数 `strip_fragment_and_query(input)` 统一接入全部 9 类 URI 协议（`ss`/`trojan`/`vless`/`hysteria2`/`hysteria`/`anytls`/`tuic`/`wireguard`/`socks5`），并补齐全部 9 协议含 `@` 查询参数的 `server` 主机名与 `port` 双断言及 VMess `sni` 回归单测。 |
| **BUG-349** | 5.2 / 5.6 | 测速 0ms 超时置底全栈对齐与单测锁定：抽离 `sort_display_delays` 纯函数统一处理延迟升序（`0 -> u32::MAX` 垫底）与名称排序，并补充单元测试防止退化。 |
| **BUG-350** | 2.4 | 现代化 Win32 SetWindowSubclass 窗口钩子：通过 `comctl32` 原生 `{SetWindowSubclass, DefSubclassProc, RemoveWindowSubclass}` 标准子类化架构绑定窗口生命周期，并在收到 `WM_NCDESTROY` 时自动解绑释放，原生支持任意次主窗口销毁与重建。 |
| **BUG-351** | 3.2 | 韧性命名管道连接池与 WaitNamedPipeW：在 Windows 平台引入 `WaitNamedPipeW` 阻塞线程池事件等待配合退避重试，并在连接池层记录连接复用标记对幂等查询实现陈旧断开连接的无感单次重建。 |
| **BUG-352** | 1.3 / 1.8 | ServiceManager 异步互斥化状态门控：将 `ServiceManager` 内部的 `operation_running: AtomicBool` 重构为 `tokio::sync::Mutex<()>` 异步互斥锁，使并发 `refresh`/`apply` 安全排队执行而不再相互报错。 |
| **BUG-353** | 5.1 / 8.7 | 后台常驻守护任务 spawn_supervised 监督器：在 `AsyncHandler` 中新增 `spawn_supervised` 守护包装器（`AssertUnwindSafe(...).catch_unwind()` + 错误日志留痕 + 退避自重启，应用退出时自动终止），加固后台健康监测主循环。 |
| **BUG-354** | 2.12 / 8.7 | 解封常规屏幕高度下的顶栏帮助与诊断菜单：将 `max-height: 830px` 下的 `display: none` 改为重定位至标题栏紧凑槽位，让默认窗口高度（680px）与常规屏幕用户随时可访问诊断导出、首次引导与项目主页。 |
| **BUG-355** | 2.8 / 5.2 / 5.6 | F4 / F1 快捷测速择优与 Ctrl+F 搜索聚焦快捷键：并入 `disable-webview-shortcuts.ts` 单一 `document` 级 `keydown` 监听器，对称校验全量弹窗/菜单遮挡（含客户端/内核更新弹窗、列管理弹窗与帮助菜单）、输入编辑态、IME 合成与长按防重（绝不误伤 `Alt+F4`/`Ctrl+F4`），并移除 `ProxyGroups` 硬编码 `'PROXY'` 兜底。 |
| **BUG-356** | 2.8 / 2.10 | 手动点击切换代理节点 0ms 乐观高亮：点击节点瞬间对 `queryClient` 的 `['getProxies']` 缓存执行乐观 `now` 更新实现 `0ms` 即时选中高亮，后台异步提交并在极少数失败场景下自动回滚缓存。 |
| **BUG-357** | 2.8 / 5.2 | 节点卡片延迟徽章独立点击单点测速热区：将节点卡片/紧凑卡片右侧延迟徽章设为独立点击热区（`e.stopPropagation()`），点击延迟数值仅对该节点执行单点实时测速（`select=false`）而不切换当前出口，点击卡片其余区域仍立即切换节点。 |
| **BUG-358** | 2.7 | 特色皮肤双滑块拖动渲染与持久化落盘闭环：拖动过程 `onChange` 以内存态与 CSS 变量驱动 60fps 实时光影预览，结合 200ms 防抖去重落盘、`onChangeCommitted` 去重提交及失焦/`visibilitychange`/`pagehide`/卸载多路冲刷，兼顾丝滑手感与零配置丢失。 |
| **BUG-359** | 5.3 / 5.5 | 全节点超时或物理断网时顶栏轻量状态胶囊与一键自愈：后端监测在全节点不可达/物理离线及恢复时派发状态事件，顶栏展示警示胶囊，点击重测成功经 `broadcastNetworkHealth` 同步模块态，重测未执行、全超时或异常均给出明确 i18n 提示。 |
| **BUG-360** | 2.8 | 节点过滤输入框自动测速择优状态微反馈：在过滤自动测速防抖等待与执行期间为过滤框提供清晰的轻量加载/脉冲视觉指示及无障碍提示，测速择优完成后平滑收起。 |
| **BUG-361** | 2.9 | 系统托盘图标左键单击唤醒与隐藏双向切换：在不触发运行期后台轮询或菜单销毁重建的前提下，左键单击回调先判断主窗口状态：若不处于轻量模式且主窗口当前可见、未最小化且已获焦点，则平稳隐藏主窗口；否则执行唤醒恢复并置顶聚焦（托盘图标仅随代理模式切换，菜单仅更新静态勾选前缀；见 §2.9）。 |
| **BUG-362** | 7.6 / 8.2 | 远程订阅默认 `User-Agent` 版本解耦与单订阅自定义逃生阀闭环：在 `constants::network` 定义单一事实源 `SUBSCRIPTION_COMPAT_USER_AGENT = "clash-verge/v2.5.10"` 与 `CARGO_PKG_VERSION` 彻底解耦，空串自动回退，并在 `PrfItem::from_url` 中完整持久化保留 `option.user_agent`。 |
| **BUG-363** | 4.1 / 4.2 / 5.1 | [已于 v3.3.3 废止移除全部看门狗及哨兵窗口代码] 早期轻量模式伴生看门狗第一阶段方案：伴生看门狗异名异目录物理隔离（`%LOCALAPPDATA%\<APP_ID>\guard\clash-mini-guard.exe` + `--target-exe` + NSIS 安装卸载联动）、主进程与看门狗双向互锁监督重建、双端挂载主屏工作区 `1×1` `LWA_ALPHA = 1` 穿透哨兵窗口（原 `120s` 稳态代数复位已在 `BUG-365` 中废除）。 |
| **BUG-364** | 3.7 | TUN 模式 `sniffer` 三态控制与 DNS 全链路统一治理：提供 `Auto / On / Off`（`null / true / false` + `enable_tun_mode` 门控）与后端显式 `null` 复位，开启时权威强制 `force-dns-mapping` / `parse-pure-ip` 并统一顶层与 `sniff.HTTP` `override-destination`，尊重 `Auto` 态下订阅显式 `sniffer.enable: false`；`sanitize_dns_upstreams` 统一治理 `dns.nameserver` / `fallback` / `nameserver-policy`（固定保留策略键与 `default-nameserver`），以 `is_local_or_domestic_dns_host` 统一内外判定并剥离 `#DIRECT`/`#PROXY` 伪后缀（保留 `#h3=true`），按 `respect-rules` 插入境内替代 DoH 或 `PROXY` 保通规则。 |
| **BUG-365** | 4.1 / 4.2 / 5.1 / 8.7 | [已于 v3.3.3 废止移除看门狗与 Session-0 Breakaway 拉起链路] 早期 `v3.2.8` 轻量模式双进程守护加固方案（注：`cmd/diagnostics.rs` WER/日志采集仍保留只读取证能力）：Session-0 WMI (`WmiPrvSE.exe`) + Task Scheduler COM (`ITaskService`) Broker 脱离父 Job 隔离（含 `same_job_degraded` 显式告警与 Scenario 6 端到端测试）、独立 `logs/guardian.log` 账本（含写失败非静默回退与全退场/降级日志）、15s 独立 `guard/` 心跳、`MoveFileExW` + 递增序列原子替换、两阶段 `prepared`/`confirmed` 带外凭证（`pid && session_id` + 30s/120s TTL + `3500ms` 二次复核）、无时间清零的 OS 启动指纹 + 纪元会话熔断（fail-closed）及 `cmd/diagnostics.rs` 360/WER 只读取证接入。 |
| **BUG-366** | 3.7 / 4.1 / 8.5 | GFWList 模式移除 `MATCH,DIRECT` 前的 `- IP-CIDR6,::/0,PROXY` 兜底消除 IPv6 流量偷跑，清理悬空文档注释 / `.unwrap()` / 单导出冗余分支与 `read_respawn_state_checked` TOCTOU，同步对齐 `app-update.json` 说明。 |
| **BUG-367** | 2.2 / 7.7 / 8.9 | 路径控制精准目标化规则生成（禁止扩域为 `DOMAIN-SUFFIX`）与「直连 / 代理 / 封锁 / 已断开」四态单表融合，补齐自定义 `Merge` 路由 `.yaml` 一键导出保存与重装后导入恢复闭环。 |
| **BUG-368** | 2.8 / 4.2 / 5.1 / 5.2 / 5.3 / 5.4 | 修复 3s 重试复核误命中 10s 死缓存跳过二次真实拨测；引入 `METRICS_PROBE_LOCK` 与实时快照复用抑制并发/背靠背拨测风暴；故障转移成功后精确关闭仍挂死在原故障节点上的存量连接（**此项已判定为误杀健康长流，默认关闭并于 v3.3.4 彻底移除相关开关与函数，现行规范见 §4.2**）；前后端统一升级多关键词包含（`\|`/`｜`）与排除（`-`/`!`/`！`）节点过滤引擎并补齐双端单测。 |
| **BUG-369** | 2.8 / 5.2 / 5.4 / 5.6 | 修复 `DelayManager` 在 `-2` 测试态下复用注入误砍合法缓存寿命、补充唤醒测速 `-2` 快照回滚、`evaluate_failover` 体检直播延迟与 `restart_core` 立即唤醒推送；将 `useHeadStateNew` 重构为 `useSyncExternalStore` 模块级单例彻底根除 `filterText` 被并发 `sortType` 覆写；新增规范化连接指纹 `deduplicate_cloned_proxies` 同配置克隆去重与持久化真 HTTPS 验真状态机（1 次过真即单向粘滞、有在线参照通过前提下连续失败 `> 5` 次永久拉黑并在前后端全链路隐藏）。 |
| **BUG-370** | 2.8 / 5.2 / 5.6 | 修复局部测速 URL 失真、后台恢复未同步 UI 与前端延迟缓存过期冻结的三处同步缺口：`probe_metrics_group` 在非全量失效时触发备用测速 URL（`FALLBACK_TEST_URL` ↔ `DEFAULT_TEST_URL`）复测并合并最健康延迟；`evaluate_failover` 新增 `LAST_SYNCED_ACTIVE_HISTORY_TIME` 去重，确保从异常态恢复及正常 300s 体检新测量落地时及时向前端推送更新；`DelayManager::evictExpiredCache` 淘汰过期非 `-2` 缓存时主动向节点与组监听者广播 `{ delay: -1, updatedAt: 0 }` 唤醒重算，并同步修复 `useProxyDelayState` 与 `ActiveNodeCard` 的回落/切换同步（见 §5.2、§5.6）。 |
| **BUG-371** | 2.2 / 2.10 | 修复「流量接管模式（含代理方式）」与「基础设置」二选一/三选一开关操作期间与操作结果/失败反馈弹窗退化缺失问题：`notice-service` 新增 `MIN_LOADING_NOTICE_DWELL_MS`（`320ms`）最小停留防护（`markLoadingNotice` / `settleLoadingNotice`）防止快速 IPC 切换一闪而过；`NoticeManager` `zIndex` 升至 `2200` 高于 MUI Tooltip（`1500`）；`patchVerge` 与 `useServiceInstaller` 支持并发丢弃判定返回值与 `{ silentError }` 单次报错去重；补齐 `handleRuleFallbackChange`、开机自启（含注册表校验不一致告警）、启动时最小化、TUN 域名嗅探（含非 TUN 模式点击警示）及端口 `1–65535` 校验全套三段式反馈弹窗（见 §2.2、§2.10）。 |
| **BUG-372** | 4.2 / 5.1 / 5.2 / 5.3 / 5.8 | 测速失真误判自愈与周期性清连接误杀健康长流修复：`probe_metrics_group` 返回 `MetricsProbeOutcome { delays, primary_url_distorted }`，主 URL 整体失败且备用成功时 `evaluate_failover` 直接 `NoAction` 并清零失败计数；`try_reuse_probe_snapshot` / 1500ms 短窗合并改为「目标全灭一律禁止复用」并移除 `dual_url_verified`；自愈切换后不再关闭任何存量连接（相关临时开关与函数已于 v3.3.4 清理移除，见 §4.2 / §5.2）。 |
| **BUG-373** | 5.1 / 5.3 | 三条自愈入口收编统一裁决器：新增 `SelfHealTrigger` / `SelfHealDecision` / `decide_self_heal` 纯函数与 `FAILOVER_CONFIRM_FAILS` / `API_ERROR_HEAL_THRESHOLD` 常量；网络离线转在线恢复恒判 `Wait`（安静等待 + 立即重新测量，删除一次判定即自愈与同周期双拨测），API 连续异常达阈值后先经 `is_clash_api_alive()` 复核，仅仍不可达才放行；主路径连续 2 次确认改由裁决器判定，BUG-318 记账链路不变（见 §5.1 / §5.3）。 |
| **BUG-374** | 附录 A / 工程工具链 | 修复 husky `sh -e` 执行 pre-commit 时 `VAR=$(... | grep ...)` 无匹配返回 1 触发 errexit 静默终止：为 `.husky/pre-commit` 两条暂存文件探测赋值补 `|| true` 兜底，使纯文档提交、仅 agreement 提交可正常走完 `cargo make pre-commit`，并让「修改顺序铁律」提示分支恢复可达；未改动 husky 自动生成文件、未使用 `--no-verify`。 |
| **BUG-375** | 4.2 / 5.1 / 5.3 | 三态健康迟滞、自愈门检与切换速率上限：`NodeStatus` 新增 `Suspect` 与纯函数 `classify_node_status`，`alive=false` 但 30 秒内曾成功判疑似抖动而非已死；`FailoverVerdict` 新增 `ShouldMigrate`、`SelfHealOrigin` 区分三类来源，仅节点侧故障转移经「原节点确已死」门检复核；新增 120 秒最小实际切换间隔与 600 秒 / 3 次滑动窗口上限（见 §5.1 / §5.3）；存量连接清理保持移除状态。 |
| **BUG-376** | §5 最高方针 | 初始化自动选点遵守「连续性优先」：前端 `hasUsablePersistedExit` 与后端 `persisted_exit_is_usable` / `has_usable_persisted_exit` 门检使冷启动、轻量模式唤醒、Profile 切换及纯托盘启动仅在持久出口结构性失效时才允许切点，有效时只刷新延迟、不覆盖用户选择；10 秒兜底定时器不再以「没有健康延迟」为由选点；`trigger_backend_auto_select_inner` 选回同一节点时跳过 `select_node_for_group` 写入。F1/F4 手动选点与后台故障自愈不受影响（见 §5 最高方针）。 |
| **BUG-377** | §5 最高方针 / 5.1 / 5.2 | 统一后台测量源并隔离异源验真污染：删除验真独立的 6000ms 超时改用 `NODE_TEST_TIMEOUT_MS (5000ms)`，验真两次全组拨测纳入 `METRICS_PROBE_LOCK` 使 5 处 `delay_group` 全部串行化；删除 `DEAD_CONFIRM_MIN_INTERVAL_SECS` 与 3 秒强制双拨测，`should_probe_active_node` 收紧为仅「健康且 60s 内新鲜」免拨测、体检拨测禁用 1500ms 短窗合并，判死只建立在本轮主/备 URL 实拨结果上，杜绝验真写入的 `[5000, 6000)` 越界延迟 + 新鲜时间戳在免拨测轮被误判为节点已死并触发自愈切点。三态迟滞、连续 2 次裁决器、死节点门检与切换速率上限保持不变（见 §5.1 / §5.2）。 |
| **BUG-378** | §5 最高方针 / 5.1 / 5.2 / 5.5 / 5.6 | 文档治理：消除 S1–S5 落地后的规范内部矛盾。§5.5「离线转在线秒级触发自愈」改写为恒判 `Wait`；§5.1 补齐 15 / 3 / 5 三档周期并把已不存在的 `[探针]` 章节改为溯源说明；§5.2 澄清废除的是后端测速取数兜底、与 §5.6 前端 `history` 回落作用域不同（原历史草案 `docs/node-health-architecture.md` 已于 v3.3.4 随宪章收口彻底清理移除）。 |
| **BUG-379** | 5.1 / 5.2 | 终结后台 60s 全组盲测风暴并维持 §5.2 无回环铁律：`monitor.rs` 引入基于本地 `/connections` 字节增量的被动流量证活状态机——活跃下行传输（`Flowing`）免拨测上限延展至 `PASSIVE_TRAFFIC_MAX_SKIP_SECS (480s < 600s)`、完全空闲（`Idle`）对齐测量组周期 `IDLE_HEALTH_FRESH_SECS (300s)`、有出无回黑洞（`Stalled`）带 `STALL_REPROBE_COOLDOWN_SECS (60s)` 冷却立即实拨复核；非 `Alive` 仍强制实拨，保留 BUG-377 异源隔离不变式。 |
| **BUG-380** | §5 最高方针 / 5.4 | 真 HTTPS 验真由不可逆永久拉黑改为 24h 可恢复软隔离：`NodeVerifyRecord::is_confirmed_fake_at` 复用 `last_verified_at` 实施 `HTTPS_VERIFY_BAN_TTL_SECS (86_400)` TTL 期满自动缓刑重测；`feat/profile.rs::update_profile` 在用户手动更新订阅时调用 `clear_unverified_https_failures()` 立即重置未验真失败记录（保留 `verified_true` 白名单）并刷新列表；同步在 §2.8/§5 明确记录过滤框 700ms 地域策略即时切换特例豁免。 |
| **BUG-381** | 2.12 / 6.4 | 移除重型嵌入式 JS 引擎 `boa_engine` 与 `enhance/script.rs` / 内置 `.js` 文件，改用原生 Rust `apply_builtin_guards` 完成 `mode: script` 归一与 `hysteria` string `alpn` 转数组；停止新建 `Script` 默认项，保留存量 `profiles.yaml` 历史 `type: script` 兼容忽略。 |
| **BUG-382** | 2.11 / 6.1 | 清理不可达流媒体解锁 `/unlock` 路由（`src/pages/unlock.tsx`、`cmd/media_unlock_checker` 及废弃翻译键）与零使用 `tauri-plugin-http` / `@tauri-apps/plugin-http`，收紧 WebView 网络接口暴露并消除重复 `reqwest v0.12.28` 依赖树。 |
| **BUG-383** | 2.8 / 5.4 | 根治借壳真实可用后端（如香港出口）的订阅横幅假节点漏网并绕过用户 `-香港` 排除过滤问题：前后端 1:1 增设非穷举结构形态判定（`is_structural_banner_name` / `isStructuralBannerName`：无地区暗示下的内嵌裸域名与冒号键值配额/日期格式），后端 `enforce_mini_agreements` 改在剔除前先行按 `RawProxyEntry` 指纹分组执行 `resolve_proxy_clone_topology`——共享横幅专用端点整组连坐 `0` 保留并登记内存 `RUNTIME_TAINTED_BANNER_FPS`，寄生真地区节点组强制折叠归真为带地区名的节点，令真实香港借壳体稳妥受控于地域过滤与排除规则。 |
| **BUG-384** | 2.7 | 修复 Monochrome（黑白极简工业风）皮肤下「基础设置」卡片四枚 `size="small"` 小开关仍残留旧 `56×28` 圆角胶囊样式问题：`base-switch.tsx` 导出 `getSwitchSkinStyle` 并抽出共享直角方形构造器 `buildSquareSwitchStyle`，使 `monochrome` 与 `cyberpunk` 对齐 `28×14` / `sizeSmall` `18×10`、`borderRadius: 0`（滑块 `0px`）的小巧直角方形造型与高反差黑白灰态区分。 |
| **BUG-385** | 2.2 / 5.7 | 补齐隐藏组件彻底卸载、30 秒绘图 Worker 延迟回收与关闭设置面板时的中间态竞态防护：设置抽屉开启时同步卸载被全屏遮挡的 `ProxyGroups` 虚拟列表，将 `useConnectionData` / `usePathControl` / `ConnectionDetail` 下沉封装至 `ConnectionsPanel` 内部使关闭抽屉或窄窗时内存立即归零；`useTrafficMonitor` Worker 闲置销毁延迟由 5 分钟缩短至 `TRAFFIC_WORKER_DESTROY_DELAY_MS`（`30_000ms`）；收口右上角关闭按钮 `zIndex` 为 `SETTINGS_CLOSE_BUTTON_Z_INDEX`（`250`）并标记 `data-close-settings="true"`，补齐 Mixed Port 失焦保存与关闭取消守卫、`AddUrlDialog` 顶层保活、抽屉关闭菜单锚点清理及 `ProxyHead` `60ms` 定时器卸载清理。 |
| **BUG-386** | 2.4 / 2.5 | 根治鼠标拖拽调整窗口尺寸时的剧烈边缘抖动与反复跨越断点拖拽卡死：新增 Rust 原生原子 Win32 缩放管线（`begin_window_resize` / `step_window_resize` / `end_window_resize`），改用 OS 全局物理光标 `GetCursorPos` + 不变起始锚定矩形计算单次合并 `SetWindowPos(..., SWP_NOZORDER \| SWP_NOACTIVATE \| SWP_NOOWNERZORDER \| SWP_NOCOPYBITS)`，彻底根除 `e.clientX/Y` 正反馈震荡、双 IPC 撕裂及 `tao` 样式偏差；升级 `sizing_subclass_proc` 覆盖八方向宽高钳制与 `WM_ENTERSIZEMOVE`/`WM_EXITSIZEMOVE` 同步；前端 `ResizeHandles` 引入单调 `gestureSeq` 防并发并在 `WindowProvider` 拖拽期间冻结 `285px`/`126px` 断点组件拆装，松手由 `RESIZE_ENDED_EVENT` 携带精确客户区尺寸一次性对齐；`lib.rs` 拖拽期跳过逐像素几何查询。 |
| **BUG-387** | 5.2 / 5.6 | 在不改变任何自愈切点门限前提下根除前端延迟与真实网络状态双向失步：① [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 将 `LAST_SYNCED_ACTIVE_HISTORY_TIME` 升级为 `LAST_UI_ACTIVE_SYNC`（`UiActiveSyncState { node_name, history_time, reported_alive }`），推送 `Timeout (0)` 时不记录旧成功 `history_time` 而标记 `reported_alive = false` 确保恢复必同步；新增 `build_probe_ui_display` / `compute_next_ui_active_sync` / `should_sync_alive_ui` 纯函数使 `!primary_url_distorted` 下全超时空 `map` 也下发全组 `0`、`primary_url_distorted = true` 仅推送真实有效延迟不把未响应节点漆成 30 分钟 `0`、`ActiveTrafficState::Stalled` 叠加全灭 `None` 时以 `from_probe = false` 下发 30 秒 `(active_node, 0)` 消除顶栏假绿；② `AutoSelectOutcome` 增设 `covers_all_group_nodes` 门控防止子集测速或失真残缺探测提前置位 `reported_live_to_ui = true`；③ 前端 [delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts) 修复 `getDelayFix` 在 `proxy.alive === true` 且有效 `history` 时间戳严格新于缓存 `Timeout` 时立即淘汰旧 `0` 缓存回显新延迟，并限制 `injectBatchResults(..., { fromProbe: false })` 对 `Timeout` 一律封顶 `30s` 寿命；补齐 Rust 与 Vitest 双向同步回归单测。 |
| **BUG-388** | 5.1 / 5.2 / 5.3 / 5.4 | 加固后端连续性自愈边界与子集同源精度：`enhance/mod.rs` 为 `PROXY__METRICS` 显式写入 `lazy: false` 并删除 `constants.rs` 未使用 `INTERNAL_CONTROL_TIMEOUT_MS`（`monitor.rs` 复用 `constants::timing::NODE_DELAY_MAX_MS`）；`monitor.rs` 新增 `should_allow_https_migration` 阻止 `Flowing` 有流量态期间执行即时 HTTPS 验真/迁移、`HTTPS_VERIFY_MIGRATE_MIN_FAILS (2)` 收紧迁移门槛、`HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS (600s)` 遏制 HTTPS 全败探测风暴、`self_heal_subject_to_switch_rate_limit` 豁免已确证 `ConfirmedDead` 的速率截断、`should_keep_alive_on_api_recovery` 在 `ApiChannelDown` 恢复且现出口 `Alive` 时直接保活、`MANUAL_SWITCH_EPOCH` + `AUTO_SELECT_MANUAL_PREEMPTED` 防止在途自动选点覆盖用户手动切换；`node.rs` 改用 `str::trim_start` 对齐全 Unicode 空白（含全角空格 `\u{3000}`）；`lifecycle.rs::pick_subset_fallback_node` 严格禁止非空 `filter_text` 时越界切出筛选子集。 |
| **BUG-389** | 2.12 / 5.6 | 打通焦点快捷键与新手引导全协议导入保护：`disable-webview-shortcuts.ts` 将未修饰 `F4`/`F1` 提升至 `isEditableTarget` 判断之前，使节点过滤框等编辑焦点的用户也能一键触发列表测速与全量优选（保持 `Ctrl+F` 聚焦保护与 `Alt+F4` 零拦截）；`src/utils/node.ts` 新增并导出 `SUPPORTED_NODE_URI_SCHEMES`（11 种单节点 URI）与 `isSupportedSubscriptionOrNodeInput` 供 `FirstRunGuideDialog` 统一调用，打通剪贴板自动识别与单/多行全协议节点导入，清理重复 `⭐` 前缀，并限制重开引导时不再强行覆盖用户既有的系统代理开关偏好。 |
| **BUG-390** | 2.8 / 7.4 | 补齐订阅操作安全保护、离线重测防切点与全语种文案收敛：`ProfileImportCard` 行内刷新/删除按钮新增 `Tooltip` 与 `aria-label`；行内删除图标与右键「删除配置」统一通过 `LayoutDialogs` 3D 窄窗二次确认框删除（复用 `profiles.modals.confirmDelete`）；`ActiveNodeCard` 断网警示胶囊在 `healthIssue === 'offline'` 时传 `select: false` 仅测延迟不切换；`_layout.tsx` 启动重试失败报错改为 `layout.notices.profileActivationFailed`，同步补齐 13 语种 `errScheme` / `delayCheck` / `shared.placeholders.filter` 并在 `settings.json` 剔除 3 个零引用废键。 |
| **BUG-391** | 六 / 八 | 清理死代码、冗余依赖与全量文档对齐收官：通过 `git rm` 移除误追踪的 `src-tauri/.agents/` 审计中间产物与废弃 `resize-geometry.ts`（将 `window-resizing` 用例连同 `BUG-389` 用例收敛至 `src/utils/window-resizing.test.ts`）；移除 Rust 17 个零调用死命令及 `network-interface` / `gethostname` / `tokio-rustls` / `rust_iso3166` / `src-tauri` 侧 `criterion` 冗余依赖；清理前端零调用 `openCoreDir` / `openLogsDir` / `uninstallService`；全量同步 `clash_mini_agreements.md`、`docs/ARCHITECTURE.md`、`docs/implementation-notes.md` 与 `用户必读.txt`。 |
| **BUG-392** | 2.8 / 5.2 | **【待用户确认】** 根治节点过滤框输入 `-香港` 等排除词时的 IME 抢跑与在途竞态漏切：`node.ts` 导出 `hasEffectiveFilterTerm` / `isIncompleteFilterInput` 并补齐单测；`BaseSearchBox` 增加中文 IME 组词本地缓冲（`isComposingRef` + `composingText`）阻断拼音中间态（如 `"-x"`）派发；`ProxyHead` 拦截纯算子与末尾悬空前缀（如 `"-"`、`"-香港|-"`）抢跑测速；`use-head-state.ts` 新增 `flushHeadStatePersist` 并在 `ProxyGroups::handleCheckAll`/`handleCheckFull` 选点前同步落盘，且 `handleCheckAll` 补齐过滤变更尾部重测队列（`pendingRecheckGroupsRef`）与有限次 `AUTO_SELECT_BUSY` 续试；`monitor.rs::trigger_backend_auto_select_inner` 通过 `select_fastest_candidate` 在 `select_node_for_group` 切换**之前**实时重读 `latest_filter_text` 淘汰越界候选并补齐回归单测。 |
| **BUG-393** | 2.12 / 4.1 / 8.9 | **【待用户确认】** 新手引导与白名单指引明确信任区目标路径及 13 语种架构文案统一：`src/locales/*/layout.json` 与 `docs/antivirus-whitelist.md` 在 `antivirus.brief` / `antivirus.how` 中明确列出默认安装目录 `C:\Program Files\Clash Mini\` 及 `clash-mini.exe` / `mini-mihomo.exe`；`FirstRunGuideDialog` 与 `AntivirusWhitelistDialog` 增加 `overflowWrap: 'anywhere'` 防窄窗路径溢出；同步将 `ar`/`de`/`es`/`fa`/`id`/`jp`/`ko`/`ru`/`tr`/`tt` 10 语种残留的旧看门狗文案更新为现行轻量模式驻留架构说明。 |
| **BUG-394** | 1.1 / 2.4 / 4.5 / 5.1 | **【待用户确认】** 根治极端高 CPU 负载下退出轻量模式重建窗口引发的主线程与 Tokio 工作池连环死锁：`async_handler.rs` 新增 2 线程物理隔离守护运行时 `DAEMON_RUNTIME`（`spawn_daemon`/`spawn_daemon_supervised`），将 `monitor.rs` 后台监测自愈与 `server.rs` 单实例服务（超时放宽至 `1500ms`）彻底迁离 Tauri 默认工作池；`resolve/window.rs` 引入原子 `MAIN_WINDOW_HWND`（`WM_NCDESTROY` 及 `destroy_main_window` 自动清零）并将 `HWND` 样式/尺寸/子类化钩子收拢至 `run_on_main_thread` + `12s` 超时销毁回滚；`window_manager.rs` 将窗口可见性/焦点/最小化与显示器枚举改造为零跨线程阻塞的 Win32 直查（`IsWindow`/`IsWindowVisible`/`IsIconic`/`GetForegroundWindow` + 原子焦点缓存、`EnumDisplayMonitors` + `GetMonitorInfoW`）；`notification.rs` 增加 `is_main_window_ready()` 守卫。 |
| **BUG-395** | 1.1 / 2.9 / 2.10.1 / 2.12 / 4.1 | **【待用户确认】** 新手引导瘦身、帮助菜单「轻量模式防误杀指南」重构与「静默启动」全场景无窗进托盘防误杀机制：`lightweight.rs` 新增 `should_enter_silent_tray_on_launch()` 与 `has_real_profile_items()`，凡开启 `enable_silent_start` 且已有真实配置时，开机 `--silent`、手动点击图标及单实例 `/commands/visible` 均直接维持轻量托盘驻留不露主窗（仅左键托盘图标开窗），手动启动时经 `Tray::notify_silent_running()` 弹出原生 Win32 `Shell_NotifyIconW(NIF_INFO)` 气泡（`2s` 防抖、零 GUI 窗）；`FirstRunGuideDialog` Step 2 移除冗余安全软件卡片；`?` 帮助菜单入口升级为「🛡️ 轻量模式防误杀指南」，重构 `AntivirusWhitelistDialog`、全部 13 语种 `layout.json` 与 `docs/antivirus-whitelist.md`，提供弹窗内一键开启「静默启动」（含日常防露窗提示）及 Windows 安全模式配置 `whiteinternal.ini` 硬白名单（含路径与片段一键复制）。 |
| **BUG-396** | 2.8 / 4.2 / 7.7 | **【待用户确认】** 区分前后台切点清连接边界并补齐路径控制详情节点显式标注：① 解决「重装先走受限香港节点建连 → 输入 `-香港` 或手动切日/新节点后 HTTP/2 长连接仍粘死在旧节点」问题——后台自愈与冷启动保持无缝热切换，仅当用户通过前台 4 大显式切点入口（列表点选、顶栏胶囊轮换、过滤框/`F4`/`F1`、健康胶囊重测）换节点时，经 1:1 同源 `should_close_stale_proxy_connection`（`monitor.rs` + `cmd/proxy.rs::trigger_auto_select`）与 `shouldCloseStaleProxyConnection`（`path-control.ts` + `mihomo-api.ts` + `use-proxy-selection.ts` + `active-node-card.tsx` + `proxy-groups.tsx`）精准关闭 `PROXY` 组中非新目标节点的存量代理连接，绝不误伤 `DIRECT` 及同节点连接；② [connection-detail.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-detail.tsx) 在 `主机` 正下方新增 `代理节点` 行（`resolveConnectionNodeDisplay` + 全 13 语种 `connections.json` `proxyNode`/`staleNodeTag`），对滞留在后台切换前旧节点上的活跃连接醒目标注 `（旧节点连接）`，并补齐 Rust 与 Vitest 单测。 |
| **BUG-397** | 2.8 / 4.2 / 5.2 / 5.6 | **【待用户确认】** 将列表顶部闪电图标 `⚡`（`BoltOutlined`）与 `F4` 快捷键由「测速并自动切至最快节点 + 清理旧连接」收敛为「仅刷新当前可见列表延迟、不换节点、不断连接」（`select: false, cleanStaleConnections: false`）：在 [node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts) 新增共享意图裁决纯函数 `resolveBatchCheckIntent` 并补齐单测，调整 [proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx)、[proxy-render.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx) 与 [proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx)（`handleCheckAll` 默认 `select=false` 并追踪 `activeTestSelectRef`/`pendingRecheckSelectRef`，在途纯测速遇到过滤提交时可平滑升级并排队重测为 `select: true`），同时保留过滤框输入（700ms 防抖/Enter）、`F1` 全量优选、列表点选及顶栏胶囊交互的原有切点与清连接行为。 |

---

## 附录 B: 已知非缺陷（NOT A BUG，勿重复排查）

排查内核日志（`%APPDATA%/<appid>/logs/service/`）时，下面三类告警属于**网络现实或正常生命周期噪声**，不是配置/规则缺陷，不要再立项修。每类都给了可复现的判据，照判据核对一遍即可结案：

| 现象 | 判据（怎么证伪「这是 bug」） | 为什么不改 | 处置 |
| --- | --- | --- | --- |
| `dial DIRECT (match GeoIp/cn) ... : i/o timeout`，境内目标，**IPv4 与 IPv6 都会出现** | ① 同批日志里能找到境内 IPv4 目标同样超时（2026-10-23 实证：`cn.bing.com`→`202.89.233.101`、`agt.p.360.cn`→`1.192.136.133`、`frontier.zijieapi.com`→`111.63.63.169`）；② 事后直连同一 IPv6 目标能通（实测 `[2409:8c20:818:11d::c4]:80` → 404 / 227ms，`[2409:8c1e:75b0:78::d1]:80` → 404 / 89ms）| 境内直连路径（Wi-Fi/ISP 上游瞬时丢包或目标站限流）的偶发抖动。**规则层无法判断「此刻这条路径抖了一下」**；把境内目标改走代理只会更慢更不稳；给 IPv6 加 `REJECT` 则会连同正常 IPv6 一起废掉 | **不改规则**。应用层 Happy Eyeballs 已做 v4/v6 竞速 + 失败重试，用户无感 |
| `operation was canceled` 成批出现，连 `223.6.6.6` / `120.53.53.53` 等国内 DNS 也在内 | 时间点与内核重启/退出、TUN 重建重合 | 核心 teardown 时统一取消在途 dial，属正常生命周期 | 不改 |
| `dial PROXY ... --> <节点server>:<端口> i/o timeout`（目标是**代理节点自身**，不是用户目标） | 日志里 `-->` 右侧是节点 server 的 IP:端口而非业务域名 | 不是规则缺陷，是节点侧故障或该 VPS 当时拨不上 | 属 node-health 范畴：由 `PROXY__METRICS` url-test 标红 + 自愈监控切换。2026-10-23 实证：`直连-日本-anytls`（`188.253.123.188:15404`）在 05:39 拨号超时，后台监测先报「网络已断开 → 暂停健康检测」、恢复后按宽限期不记失败，13:00:06 自动优选切到 `日本aw6`（174ms），未误杀、未空转 |

**通用判据**：某条 `i/o timeout` 是不是缺陷，只看两点——目标是不是**代理节点自身**（是 → node-health，不是 → 用户目标），以及**同类目标换个时间点是否可达**（可达 → 网络抖动，不可达 → 才是配置问题）。



