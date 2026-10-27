## v3.1.4

### ✨ Onboarding Experience & UI Consistency

- **首次使用向导全链路重设计与 GitHub Star 激励转化（First-Run Onboarding Redesign）**：
  - **两步式价值感知向导（`STEP 1 / 2` → `STEP 2 / 2`）**：新增顶部步骤胶囊徽章与三枚核心特性标识（智能优选低延迟节点 / 防 DNS 泄漏保护 / 轻量静默驻留）；
  - **剪贴板智能嗅探一键填入**：弹窗打开时自动静默检测系统剪贴板，若包含单行 `http(s)://` 订阅链接且输入框为空，展示可一键填入的横幅提示，实现零键盘输入导入；
  - **实时连通性仪表盘与开源支持卡**：导入成功后原地切换至第二步，实时展示当前最优出口节点名称与毫秒级延迟徽章，并新增暖金渐变 GitHub ⭐ Star 激励卡（含主按钮直达仓库测速点 Star 及次级 Wiki 指南入口）；
  - **顶栏帮助菜单常驻 Star 入口**：`?` 帮助下拉菜单首项升级为醒目的 **`⭐ 去 GitHub 点亮 Star`**（13 个语种资源全量同步对齐）。
- **顶部活跃节点栏与下方节点表格延迟颜色标准 100% 统一**：
  - **现象**：此前顶部活跃节点栏（`ActiveNodeStatusCard` / `getSignalIcon`）沿用了独立硬编码的旧分段阈值（`200 / 300 / 500ms` 及 `info.main`）与三分支胶囊底色，导致同一延迟数值在 `200 ~ 4999ms` 区间内顶部栏与下方节点表格（`ProxyItem`）呈现出截然不同的颜色；
  - **修复**：在 `src/services/delay.ts` 中收敛出共用颜色解析函数 `resolveSignalDelayColor`，使顶部活跃节点栏与下方节点表格 100% 共享 `formatDelayColor` 单一真源（`< 250ms` 绿色 `success.main`、`250 ~ 399ms` 主题色 `primary.main`、`400 ~ 4999ms` 橙色 `warning.dark`/`warning.main`、`0` 或 `>= 5000ms` 红色 `error.main`），同步修复胶囊背景色映射并补充深浅色主题全区间单测。

---

## v3.1.3

### 🐞 Fixed Bugs & Core Lifecycle

- **关闭 TUN 模式后彻底释放系统服务并切回用户态 Sidecar（BUG-305 / F-01）**：
  - **现象**：此前从 TUN 模式切回「系统代理」或「手动模式」时，仅执行 `UpdateFlags::CLASH_CONFIG` 热重载，Mihomo 内核仍继续驻留在高权限 Windows 服务（`RunningMode::Service`）中运行，未释放服务句柄也未切回普通用户态 `Sidecar`。同时前端开启 TUN 时在 `patchVerge` 之后又重复调用了一次 `restartCore()`，造成双重重启延迟。
  - **修复**：后端 `determine_update_flags` 对比修改前后的 `enable_tun_mode` 与当前 `RunningMode`，开启或关闭 TUN 时均自动附加 `UpdateFlags::RESTART_CORE`，在关闭系统代理时先清理注册表再重启切换运行模式，并在 `RESTART_CORE` 已执行时跳过冗余热重载；前端移除重复的 `restartCore()` 调用并统一刷新系统运行态。
- **Win32 `WM_SIZING` 防上边缘缩放下移钩子 100% 挂载生效（BUG-305 / F-02）**：
  - **现象**：`setup_wm_sizing_hook` 原先仅在 `RunEvent::Ready` 中调用，比异步创建主窗口早约 0.5 秒触发，导致首次冷启动时 `get_webview_window("main")` 返回 `None` 跳过安装；且旧的全局重入锁在轻量模式销毁并重建主窗口后阻止为新 HWND 重新挂载钩子。
  - **修复**：将 `setup_wm_sizing_hook` 迁入 `build_new_window` 窗口创建后立即执行，并改用 `HOOKED_HWND` 按窗口句柄原子判重，确保初次启动与轻量模式唤醒重建窗口后均 100% 生效。
- **单例唤醒补齐 `clash-mini://` 协议匹配并修复 Deep Link 参数截断（BUG-305 / F-03）**：
  - **修复**：`check_singleton` 新增 `clash-mini:` 与 `clash-verge:` 协议前缀识别，改用 `.query(&[("param", param)])` 自动对含 `&` 的订阅 URL 进行百分比编码，防止服务端查询参数截断；非 `--silent` 普通二次启动时正确唤醒已有实例主窗口。

### ⚡ Performance & Bundle Size Optimization

- **剥离未使用的 Monaco Editor 子系统与 16 个冗余依赖（BUG-305 / FE-01, FE-02, FE-04, FE-08, FE-09, FE-10）**：
  - 移除零引用的 Monaco 编辑器组件、YAML Worker 及 16 个未使用的 npm 依赖包（`node_modules` 净减 122 个包），前端 `dist/` 构建产物直降 **>4.4MB (>60%)**；
  - 清理 4 个无引用历史基础组件、约 560 行未使用的手动出站代理配置与 WebDAV 类型定义、以及后端 13 语种共 52 处孤立 i18n 键；`vite.config.mts` 移除遗留的 `safari>=14` 目标，进一步缩减 polyfill 体积。
- **前后端热路径与渲染生命周期优化（BUG-305 / F-04, F-05, F-06, F-07, FE-03, FE-05, FE-06, FE-07）**：
  - `get_proxy_addr` 改用强类型轻量结构体单次读取并反序列化 `clash-mini.yaml`，消除 fallback 阶段的二次磁盘读取与全量动态 AST 构建；
  - 残留核心清理改用 `System::new_with_specifics` 仅扫描进程列表，避免枚举磁盘/网卡/传感器带来的 50~150ms 阻塞；后台监控任务覆盖前显式 `abort` 旧句柄；系统服务管理命令统一解析 `%SystemRoot%\System32\sc.exe` 绝对路径并规范化安装器路径；
  - `useProxyDelayState` 改用 `useReducer` 惰性初始化消除节点卡片挂载时的首帧双重渲染；`FirstRunGuideDialog` 解耦为细粒度 Context 订阅；`ProxyRenderComponent` 移除无效的 `useVerge` / `useIconCache` 行级订阅；收敛 `useSkinControls` 与 `useCustomTheme` 的 `controlSkin` 状态为单一真源。

---

## v3.1.2

### 🐞 Fixed Bugs & Threshold Consistency

- **前后端判死阈值分裂修复：前端单一真源补齐 2000 → 5000 同步（BUG-304 收尾）**：
  - **现象**：v3.1.1 中后端 monitor.rs 判死阈值已放宽至 5000ms，但前端 `NODE_DELAY_MAX_MS` 仍停留在 2000ms——实测延迟 2000~5000ms 的「拥塞期慢而可用」节点，后端判定存活，UI 却照样红字 Time out 并沉底排序，BUG-304 的现象在显示层复发。
  - **修复**：前端延迟单一真源（`src/services/delay.ts`）、后端 timing 模块的第二处同语义常量（`src-tauri/src/constants.rs`）与两处注释口径统一同步为 5000ms，与 monitor.rs 单一真源完全一致。
  - **回归锁定**：新增 BUG-304 阈值同步用例——前端真源必须恒等于 5000、3000ms/4999ms 按有效延迟呈现、恰 5000ms 才判 Timeout，防止后续再漂移。

---

## v3.1.1

### 🐞 Fixed Bugs & Congestion-Tolerant Speed Test

- **链路拥塞期活节点被整组误判为断流，UI 全员 Time out（BUG-304，双路对照监控实证）**：
  - **现象**：节点列表频繁整批转 Time out，数分钟后自行恢复；同一时刻绕过内核从物理网卡直连同一节点服务器 TCP 握手仅 222~464ms，节点服务器与本地宽带均正常。60 秒周期双路对照监控（内核测速 vs 物理直连）捕获到链路拥塞簇：TCP 握手在 106ms ↔ 2149ms 之间以约 5 分钟为周期成簇波动，机场中转集群与 hy2（UDP）节点则确属机场侧故障/UDP QoS。
  - **根因**：节点判死阈值与批量测速超时共用 2000ms 单一真源。拥塞簇内 TCP 握手即需 1.1~2.1s，叠加 TLS 握手与 204 探测往返后，「慢而可用」的活节点总耗时稳定超过 2s 预算，被整组误判为断流；下一轮 300s 测速周期到来前 UI 一直显示全员 Time out。
  - **修复**：判死阈值与批量测速超时统一 2000ms → 5000ms（`NODE_DELAY_MAX_MS` 单一真源，同步驱动探针超时、判死阈值与有效延迟区间三处语义）。5s 预算可覆盖拥塞期「慢而可用」的节点；真死节点（机场集群故障）5s 内同样超时，不会产生误活。

### 📖 Engineering Standards

- agreements 新增 BUG-304，完整归档本次「双路对照抓现行」的诊断方法（内核测速与物理直连并行采样，异常组合自动标注 KERNEL-FAULT / LINK-CONGESTION），供后续同类「是机场还是程序」争议复用。

---

## v3.0.10

### 🐞 Fixed Bugs & Accurate Status Display
- **已断流的节点在节点列表里仍显示绿色旧延迟（BUG-303，3.0.9 现场实证）**：
  - **现象**：活跃节点栏已显示 Time out，同一个节点在下方的节点列表里却是绿色的 106ms；点它重新测速依然 Timeout，列表里的数字纹丝不动。
  - **延迟取值改为三级真源**：内核拨测失败时只把节点标记为不可用、**不再追加 history**，history 会永久停在最后一次成功的那个值。此前前端查不到本次测速结论时会无条件回落到 history，等于把「过去的成功」当成了「现在的延迟」。现在回落前先校验内核存活状态与记录新鲜度——已判死、或记录早于测量组 2 个周期（10 分钟）的，一律按 Timeout 呈现。
  - **本次测速结论仍然优先于内核存活标记**：依《设计契约 5.8》，测速链路失真时内核会把全组置为不可用，若以它优先会让整个列表瞬间全红。因此已有真实测量结果时仍以测量结果为准。
  - **同一节点的延迟订阅改为支持多个订阅者**：活跃节点栏与列表项一直在监听同一个节点，旧实现里后注册的会顶掉先注册的，被顶掉的那个从此收不到任何更新、永久停在挂载时的旧值。现在两者并存，注销时也只摘除自己那一个。
  - **自愈失败同样刷新前端数据**：此前只有选点成功才通知前端重拉代理列表，失败时不发，前端于是长期握着陈旧的内核存活状态与延迟。现在失败分支也会刷新，界面延迟与真实连通状态保持一致。

### 🧪 Tests & Engineering Standards
- 新增 6 例延迟呈现回归用例：覆盖「内核已判死不得回落 history」「history 超期不再冒充当前延迟」「时间格式异常时保守回落」「本次测速结论优先于内核标记」，以及多订阅者的同时通知与精确注销。
- 修正缺陷编号：本条此前误用了已被占用的 `BUG-288`，顺延为 `BUG-303`，全表 59 条编号复核无重复。
- 修复 `package.json` 与 `src-tauri/tauri.conf.json` 的格式化偏差，避免 CI `pnpm format:check` 门槛误红。

---

## v3.0.9

### 🎨 UI & Minimalist Design
- **极窄隐匿模式窗框发光装饰移除（回归极简纯净边缘）**：
  - 用户在极窄且极小高度（≤126px）静置 10 秒后，标题栏依然自动平滑隐藏、内容区撑满 100vh 且全区域可按住无缝拖拽；
  - 彻底删除 `GlowBorder` 边框组件及关联的 6 套皮肤渐变分支、LocalStorage 读取与全局事件监听器，避免任何潜在渲染与计算开销，呈现无多余修饰的极简纯净边缘。

### 📖 Architecture & Standards
- **窗口模式设计契约与架构清单同步定型**：
  - 《设计契约 2.6》正式更名为「极窄隐匿模式（自动隐藏标题栏规范）」，同步更新为纯净无装饰定型说明；
  - 修正 `docs/window-mode-optimization-checklist.md` 中过期的 135px 阈值与 `GlowBorder` 行号引用；后端 `window.rs` 相应更新自绘注释。
- **新增附录 B《已知非缺陷（NOT A BUG）》日志判据规范**：
  - 基于现场日志实证，系统性归档境内直连抖动（`GeoIp/cn` i/o timeout）、核心退出批处理取消（`operation was canceled`）以及节点侧拨号超时（node-health）三大类常规日志现象；
  - 确立两点可复现通用判据，杜绝将正常生命周期与网络抖动误判为配置/规则缺陷而反复立项排查。

---

## v3.0.8

### 🐞 Fixed Bugs & DNS Reliability
- **DoH 上游辅 IP 漏入直连，`8.8.4.4:443` 超时刷屏（BUG-302，3.0.7 现场实证）**：
  - **主辅 IP 对偶补齐**：`dns.fallback` 里只写 `8.8.8.8` 时，内核解析 `dns.google` 得到的辅 IP `8.8.4.4` 此前没有任何规则接住，一路漏到 `MATCH,DIRECT` 直连失败（3.0.7 日志实证 29 条 `dial DIRECT (match Match/) mihomo --> 8.8.4.4:443 error: i/o timeout`）。现在 Google DNS（`8.8.8.8` ↔ `8.8.4.4`）与 Cloudflare DNS（`1.1.1.1` ↔ `1.0.0.1`）的主辅对偶会自动补齐走代理规则，只配了主 IP 也能覆盖辅 IP。
  - **域名型上游改为按域名直连代理**：`https://dns.google/dns-query` 这类域名上游过去不生成任何规则，只能靠解析后的 IP 碰运气命中；现在生成 `DOMAIN,dns.google,PROXY`，内核建连时按域名直接走代理，并同步补齐其固定落地 IP（`8.8.8.8` / `8.8.4.4`）。
  - **规则仍全部从真实 `dns.fallback` 推导**：订阅换上游自动跟随，没有配置这些 DNS 的用户不会凭空多出规则；插入位置保持在 `RULE-SET,gfwlist` 之后、用户 `append-rules` 之前，不抢用户自定义规则的最终决定权。

### 🧪 Tests & Engineering Standards
- 新增 DoH 上游规则回归用例：覆盖「只配 `8.8.8.8` 时自动补齐 `8.8.4.4`」「只配 `dns.google` 时同时生成 `DOMAIN` 与落地 IP 规则」两种现场形态，并锁定主辅对偶与去重顺序。

---

## v3.0.7

### 🎨 Architecture & Code Elegance
- **窗口首帧状态镜像对齐**：
  - `WindowProvider` 提取首帧常量 `initialIsMini`，确保 `isMiniStatus` 响应式状态与 `isMiniStatusRef` 镜像在第 0 帧绝对同步，消除异步微任务下可能存在的瞬态不一致隐患。
- **设计协议书全链路数值对齐**：
  - 《设计契约 2.6》正文全面同步定型为 126px 阈值与 189 物理像素描述，消灭历史重构遗留的数值漂移与表述分歧。
- **Rust 后端 Clippy 坏味道清零**：
  - 修复 `build.rs` if 折叠简化、`diagnostics.rs` 倒序 key 规整、`tun.rs` 优雅降级 `unwrap_or_default` 以及 `init.rs` match 逻辑分支优化，保持后端严苛的零警告零坏味道标准。

### 🧪 Tests & Engineering Standards
- **工程构建基底更新**：更新 `biome.json` 的 `$schema` 至 `2.4.13`，保持前端工程化规则库最新。

---

## v3.0.6

### 🐞 Fixed Bugs & Network Recovery
- **订阅占位假节点拦截与开机断网自愈修复（BUG-301）**：
  - **占位假节点精准拦截**：部分订阅将提示信息或广告伪装为节点（如 `server: '1'` 或纯汉字/无点域名），该类无效节点入选 `PROXY` 组首位会导致 TUN 模式下全系统流量打入死地址引起断网。引入严格的 `is_valid_node_server` 校验，仅允许合规的 IPv4/IPv6 或带点的有效 FQDN 域名，严禁无效占位节点入组。
  - **开机恢复自愈重试**：核心启动节点恢复逻辑增设 90 秒预算重试（3 秒间隔），解决开机瞬间内核尚未完全就绪导致恢复失败并落入 80 秒断流死循环的偶发故障。
- **极窄模式最小高度阈值终态定型（BUG-300 定型）**：
  - 门禁阈值由 121px 最终精确定型为 **`126px`**：在 Windows 最常见的 150% DPI 缩放下，$126 \times 1.5 = 189$ 恰为完全无余数的整数物理像素，彻底消除浮点舍入歧义，无需容差兜底即可实现像素级完美闭环。

---

## v3.0.5

### 🐞 Fixed Bugs & System Reliability
- **TUN 服务模式三层失效根治（BUG-298）**：
  - **服务就绪预算放宽**：SCM 首次启动等待预算从 5s 延长至 30s，消除高负载启动下的服务超时误判。
  - **预算与单次重试解耦**：将启动总预算（`timing::SERVICE_WAIT_MAX`）与单次请求重试（`max_retries`）拆分，杜绝重试耗尽导致虚假失败。
  - **锁外等待避免缓存踩踏**：内核重启引入锁外独立等待服务就绪，彻底避免读取到陈旧的 `Unavailable` 缓存而错误触发静默降级为 Sidecar 模式。
  - **状态事件保真同步**：降级与回退场景补发 `Handle::refresh_verge()` 事件，确保前端 TUN 状态开关与系统底层运行状态始终保持一致。
- **无框阴影消除与对称余量修正（BUG-300）**：
  - 修正无系统边框阴影后的垂直几何余量，窗口最小高度门禁从 135px 缩减调整为 121px，消除底部被吞噬的 14px 边距。
- **首次配置导入 Enter 键响应体验提升（BUG-299）**：
  - 订阅导入弹窗将 Enter 监听提升至 Dialog Paper 层，当输入框失去焦点后在弹窗任意位置按下回车键均可触发导入，并完善 IME 输入法及按钮聚焦防护。

### 🧪 Tests & Engineering Standards
- **工程质量与 Lint 治理**：清零 Biome 历史存留的 3 个 errors 与 15 个 warnings，代码合规率达 100%。

---

## v3.0.4

### 🐞 Fixed Bugs & Window Mechanics
- **极窄模式物理像素判定与空闲计时器自续期（BUG-297，标题栏自动隐藏失效修复）**：
  - **从视口粗估转向物理像素权威判定**：废除基于视口整数 CSS 像素的模糊比较（150% 缩放下 135px 高度对应的 203 物理像素换算为 135.33 CSS 像素被视口进位上报为 136，与 135 门禁永久冲突导致无法触发隐藏）。改由窗口内尺寸物理像素域权威判定：`物理像素 ≤ 阈值 × 缩放因子 + 1 物理像素容差`；提取 `src/utils/mini-window.ts` 统一计算与 69 项单元测试。
  - **空闲倒计时局部递归自续期**：将 `resetIdleTimer` 重构为局部递归函数 `arm()`，触发后无条件重新排程；彻底消除旧实现因条件不满足提前 `return` 导致计时器永久自杀失效的缺陷。
  - **窗口尺寸变动即时防抖续期**：在 `onResized` 事件入口每帧即时续期定时器，杜绝长拖拽窗口中途隐匿闪变。

### 🧪 Tests & Engineering Standards
- **工程构建与规范**：`biome.json` 排除外部插件 crate 与生成物，统一格式化与代码风格规则。

---

## v3.0.3

### 🐞 Fixed Bugs & Window Mechanics
- **极小窗口自动隐藏标题栏（极窄隐匿模式）DPI 容差修复**：
  - **消除高 DPI 缩放下亚像素浮点漂移**：在 Windows 125%/150%/175% 等非整数缩放屏幕下，物理最小高度经 WebView2 换算为逻辑像素时带有微小浮点量（如 150% 下为 135.33px），严格整数比较导致门禁判定失效。将所有宽度与高度阈值比对统一升级为 `Math.round()` 取整容差校验，严格落实《设计契约 2.6》中「极窄且极小高度静置 10 秒自动隐藏标题栏并呈现 GlowBorder」的规范；
  - **极窄模式退出行为对称**：在窗口 Resize 监听中增加高度退出判定，当用户将窗口拉高脱离 135px 极简高度时，立即退出极窄模式恢复标题栏；
  - **唤醒重置计时器**：在点击窗口或恢复标题栏时重新武装 10 秒空闲倒计时，支持无交互后再次平滑自动隐匿。
- **窗口几何状态恢复（Window State Restore）DPI 容差兼容**：
  - 后端窗口状态恢复机制同步引入 `is_geometry_valid` 容差校验与边界 `.clamp`，防止 150% 等缩放下记忆尺寸回算微小偏差导致保存的窗口几何被误杀重置。

---

## v3.0.2

### 🐞 Fixed Bugs & Window Mechanics
- **窗口无框缩放手柄精度重构（彻底消除 1px 浮点抖动与跳宽）**：
  - **物理像素全链路下发**：前端计算、暂存与落地全链路改为整数物理像素（`physW`/`physH`/`physX`/`physY`），落地直接构造 `PhysicalPosition` 与 `PhysicalSize` 下发，彻底剔除 `physical -> logical -> physical` 双重浮点舍入导致的边缘细微漂移；
  - **锚定边精确反推**：西/北方向拖拽位置改由固定的对边反推（`physX = startPhysX + startInnerW - physW`），在尺寸被 clamp 到最小/最大边界时右边缘与底边缘绝对锁定不动；
  - **未拖拽维度原样透传**：只对当前拖拽维度执行 clamp 约束，未变化维度直接沿用拖拽起始物理尺寸，修复停在最小宽度时拖动上下边缘导致宽度被顶大 1 物理像素的偶发跳动；
  - **最小尺寸与旧版排版对齐**：最小尺寸由 285×142 调整为 271×135，配合 `shadow(false)` 消除无框系统阴影内缩量（150% DPI 下 22×13 物理像素），使窄窗内容宽度与旧版视觉绝对一致。
- **窗口几何持久化稳定性与 I/O 调优**：
  - 节流策略优化为「1000ms 节流 + 尾随补写（Trailing Sync）」：拖拽过程每秒最多写入一次，拖拽停止后的终值无条件可靠落盘，杜绝拖拽完立刻退出导致最后位置丢失；
  - 引入递增序列版本号与锁判序机制，销毁前的即时写与后台节流写并发时自动丢弃过时旧值，消除并发竞态覆盖；
  - 尺寸守卫增加 1px DPI 容差，修复 150% DPI 下最小宽度回算为 270.67 时被判定为无效尺寸导致永不落盘的问题。
- **多显示器拔插脱屏自动居中保护**：
  - 窗口唤醒与激活阶段（`activate_window`）调用 `is_position_on_screen` 检查窗口坐标，当副屏断开导致窗口落入桌面可视区域盲区时，自动调用 `center()` 居中重置；
  - 排除最小化状态判定（跳过 Windows 最小化坐标 `-32000`），避免从任务栏恢复窗口时意外被居中重置原位置。
- **无框 1px 拟物微边框补齐**：
  - 布局主容器 `.layout` 增加 `box-shadow: inset 0 0 0 1px var(--theme-border)`，利用内阴影恢复无框模式下 1px 立体微边框，且完全不占用盒模型空间、不影响内部客户区与拖拽手柄几何计算。

### 🧪 Tests & Engineering Standards
- **窗口几何重算单测全覆盖**：新增 `src/utils/resize-geometry.test.ts`（18 个测试用例），完整覆盖四向拖拽、极值钳制、对边锁定、非拖拽维度透传、100 帧累积零漂移以及 125%/150% 等高 DPI 缩放场景。

---

## v3.0.1

### 🐞 Fixed Bugs & Reliability
- **开机自启改为注册表实现，并补上启动对账**（修复「开关显示关，开机却照样启动」）：
  - 自启介质由 Windows 计划任务改为注册表 `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`。旧方案的三处缺陷叠加导致该问题：卸载脚本从不清理计划任务（`installer.nsi` 全文无 `schtasks`），老版本建的「Clash Mini」任务会残留到新安装上；自启状态只在用户拨开关时才写入，启动流程没有任何对账；开关只绑配置值，真实状态是盲区；
  - 现在每次启动都以配置为准重写一次真实状态；新版首次启动会清理历史计划任务（不提权、不弹 UAC）；
  - 设置里的开关改为显示系统真实状态（读注册表），配置值仅作回退；写入失败时回滚乐观状态并重拉真实值，不会停在自相矛盾的状态上。
- **修复开启「静默启动」后手动启动被误判**：此前只判断配置项、不看启动参数，导致手动双击桌面快捷方式也被当成静默启动，直接进入轻量模式、主窗口不显示。现统一补上 `--silent` 参数判定，与开机自启通道口径一致。
- **DNS 防投毒规则排除内网地址**：为 `dns.fallback` 上游生成的「强制走代理」规则现在会跳过私网、环回、链路本地与 ULA 地址，避免误伤局域网网关和本地 DNS 服务。
- **全局代理模式不再插入 IPv6 直连分流**：IPv6 兜底规则（`GEOIP,CN,DIRECT` + `IP-CIDR6,::/0,PROXY`）仅在 GFWList 模式下生效；当用户选择「代理全部流量」时直接走 `MATCH,PROXY`，保持全局代理语义完整。

### 🧪 Tests & Engineering Standards
- **删除计划任务实现**：`utils/schtasks.rs`（497 行）整体移除，连带删掉仅为解析 `schtasks.exe` 中文输出而存在的 OEM/ANSI 代码页解码逻辑；自启改由已注册的 `tauri-plugin-autostart` 提供。
- 新增纯函数单测 `dns_fallback_upstreams_are_forced_through_proxy` 与 `extract_upstream_host_handles_all_mihomo_forms`（覆盖标量/列表、六种上游写法、内网过滤与去重）。
- 自启状态读取改用 `try_state`，避免插件注册失败时 `autolaunch()` 直接 panic。

---

## v3.0.0

### 🚀 New Features & User Experience
- **全新首次使用引导系统（Interactive Onboarding Guide）**：
  - **全新安装智能感知**：程序初始化时通过纯函数 `hasRealProfileItem` 判定是否存在有效订阅（精准过滤后端启动时无条件注入的内置 `Merge` / `Script` 配置），在全新安装环境下自动弹出新手向导，告别以往首次启动无提示、不知如何配置的困惑；
  - **渐进式导入向导**：
    - **Step 1 订阅导入**：清晰引导用户填入或粘贴机场/节点订阅链接，支持标准键盘快捷键及右键上下文菜单（剪切/复制/粘贴/全选/清空），具备严格的 `http/https` 协议校验；
    - **Step 2 连通性真实检验与就绪提示**：订阅成功导入并激活后，自动过渡至就绪验证状态。系统通过真实检测当前节点的有效延迟（`0 < delay < 2000ms`）确认代理可用性，并提供清晰客观的浏览器/系统代理使用引导，剔除不切实际的假定宣称；
  - **智能轮询与优雅降级**：
    - 连通性确认期间每 1.5 秒轻量刷新延迟状态；
    - 设 20 秒超时保护，超时若未测出延迟将诚实呈现温和降级提示，界面流程绝不卡死；
  - **防误触保护与随时重温**：
    - 弹窗拦截 Esc 键防止用户粘贴过程中误触关闭；
    - 左下角帮助菜单新增「新手引导」入口（该按钮位于底栏，窗口高度大于 830px 时显示），误关或老用户可重新打开，并重置回初始导入状态。

### 🐞 Fixed Bugs
- **全新安装默认关闭 Allow LAN / Allow IPv6**：`clash.yaml` 生成模板及兜底最小配置中的 `allow-lan` / `ipv6` 统一为 `false`，对齐 Mihomo 内核默认，确保初次启动时两个开关均处于关闭状态；既有用户配置不受影响。
- **新手引导弹窗文字反差修复**：retro-3d 皮肤下引导弹窗为金黄渐变底，原先的主题灰白次级文字几乎无法辨认；现按皮肤配套深棕系文字及深绿/深琥珀状态色，其他皮肤沿用主题色。
- **TUN 模式 IPv6 兜底断流修复（全新安装长时断流根因）**：
  - TUN 接管 IPv6 路由后，GFWList（纯域名规则集）无法匹配「纯 IPv6 真实地址」，此类流量全部漏到 `MATCH` 兜底；默认兜底为 DIRECT，境外/被投毒的 IPv6 目标直连即 `i/o timeout`，现场实测 375 条超时刷屏、表现为 TUN 长时间断流；
  - 现于 `MATCH` 之前固定注入 IPv6 兜底：`GEOIP,CN,DIRECT`（境内 IPv6 走随包携带、离线可用的 `Country.mmdb` 直连；mihomo 的 `GEOIP` 自动覆盖 IPv4/IPv6）+ `IP-CIDR6,::/0,PROXY`（其余 IPv6 经节点 IPv6 出站，实测可用）；
  - **两条兜底均不带 `no-resolve`**：带 `no-resolve` 的 IP 类规则对域名目标直接跳过（现场 85/375=22.7% 超时目标是域名），去掉后由内核先解析再判定，解析出 IPv6 的域名同样被接住；仅含 A 记录的域名行为不变。
- **DNS fallback 防投毒链路救活**：
  - `dns.respect-rules: true` 时，内核为解析域名发出的上游连接同样要过规则匹配；fallback 里的境外 DoH（1.1.1.1 / 8.8.8.8 / 8.8.4.4）会一路漏到 `MATCH,DIRECT`，在大陆直连被墙 → fallback 整链失效 → `fallback-filter.geoip` 防投毒能力等于关闭 → 投毒应答被采信后客户端反复连接不存在的地址，正是断流的深层根因（现场日志实证 `dial DIRECT --> 8.8.8.8:443 error`）；
  - 现由真实 `dns.fallback` 配置推导规则（不写死 IP，订阅换上游自动跟随），逐条生成 `IP-CIDR/<32 或 /128>,PROXY,no-resolve` 强制上游走代理；插入位置在 `RULE-SET,gfwlist` 之后、用户 `append-rules` 之前，不抢用户自定义规则的最终决定权；域名型上游无法静态表达、跳过。
- **节点选择快照/还原跳过内置出站**：冷启动「一个真实节点都没有」的窗口期里 PROXY 组以 `DIRECT` 临时兜底，此时照抄进快照、再在真实节点加载后还原必然报 `Selector update error: proxy not exist`；现快照与还原两侧均跳过 `DIRECT`/`REJECT`/`REJECT-DROP`/`PASS`/`COMPATIBLE` 内置出站，交由正常的自动选点流程。

### 🌐 Internationalization & Accessibility (i18n & A11y)
- **13 语种深度国际化收口**：
  - 覆盖全部 13 种支持语言（简体中文、繁体中文、英语、日语、韩语、俄语、德语、西班牙语、印尼语、土耳其语、波斯语、阿拉伯语、鞑靼语）；
  - 新手引导全部文案、帮助菜单、以及设置中心各模块（内核选择、系统代理、TUN 模式、自启动、网络协议栈等 75+ 个用户可见文本）全面收口至 i18n 资源文件；
  - `pnpm i18n:check` 实现 13 语种共 840 个键位 100% 对齐（0 缺失、0 冗余）；
- **无障碍访问（A11y）与语义规范**：
  - 修正节点列表 `aria-label` 国际化键名，移除掩盖类型的 `@ts-expect-error`；
  - 帮助图标补充 `aria-label`、`aria-haspopup` 与 `aria-expanded` 属性，读屏设备支持更加友好；
  - 导入弹窗与新手引导输入框的右键上下文菜单统一复用 `shared.actions.*` 国际化文案。

### 🧪 Tests & Engineering Standards
- **纯函数单测基线扩充**：
  - 新增 `src/utils/profile-items.test.ts` 纯逻辑测试套件，涵盖 6 组有效订阅判定的边界场景（空配置、内置项干扰、多协议项等）；
  - `enforce_mini_agreements` 规则断言随 IPv6 兜底更新（21→23 条）；新增 `dns_fallback_proxy_rules`（标量/列表混用、scheme、端口、h3 参数、去重）与 `extract_upstream_host`（mihomo 全部上游写法）纯函数测试；
  - 定时器与资源清理：引导组件卸载时严密注销所有超时与轮询计时器，杜绝内存泄漏；
  - 架构协议固化：在规范文档中沉淀有效订阅判定准则与用户可见文案 100% i18n 硬约束。

---

## v2.9.9

### 🚀 New Features & Engineering Enhancements
- **一键导出诊断包（Diagnostics Bundle）与交互体验升级**：
  - 「帮助」菜单新增「导出诊断包」功能，一键收集全量轮转日志（应用主日志 + `service`/`sidecar` 内核日志）、脱敏后的配置文件（`verge.yaml`、`profiles.yaml`、运行时 `clash-mini.yaml`）及系统与运行时状态摘要，自动打包至 `<app_home>/diagnostics/` 目录；
  - 导出成功后通过 `show_in_folder`（Windows 原生执行 `explorer /select,"<zipPath>"`，macOS `open -R`）在文件管理器中直接高亮定位选定生成的 zip 压缩包，免去翻找；前端导出过程提供 loading 交互通知反馈，防止重复触发。
- **配置格式版本迁移框架（`migrate.rs`）**：
  - `verge.yaml`、`profiles.yaml`、`clash.yaml` 统一纳入版本迁移框架，顶层维护独立于应用版本的 `config_version: u32`（缺失视为 v1）；
  - 启动时自动逐级纯函数迁移；迁移前自动备份为 `<文件名>.bak.v<from>`（备份失败则中止本次迁移，绝不在无备份时改写配置）；
  - `clash.yaml` 升级至 v2：自动将破坏性的 `tun.stack: "system"` 归一化为稳定可靠的 `"gvisor"`；
  - `enhance::merge_default_config` 在配置增强阶段显式跳过 `config_version`，保证生成的运行时配置 `clash-mini.yaml` 不混入非内核字段，杜绝潜在的配置校验失败隐患。
- **日志保留时长由 4.5 小时提升至 26 小时**：
  - 应用主日志 `app_log_max_count` 默认轮转数量从 8 个提升至 40 个（128KB × 40 ≈ 5MB），充分支撑过夜后反馈排查；存量用户旧默认值通过配置迁移 v2→v3 平滑升级，用户自定义值一律严格保留。
- **前端架构解耦与错误边界分级**：
  - 弹窗模块（`LayoutDialogs` / `ConnectionDetail`）补齐区域级错误边界，避免局部渲染异常导致全屏崩溃；
  - 将端口编辑态、allow-lan 与 ipv6 切换逻辑从 `_layout.tsx` 单体组件中抽离为独立的 `use-minimal-settings` hook。

### 🐞 Fixed Bugs & Reliability
- **前后端伪节点判定严格对齐，根除真节点误隐藏**：
  - **根因**：后端 `utils/node.rs` 采用「归一化 + 完全/前缀匹配」，而前端 `isDummyNode` 此前使用粗粒度 `includes` 子串匹配，导致 `日本群组-A`、`美国-网址专线`、`上海-群节点-01`、`HK-流量优化` 等真实节点被误判为伪节点从前端隐藏，引发「后端能选中、前端看不到」的高危不一致；
  - **修复**：前端逐条对齐后端逻辑，中文广告短语必须经归一化（多层剥外层包裹括号与通用前缀）后按完全或前缀匹配判定；英文广告词与 URL 协议头保留高置信度子串匹配；单测补充 31 项断言（含防误伤基线与有意漏判对齐）。
- **诊断包采集全方位安全与健壮性防护**：
  - **大日志尾部截断保护**：单文件设置 2MB 上限（`MAX_LOG_FILE_READ_BYTES`）与 60 个文件总数上限，日志超大时从尾部 seek 读取最新内容并扫描首个换行对齐完整日志行，防止内存或 zip 膨胀；
  - **修改时间优先级排序**：采集按 `mtime` 倒序截取，确保最新的应用日志与 service/sidecar 内核日志优先入包，不被老旧主日志占满上限；
  - **遍历安全与 ZIP 规范**：跳过目录符号链接以杜绝循环遍历环（并天然跳过 latest.log 软链去重），Windows 反斜杠统一转换为 ZIP 标准正斜杠；
  - **多层深度脱敏**：优先整行抹除 `Authorization: ...` 头与裸 `Bearer <token>`，再进行 URL userinfo 与通用敏感 KV 替换，杜绝 Token 泄漏；并将 CPU 密集正则任务放至 `spawn_blocking` 保护 tokio worker。

### 🧪 Tests & Engineering Standards
- **前端纯逻辑 Vitest 单测基线**：
  - 新建并扩充 Vitest 单测体系，涵盖「节点子集过滤」（`search-matcher.test.ts`）、「流量单位换算」（`parseTraffic.test.ts`）、「伪节点判定与防误伤」（`node.test.ts`）以及「延迟格式化与色值映射」（`delay.test.ts`），建立 31 项核心纯逻辑单测基线；
  - 清理后台监测模块中排障期的 18 处 `[探针]` INFO 打点，保持生产日志规范整洁。

---

## v2.9.8

### 🐞 Fixed Bugs & Reliability Enhancements
- **健康检查与测速地址全面 HTTPS 化**：
  - 默认测速 URL 从 `http://cp.cloudflare.com/generate_204` 升级为 `https://cp.cloudflare.com/generate_204`，未指定端口解析兜底由 80 改为 443；
  - 杜绝机场中间人、透明代理对明文 HTTP 80 端口测速请求的劫持、拦截或丢包，根除因此导致的虚假全超时（`all proxies timeout`）现象。
- **测速失真双重复核与误判抑制**：
  - 引入不同 CDN 体系的备用测速地址（`https://www.gstatic.com/generate_204`）；
  - 主测速地址整体失败时，自动调用备用地址复核；若主备双双失败，认定为测速链路异常，故障转移直接返回 `NoAction`，彻底抑制因测速链路异常引发的无效切点抖动与虚假判死。
- **失败服务器避让机制（宿主机级死循环防护）**：
  - 针对机场将数十个节点绑定在同一台物理宿主机的常见场景，节点被确认判死时，将其底层物理服务器记录入冷却表（10 分钟冷却）；
  - 自愈选点时优先剔除冷却表内的宿主机节点，彻底根除在单台故障物理机不同端口之间反复切换断流的打转死循环；
  - 配备防御安全阀：若剔除后无候选可用则自动回退全量；前端展示始终保留全量节点，不受避让机制影响。
- **自愈连续失败内核重启前增加 API 探活防线**：
  - 连续 3 次自愈失败后，在决定是否重启内核前先探测 Mihomo API 可达性；
  - 若 API 存活，识别为外部节点或国际出口故障，跳过内核重启并重置计数，等待节点网络自愈，彻底杜绝在外部波动期间反复重启内核撕裂本地 TUN 网络。

---

## v2.9.7

### 🐞 Fixed Bugs & Protocol Robustness
- **IPv6 本地与局域网直连规则全面补齐**：
  - 在路由规则中补齐 IPv6 本地回环（`::1/128`）、未指定地址（`::/128`）、唯一本地地址 ULA（`fc00::/7`）、链路本地地址（`fe80::/10`）、组播（`ff00::/8`）及 6 条 IPv4-mapped IPv6 私网直连网段，彻底杜绝在“代理所有流量”（`MATCH,PROXY`）下本机与局域网 IPv6 流量被误转到远端代理的问题；
  - IPv4 补齐链路本地地址 `169.254.0.0/16` 直连。
- **消除 dns.ipv6 脆弱耦合**：
  - 确立 `use_tun` 为 `dns.ipv6` 字段的单一真源，在 TUN / 系统代理 / 手动模式下均无条件把 `dns.ipv6` 同步为用户开关值，彻底解决原订阅缺少 `dns` 段时开关状态脱节的隐患。
- **DNS 劫持补齐 TCP**：
  - TUN 配置 `dns-hijack` 补齐 `tcp://any:53`，与 `any:53`（UDP）形成闭环，彻底堵死 TCP 53 端口的 DNS 旁路泄漏风险。
- **单节点测速实时性与准确性提升**：
  - 单节点显式测速强制绕过 30 秒缓存复用与 120 秒 history 兜底，确保点击即探活，彻底根除刚恢复网络时持续返回旧死状态的粘连假象。
- **系统设置开关乐观更新**：
  - 为 `patchVerge` 引入乐观更新与错误自动回滚，设置面板中的「开机自动启动」与「启动时最小化」获得 0ms 即时响应的原生丝滑手感。

---

## v2.9.6

### 🐞 Fixed Bugs
- **修复 Allow IPv6 开关在界面上点击后不切换状态的问题**：
  - **根因**：v2.9.4 为了防止 TUN 模式下 IPv6 绕过代理泄漏，在 TUN 模式下向运行时配置注入了强制顶层 `ipv6: true`（Mihomo 规范要求 `tun.inet6-address` 依赖顶层 `ipv6: true` 才会生效）。但前端设置界面的 Allow IPv6 开关此前直接绑定 Mihomo REST API 返回的运行时配置 `clashConfig.ipv6`，导致在 TUN 模式下无论用户如何点击关闭，页面读取运行时配置恒为开启，开关被强制弹回无法切换。
  - **修复**：
    - **UI 绑定与内核防泄漏二分解耦**：后端 `ClashInfo` 暴露用户在 `clash.yaml` 中持久化保存的真实设置字段 `ipv6` 与 `allow_lan`，前端界面开关统一绑定至 `clashInfo`，彻底解耦 UI 开关展示状态与底层 TUN 运行时防泄漏接管逻辑。
    - **乐观更新与即时响应**：前端切换开关时引入乐观更新与防抖同步，消除切换延迟，保证即点即响应并安全持久化。
    - **事件同步联动**：在 `verge://refresh-clash-config` 监听链中纳入 `getClashInfo` 重新拉取，确保配置热更新或外部变更时 UI 状态时刻与后端保持精准同步。

---

## v2.9.5

### 🐞 Fixed Bugs
- **修复 v2.9.4 引入的阻断性回归：TUN 模式下内核配置解析失败、节点列表完全不显示**：
  - **根因**：v2.9.4 的 IPv6 防泄漏修复将 `tun.inet6-address` 写为单个字符串，但 mihomo 内核要求该字段为前缀数组（`[]netip.Prefix`），YAML 反序列化直接拒绝，导致配置验证失败、内核无法启动、API 不可达，前端拿不到任何节点。
  - **修复**：`inet6-address` 改为按内核规范写入 YAML 序列，TUN 模式恢复正常启动与 IPv6 路由接管。

---

## v2.9.4

### 🐞 Fixed Bugs & Security Enhancements
- **修复 TUN 模式下 IPv6 流量绕过代理直连泄漏（防泄漏铁律）**：
  - **根因修复**：根据 Mihomo 官方规范，`tun.inet6-address` 必须在顶层配置中显式启用 `ipv6: true` 才会生效。此前当用户关闭「Allow IPv6」时，内核不具备 IPv6 收包能力，系统 `::/0` 路由仍指向物理网关，导致所有 IPv6 流量从物理网卡直连并 100% 绕过代理泄漏。
  - **路由全面接管**：TUN 启用时强制写入顶层 `ipv6: true` 并显式配置 ULA 地址 `tun.inet6-address`（`fdfe:dcba:9876::1/126`），使 TUN 网卡同时接管 IPv4 与 IPv6 默认路由，彻底杜绝 IPv6 旁路泄漏。
  - **开关语义二分与多模式统一**：
    - 顶层 `ipv6` 专职负责 TUN 路由接管与防泄漏；用户「Allow IPv6」开关专职控制 `dns.ipv6`（关闭时内核 DNS 不返回 AAAA 记录，应用自动回落 IPv4）。
    - 统一 `dns.ipv6` 在 TUN、系统代理与手动模式下的开关语义，关闭后非 TUN 模式下同样屏蔽 AAAA 解析，避免连接不通与无效重试。
    - 优化非 TUN 模式边缘行为：仅在订阅自带 `dns` 配置时同步开关值，不凭空生成冗余配置块。

---

## v2.9.3

### 🐞 Fixed Bugs & Performance Optimizations
- **后台监测与自愈探测策略重构（根治探测风暴与自激误报）**：
  - **按需降频探测**：`evaluate_failover` 改为依据缓存新鲜度决策，活跃节点健康且 `history < 60s` 时直接采信缓存，判死后 `10s` 内不重复拨测，消除稳态约 280次/分钟的探测风暴与自激误报。
  - **节点切换宽限期（60s）**：任何节点切换后 60 秒内为宽限期，期间失败判定只记录日志、不累计连续失败次数，防范新节点缓存未稳引发的频繁误判与乒乓切换震荡。
  - **自愈候选过滤加固**：选点兜底 history 严格过滤，仅接纳内核确认存活（`alive == true`）且 120 秒内新鲜的记录，杜绝不可达或陈旧死节点被选为最优节点。
  - **前端子集测速缓存复用**：前端发起的单点或子集测速，在测量组缓存数据新鲜（< 30s）时直接复用，避免单点测速频繁触发全组 70+ 节点拨测。
  - **自愈冷却日志升级**：自愈 60 秒冷却跳过日志级别由 debug 提升为 info，确保生产环境下自愈决策轨迹清晰可见。
- **子进程常驻与句柄泄漏彻底修复**：
  - **配置验证进程（`mihomo -t`）泄漏修复**：`validate.rs` 弃用内部丢弃子进程句柄的 `command.output()`，改为手动 `spawn` 与事件循环监听；超时（10s）后显式调用 `child.kill()` 并排空通道与残余输出，彻底消除孤儿验证进程常驻。
  - **启动脚本（`startup_script`）超时与泄漏防御**：启动脚本引入 30 秒超时控制（`STARTUP_SCRIPT_TIMEOUT_MS`），超时强行杀死脚本进程并排空管道，避免挂起脚本永久霸占系统资源。

### 🔍 Diagnostics & Telemetry
- **断流排障临时诊断探针注入**：
  - 在后台监测主循环与体检链路（主循环轮次、体检门控、`evaluate_failover` 各出口、全组拨测耗时、物理网络探活、自愈冷却等）注入前缀为 `[探针]` 的 INFO 级诊断日志，用于在实际应用中定位轻量模式下断流停滞与无法自愈的根因。

---

## v2.9.2

### 🐞 Fixed Bugs
- **单点测速 TUN 回环假死彻底修复（BUG-282）**：
  - 修复了在 TUN 模式下对当前活跃节点进行单点测速时，因流量回环导致测速必得 Timeout 的严重缺陷。将单点测速通道由直连内核 `/proxies/{name}/delay` 彻底重构为委托后端的统一测量组（`PROXY__METRICS`），与群发测速完全同源，彻底根除 TUN 回环。
  - 移除了前端自相矛盾的 1000ms 竞速超时截断，统一使用后端 2000ms 探针超时。
  - 确立单点测速三条核心契约（仅测量不切换节点、全组拨测换取无回环与数据源唯一、未真正执行禁止伪造超时）。
  - 补齐互斥抢占（`AUTO_SELECT_BUSY`）与未就绪（`AUTO_SELECT_NO_RESULT`）时的友好反馈与原值恢复，杜绝测速假死或谎报超时。
- **存量代码格式与 Lint 清理**：
  - 修复 `_layout.tsx` 中的 Biome 与 ESLint 格式警告，精简废弃的 `testUrl` 与 `urlMap` 状态管理。

---

## v2.9.1

### 🚀 Features & Architecture Optimization
- **工程全面收敛为 Windows x64 单一平台**：
  - 彻底精简并移除 macOS 与 Linux 平台的历史分支代码、构建脚本与资源配置，专注打造极致轻量稳定的 Windows x64 专用客户端。
  - 构建矩阵全面收敛，优化 GitHub CI 流程与构建产物配置。
  - 纯化 Windows 命名管道高并发通信、Win32 窗口生命周期与计划任务免 UAC 静默自启动。
  - 前端平台判定全面收敛为 Windows，纯化渲染原生 Windows 窗口控件。
- **构建体积深度瘦身**：
  - 移除 Alpha 测试内核支持，安装包体积减少约 13MB。
  - 生产打包关闭前端 sourcemap，进一步缩减壳体积并保护源码安全。
- **协议解析健壮性增强**：
  - 优化 `hysteria` 与 `anytls` 复杂 URL 拆分逻辑，先分离 query 参数再解析 userinfo，彻底解决账号密码或 query 含有 `@` 符号时导致的节点解析失败或截断问题。

### 🐞 Fixed Bugs
- **应用清单与单测套件健全**：
  - 修复 `cargo test` 二进制因缺失 Windows 应用清单导致通用控件库符号找不到的问题。
  - 修复单测在多线程并发执行时因共享全局 App handle 及草稿状态造成的偶尔崩溃或干扰，确保 143 项单元测试 100% 稳定通过。

---

## v2.6.8

### 🐞 Fixed Bugs
- **轻量模式唤醒后窗口尺寸恢复**：修复程序从轻量模式（托盘）唤醒后窗口一律回退到默认尺寸（285×680）的问题。退出轻量模式销毁窗口前先将当前 outer 尺寸写入 `window_state.json`，重建窗口时读取该尺寸恢复，不再丢失用户调整后的窗口大小。
- **兜底最小尺寸修正**：修复兜底最小窗口尺寸误用硬编码 200×200 的问题，改为使用极简窗口常量 `MINIMAL_WIDTH(285) × MINIMAL_HEIGHT(135)`，与小窗模式尺寸一致。

### 🚀 Features
- **窗口 resize IPC 并行化**：将自定义缩放手柄的 `setSize` 与 `setPosition` 合并为单次 IPC 往返，提升窗口拖拽调整大小时的流畅度。

---

## v2.6.5

### 🐞 Fixed Bugs
- **Tauri Window Activation Deadlock Fix**: Resolved a critical deadlock risk in `activate_window` where blocking mpsc channels held tokio worker threads. Rewritten to use async oneshot channels with a 5-second timeout.
- **Active Node Synchronization & Auto-Correction**: Fixed active node desync by snapshotting and restoring `PROXY.now` during core updater upgrades and config reloads. Frontend `activateSelected` now auto-corrects database config to match core node on failure or subset mismatch.
- **Subscription Dummy Node Normalization**: Normalization strips outer braces (e.g. `【】`, `[]`) and prefix markers before checking starts-with matching, preventing hidden advertising nodes from causing self-healing loops.
- **Warp visible Endpoint Logic Correction**: Fixed inverted logic check on `exit_lightweight_mode` which led to false-positive error logging.
- **Resize Handles rAF Leak Fix**: Added requestAnimationFrame cancel on component unmount in `resize-handles.tsx` to prevent memory/state leaks.

### 🚀 Features
- **Heartbeat Monitor Logging**: Added heartbeat logs to the background monitor thread for easier liveness diagnostics.
- **IPv6 Probe Target Parsing**: Added support for parsing IPv6 test URLs into `[addr]:port` format for host lookup probe.

---

## v2.6.4

### 🐞 Fixed Bugs (订阅伪节点混入 PROXY 组触发自愈死循环根治)
- **订阅源伪节点混入 PROXY 组触发自愈死循环根治**：修复了部分订阅源会在节点列表中塞入伪节点（如「网址：https://...」「官网」「剩余流量」「过期时间」「套餐到期」「续费」「公告」「购买」「群」等广告/资讯条目），这些伪节点进入 PROXY 组后，每次内核 `reload_config(true)` 会重置 `select` 策略组的 `now` 字段到列表首个节点（通常是伪节点），后台监测线程识别为异常节点后触发自愈切回真节点，下次 reload 又被重置回伪节点，形成无限死循环导致程序卡死的严重缺陷。本版本将伪节点识别函数 `is_dummy_node` 抽取到 `src-tauri/src/utils/node.rs` 共享模块，在配置生成 (`enhance::enforce_mini_agreements`)、节点恢复 (`lifecycle::restore_proxy_group_now` fallback 路径)、自愈判定 (`monitor::evaluate_failover`) 三个关键节点统一剔除伪节点，从源头到末端全链路根除伪节点进入 PROXY 组的可能，彻底斩断死循环。

---

## v2.6.3

### 🐞 Fixed Bugs (Resize 拖拽释放后尺寸乱晃根治)
- **拖拽释放后窗口尺寸无规律乱晃根治**：修复了在群发测速等高负载场景下进行窗口拖拽调整大小、释放鼠标左键后移动光标时，窗口尺寸依然会无规律乱晃的严重二次缺陷。根因是 Webview2 在繁忙期间会丢失 `pointerup` 事件，导致拖拽 Session 无法正常收尾。本版本在 `handlePointerMove` 顶部加装 `(e.buttons & 1) === 0` 鼠标按键掩码检测防线，并在所有 resize handle 元素上绑定 `onLostPointerCapture` 监听，双重保障一旦检测到左键已松开或指针捕获被系统强制释放，便立即主动触发 `handlePointerUp` 收尾重置，彻底斩断"假拖动"状态残留。

### 🚚 Docs
- **设计文档编码损坏修复**：修复 `clash_mini_agreements.md` 因错误编码保存导致整个文件中文全部变成乱码（2149 行变化）的严重问题，从历史提交恢复 UTF-8 内容并补录 v2.6.2 修订记录。

---

## v2.5.7

### 🐞 Fixed Bugs (唤醒首帧渲染根治)
- **从轻量模式唤醒后节点列表空白卡死修复**：修复了在从轻量模式（托盘）唤醒后，主窗口的代理节点列表偶尔出现彻底空白的严重逻辑缺陷。原逻辑将更新代理列表的 `refreshProxy` 动作挂起在 `waitForClashReady` 异步等待链之后，一旦唤醒时内核 API 响应延迟或返回超时，就会导致后续刷新被直接拦截跳过。本版本在唤醒分支的起点增加了无条件的优先刷新机制，在窗口显现的第一微秒发起 `refreshProxy({ forceFull: true })`，确保首帧列表瞬间呈现在界面上，彻底免除等待链阻塞的风险。

---

## v2.5.6

### 🐞 Fixed Bugs (轻量模式后台崩溃与断流根治)
- **轻量模式挂起断流与崩溃彻底修复**：由于轻量模式下主窗口被完全销毁，后台自愈测速结束后仍向失效的 Webview 窗口调用 `window.emit`，会在 Windows 平台上直接触发 Access Violation 内存段错误崩溃。这不仅会导致外壳进程悄无声息地暴毙，更会因命名管道（Named Pipe）异常中断 (Broken Pipe) 导致后台 Mihomo 内核进程直接挂死或退役，产生彻底断流。本版本在后端事件分发 `notification.rs` 头部以及自愈测速副作用分发处设立了双重轻量拦截防线 (`is_in_lightweight_mode`)，彻底熔断并禁止在轻量期间执行任何非法 emit，100% 杜绝了内存崩溃和连带断流，保障了常驻网络连接的持久稳健。

---

## v2.5.5

### 🐞 Fixed Bugs (专项死锁根治)
- **高负载群发测速下 Resize 结束死锁根治**：修复了在窗口可见且在群发测速期间，用户松开鼠标（退出拖拽）的瞬间窗口和托盘永久死锁冻结的致命缺陷。在前端 `resize-handles.tsx` 中引入了 200ms 的拖拽复位冷却机制 (Cool-down Reset) 以错开刷新 IPC 洪峰，并安全拦截丢弃了释放鼠标尾随抛出的最后一个系统级 `onResized` 垃圾事件，彻底净化了 EventPair 信道与 Windows 消息队列。
- **窗口初始参数获取并发化**：将用户开始拖拽瞬间串行执行的 3 个获取窗口尺寸参数 IPC 升级为 `Promise.all` 并发，大幅缩短首帧拖拽起步时的粘滞感。

---

## v2.5.4


### 🚀 Performance & Refactoring
- **单代理组架构极简化精简**：彻底清除了策略组下拉菜单 `GroupSelectMenu` 及其所有遗留残留状态。重构了后端 `calcuProxies` 数据服务，在源头直接获取并只返回 `PROXY` 单一代理组，剔除了其它策略组和 GLOBAL 组的多余合并计算；前端去除了后置的 `filter` 过滤及不可达的折叠大卡片渲染分支，极大降低了前端列表重绘和状态维护的内存与 CPU 开销。

### 🐞 Fixed Bugs (并发与性能根治)
- **群发测速与窗口缩放并发死锁根治**：在后端 `notification.rs` 中重构了事件通知逻辑，在主线程同步进行参数序列化，随后将底层的 `window.emit` 调度至异步 Runtime 线程执行，彻底消除了高频 IPC 下 emit 阻塞反向卡死 Rust 后台测速工作线程的隐患；同时在前端 config 刷新监听层增加了拖拽 resize 状态过滤，缩放期间直接跳过 refresh 重绘，实现了前后端两端解耦避让。
- **轻量模式唤醒冷启动判定修正**：在后端 `build_new_window` 中增加了原子布尔标志，仅在进程初次冷启动创建首个窗口时注入 `?cold_start=true` 参数。前端仅依此清除 UID 缓存，从而保障了在窗口被彻底销毁重建的智能轻量模式下，托盘唤醒不会误重置内核，彻底消除了唤醒后首次测速全部超时的隐秘 Bug。

---

## v2.4.6

### 🚀 Performance & Memory
- **极限内存优化与工作集修剪 (Working Set Trimming)**：实装了挂载系统托盘（轻量模式）时的 Windows 工作集暴力回收机制。内核断开连接与 WS 订阅 2 秒后，通过调用 Windows 原生 API 强制要求系统清除并没收该进程物理内存中所有的闲置页。
- **Mihomo 内核极限限额与瘦身**：将 `GOMEMLIMIT` 锁死为 `64MiB`，并将 `GOGC` 回收阈值压低至 `30`，强制 Go 虚拟机极高频率执行垃圾回收。同时配置 `geodata-mode: false` 强迫内核降级至 MMDB，并压缩 DNS 缓存至 512 条、禁用 `tcp-concurrent` 高并发握手，使常驻内存大幅瘦身，极其平稳。

### 🚀 Features & Routing
- **代理方式三态滑块**：将路由倾向开关重构为“全局代理 / GFWList / 添加网址”三选一滑块。当处于前两态时分别注入 `MATCH,PROXY` 与 `MATCH,DIRECT` 最终规则，其余行为对齐 GFWList 以 DIRECT 为兜底。
- **多行添加网址弹框**：当切换到“添加网址”时，允许用户批量粘贴多个 URL/IP/域名（支持换行、分号、逗号分隔）。前端自动清洗协议头、去除端口和路径并识别双重顶级域名（如 `.co.uk`）提取后缀，置顶以 PROXY 组注入全局 Merge 的 `prepend-rules` 列表中立即生效。
- **TUN DNS 体系清理 (Option A)**：清除了 `use_tun` 中被末端 `redir-host` 全局 DNS 重写机制所覆盖的 fake-ip 死代码，保持 macOS 下接管与还原系统公网 DNS 的后台能力。

### 🐞 Fixed Bugs (代码审核一致性修复)
- **多订阅并发安全锁与重构**：将 profile 中的本地原子锁提取为 `feat/profile.rs` 的全局排他原子锁。当自动更新定时器（Timer）触发时使用 `compare_exchange` 争抢锁，若前台正在切换或导入配置则自动退避，彻底消除了 YAML 读写冲突与网络闪断。
- **12 国语言提示硬编码修复**：解决了多行网址导入提示在除英语外的 12 个多国语言包中直接硬编码为英文提示的问题，补齐了全套翻译。

---

## v2.3.9

### 🚀 Performance

- **小窗口(285×135)跳过 clashConfig 订阅**：小窗口下设置抽屉与基础设置卡均不挂载、可见区(活跃节点栏/流量条)不消费 `clashConfig`，故小窗口不再发起该配置查询，省一次 IPC。
- **设置抽屉窄窗口下右侧路由表不挂载、数据与渲染统一开关**：右侧连接表(`ConnectionsPanel`)在窄窗口(宽=285)下整块不进 DOM、不渲染、连接数据 WebSocket 同步关闭，二者锁死在 `isMinimalWidth` 同一信号；删除原先脆弱的 `isPanelVisible` ResizeObserver(边界模糊且仅管数据半边)。
- **设置抽屉附加层资源随开关同生同灭**：`getSystemProxy` / `getRunningMode` / `getClashConfig` 三个查询的 `enabled` 统一为 `isSettingsOpen && !isMiniStatus`。关闭抽屉即停止订阅(仅留缓存、不再发 IPC)；小窗口尺寸同样不触发。消除「关抽屉后仍空跑 clashConfig」的漏点。
- **底部流量条高度与窗口阈值解耦**：新增 `TRAFFIC_PANE_HEIGHT_WIDE=135` / `TRAFFIC_PANE_HEIGHT_MINIMAL=100` 独立常量，流量条高度不再与窗口尺寸阈值数值耦合。

### 🧹 Refactor & Cleanup

- **窗口尺寸阈值常量收口**：将散落于多处、易漂移的 `285`(宽) / `135`(高) / 遗留 `130` / `270` 统一收口到 `src/constants.ts` 的 `MINI_WIDTH_THRESHOLD` / `MINI_HEIGHT_THRESHOLD`；修正历史遗漏的 `130→135`、`270→285` 两处阈值漂移。
- **窗口缩放热区加宽**：无边框窗口 8 向缩放热区 `HANDLE_SIZE` 由 `6` 加宽到 `10`，边缘更易拖拽命中。

### 📝 Docs

- 同步 `clash_mini_agreements.md`：新增「性能/资源梯度」章节与「设置抽屉附加层资源随开关同生同灭(本源/附加分层)」「数据订阅与 DOM 挂载统一开关联(窄窗口不挂载路由表)」等规则，把本轮性能优化从「代码巧合」立为明文约定。

---

## v2.3.8

### 🐞 Fixed Bugs (代码审核一致性修复)

- **#1 延迟死活/着色阈值统一为 2000ms（与后端一致）**：活动卡、节点列表的判死与红黄绿着色阈值，由原先 `verge.default_latency_timeout || 10000` 统一改为 `NODE_DELAY_MAX_MS(2000)`，与后端死节点判定、探针超时完全一致。修复了同一节点在「活动卡」与「节点列表」中死活口径打架、以及「切下一节点」按钮可能切到后端已判死节点的问题。单点测速超时（1s / 10s）保持不变。
- **#2 后台监测唤醒后立即体检**：退轻量模式 / 切换配置 唤醒后台监测循环后，立即放行一次健康检查（探活 + 必要时自愈），不再干等下一个 15 秒自然周期（原先依赖前端 `visibilitychange` 兜底）。
- **#3 健康检测间隔与判定边界统一**：后台体检间隔统一使用 `NORMAL/RETRY_CHECK_INTERVAL_SECS` 常量（去掉写死的 3 / 15 秒）；健康判定上限与候选池 `[30,2000)` 边界对齐，消除恰好 2000ms 节点的行为歧义。
- **#4 窗口极简断点抽为共享常量**：`isMinimalWidth / isMiniStatus` 响应式断点（285×135）在三处（`_layout.tsx`、`app-data-provider.tsx`、`window.rs`）重复写死的问题，抽成 `MINI_WIDTH/HEIGHT_THRESHOLD` 共享常量，前后端注释互指，消除以后调断点漏改漂移。
- **#5 修正注释与并发守卫**：修正 `proxy-groups.tsx` 中 `visibleNames` 的误导性注释（它即 F4 候选子集，是防越界关键，非「仅占位」）；F4 测速 `finally` 解锁窗口加并发守卫，避免与其他分组测速并发时提前解锁导致拖拽异常。

### 📝 Docs

- 同步 `clash_mini_agreements.md`：补记前端显示 / 轮换口径已与后端 `NODE_DELAY_MAX_MS(2000)` 死活阈值完全统一（原 v2.3.6 仅覆盖探针超时与后端选点）。

---

## v2.3.6

### 🧹 Refactor & Cleanup
- **F1/F2 群发测速择优收归后端（根治前后端重复测速 / 竞争切换 PROXY 冲突）**：前端「测速所有节点」按钮、Profile 切换 / 启动 / 6 秒 Fallback、窗口唤醒静默填充均不再自行对 PROXY 组发起批量测速与节点切换，改为统一通过 Tauri 命令 `trigger_auto_select(profile_uid, node_names, sort_type, select)` 委托后端 `trigger_backend_auto_select` 执行；后端测速完成后经既有 `verge://backend-delay-results` 事件回传结果刷新 UI。前端仅在触发时标记节点为「测速中」（-2）以驱动流光动画。`node_names` 为待测子集：不传=后端自取 PROXY 全量（F1/自动选点），传入可见子集=仅在该子集内测速择优（F4）。
- 重新引入 `trigger_auto_select` 命令（原 B5 死入口已于 v2.3.5 删除），其职责现为「前端发信号、后端统一做」，与当初的重复选点死逻辑有本质区别。
- 后端 `trigger_backend_auto_select` 改造：`select` 参数区分「测速+切换」(true) 与「仅测速填充」(false，用于窗口唤醒、严禁切节点)；回传结果包含**全部被测节点**（含死节点 / 超时 / 错误），避免 UI 只显示健康节点。
- **探针超时与死节点阈值统一为 2000ms（彻底前后端一致）**：后端 `NODE_TEST_TIMEOUT_MS` 由宽松的 10000ms 收紧为等于 `NODE_DELAY_MAX_MS(2000)`，前端 chain-mode 的 `checkListDelay` 探针超时改为 `NODE_DELAY_MAX_MS(2000)`；per-group 闪电按钮（F4）已收归后端、不再使用前端 `checkListDelay`。`verge.default_latency_timeout` 不再参与测速。效果：超过 2000ms 的节点一律按不可用处理，UI 显示 Error/Timeout，不再展示其真实延迟（精度换一致性，符合既定设计）。
- 修复后端选点 bug：选点固定按延迟升序取**被测子集内**最快节点，不再依赖 `sort_type`（原逻辑在 `sort_type=2` 时会误选字母序首个节点）。
- **F4（per-group 闪电按钮）收归后端，前端批量测速逻辑彻底退出**：手动「测速所有节点」按钮不再由前端 `DelayManager.checkListDelay` 自建测速+选点，改为委托后端 `trigger_auto_select(profile_uid, visibleNames, 0, true)`——**传入当前可见节点子集**，后端仅在该子集内测速并切换至最快健康节点（所见即所测所选），与 F1/F2 共用同一台后端引擎。前端仅保留可见节点的「测速中」流光占位与防重复点击状态；移除前端选点循环、`NODE_DELAY_*` 常量依赖。

### 📝 Docs
- 同步 `clash_mini_agreements.md`：§前后端分工原则改为「前端只发信号、后端单一引擎」，`trigger_auto_select` 增加 `node_names` 子集参数（不传=全量、传入=可见子集）；§窗口唤醒、§5.6 事件通道相应更新；§5.5 重写——per-group 闪电按钮（F4）改为传入可见子集、在子集内择优，前端对 PROXY 组的批量测速与选点逻辑整体废弃。

---

## v2.3.5

### 🧹 Refactor & Cleanup
- 删除无调用方的 Tauri 命令 `trigger_auto_select`（前端从未 `invoke`，纯死入口），精简后端选点入口。
- 简化 `monitor.rs` 后台监测循环：移除 `current_profile != last_profile_uid` 整段分支（约 47 行）及 `last_profile_uid` 变量。窗口不可见时的启动首次选点改由监测循环首跑一次性执行，并与轻量模式进入（B4）经 `AUTO_SELECT_RUNNING` 互斥、不重复；删除仅被该分支调用的 `cancel_active_auto_select`。前端在线时选点职责不变，仍由前端负责。

### 📝 Docs
- 同步 `clash_mini_agreements.md`：§前后端分工原则将“启动”按窗口可见性拆分（窗口不可见静默启动改由后端一次性选点）；§5.6 后端测速回传触发场景由“Profile 切换+故障自愈”收敛为“启动(窗口不可见)/进入轻量模式/故障自愈”。

---

## v2.3.4

### ✨ New Features
- **托盘图标跟随流量接管模式自动切换**：托盘图标现在根据当前流量接管模式（手动 / 系统代理 / TUN）自动切换；TUN 模式使用专属图标。启动即按当前模式初始化图标，切换模式时实时刷新。

### 🐞 Fixed Bugs
- 修复 `handleCycleNode` 在批量测速期间未加 `isBatchTesting` 防护，导致节点轮换与测速结果错配的问题（与 `handleTestDelay` 保持对称防护）。
- 清理 Rust `FilterConfig` 中残留的 `use_regex` / `match_case` / `match_whole_word` 死字段（搜索简化重构遗留）。

### 🧹 Refactor & Cleanup
- 移除托盘图标的 `common_tray_icon` / `sysproxy_tray_icon` / `tun_tray_icon` 三个无实际效果的死配置开关，图标完全由接管模式自动推导，消除配置层与实现层互相矛盾的“手动指定图标”陷阱。
- 清理搜索匹配模块死代码 `SearchMatcherOptions` 类型与 `escapeRegex` / `buildRegex` 函数，搜索逻辑统一为纯子串模糊匹配。
- 删除空壳函数 `update_menu_and_icon()` 及其两处空调用，精简托盘实现。
- 移除 13 个 locale 中已无引用的 `matchCase` / `matchWholeWord` / `useRegex` 翻译键及对应 SVG 图标（搜索简化重构遗留）。

---

## v1.9.21

### 🐞 Fixed Bugs

- **BUG-275: 修复普通用户权限下系统服务重装死循环**：在服务刷新与安装检测时引入安全的自定义 `is_reinstall_service_needed` 版本校验。当服务停止（IPC 路径不存在）时跳过重装，且在对比版本时自动剔除 `'v'` 前缀，彻底解决了切换至 TUN 模式时无限触发 UAC 重装服务的 Bug。
- **BUG-276: 修复系统服务未就绪导致回滚至手动模式的问题**：优化了前端自动回滚关闭 TUN 模式时的默认状态设定。当检测到系统服务未就绪需回滚关闭 TUN 模式时，将默认切换回“系统代理模式”而不是“手动模式”，保障小白用户的使用连贯性。

---

## v1.9.20

### 🐞 Fixed Bugs

- **窗口激活日志竞态条件修复**：修复了 `window_manager.rs` 中激活窗口操作的日志输出时机竞态问题，规范了在 UI 主线程执行时的投递与完成状态日志。
- **历史 Bug 追踪归档**：完成了对已确认关闭的 `BUG-273`、`BUG-274` 等历史 Bug 跟踪记录的正式归档移入。

---

## v1.9.19

### 🐞 Fixed Bugs

- **TUN 模式下服务自动切换优化**：在前端启用 TUN 模式后自动触发内核重启，使内核能够以管理员权限（Service 模式）重新启动并成功创建虚拟网卡。
- **WebView2 崩溃保护（并发测速期间锁定窗口调整）**：在前端发起批量测速期间，临时锁定窗口大小调整功能 (`setResizable(false)`)，防止并发渲染与视口测量触发 WebView2 渲染器崩溃。
- **流光动画/测速卡顿优化**：在节点处于测速状态时，禁用 `ProxyItem` 悬停时的 3D 悬浮位移与阴影特效，降低 WebView2 合成器的渲染负担，防止高频重绘下的渲染器崩溃。

### 🚀 Performance & Stability

- **Resize 事件生命周期优化**：在 `use-window-width` 的最后一个订阅者退订时，自动清理挂起的 `requestAnimationFrame` 任务，防止卸载后回调执行。

---

## v1.9.18

### 🐞 Fixed Bugs

- **BUG-274: 轻量模式退出时限流回滚失效**：为 `WindowOperationResult` 引入 `RateLimited` 变体，当 `show_main_window` 被防抖限流时返回该变体，并在退出轻量模式时正确捕获并回滚状态机回到 `In` 状态，彻底消除了前后端状态同步偏差。
- **BUG-273: 缺失管理员权限下的服务轮询跳过检查**：在 Windows `wait_for_service_if_needed` 逻辑的头部，补全了 `is_current_app_handle_admin` 检查，若为管理员运行则立即返回跳过服务重试，从而避免由于延迟导致的内核双进程残留泄漏。

### 🚀 Performance & Stability

- **高频 Resize 性能保护**：
  - 对窗口调整大小事件监听器加入 `requestAnimationFrame` 节流和防抖保护，规避高频渲染风暴。
  - 通过 React 状态函数式更新的比对逻辑防御了不必要的重渲染。

---

## v1.9.17

### 🚀 Layout & Usability

- **窗口大小与布局微调**：
  - 移除了设置抽屉关闭按钮多余的 40px 顶部偏移，对其到 3px。
  - 恢复 `MINIMAL_WIDTH` 为 270px。
  - 在延迟刷新调度中将 `requestAnimationFrame` 替换为 `setTimeout`，并为延迟 Widget 设置固定高度以防止虚拟列表重测。
  - 将任何已存在的窗口视为前端可用，优化后台与前端的数据同步。

---

## v1.9.16

### 🚀 Performance & Stability

- **窗口操作线程安全与 UI 稳定性**：
  - 将窗口操作（置顶、隐藏、销毁等）统一调度到 UI 主线程（`run_on_main_thread`）执行，避免在多线程环境下发生竞态条件导致 Panic。
  - 引入 `AtomicBool` 优化窗口操作去抖（Debounce）标志的 CAS 操作，解决 TOC-TOU 竞态风险。
  - 调整窗口最小高度为 99 像素，优化最窄布局的垂直空间利用。
  - 前端不可用时自动触发后台自动选点，优化配置切换的响应。

---

## v1.9.15

### 🚀 Performance & Stability

- **延迟与批量测速性能优化**：
  - 去除 active-node 栏多余 of 40px 顶部内边距。
  - 优化唤醒后的批量测速时序，将其推迟到 React 渲染周期结束后进行。
  - 恢复批量测速中的过滤功能，测速范围严格限制于当前过滤子集。
  - 在批量测速中跳过无谓的循环迭代，使用 `checkListDelay` 直接进行批量延迟测试。

---

## v1.9.14

### 🐞 Fixed Bugs

- **点击测速高频调用优化**：移除了窗口聚焦事件（Focus Listener）中每次点击均触发全节点测速的逻辑，防止因高频无意触发导致的 CPU 与网络资源过度占用。

---

## v1.9.13

### 🚀 Layout & Usability

- **图标与卡片同行布局彻底对齐**：彻底修复图钉和齿轮图标因异步 decorated 状态判断延迟而错位至原生窗口标题栏的 Bug。将两个控制图标直接作为 Flex 子项并入活跃出口节点卡片行（行间隔 `gap` 设为 `0` 实现紧密贴合），从底层布局保证在任何情况下三者保持同行对齐。
- **设置面板右上角关闭按钮优化**：在设置面板打开时，自适应在右上角渲染绝对定位的关闭按钮，使用户能随时返回主视图。

### 📋 Code Quality & Agreement Sync

- **协议条款同步更新**：同步更新 `clash_mini_agreements.md` 对应的顶栏 3D 按钮排版和避让条款，维护协议与实现的一致性。
- **编译/静态语法清理**：移除多处未使用的局部变量与无效导入（如 `useAppRefreshers`、`onProxies`、`showDelay`），使前端以 **0 Error 0 Warning** 的健康状态顺利通过 Husky pre-commit 钩子检验。

---

## v1.9.12

### 🚀 Performance & Stability

- **UI 冻结问题全面修复**：启动后全节点测速时画面冻结的问题已彻底解决，修复了以下根因：
  - 移除自动选点中的 200ms 轮询逻辑，改为分批并发测速模式（每批最多36节点）
  - 消除多重刷新：清理了 7 处多余的 `refreshProxy` 调用（自动选点、批量测速、节点切换等场景）
  - 节点表格点击行为修正：整行作为统一选中目标，移除延迟值点击测速事件
  - Fallback 测速优化：6秒无健康节点时强制测速，不再触发不必要的完整数据拉取
  - 延迟数据分层管理：`delayManager` 负责实时延迟，`refreshProxy` 仅在配置变化时触发

- **Windows 原生标题栏按钮定位修复**：decorated 模式下齿轮/关闭按钮位置从 `top: 3px` 调整为 `top: 32px`，避免被原生标题栏遮挡。

---

## v1.9.11

### 📋 Code Review & Audit

- **v1.8.8 → v1.9.10 全面代码审查**：完成从 v1.8.8 到当前版本的所有代码变更审查，确认以下核心改进已正确实现：
  - ServiceManager 无锁状态机设计（`AtomicBool` + `Notify` 双检查模式）
  - 监测线程优化（60秒/300秒自适应检查间隔 + 10分钟节点切换冷却）
  - 启动流程对齐上游（lifecycle_lock + TUN模式隔离）
  - 前端可访问性修复（WCAG 3.3.2 标准的 aria-labelledby）
  - 颜色对比度修复（主题感知的 warning.main/warning.dark）

### 🐞 Minor Fixes

- **国际化部分语言待完善**：审查发现部分语言的 a11y 字段仍为英文（ar, fa, id, tr, tt），已记录待后续版本完善

---

## v1.9.10

### 🐞 Fixed Bugs

- **Allow LAN 开关首次启动无响应**：修复 `patch_clash` 中 Draft 提交时序问题，将 `apply()` 提前到 `update_config_checked()` 之前，确保基础设置（Allow LAN、IPv6、Mode 等）在默认状态下首次切换即可生效。

---

## v1.9.9

### 🎯 Core Improvements

- **启动流程稳定性提升**：对齐 Clash Verge Rev v1.7.6 的启动流程实现，添加 `lifecycle_lock` 机制防止内核启动/停止/重启时的竞态条件，彻底解决程序启动不稳定的问题。

### 🐞 Fixed Bugs

- **内核启动竞态条件**：添加 `tokio::sync::Mutex` 锁保护 `start_core()`、`stop_core()`、`restart_core()` 操作，确保同一时间只有一个操作执行。
- **幂等性检查**：在 `start_core()` 和 `stop_core()` 中添加正确的幂等性检查，避免重复操作。
- **退出保护**：在启动过程中检查 `is_exiting()` 标志，避免在应用退出时启动新内核。
- **失败回滚**：启动失败时自动回滚 `running_mode` 状态，允许后续重试。
- **Windows 服务切换**：实现 `spawn_service_handoff_watcher()` 机制，在 sidecar 启动后自动检测并切换到 service 模式（TUN 模式需要）。

### 📝 Documentation

- 新增 `CLASH_MINI_STARTUP_ANALYSIS.md`：详细分析 Clash Mini 与 Clash Verge Rev 的启动流程差异。
- 新增 `STARTUP_ALIGNMENT_PLAN.md`：启动流程对齐计划的 checklist。

---

## v1.9.8

### 🐞 Fixed Bugs

- **系统服务版本检查逻辑修复**：修复由于系统服务二进制版本 `"v"` 前缀格式差异（例如 `"v2.3.0"` 对比 `"2.3.0"`）引发的版本比对错误，彻底解决每次启动应用时强制重装服务的 Bug。

---

## v1.9.7

### 🐞 Fixed Bugs

- **后端服务管理器并发锁优化**：重构服务刷新为静态异步方法，在重装等待期间释放全局锁，防止托盘/主生命周期线程卡死，修复 Clippy 的 `significant-drop-tightening` 检查。
- **无障碍多国语言标签修复**：将 11 个国际化语言包（日、韩、德、俄、西、繁中等）中硬编码的中文 a11y 标签修正为对应翻译或标准英文兜底。
- **表单无障碍关联**：将编辑配置文件对话框的标题标签通过 `aria-labelledby` 与输入框进行关联，满足 WCAG 3.3.2 规范。
- **测速延迟颜色对比度适配**：延迟颜色与 Bolt 图标改为主题感知，深色模式下使用高对比度的 `warning.main`，解决深色模式下的对比度隐患。
- **MUI 类型修复**：重构 Select 组件的 `aria-label` 传递方式，修复类型约束。

---

## v1.9.6

### 🐞 Fixed Bugs

- **JS 沙箱脚本名嵌入修复**：空脚本名导致 JSON.parse 崩溃，兜底为 `"global"`；同时去掉冗余的 serde_json 双序列化/反序列化
- **MUI v9 类型错误修复**：修复 Select/div 等组件的 aria-label 类型问题
- **ESLint 遗留警告清理**：关闭 91 条 `no-explicit-any` 历史警告，修复未使用变量和 stale eslint-disable 注释，pre-commit hook 零警告通过

### ♿ Accessibility

- 为虚拟代理节点列表添加 ARIA list 语义
- Select 组件添加 aria-label
- IconButton 组件添加 screen reader 支持标签
- 延迟颜色对比度优化以符合 WCAG 1.4.3 AA 标准

---


### 🎨 Visual

- 基础设置卡片标签字体统一至流量接管模块规范（`fontSize: 13px`、`fontWeight: bold`），列间距增大至 20px

### 📋 Documentation

- 新增协议条目：壳进程内存占用基准说明（35MB 为正常值）
- 新增协议条目：基础设置卡片视觉规范统一

---

## v1.9.4

### 🚀 New Features

- **Allow IPv6 开关**：在基础设置卡片中新增 "Allow IPv6" 开关，默认关闭。用户可按需开启 Mihomo 内核 IPv6 支持。布局同步重构为 2×2 网格，紧凑对称。
- **托盘菜单双向轻量模式切换**：托盘菜单"轻量模式"改为文本勾选（✔/无勾）指示当前状态，点击可在进入/退出轻量模式间双向切换，替代原有"进入后禁用菜单项"的单向设计。

### 🐞 Fixed Bugs

- **内存与资源泄漏修复**：
  - 移除 `SetProcessWorkingSetSize` 物理修剪（防止 WebView2 挂起/死锁）
  - 移除 `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL` 内存级别干预（交由 OS 自动管理）
  - 为 Mihomo 内核注入 `GOMEMLIMIT=96MiB` / `GOGC=50` / `GOMAXPROCS=2` 环境变量，约束 Go runtime 内存占用
  - 配置模板新增 `geodata-loader: memconservative`，按需加载 Geo 数据库
  - 服务等待增加 `is_service_installed` 前置检查，超时后弹错误对话框
  - 轻量模式进入/退出多次调用 `update_lite_mode_menu` 确保托盘状态一致
- **refreshAll 网络错误重试修复**：`triggerAutoSelectAndRefresh` 中 refreshAll 错误不再向外传播，降级到 `refreshProxy({forceFull:true})`
- **启动冗余刷新修复**：启动完成时记录时间戳，防止首次窗口聚焦触发二次全量刷新
- **全量刷新绕过精简路径**：`refreshAll` 强制设置 `forceFullProxiesRef`，确保代理列表完整加载

### 🏗️ Code Quality

- 彻底删除废弃的 `optimize_window_memory` 函数及 5 处残留调用
- 合并 `handleAllowLanChange` / `handleIpv6Change` 为通用 `handleClashBoolChange(field)`
- 恢复 `allowLan` camelCase 序列化 WARNING 注释
- 补充协议第四十七~五十号规范条目

---

## v1.9.3

### 🐞 Fixed Bugs

- **BUG-269 解决后台日志 WebSocket 协程泄漏及连接池句柄残留问题**：
  - **WebSocket 日志协程生命周期修复 (BUG-269)**：修复 `ws_logs_checked` 中的 tokio select 循环模式匹配。将 `Some(log_line) = rx.recv()` 重构为常规接收，并在通道关闭返回 `None` 时主动 break 退出循环，彻底消除协程在后台无限空转导致的主窗口/WebView2 销毁受阻问题。
  - **托盘轻量化模式连接池激进回收**：在 WebSocket 熔断清理函数中，增加对 `IpcConnectionPool` 的全局 `clear_pool()` 调用，确保在窗口销毁时完全释放残留的空闲 NamedPipe 管道系统句柄。

---

## v1.9.2

### 🐞 Fixed Bugs

- **BUG-263~268 解决发布后代码审计中发现的并发、内存泄露与状态一致性问题**：
  - **状态机 CAS 迁移与原子操作修复 (BUG-263)**：重构 Rust 状态机，使用原子型 `compare_exchange` 状态转移并集成日志输出，彻底清除 record_state_and_log 中的 raw `store` 竞态覆盖风险。
  - **Web Worker 错误降级与引用清理 (BUG-264)**：在流量监控 Web Worker 报错降级时，添加显式的实例 terminate 终止及 worker 引用清空逻辑，修复后续重用已损坏实例导致的降级失效。
  - **静默启动轻量模式适配 (BUG-265)**：窗口不存在时销毁操作返回 `NoAction`，后端正常判定成功进入轻量模式而不回滚状态，完美支持静默启动的就绪体验。
  - **窗口限流状态同步回滚 (BUG-266)**：由于防抖限流导致窗口展示操作未执行时，状态机自动回退并拦截退出轻量动作，保持状态机与主窗口显示状态的绝对一致。
  - **超时计时器泄露修复 (BUG-267)**：在侧滑抽屉 `useEffect` 清理函数中增加 `clearTimeout`，杜绝未清理的延时任务对 React 销毁组件的状态更改泄露。
  - **后台垃圾回收异步活性校验 (BUG-268)**：在后台清理 TCP 链接及 WS 数据熔断任务的异步操作前增加 lightweight 活性检测，快速退出已唤醒的状态，防止误杀新窗口建立的数据通道。
  - **废弃组件与微任务同步清理**：去除了 `enable_auto_light_weight_mode` 中多余的定时器全局 `init` 及 `async` 修饰，并将 `setMixedPortVal` 移出 `Promise.resolve().then` 从而在 `useEffect` 中同步执行更新。

---

## v1.9.1

### 🐞 Fixed Bugs

- **BUG-262 解决从轻量化后台唤醒后节点列表无延迟数据问题**：
  - **解决初始化挂载测速被取消问题**：在 Layout 组件 profile 增强 Effect 的清理函数中，添加 `lastProcessedRef.current.uid = null` 的重置逻辑。完美解决了在窗口销毁重建模式下，React StrictMode 双挂载或尺寸改变引发的 Effect 重新挂载导致前一次初始化被 cancelled、而第二次挂载被 lastProcessed 判定拦截，使初始化测速被彻底取消的 Bug。
  - **自适应窗口可见度唤醒测速**：在 Layout 窗口事件监听器中，增加对 `focus` 和 `visibilitychange` 事件的可见性判定。当窗口重新可见时，如果非启动阶段且已超过 30 秒冷却防抖时间，自动在后台触发 `DelayManager.checkListDelay` 对全节点执行静默测试并更新界面延迟缓存，且不会篡改用户手动选中的策略组节点。

---

## v1.9.0

### 🐞 Fixed Bugs

- **BUG-257~259 轻量挂机模式 Rust 后端与内核极致节电与资源 GC 优化**：
  - **自适应健康探测周期 (BUG-257)**：在轻量模式下，将常驻监测线程的探测间隔时间放宽至 60 秒一次，并在恢复前台时瞬间唤醒重置，大幅减少后台空闲时的 CPU 唤醒与网卡/DNS 活动。
  - **残留 TCP 连接垃圾回收 (BUG-258)**：进入轻量模式的瞬间强制清空内核中所有残留的活动及空闲连接，使 Mihomo 内核回归超低功耗的挂机状态。
  - **WebSocket 数据订阅自动熔断 (BUG-259)**：在轻量模式下彻底断开并清空后台的 WebSocket 数据订阅，避免主窗口销毁后多余的 JSON 反序列化及 IPC 通信资源开销。

- **BUG-260~261 前端 WebView2 内存膨胀与泄露问题深度修复**：
  - **Web Worker 单例复用与生命周期泄露修复 (BUG-260)**：将流量监控 Hook 中的 Web Worker 从“频繁 terminate 与重新 new 实例化”改为“单例长连接复用，发 stop 消息暂停采样”。完美消除了 WebView2 频繁启停线程带来的 V8 隔离堆及系统句柄泄露，实现常驻内存稳定不攀升。
  - **设置抽屉回归 CSS 平移隐藏机制 (BUG-261)**：撤销设置抽屉在大/小视口拉伸以及开关时采用 React 物理卸载 `{drawerOpen && ...}` 的逻辑，回退到 1.8.2 的经典 CSS `transform` 与 `pointer-events` 平移移出隐藏逻辑。从根本上杜绝了组件卸载时 Emotion 动态注入 `<style>` 标签的无限累加泄露以及 MUI 组件 unmount 监听器残留泄露，恢复 100% 的内存稳定性。

---

## v1.8.9

### 🐞 Fixed Bugs

- **BUG-256 自动轻量模式关闭窗口崩溃与托盘失效修复**：
  - 将窗口关闭（CloseRequested）、聚焦（Focused）和销毁（Destroyed）等生命周期事件的捕获逻辑从 `app.run` 移入 `tauri::Builder` 的 `on_window_event` 拦截器中，确保在窗口真正被 OS / 默认行为销毁前完成隐藏，并安全地在异步任务中执行轻量化转换，彻底解决了 Windows / Tao 引擎下关闭窗口导致的 `cannot move state from Destroyed` 崩溃 Panic。

---

## v1.8.8

### 🐞 Fixed Bugs

- **BUG-240~255 16项审计遗留代码规范与安全性改进修复**：
  - **组件双重触发与内存泄漏优化 (BUG-240, 241, 243, 247)**：修复静默启动期间因窗口尺寸初始为0引起的 `getProxies` 重复触发；确保在迷你模式下完全卸载设置抽屉，杜绝 WebSocket 后台高频连接开销；为空闲计时器在 `resetIdleTimer` 回调中采用实时尺寸校验以防止过期状态缓存导致的误隐藏问题；在 `_layout` 状态更新中使用异步非阻塞延迟以消除 React render 周期中的同步状态更新警告。
  - **窗口自适应阈值对齐与多余无用代码清理 (BUG-242, 244, 245, 246)**：使 window-provider 中的 chrome 隐藏判定阈值与 React 布局的迷你模式阈值均对齐为 `285px`，消除窄宽度下的黑边与无标题栏排版错位；清理 `base-page`、`proxy-groups` 与 `layout-dialogs` 中声明但未使用的多余变量或阴影变量。
  - **严格 TypeScript 去 `any` 强类型规约 (BUG-248, 249, 250, 251, 252, 253, 254, 255)**：对 `app-data-context`、`debounce` 工具类、`cmds` 原始代理组转换器、延迟测试定时器 `timerId`、i18n 语言加载 reduce 累加器、Mihomo 配置中的 `nameserver-policy` 字典、流量监控 validator 验证以及 `debugLog` 的参数类型进行重构，全面消除 `any` 类型声明，使用强类型或安全的 `unknown` 进行类型收窄，保证项目严苛编译校验通过。

---

## v1.8.1

### 🐞 Fixed Bugs

- **BUG-217~238 22项系统代码安全与样式兼容Bug修复**：
  - **测速逻辑与定时器漏孔 (BUG-217~222, 224)**：在配置切换时实现自动清理与彻底关闭后台测速定时器；利用 Promise 锁对选点行为执行严格串行化，避免临时闪连与极速终选对后端造成的并发请求竞态；吞掉延迟测试后的 unhandled rejections，并重新对齐了顶栏的 theme/skin 响应依赖。
  - **内核底层并发安全与 SSRF 防护 (BUG-225~232)**：修复 TcpStream 读取空切片导致测速永远 0ms 或超时的逻辑缺陷；将 Boa 解释器及外部验证命令输出封装至 tokio 超时机制及循环指令限制下，彻底避免死循环卡死 tokio 主线程；针对订阅/图片引入 ToSocketAddrs DNS 动态解析，并将解析出的 IP 地址与 IPv6 link-local 和 ULA 敏感子网对比，防范 SSRF DNS 重新绑定绕过；添加单例冲突时的 native MessageBox 错误弹窗提示；对订阅下载加入 10MB 物理上限及流式传输限制，防止 OOM 崩溃。
  - **界面层级特异性与高 DPI 裁剪 (BUG-223, 233~238)**：使用 `html[data-control-skin]` 提升 opaque 对话框遮罩的权重，消除毛玻璃下的文本重合穿透；修复磨砂卡片在浅色模式下的低对比度问题；移除全局 scss 的 `!important` scrollbar 限制以支持局部 `scrollbar-width: none` 连接表，并绑定 scrollbar CSS 变量；将 compact 代理项行高从 20px 统一扩容为 24px 并去除高度死锁，防止 Windows 高 DPI 缩放下的文字和延迟物理裁剪；对侧栏菜单提供 Hover-dependent 滚动条显隐机制。

---

## v1.8.0

### 🐞 Fixed Bugs

- **彻底回滚严格 CSP 安全策略并恢复界面排版正常显示**：
  - 将 `tauri.conf.json` 中的 `csp` 配置彻底回滚为 `null`，移除内容安全策略对 Emotion 动态样式注入的拦截。
  - 将 `assetProtocol.scope.allow` 的本地资源读取范围重新放开为 `["**"]`，恢复各目录下本地字体、图标等资源的正常读取协议许可。
  - 清理并还原了前端代码里为了“应付 CSP 拦截”临时加入的所有内联 `style` 绕路样式，将窗口控制、顶栏图标、测速加载状态及信号图标完美退回至标准的 Emotion/MUI `sx` 属性，彻底解决了界面排版在 WebView2 下容易崩塌塌陷的问题。

---

## v1.7.9

### 🐞 Fixed Bugs

- **修复系统测速模式切换通信接口报错与构建一致性校验**：
  - 修复前端 `FindProcessMode` 选项为首字母大写（`'Strict' | 'Always' | 'Off'`）而 Rust 后端只认纯小写（`'strict' | 'always' | 'off'`）导致的切换模式接口通信解析失败问题。
  - 同步 `tauri-plugin-mihomo` 插件的前端 package.json 版本号从 0.5.2 至 0.5.4，以对齐 Rust backend 的版本配置。
  - 在 `tauri.conf.json` 中将 `beforeBuildCommand` 恢复为 `pnpm web:build`，确保打包时包含完整的 TypeScript 类型校验（`tsc --noEmit`），防范未来潜在的类型不匹配隐患。

---

## v1.7.8

### 🐞 Fixed Bugs

- **彻底修复严格 CSP 下的活动出口节点与桌面端样式特异性覆盖冲突 (BUG-215, BUG-216)**：
  - 彻底解决了由于 Material-UI `<Chip>` 内部样式高优先级特异性覆盖导致测速信号钻石图标变大（占据整个顶栏）以及后台测速菊花加载圈失效的问题，改用原生的 inline `style` 属性绕过 CSP 与 CSS 特异性重写（1000 权重覆盖）。
  - 修复窄版面（640px 窗口宽度内）下当前活跃出口节点文字长度拉伸导致的顶部状态条拉伸变形，追加合理的响应式 `maxWidth`（`120px`/`180px`）自适应限制。
  - 修复代理页面分组表头列表数据丢失且折叠交互失效的缺陷，并在 `use-render-list.ts` 中恢复代理列数设置以符合单列/多列表格用户配置。

---

## v1.7.7


### 🐞 Fixed Bugs

- **从根本上修复严格CSP下的样式失效与活动节点布局崩溃 (BUG-215, BUG-216)**：
  - 修正了 `tauri.conf.json` 中的 CSP 策略配置，显式放行本地 `tauri:` 与 `asset:` 方案源。彻底解决了由于 WebView2 不判定本地自定义协议同源而拦截静态 CSS 与 Emotion 动态注入样式，导致主界面退化为灰白色且活动节点卡片失去高度约束塌陷的问题。
  - 完全还原并清除了 1.7.6 中为了回避 CSP 拦截而作出的 23 处内联 `style` 临时绕路样式，将窗口控制按钮、顶部置顶及设置齿轮图标等全部重构对齐回标准的 MUI `sx` 属性，保持代码库的纯正与规范。

---

## v1.7.6

### 🐞 Fixed Bugs

- **补全严格CSP下右上角控制按钮及窗口边框图标尺寸约束 (BUG-215)**：
  - 修复无边框和有边框模式下，右上角的设置齿轮、图钉固定、关闭按钮以及窗口控制按钮（最小化、最大化、关闭）由于缺少显式内联 `width` / `height` 导致在 CSP 下体积膨胀至 300px 撑大整个顶部控制栏的缺陷。
  - 为 `_layout.tsx` 中的 `PushPinRounded`、`CloseRounded`、`SettingsRoundedIcon` 增加显式 `width: '20px', height: '20px'` 内联样式。
  - 为 `window-controller.tsx` 中的控制按钮（`Close`、`Minimize`、`FilterNone`、`CropSquare`）增加显式内联宽高限制。

---

## v1.7.5

- 发行版本编译对齐与常规底层库更新。

---

## v1.7.4

### 🐞 Fixed Bugs

- **修复严格CSP下活跃连接节点状态条布局崩溃 (BUG-215)**：
  - 针对在启用严格 CSP 的环境下，由于 Emotion 动态样式注入延迟导致 MUI SvgIcon/loading 旋转图标尺寸变大（300px）从而撑破顶层布局的缺陷进行修复。
  - 为所有信号图标及 Loading 圈增加显式内联 `sx` 尺寸限制（`12px`），并对卡片及节点名标签增加 `minWidth: 0` 和 `overflow: 'hidden'` 保证长节点名干净截断不撑开布局。

---

## v1.7.3

### 🐞 Fixed Bugs

- **还原设置抽屉物理裁剪与响应式隐藏设计 (BUG-205, BUG-206)**：
  - 撤销对设置抽屉换行布局及最小宽度的修改，完全恢复为原生不换行布局与 `minWidth: 0`，以恢复在默认 270px 窄窗口下连接面板被正常物理裁剪遮挡的原有设计（BUG-205）。
  - 撤销对语言与皮肤选择器显示隐藏高度阈值的修改，完全恢复为原始的 `@media (max-height: 830px)` 响应式裁剪规则，确保窄高度窗口下的布局整洁（BUG-206）。

---

## v1.7.2

### 🐞 Fixed Bugs

- **多皮肤排版与样式调优 (BUG-212, BUG-213, BUG-214)**：
  - 修复 Original 皮肤 accent 颜色未跟滑块动态绑定的问题。
  - 修复 Monochrome 皮肤在深色模式下 cards/panels 缺乏对比度背景色融合的问题。
  - 修复 Cyberpunk 和 Monochrome 皮肤中 small 尺寸开关没有适配比例的尺寸溢出问题。
- **后台死锁与启动阻塞修复 (BUG-209, BUG-210)**：
  - 将 `init_startup_script` 的执行迁移到后台 `tokio::spawn` 异步线程，防止启动脚本长时间执行导致的主 UI 线程卡死/黑屏。
  - 解决 Service Manager 初始化提权重装服务时的同步阻塞，重装过程采用 `tokio::task::spawn_blocking` 以彻底避免 Mutex 死锁和启动挂起。
- **WebView 窗口销毁性能优化 (BUG-211)**：
  - 关闭窗口时，如果未开启“自动轻量化模式”，改为隐藏 WebView 并调用 WebView2 内存回收（Low 级别），不再无条件销毁整个窗口，从而极大地提升了二次打开窗口的速度，并保留了前端的临时交互状态。
- **便携模式资源协议范围放宽 (BUG-207)**：
  - 放宽 asset 协议的 scope，在 `tauri.conf.json` 中允许加载 `$EXE_DIR/**` 和 `$RESOURCE_DIR/**`，解决便携绿色版中 profile 缓存图片加载失败显示破损的问题。
- **主题即时更新事件补发 (BUG-208)**：
  - 修复 patch 修改主题设置后未能标记 config 变化的问题，确保即时触发前端 RefreshVerge 广播重新应用 CSS，不需要再手动重启程序。

---

## v1.7.1

### 🐞 Fixed Bugs

- **CSP 字体拦截修复**：将 `Outfit` 字体通过 `@fontsource/outfit` 本地化打包引入项目，完全摆脱对外部 Google 字体库（`fonts.googleapis.com`/`fonts.gstatic.com`）的依赖，解决了在启用严格 CSP 的系统环境下因字体缺失导致的主界面 3D 拟物排版与 Label 崩溃混乱的问题。
- **修复 Clippy 编译警告**：修复了 Rust 编译器下的 `unused_trait_names` 和 `unused_async` 警告以顺利通过 pre-push 编译质量拦截。

---

## v1.7.0

### 🐞 Fixed Bugs

- **修复 Rust 编译错误**：在 `window_manager.rs` 添加缺失的 `use std::pin::Pin`，清理多余 import。
- **修复 TypeScript import 类型错误**：`app-data-context.ts` 中 `BaseConfig`/`ProxyProvider`/`Rule`/`RuleProvider` 改为 `import type`，防止运行时因无法解析外部包类型导致前端白屏。
- **恢复滚动条 `!important` 样式**：`proxy-groups.tsx` 中 `scrollbarWidth` 加回 `!important`，确保各浏览器滚动条隐藏优先级。
- **修复代理组空回退逻辑**：`use-render-list.ts` 移除 GLOBAL 回退兜底，groups 为空时渲染空列表，避免将 GLOBAL.all 中的组名列表误作节点渲染。

---

## v1.6.8

### 🔒 Security Hardening

- **CSS Injection 安全加固**：后端 URL 验证 + 前端 Content-Security-Policy 双重防御。
- **加固 deny.toml**：禁止通配符依赖、添加许可证白名单、标注所有 RUSTSEC 通报。
- **锁定 4 个 Git 依赖到 commit hash**：构建完全可重现，消除供应链风险。
- **消除 `as any` 类型安全违规**：启用 `no-explicit-any` ESLint 规则，全量修复类型擦除。
- **替换 8 处空 catch 块**：补充 `console.warn` 日志，消除静默吞异常。

### 🚀 Optimizations

- **内联 sysinfo 插件**：移除独立 `tauri-plugin-clash-verge-sysinfo` crate，将功能内联到 `src-tauri/src/utils/sysinfo.rs`，减少编译产物体积。
- **自适应窗口防抖**：空闲用户窗口操作即时响应，高频点击渐进延迟，消除 UI 卡顿。
- **添加细粒度 ErrorBoundary**：代理列表、连接管理、路由区域各有独立错误边界，防止单点崩溃白屏。
- **清理 package.json**：移除 45 个一次性 scratch 脚本，减少依赖污染。

---

## v1.6.7

### 🔒 Security Hardening

- **CSS Injection 安全加固**：后端 URL 验证 + 前端 Content-Security-Policy 双重防御。
- **加固 deny.toml**：禁止通配符依赖、添加许可证白名单、标注所有 RUSTSEC 通报。
- **锁定 4 个 Git 依赖到 commit hash**：构建完全可重现，消除供应链风险。

### 🐞 Fixed Bugs

- **消除 `as any` 类型安全违规**：启用 `no-explicit-any` ESLint 规则，全量修复类型擦除。
- **替换 8 处空 catch 块**：补充 `console.warn` 日志，消除静默吞异常。

### 🚀 Optimizations

- **内联 sysinfo 插件**：移除独立 `tauri-plugin-clash-verge-sysinfo` crate，将功能内联到 `src-tauri/src/utils/sysinfo.rs`，减少编译产物体积。
- **自适应窗口防抖**：空闲用户窗口操作即时响应，高频点击渐进延迟，消除 UI 卡顿。
- **添加细粒度 ErrorBoundary**：代理列表、连接管理、路由区域各有独立错误边界，防止单点崩溃白屏。
- **清理 package.json**：移除 45 个一次性 scratch 脚本，减少依赖污染。

---

## v1.6.6

### 🔒 Security Hardening

- **CSP 内容安全策略**：设置严格 CSP，防止 XSS 攻击加载外部脚本。
- **权限最小化**：移除 shell 执行权限、收紧 FS/Asset 作用域、HTTP 插件域名白名单。
- **URL 输入验证**：open_web_url 协议验证、ZIP 解压防路径遍历、SSRF 防护（订阅/图标 URL）。
- **JS 沙箱加固**：冻结原型链防止逃逸，JSON 传递 name 参数替代字符串拼接。
- **运行时防护**：启用 release overflow-checks，YAML 解析 50MB 大小限制。
- **安全审计**：完成全量代码安全评估，修复 15/18 项安全问题（4 项架构级问题延期处理）。

### 🐞 Fixed Bugs

- Fix clippy `missing_const_for_fn` warning in service.rs.

---

## v1.6.5

### 🚀 Optimizations

- Item A.1: Optimized Connection Table rendering by comparing raw row data index/origin instead of TanStack Table wrapper references to prevent redundant row re-renders.
- Item A.2: Memoized LogItem component and cached Regex compilation to prevent duplicate compilations across visible logs.
- Item A.3: Implemented a global singleton resize listener in use-window-width to avoid registering O(N) resize event listeners for proxy cards.
- Item A.4: Replaced useVerge hook subscription with synchronous config cache reading getPreloadConfig in useProxyDelayState hook to reduce Query observers.
- Item A.5: Added refresh token trigger in use-filter-sort useMemo dependencies to ensure proxy sorting updates immediately upon latency checks.
- Item B.1: Replaced tokio::spawn loop in latency sweep with a fixed worker pool of size 32 using lock-free AtomicUsize indexing in monitor.rs.
- Item B.2: Moved service installation and IPC waiting logic outside the SERVICE_MANAGER Mutex lock to resolve GUI hangs.
- Item B.3: Simplified tray proxy selection sync to a direct zero-cost NO-OP for the static tray menu.
- Item B.4: Replaced 200ms sleep busy loop in timer.rs with a tokio::sync::Notify awaiter in resolve/mod.rs.
- Item B.5: Replaced 1-second busy loop in background monitor with a tokio::select! block waiting on a 15-second timer or PROFILE_SWITCH_NOTIFY.
- Item C.1: Implemented write_file_if_changed in profiles.rs to compare content before writing config, saving I/O overhead.
- Item C.2: Configured reqwest Client with connection pooling limits to avoid socket churn under high concurrency.
- Item C.3: Wrapped backup output file in std::io::BufWriter to aggregate small metadata writes.
- Item D.1: Integrated useVisibility event listeners to disable system state polling when the window is hidden or minimized.

### 🐞 Fixed Bugs

- Fix all ESLint warnings and React Compiler/Purity warnings in layout and settings components (active-node-card, basic-settings-card, help-menu-button, profile-import-card, routing-preference-card, takeover-mode-card, theme-settings-card, style-helpers).

---

## v1.6.3

### 🚀 New Features

- Optimize auto-select health check sensitivity: increased timeout from 500ms to 1000ms and failure threshold from 3 to 5 to prevent high-frequency node switching.

### 🐞 Fixed Bugs

- Fix BUG-190: Implement exponential backoff cooldown (60s to 15m) for self-healing auto-select daemon when no nodes >= 30ms are available.
- Fix BUG-191: Reuse reqwest::Client connection pool in Mihomo struct to prevent socket and CPU resource exhaustion.
- Fix BUG-192: Throttle core updater download progress emissions to only when the integer percentage changes.
- Fix BUG-193: Replace semaphore add_permits(1) with a PoolPermit enum to prevent connection pool capacity inflation under RejectPolicy::New.
- Fix BUG-194: Proactively check and clean up invalid/dead sockets inside the connection pool.
- Fix BUG-195: Stream-decompress zip/gz packages directly to disk during updates to avoid buffering large files (~40MB) in RAM.
- Fix BUG-196: Delete uncompiled dead code speed_task.rs.
- Fix BUG-197: Add an active mounting flag to useConnectionData hook to prevent infinite background polling on unmount.
- Fix BUG-198 / FEAT-DELETE-001: Completely remove the window snap (磁吸) feature and its associated hook.
- Fix BUG-199: Add null safety checks for unlistenPromise during useCustomTheme cleanup.
- Fix BUG-200: Add a 2-hour active GC timer to DelayManager cache to clean up expired entries.
- Fix BUG-201: Replace 1-second setInterval polling in useVisibility with 100% event-driven native Tauri listeners.

---

## v1.6.0

### 🐞 Fixed Bugs

- Fix BUG-188: Handle discarded return values of destroy_main_window/show_main_window by logging or returning errors properly to prevent potential silent failures in main window lifecycle management.
- Fix BUG-189: Remove incorrect `/RU ""` from `create_task` that was erroneously added in the BUG-173 fix. The `/RU ""` parameter conflicts with the XML `<UserId>` when running as admin, causing `schtasks` to fail with "未指定的错误". Only `create_task_elevated` (UAC elevation path) needs `/RU ""` because the process context is lost after elevation. `create_task` runs directly without elevation and the XML provides the UserId, so `/RU ""` is unnecessary and harmful.

---

## v1.5.9

### 🚀 New Features

- Default theme changed to dark mode for first-time users, improving out-of-the-box experience in low-light environments.
- `allow-lan` now defaults to `true`, enabling LAN device proxying without manual configuration.

### 🐞 Fixed Bugs

- Fix BUG-174: Resolve text color contrast issue in Retro-3D dark mode update log cards. Changed text color from low-contrast gold `#FFE082` to high-contrast dark bronze black `#2C1F03`.
- Fix BUG-175: Window close button now immediately enters lightweight mode after hiding the window, eliminating the 10-second delay. Removed obsolete timer chain and listener code.
- Fix BUG-177: Monitor health-check error logs in the fault-recovery branch were being silently dropped. Now properly logged via `logging!` macro.
- Fix BUG-183: `FrontendEvent::DelayResults` variant now uses `SmartString` instead of `std::string::String` for consistency with the rest of the codebase.

### 🚀 Optimizations

- Enhance BUG-176: Backend Monitor speed-test results are now pushed to the frontend UI in real-time via Tauri events (`verge://backend-delay-results`), eliminating the need for manual refresh to see auto-selected node latencies.
- Hardened AsyncHandler::spawn closures with error logging to prevent silent failures in background tasks (BUG-179).
- `injectBatchResults` now preserves `elapsed` metadata when batch-writing delay results (BUG-180).
- Reset `last_check_time` after Monitor fault recovery to prevent excessive immediate re-checks (BUG-184).
- Added CI code-formatting consistency check job (BUG-186).

---

## v1.5.7

### 🐞 Fixed Bugs

- Fix BUG-170: Fix autostart switch timing race, eliminate unnecessary UAC elevation prompts for standard user tasks, and resolve early-return syntax bug in configuration patching which caused failed config drafts to stay dirty in memory and get applied in subsequent unrelated edits.
- Fix BUG-173: Add /RU "" parameter to schtasks /Create commands to bypass the user password verification prompt when creating auto-launch tasks, resolving task creation failures in Administrator mode.
- Fix BUG-171: Resolve auto-select hanging issue on profile switch/import, improve Windows named pipe busy retries under concurrency, clear stale connection handles on restart, and gracefully map non-JSON API errors.

## v1.5.3

### 🐞 Fixed Bugs

- Fix BUG-138: Redesign lightning speed test button to test only visible (filtered) nodes instead of all nodes, using front-end delayManager concurrency for per-node shimmer animation, and prevent auto-switching active nodes.
- Fix BUG-139: Fix traffic metrics cards layout nesting error under both wide and narrow window modes.

### 🚀 Optimizations

- Optimize: Unify the default concurrency limit to 36 to match the configuration default.
- Refactor: Format imports and simplify logger calls in Rust backend `enhance/mod.rs`.

## v1.5.2

### 🐞 Fixed Bugs

- Fix BUG-138: Fix lightning speed test button failure caused by empty test URL string.
- Fix: Use pure CSS to fix narrow window folding and wide window equal-width layout.

## v1.5.1

### 🐞 Fixed Bugs

- Fix BUG-121/BUG-138: Use frontend `delayProxyByName` API path to prevent lock conflicts.
- Fix: Remove "WinAero" prefix from base settings card title.

## v1.5.0

### 🐞 Fixed Bugs

- Fix BUG-121: Resolve issue where the active outbound node is permanently occupied by a fixed node (advertising/dummy node) after importing a new subscription, and the auto-optimization process never triggers. Root cause: backend monitor thread's `trigger_backend_auto_select` locks `AUTO_SELECT_RUNNING` on the OLD config, and the frontend's `triggerAutoSelectAndRefresh` silently swallows the `AUTO_SELECT_BUSY` error without retry, causing subsequent `refreshProxy`, `setHeadState(sortType=1)`, and Fallback timer to never execute. Fix: added AUTO_SELECT_BUSY retry (up to 5 attempts, 600ms interval) in `triggerAutoSelectAndRefresh`, and moved `refreshProxy`/`setHeadState`/Fallback timer outside the try-catch block to ensure they always execute.
- Fix BUG-138: Resolve issue where clicking the lightning cursor (batch speed test) at the top of the proxy table has no response. Root cause: `trigger_backend_auto_select` path has three inconsistencies compared to the single-node speed test path: (1) `create_client()` had an aggressive 3-second timeout, (2) `delay > 50` lower-bound filter excluded low-latency nodes, (3) `handleCheckAll` had no retry for `AUTO_SELECT_BUSY`. Fix: increased client timeout to 10s, removed the `delay > 50` filter, added AUTO_SELECT_BUSY retry in `handleCheckAll`.

### 🚀 Optimizations

- Refactor: Merged "Takeover Mode" and "Routing Preference" cards into a single unified card module, removing the gap between them.
- Refactor: Swapped the positions of the Upload and Download traffic metric groups at the bottom of the window (Download on left, Upload on right).
- Change: Default sort mode of the table header cursor changed from "default sort" to "sort by delay".
- Change: Adjusted slider min/max values for various skin styles (Depth 2.0→1.0, Radius 5.0→3.0, Roundness 2.0→3.0, Opacity 2.0→5.0, Monochrome Radius 1.0→3.0 & min 0.3→0.0, Vibrancy 2.0→5.0, Shadow 2.0→5.0).

## v1.4.9

### 🐞 Fixed Bugs

- Fix BUG-136: Resolve issue where importing subscription links failed with error "订阅链接内容格式错误，既不是合法的 YAML 配置文件，也无法解析为节点链接列表". Supported Base64-encoded YAML files in the format detection funnel, and made Base64 decoding robust against internal newlines and whitespaces.

## v1.3.4

### 🐞 Fixed Bugs

- Fix BUG-103: Resolve global environment type pollution issue. Split core business types and interfaces in `global.d.ts` into independent ESM module files, and explicitly import them in components/hooks; refactored `IProxyConfig` interface definition to use discriminated unions and intersection types, eliminating overlapping type conflicts and successfully passing compilation.

## v1.3.3

### 🚀 Optimizations

- Optimize BUG-097: Resolve issue where asynchronous worker threads were blocked by synchronous disk I/O. Migrated configuration file reading to `tokio::fs` async APIs and updated large package file writes to be handled by `spawn_blocking`, eliminating transient freezes and latency spikes.
- Optimize BUG-098: Resolve issue where synchronous system process scanning blocked asynchronous threads. Executed `sysinfo` process scanning and force-killing inside `spawn_blocking` to avoid hanging Tokio scheduler worker coroutines.
- Optimize BUG-101: Optimize internal functions and state type safety in TS helper hooks. Wrapped exposed callbacks with `useCallback` to stabilize references, and established exact TS type definitions for core data structures like `proxies`.
- Optimize BUG-105: Optimize callback reference stability inside `use-profiles.ts`. Wrapped `mutateProfiles`, `patchProfiles`, and `patchCurrent` callback functions with `useCallback` to prevent redundant sub-component re-renders.

### 🐞 Fixed Bugs

- Fix BUG-069: Resolve issue where setting switch size="small" under Monochrome skin would freeze on the left side and fail to toggle or interact normally.
- Fix BUG-096: Resolve deadlock issue caused by backend resident threads holding `RwLock` read locks across await points. Acquired locks and completed Future computations inside independent local scopes, allowing the read lock to be dropped and released before the await point.
- Fix BUG-099: Resolve main UI thread white screen hanging and unresponsiveness caused by the synchronous `block_on` call in the Tauri Setup hook at client startup. Switched to `async_runtime::spawn` to load silent update check tasks asynchronously in the background.
- Fix BUG-100: Resolve truncation and overflow hazards of backend timestamp casting on 32-bit OS or embedded platforms. Standardized all timestamp fields in `PrfItem` and `IProfiles` to `Option<i64>` type, abolishing `as usize` type casting.
- Fix BUG-104: Fix casing inconsistency in `useWindowSnap.ts` file name. Renamed it to `use-window-snap.ts` and updated import paths accordingly, aligning with the project's kebab-case naming convention.
- Fix BUG-106: Fix type system failure caused by loose `any` type definitions for internal state variables and core data objects in `_layout.tsx`. Strictly typed `clientUpdateObj` (as `Update`) and `coreUpdateRelease` (as `GithubRelease`) and applied safe casting.

## v1.3.2

### 🚀 Optimizations

- Optimize BUG-082: Implement automatic background memory reclamation and optimization for WebView2 runtime on Windows. Actively send memory reclamation signals to the WebView2 process when the window is minimized or hidden in the system tray, minimizing physical memory usage during standby.

### 🐞 Fixed Bugs

- Fix BUG-093: Resolve significant delays in auto-activating and connecting nodes after Clash Mini starts or reloads configs. Concurrent-probed API ports and `DIRECT` node readiness to shorten cold start selection time, and applied fuzzy comparison to strip timestamp suffixes of historical selections after subscription updates to restore node selection.

## v1.3.1

### 🐞 Fixed Bugs

- Fix BUG-092: Resolve issue where clicking the "lightning cursor" at the top of the active outbound node list on the home page to trigger speed tests for all nodes failed to automatically select and switch to the fastest healthy node in the current filtered subset.
- Fix BUG-091: Resolve issue where the subscription link input field in production could not use right-click for paste, copy, cut, select all, etc.
- Fix BUG-090: Resolve issue where the text of "Subscription & Config", "Traffic Takeover Mode", "Basic Settings" cards on the left panel, and the active outbound node status bar on top in Trump-3D (retro-3d) dark mode rendered white by default, blending into the bright gold background and making reading difficult. Forced text color to high-contrast dark bronze black `#2C1F03` and provided natural hover feedback.

## v1.3.0

### 🚀 Optimizations

- Optimize BUG-087: Renamed the skeuomorphic 3D theme `Retro-3D` to `Trump-3D`, and redesigned it into a luxurious gold bar visual style. Applied linear high-reflection gold bar gradient backgrounds, dark bronze engraved lettering projections, heavy mechanical button press feedback, double-layer gold borders, and dark gold ambient lighting effects to enhance the skeuomorphic feel.
- Optimize BUG-086: Shortened the throttling interval of traffic data updates in narrow layout mode from 3000ms to 1000ms to eliminate traffic chart dropping to zero and jagged line breaks, and automatically suspended queries when the window is hidden to optimize resource consumption.

### 🐞 Fixed Bugs

- Fix BUG-089: Resolve issue where periodic health monitoring of active node latency in maximized mode triggered high-frequency speed test timeouts due to deprecated backend APIs, forcing frequent re-selection and switching of nodes. Refactored it to call the new `delayProxyByName` API provided by the plugin, and implemented a silent strategy (no notification popups) for background auto-selection of the same fastest node.
- Fix BUG-081: Completed promise chains for data refresh and added a delay before config polling, resolving the synchronization issue where home page node list failed to refresh immediately when importing/updating subscriptions and required waiting two minutes.
- Fix BUG-083: Resolve issue where auto-selection failed to read the correct Profile ID for subset filtering at startup due to the state closure not being ready.
- Fix BUG-084: Cleaned and removed redundant `!isDummyNode` filtering logic in auto-selection, delegating it to the data loading layer for a single clean interception.
- Fix BUG-085: Removed "All routes are busy" bubble notifications in background silent health checks to avoid distracting users with redundant error popups.
- Fix BUG-088: Fix issue in Trump-3D dark mode where default buttons with Contained style had light gold text blending into the bright gold background, making it hard to read. Adjusted text color to high-contrast dark bronze.

## v1.2.8

### 🐞 Fixed Bugs

- Fix BUG-081: Resolve slow response of auto-selecting the fastest node after starting or switching subscriptions, and the issue where advertising/placeholder nodes (such as remaining traffic, official website) occupied the top spots of the node list and falsely succeeded speed tests, interfering with background `NodeMonitor` checks. Implemented deep purification filtering in global data layers (`calcuProxies`, `calcuProxyProviders`, `fetchProxies`), and automatically triggered speed tests when `NodeMonitor` detects an ad node to correct the selection.

## v1.2.6

### 🚀 Optimizations

- Optimize BUG-079: Refined the tooltip description of "Rules Adjustable" in routing strategy to "arbitrarily adjust path controls on top of preset rules", making it more accurate and aligned with the "Path Control" feature, and updated translations for all 13 supported languages.

### 🐞 Fixed Bugs

- Fix BUG-080: Resolve unfriendly update notifications when versions match. When the kernel or client is already the latest version, replaced the error popups with info tooltips to improve interaction friendliness and robustness.

## v1.2.5

### 🚀 Optimizations

- Dead Code Cleanup: Removed redundant deprecated functions, dead code, and unused variables, streamlined the system tray menu, and restored the mechanism of automatically pulling the latest stable kernel.

### 🐞 Fixed Bugs

- Fix BUG-078: Resolve freeze issues in kernel upgrade and update downloads. Added 20-second timeout control for response stream chunk reads, and implemented multi-channel (Localhost, System, None) silent retries and auto-fallback, shortening TCP handshake timeout to 10 seconds.

## v1.2.4

### 🚀 Optimizations

- Performance Optimizations: Implemented batch buffered logging to reduce backend IPC event dispatch frequency; suspended redundant rule and policy group queries in mini-monitoring mode; implemented 1D incremental connection list updates to reduce data throughput; limited the list to display at most 100 most active physical nodes to improve React rendering performance.
- Asymmetric Window Wakeup: Wakes up WebSocket connections immediately when the window becomes visible, and disconnects with a 1-second delay when minimized or hidden.
- Adaptive Borders: Replaced dynamic animated borders with static, adaptive double/solid borders to reduce system CPU overhead.

### 🐞 Fixed Bugs

- Fix BUG-077: Fix update failure caused by incorrect regex matching of the Mihomo core auto-update filename.

## v1.2.3

### 🚀 Optimizations

- Optimize BUG-075: Completely resolve memory leak issues in backend connection list polling. Automatically suspend polling when the Connections drawer is closed; introduced client-side Traffic Accumulator to perform incremental memory calculation via low-frequency traffic SSE stream when the drawer is closed, ensuring accumulated data updates on the bottom control panel.

### 🐞 Fixed Bugs

- Fix BUG-076: Resolve issue where the update button remained clickable and triggered duplicate downloads when the kernel was already the latest version. Added static version comparison (filtering tag prefixes/suffixes) to disable the button and show "Up to date" when versions match.
- Fix Auto-update 404 error: Created `updater/app-update.json` and redirected Tauri auto-update URL to the GitHub repository Raw address, ensuring check-for-updates in client no longer throws errors.
- Refine Silent Release Guide: Added Phase 3 in `clash_mini_silent_release.md` to establish updater configuration file maintenance instructions.

## v1.2.2

### 🚀 Optimizations

- Pre-flight preparations for performance and update optimizations.

## v1.2.1

### 🐞 Fixed Bugs

- Fix BUG-065: Pin active outbound node real-time subset rotation filtering limits. When clicking the node name in the active outbound node status bar to rotate, dynamically extract and apply current search, filter, sorting, and hide-timeout conditions to ensure the rotation is restricted within the currently displayed candidate node subset.
- Fix BUG-074: Kernel update connection timeout and redundant version formatting. Refactored the backend to introduce a `NetworkManager` multi-downgrade automatic fallback mechanism (Local Port -> System Proxy -> Direct) for checking kernel updates, solving timeout connection failures caused by network environments; added universal `formatCoreVersion` formatter on frontend to eliminate duplicate `vv` prefix and unify to `Ver.X.Y.Z` format.

## v1.2.0

### 🐞 Fixed Bugs

- Fix BUG-072: Resolve issue where Allow LAN switch bounced back automatically and failed to stay enabled. Added lowercase deserialization config and `ts-rs` conversion annotations to the backend core `FindProcessMode` model, eliminating bouncing switches caused by Mihomo core field parsing errors.

### 🚀 Optimizations

- Refine Subscription Card Right-Click Menu: Added context menu (Edit, Edit File, Copy Link, Update, Delete) on subscription cards, and supported a simple config editing dialog, adapting seamlessly to all 6 visual themes.

## v1.1.3

### 🐞 Fixed Bugs

- Fix BUG-066: Optimize speed slider breathing/pulsing animation in Cyberpunk skin. Switched to inset box-shadow and border breathing, accelerated breathing frequency, and dynamically scaled aurora flow speed with slider value, solving the clipping and lack-of-effect issues.
- Fix BUG-067: Optimize shadow slider feedback in Modern Flat skin. Introduced adaptive base shadow density variables for light and dark modes (darker in dark mode) to unify button, card, input, and panel projections.
- Enhance Adjustment Freedom: Unified the maximum limit range of double sliders under all styles from `2.0` (or `3.0`) to `5.0` to provide users with stronger visual contrast and fine-grained control.

## v1.1.2

### 🚀 Optimizations

- Added 1-click loop switching of active nodes in the navigation bar.
- Streamlined development agreement `clash_mini_agreements.md`, removing redundant pitfalls and release SOP.

## v2.0.4

### 🐞 Fixed Bugs

- Fix BUG-041: Completely restructured home page node list rendering, removing React state `isMinimalHeight` and height listeners. The node list now renders unconditionally and clips naturally with window size, fixing the blank list issue caused by rendering timings at initial load.
- Optimize Agreement: Updated [Highest Design Baseline] to skeuomorphic 3D and physical texture aesthetics, and corrected development boundaries and Rust backend refactoring rules.
- Fix Doc Consistency: Corrected bug registration references in `clash_mini_pitfalls.md` and `clash_mini_silent_release.md` to `bug_list.md`.

## v2.0.3

### 🚀 Optimizations

- Upgrade Compile Environment: Updated project dependencies, supporting TypeScript type checks and backend Clippy static compilation.

## v2.0.2

### 🚀 Optimizations

- Narrow Layout Optimization: Right-aligned the protocol column, left-aligned the node name, and centered the latency column in the home page node list, and optimized the minimum window height to `163px` in minimalist mode.

## v2.0.1

### 🐞 Fixed Bugs

- Fix BUG-034: Optimize subscription activation auto-chain actions, restricting speed-tests and fastest node selection strictly to the current filtered home page node subset, preventing unavailable nodes from being incorrectly selected.
- Archive README_First: Updated and archived beginner user guides.

## v1.1.9

### 🐞 Fixed Bugs

- Fix BUG-073: Implemented static system tray design on Windows, avoiding COM `E_FAIL` (os error -2147467259) errors when dynamic update instructions sent to Explorer are blocked under Administrator privileges.
- Fix BUG-038: Fix setting page "Active/History" connection switch being blocked by the header, making parts of it unclickable.

### 🚀 Optimizations

- Optimize STYLE-001: Upgraded 23 core interactive components (including buttons, selectors, switches, sliders, inputs, and cards) in main scene and settings page to skeuomorphic 3D Bevel textures and neon glow, linked to adaptive depth and glow variables.

## v1.1.8

### 🐞 Fixed Bugs

- Fix BUG-071: Client and Mihomo core auto-update development and UI dropdown integration. Upgraded `tauri-plugin-mihomo` dependency to support newer kernels (v1.19.27+), resolving blank active node lists.
- Fix BUG-072: Fix Allow LAN switch bouncing back due to LogLevel serialization issues in newer plugin models.rs.

### 🚀 Optimizations

- Popover Menu Restructuring: Reconstructed the sidebar "Help" button into an upward Popover menu, adapting to all 6 visual skins with 3D bevel and glow effects.
- Adaptive Updates: Supports client silent background update polling and boot-ready detection, integrating kernel hot-swap and service release mechanisms.

## Upstream Release History (Clash Verge History)

## v2.5.2

### 🐞 Fixed Bugs

- Possible styling errors in macOS tray speed display.

<details>
<summary><strong> ✨ New Features </strong></summary>

- Added display support for TrustTunnel, OpenVPN, Tailscale, GostRelay nodes.

</details>

<details>
<summary><strong> 🚀 Optimizations </strong></summary>

- Close autofill popups.

</details>
