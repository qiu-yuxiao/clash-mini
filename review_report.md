# 代码审核报告 — v2.4.4 后激进优化（4d9b1000）

**审核时间**: 2026-07-11 23:58  
**commit**: `4d9b1000` — perf: heavily optimize mihomo memory footprint for lightweight usage  
**变更量**: 2 文件，+10/-3

---

## 总评

**整体质量: B，有 1 个 🔴 blocker 级问题。**

虽然只有 13 行净变更，但每一行都涉及内核配置或 Go 运行时调优。其中 `store-selected: false` 这条有潜在的数据一致性和自愈异常风险，是最大隐患。

---

## 逐条分析

### 1. 🔴 `profile.store-selected = false` — 破坏自愈/重载的节点选择持久化

**位置**: `src-tauri/src/config/clash.rs:114`  
**改动**: `store-selected` 从 `true` → `false`

**这行是什么意思？**  
mihomo 内核的 `store-selected` 控制是否在内存/文件中持久化用户对各策略组的手动节点选择。当 Clash Mini 重启内核时（config 变更、自愈切换、allow-lan/ipv6 切换等），若 `store-selected: true`，内核会自动恢复到之前选定的节点。若 `store-selected: false`，内核重启后所有策略组节点都会被重置为默认（通常是 `DIRECT` 或 `REJECT`）。

**为什么这是 blocker？**

Clash Mini 的核心功能正是选点 + 自愈。内核重启在 Clash Mini 中是常态——比如 `patch_clash` 中 allow-lan/ipv6 变更会触发 `restart_core`，`patch_verge` 中端口变更、TUN 模式切换都会触 `RESTART_CORE`。每次重启后：

1. PROXY 组节点被重置为默认
2. 自愈逻辑（`monitor.rs`）再重新选最快的——这个流程本身没问题
3. 但**其他策略组**（如选择器组、手动分流组）的节点选择会丢失！如果用户手动在某个分流组里选了特定节点，重启后它就丢了。

**自愈链路的实际影响**：

当前自愈流程是 `monitor.rs:restore_profile_selected_nodes` → `trigger_backend_auto_select`。`restore_profile_selected_nodes` 从 profile 配置文件中读取 `selected` 字段，然后把之前保存的节点选择发回给内核。但若内核 `store-selected: false`，这个恢复操作虽然能生效（Clash Mini 通过 API 设回去了），但内核下次重启不会再持久化它。

所以当前自愈链路的外部行为可能没变——Clash Mini 在重启后仍然通过 profile 的 `selected` + auto-select 来恢复节点。**但这是一个脆弱的依赖**：`restore_profile_selected_nodes` 和 `store-selected` 之间存在隐式的功能冗余。如果有人将来优化了 `restore_profile_selected_nodes`（比如删了它觉得重复），自愈就断了。

**建议**：
- **必须确认** `store-selected: false` 是否真的必要。省的是内核内部的存储，这个数据极小（每个策略组存一个节点名）。
- 如果确实要省这点内存，至少补一条注释说明自愈链路依赖 `restore_profile_selected_nodes` 来弥补，两者需一起维护。

---

### 2. 🟡 `GOMEMLIMIT 96MiB → 64MiB` + `GOGC 50 → 30`

**位置**: `src-tauri/src/core/manager/state.rs:45-46`

**改动**：Go 内存限制从 96MB 降到 64MB，GC 触发阈值从 50% 降到 30%。

**实际收益**：64MB 上线 + GC 更频繁 ≈ mihomo 常驻内存 ~40-50MB。相比原来 96MB + GOGC 50 的大约 ~60-70MB，能省大约 15-20MB。

**风险**：
- mihomo 在解析大订阅配置（几百个节点）时，瞬时内存可能超过 64MB 硬限制，导致 Go 运行时 OOM。自定义订阅（如带 ruleset 的）尤其危险。
- GOGC 30 会让 GC CPU 开销上升约 67%（相对 50），在低端机器上可能增加延迟抖动。

**建议**：可以接受，但建议验证大订阅场景下不会 OOM。64MB 对于纯净版 mihomo 通常够用，但若用户开启了完整 geoip/geosite 数据库，可能不够。

---

### 3. 💭 `tcp-concurrent: false`

**位置**: `src-tauri/src/config/clash.rs:106`

禁用 TCP 并发连接处理。mihomo 在处理大量 TCP 连接时，并发模式会为每个连接分配额外内存。禁用后可以省内存，但单连接吞吐量会下降。

对于 Clash Mini 的"轻量使用"定位，可以接受。

---

### 4. 💭 `dns: { enable: true, cache-size: 512 }`

**位置**: `src-tauri/src/config/clash.rs:108-111`

新增 DNS 最小配置。此前 `generate_minimal_config` 不包含 DNS 配置，mihomo 会用默认值。显式设置 `cache-size: 512`（默认是 4096）可以减少 DNS 缓存的常驻内存。512 条对于单用户设备也够用。

**注意**：启用了 DNS 后，内核的 DNS 解析行为会改变——`dns.enable: true` 意味着所有 DNS 查询都会经过 mihomo 的 DNS 模块，而不仅仅是代理流量。这可能导致本地程序（如 .exe 开发工具）的域名解析行为变化。但 Clash Mini 之前的运行时配置（非兜底配置）已经包含了 DNS 设置，所以这个变化只影响"兜底启动"的场景。

---

## 总结

| 级别 | 说明 | 文件 | 状态 |
|------|------|------|------|
| 🔴 | `profile.store-selected` 已改回 `true` | `config/clash.rs:114` | ✅ 已修 |
| 🟡 | `GOMEMLIMIT 64MiB` + `GOGC 30` — 省 ~15-20MB 内存，但大订阅配置可能 OOM | `core/manager/state.rs:45-46` | 保留 |
| 💭 | `tcp-concurrent: false` — 可接受 | `config/clash.rs:106` | 保留 |
| 💭 | `dns: { cache-size: 512 }` — 只影响兜底启动场景，可接受 | `config/clash.rs:108-111` | 保留 |

**建议优先讨论 🔴 blocker 项**：`store-selected` 为什么要改成 `false`？省的是什么？
