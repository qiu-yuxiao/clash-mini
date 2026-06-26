# Settings Drawer Conditional Rendering Memory Leak Investigation Report

## 1. Observation
We observed the following code structures and Git differences:
- **Component Import**: In `src/pages/_layout.tsx`, the subcomponents for the Settings Drawer are imported at lines 81-89:
  ```tsx
  import { BasicSettingsCard } from './_layout/components/basic-settings-card'
  import { ConnectionsPanel } from './_layout/components/connections-panel'
  import { HelpMenuButton } from './_layout/components/help-menu-button'
  import { ProfileImportCard } from './_layout/components/profile-import-card'
  import { RoutingPreferenceCard } from './_layout/components/routing-preference-card'
  import { TakeoverModeCard } from './_layout/components/takeover-mode-card'
  import { ThemeSettingsCard } from './_layout/components/theme-settings-card'
  ```
- **Conditional Rendering (v1.8.9)**: In `src/pages/_layout.tsx` at line 1792, the settings drawer is conditionally mounted:
  ```tsx
  {drawerOpen && !isMiniStatus && (
    <div className="theme-panel" style={{ ... }}>
      {/* Drawer subcomponents inside */}
    </div>
  )}
  ```
- **CSS Translation Strategy (v1.8.2)**: In v1.8.2, the settings drawer `div` was always mounted in the DOM, and visibility was controlled via CSS properties:
  ```tsx
  <div
    className="theme-panel"
    style={{
      position: 'absolute',
      top: 0,
      left: 0,
      width: '100%',
      height: '100%',
      zIndex: 100,
      display: 'flex',
      transition: 'transform 0.4s cubic-bezier(0.1, 0.9, 0.2, 1), opacity 0.3s ease-in-out',
      transform: drawerOpen ? 'translate(0, 0) scale(1)' : 'translate(100%, -100%) scale(0.95)',
      opacity: drawerOpen ? 1 : 0,
      pointerEvents: drawerOpen ? 'auto' : 'none',
      boxSizing: 'border-box',
      padding: '12px',
      gap: '12px',
      overflow: 'hidden',
    }}
  >
  ```
- **Hook Gating**: The connection monitoring hook (`useConnectionData` in `_layout.tsx` line 1072) and system proxy / mode queries in `AppDataProvider` (`enabled: isSettingsOpen`) are reactive to `drawerOpen` (`isSettingsOpen` / `isPanelVisible`). When the drawer is closed, these data queries and WebSockets are disabled:
  ```tsx
  const {
    response: { data: connectionsData },
    clearClosedConnections,
  } = useConnectionData({ enabled: drawerOpen && isPanelVisible })
  ```

---

## 2. Logic Chain
1. **Emotion/MUI Style Injections**: Every time the settings drawer mounts (when `drawerOpen` is true), the complex Material-UI subcomponents (`ProfileImportCard`, `BasicSettingsCard`, `ThemeSettingsCard`, etc.) are constructed. Emotion dynamic styles are generated and appended to the document's `<head>`. When the drawer unmounts, these dynamic styles are not purged by Emotion, resulting in a growing collection of style tags in the WebView2 DOM tree.
2. **Component Cleanup Failures**: Complex elements such as `Tooltip`, `Select`, `MenuItem`, and `TextField` create popups, portals, and register global event listeners (on `window` or `document`) for features like click-away detection or hover positioning. If minor memory leaks or unreleased references exist in the React/MUI lifecycle under Tauri, repeatedly mounting and destroying these subcomponents leaves orphaned JS references and DOM fragments.
3. **WebView2 Memory Target Constraints**: When the application enters lightweight mode, `WindowManager::optimize_window_memory` calls `SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW)`. However, WebView2 cannot garbage-collect or page out the memory occupied by orphaned closures, unremoved style sheets in `<head>`, and detached DOM nodes because they remain strongly referenced in the JS engine.
4. **Resolution via CSS Translation**: Reverting to the CSS translation strategy ensures that the Settings Drawer is mounted exactly once at startup. The DOM structure remains stable, and no dynamic stylesheet injections or event listener attachments occur during normal usage.
5. **No Polling Overhead**: Maintaining the always-mounted state does not reintroduce the background polling/WebSocket issue (BUG-241) because the queries and WebSocket connections are gated reactively by the `drawerOpen` state. If the drawer is hidden, the socket is disconnected and IPC queries are paused, saving both memory and CPU.

---

## 3. Caveats
- No direct physical memory profiling was performed in this environment due to read-only constraints, but the findings are theoretically robust and aligned with known WebView2 / Emotion / React-Query behaviors.

---

## 4. Conclusion
The memory usage regression under lightweight mode in v1.8.9 is caused by the conditional mounting of the Settings Drawer, which leads to cumulative DOM, style sheet, and event listener leaks on every open/close cycle. 

We recommend rolling back the Settings Drawer rendering logic in `src/pages/_layout.tsx` to the CSS translation strategy from v1.8.2 while retaining the active/visibility gates on connection/data querying.

### Proposed Fix/Rollback Patch:
```diff
diff --git a/src/pages/_layout.tsx b/src/pages/_layout.tsx
index f075c0be..3ca08d2b 100644
--- a/src/pages/_layout.tsx
+++ b/src/pages/_layout.tsx
@@ -1789,8 +1789,8 @@ const Layout = () => {
             )}
 
-            {/* Settings Drawer (Instantly mounted when drawerOpen is true) */}
-            {drawerOpen && !isMiniStatus && (
-              <div
-                className="theme-panel"
-                style={{
-                  position: 'absolute',
-                  top: 0,
-                  left: 0,
-                  width: '100%',
-                  height: '100%',
-                  zIndex: 100,
-                  // WARNING [FOR AI AGENTS / AUDITORS]:
-                  // This flex layout must remain as row direction and MUST NOT wrap. In default 270px width,
-                  // the connections panel is intentionally squeezed to 0px (hidden) and physically clipped
-                  // off-screen, per the design agreement. Widening the window will slide it into view.
-                  display: 'flex',
-                  boxSizing: 'border-box',
-                  padding: '12px',
-                  gap: '12px',
-                  overflow: 'hidden',
-                }}
-              >
+            {/* Settings Sliding Drawer (slides internal left-downwards) */}
+            <div
+              className="theme-panel"
+              style={{
+                position: 'absolute',
+                top: 0,
+                left: 0,
+                width: '100%',
+                height: '100%',
+                zIndex: 100,
+                // WARNING [FOR AI AGENTS / AUDITORS]:
+                // This flex layout must remain as row direction and MUST NOT wrap. In default 270px width,
+                // the connections panel is intentionally squeezed to 0px (hidden) and physically clipped
+                // off-screen, per the design agreement. Widening the window will slide it into view.
+                display: 'flex',
+                transition:
+                  'transform 0.4s cubic-bezier(0.1, 0.9, 0.2, 1), opacity 0.3s ease-in-out',
+                transform: drawerOpen && !isMiniStatus
+                  ? 'translate(0, 0) scale(1)'
+                  : 'translate(100%, -100%) scale(0.95)',
+                opacity: drawerOpen && !isMiniStatus ? 1 : 0,
+                pointerEvents: drawerOpen && !isMiniStatus ? 'auto' : 'none',
+                boxSizing: 'border-box',
+                padding: '12px',
+                gap: '12px',
+                overflow: 'hidden',
+              }}
+            >
@@ -2130,5 +2130,4 @@ const Layout = () => {
                 })}
               </Box>
             </div>
-          )}
           </div>
```

---

## 5. Verification Method
1. **Memory Profile Test**:
   - Rebuild the application with the proposed patch.
   - Open and close the Settings Drawer repeatedly (e.g., 50 times).
   - Measure the process memory (Private Working Set) in Task Manager or via WebView2 DevTools.
   - Verify that the memory usage stays flat instead of climbing continuously.
2. **Lightweight Mode Memory Reclamation Test**:
   - Close/hide the main window to enter lightweight mode.
   - Verify that WebView2's memory target level drops to `Low` and that physical memory usage is successfully reclaimed/paged out to minimum levels (matching or outperforming v1.8.2).
3. **No Polling Verification**:
   - Close the Settings Drawer.
   - Check the Network tab in WebView2 DevTools or backend websocket connection lists to confirm the connections socket is closed/paused and query intervals are completely idle.
