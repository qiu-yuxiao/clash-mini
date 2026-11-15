# Clash Mini 待处理 Bug 跟踪列表 (Pending Bug Tracking List)

> [!IMPORTANT]
> **AI 协同开发与维护规范 (CRITICAL MAINTENANCE RULES)**:
> 1. **待验证/排查中 Bug 强制卡片化**：所有状态为 `排查中` 或 `代码已修正，待用户确认` 的条目必须在顶部「待验证与活动中 Bug 详情」区以结构化卡片登记，严禁在用户明确人工确认前提前归档入底部历史表。
> 2. **已确认历史条目索引**：`BUG-057` ~ `BUG-378` 已通过用户验收并完整归档于 `clash_mini_agreements.md` 附录 A。

---

## 📌 待验证与活动中 Bug 详情 (Active & Pending Bugs)

### BUG-379 后台监测稳态 60s 全组盲测风暴治理：被动下行流量证活 + 停滞秒级复核 + 空闲 300s 降频

- **对应模块**：[monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs)（`evaluate_failover`、`should_probe_active_node`、`sample_active_node_traffic`）
- **问题描述**：`HEALTH_DATA_FRESH_SECS` 原固定为 `60` 秒，导致即便用户正在流畅浏览/看视频，后台守护线程仍每分钟对 `PROXY__METRICS` 全组所有节点发起一轮 `delay_group` 并发握手拨测（以 50 节点计每小时高达 3,000 次跨网握手，干扰长流且耗费机场/硬件资源）；而简单拉长阈值又会拖慢真死节点的自愈发现速度。
- **修复方案**：引入基于本地命名管道 `/connections` 的**被动流量证活状态机**（零外网发包、`history` 缓存保鲜封顶 `480s < 600s` 前端兜底上限，且严守 §5.2 铁律绝不启用 `/proxies/{name}/delay`）：
  1. **活跃传输态（`delta_download > 0`）**：下行回包直接证实远端真实存活，保鲜上限放宽至 `PASSIVE_TRAFFIC_MAX_SKIP_SECS (480s)`，稳态拨测频率直降 **87.5%**；
  2. **黑洞停滞态（`delta_upload > 0 && delta_download == 0`）**：有出无回立即突破缓存在本轮 15s 周期内触发 `probe_metrics_group` 复核（配 `STALL_REPROBE_COOLDOWN_SECS = 60s` 防单向流抖动冷却），使用中突发死节点恢复响应从最慢 `63s` 缩短至 **`~18s`**；
  3. **完全空闲态（无字节变化）**：保鲜窗口放宽至 `IDLE_HEALTH_FRESH_SECS (300s)`，对齐 `PROXY__METRICS` 自然周期，静默挂机拨测直降 **80%**；非 `Alive`（`Suspect`/`Dead`/`Unknown`）仍强制实拨，保留 BUG-377 异源隔离不变式。
- **状态**：代码已修正，待用户确认

---

### BUG-380 真 HTTPS 验真「终身死刑」改为 24h 软隔离自动缓刑 + 手动更新订阅解封

- **对应模块**：[node.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/node.rs)（`NodeVerifyRecord`、`apply_verify_batch_to_records`、`clear_unverified_https_failures`）、[profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/profile.rs)（`update_profile`）
- **问题描述**：此前 `node_https_verify.json` 的假节点判定一旦累计失败超过 5 次，便在无过期机制与解封入口的情况下对节点指纹永久拉黑屏蔽；机场短时维护或 GitHub/Wikipedia 验真源偶发抽风后，真节点即使早已恢复也永远无法在 Clash Mini 中重见天日（重启或重导同节点订阅均无效）。
- **修复方案**：
  1. 复用持久化字段 `last_verified_at` 实行 **24 小时软隔离 TTL**（`HTTPS_VERIFY_BAN_TTL_SECS = 86_400`，零格式迁移成本）：满 24h 自动解除隔离并在下一轮恢复观察；期间若重新失败则清零重计一轮 6 次宽限期，若通过则升级单向粘滞真值 `verified_true = true`；
  2. 新增 `clear_unverified_https_failures()`，并在用户**手动点击更新订阅**（`is_mannual_trigger == true`）时自动清空所有未验真节点的历史失败计数与软隔离标记（保留已验真白名单），立即随配置重载/列表刷新恢复可见。
- **状态**：代码已修正，待用户确认

---

### BUG-381 内置脚本增强退场：`boa_engine` 改为原生 Rust 卫兵

- **对应模块**：[enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs)（`apply_builtin_guards`）、[enhance/chain.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/chain.rs)、[config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/config.rs)、[prfitem.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/prfitem.rs)、[validate.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs)、[Cargo.toml](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/Cargo.toml)
- **问题描述**：Clash Mini 遵循零脚本编辑器的极简哲学，前端早已不提供 JS 脚本入口，但后端仍全量引入重型 Rust JS 引擎 `boa_engine`（`v0.21.1`），并在每次 `enhance()` 与配置初始化时执行 `meta_guard.js` / `meta_hy_alpn.js`（仅做 2 项极简 YAML 字段归一化）以及创建无用的默认 `Script` 增强文件，白白增大二进制体积、编译耗时与内存开销。
- **修复方案**：
  1. 在 [enhance/mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/mod.rs) 以原生 Rust `apply_builtin_guards` 等价替代原内置 `meta_guard.js`（`mode: script` → `rule`）与 `meta_hy_alpn.js`（`hysteria` 字符串 `alpn` → 单元素数组）；
  2. 移除 `boa_engine` 依赖、`src-tauri/src/enhance/script.rs` 与 `builtin/*.js`；停止 `ensure_default_profile_items` / `create_enhance_items` 新建 `Script` 项，同时保持对存量 `profiles.yaml` 中历史 `type: script` 条目与旧 `.js` 文件的 100% 向后兼容放行（安全忽略无操作）。
- **状态**：代码已修正，待用户确认

---

## 📌 已解决的历史 Bug 索引 (Resolved Historical Bugs)

历史已确认关闭条目（`BUG-057` ~ `BUG-378`）完整追溯映射详见 `clash_mini_agreements.md`「附录 A: 历史缺陷与工程修复（Bug ID）追溯映射表」。

| 缺陷编号 | 对应版本基线 | 状态 |
| :--- | :---: | :---: |
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
