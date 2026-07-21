# Clash Mini 架构深度评审（控制逻辑 / 信息传导 / 防阻塞 / 稳定性）

> 评审日期：2026-07-21 · 基线：v2.6.9（dev@6aaac40c）
> 方法：设计书（clash_mini_agreements.md + node-health-architecture.md）对照真实代码逐条核实，所有结论带文件:行号。
> 阅读方式：每项先说「对你用起来意味着什么」，再给技术定位。

---

## 处理进度（逐条处理中）

| # | 问题 | 状态 | 路线 |
|---|---|---|---|
| 🔴1 | 手动切节点后高亮停旧节点 | ✅ 已修已提交 `47aa63c8` | 路线A：前端切完 `invalidateQueries(['getProxies'])`，用项目既有刷新机制 |
| 🔴2 | proxy_head_state.json 非原子写 | ✅ 已修已提交 `159a4fe9` | 新增 `help::save_json`（原子写范式复用 save_yaml），读取失败补 warn! |
| 🔴3 | destroy_main_window oneshot 无超时 | ✅ 已修已提交 `4b566b72` | `rx.await` 包 `timeout(5s)`，与 `activate_window` 对齐；超时/oneshot丢弃返 Failed |
| 🟡4 | 设计书 §4.3/§5.1 周期脱节（60s/300s 空话） | ✅ 已改设计书（路线 B，未动代码） | §4.3 改「后台监测周期恒定」，§5.1「300秒」→「15秒」，BUG-257 设计确认 |
| 🟡5 | 内核全死时「连续 3 次 API 异常触发自愈」永不触发；内核死不弹 Windows 警报 | ✅ 已修 `69cb4815`+`33a48961`+`6c3ed33b` | None/Err 分支累计 `api_error_count` 触发自愈；None/Err 分支累计 `auto_select_fail_count` 满 5 次弹 Windows 警报 |
| 🟡6 | 网络恢复/API异常两条自愈路径绕过 60s 冷却与失败计数 | ✅ 已修（统一 helper） | 抽 `self_heal_with_accounting()`，主/网络恢复/API异常三处自愈统一走冷却+计数+警报记账 |
| 🟡7 | `ACTIVE_TASKS` 死设施："Profile 切换中止旧测速"是空转幽灵机制 | ✅ 已修（方式一：删除） | 删 `AbortHandle` 导入/`ACTIVE_TASKS` 队列/`abort_all_active_tasks()` 函数/主循环空转中止块/`window.rs:16` 调用；正确性仍由 uid 双重校验兜底 |

## 一、总体判断

**地基是对的，楼也基本盖对了，但图纸和楼有出入。**

`node-health-architecture.md` 的五层设计（测量归内核 / 子集结构性 / 可靠信号自愈 / 手动优先 / 展示透传）是治本的，而且核心部分真的落地了：select + url-test 双组解耦、热切换零断流、7-19 污染事故的三层校验、轻量模式状态机、主线程纪律，这些都不是纸面文章。

但核实发现 **3 处设计书声称的机制在代码里不存在或已改道**，以及 **3 个高优先级真实风险** 和一批中低问题。设计书刚重构完，正是把「脱节条目」对齐的好时机。

---

## 二、🔴 高优先级（3 项，建议排期修）

### 🔴 1. 手动点击切节点后，界面高亮可能还停在旧节点上
- **对你的影响**：在节点列表点了一个新节点，内核其实已经切过去了（流量在走新节点），但列表里的高亮/活跃卡还显示旧节点，直到某次无关刷新才纠正。看起来就像「点了没反应」，可能诱使你重复点击。
- **定位**：`src/hooks/use-proxy-selection.ts:77-106` 手动切节点后没有失效 `['getProxies']` 缓存（react-query `refetchInterval:false`，`app-data-provider.tsx:156-162`）。后端自动切点有 `refresh_proxies` 事件推送（monitor.rs:672），手动路径没有。这正是 `ecd96676` 修过的同一类前后端不一致，点击路径还开着。
- **修复方向**：`selectNodeForGroupWithTimeout` 成功后 `invalidateQueries(['getProxies'])`，或乐观 `setQueryData` 直接更新 `now`。

### 🔴 2. 过滤状态文件是非原子写，撕裂读会让防污染防线开个洞
- **对你的影响**：极端时序下（后端恰好读到写了一半的 `proxy_head_state.json`），解析失败会**静默当作"无过滤词"**，选点候选池从"日本子集"悄悄放大成全量——这正是 2026-07-19 事故修了半边的那个洞的另一侧。
- **定位**：`cmd/proxy.rs:100-105` 用 `tokio::fs::write` 直接覆写 JSON（非原子）；后端 `monitor.rs:142-166` 每次现读现解析，失败即 fail-open 为 `filter=""`。而防污染校验（monitor.rs:635-664）依赖这个 filter。
- **修复方向**：写临时文件 + rename（Windows 上 rename 同卷原子）；或后端按 mtime 缓存、解析失败沿用上次成功值而非全量。

### 🔴 3. 进入轻量模式的「读窗口尺寸」等待没有超时——UI 线程卡死会拖垮整个托盘
- **对你的影响**：如果 WebView/UI 线程哪天卡死（模态循环、GPU 挂起），点关闭进轻量模式会**永远卡住**，而且因为它持有全局轻量锁，托盘的「恢复窗口/退出」也全部失灵，只能任务管理器强杀。
- **定位**：`window_manager.rs:503` `rx.await` 无超时。同款风险在 `activate_window` 已经配了 5 秒超时（window_manager.rs:398），destroy 这条漏了。卡死期间持有 `LIGHTWEIGHT_LOCK`（lightweight.rs:10），托盘入口全被串行锁堵死。
- **修复方向**：给 oneshot 等待加 5 秒 timeout，超时按销毁失败处理走既有回滚分支（lightweight.rs:112-118 已经准备好了）。

---

## 三、按维度详评

### 3.1 控制逻辑

**做得好的（值得保持）：**
- 统一选点入口 + `AtomicBool` CAS 抢占 + Drop Guard 释放（monitor.rs:417-433），panic 路径也不泄漏互斥；前端命令与 monitor 共用同一入口同一互斥，冲突返回 `AUTO_SELECT_BUSY` 且 monitor 对其特判不冷却不计失败（:958-967）。
- 轻量模式三态 CAS 状态机（Normal/In/Exiting，lightweight.rs:27-72）+ 全局 Mutex 串行化进出，核实无非法转移路径；进出失败都有回滚。
- 内核重启/热重载/内核升级三条路径的 PROXY `now` 快照-恢复全覆盖（lifecycle.rs:96-117、config.rs:130-140、core_updater.rs:494-495）。
- 7-19 污染事故的三层校验**真落地了**：回写前校验（monitor.rs:635-664）、还原前校验（monitor.rs:261-279）、前端第三道反向校准（use-profiles.ts:237-250）。

**🟡 4. 设计书曾称"活跃60s/轻量300s自适应周期"——实际恒 15s（✅ 已通过路线 B 修正设计书）**
- **处置（2026-07-21）**：经设计判断，Clash Mini 作为代理程序，节点健康与程序前/后台状态无关，轻量模式无需放慢监测；恒定 15s 为正确设计。设计书 §4.3 已改为「后台监测周期恒定」并写明依据，§5.1 第 224 行「300 秒」引用已改为「15 秒」，BUG-257 标注为设计确认。代码无需改动。
- **原定位（存档）**：`monitor.rs:694-702` 周期只看 `is_retry_mode/was_online`，轻量状态从不参与 sleep 决策，恒 15 秒；原 §4.3 声称的「轻量 300s 省电」属图纸与楼不符。

**🟡 5. 内核全死时，"连续 3 次 API 异常触发自愈"永远不会触发** ✅ 已修（提交 `2aac6ee9`）
- 影响：设计书 §5.1 专门为「内核卡死」准备的自愈，在内核彻底挂掉（连节点名都拿不到）的场景下一次都不计数——恰恰在最需要它的场景失效。表现就是内核真死时 monitor 安静空转，不断流告警也没有。
- 定位：`monitor.rs:882-889`，`get_active_node_name()` 返回 None 时 `continue` 发生在 `evaluate_failover` 之前，`api_error_count` 跳过累加。
- 修复（路线 A）：None 分支同样累加 `api_error_count`，满 3 次触发自愈尝试，复用现有计数器与阈值。内核死亡时不再静默，每轮打 warn 日志、每 3 轮尝试自愈。
- 残余（已补齐）：原「内核全死不弹 Windows 警报」已在 `33a48961`（None 分支）+ `6c3ed33b`（Err 分支）补齐——两条路径现均累计 `auto_select_fail_count`，满 5 次弹 Windows 警报。最终 🟡6 又将三处自愈统一收口到 `self_heal_with_accounting()`（冷却+计数+警报一份实现）。

**🟡 6. 网络恢复/触发的两条自愈路径绕过冷却与失败计数** ✅ 已修（统一 helper）
- 影响：网络抖动（Wi-Fi 反复掉线重连）时可能背靠背触发全组拨测；且这两条路径不更新 `last_auto_select_time`、不累计 `auto_select_fail_count`，导致「连续 5 次失败弹窗告警」对它们永远不可达——真出问题用户收不到告警。
- 定位：`monitor.rs`（网络恢复自愈）、`monitor.rs`（API 异常自愈 Err 分支），对照主路径冷却。
- 修复（治本，用户选路线）：抽 `self_heal_with_accounting(profile_uid, &mut auto_select_fail_count, &mut last_auto_select_time, &mut last_check_time)`，三处自愈调用（主路径 `consecutive_fails>=2` / 网络恢复 / API 异常）统一走同一份冷却(60s)+失败计数+5次警报记账，消除三处重复与行为不一致。

**🟡 7. `ACTIVE_TASKS` 是死设施："Profile 切换中止旧测速"是死代码** ✅ 已修（方式一：删除）
- 影响：切换 Profile 后，旧的选点任务仍持互斥锁跑到自然结束（最坏约 4 秒+），实际防串靠 uid 双重校验（:608-620）兜底——兜底是有效的，但注释承诺的中止机制不存在，误导后来者。
- 定位：`monitor.rs:16` 的 Vec 从无 push；`abort_all_active_tasks`（:22）与 :786-798 的调用点全是空转。
- 修复（用户拍板方式一，commit 见下）：删除 `AbortHandle` 导入、`ACTIVE_TASKS` 队列、`abort_all_active_tasks()` 函数、`monitor.rs` 主循环里那段空转的「中止旧测速」块、以及 `feat/window.rs:16` 的调用。正确性仍由 uid 双重校验兜底，删后零副作用，代码消除「幽灵机制」。

### 3.2 信息传导

**做得好的：**
- 测速结果唯一回传通道：`delay_group → notify_delay_results → verge://backend-delay-results → DelayManager.injectBatchResults`（rAF 批量合并防刷新风暴），前端批量自测已整体收编。
- 后端→前端事件双过滤：轻量模式 + 无窗口时静默丢弃（notification.rs:48-55），exiting 也有护栏（handle.rs:105-112），不存在向已销毁窗口 emit 导致 panic 的路径。
- WS 订阅统一 ref-count 共享、可见性门控（隐藏 1s 尾随防抖后断连）、监听器注册/注销对称，未发现泄漏。

**🟡 8. 过滤词双源真相 + 前端算了子集又丢弃**
- 影响：前端过滤框的状态同时写 localStorage（前端读）和 `proxy_head_state.json`（后端读），中间有 100ms 防抖窗口期两源不一致；更别扭的是 `profile-coordination.ts:116` 明明算好了过滤子集，触发后端时却传 `undefined`，让后端按自己读到的 filter 重新推——**你屏幕上看到的子集和后端实际用的子集，在窗口期内可能不是同一个**。
- 定位：`use-head-state.ts:100-113` 双写；`profile-coordination.ts:116` 丢弃已算好的 `names`。
- 修复方向：把已算好的 `names` 直接传给后端（变量就在 :153 作用域里），消除双源推算。

**🟡 9. 回写校验用"第二次读盘的 filter"而非本次候选快照**
- 影响：测速进行期间（≥2 秒）你如果改了过滤词，最快节点会因"不在新范围"被拒写回——但内核已经切过去了，于是内核节点与 `profile.selected` 再次脱节，轻量唤醒后可能被 restore 切回旧节点。
- 定位：`monitor.rs:637-643` 第二次读盘取 filterText，而非对照本次 `valid_nodes` 候选快照校验。
- 修复方向：校验口径改为「最快节点 ∈ 本次 candidates」，与切出去的动作天然一致。

**🟡 10. 残留一处单点自测入口（TUN 下会报假超时）**
- 影响：活跃节点卡片的「测速」仍走 `/proxies/{name}/delay` 自测——TUN 模式下流量回环，可能把正在扛流量的节点显示成 Timeout，误导你以为当前节点挂了。这是全前端唯一没掐干净的自测。
- 定位：`active-node-card.tsx:108` → `DelayManager.checkDelay` → `delayProxyByNameWithTimeout`（delay.ts:271-375）。monitor.rs:544-547 的注释明确批判过这条路径。
- 修复方向：改为读 `PROXY__METRICS` 测量组该节点的 `now/history`（权威值），或委托后端单点测量。

**🟡 11. 快照/重载/恢复是非原子的，窗口期内 monitor 可能读到"假首节点"**
- 影响：配置热重载的瞬间窗口里，Selector `now` 被内核重置为列表首节点（常为广告假节点），若 monitor 的 15 秒判定恰好落进窗口，可能误触一次不必要的 failover，或把错误节点写回 selected。
- 定位：`config.rs:123-137` 快照→reload→恢复三段不原子；`ecd96676` 修复后的残留窗口。
- 修复方向：reload 期间置一个"配置变更中"标志，monitor 判定与前端 refetch 看到标志就跳过本轮。

### 3.3 防阻塞

**做得好的：**
- Local Socket 全部 3s 显式超时（mihomo.rs:32,184；ipc.rs:579-582），`delay_group` 动态上限约 4s，DNS 探测 2s 超时；monitor 路径核实无无限 await。
- 主循环 `tokio::select!` 可被打断（sleep / 唤醒 Notify / Profile 切换 Notify 三路），`Notify::notify_one` 带 permit 存储不丢唤醒。
- std::Mutex 全部 poison-tolerant 获取（unwrap_or_else(into_inner)），一个线程 panic 不会永久锁死全局。

**🟡 12. 轻量模式下退出请求被静默吞掉（macOS Cmd+Q 无效）**
- 影响：轻量模式（无窗口）时按 Cmd+Q / 触发系统退出，程序只是 `prevent_exit()` 然后什么都不做——用户觉得"这软件退不掉"，只剩托盘「退出」一条路。
- 定位：`lib.rs:563-565`，注释说"先退出轻量模式"但没有任何后续动作。
- 修复方向：prevent_exit 后显式走「退出轻量 → 再退出」编排，或直接放行退出。

**🟡 13. 窗口销毁失败不传播 + 双窗口状态持久化互相竞争**
- 影响（a）：`destroy()` 失败只记 debug 日志，状态机照样记 `In`——窗口其实还活着，回滚分支（lightweight.rs:112-118）几乎永远不会触发，状态与事实分裂。
- 影响（b）：`tauri-plugin-window-state`（StateFlags::default 恢复 size+position）与自家 `window_state.json` + `force_set_window_outer_size` 两套持久化同时作用于窗口创建，尺寸来源有两个裁判，未来改尺寸逻辑极易踩出诡异 bug。
- 定位：window_manager.rs:495；lib.rs:132-140 vs window.rs:67-69,139。
- 修复方向：destroy 结果纳入状态机决策；二选一砍掉一套窗口状态持久化（建议留自家的，plugin 收窄为不管 size/position）。

**💭 14. 10 秒强制显示兜底在异步线程直接调窗口 API**（window.rs:146-165 的 `is_visible/show/set_focus` 来自 tokio::spawn）——BUG-259 同类违规，虽然概率低；改 `run_on_main_thread` 即可。

**💭 15. Resized 每事件 spawn 一个任务，快速拖拽时无上限**；250ms 节流是 load-then-store 非原子（良性，但有托底——销毁时主线程兜底读尺寸）。可改 debounce 尾触发。

### 3.4 稳定性

**做得好的（这一维整体最扎实）：**
- 退出清理链完整：abort monitor/清理任务/定时器 → 清 WS → reset sysproxy(1.5s) → disable tun(1s) → stop_core(2s) → `kill_all_mini_cores` 扫杀 `mini-` 前缀残留（feat/window.rs:60、manager/state.rs:177-200）。
- 托盘生命周期独立于窗口：窗口重建失败不丢托盘，用户永远能从托盘「退出」（tray/mod.rs:283-284 绕过防抖直达 quit）；单实例端口绑定 TOCTOU 安全。
- monitor 全文件核实无 unwrap/expect/panic；弹窗用 spawn_blocking 不堵 worker。
- WS 断连前端 1s 重连循环 + 挂载自动恢复，轻量唤醒后数据流无感瞬连的设计成立。

**🟡 16. WS 重连固定 1 秒无退避**——内核挂掉期间三条通道每秒各重试一次，日志噪音 + 无谓 syscall。改指数退避（1s→2s→…→30s 封顶）即可。（低）

**💭 17. 就绪窗口不匹配**：前端 `waitForClashReady` 10s vs 后端 30s+20s——慢启动机器上前端先放弃，触发一次被吞的 `AUTO_SELECT_BUSY` 噪音。无害但建议对齐。

**💭 18. monitor 无重复启动防护**（resolve/mod.rs:130 单次调用，属约定而非防护）；**💭 19. `restore_proxy_group_now` 不校验快照节点是否 dummy/在子集内**（lifecycle.rs:306），假节点经 provider 混入后可能被原样恢复；**💭 20. 写回失败仅 warn**（monitor.rs:653-655），靠后两层校验自愈，可接受但建议升 error 并触发一次校准。

### 3.5 安全性（顺带核查）

设计书 §6 的权限最小化、SSRF/Zip Slip 防护、JS 沙箱冻结、50MB YAML 上限、overflow-checks 均有实现笔记对应落地，本轮未发现新的攻击面。单实例 IPC 有密钥校验，external_controller 有固定密钥。此维度干净。

---

## 四、设计书需要修订的脱节条目（重要——刚重构完，趁热对齐）

| 设计书条目 | 实际代码 | 建议 |
|---|---|---|
| §4.3/§5.1 活跃 60s、轻量 300s 自适应周期；BUG-257 已完成 | 恒 15s，轻量状态不进 sleep 决策（monitor.rs:694-702） | ✅ 已改设计书承认现状（路线 B）：§4.3 改「后台监测周期恒定」，§5.1「300 秒」→「15 秒」 |
| §5.5 后端并发测速上限 32 | 已改单次 `delay_group`，并发归内核 url-test（commit 49212979 已删 worker 池） | ✅ 已改设计书（见下 commit）：第 219 行改「测量统一委托内核、无应用侧并发池」 |
| §5.1 连续 3 次 API 异常触发自愈 | 内核全死场景永不计数（见 🟡5） | ✅ 已修代码（None/Err 分支累计 api_error_count + auto_select_fail_count，见 🟡5 及后续提交）；三处自愈已收口到 `self_heal_with_accounting()`（🟡6） |
| §5.1「monitor 快照变量防遮蔽」编码规范 | 无对应变量存在（当前 check_interval 每轮经 check_interval_secs 重算，无 sleep 前快照/后遮蔽结构） | ✅ 已删设计书第 236 行（用户判定：设计书本不该写实现级编码规范，且规范看守的结构已不存在） |
| §5.5 F4「仅对子集并发测速」 | 实际对全组拨测、子集内挑最快（无害但字面不符） | ✅ 已改设计书（见下 commit）：§5.5 范围限制改「选点限定子集、测速覆盖全量」，「所见即所选」成立、「所见即所测」不字面成立 |

---

## 五、亮点清单（这些设计判断是对的，值得肯定）

1. **测量归内核**：`PROXY=select` + `PROXY__METRICS=url-test` 双组解耦，从测量源消除 TUN 自指回环——这是整个子系统治本的地基。
2. **热切换零断流**：禁整份 reload、只 `PUT /proxies/{group}`，断连风暴从结构上不可能。
3. **三层防污染校验**（回写前/还原前/前端反向校准）真落地，7-19 事故修复闭环。
4. **轻量状态机**：三态 CAS + 全局锁 + 失败回滚 + 不断用户连接只熔断 WS，状态机教科书级干净。
5. **主线程纪律**：窗口操作集中调度，只读查询放行，BUG-259 类崩溃系统性收敛。
6. **事件双过滤 + poison-tolerant 锁 + Drop Guard**：并发细节的基本功扎实，monitor 零 panic 路径。

---

## 六、建议行动顺序

1. 先修 3 个 🔴（手动切换不刷新 / JSON 原子写 / destroy 超时）——都是小改动、收益直接。
2. 再处理 🟡4/5/6（monitor 控制逻辑三件套：周期、全死计数、冷却记账统一）——这决定无人值守时的真实行为。
3. 顺手把设计书 5 条脱节改掉（半小时的事，防止后来者按幽灵机制做设计）。
4. 💭 项攒着，下次性能/稳定性专项一起清。

*本评审只做诊断，未改任何代码。*
