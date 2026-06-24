# Analysis: Clash Mini Active Connection Node and Latency Display Components

This report documents the React components, page layout integration, state hooks, and delay/latency calculation mechanisms related to the top active connection node and latency display row in the Clash Mini project.

---

## 1. Components and File Locations

The active connection node and latency display row are implemented and integrated across the following source files:

### A. Active Connection Node Card Component
* **File Path**: `src/pages/_layout/components/active-node-card.tsx`
* **React Component**: `ActiveNodeStatusCard`
* **Role**: Renders the top status row containing:
  - The label `"当前活跃出口节点："` (via localization key `settings.mini.activeNodeLabel`).
  - The friendly protocol name (e.g., `Shadowsocks`, `VMess`, or `Direct`) calculated using `getFriendlyProtocolName(type)`.
  - The active node's display name (filtered to strip trailing index numbers).
  - The latency status chip showing the current delay formatting (`delayManager.formatDelay(delay)`) and icon/color representation.
  - The physical address of the active node `(IP:Port)` queried asynchronously.
  - Interactive mouse actions: clicking the node name triggers cycling to the next candidate node, and clicking the latency chip triggers a recheck of the current node's latency.

### B. Layout Page Wrapper
* **File Path**: `src/pages/_layout.tsx`
* **Role**: Integrates the `ActiveNodeStatusCard` in the main application shell layout.
  - The component is rendered as part of the upper pane (node selection area) under the conditional check `!drawerOpen`:
    ```tsx
    {/*置顶当前节点与快捷控制栏（仅在未打开设置时渲染）*/}
    {!drawerOpen && (
      <div
        data-no-drag="true"
        style={{
          display: 'flex',
          alignItems: 'center',
          width: '100%',
          boxSizing: 'border-box',
          padding: '3px 36px 2px 8px',
          position: 'relative',
          zIndex: 110,
        }}
      >
        <div style={{ flex: 1, minWidth: 0 }}>
          <ActiveNodeStatusCard />
        </div>
        ...
      </div>
    )}
    ```

### C. Visual Style & Helper Utilities
* **File Path**: `src/pages/_layout/utils/style-helpers.tsx`
* **Role**: Defines visual properties mapping latency to MUI icons and colors:
  - `getSignalIcon(delay, t)`: Maps delay ranges to signal strength icons (`SignalWifi4Bar`, `SignalWifi3Bar`, `SignalWifi2Bar`, `SignalWifi1Bar`, `SignalWifi0Bar`, `WifiOff`) and status descriptions.
  - `convertDelayColor(delayValue)`: Convers delay values into MUI color classes (`success`, `warning`, `error`, `primary`, or `default`) by querying `delayManager.formatDelayColor`.

### D. Delay / Latency Service Manager
* **File Path**: `src/services/delay.ts`
* **Role**: Handles caching, checking, and formatting the latency/delay calculations.
  - Maintains `DelayUpdate` cache in a `Map<string, DelayUpdate>` with a 30-minute TTL.
  - `checkDelay(name, group, timeout)`: Triggers latency test via Tauri plugin API `delayProxyByName(name, url, timeout)` with 500ms minimum display threshold (loading state).
  - `checkListDelay(nameList, group, timeout, concurrency)`: Batches list checks with concurrency limit.
  - `formatDelay(delay, timeout)` & `formatDelayColor(delay, timeout)`.

### E. App Data Providers
* **File Path**: `src/providers/app-data-context.ts` & `src/providers/app-data-provider.tsx`
* **Role**: Manages the core query and states for proxies and refreshers (`useProxiesData`, `useAppRefreshers`).
  - Implements the optimized `fetchProxies` logic under `isMiniStatus` (narrow width state) to only fetch details for the `PROXY` group and active node name, bypassing full proxy list calculations to prevent layout collapse and performance degradation.

---

## 2. State Hooks and Custom Hooks

The active connection node and latency calculations are controlled by the following state hooks:

### A. States within `ActiveNodeStatusCard` (`active-node-card.tsx`)
1. **`useState(false)` (for `testing`)**:
   - Controls the loading/spinning animation in the Chip when a manual latency test is running.
2. **`useState<string>('')` (for `nodeAddr`)**:
   - Stores the physical address `(IP:Port)` of the current active node.
3. **`useEffect` (Address Fetcher)**:
   - Triggers `getProxyAddr(activeNodeName, activeNodeRecord?.provider)` on mount or whenever `activeNodeName` or the node's provider changes.
   - Cleans up via a `cancelled` boolean guard to prevent state updates on unmounted components (race condition prevention).
4. **`useMemo` (Data Selectors)**:
   - `primaryGroup`: Resolves the group representing node selection (e.g., `PROXY`, `auto`, `select`).
   - `activeNodeRecord`: Dynamically extracts the active node item from `proxies` records, group lists, or global lists.
   - `delay`: Retrieves the cached or historical delay for the active node using `delayManager.getDelayFix`.
   - `skinFallback`: Reads the theme skin setting from `localStorage`.

### B. States within Layout (`_layout.tsx`)
1. **`isMinimalWidth` state (`useState`)**:
   - Initialized and updated via window resize listener checking `window.innerWidth <= 285`.
2. **`useHeadStateNew`**:
   - Manages node listing, filtering, search inputs, and sort status.

### C. States within `AppDataProvider` (`app-data-provider.tsx`)
1. **`isMinimalWidth` state (`useState`)**:
   - Tracks if width `<= 285` px.
2. **`isMiniStatus` state (`useState`)**:
   - Tracks if width `<= 285` px and height `<= 100` px.
3. **`forceFullProxiesRef` (`useRef`)**:
   - Bypasses the narrow-layout proxy query optimization to fetch full proxy details when force-refreshed.
4. **`useQuery` (`getProxies` query)**:
   - Fetches proxy data via `fetchProxies` every 3000ms when the window is visible.

---

## 3. Calculations and Node Selection Logic

### A. Latency Thresholds and Mapping
In `style-helpers.tsx`, delay thresholds are defined as:
* `delay === -2`: Testing state (displays loading animation).
* `delay === -1`: Untested node.
* `delay > 1e5` or `delay === 0` or `delay >= 10000`: Timeout/Error state.
* `delay >= 500`: High Latency (`error.main` / Red).
* `delay >= 300`: Medium Latency (`warning.main` / Orange).
* `delay >= 200`: Good Latency (`info.main` / Blue).
* `delay < 200`: Excellent Latency (`success.main` / Green).

### B. Active Node Cycling (`handleCycleNode`)
When a user clicks on the active node name:
1. It reads the local storage key `'proxy-head-state'` to dynamically retrieve current filtering/sorting/regex rules for the primary proxy group.
2. It processes the list through `filterSort` to obtain the candidate list.
3. It searches for the index of `activeNodeName` in the list.
4. It iterates to find the next healthy candidate node (non-timeout, non-zero latency).
5. If no healthy candidates are found, it falls back to the absolute next candidate index.
6. It triggers `selectNodeForGroup(primaryGroup.name, nextNodeName)` and refreshes display using `refreshProxy()`.

---

## 4. Layout Clipping & Design Agreements

### A. Layout Hiding Warning Comment
In `src/pages/_layout/components/connections-panel.tsx` (lines 45-50):
```tsx
// WARNING [FOR AI AGENTS / AUDITORS]:
// This minWidth must remain 0 and MUST NOT be set to a fixed minimum width.
// Shrinking to 0px is the INTENDED behavior for 270px narrow width mode to hide
// the panel by sliding/clipping it off-screen, per the design agreement.
minWidth: 0,
```
This design agreement ensures that in narrow modes (e.g. 270px window width), the connections panel collapses to `0px` and slides off-screen, allowing the top active node row and main node selection items to clip clean without layout distortion.

### B. Active Node Cycle Constraints
In `clash_mini_agreements.md` (Section 14: 导航栏置顶出口节点实时子集轮换规范):
* It forbids caching static node lists in `useMemo` or `useEffect` for `handleCycleNode`.
* It mandates reading filtering inputs (`clash-verge-node-search`, `clash-verge-hide-unavialable`, and `clash-verge-sort-type`) from `localStorage` immediately upon user interaction to compute the correct current candidate subset.
