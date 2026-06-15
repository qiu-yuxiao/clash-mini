# Handoff Report — Audit of Tauri IPC and React State Hooks

## 1. Observation

- **Observation 1: Dead Code useEffect in `use-connection-data.ts`**
  - **File Path**: `src/hooks/use-connection-data.ts`
  - **Lines**: 123-125
  - **Verbatim Code**:
    ```typescript
    useEffect(() => {
      if (isWsActive || !isVisible || !enabled) return
    ```
  - **Context**: `isWsActive` is defined on line 98 as:
    ```typescript
    const isWsActive = enabled && isVisible
    ```

- **Observation 2: Uncleared Timer Leak in `delay.ts`**
  - **File Path**: `src/services/delay.ts`
  - **Lines**: 220-228
  - **Verbatim Code**:
    ```typescript
    // 设置超时处理, delay = 0 为超时
    const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
      setTimeout(() => resolve({ delay: 0 }), timeout)
    })

    // 使用Promise.race来实现超时控制
    const result = await Promise.race([
      delayProxyByName(name, url, timeout),
      timeoutPromise,
    ])
    ```

- **Observation 3: Hook/Component Visibility Control**
  - **File Path**: `src/hooks/use-visibility.ts` (Lines 93-94) returning `documentVisible && !isMinimized`.
  - **File Path**: `src/hooks/use-traffic-monitor.ts` (Lines 343-380) checking `isActive = enabled && isVisible` to start/stop the data sampler worker client.
  - **File Path**: `src/components/home/enhanced-canvas-traffic-graph.tsx` (Lines 1006-1023) checking window blur and document visibility to skip canvas drawing.

- **Observation 4: WebSocket and React State Connection**
  - **File Path**: `src/hooks/use-mihomo-ws-subscription.ts` (Lines 250-257) writing updates directly to the React Query cache using `queryClient.setQueryData<T>([subscriptionCacheKey], resolved)`.

---

## 2. Logic Chain

1. **Dead Polling Fallback**:
   - `isWsActive` is defined as `enabled && isVisible` (Observation 1).
   - The condition `isWsActive || !isVisible || !enabled` can be rewritten as `(enabled && isVisible) || !isVisible || !enabled`.
   - If `enabled` is `true` and `isVisible` is `true`, `isWsActive` is `true`, so the function returns early.
   - If `enabled` is `false`, `!enabled` is `true`, so the function returns early.
   - If `isVisible` is `false`, `!isVisible` is `true`, so the function returns early.
   - Therefore, the early return matches all possible combinations, making the inner body (polling totals every 3 seconds) dead code that is never reached.

2. **Timer Leakage**:
   - `setTimeout` is scheduled to resolve after `timeout` milliseconds (Observation 2).
   - If the network call (`delayProxyByName`) resolves or rejects before the timeout (e.g., in 50ms), `Promise.race` immediately resolves.
   - The scheduled timer `setTimeout` is never cleared via `clearTimeout`.
   - The event loop retains the timer until it expires (up to 10 seconds), resulting in temporary resource leakage during batch tests.

3. **IPC to State and Render updates**:
   - Real-time websocket data updates the React Query cache via `queryClient.setQueryData` (Observation 4).
   - The React hook `useQuery` listens to the cache key, triggering a complete re-render of components using the hook (e.g., `EnhancedCanvasTrafficGraph`).
   - Although the traffic graph uses `requestAnimationFrame` for actual rendering, the React component tree is still forced to re-render, creating virtual DOM diffing overhead during high-frequency streaming.

---

## 3. Caveats

- We assumed that the low-frequency polling fallback in `useConnectionData` is intended to run if the WebSocket is disconnected or errors out (rather than just checking if it is *supposed* to be active). If the polling fallback is actually retired and no longer desired, the dead code block can simply be deleted.
- We did not run dynamic runtime memory profiling (e.g., heap dump analysis) because we are in a read-only code exploration environment; the audit is strictly static code analysis.

---

## 4. Conclusion

The TS frontend establishes high-frequency subscriptions via a shared WebSocket abstraction (`useMihomoWsSubscription`) backed by React Query cache updates. While it correctly terminates WebSocket connections when the window is hidden or minimized (via `useVisibility`), it contains two significant inefficiencies:
1. An unreachable fallback polling effect in `useConnectionData`.
2. A temporary timer leakage for every successful node delay test in `DelayManager`.
Furthermore, UI performance can be significantly optimized by adopting Ref-driven Canvas Rendering (bypassing React re-renders) and increasing/dynamicizing subscription update throttling.

---

## 5. Verification Method

To verify the logic and findings:
1. **Verification of Dead Code**: Open `src/hooks/use-connection-data.ts`, inspect line 124, and evaluate the truth table for `isWsActive || !isVisible || !enabled`. Note that it covers the entire boolean space of `(isVisible, enabled)`.
2. **Verification of Timer Leak**: Open `src/services/delay.ts` and inspect lines 220-228. Verify that no `clearTimeout` is called on the timeout timer created within the `timeoutPromise`.
3. **Execution of Tests**: Run `npm run test` or the project-specific unit test suite to ensure any subsequent refactoring of these files does not break existing test cases.
