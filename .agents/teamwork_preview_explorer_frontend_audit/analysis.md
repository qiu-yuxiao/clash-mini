# Frontend Audit Report: Clash Mini modifications (d3831a0ce5ecc6b2..196e7c01)

## Executive Summary

Between release commit `d3831a0ce5ecc6b2c040368570773f2622d0b91b` and latest HEAD (`196e7c01`), a series of React, TypeScript, and CSS changes were made to optimize auto-node selection, button style skins, and delay measurements. 

This audit identified **8 specific findings** ranging from **major React Hook race conditions** and **profile synchronization leaks**, to **unhandled promise rejections**, **timer resource leaks**, and **low-contrast styling issues**.

---

## Detailed Findings & Proposed Fixes

### Finding 1: React Hook Dependency Safety — Permanent Cancellation of Profile Activation
* **File Path:** `src/pages/_layout.tsx`
* **Line Numbers:** 1114-1145
* **Root Cause:**
  The `useEffect` hook handling active profile enhancement/activation added `refreshProxy`, `setHeadStateForSort`, and `t` to its dependency array. 
  1. `setHeadStateForSort` (returned by `useHeadStateNew`) changes its reference whenever the active profile changes.
  2. The translation function `t` updates its reference whenever translation assets load or languages change.
  3. When `t` or `setHeadStateForSort` changes, the hook's cleanup function runs, setting `cancelled = true`. This aborts the active asynchronous `enhanceProfiles().then(...)` promise chain.
  4. However, `lastEnhancedProfileRef.current` was already synchronously updated to `currentProfileUid`.
  5. On the subsequent run, the condition `lastEnhancedProfileRef.current !== currentProfileUid` evaluates to `false`, causing the body of the `useEffect` to do nothing. As a result, profile activation and auto-selection are permanently aborted.
* **Proposed Code Diff Recommendation:**
```diff
<<<<
  // Automatically enhance profile when it is loaded or switched (flatten to single PROXY group)
  useEffect(() => {
    if (
      currentProfileUid &&
      lastEnhancedProfileRef.current !== currentProfileUid
    ) {
      lastEnhancedProfileRef.current = currentProfileUid
      const uid = currentProfileUid
      let cancelled = false
      enhanceProfiles()
        .then(async () => {
          if (cancelled || isImportingRef.current) return
          console.log(`[Layout] Enhanced active profile: ${uid}`)
          await activateSelectedRef.current()
          if (cancelled || isImportingRef.current) return
          // 等待 Clash 内核就绪（最多 20 秒），然后触发自动选点并刷新前端
          await waitForClashReady(t)
          if (cancelled || isImportingRef.current) return
          await triggerAutoSelectAndRefresh(refreshProxy, t, fallbackTimerRef, setHeadStateForSort)
        })
        .catch((err) => {
          if (cancelled) return
          console.error(
            `[Layout] Failed to enhance profile ${uid}:`,
            err,
          )
          lastEnhancedProfileRef.current = null
        })
      return () => {
        cancelled = true
      }
    }
  }, [currentProfileUid, refreshProxy, setHeadStateForSort, t])
====
  // Wrap callbacks in refs to keep them stable and avoid effect re-execution
  const refreshProxyRef = useRef(refreshProxy)
  const setHeadStateForSortRef = useRef(setHeadStateForSort)
  const tRef = useRef(t)

  useEffect(() => {
    refreshProxyRef.current = refreshProxy
    setHeadStateForSortRef.current = setHeadStateForSort
    tRef.current = t
  })

  // Automatically enhance profile when it is loaded or switched (flatten to single PROXY group)
  useEffect(() => {
    if (
      currentProfileUid &&
      lastEnhancedProfileRef.current !== currentProfileUid
    ) {
      lastEnhancedProfileRef.current = currentProfileUid
      const uid = currentProfileUid
      let cancelled = false
      enhanceProfiles()
        .then(async () => {
          if (cancelled || isImportingRef.current) return
          console.log(`[Layout] Enhanced active profile: ${uid}`)
          await activateSelectedRef.current()
          if (cancelled || isImportingRef.current) return
          // 等待 Clash 内核就绪（最多 10 秒），并支持取消
          await waitForClashReady(tRef.current, () => cancelled)
          if (cancelled || isImportingRef.current) return
          await triggerAutoSelectAndRefresh(
            refreshProxyRef.current,
            tRef.current,
            fallbackTimerRef,
            setHeadStateForSortRef.current
          )
        })
        .catch((err) => {
          if (cancelled) return
          console.error(
            `[Layout] Failed to enhance profile ${uid}:`,
            err,
          )
          lastEnhancedProfileRef.current = null
        })
      return () => {
        cancelled = true
        if (fallbackTimerRef.current) {
          clearTimeout(fallbackTimerRef.current)
          fallbackTimerRef.current = null
        }
      }
    }
  }, [currentProfileUid])
>>>>
```

---

### Finding 2: Profile Switch State Race Condition & Background Timer Leak
* **File Path:** `src/pages/_layout.tsx`
* **Line Numbers:** 220-305
* **Root Cause:**
  `frontendAutoSelect` schedules a polling `setInterval` loop to check for healthy nodes. If the user switches profiles rapidly, the `useEffect` cleans up `cancelled = true` for the active promise, but the `activeAutoSelectTimer` interval is **never cleared** in the background. It will continue to tick, and once a node delay matches, it will call `selectNodeForGroup` and `refreshProxy`, which overwrites the active proxy selection of the *newly activated* profile.
* **Proposed Code Diff Recommendation:**
```diff
<<<<
async function frontendAutoSelect(
  groupName: string,
  refreshProxy: (opts?: { forceFull?: boolean }) => Promise<any>,
  timeout = 10000,
  concurrency = 36,
): Promise<[string, number][]> {
====
async function frontendAutoSelect(
  groupName: string,
  refreshProxy: (opts?: { forceFull?: boolean }) => Promise<any>,
  timeout = 10000,
  concurrency = 36,
  isCancelled: () => boolean = () => false,
): Promise<[string, number][]> {
>>>>
```
And check `isCancelled` on each tick inside the interval callback:
```diff
<<<<
  return new Promise<[string, number][]>((resolve) => {
    activeAutoSelectTimer = setInterval(async () => {
      // 收集当前已测出的健康节点并统计已测试数量
====
  return new Promise<[string, number][]>((resolve) => {
    activeAutoSelectTimer = setInterval(async () => {
      if (isCancelled()) {
        if (activeAutoSelectTimer) {
          clearInterval(activeAutoSelectTimer)
          activeAutoSelectTimer = null
        }
        resolve([])
        return
      }
      // 收集当前已测出的健康节点并统计已测试数量
>>>>
```

---

### Finding 3: Double Selection Request Race Condition
* **File Path:** `src/pages/_layout.tsx`
* **Line Numbers:** 247-295
* **Root Cause:**
  Inside the `setInterval` polling loop, if `isFinalSelection` is met on the very first tick (e.g., when 5 healthy nodes are found in the first 200ms), `hasSelectedTemp` is false.
  1. The "临时闪连" block executes: calling `selectNodeForGroup` and `refreshProxy` asynchronously.
  2. The code immediately continues to the "极速终选" block since `isFinalSelection` is true. It clears the timer and calls `selectNodeForGroup` and `refreshProxy` asynchronously again.
  This triggers two overlapping backend requests in parallel, resulting in a state race condition.
* **Proposed Code Diff Recommendation:**
  Ensure the temporary connection only triggers if it is NOT the final selection, or track the selected node to skip redundant calls:
```diff
<<<<
      // 临时闪连：一旦检测到第 1 个健康可用节点，立即尝试切换以闪连网络
      if (!hasSelectedTemp && healthyNodes.length >= 1) {
        hasSelectedTemp = true
        const tempTarget = healthyNodes[0].name
        console.log(`[Layout] 自动选点触发临时闪连: ${tempTarget} (${healthyNodes[0].delay}ms)`)
        try {
          await selectNodeForGroup(groupName, tempTarget)
          await refreshProxy({ forceFull: true })
        } catch (err) {
          console.error('[Layout] 临时闪连切换失败:', err)
        }
      }
====
      let selectedNode = '';
      // 临时闪连：仅在非终选状态下，一旦检测到第 1 个健康可用节点，立即尝试切换以闪连网络
      if (!isFinalSelection && !hasSelectedTemp && healthyNodes.length >= 1) {
        hasSelectedTemp = true
        selectedNode = healthyNodes[0].name
        console.log(`[Layout] 自动选点触发临时闪连: ${selectedNode} (${healthyNodes[0].delay}ms)`)
        try {
          await selectNodeForGroup(groupName, selectedNode)
          await refreshProxy({ forceFull: true })
        } catch (err) {
          console.error('[Layout] 临时闪连切换失败:', err)
        }
      }
>>>>
```
And check if we already selected it in the final selection:
```diff
<<<<
        if (healthyNodes.length >= 1) {
          const targetNode = healthyNodes[0].name
          const targetDelay = healthyNodes[0].delay
          console.log(`[Layout] 自动选点触发极速终选: ${targetNode} (${targetDelay}ms)`)
          try {
            await selectNodeForGroup(groupName, targetNode)
            await refreshProxy({ forceFull: true })
          } catch (err) {
            console.error('[Layout] 极速终选切换失败:', err)
          }
        }
====
        if (healthyNodes.length >= 1) {
          const targetNode = healthyNodes[0].name
          const targetDelay = healthyNodes[0].delay
          if (targetNode !== selectedNode) {
            console.log(`[Layout] 自动选点触发极速终选: ${targetNode} (${targetDelay}ms)`)
            try {
              await selectNodeForGroup(groupName, targetNode)
              await refreshProxy({ forceFull: true })
            } catch (err) {
              console.error('[Layout] 极速终选切换失败:', err)
            }
          } else {
            console.log(`[Layout] 终选节点与闪连节点相同: ${targetNode}，无需重复切换`)
          }
        }
>>>>
```

---

### Finding 4: Memory Leak of Unresolved Promises
* **File Path:** `src/pages/_layout.tsx`
* **Line Numbers:** 226-231
* **Root Cause:**
  When `frontendAutoSelect` is called, it returns a Promise. If a subsequent auto-select is triggered, the previous timer is cleared (`clearInterval(activeAutoSelectTimer)`), but the Promise of the first invocation is left **unresolved** in memory forever.
* **Proposed Code Diff Recommendation:**
  Store the previous Promise's `reject` callback and reject it with a cancellation error when a new auto-select is triggered:
```diff
<<<<
let activeAutoSelectTimer: any = null

async function frontendAutoSelect(
  groupName: string,
  refreshProxy: (opts?: { forceFull?: boolean }) => Promise<any>,
  timeout = 10000,
  concurrency = 36,
): Promise<[string, number][]> {
  const proxyGroup = await getProxyByName(groupName)
  const allNames = (proxyGroup?.all || []).filter(
    (name: string) => !isDummyName(name),
  )
  if (allNames.length === 0) return []

  if (activeAutoSelectTimer) {
    clearInterval(activeAutoSelectTimer)
    activeAutoSelectTimer = null
  }
====
let activeAutoSelectTimer: any = null
let activeAutoSelectReject: ((err: any) => void) | null = null

async function frontendAutoSelect(
  groupName: string,
  refreshProxy: (opts?: { forceFull?: boolean }) => Promise<any>,
  timeout = 10000,
  concurrency = 36,
  isCancelled: () => boolean = () => false,
): Promise<[string, number][]> {
  const proxyGroup = await getProxyByName(groupName)
  const allNames = (proxyGroup?.all || []).filter(
    (name: string) => !isDummyName(name),
  )
  if (allNames.length === 0) return []

  if (activeAutoSelectTimer) {
    clearInterval(activeAutoSelectTimer)
    activeAutoSelectTimer = null
  }
  if (activeAutoSelectReject) {
    activeAutoSelectReject(new Error('Cancelled by new auto-select invocation'))
    activeAutoSelectReject = null
  }
>>>>
```
And capture the reject callback:
```diff
<<<<
  return new Promise<[string, number][]>((resolve) => {
    activeAutoSelectTimer = setInterval(async () => {
====
  return new Promise<[string, number][]>((resolve, reject) => {
    activeAutoSelectReject = reject
    activeAutoSelectTimer = setInterval(async () => {
>>>>
```

---

### Finding 5: Unhandled Promise Rejection in `checkDelay`
* **File Path:** `src/services/delay.ts`
* **Line Numbers:** 225-228
* **Root Cause:**
  `Promise.race` resolves immediately when `timeoutPromise` wins. However, the losing promise `delayProxyByName(name, url, timeout)` is left running in the background. If it eventually fails/rejects, its rejection is unhandled, triggering a global `UnhandledPromiseRejection` crash/error event.
* **Proposed Code Diff Recommendation:**
  Attach a `.catch()` block directly to the API call within `Promise.race` so that background failures are swallowed or resolved safely:
```diff
<<<<
      // 使用Promise.race来实现超时控制
      const result = await Promise.race([
        delayProxyByName(name, url, timeout),
        timeoutPromise,
      ])
====
      // 使用Promise.race来实现超时控制，添加 catch 避免 unhandled rejection
      const result = await Promise.race([
        delayProxyByName(name, url, timeout).catch((err) => {
          console.error(`[DelayManager] 后台测速接口异常: ${name}`, err)
          return { delay: 1e6 }
        }),
        timeoutPromise,
      ])
>>>>
```

---

### Finding 6: Timeout Resource (Timer Handle) Leak
* **File Path:** `src/services/delay.ts`
* **Line Numbers:** 220-222
* **Root Cause:**
  The `setTimeout` scheduled inside `timeoutPromise` is never cleared if the API call completes before the timeout. This wastes timer handles.
* **Proposed Code Diff Recommendation:**
```diff
<<<<
      // 设置超时处理, delay = 0 为超时
      const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
        setTimeout(() => resolve({ delay: 0 }), timeout)
      })

      // 使用Promise.race来实现超时控制
      const result = await Promise.race([
        delayProxyByName(name, url, timeout),
        timeoutPromise,
      ])
====
      let timerId: any = null
      // 设置超时处理, delay = 0 为超时
      const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
        timerId = setTimeout(() => resolve({ delay: 0 }), timeout)
      })

      // 使用Promise.race来实现超时控制，并进行异常捕捉与定时器释放
      const result = await Promise.race([
        delayProxyByName(name, url, timeout).catch((err) => {
          console.error(`[DelayManager] 后台测速接口异常: ${name}`, err)
          return { delay: 1e6 }
        }),
        timeoutPromise,
      ])
      if (timerId) clearTimeout(timerId)
>>>>
```

---

### Finding 7: Contrast and Invisible Borders Styling Issue
* **File Path:** `src/utils/button-styles.ts`
* **Line Numbers:** 324-332
* **Root Cause:**
  In the Frosted Glass (`frosted-glass`) skin, the background and border for the disabled button state are hardcoded to white transparency (`rgba(255, 255, 255, 0.03)` / `0.05`). 
  Under light mode (where the page background is white/light grey), these boundaries become completely invisible. The disabled text (`rgba(0, 0, 0, 0.26)`) floats in space with no outline and a sub-2.6:1 contrast ratio, breaking accessibility standards.
* **Proposed Code Diff Recommendation:**
```diff
<<<<
      '&.Mui-disabled': {
        background: 'rgba(255, 255, 255, 0.03) !important',
        borderColor: 'rgba(255, 255, 255, 0.05) !important',
        color: isLight
          ? 'rgba(0, 0, 0, 0.26) !important'
          : 'rgba(255, 255, 255, 0.25) !important',
        boxShadow: 'none !important',
        backdropFilter: 'none !important',
        transform: 'none !important',
      },
====
      '&.Mui-disabled': {
        background: isLight 
          ? 'rgba(0, 0, 0, 0.03) !important' 
          : 'rgba(255, 255, 255, 0.03) !important',
        borderColor: isLight 
          ? 'rgba(0, 0, 0, 0.05) !important' 
          : 'rgba(255, 255, 255, 0.05) !important',
        color: isLight
          ? 'rgba(0, 0, 0, 0.26) !important'
          : 'rgba(255, 255, 255, 0.25) !important',
        boxShadow: 'none !important',
        backdropFilter: 'none !important',
        transform: 'none !important',
      },
>>>>
```

---

### Finding 8: React `useMemo` Dependency Array Omits theme and skin
* **File Path:** `src/pages/_layout.tsx`
* **Line Numbers:** 1478-1532
* **Root Cause:**
  The `useMemo` returning the titlebar JSX block removed `theme` and `controlSkin` from its dependency array. 
  1. The titlebar contains buttons styled via `get3DButtonStyle(theme, ...)` which depends on the active skin (`controlSkin`) and theme mode.
  2. If these variables are omitted, changing them will not cause React to rebuild the titlebar element tree, potentially leading to stale styles or layout glitches when switching skins/themes.
* **Proposed Code Diff Recommendation:**
  Add `theme` and `controlSkin` back into the dependency array:
```diff
<<<<
    [
      decorated,
      isDecorationsHidden,
      drawerOpen,
      patchVerge,
      verge?.enable_always_on_top,
    ],
====
    [
      decorated,
      isDecorationsHidden,
      drawerOpen,
      patchVerge,
      verge?.enable_always_on_top,
      theme,
      controlSkin,
    ],
>>>>
```

---

## Handoff Report Verification & Caveats

* **Caveats:**
  1. The audit scope was strictly limited to the four requested files: `src/pages/_layout.tsx`, `src/services/delay.ts`, `src/utils/button-styles.ts`, and `crates/tauri-plugin-mihomo/guest-js/index.ts`. Other project files were not analyzed.
  2. No code changes have been applied to the workspace.
* **Verification Method:**
  Independent verification of these fixes can be performed by:
  1. Applying the recommended patches.
  2. Testing profile switching rapidly (10+ clicks per second) to confirm that the active profile correctly loads, and no background timers overwrite the active proxy configuration.
  3. Inducing artificial network failures or 100% timeouts to verify that `DelayManager.checkDelay` no longer outputs unhandled promise rejections to the DevTools console.
  4. Toggling the app theme to Light mode with the Frosted Glass skin activated, inspecting disabled buttons to confirm borders/frames are clearly visible.
