# Handoff Report: Active Connection Node & Latency Display Row Exploration

## 1. Observation
We explored the ClashVerge codebase and observed the following:
* **Active Node Card Component**:
  - Located in `src/pages/_layout/components/active-node-card.tsx` rendering `ActiveNodeStatusCard` (lines 34-361).
  - It handles active node cycling on click via `handleCycleNode` (lines 132-207) and manual latency recheck via `handleTestDelay` (lines 114-130).
* **Layout Mount Point**:
  - Located in `src/pages/_layout.tsx` (lines 79, 1675), rendered inside a flex element:
    ```tsx
    <div style={{ flex: 1, minWidth: 0 }}>
      <ActiveNodeStatusCard />
    </div>
    ```
* **Latency Management & Caching**:
  - Located in `src/services/delay.ts` (lines 15-400) via the `DelayManager` class.
  - Implements caching in `cache = new Map<string, DelayUpdate>()` with a 30-minute TTL (line 13).
  - Visual color mappings and signal strength thresholds are located in `src/pages/_layout/utils/style-helpers.tsx` (lines 19-87) via `getSignalIcon` and `convertDelayColor`.
* **State Management & Optimization**:
  - Query state and optimized fetching are located in `src/providers/app-data-provider.tsx` (lines 68-189).
  - Narrow width mode detection uses `isMiniStatus` (`window.innerWidth <= 285 && window.innerHeight <= 100`). When true, `fetchProxies` only queries the `PROXY` group details instead of all groups to save overhead.
* **Layout Hiding & Clipping Warning Comment**:
  - Located in `src/pages/_layout/components/connections-panel.tsx` (lines 45-50):
    ```tsx
    // WARNING [FOR AI AGENTS / AUDITORS]:
    // This minWidth must remain 0 and MUST NOT be set to a fixed minimum width.
    // Shrinking to 0px is the INTENDED behavior for 270px narrow width mode to hide
    // the panel by sliding/clipping it off-screen, per the design agreement.
    minWidth: 0,
    ```

## 2. Logic Chain
1. Component rendering is delegated to `ActiveNodeStatusCard` (`active-node-card.tsx`), integrated under the main `_layout.tsx` container inside a flex box layout.
2. Latency calculations and caching are managed via the `DelayManager` class (`src/services/delay.ts`), which performs single-node and batch latency testing.
3. State hooks for latency status (`testing`), address resolver (`nodeAddr`), window size (`isMinimalWidth`, `isMiniStatus`), and query parameters (`getProxies` with refetch interval of 3000ms) control the rendering, optimization, and visibility of the components.
4. Visibility/layout clipping for narrow width modes is dictated by the design agreement warning comment in `connections-panel.tsx` (`minWidth: 0`), allowing sliding off-screen under 270px width.

## 3. Caveats
* We did not investigate backend Tauri Rust handlers (e.g. `get_proxy_addr` or `delay_proxy_by_name` rust implementations) beyond their JS wrapper definitions.
* We assumed the default layout mode is governed by the 6 visual skins mentioned in `clash_mini_agreements.md` (e.g., `retro-3d`), which might use alternate styling components not fully mapped here.

## 4. Conclusion
The active connection node and latency display row are fully managed by the `ActiveNodeStatusCard` component, supported by the `DelayManager` caching service and optimized at the context provider level (`AppDataProvider`) for narrow window profiles. Layout clipping for narrow sizes is enforced by `minWidth: 0` constraints per warning comments in `connections-panel.tsx`.

## 5. Verification Method
* Inspect the target components:
  - `src/pages/_layout/components/active-node-card.tsx`
  - `src/providers/app-data-provider.tsx`
  - `src/pages/_layout/components/connections-panel.tsx`
* Verify TypeScript compilation using:
  ```bash
  pnpm typecheck
  ```
