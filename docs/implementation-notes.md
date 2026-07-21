# Clash Mini 实现笔记（代码细节备查）

> 本文件是《开发设计书》(clash_mini_agreements.md) 的**配套实现笔记**，不属于设计约束本身。
> 设计书只描述「要做什么、为什么」；本笔记记录「具体落在哪些函数 / 文件 / 变量上」，供日后排查「某条设计规则由哪段代码实现」时查证。
> 若某条设计规则对应的实现已变更，请以当前代码为准，并顺手更新本笔记。

---

## 一、项目定位与架构

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `mini-mihomo` / `mini-mihomo-alpha` | 内核二进制名 | 与原版 `verge-mihomo` 物理隔离（§1.9） |
| `clash_verge_service` | 系统服务名 | TUN 模式服务共享（§1.5） |
| `verge.yaml` / `profiles.yaml` | 配置文件 | 与原版配置隔离（§1.3） |
| `AppData\Roaming\io.github.clash-mini.clash-mini` | 数据目录 | 配置独立加载（§1.3） |
| `10801` / `9098` | Mixed / Controller 端口 | 端口避让原版（§1.4） |
| `33335`(Release) / `33336`(Dev) | 单实例检测端口 | 单实例隔离（§1.8） |
| `GOMEMLIMIT=96MiB` / `GOGC=50` / `GOMAXPROCS=2` / `geodata-loader=memconservative` | 内核环境变量 | 壳进程/内核内存基准（§1.2） |
| `SetProcessWorkingSetSize` / `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW` | Windows API | 已**废弃**，v1.9.4 起删除（§1.2） |
| `optimize_window_memory` | 函数 | 已**废弃**删除（§1.2） |

## 二、界面与交互（窗口 / 缩放 / resize）

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `startResizeDragging` | Tauri API | **禁止**使用：Windows 下触发模态 resize 死锁（§2.2 标题栏） |
| 自定义 pointer-based resize：`onPointerDown` + `setPointerCapture` → `pointermove` + `requestAnimationFrame` 节流 → `setSize`/`setPosition` → `pointerup` | 前端 resize 实现 | 替代 `startResizeDragging`，避免 UI 线程卡死（§2.2） |
| `window.__isResizing` 全局标志 | 前端全局变量 | resize 期间跳过 IPC 调用，防 IPC 洪水（§2.2） |
| `onResized`（`useVisibility`、`WindowProvider`） | 前端回调 | 检查 `__isResizing` 跳过 IPC（§2.2） |
| `trigger_backend_auto_select` | 后端命令 | 批量测速已迁后端，前端仅持 1 个 IPC（§2.2） |
| `setResizable` / `batchTestLockRef` / `DragRegionContext` | 已移除 | resize 期间不再锁窗口（§2.2） |
| `build_new_window` 的 `builder.build()` 之后 | 窗口创建 | Windows 初次启动剥除 caption 样式位的时机（§2.2） |
| `strip_caption_style` / `SetWindowLongPtrW(GWL_STYLE, ...)` / `WS_CAPTION` / `WS_THICKFRAME` | Win32 调用 | 清 `WS_CAPTION` 保留 `WS_THICKFRAME`，修复首次启动 outer 多 15-22px（§2.2） |
| `SetWindowPos(SWP_FRAMECHANGED \| ...)` / `force_set_window_outer_size` | Win32 调用 | 触发 `WM_NCCALCSIZE` 重算 + 直接设 outer 尺寸（按 DPI 缩放）（§2.2） |
| `handle_ready_resumed` / `RunEvent::Ready` / `get_webview_window("main")` / `init_window` | 启动事件 | 修复代码必须在 `Ok(window)` 分支执行，不能在 `RunEvent::Ready`（时机过早返回 None）（§2.2） |
| `ProxyGroups` / `ConnectionTable` / `ProxyVirtualList` / `ProxyItem` / `ProxyHead` | 前端组件 | 类 Excel 表格，滚动条隐藏（§2.10） |
| `EnhancedCanvasTrafficGraph` / `TrafficGraph` | 前端图表组件 | 宽窗大图 / 窄窗小图切换（§2.2） |
| `get3DButtonStyle` / `--depth-factor` / `--vibrancy-factor` | 皮肤样式 | 3D 拟物与双控制变量联动（§2.4/2.8/STYLE-001） |
| `tray-icon.png` / `clash_mini_ragdoll_metal` | 托盘图标资源 | 静态猫咪头像（§2.1/2.18） |
| `set_icon` / `set_tooltip` / `set_menu` | 托盘 API | **禁止**运行期调用，防 `E_FAIL`（§2.18, BUG-073） |
| BUG-259 窗口操作必须在 UI 主线程 | 线程约束 | 见 §八 工程规范 8.3 |

## 三、内核与运行时

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `NetworkManager` Fallback（Localhost → System → Direct） | 后端网络托管 | 内核更新请求严禁裸 HTTP；三级降级（§3.3） |
| `formatCoreVersion` | 前端函数 | 内核版本前缀剥离 + `Ver.` 统一格式（§3.3） |
| `fix_dirty_url` / `validate_url_no_ssrf` | 函数 | 订阅 URL SSRF 防护（§6.2） |
| `download_icon_cache` | 函数 | 图标下载 URL 协议 + SSRF 校验（§6.2） |
| `safe_extract_zip` | 函数 | Zip Slip 防护，替代 `zip.extract`（§6.2） |
| `serde_json::to_string` | 序列化 | JS 脚本传参防注入，替代字符串拼接（§6.2） |
| `boa_engine` | JS 引擎 | 原型链冻结等沙箱加固（§6.3） |
| `geodata-loader=memconservative` | 内核配置 | 按需加载 GeoIP/GeoSite，降内存（§3.6） |

## 四、轻量模式

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `setup_light_weight_timer` / `cancel_light_weight_timer` | 已移除 | 旧定时器机制全部移除（§4.1） |
| `enable_auto_light_weight_mode` | 配置项 | 保留字段但无运行时行为（向后兼容）（§4.1） |
| `auto_lightweight_boot` | 函数 | 直接读 `enable_silent_start` 判断自动轻量启动（§4.1） |
| 前端 WS 通道 `/traffic` `/logs` `/connections` | WebSocket 端点 | 进入轻量模式主动熔断（§4.2） |
| 前端 `useEffect` 挂载逻辑 | React | 退出轻量时自动重连 WS，无需手动重建（§4.2） |
| `isStartingUpRef.current` / `trigger_auto_select(uid, 0, false)` / `verge://backend-delay-results` | 前端/事件 | 唤醒静默填充延迟缓存，不切节点（§4.4） |

## 五、后台监控与自愈

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `evaluate_failover` / `delay_group("PROXY__METRICS", ...)` | 后端函数 | 每次判定前内核级即时拨测，消除 5 分钟盲区（§5.1 / v2.6.6 修复） |
| `NODE_DELAY_MAX_MS`(2000) / `NODE_TEST_TIMEOUT_MS`(2000) | 常量 | 探针超时 == 判死阈值 == 2000ms（§5.1） |
| `AUTO_SELECT_BUSY` | 状态 | 选点冲突不算失败，不冷却（§5.1） |
| `api_error_count` / `auto_select_fail_count` | 计数器 | API 异常计数触发自愈 / 5 次弹窗（§5.1） |
| `check_active_node_health` | 已废弃函数 | 旧 15s 自测方案（TUN 下测不准）被 PROXY__METRICS 取代（§5.1 历史） |
| `trigger_auto_select(profile_uid, node_names, 0, select)` / `trigger_backend_auto_select` | 后端命令 | F1/F2/F4 统一收归后端选点（§5.5/5.6） |
| `AUTO_SELECT_RUNNING` | 互斥标志 | 全局并发互斥，杜绝重复测速叠加（§5.5/5.6） |
| `DelayManager.injectBatchResults` / `setDelay` / `queueGroupNotification` | 前端缓存 | 回传结果逐节点写回 + 分组刷新（§5.6） |
| `FrontendEvent::DelayResults` / `Handle::notify_delay_results("PROXY", display)` / `verge://backend-delay-results` | 事件通道 | 测速结果唯一回传通道（§5.6） |
| `handleCycleNode` / `filterSort` / `localStorage`(`clash-verge-node-search` / `clash-verge-hide-unavialable` / `clash-verge-sort-type`) | 前端 | 轮换实时提取过滤/排序子集（§5.7） |
| `useTrafficData` / `useConnectionData` / `useMihomoWsSubscription` | 前端 Hook | 隐藏/最小化时断 WS 订阅（§5.3） |
| `ConnectionsPanel` / `isMinimalWidth` | 前端 | 连接表宽窄窗同开关（§5.3） |
| `getSystemProxy`(`sysproxy`) / `getRunningMode`(`runningMode`) / `getClashConfig`(`clashConfig`) | 前端查询 | `enabled = isSettingsOpen && !isMiniStatus`（§5.3） |
| `LogsPage` / `logsOpen` | 前端 | 日志弹窗关闭即卸载，注销 WS（§5.3） |
| `TrafficDataSampler` / `snapshotIntervalMs`(1000ms) | 前端采样 | 流量图重绘节流（§5.3） |
| `app-data-provider.tsx` / `getAppUptime`(`appUptime`) | 前端 | 运行时间轮询消除（§5.3） |
| `use-layout-events` | 前端 | 测速回传监听器生命周期（§5.6） |

## 六、安全与健壮性

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `shell:allow-open`（移除 `execute`/`spawn`/`kill`/`stdin-write`） | Tauri 权限 | 权限最小化（§6.1） |
| FS 作用域 `["$APPDATA/**"]`（移除 `**`） | Tauri 权限 | 防前端读任意文件（§6.1） |
| HTTP 插件域名白名单（`cdn.jsdelivr.net` 等） | Tauri 权限 | 收紧 HTTP（§6.1） |
| `open_web_url` | 函数 | 仅允许 http/https（§6.2） |
| `danger_accept_invalid_certs` | 配置 | 启用记 warn 日志（§6.2） |
| `boa_engine` 原型链冻结（Object/Function/Array/String/Number/Boolean.prototype） | JS 沙箱 | 防原型污染（§6.3） |
| `overflow-checks=true`(release) | 编译配置 | 防整数溢出（§6.4） |
| YAML 大小限制 50MB | 解析前置 | 防解压炸弹（§6.4） |

## 七、数据流与 API

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `enhance_profiles` | 后端函数 | 保证始终存在唯一 PROXY 组（§7.1） |
| 代理组名固定 `'PROXY'`（禁动态查找） | 常量 | 单 PROXY 组模型（§7.1） |
| `calcuProxies` | 函数 | 保证 PROXY 组存在（§7.1） |
| `activateSelected` / `restore_profile_selected_nodes` | 前端/后端 | 只处理 PROXY 组（§7.1） |
| `restart_core` / `update_config_checked` | 后端函数 | 内核重启前后 PROXY `now` 快照恢复（§7.1） |
| `trigger_backend_auto_select` → `select_node_for_group("PROXY", ...)` → `profiles_patch_item_safe` | 后端链路 | 选点成功回写 `profile.selected`（§7.1） |
| `handleImportProfile` / `restartCore()` | 前端 | 导入后重启内核（§7.2） |
| `viewProfile(uid)` / `patchProfile(uid, ...)` / `mutateProfiles()` | 后端/前端 | 订阅右键菜单与编辑（§7.3） |
| `getMenuItemHoverStyle` / `get3DInputStyle` / `get3DButtonStyle` | 样式函数 | 菜单/对话框 6 皮肤适配（§7.3） |
| `FindProcessMode`(枚举, `rename_all="lowercase"`) | Rust 枚举 | 解决 Allow LAN 等开关回弹（§7.3） |
| `_layout.tsx` / `src/locales`(`settings.json`) / `routingTooltipRules` | 前端/国际化 | 策略分流倾向文案统一（§7.5） |
| `debounce`(泛型约束) / `calcuProxyProviders`(禁 `as unknown as IProxyItem`) | 编码约束 | 类型安全（§7.6） |

## 八、工程规范

| 代码标识 | 类型 | 服务的设计规则 |
|---|---|---|
| `bug_list.md` | 文档 | Bug 确认时序：发版前只登记「待验证」（§8.1） |
| `verify.py` | 预检脚本 | 发版预检放行「待验证」状态（§8.1） |
| `RunEvent::WindowEvent` / `api.prevent_close()` / `tauri::Builder.on_window_event` | Rust 窗口事件 | 禁止在 `app.run` 循环捕获 CloseRequested；统一 `.on_window_event` 注册（§8.3） |
| `window.get_webview_window("main")` → `window.hide()` | Rust 调用 | 关闭前 hide 规避 Destroyed Panic（§8.3） |
| `patch_clash()`：`apply()` 先于 `enhance()`；`get_config_values()` | Rust 调用 | Config Draft 时序（§8.4） |
| 轻量模式窗口尺寸恢复：`window_state.json` / `restore_window_size()` / `save_window_size()` / `destroy_main_window()` / `build_new_window()` | Rust 窗口管理 | 进入轻量模式存尺寸、唤醒读回套用（v2.6.9 修复，详见 audit-log） |
