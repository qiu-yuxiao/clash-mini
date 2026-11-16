# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：`BUG-057` ~ `BUG-387` 已通过用户验收并完整归档于 `clash_mini_agreements.md` 附录 A。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-388 后台测量组自然周期激活、真 HTTPS 验真防误切与自愈逃生/防竞态收口

- **对应模块**：[mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)、[constants.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/constants.rs)、[node.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/node.rs)、[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)、[lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs)、[node.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.test.ts)
- **问题描述**：① `PROXY__METRICS` 未显式设置 `lazy: false`，Mihomo 默认对无流量组跳过周期拨测，削弱 300s 自然保鲜前提；② `has_verify_failure_node` 仅失败 1 次即允许对正常传输（`Flowing`）中的健康节点触发 `ShouldMigrate`，且真 HTTPS 验真靶站全灭时缺少退避、导致每 60s/180s 反复全组拨测；③ `ApiChannelDown` 通道异常恢复后无条件按最快节点重选切换、`ConfirmedDead` 二次逃生受 `120s` 切换速率上限封锁、后台自愈在选点期间可能覆盖用户同期手动切换的节点；④ `lifecycle::select_fallback_in_subset` 在非空过滤词下末级回退会越出排除子集（如 `-香港` 误落回香港节点）；⑤ `node.rs` 冒号配额空白判断仅跳 ASCII 空白、与前端 `\s*` 在全角空格上存在差异，且 `monitor.rs` 重复定义 `NODE_DELAY_MAX_MS`。
- **修复方案**：
  1. 在 [mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs) 为 `PROXY__METRICS` 显式注入 `lazy: false` 并补齐单测断言；
  2. 在 [node.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/node.rs) 新增 `HTTPS_VERIFY_MIGRATE_MIN_FAILS = 2` 收紧迁移阈值、将 `has_colon_quota_or_date` 空白跳过对齐 Unicode（同步双端全角空格单测），并在 [monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs) 为验真全灭新增 `HTTPS_VERIFY_ALL_FAILED_BACKOFF_SECS (600s)` 退避、`Flowing` 下行态禁止低次验真迁移（`should_allow_https_migration`）；
  3. 放行 `ConfirmedDead` 逃生免受 `120s` 切换速率锁（`self_heal_subject_to_switch_rate_limit`），为 `ApiChannelDown` 补充恢复保活门检（API 已恢复且原出口可用时仅刷延迟不换节点），并引入 `MANUAL_SWITCH_EPOCH` 防止后台自愈覆盖手动切点及收敛复用 `constants::timing::NODE_DELAY_MAX_MS`；
  4. 在 [lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs) 抽出 `pick_subset_fallback_node`，仅当 `filter_text` 为空时才回退全量列表，彻底禁止非空过滤越界盲选。
- **状态**：代码已修正，待用户确认

### BUG-389 主界面过滤框聚焦下 F1/F4 快捷键失灵修复与新手引导全协议导入放通

- **对应模块**：[disable-webview-shortcuts.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/disable-webview-shortcuts.ts)、[node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts)、[first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx)、[window-resizing.test.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/window-resizing.test.ts)
- **问题描述**：① `ProxyHead` 挂载 100ms 后自动聚焦过滤框，而 `disable-webview-shortcuts.ts` 在 `isEditableTarget` 提前返回，导致开窗直接按或输入过滤词时按 `F1`/`F4` 测速快捷键被静默吞掉；② `FirstRunGuideDialog` 硬性拦截非 `http(s)://` 链接，手持 `vmess://` / `vless://` / `ss://` / `trojan://` / `hy2://` 等节点配置的用户首屏直接被拒（与全协议支持定位不符），剪贴板嗅探亦未覆盖单节点 URI，且重开引导导入会覆盖用户已选「手动模式」。
- **修复方案**：
  1. 调整 [disable-webview-shortcuts.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/disable-webview-shortcuts.ts) 调度顺序，在无弹窗遮挡/非 IME/非连发前提下于 `isEditableTarget` 前优先放行无修饰 `F4`/`F1`，保留 `Ctrl+F` 仅非编辑态触发；
  2. 在 [node.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/utils/node.ts) 导出 `SUPPORTED_NODE_URI_SCHEMES` 与 `isSupportedSubscriptionOrNodeInput` 统一校验 `http(s)://` 订阅与 `universal_parser.rs` 支持的 11 种节点协议 URI（由 [first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx) 统一复用并放通剪贴板单/多行节点嗅探），限定仅首次引导未完成时才自动开启系统代理，并补齐 Vitest 单元测试。
- **状态**：代码已修正，待用户确认

### BUG-390 订阅删除二次确认防呆、物理断网胶囊防盲切与全界面多语言/提示收口

- **对应模块**：[profile-import-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/profile-import-card.tsx)、[layout-dialogs.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/layout-dialogs.tsx)、[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx)、[active-node-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx)、[first-run-guide-dialog.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/first-run-guide-dialog.tsx)、`src/locales/*`
- **问题描述**：① 订阅列表右侧 12px 删除按钮无 Tooltip/无二次确认即直接删除订阅，易误删唯一配置导致断网；② 物理断网警示胶囊点击直接以 `select=true` 盲切节点，违背连续性优先；③ `_layout.tsx` 存在英文硬编码报错、首次引导按钮存在双星号 `⭐ ⭐`、闪电测速按钮与过滤框未提示 `F4`/`F1`/过滤语法。
- **修复方案**：
  1. 为订阅卡片刷新/删除按钮补全无障碍与 Tooltip 提示，并在 [layout-dialogs.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/layout-dialogs.tsx) 与 [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) 接入窄窗 3D 删除二次确认弹窗；
  2. 在 [active-node-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx) 限定仅超时态胶囊点击触发测速择优，物理断网态点击改用 `select = false` 仅探活恢复；
  3. 清除引导双星号与硬编码英文报错，在 13 个语种资源同步补全 `layout.notices.profileActivationFailed`、多协议提示、`F4/F1` 测速提示与过滤语法占位符。
- **状态**：代码已修正，待用户确认

### BUG-391 全栈历史死代码、零引用依赖清扫与六合一权威文档/架构基线大对齐

- **对应模块**：[app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs)、[network.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/network.rs)、[profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs)、[runtime.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/runtime.rs)、[sysinfo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/sysinfo.rs)、[Cargo.toml](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/Cargo.toml)、[cmds.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts)、[clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md)、[ARCHITECTURE.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/ARCHITECTURE.md)、[implementation-notes.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/implementation-notes.md)、[用户必读.txt](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/%E7%94%A8%E6%88%B7%E5%BF%85%E8%AF%BB.txt)
- **问题描述**：① 仓库残留 `src-tauri/.agents/.../progress.md` 审计中间文件、17 个未注册/零调用后端命令及 4 个零引用 Rust 依赖（`network-interface`、`gethostname`、`tokio-rustls`、`rust_iso3166`）与无用 `criterion` 开发依赖；② 前端残留零调用 IPC 封装、`resize-geometry.ts` 废弃原型及若干废弃 i18n 键；③ `clash_mini_agreements.md` 目录与正文标题失步、`ARCHITECTURE.md` 滞后于 `boa_engine`/流媒体解锁移除与实际连接池配置、`implementation-notes.md` 节号错位、`用户必读.txt` 仍写旧「自动连切」叙事。
- **修复方案**：
  1. 清除残留跟踪文件、零调用死命令/辅助函数/死依赖及前端废弃缩放原型与封装，执行 `pnpm i18n:format && pnpm i18n:types` 瘦身词条；
  2. 全面对齐 `clash_mini_agreements.md`、`docs/ARCHITECTURE.md`、`docs/implementation-notes.md` 与 `用户必读.txt` 至 `v3.5.0` 现行架构与设计事实，移除虚构/已废除标识符描述。
- **状态**：代码已修正，待用户确认

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

历史已确认关闭条目（`BUG-057` ~ `BUG-387`）完整追溯映射详见 `clash_mini_agreements.md`「附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表」。

| 缺陷编号 | 对应版本基线 | 状态 |
| :--- | :---: | :---: |
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
