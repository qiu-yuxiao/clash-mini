# Handoff Report - explorer_m1_2

This report provides the detailed findings, logic chain, and proposed strategy for conditional WebSocket subscriptions in Clash Mini based on window visibility.

---

## 1. Observation
I observed the following code structures and behaviors across the codebase:

### A. Core Subscription Behavior (`src/hooks/use-mihomo-ws-subscription.ts`)
- **Key Check in `useEffect` (lines 233-234)**:
  ```typescript
  useEffect(() => {
    if (!subscriptionCacheKey) return
  ```
- **Cleanup of Old Connections (lines 342-351)**:
  ```typescript
      entry.refs -= 1
      if (entry.refs <= 0) {
        entry.closed = true
        if (entry.reconnectTimer) {
          clearTimeout(entry.reconnectTimer)
          entry.reconnectTimer = null
        }
        sharedSubscriptions.delete(subscriptionCacheKey)
        void closeSharedSocket(entry)
      }
  ```

### B. Connection Hook (`src/hooks/use-connection-data.ts`)
- **Visibility Integration (lines 93-104)**:
  ```typescript
  export const useConnectionData = (options?: { enabled?: boolean }) => {
    const enabled = options?.enabled ?? true
    const isVisible = useVisibility()
    const queryClient = useQueryClient()
  
    const isWsActive = enabled && isVisible
  
    const { response, refresh, subscriptionCacheKey } =
      useMihomoWsSubscription<ConnectionMonitorData>({
        storageKey: 'mihomo_connection_date',
        buildSubscriptKey: (date) => (isWsActive ? `getClashConnection-${date}` : null),
  ```
- **Polling Suppression (lines 123-125)**:
  ```typescript
    useEffect(() => {
      if (isWsActive || !isVisible) return
  ```

### C. Traffic Hook (`src/hooks/use-traffic-data.ts`)
- **Current Setup (lines 28-36)**:
  ```typescript
  export const useTrafficData = (options?: { enabled?: boolean }) => {
    const enabled = options?.enabled ?? true
  
    const {
      graphData: { appendData },
    } = useTrafficMonitorEnhanced({ subscribe: false, enabled })
    const { response, refresh } = useMihomoWsSubscription<ITrafficItem>({
      storageKey: 'mihomo_traffic_date',
      buildSubscriptKey: (date) => (enabled ? `getClashTraffic-${date}` : null),
  ```

### D. Logs Hook (`src/hooks/use-log-data.ts`)
- **Current Setup (lines 50-62)**:
  ```typescript
  export const useLogData = () => {
    const queryClient = useQueryClient()
    const [clashLog] = useClashLog()
    const enableLog = clashLog?.enable ?? true
    const logLevel = clashLog?.logLevel ?? 'info'
    const allowedTypes = LOG_LEVEL_FILTERS[logLevel] ?? DEFAULT_LOG_TYPES
    const hasLoadedInitialLogsRef = useRef(false)
  
    const { response, refresh, subscriptionCacheKey } = useMihomoWsSubscription<
      ILogItem[]
    >({
      storageKey: 'mihomo_logs_date',
      buildSubscriptKey: (date) => (enableLog ? `getClashLog-${date}` : null),
  ```

---

## 2. Logic Chain
1. **Dynamic Disconnection Mechanism**: According to observations in `use-mihomo-ws-subscription.ts` (Section 1A), if `subscriptionCacheKey` evaluates to `null`, the `useEffect` returned cleanup runs and decrements reference count. If reference count reaches `0`, the WebSocket is disconnected and closed.
2. **Current Visibility Handling**:
   - `useConnectionData` (Section 1B) computes `isWsActive` using `isVisible`, and maps it to `null` if the page is invisible. It also stops polling.
   - `useTrafficData` (Section 1C) does not use `isVisible` to compute `enabled`.
   - `useLogData` (Section 1D) does not use `isVisible` to compute `enableLog`.
3. **Proposed Action**: By modifying `useTrafficData` and `useLogData` to query `useVisibility()` internally and merging it with their active/enabled options, we will ensure that when `isVisible === false`, their respective keys resolve to `null`.
4. **Outcome**: The subscription keys transition to `null`, automatically cleaning up and disconnecting the underlying WebSocket connections for both traffic and logs when the application window is hidden or minimized.

---

## 3. Caveats
- **Tauri Window Event Dependency**: This logic relies on `useVisibility()` correctly emitting `false` when the Tauri window is minimized or hidden. (This is being implemented in Milestone 1.1).
- **Log Gaps**: Disconnecting logs during minimization will lead to gaps in log history. Because logs have no unique identifier (`ILogItem` contains only `type`, `time`, and `payload`), deduplicating REST logs on reconnection is not feasible. The proposed strategy keeps cached logs and appends new ones on reconnect, which is standard and safe.

---

## 4. Conclusion
We conclude that `useConnectionData` is already correctly optimized and requires no modifications. To achieve visibility-based WebSocket lifecycle management for traffic and logs, we must import and integrate `useVisibility()` into `src/hooks/use-traffic-data.ts` and `src/hooks/use-log-data.ts` to conditionally return `null` keys.

---

## 5. Verification Method
To verify this strategy:
1. Apply the diffs specified in the `analysis.md` report.
2. Build and launch the application.
3. Open the developer console or monitor system TCP connections.
4. Open the logs page and traffic monitor page.
5. Minimize the application window or switch virtual desktops.
6. Verify that the WebSocket connections to Clash core for traffic (`/traffic`) and logs (`/logs`) are closed.
7. Restore/focus the application window.
8. Verify that the connections are successfully re-established.
