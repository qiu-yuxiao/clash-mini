# BUG-239 Code Audit Report: Event-Driven Synchronization and Background Resource Leak Auditing

## 1. Executive Summary

This code audit report evaluates the correctness, performance, and compliance of the backend-to-frontend event-driven synchronization system in Clash Mini (ClashVerge). Specifically, it assesses the mechanisms introduced to address BUG-239 regarding backend status updates, UI refreshes, and compliance with the development agreements. 

The audit covers two primary subsystems:
1. **The Event System**: Investigating Tauri plugin commands, custom event handlers, event name consistency, throttling behaviors, and race conditions leading to redundant fetches.
2. **The Connection Monitor System**: Reviewing the WebSocket subscription lifestyle, `ResizeObserver` performance, window visibility checks, and auditing compliance with background resource minimization.

Our assessment reveals that while BUG-239 corrections introduced vital updates, several correctness gaps (such as missing triggers and double-fetching) and an agreement violation (missing REST polling fallback) must be resolved to ensure complete system robustness and optimal CPU usage.

---

## 2. Correctness & Completeness Audit (R1)

### 2.1 Backend Commands Analysis
We conducted a detailed analysis of the following key files:
- [crates/tauri-plugin-mihomo/src/commands.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs)
- [src-tauri/src/core/handle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/handle.rs)
- [src-tauri/src/core/notification.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/notification.rs)

The Tauri plugin `tauri-plugin-mihomo` exposes several async Rust commands that invoke backend functions of the Mihomo core. When a command alters the state of the backend proxy configurations or rules, it should notify the frontend via an IPC event. However, this notifications flow is decentralized:
- Commands like `select_node_for_group` and `delay_proxy_by_name` directly call `app.emit("verge://refresh-proxy-config", "yes")`.
- The main backend crate manages events centrally using `Handle` (defined in [src-tauri/src/core/handle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/handle.rs)) and the `NotificationSystem` (defined in [src-tauri/src/core/notification.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/notification.rs)), which translates structured `FrontendEvent` enums into event names (e.g. `verge://refresh-clash-config`, `verge://refresh-proxy-config`).

### 2.2 Missing Triggers
**Question**: Are there any missing triggers?
**Answer**: **YES**.

#### Evidence:
1. **`healthcheck_node_in_provider` (lines 153-164)**:
   In [crates/tauri-plugin-mihomo/src/commands.rs:153-164](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs#L153-L164), the command runs the healthcheck for a single node inside a proxy provider but does **not** call `app.emit(...)` to trigger a frontend refresh. As a result, the frontend node lists display stale delay metrics until a full test is manually initiated elsewhere.
2. **`reload_config` (lines 245-248)**:
   In [crates/tauri-plugin-mihomo/src/commands.rs:245-248](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs#L245-L248), reloading the configuration changes the entire routing tree and proxy layout on the backend. This command has **no** event emission, resulting in the UI showing out-of-date proxy lists.
3. **`update_rule_provider` (lines 232-235)**:
   In [crates/tauri-plugin-mihomo/src/commands.rs:232-235](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs#L232-L235), updating a rule provider successfully writes changes to the core state but emits no events, leaving the rule overview UI out of sync.
4. **Loop Updates**:
   When the frontend loops to update multiple proxy providers, it triggers the Rust command `update_proxy_provider` once per provider. Each invocation issues an immediate, individual `app.emit("verge://refresh-proxy-config", "yes")`. These rapid-fire events collide directly with the frontend's throttle window, meaning all updates except the first one are ignored by the UI.

### 2.3 Event Name Consistency and Race Conditions
There is a minor structural inconsistency between events emitted directly by the plugin commands and those emitted via `Handle`. Although the string literals match (`"verge://refresh-proxy-config"` and `"verge://refresh-clash-config"`), they bypass the central `NotificationSystem`. 

Because events are emitted immediately without debouncing on the backend, multiple sequential command executions generate a flood of IPC messages. If these messages arrive within the frontend's throttle threshold, events are dropped, causing stale UI states. Conversely, if they fall outside the throttle threshold, they cause redundant network fetches and React re-render loops.

---

## 3. Potential Issues & Security Risks (R2)

### 3.1 Unconditional Speed Test Event Emission
**Question**: Does `delay_proxy_by_name` emit the refresh event even when the speed test fails?
**Answer**: **YES**.

#### Code Evidence:
In [crates/tauri-plugin-mihomo/src/commands.rs:205-216](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs#L205-L216):
```rust
#[command]
pub(crate) async fn delay_proxy_by_name<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<Mihomo>>,
    proxy_name: String,
    test_url: String,
    timeout: u32,
) -> Result<ProxyDelay> {
    let mihomo = state.read().await.clone();
    let res = mihomo.delay_proxy_by_name(&proxy_name, &test_url, timeout).await;
    let _ = app.emit("verge://refresh-proxy-config", "yes");
    res
}
```
The variable `res` holds the result of the backend speed test (which may be a `Result::Err`). The event emission `app.emit("verge://refresh-proxy-config", "yes")` is called unconditionally on the next line before returning `res`. Consequently, even if a node speed test fails due to timeout or network errors, a refresh event is broadcast to the entire frontend.

#### Safety of `app.emit`:
`app.emit` is inherently thread-safe and non-blocking. However, emitting events unconditionally on failures creates unnecessary work for the frontend. It triggers query invalidation and refetching of `/proxies` from the core over IPC, leading to CPU spikes and layout recalculations.

### 3.2 Double Listening to `"verge://refresh-clash-config"`
**Question**: Does the double listening of `"verge://refresh-clash-config"` in `app-data-provider.tsx` and `use-layout-events.ts` cause duplicate fetches or race conditions?
**Answer**: **YES**.

#### Analysis:
1. In [src/providers/app-data-provider.tsx:333-336](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/app-data-provider.tsx#L333-L336):
   ```typescript
   const uClash = await listen(
     'verge://refresh-clash-config',
     handleRefreshProxy,
   )
   ```
   This listener responds to `verge://refresh-clash-config` by calling `handleRefreshProxy`, which triggers `refreshProxy()` (i.e. `_refetchProxy()` which invokes `fetchProxies` query directly).
2. Concurrently, in [src/pages/_layout/hooks/use-layout-events.ts:46-53](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-layout-events.ts#L46-L53):
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
   This listener reacts to the same event by invalidating the React Query cache key `'getProxies'`. In `@tanstack/react-query`, invalidating an active query automatically forces a refetch.

Because both listeners fire simultaneously, the frontend sends two concurrent, duplicate HTTP requests to retrieve proxy data. This causes double fetches, unnecessary IPC traffic, and a race condition where out-of-order network responses could result in UI flicker or stale displays.

### 3.3 ResizeObserver and Transition Behaviors
In [src/pages/_layout.tsx:1042-1065](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1042-L1065):
- **Slide Animations**: When the settings drawer opens, the width of the panel transitions from `0px` to `350px`. The `ResizeObserver` sets `isPanelVisible` to `true` as soon as the width exceeds `10px`. This means the WebSocket connection is established and starts receiving traffic data *during* the opening animation rather than waiting for it to finish.
- **Connection Thrashing**: If the drawer rests at an intermediate state near `10px`, or is toggled rapidly, the observer will fire repeatedly, resulting in rapid connect/disconnect loops of the WebSocket connection.
- **Immediate Disconnect**: When the drawer is closed, `drawerOpen` changes to `false`, immediately setting `isPanelVisible = false` and `isWsActive = false`. This terminates the WebSocket connection instantly, adhering to the background communication靜默 (silence) requirements.

### 3.4 Shared Throttle and Event Loss
In [src/providers/app-data-provider.tsx:277-301](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/app-data-provider.tsx#L277-L301):
```typescript
let lastUpdateTime = 0
const refreshThrottle = 800
```
`lastUpdateTime` is shared by both `handleProfileChanged` and `handleRefreshProxy`.
- When a profile change event is processed, `lastUpdateTime` is updated to `Date.now()`.
- If a proxy refresh event (`verge://refresh-proxy-config`) is received within `800ms` of the profile change, the calculation `now - lastUpdateTime <= refreshThrottle` evaluates to `true` and the proxy refresh returns early.
- As a result, the proxy data is never refetched, causing the node list to display stale information.

---

## 4. Best Solutions & Alternatives Comparison (R3)

### 4.1 Event System Architecture Comparison

| Approach | Advantages | Disadvantages | Suitability |
| :--- | :--- | :--- | :--- |
| **Direct Tauri Emit** (Current) | Extremely simple to write; low boilerplate. | Couples Rust command plugins directly to Tauri's global event system. No centralized control, debouncing, or event lifecycle. | Poor for complex state changes. |
| **Backend Central Emit (`Handle`)** | Decouples plugin commands from event emissions. Provides a single source of truth. Allows event aggregation and debouncing before sending to frontend. | Slightly higher boilerplate in Rust. | **Best** for general UI status updates. |
| **Tauri v2 Channel** | Scoped, typed, and direct one-to-one streaming between command callers and UI. | Overkill for global broadcast updates. Higher runtime overhead for simple notifications. | Best for streaming logs or high-frequency stats. |

### 4.2 Visibility Observer Comparison

| Observer | Performance | Detection Accuracy | Background Leak Prevention |
| :--- | :--- | :--- | :--- |
| **`ResizeObserver`** (Current) | High performance; native browser implementation. | Detects size changes. Fails to detect if parent components are hidden (`display: none`) or window is minimized. | Partial; relies on width thresholding. |
| **`IntersectionObserver`** | High performance; native browser implementation. | Detects actual visibility on screen (occlusion, offscreen sliding, `display: none` of parents). | **Best**; ensures zero connection when elements are offscreen. |
| **Window Visibility Hook** | High performance. | Checks browser-level tab/window visibility only. Cannot check element-level containment. | Best for global page minimization. |

---

## 5. Code Quality & Architectural Consistency (R4)

### 5.1 Agreement Violation: Section 六 (Six)
We detected a direct violation of **Section 六 (Six)** of [clash_mini_agreements.md:2066-2076](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/clash_mini_agreements.md#L2066-L2076). 

The agreement states:
> "当设置面板关闭时（`drawerOpen === false` 且可见），连接管理自动切换为**低频静默监控模式**：断开 WebSocket，降级为每 3 秒发起单次轻量级 `getConnections` HTTP REST 轮询。在此模式下，为了节省 CPU，**严禁**执行任何连接列表的差异对比、排序及 Map 内存重构计算，直接提取 totals 计入状态，且保持连接明细列表为空数组。"

In the current implementation of [src/hooks/use-connection-data.ts:25-45](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-connection-data.ts#L25-L45), when `enabled` is `false` (meaning the settings drawer is closed), the WebSocket subscription is disabled (`isWsActive = false`). However, **no fallback REST polling is implemented**. The connection totals (`uploadTotal` and `downloadTotal`) simply freeze at their last known values instead of updating every 3 seconds, which violates Section 六.

### 5.2 Tauri Plugin Role Boundaries
The Tauri plugin `tauri-plugin-mihomo` should ideally act as a clean, stateless wrapper around the Mihomo core client library. Handling UI-specific notifications, debouncing, and profile change flows directly inside commands violates separation of concerns. A cleaner design would delegate command-induced event notifications to a central event bus or `Handle` manager inside the main application backend.

---

## 6. Concrete Code Improvement Diffs

This section provides the concrete git diffs to address the issues outlined in this audit report. These changes comply with the minimal-change principle and fully implement the authorized development agreements.

### a. Emit Refresh Event Only on Successful Speed Test
*Ensures `delay_proxy_by_name` only emits UI refresh events when the backend speed test succeeds.*

```diff
diff --git a/crates/tauri-plugin-mihomo/src/commands.rs b/crates/tauri-plugin-mihomo/src/commands.rs
--- a/crates/tauri-plugin-mihomo/src/commands.rs
+++ b/crates/tauri-plugin-mihomo/src/commands.rs
@@ -211,5 +211,7 @@
     let mihomo = state.read().await.clone();
     let res = mihomo.delay_proxy_by_name(&proxy_name, &test_url, timeout).await;
-    let _ = app.emit("verge://refresh-proxy-config", "yes");
+    if res.is_ok() {
+        let _ = app.emit("verge://refresh-proxy-config", "yes");
+    }
     res
 }
```

### b. Separate Throttle Timestamps in `app-data-provider.tsx`
*Prevents proxy refresh events from being dropped when they occur within 800ms of a profile change event.*

```diff
diff --git a/src/providers/app-data-provider.tsx b/src/providers/app-data-provider.tsx
--- a/src/providers/app-data-provider.tsx
+++ b/src/providers/app-data-provider.tsx
@@ -276,4 +276,5 @@
     let lastProfileId: string | null = null
-    let lastUpdateTime = 0
+    let lastProfileUpdateTime = 0
+    let lastProxyUpdateTime = 0
     const refreshThrottle = 800
 
     const handleProfileChanged = (event: { payload: string }) => {
       const newProfileId = event.payload
       const now = Date.now()
       if (
         lastProfileId === newProfileId &&
-        now - lastUpdateTime < refreshThrottle
+        now - lastProfileUpdateTime < refreshThrottle
       ) {
         return
       }
       lastProfileId = newProfileId
-      lastUpdateTime = now
+      lastProfileUpdateTime = now
       void queryClient.invalidateQueries({ queryKey: ['getProfiles'] })
       refreshRules().catch(() => console.warn('[app-data] refreshRules failed'))
       refreshRuleProviders().catch(() => console.warn('[app-data] refreshRuleProviders failed'))
     }
 
     const handleRefreshProxy = () => {
       const now = Date.now()
-      if (now - lastUpdateTime <= refreshThrottle) return
-      lastUpdateTime = now
+      if (now - lastProxyUpdateTime <= refreshThrottle) return
+      lastProxyUpdateTime = now
       refreshProxy().catch(() => console.warn('[app-data] refreshProxy failed'))
     }
```

### c. Resolve Double Listening of `"verge://refresh-clash-config"`
*Removes the duplicate `'verge://refresh-clash-config'` listener from `app-data-provider.tsx`, letting the query invalidation in `use-layout-events.ts` centrally manage the proxy refetch.*

```diff
diff --git a/src/providers/app-data-provider.tsx b/src/providers/app-data-provider.tsx
--- a/src/providers/app-data-provider.tsx
+++ b/src/providers/app-data-provider.tsx
@@ -273,3 +273,2 @@
     let unlistenProxy: (() => void) | null = null
-    let unlistenClash: (() => void) | null = null
 
@@ -332,13 +331,2 @@
 
-      try {
-        const uClash = await listen(
-          'verge://refresh-clash-config',
-          handleRefreshProxy,
-        )
-        if (!active) {
-          uClash()
-        } else {
-          unlistenClash = uClash
-        }
-      } catch (error) {
-        console.warn('[AppDataProvider] 设置 Clash 事件监听器失败:', error)
-      }
@@ -354,3 +341,2 @@
       unlistenProxy?.()
-      unlistenClash?.()
     }
```

### d. Implement REST Polling Fallback to Comply with Section 六
*Ensures that when the drawer is closed but the window remains visible, connection data reverts to low-frequency REST polling of totals every 3 seconds, keeping connection lists empty to conserve CPU resources.*

```diff
diff --git a/src/hooks/use-connection-data.ts b/src/hooks/use-connection-data.ts
--- a/src/hooks/use-connection-data.ts
+++ b/src/hooks/use-connection-data.ts
@@ -1,7 +1,7 @@
-import { useQueryClient } from '@tanstack/react-query'
+import { type useEffect } from 'react'
+import { useQueryClient } from '@tanstack/react-query'
 
 import { useVisibility } from '@/hooks/use-visibility'
 import type { IConnectionsItem } from '@/types/connection'
-import { MihomoWebSocket } from 'tauri-plugin-mihomo-api'
+import { MihomoWebSocket, getConnections } from 'tauri-plugin-mihomo-api'
 
 import { useMihomoWsSubscription } from './use-mihomo-ws-subscription'
@@ -213,2 +213,26 @@
     })
+
+  useEffect(() => {
+    if (isWsActive || !isVisible || !subscriptionCacheKey) return
+
+    let timer: ReturnType<typeof setInterval> | null = null
+
+    const poll = async () => {
+      try {
+        const res = await getConnections()
+        queryClient.setQueryData<ConnectionMonitorData>([subscriptionCacheKey], {
+          uploadTotal: res.uploadTotal,
+          downloadTotal: res.downloadTotal,
+          activeConnections: [],
+          closedConnections: [],
+        })
+      } catch (err) {
+        console.error('Failed to poll connections fallback:', err)
+      }
+    }
+
+    void poll()
+    timer = setInterval(poll, 3000)
+
+    return () => {
+      if (timer) clearInterval(timer)
+    }
+  }, [isWsActive, isVisible, subscriptionCacheKey, queryClient])
 
```

### e. Add Emit/Refresh Triggers for Single-Node Health Check and Config Reloads
*Ensures that single-node health checks, config reloads, and rule provider updates correctly notify the frontend.*

```diff
diff --git a/crates/tauri-plugin-mihomo/src/commands.rs b/crates/tauri-plugin-mihomo/src/commands.rs
--- a/crates/tauri-plugin-mihomo/src/commands.rs
+++ b/crates/tauri-plugin-mihomo/src/commands.rs
@@ -151,14 +151,19 @@
 
 #[command]
-pub(crate) async fn healthcheck_node_in_provider(
+pub(crate) async fn healthcheck_node_in_provider<R: Runtime>(
+    app: AppHandle<R>,
     state: State<'_, RwLock<Mihomo>>,
     provider_name: String,
     proxy_name: String,
     test_url: String,
     timeout: u32,
 ) -> Result<ProxyDelay> {
     let mihomo = state.read().await.clone();
-    mihomo
+    let res = mihomo
         .healthcheck_node_in_provider(&provider_name, &proxy_name, &test_url, timeout)
-        .await
+        .await;
+    if res.is_ok() {
+        let _ = app.emit("verge://refresh-proxy-config", "yes");
+    }
+    res
 }
@@ -230,6 +235,11 @@
 
 #[command]
-pub(crate) async fn update_rule_provider(state: State<'_, RwLock<Mihomo>>, provider_name: String) -> Result<()> {
+pub(crate) async fn update_rule_provider<R: Runtime>(
+    app: AppHandle<R>,
+    state: State<'_, RwLock<Mihomo>>,
+    provider_name: String,
+) -> Result<()> {
     let mihomo = state.read().await.clone();
-    mihomo.update_rule_provider(&provider_name).await
+    let res = mihomo.update_rule_provider(&provider_name).await;
+    let _ = app.emit("verge://refresh-clash-config", "yes");
+    res
 }
@@ -243,6 +253,11 @@
 
 #[command]
-pub(crate) async fn reload_config(state: State<'_, RwLock<Mihomo>>, force: bool, config_path: String) -> Result<()> {
+pub(crate) async fn reload_config<R: Runtime>(
+    app: AppHandle<R>,
+    state: State<'_, RwLock<Mihomo>>,
+    force: bool,
+    config_path: String,
+) -> Result<()> {
     let mihomo = state.read().await.clone();
-    mihomo.reload_config(force, &config_path).await
+    let res = mihomo.reload_config(force, &config_path).await;
+    let _ = app.emit("verge://refresh-clash-config", "yes");
+    res
 }
```
