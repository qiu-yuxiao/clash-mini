# BUG-239 Codebase Audit Report

This report documents the codebase audit of Clash Mini (ClashVerge) regarding the BUG-239 code corrections and general compliance with the authorized project agreements.

---

## 1. Observations

### 1.1 Backend Proxy Commands and Event Emission
- In [crates/tauri-plugin-mihomo/src/commands.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs):
  - **`delay_proxy_by_name` (lines 205-216):**
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
  - **`healthcheck_node_in_provider` (lines 153-164):**
    ```rust
    #[command]
    pub(crate) async fn healthcheck_node_in_provider(
        state: State<'_, RwLock<Mihomo>>,
        provider_name: String,
        proxy_name: String,
        test_url: String,
        timeout: u32,
    ) -> Result<ProxyDelay> {
        let mihomo = state.read().await.clone();
        mihomo
            .healthcheck_node_in_provider(&provider_name, &proxy_name, &test_url, timeout)
            .await
    }
    ```
  - **`reload_config` (lines 245-248):**
    ```rust
    #[command]
    pub(crate) async fn reload_config(state: State<'_, RwLock<Mihomo>>, force: bool, config_path: String) -> Result<()> {
        let mihomo = state.read().await.clone();
        mihomo.reload_config(force, &config_path).await
    }
    ```
  - **`update_rule_provider` (lines 232-235):**
    ```rust
    #[command]
    pub(crate) async fn update_rule_provider(state: State<'_, RwLock<Mihomo>>, provider_name: String) -> Result<()> {
        let mihomo = state.read().await.clone();
        mihomo.update_rule_provider(&provider_name).await
    }
    ```

### 1.2 Frontend Refetch and Event Listening
- In [src/providers/app-data-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/app-data-provider.tsx):
  - **`getProxies` query (lines 190-199):**
    ```typescript
    const {
      data: proxiesData,
      isPending: isProxiesPending,
      refetch: _refetchProxy,
    } = useQuery({
      queryKey: ['getProxies'],
      queryFn: fetchProxies,
      refetchInterval: false,
      refetchIntervalInBackground: false,
      ...TQ_MIHOMO,
    })
    ```
  - **Event listeners & shared throttle (lines 277-301):**
    ```typescript
    let lastUpdateTime = 0
    const refreshThrottle = 800

    const handleProfileChanged = (event: { payload: string }) => {
      const newProfileId = event.payload
      const now = Date.now()
      if (
        lastProfileId === newProfileId &&
        now - lastUpdateTime < refreshThrottle
      ) {
        return
      }
      lastProfileId = newProfileId
      lastUpdateTime = now
      void queryClient.invalidateQueries({ queryKey: ['getProfiles'] })
      refreshRules().catch(() => console.warn('[app-data] refreshRules failed'))
      refreshRuleProviders().catch(() => console.warn('[app-data] refreshRuleProviders failed'))
    }

    const handleRefreshProxy = () => {
      const now = Date.now()
      if (now - lastUpdateTime <= refreshThrottle) return
      lastUpdateTime = now
      refreshProxy().catch(() => console.warn('[app-data] refreshProxy failed'))
    }
    ```

- In [src/pages/_layout/hooks/use-layout-events.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-layout-events.ts) (lines 46-53):
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

### 1.3 Connections Panel and ResizeObserver
- In [src/pages/_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx) (lines 1042-1065):
  ```typescript
  const [isPanelVisible, setIsPanelVisible] = useState(false)

  useEffect(() => {
    if (!drawerOpen) {
      setIsPanelVisible(false)
      return
    }
    const element = connectionsPanelRef.current
    if (!element) return

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setIsPanelVisible(entry.contentRect.width > 10)
      }
    })

    observer.observe(element)
    return () => {
      observer.disconnect()
    }
  }, [drawerOpen])

  const {
    response: { data: connectionsData },
    clearClosedConnections,
  } = useConnectionData({ enabled: drawerOpen && isPanelVisible })
  ```
- In [src/pages/_layout/components/connections-panel.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/connections-panel.tsx) (lines 48-52):
  ```typescript
  // WARNING [FOR AI AGENTS / AUDITORS]:
  // This minWidth must remain 0 and MUST NOT be set to a fixed minimum width.
  // Shrinking to 0px is the INTENDED behavior for 270px narrow width mode to hide
  // the panel by sliding/clipping it off-screen, per the design agreement.
  minWidth: 0,
  ```

- In [src/hooks/use-connection-data.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-connection-data.ts) (lines 25-45):
  ```typescript
  export const useConnectionData = (options?: { enabled?: boolean }) => {
    const enabled = options?.enabled ?? true
    const isVisible = useVisibility()
    const queryClient = useQueryClient()

    const isWsActive = enabled && isVisible

    const { response, refresh, subscriptionCacheKey } =
      useMihomoWsSubscription<ConnectionMonitorData>({
        storageKey: 'mihomo_connection_date',
        buildSubscriptKey: (date) =>
          isWsActive ? `getClashConnection-${date}` : null,
        buildCacheKey: (date) => `getClashConnection-${date}`,
        fallbackData: initConnData,
        connect: () => MihomoWebSocket.connect_connections(),
        ...
  ```

---

## 2. Logic Chain

### 2.1 Question 1: Missing Triggers / Missing Refreshes
1. **Observation 1.1:** `healthcheck_node_in_provider` changes the backend proxy provider delay metrics but does not trigger `app.emit("verge://refresh-proxy-config")`.
2. **Observation 1.1:** `reload_config` changes the entire proxy/group structure in the backend but does not emit `verge://refresh-proxy-config` or `verge://refresh-clash-config` directly (it relies on wrapping Rust callers to trigger events, which creates gaps if called directly).
3. **Observation 1.1:** `update_rule_provider` updates rule definitions but emits no events.
4. **Observation 1.1:** There is no "update all proxy providers" command. Frontend loops over `update_proxy_provider`, firing the emit multiple times, which collides with the throttle logic in the frontend.
5. **Conclusion:** Yes, there are missing triggers (e.g. `healthcheck_node_in_provider`, `update_rule_provider`, and direct `reload_config` invocations).

### 2.2 Question 2: Speed Test Fails & `delay_proxy_by_name`
1. **Observation 1.1:** Inside `delay_proxy_by_name`, `let res = mihomo.delay_proxy_by_name(...).await;` retrieves the delay results (which can be a failed `Result`).
2. **Observation 1.1:** On the subsequent line, `let _ = app.emit("verge://refresh-proxy-config", "yes");` is run unconditionally before the result `res` is returned.
3. **Conclusion:** Yes, `delay_proxy_by_name` emits the refresh event even when the speed test fails.

### 2.3 Question 3: Double Listening Issue
1. **Observation 1.2:** Both `app-data-provider.tsx` and `use-layout-events.ts` listen to `'verge://refresh-clash-config'`.
2. **Observation 1.2:** Upon receiving `'verge://refresh-clash-config'`, `app-data-provider.tsx` calls `refreshProxy()` which calls `_refetchProxy()`.
3. **Observation 1.2:** At the same time, `use-layout-events.ts` calls `queryClient.invalidateQueries({ queryKey: ['getProxies'] })`. In react-query, invalidating an observed query forces it to refetch.
4. **Conclusion:** Yes, this triggers two concurrent API queries to fetch proxies, leading to duplicate requests and race conditions.

### 2.4 Question 4: ResizeObserver Behavior during Transitions
1. **Observation 1.3:** During the sliding drawer opening transition, the panel width increases from 0px to 350px.
2. **Observation 1.3:** As soon as the width exceeds 10px, the `ResizeObserver` sets `isPanelVisible` to `true`, activating `isWsActive` in `useConnectionData`, which opens the WebSocket. This occurs *during* the slide transition before the panel settles.
3. **Observation 1.3:** During drawer closing, `drawerOpen` is immediately set to `false`. `isPanelVisible` instantly becomes `false`, disabling the WebSocket and tearing it down during the close animation.
4. **Conclusion:** The WebSocket drops immediately on closing (complying with the background-leak prevention policy) and reconnects as soon as width > 10px on opening. However, minor layout shifts/thrashing around 10px could cause rapid connect/disconnect cycles.

### 2.5 Question 5: Throttle Implementation & Event Loss
1. **Observation 1.2:** `lastUpdateTime` is defined at the `useEffect` closure level in `app-data-provider.tsx` and is **shared** by both `handleProfileChanged` and `handleRefreshProxy`.
2. **Observation 1.2:** A profile change updates `lastUpdateTime = Date.now()`.
3. **Observation 1.2:** A proxy event arriving within 800ms of a profile change will find `now - lastUpdateTime <= 800` to be `true` and will return immediately, completely discarding the proxy refresh.
4. **Conclusion:** Yes, the shared throttle timestamp causes proxy refresh events to be skipped/lost when they occur near profile changes.

### 2.6 Question 6: Architecture Comparison
1. **Current Direct Emit:** Simple, but tightly couples Rust commands to Tauri event emitter. Hard to trigger programmatically from Rust backend without `AppHandle`.
2. **Central Event Manager in Backend `Handle`:** Single source of truth. Decouples core command handling from UI layer. Allows debouncing events backend-side.
3. **Tauri v2 Channel:** Scoped and typed communication for one-to-one streams, but not ideal for broad global notifications.

### 2.7 Question 7: Observer Alternatives
1. **`ResizeObserver` (Current):** Detects collapsing/resizing, but doesn't check offscreen/display-none status or window minimization.
2. **`IntersectionObserver`:** Superior for screen visibility checks, detecting if the panel is off-screen or parent hidden.
3. **`useVisibility` (Focus):** Good for window-level checks, but not element-level.

### 2.8 Question 8: Compliance with `clash_mini_agreements.md`
1. **Observation 2.0:** Section VI of `clash_mini_agreements.md` requires that when `drawerOpen === false`, connection monitoring must automatically downgrade to low-frequency REST polling (every 3 seconds) for totals.
2. **Observation 1.3:** In `use-connection-data.ts`, when `enabled` is `false`, the WebSocket is disconnected, but no polling fallback is implemented. The connection data remains stale.
3. **Conclusion:** This is a direct violation of Section VI of theauthorized development agreements.

---

## 3. Caveats
- No actual runtime testing was performed (strictly read-only investigation, as required).
- Assumptions made that `@tanstack/react-query` and Tauri APIs behave per their standard specifications.

---

## 4. Conclusion
The BUG-239 corrections introduced proper event emission on proxy actions, but left issues such as:
1. **Double-fetching** of proxies due to duplicate listeners on `verge://refresh-clash-config`.
2. **Throttle collision / event loss** due to the shared `lastUpdateTime` timestamp.
3. **Missing triggers** for single-node tests and rule provider updates.
4. **Non-compliance** with Section VI of the development agreements (missing 3-second REST polling fallback).

---

## 5. Verification Method

### 5.1 Verification Commands
Verify the project compiles and tests pass using Cargo:
```powershell
cargo test -p tauri-plugin-mihomo
```

### 5.2 Files to Inspect
- Check [use-connection-data.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-connection-data.ts) lines 25-45 to verify no REST polling fallback is implemented.
- Check [app-data-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/app-data-provider.tsx) lines 277-301 to verify the shared `lastUpdateTime` variable.
- Check [commands.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/commands.rs) lines 205-216 to verify unconditional `emit` in `delay_proxy_by_name`.
