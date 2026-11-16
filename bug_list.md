# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：`BUG-057` ~ `BUG-391` 已通过用户验收并完整归档于 `clash_mini_agreements.md` 附录 A。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-392 过滤框中文输入法中间态抢跑与测速在途重触发丢弃导致排除词（如 -香港）偶发漏切

- **对应模块**：[node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts)、[base-search-box.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-search-box.tsx)、[proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx)、[use-head-state.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-head-state.ts)、[proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)
- **问题描述**：用户实机日志（`latest.log` `04:41:52` / `04:42:28`）复现出一条三环叠加竞态：① `BaseSearchBox` 未拦截中文输入法 IME 组词中间态（`onCompositionStart`/`onCompositionEnd`）且 `ProxyHead` 仅判 `!trimmed`，导致键入 `"-"` 或拼音中间串 `"-x"` 停顿 ≥700ms 即抢跑触发全量 41 节点 5 秒实时拨测；② 当该 5 秒探测在途期间用户正式提交 `"-香港"` 时，`ProxyHead` 将 `lastTriggeredFilterRef` 更新为 `"-香港"` 后调用 `ProxyGroups::handleCheckAll`，但 `handleCheckAll` 因 `testingGroupsRef.current[groupName]` 为 `true`（或后端返回 `AUTO_SELECT_BUSY`）直接 `return` 静默丢弃新子集请求且无尾部重测队列；③ 过期 41 节点探测完成后，`monitor.rs::trigger_backend_auto_select_inner` 先执行 `select_node_for_group("PROXY", "香港3 (043311)")` 再重读 `filter_text`，虽跳过回写 `profile.selected` 并记 `WARN`，却已把运行时出口切到被排除的香港节点。30s 缓存期内因旧探测 40ms 速通而无法稳定复现，冷探测 5s 窗口则必现。
- **修复方案**：
  1. 在 [node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts) 新增纯函数 `hasEffectiveFilterTerm` 与 `isIncompleteFilterInput` 并补全 [node.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.test.ts) 单测，识别孤立算子与末尾悬空前缀（如 `"-"`, `"!"`, `"日本 -"`, `"日本|"`）；
  2. 在 [base-search-box.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-search-box.tsx) 引入 IME 组词本地缓冲（`isComposingRef` + `composingText`），组词期间保持输入框平滑回显而不向外派发拼音中间态；在 [proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx) 拦截未完成过滤串抢跑触发；
  3. 在 [use-head-state.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-head-state.ts) 导出 `flushHeadStatePersist` 并在 [proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx) 的 `handleCheckAll`/`handleCheckFull` 选点前冲刷落盘；为 `handleCheckAll` 增加「过滤词变更尾部重测队列」（`pendingRecheckGroupsRef`）与有限次 `AUTO_SELECT_BUSY` 自动续试；
  4. 在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 抽出 `select_fastest_candidate` 并在 `select_node_for_group` **切换前**实时重读 `latest_filter_text` 过滤淘汰越界候选（若本批全数越界则安全跳过本次过期切换），并补齐 Rust 回归单测。
- **状态**：代码已修正，待用户确认

### BUG-393 新手引导与白名单说明补齐信任区具体目录/进程路径及 10 语种陈旧看门狗文案清理

- **对应模块**：[first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx)、[antivirus-whitelist-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/antivirus-whitelist-dialog.tsx)、`src/locales/*/layout.json`、[antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md)
- **问题描述**：① 新手引导第二步「使用 360 等安全软件？」提示卡片（`layout.components.antivirus.brief`）仅写「加入信任区」，未告知用户应把哪一个安装目录或可执行文件加入信任区；② 全文引导文档与 `antivirus.how` 此前仅列 `clash-mini.exe`，未并列说明整目录 `C:\Program Files\Clash Mini\` 及代理内核 `mini-mihomo.exe`；③ `ar`/`de`/`es`/`fa`/`id`/`jp`/`ko`/`ru`/`tr`/`tt` 共 10 个语种的 `antivirus.*` 仍残留 v3.3.3 已废除的「内置进程看门狗自动拉起」过时表述。
- **修复方案**：
  1. 在 13 个语种的 `antivirus.brief` 与 `antivirus.how` 及 [antivirus-whitelist.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/antivirus-whitelist.md) 明确写明加入安装目录（默认 `C:\Program Files\Clash Mini\`）或其中的 `clash-mini.exe` 与 `mini-mihomo.exe`；
  2. 为 [first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx) 与 [antivirus-whitelist-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/antivirus-whitelist-dialog.tsx) 补充 `overflowWrap: 'anywhere'` 防止 `271px` 窄窗下长路径溢出；
  3. 将 10 个滞后语种的 `why`/`benefit`/`consequence` 同步至现行「轻量模式关窗无主窗 + `mini-mihomo.exe` 驻留」真实架构描述。
- **状态**：代码已修正，待用户确认

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

历史已确认关闭条目（`BUG-057` ~ `BUG-391`）完整追溯映射详见 `clash_mini_agreements.md`「附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表」。

| 缺陷编号 | 对应版本基线 | 状态 |
| :--- | :---: | :---: |
| **BUG-391** | v3.5.0 | 代码已修正，已确认 |
| **BUG-390** | v3.5.0 | 代码已修正，已确认 |
| **BUG-389** | v3.5.0 | 代码已修正，已确认 |
| **BUG-388** | v3.5.0 | 代码已修正，已确认 |
| **BUG-387** | v3.4.2 | 代码已修正，已确认 |
| **BUG-386** | v3.4.2 | 代码已修正，已确认 |
| **BUG-385** | v3.4.2 | 代码已修正，已确认 |
| **BUG-384** | v3.4.2 | 代码已修正，已确认 |
| **BUG-383** | v3.4.1 | 代码已修正，已确认 |
| **BUG-382** | v3.4.0 | 代码已修正，已确认 |
| **BUG-381** | v3.4.0 | 代码已修正，已确认 |
| **BUG-380** | v3.4.0 | 代码已修正，已确认 |
| **BUG-379** | v3.4.0 | 代码已修正，已确认 |
| **BUG-378** | v3.3.4 | 文档已修正（废弃草案已物理移除），已确认 |
| **BUG-377** | v3.3.4 | 代码已修正，已确认 |
| **BUG-376** | v3.3.4 | 代码已修正，已确认 |
| **BUG-375** | v3.3.4 | 代码已修正，已确认 |
| **BUG-374** | v3.3.4 | 代码已修正，已确认 |
| **BUG-373** | v3.3.4 | 代码已修正，已确认 |
| **BUG-372** | v3.3.4 | 代码已修正，已确认 |
| **BUG-371** | v3.3.3 | 代码已修正，已确认 |
| **BUG-370** | v3.3.3 | 代码已修正，已确认 |
| **BUG-369** | v3.3.3 | 代码已修正，已确认 |
| **BUG-368** | v3.3.2 | ②④现行生效，①已由 BUG-377 收口，③清连接已停用移除，已确认 |
| **BUG-367** | v3.3.0 | 代码已修正，已确认 |
| **BUG-366** | v3.3.0 | 代码已修正，已确认 |
| **BUG-365** | v3.2.9 | 看门狗与 Session-0 拉起已于 v3.3.3 废止移除（仅留 WER/CrashDumps 取证），已确认 |
| **BUG-364** | v3.2.8 | 代码已修正，已确认 |
| **BUG-363** | v3.2.7 | 看门狗与哨兵窗已于 v3.3.3 废止移除（仅留安装清理与白名单指引），已确认 |
| **BUG-362** | v3.2.6 | 代码已修正，已确认 |
| **BUG-361** | v3.2.6 | 代码已修正，已确认 |
| **BUG-360** | v3.2.6 | 代码已修正，已确认 |
| **BUG-359** | v3.2.6 | 代码已修正，已确认 |
| **BUG-358** | v3.2.6 | 代码已修正，已确认 |
| **BUG-357** | v3.2.6 | 代码已修正，已确认 |
| **BUG-356** | v3.2.6 | 代码已修正，已确认 |
| **BUG-355** | v3.2.6 | 代码已修正，已确认 |
| **BUG-354** | v3.2.6 | 代码已修正，已确认 |
| **BUG-353** | v3.2.6 | 代码已修正，已确认 |
| **BUG-352** | v3.2.6 | 代码已修正，已确认（与 BUG-328 同源归档） |
| **BUG-351** | v3.2.6 | 代码已修正，已确认（与 BUG-329/BUG-330 同源归档） |
| **BUG-350** | v3.2.6 | 代码已修正，已确认（与 BUG-332 同源归档） |
| **BUG-349** | v3.2.6 | 代码已修正，已确认（与 BUG-322 同源归档） |
| **BUG-348** | v3.2.6 | 代码已修正，已确认（与 BUG-320 同源归档） |
| **BUG-347** | v3.2.6 | 代码已修正，已确认 |
| **BUG-346** | v3.2.6 | 代码已修正，已确认（与 BUG-319 同源归档） |
| **BUG-345** | v3.2.6 | 代码已修正，已确认（与 BUG-317 同源归档） |
| **BUG-344** | v3.2.6 | 直测已回退并收口加固，已确认 |
| **BUG-343** | v3.2.6 | 代码已修正，已确认 |
| **BUG-342** | v3.2.6 | 代码已修正，已确认 |
| **BUG-341** | v3.2.6 | 代码已修正，已确认 |
| **BUG-340** | v3.2.6 | 代码已修正，已确认 |
| **BUG-339** | v3.2.6 | 代码已修正，已确认 |
| **BUG-338** | v3.2.6 | 代码已修正，已确认 |
| **BUG-337** | v3.2.6 | 代码已修正，已确认 |
| **BUG-336** | v3.2.6 | 代码已修正，已确认 |
| **BUG-335** | v3.2.6 | 代码已修正，已确认 |
| **BUG-334** | v3.2.6 | 代码已修正，已确认 |
| **BUG-333** | v3.2.6 | copy_clash_env 零调用死命令已移除（10801 默认端口现行生效），已确认 |
| **BUG-332** | v3.2.6 | 代码已修正，已确认 |
| **BUG-331** | v3.2.6 | 代码已修正，已确认 |
| **BUG-330** | v3.2.6 | 代码已修正，已确认 |
| **BUG-329** | v3.2.6 | 代码已修正，已确认 |
| **BUG-328** | v3.2.6 | 代码已修正，已确认 |
| **BUG-327** | v3.2.6 | 代码已修正，已确认 |
| **BUG-326** | v3.2.6 | 代码已修正，已确认 |
| **BUG-325** | v3.2.6 | 代码已修正，已确认 |
| **BUG-324** | v3.2.6 | 代码已修正，已确认 |
| **BUG-323** | v3.2.6 | 代码已修正，已确认 |
| **BUG-322** | v3.2.6 | 代码已修正，已确认 |
| **BUG-321** | v3.2.6 | 代码已修正，已确认 |
| **BUG-320** | v3.2.6 | 代码已修正，已确认 |
| **BUG-319** | v3.2.6 | 代码已修正，已确认 |
| **BUG-318** | v3.2.6 | 代码已修正，已确认 |
| **BUG-317** | v3.2.6 | 代码已修正，已确认 |
| **BUG-316** | v3.2.5 | 代码已修正，已确认 |
| **BUG-315** | v3.2.4 | 代码已修正，已确认 |
| **BUG-314** | v3.2.3 | 代码已修正，已确认 |
| **BUG-312** | v3.2.2 | 代码已修正，已确认 |
| **BUG-311** | v3.2.1 | 代码已修正，已确认 |
