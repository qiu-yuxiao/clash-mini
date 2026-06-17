# Frontend Audit Handoff Report

## 1. Observation
Below is the list of issues identified during the static analysis of the React/TypeScript frontend (located in `src/`).

### Finding 1: Async Event Listener Memory Leak in AppDataProvider
*   **File Path**: `src/providers/app-data-provider.tsx`
*   **Line Ranges**: 230–292
*   **Problem Description**: In the `useEffect` block registering Tauri event listeners (`profile-changed` and `verge://refresh-proxy-config`), the listener registration is asynchronous (`await listen(...)`). If the provider component unmounts before these promises resolve, the cleanup function (which runs synchronously on unmount) will execute while the `cleanupFns` array is still empty. When the promises subsequently resolve and push the unlisten functions onto `cleanupFns`, they are never called. This leads to a persistent memory leak of Tauri global events.
*   **Code Snippet**:
    ```typescript
    useEffect(() => {
      let lastProfileId: string | null = null
      let lastUpdateTime = 0
      const refreshThrottle = 800
      const cleanupFns: Array<() => void> = []

      // ...

      const initializeListeners = async () => {
        try {
          const unlistenProfile = await listen<string>(
            'profile-changed',
            handleProfileChanged,
          )
          cleanupFns.push(unlistenProfile)
        } catch (error) {
          console.error('[AppDataProvider] 监听 Profile 事件失败:', error)
        }

        try {
          const unlistenProxy = await listen(
            'verge://refresh-proxy-config',
            handleRefreshProxy,
          )
          cleanupFns.push(unlistenProxy)
        } catch (error) {
          console.warn('[AppDataProvider] 设置 Tauri 事件监听器失败:', error)
        }
      }

      void initializeListeners()

      return () => {
        cleanupFns.forEach((fn) => {
          try {
            fn()
          } catch (error) {
            console.error('[DataProvider] Cleanup error:', error)
          }
        })
      }
    }, [refreshProxy, refreshRules, refreshRuleProviders])
    ```
*   **Proposed Refactoring**: Introduce a boolean flag `active` to track whether the effect is still active. If the effect has cleaned up before the listeners resolve, immediately invoke the unlisten callbacks.
    ```typescript
    useEffect(() => {
      let lastProfileId: string | null = null
      let lastUpdateTime = 0
      const refreshThrottle = 800
      
      let active = true
      let unlistenProfile: (() => void) | null = null
      let unlistenProxy: (() => void) | null = null

      const handleProfileChanged = (event: { payload: string }) => { ... }
      const handleRefreshProxy = () => { ... }

      const initializeListeners = async () => {
        try {
          const uProfile = await listen<string>(
            'profile-changed',
            handleProfileChanged,
          )
          if (!active) {
            uProfile()
          } else {
            unlistenProfile = uProfile
          }
        } catch (error) {
          console.error('[AppDataProvider] 监听 Profile 事件失败:', error)
        }

        try {
          const uProxy = await listen(
            'verge://refresh-proxy-config',
            handleRefreshProxy,
          )
          if (!active) {
            uProxy()
          } else {
            unlistenProxy = uProxy
          }
        } catch (error) {
          console.warn('[AppDataProvider] 设置 Tauri 事件监听器失败:', error)
        }
      }

      void initializeListeners()

      return () => {
        active = false
        if (unlistenProfile) unlistenProfile()
        if (unlistenProxy) unlistenProxy()
      }
    }, [refreshProxy, refreshRules, refreshRuleProviders])
    ```

---

### Finding 2: High-Frequency Resize Re-render Storm
*   **File Path**: `src/components/proxy/use-window-width.ts` (lines 1–17) and `src/components/proxy/proxy-item.tsx` (lines 80–82)
*   **Problem Description**: The hook `useWindowWidth` registers a window `resize` listener that updates a numeric width state (`setWidth(document.body.clientWidth)`) on every pixel of window resizing. Since the list of proxies can contain dozens or hundreds of items, and each individual `ProxyItem` invokes `useWindowWidth()`, resizing the window triggers:
    1. Dozens/hundreds of independent window resize listeners executing concurrently.
    2. High-frequency state updates in every proxy item component, forcing massive numbers of concurrent React re-renders.
    However, the proxy items only use the width to decide whether `width <= 285` (minimal layout mode).
*   **Code Snippet (`use-window-width.ts`)**:
    ```typescript
    export const useWindowWidth = () => {
      const [width, setWidth] = useState(() => document.body.clientWidth)

      useEffect(() => {
        const handleResize = () => setWidth(document.body.clientWidth)

        window.addEventListener('resize', handleResize)
        return () => {
          window.removeEventListener('resize', handleResize)
        }
      }, [])

      return { width }
    }
    ```
    **Code Snippet (`proxy-item.tsx`)**:
    ```typescript
      const { width } = useWindowWidth()
      const isMinimal = width <= 285
    ```
*   **Proposed Refactoring**:
    **Option A (MUI Media Query)**: Since the project uses Material-UI, replace the custom JS resize hook in `ProxyItem` with MUI's optimized CSS-media-query-backed hook:
    ```typescript
    import { useMediaQuery } from '@mui/material'
    // Inside ProxyItem:
    const isMinimal = useMediaQuery('(max-width:285px)')
    ```
    **Option B (Threshold Boolean Hook)**: Alternatively, write an optimized hook that only tracks a boolean threshold. This allows React to bail out of state updates when the value does not cross the boundary:
    ```typescript
    export const useIsMinimal = (threshold = 285) => {
      const [isMinimal, setIsMinimal] = useState(() => window.innerWidth <= threshold)
      
      useEffect(() => {
        const handleResize = () => {
          setIsMinimal(window.innerWidth <= threshold)
        }
        window.addEventListener('resize', handleResize)
        return () => window.removeEventListener('resize', handleResize)
      }, [threshold])
      
      return isMinimal
    }
    ```

---

### Finding 3: Architectural "God Component" Monolith in Layout
*   **File Path**: `src/pages/_layout.tsx`
*   **Line Ranges**: 1–4997 (Entire file)
*   **Problem Description**: The main `Layout` component is a 4,997-line monolithic file containing 38 separate `useState` hooks. It violates the Single Responsibility Principle by combining shell drawer layouts, connection list tables, profile list and modification managers, system settings panel forms, update checks, theme managers, and inline dialogues. Any single state change triggers a heavy evaluation of this massive component.
*   **Code Snippet (Example State Declarations)**:
    ```typescript
    // Inside _layout.tsx (starts at line 1242 and spans hundreds of lines of state and callbacks)
    const [controlSkin, setControlSkin] = useState(...)
    const [depthFactor, setDepthFactor] = useState(...)
    const [vibrancyFactor, setVibrancyFactor] = useState(...)
    const [helpAnchorEl, setHelpAnchorEl] = useState(...)
    const [clientUpdateOpen, setClientUpdateOpen] = useState(...)
    // ... total of 38 useState hooks
    ```
*   **Proposed Refactoring**:
    1.  **Decompose to view panels**: Extract domain panels into subcomponents located in `src/components/`, such as `ProfilesPanel`, `ConnectionsPanel`, `SettingsPanel`.
    2.  **Move business logic to custom hooks**: Extract client/core update state logic and network requests into a custom hook `useAppUpdater`.
    3.  **Shell only**: Simplify `_layout.tsx` to only render the sidebar navigation skeleton, and mount the sub-panels dynamically based on route selection.

---

### Finding 4: Global Ambient Type Pollution
*   **File Path**: `src/types/global.d.ts`
*   **Line Ranges**: 1–1097
*   **Problem Description**: A single central ambient definition file `global.d.ts` contains almost all major domain interfaces (`IConfigData`, `IProxyItem`, `IProxyGroupItem`, `ITrafficItem`, etc.). Ambient global declarations mask dependency imports and increase the risk of namespace collisions, making future refactoring or modularization difficult.
*   **Proposed Refactoring**: Declare types using standard TypeScript ES modules (`export interface`) in domain files under `src/types/` (e.g. `src/types/clash.ts`, `src/types/profile.ts`) and import them where needed.

---

### Finding 5: Naming Convention Inconsistency
*   **File Path**: `src/hooks/useWindowSnap.ts`
*   **Problem Description**: The hook `useWindowSnap.ts` uses camelCase naming for its file name, while all other custom hook files in the directory use kebab-case (`use-clash.ts`, `use-traffic-monitor.ts`).
*   **Proposed Refactoring**: Rename the file to `use-window-snap.ts` to follow the project's folder convention.

---

### Finding 6: Non-Memoized Hook Functions
*   **File Path**: `src/hooks/use-clash.ts` and `src/hooks/use-profiles.ts`
*   **Problem Description**: Helper functions returned by custom hooks (e.g. `mutateClash` in `useClash`, `mutateProfiles` and `patchProfiles` in `useProfiles`) are recreated on every render. If components pass these functions down as props or list them as dependencies in `useEffect`, it will trigger unnecessary renders or effect loops.
*   **Code Snippet (`use-profiles.ts`)**:
    ```typescript
    const mutateProfiles = async () => {
      await refetch()
    }
    ```
*   **Proposed Refactoring**: Wrap helper functions in `useCallback`:
    ```typescript
    const mutateProfiles = useCallback(async () => {
      await refetch()
    }, [refetch])
    ```

---

### Finding 7: Loose Type Definitions (Any Type)
*   **File Path**: `src/providers/app-data-context.ts` (lines 11, 14, 44, 60) and `src/pages/_layout.tsx` (lines 1314, 1324)
*   **Problem Description**: Context attributes like `proxies` and `sysproxy` are typed as `any`. Similarly, layout states like `clientUpdateObj` and `coreUpdateRelease` are typed as `any`. This bypasses TypeScript checks and could lead to runtime errors when object fields are accessed.
*   **Proposed Refactoring**: Replace `any` with specific types or interface definitions (e.g., using model types exported from `tauri-plugin-mihomo-api` or a custom types file).

---

## 2. Logic Chain
1.  **Memory Leak**: Tauri's `listen` is asynchronous, returning a promise. Because the registration runs in an async wrapper inside `useEffect`, and the cleanup runs immediately upon unmount, any unmount that happens before the promise resolves will fail to clean up the registered listener since the resolved unlisten function hasn't been pushed to the cleanup array yet.
2.  **Resize Loop**: Listening to `resize` events on every pixel and updating numeric state forces continuous layout recalculations and React re-renders. Since each `ProxyItem` component subscribes to this hook, the number of re-renders scales as `O(N * pixels)` where `N` is the number of proxy items. Changing this to CSS-media-query-backed hooks or boolean threshold variables stops re-renders during resizing, since the boolean state changes only when crossing the boundary.
3.  **God Component**: A component with nearly 5,000 lines and 38 states violates basic software design patterns (SRP). It is difficult to read and forces React to execute a huge render function when any small UI state changes.

## 3. Caveats
- No runtime testing was done as this is a static, read-only audit.
- Specific impact on performance (e.g. framerates during resize) was inferred based on code structure and standard React rendering performance metrics.

## 4. Conclusion
The frontend is built on modern technologies (React 19, React Router 7, TanStack Query) and has clean state abstractions (using `foxact`). However, it suffers from a critical async race condition memory leak in Tauri event listeners, a significant rendering performance issue during window resizing, and architectural coupling in the layout module. Addressing these issues will yield a much more robust, performant, and maintainable codebase.

## 5. Verification Method
- **Memory Leak**: Place a `console.log` inside the `useEffect` cleanup and inside the Tauri listener callback. Mount and immediately unmount `AppDataProvider` (e.g., toggling some layout or routing). Trigger the `profile-changed` Tauri event from Rust side. If the callback still prints to console after unmount, the listener leaked.
- **Resize Storm**: Open React Developer Tools Profiler, resize the window. Observe the number of re-renders triggered in the proxy item list. Apply the media query / boolean threshold optimization and observe the dramatic drop in re-render counts.
- **Project build/test command**: The project configuration uses `pnpm typecheck` (`tsc --noEmit`) to verify TypeScript compilations.

---

## 6. Summary Checklist
- [ ] Refactor async Tauri event listeners in `AppDataProvider` to prevent promise race conditions.
- [ ] Optimize window width resize tracking inside `ProxyItem` (replace `useWindowWidth` with media queries or boolean `useIsMinimal`).
- [ ] Decompose `src/pages/_layout.tsx` into smaller view-only panel components and custom hooks.
- [ ] Shift ambient type definitions in `global.d.ts` to module-scoped ESM types.
- [ ] Rename `useWindowSnap.ts` to `use-window-snap.ts`.
- [ ] Memoize utility functions in `useClash` and `useProfiles` hooks using `useCallback`.
- [ ] Clean up `any` type definitions in context schemas and state variables.
