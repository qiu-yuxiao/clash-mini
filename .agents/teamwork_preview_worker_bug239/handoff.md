# BUG-239 Code Audit Report - Handoff Report

## 1. Observation

1. In [crates/tauri-plugin-mihomo/src/commands.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs):
   - Command `delay_proxy_by_name` (lines 205-216) emits the refresh event unconditionally after calling speed test, even if the result `res` represents a failed test:
     ```rust
     let res = mihomo.delay_proxy_by_name(&proxy_name, &test_url, timeout).await;
     let _ = app.emit("verge://refresh-proxy-config", "yes");
     res
     ```
   - Command `healthcheck_node_in_provider` (lines 153-164) executes node healthchecks without calling `app.emit(...)` to trigger a frontend refresh.
   - Command `reload_config` (lines 245-248) reloads configuration but does not emit `verge://refresh-clash-config` or similar events.
   - Command `update_rule_provider` (lines 232-235) updates rules but does not emit refresh events.

2. In [src/providers/app-data-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/app-data-provider.tsx):
   - The React Query `getProxies` query is registered on lines 190-199.
   - The event listener for `'verge://refresh-clash-config'` (lines 333-336) is registered:
     ```typescript
     const uClash = await listen(
       'verge://refresh-clash-config',
       handleRefreshProxy,
     )
     ```
     This triggers `handleRefreshProxy`, which calls `refreshProxy()` (calls `_refetchProxy()` direct query refetch).
   - In `handleProfileChanged` (lines 280-294) and `handleRefreshProxy` (lines 296-301), a single variable `lastUpdateTime` is shared to throttle updates (lines 277-278):
     ```typescript
     let lastUpdateTime = 0
     const refreshThrottle = 800
     ```

3. In [src/pages/_layout/hooks/use-layout-events.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-layout-events.ts):
   - The event listener for `'verge://refresh-clash-config'` (lines 46-53) is registered:
     ```typescript
     register(
       addListener('verge://refresh-clash-config', async () => {
         revalidateKeys([
           'getProxies',
           'getVersion',
           'getClashConfig',
           'getProxyProviders',
         ])
       }),
     )
     ```
     This invalidates query keys including `'getProxies'`, which forces React Query to automatically refetch.

4. In [src/hooks/use-connection-data.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-connection-data.ts):
   - The connection hook `useConnectionData` (lines 25-45) disables the WebSocket subscription when `isWsActive` is `false` (drawer closed). However, there is no HTTP REST polling fallback implementation to update totals periodically.

5. In [clash_mini_agreements.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md):
   - Section 六 (lines 2066-2076) mandates:
     > "当设置面板关闭时（`drawerOpen === false` 且可见），连接管理自动切换为**低频静默监控模式**：断开 WebSocket，降级为每 3 秒发起单次轻量级 `getConnections` HTTP REST 轮询。在此模式下，为了节省 CPU，**严禁**执行任何连接列表的差异对比、排序及 Map 内存重构计算，直接提取 totals 计入状态，且保持连接明细列表为空数组。"

---

## 2. Logic Chain

1. **Question 1: Missing Triggers**
   - Direct observation (1) shows that `healthcheck_node_in_provider`, `reload_config`, and `update_rule_provider` perform core state modifications but do not emit events.
   - Therefore, the UI fails to reflect updated states immediately after these commands run.

2. **Question 2: Speed Test Fails & `delay_proxy_by_name`**
   - Direct observation (1) shows `app.emit("verge://refresh-proxy-config", "yes")` is called unconditionally.
   - Therefore, the refresh event is emitted even if the speed test fails.

3. **Question 3: Double Listening Issue**
   - Direct observations (2) and (3) show that both `app-data-provider.tsx` and `use-layout-events.ts` register separate listeners for `'verge://refresh-clash-config'`.
   - The first listener calls React Query's `refetch()` directly. The second listener invalidates the same query key (`'getProxies'`), which also triggers a refetch.
   - Therefore, when `'verge://refresh-clash-config'` is received, the frontend initiates two concurrent network requests for the same proxies data, causing duplicate fetches and race conditions.

4. **Question 4: Throttle implementation & Event Loss**
   - Direct observation (2) shows that `lastUpdateTime` is shared. A profile change sets `lastUpdateTime = Date.now()`.
   - Therefore, any proxy refresh event arriving within 800ms of a profile change gets throttled and dropped, leading to out-of-date UI representation.

5. **Question 5: Compliance with `clash_mini_agreements.md`**
   - Direct observation (4) shows no polling fallback is implemented in `use-connection-data.ts` when `isWsActive` is false.
   - Direct observation (5) shows the agreement requires a 3-second REST polling fallback under these conditions.
   - Therefore, the current codebase violates Section 六 of the development agreements.

---

## 3. Caveats

- We did not perform live runtime verification of the compiled app because the shell commands timed out.
- The analysis relies on static analysis and the findings of the Explorer agent.
- No source code modifications were performed, conforming strictly to the "Do NOT modify any source files. Write ONLY to docs/bug239_audit_report.md" directive.

---

## 4. Conclusion

The BUG-239 audit report has been fully compiled and written to `docs/bug239_audit_report.md`. The report contains:
1. Executive Summary
2. Correctness & Completeness Audit (R1)
3. Potential Issues & Security Risks (R2)
4. Best Solutions & Alternatives Comparison (R3)
5. Code Quality & Architectural Consistency (R4)
6. Concrete Code Improvement Diffs (addressing issues a, b, c, d, and e)

All file paths and line number ranges are linked with clickable `file:///` links.

---

## 5. Verification Method

- Check the content of the generated report at [docs/bug239_audit_report.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/docs/bug239_audit_report.md).
- Run `git status` to verify that no source code files are modified. Only `docs/bug239_audit_report.md` should be newly added.
