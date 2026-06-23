# Frontend Resource Optimization Audit Report

This report presents findings from a read-only audit of the React/TypeScript frontend (located in the `src` directory) of Clash Verge for system resource optimizations (CPU, memory, re-renders, visibility throttling, and event listener cleanup).

---

## 1. Observation

### Observation 1.1: Ineffective Memoization in `ConnectionTable`
- **File Path**: `src/components/connection/connection-table.tsx`
- **Line Range**: 185–191
- **Verbatim Code**:
  ```typescript
    (prev, next) =>
      prev.row === next.row &&
      prev.virtualStart === next.virtualStart &&
      prev.virtualSize === next.virtualSize &&
      prev.onShowDetail === next.onShowDetail &&
      prev.onContextMenu === next.onContextMenu,
  )
  ```
- **Root Cause**: `row` is a TanStack Table `Row` wrapper instance created by `useReactTable` inside `ConnectionTable`. Since `useReactTable` returns new row wrappers on every render of the parent component, `prev.row === next.row` is always `false`. This breaks memoization completely, causing all rows in the connection table to re-render on every state change/tick (which happens every 1 second when active).

### Observation 1.2: Missing Memoization in `LogItem`
- **File Path**: `src/components/log/log-item.tsx`
- **Line Range**: 51–122
- **Verbatim Code**:
  ```typescript
  const LogItem = ({ value, searchState }: Props) => {
    const renderHighlightText = (text: string) => {
      if (!searchState?.text.trim()) return text
      // Regular expression compilation and match logic
      ...
    }
    return (
      <Item>
        <div>
          <span className="time">{renderHighlightText(value.time || '')}</span>
          <span className="type" data-type={(value.type || '').toLowerCase()}>
            {renderHighlightText(value.type || '')}
          </span>
        </div>
        <div>
          <span className="data">{renderHighlightText(value.payload)}</span>
        </div>
      </Item>
    )
  }
  ```
- **Root Cause**: `LogItem` is rendered within a virtualized list, but is not wrapped in `React.memo`. When log entries stream in via WebSockets, the parent updates the list array reference, triggering a full re-render of all visible items. For every render of `LogItem`, `renderHighlightText` is called three times, which compiles a regular expression and executes regex loops.

### Observation 1.3: Redundant Global Resize Listeners via `useWindowWidth` in `ProxyItem`
- **File Path**: `src/components/proxy/use-window-width.ts`
- **Line Range**: 13–31
- **Verbatim Code**:
  ```typescript
  export const useWindowWidth = () => {
    const [isMinimal, setIsMinimal] = useState(getIsMinimal)

    useEffect(() => {
      const handleResize = () => {
        const minimal = getIsMinimal()
        setIsMinimal((prev) => {
          if (prev !== minimal) {
            return minimal
          }
          return prev
        })
      }

      window.addEventListener('resize', handleResize)
      return () => {
        window.removeEventListener('resize', handleResize)
      }
    }, [])
    ...
  }
  ```
- **Root Cause**: `ProxyItem` (`src/components/proxy/proxy-item.tsx:81`) calls `useWindowWidth()`. Since `ProxyItem` is rendered for every visible node in a proxy group grid (e.g. up to 30+ visible nodes simultaneously), multiple identical `resize` event listeners are registered on the global `window` object. This wastes memory and degrades resize performance.

### Observation 1.4: Redundant Configuration Queries in `useProxyDelayState`
- **File Path**: `src/hooks/use-proxy-delay-state.ts`
- **Line Range**: 34–35
- **Verbatim Code**:
  ```typescript
    const { verge } = useVerge()
    const timeout = verge?.default_latency_timeout || 10000
  ```
- **Root Cause**: `useProxyDelayState` is invoked inside every single `ProxyItem`. The call to `useVerge()` internally hooks into TanStack Query (`useQuery`), running multiple parallel config state lookups. This introduces unnecessary query subscription overhead for every visible node.

### Observation 1.5: Unchecked Background Polling in `useSystemState`
- **File Path**: `src/hooks/use-system-state.ts`
- **Line Range**: 38–53
- **Verbatim Code**:
  ```typescript
    const {
      data: systemState = defaultSystemState,
      refetch: mutateSystemState,
      isLoading,
    } = useQuery({
      queryKey: ['getSystemState'],
      queryFn: async () => {
        const [runningMode, isAdminMode, isServiceOk] = await Promise.all([
          getRunningMode(),
          isAdmin(),
          isServiceAvailable(),
        ])
        return { runningMode, isAdminMode, isServiceOk } as SystemState
      },
      refetchInterval: isStartingUp ? 2000 : 30000,
    })
  ```
- **Root Cause**: The polling query `getSystemState` runs continuously in the background (every 30s, or 2s during startup) even when the window/page is hidden, minimized, or inactive. Unlike connection/traffic monitors, it does not leverage `useVisibility()` to pause background polling.

### Observation 1.6: Stale Cache / Missing Dependency in `useFilterSort`
- **File Path**: `src/components/proxy/use-filter-sort.ts`
- **Line Range**: 48–64
- **Verbatim Code**:
  ```typescript
    const compute = useMemo(() => {
      const fp = filterProxies(proxies, groupName, filterText, searchState)
      const sp = sortProxies(
        fp,
        groupName,
        sortType,
        verge?.default_latency_timeout,
      )
      return sp
    }, [
      proxies,
      groupName,
      filterText,
      sortType,
      searchState,
      verge?.default_latency_timeout,
    ])
  ```
- **Root Cause**: The `useMemo` dependency array misses the state variable `_` generated by the `bumpRefresh` reducer (`src/components/proxy/use-filter-sort.ts:25`), which is triggered when `delayManager` announces a delay update. Consequently, when background delay measurements complete, the list does not re-sort or re-filter dynamically in real-time.

### Observation 1.7: Leaked Timeout Timers in `UnlockPage`
- **File Path**: `src/pages/unlock.tsx`
- **Line Range**: 215–230
- **Verbatim Code**:
  ```typescript
    const invokeWithTimeout = async <T,>(
      cmd: string,
      args?: any,
      timeout = 15000,
    ): Promise<T> => {
      return Promise.race([
        invoke<T>(cmd, args),
        new Promise<T>((_, reject) =>
          setTimeout(
            () =>
              reject(new Error(t('tests.unlock.page.messages.detectionTimeout'))),
            timeout,
          ),
        ),
      ])
    }
  ```
- **Root Cause**: The `Promise.race` resolves immediately if the `invoke` completes first, but the timeout promise's `setTimeout` remains scheduled in the browser's event loop for the full 15 seconds. If many single tests are run, it leaks multiple un-cleared timeouts holding references to translation helpers and reject handlers.

---

## 2. Logic Chain

1. **Rendering Performance**:
   - Because `prev.row === next.row` is checking wrapper reference equality and `row` wrappers are reconstructed by `useReactTable` on every parent render, the custom comparison function in `RowComponent` evaluates to `false` on every state update, completely bypassing React memoization.
   - For `LogItem`, since the log stream continuously appends elements, the parent state changes frequently. Unmemoized `LogItem`s will re-render, executing regex compilation and parsing three times per item, leading to excessive CPU usage during log spikes.
2. **Listener Overhead**:
   - Every `ProxyItem` calling `useWindowWidth` binds a global `resize` event listener. When a grid displays 30 nodes, 30 duplicate window listeners are bound. These listeners compete on resize events, degrading browser responsiveness.
3. **Redundant Queries**:
   - 30 duplicate instances of `useVerge` in `useProxyDelayState` inside `ProxyItem` create 30 concurrent query subscriptions to TanStack Query's cache. Hoisting this timeout parameter avoids this overhead.
4. **Visibility & Backgrounding**:
   - In `useSystemState`, polling the backend for service status when the app is minimized is waste of resources. Integrating `useVisibility` preserves system resources by suspending queries when not in view.
5. **Caching & State Updates**:
   - The missing `_` dependency in `useFilterSort`'s `useMemo` means the list doesn't recalculate when delay measurements finish. Since the `proxies` array reference doesn't change when delay is set, the memoized value remains stale.
6. **Memory Leaks**:
   - Uncleared timers in `invokeWithTimeout` stay active for the entire duration of the timeout even after success. Clearing them in a `finally` block prevents background timer accumulation.

---

## 3. Caveats

- **Network Constraints**: The audit was performed entirely read-only using static analysis. Real-time profiling via Chrome DevTools (Performance/Memory panel) was not executed, so raw performance metrics (in milliseconds or bytes) are estimated based on typical React runtime characteristics.
- **Tauri IPC Overhead**: `getRunningMode` and related APIs in Tauri are relatively lightweight but still involve asynchronous Rust-to-JS serialization.

---

## 4. Conclusion

The audit identifies several opportunities to decrease CPU and memory consumption. By implementing the suggested code changes:
- CPU usage during logging and connection updates will decrease thanks to proper component memoization and regex caching.
- Resize listener overhead will drop from $O(N)$ (where $N$ is the number of visible proxy nodes) to $O(1)$.
- Background CPU wakeups will decrease by throttling `getSystemState` queries when the app is hidden.
- Real-time delay sorting will function correctly without waiting for full data refetches.

### Suggested Code Diffs

#### Fix 1: Memoization in `ConnectionTable` (`src/components/connection/connection-table.tsx`)
```diff
<<<<
  (prev, next) =>
    prev.row === next.row &&
    prev.virtualStart === next.virtualStart &&
    prev.virtualSize === next.virtualSize &&
    prev.onShowDetail === next.onShowDetail &&
    prev.onContextMenu === next.onContextMenu,
)
====
  (prev, next) =>
    prev.row.original === next.row.original &&
    prev.row.index === next.row.index &&
    prev.virtualStart === next.virtualStart &&
    prev.virtualSize === next.virtualSize &&
    prev.onShowDetail === next.onShowDetail &&
    prev.onContextMenu === next.onContextMenu,
)
>>>>
```

#### Fix 2: Memoization in `LogItem` (`src/components/log/log-item.tsx`)
```diff
<<<<
const LogItem = ({ value, searchState }: Props) => {
  const renderHighlightText = (text: string) => {
  ...
  return (
    <Item>
      ...
    </Item>
  )
}

export default LogItem
====
import { memo } from 'react'

const LogItem = memo(({ value, searchState }: Props) => {
  const renderHighlightText = (text: string) => {
  ...
  return (
    <Item>
      ...
    </Item>
  )
}, (prev, next) => {
  return prev.value === next.value && prev.searchState === next.searchState
})

export default LogItem
>>>>
```

#### Fix 3: Throttling Polling in `useSystemState` (`src/hooks/use-system-state.ts`)
```diff
<<<<
export function useSystemState() {
  const { verge, patchVerge } = useVerge()
  const disablingTunRef = useRef(false)
  const [isStartingUp, setIsStartingUp] = useState(true)

  useEffect(() => {
    const timer = setTimeout(() => setIsStartingUp(false), STARTUP_GRACE_MS)
    return () => clearTimeout(timer)
  }, [])

  const {
    data: systemState = defaultSystemState,
    refetch: mutateSystemState,
    isLoading,
  } = useQuery({
    queryKey: ['getSystemState'],
    queryFn: async () => {
      const [runningMode, isAdminMode, isServiceOk] = await Promise.all([
        getRunningMode(),
        isAdmin(),
        isServiceAvailable(),
      ])
      return { runningMode, isAdminMode, isServiceOk } as SystemState
    },
    refetchInterval: isStartingUp ? 2000 : 30000,
  })
====
import { useVisibility } from './use-visibility'

export function useSystemState() {
  const { verge, patchVerge } = useVerge()
  const disablingTunRef = useRef(false)
  const [isStartingUp, setIsStartingUp] = useState(true)
  const isVisible = useVisibility()

  useEffect(() => {
    const timer = setTimeout(() => setIsStartingUp(false), STARTUP_GRACE_MS)
    return () => clearTimeout(timer)
  }, [])

  const {
    data: systemState = defaultSystemState,
    refetch: mutateSystemState,
    isLoading,
  } = useQuery({
    queryKey: ['getSystemState'],
    queryFn: async () => {
      const [runningMode, isAdminMode, isServiceOk] = await Promise.all([
        getRunningMode(),
        isAdmin(),
        isServiceAvailable(),
      ])
      return { runningMode, isAdminMode, isServiceOk } as SystemState
    },
    refetchInterval: isVisible ? (isStartingUp ? 2000 : 30000) : false,
  })
>>>>
```

#### Fix 4: Hoisting `useWindowWidth` and `useVerge` in `ProxyItem`
Instead of having `useWindowWidth` and `useVerge` (via `useProxyDelayState`) called inside `ProxyItem`, pass `isMinimal` and `timeout` as props:
```typescript
// src/components/proxy/proxy-item.tsx
interface Props {
  group: IProxyGroupItem
  proxy: IProxyItem
  selected: boolean
  isMinimal: boolean // Hoisted prop
  timeout: number    // Hoisted prop
  showType?: boolean
  indexInGroup?: number
  sx?: SxProps<Theme>
  onClick?: (name: string) => void
}
```
And inside `useProxyDelayState`:
```typescript
// src/hooks/use-proxy-delay-state.ts
export function useProxyDelayState(
  proxy: IProxyItem,
  groupName: string,
  timeout: number, // Pass from parent instead of using useVerge() query inside hook
): UseProxyDelayState
```

#### Fix 5: Adding Missing Dependency `_` in `useFilterSort` (`src/components/proxy/use-filter-sort.ts`)
```diff
<<<<
  const compute = useMemo(() => {
    const fp = filterProxies(proxies, groupName, filterText, searchState)
    const sp = sortProxies(
      fp,
      groupName,
      sortType,
      verge?.default_latency_timeout,
    )
    return sp
  }, [
    proxies,
    groupName,
    filterText,
    sortType,
    searchState,
    verge?.default_latency_timeout,
  ])
====
  const compute = useMemo(() => {
    const fp = filterProxies(proxies, groupName, filterText, searchState)
    const sp = sortProxies(
      fp,
      groupName,
      sortType,
      verge?.default_latency_timeout,
    )
    return sp
  }, [
    _ , // Added missing delay state trigger
    proxies,
    groupName,
    filterText,
    sortType,
    searchState,
    verge?.default_latency_timeout,
  ])
>>>>
```

#### Fix 6: Clearing Timeouts in `UnlockPage` (`src/pages/unlock.tsx`)
```diff
<<<<
  const invokeWithTimeout = async <T,>(
    cmd: string,
    args?: any,
    timeout = 15000,
  ): Promise<T> => {
    return Promise.race([
      invoke<T>(cmd, args),
      new Promise<T>((_, reject) =>
        setTimeout(
          () =>
            reject(new Error(t('tests.unlock.page.messages.detectionTimeout'))),
          timeout,
        ),
      ),
    ])
  }
====
  const invokeWithTimeout = async <T,>(
    cmd: string,
    args?: any,
    timeout = 15000,
  ): Promise<T> => {
    let timerId: any = null
    const timeoutPromise = new Promise<T>((_, reject) => {
      timerId = setTimeout(
        () =>
          reject(new Error(t('tests.unlock.page.messages.detectionTimeout'))),
        timeout,
      )
    })
    try {
      return await Promise.race([
        invoke<T>(cmd, args),
        timeoutPromise,
      ])
    } finally {
      if (timerId) {
        clearTimeout(timerId)
      }
    }
  }
>>>>
```

---

## 5. Verification Method

To verify these resource optimization recommendations:
1. **Compilation Check**:
   Run the vite build script to ensure no TypeScript compilation errors are introduced by changing any interfaces/prop definitions:
   ```powershell
   npm run build
   ```
2. **React Re-render Verification**:
   - Open React Developer Tools -> Profiler.
   - Click "Record", toggle connections or stream logs, and verify that already-rendered `RowComponent` and `LogItem` instances show as "Did not render" (colored gray) in the flame chart.
3. **Resize Listeners Verification**:
   - Open Chrome DevTools.
   - Run `getEventListeners(window)` in the Console while on the Proxy page.
   - Verify that the count of `resize` listeners is $O(1)$ instead of scaling linearly with the number of visible proxy nodes.
4. **Active Timers Audit**:
   - Run multiple media checks in the Unlock page.
   - Use the Chrome DevTools Performance panel or inspect the active timers list to verify that timers are immediately garbage collected after the Tauri command resolves.
