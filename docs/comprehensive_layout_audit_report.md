# Comprehensive Layout and Rendering Correctness Audit Report

## 1. Executive Summary
This audit was commissioned to diagnose visual layout/rendering correctness bugs in Clash Mini (Clash Verge UI variant) and to audit the impact of upgrading the `tauri-plugin-mihomo` plugin from version `0.5.2` to `0.5.4`.

### Key Findings:
1. **Outbound Node Card Layout & Icon Inflation**:
   - The layout collapse is caused by CSS flex items defaulting to `min-width: auto`. Typography constraints based on viewport breakpoints fail when the container is squeezed in narrow panels.
   - SvgIcon size inflation is caused by Emotion classes (`sx`) having lower CSS specificity than the default MUI `.MuiChip-icon` styles (which enforce `fontSize: 24px`). Reverting to inline `style` overrides restores layout dimensions to `12px`.
   - The speed test spinner fails to display on background speed tests due to a state check limitation (`testing` vs `delay === -2`).
2. **Missing Proxy Node Table/List**:
   - The proxy single-column list/table layout (`type: 2`) was inaccessible because column layout calculations hardcoded the column count to `3`, ignoring the user's setting.
   - Collapsible group headers (`type: 0`) were completely omitted from list rendering in normal mode, collapsing all proxy nodes into a single flat list without separators.
3. **Double-Border Outline**:
   - A `4px double` border was globally enforced via the `.theme-panel` class in `index.scss` and cell components inside `proxy-render.tsx` / `connection-table.tsx`. Replacing these with a clean `1px solid` border resolves the double-border style anomaly.
4. **Plugin & Build Script Audit**:
   - The upgrade from `0.5.2` to `0.5.4` is a metadata-only change in Cargo configuration files to satisfy `verify.py` validation. It does not contain layout or rendering code modifications.
   - No build scripts, patches, or batch files were deleted. Manually executing `cargo test` followed by `pnpm build` in the plugin directory is only necessary if plugin models/bindings are modified.

---

## 2. Target Layout Bug Diagnosis

### Bug 1: Outbound Node Card Collapse, Delay Icon Sizing, and Speed Test Spinner
* **Affected Files & Links**:
  - [src/pages/_layout/components/active-node-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx#L271-L320)
  - [src/pages/_layout/utils/style-helpers.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/utils/style-helpers.tsx#L20-L68)
* **Root Causes**:
  - **Layout Collapse**: The node name `<Typography>` uses responsive `maxWidth` breakpoints (e.g., `md: '360px'`). Under wide screens with narrow side panels (e.g., 270px), it defaults to `360px`, overflowing container boundaries.
  - **Icon Sizing**: `getSignalIcon` returned SvgIcons with `sx={iconStyle}` overrides. Class-level specificity is lower than MUI's internal `.MuiChip-icon` selector. The icons were styled at 24px instead of 12px.
  - **Spinner Visibility & Sizing**: The card only checked local state `testing`. Sizing was bloated due to overriding via Emotion classes. Background speed tests (`delay === -2`) rendered static grey indicators instead of animated loaders.
* **Proposed Diffs**:

#### `src/pages/_layout/components/active-node-card.tsx`
* **File Location**: [src/pages/_layout/components/active-node-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx#L275-L318)
```diff
diff --git a/src/pages/_layout/components/active-node-card.tsx b/src/pages/_layout/components/active-node-card.tsx
--- a/src/pages/_layout/components/active-node-card.tsx
+++ b/src/pages/_layout/components/active-node-card.tsx
@@ -275,3 +275,3 @@
             fontWeight: 'bold',
             fontSize: '12px',
             color: isRetro3DDark ? '#2C1F03' : 'text.primary',
-            maxWidth: { xs: '120px', sm: '240px', md: '360px' },
+            maxWidth: '120px',
             minWidth: 0,
             overflow: 'hidden',
@@ -301,9 +301,9 @@
       {activeNodeName && (
         <Chip
           size="small"
           icon={
-            testing ? (
-              <CircularProgress size={10} color="inherit" sx={{ width: '10px !important', height: '10px !important' }} />
+            (testing || delay === -2) ? (
+              <CircularProgress size={10} color="inherit" style={{ width: '10px', height: '10px' }} />
             ) : (
               signalInfo.icon
             )
           }
           label={
-            testing
+            (testing || delay === -2)
               ? t('settings.mini.statusTesting', { defaultValue: '测试中' }) +
                 '...'
               : delayManager.formatDelay(delay)
           }
```

#### `src/pages/_layout/utils/style-helpers.tsx`
* **File Location**: [src/pages/_layout/utils/style-helpers.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/utils/style-helpers.tsx#L20-L68)
```diff
diff --git a/src/pages/_layout/utils/style-helpers.tsx b/src/pages/_layout/utils/style-helpers.tsx
--- a/src/pages/_layout/utils/style-helpers.tsx
+++ b/src/pages/_layout/utils/style-helpers.tsx
@@ -20,49 +20,49 @@
 export function getSignalIcon(delay: number, t: any) {
   const iconStyle = { fontSize: '12px', width: '12px', height: '12px' }
   if (delay === -2)
     return {
-      icon: <SignalNone sx={iconStyle} />,
+      icon: <SignalNone style={iconStyle} />,
       text: t('settings.mini.statusTesting', { defaultValue: '测试中' }),
       color: 'text.secondary',
     }
   if (delay === -1)
     return {
-      icon: <SignalNone sx={iconStyle} />,
+      icon: <SignalNone style={iconStyle} />,
       text: t('settings.mini.statusUntested', { defaultValue: '未测试' }),
       color: 'text.secondary',
     }
   if (delay > 1e5)
     return {
-      icon: <SignalError sx={iconStyle} />,
+      icon: <SignalError style={iconStyle} />,
       text: t('settings.mini.statusError', { defaultValue: '错误' }),
       color: 'error.main',
     }
   if (delay === 0 || delay >= 10000)
     return {
-      icon: <SignalError sx={iconStyle} />,
+      icon: <SignalError style={iconStyle} />,
       text: t('settings.mini.statusTimeout', { defaultValue: '超时' }),
       color: 'error.main',
     }
   if (delay >= 500)
     return {
-      icon: <SignalWeak sx={iconStyle} />,
+      icon: <SignalWeak style={iconStyle} />,
       text: t('settings.mini.statusDelayHigh', { defaultValue: '延迟较高' }),
       color: 'error.main',
     }
   if (delay >= 300)
     return {
-      icon: <SignalMedium sx={iconStyle} />,
+      icon: <SignalMedium style={iconStyle} />,
       text: t('settings.mini.statusDelayMedium', { defaultValue: '延迟中等' }),
       color: 'warning.main',
     }
   if (delay >= 200)
     return {
-      icon: <SignalGood sx={iconStyle} />,
+      icon: <SignalGood style={iconStyle} />,
       text: t('settings.mini.statusDelayGood', { defaultValue: '延迟良好' }),
       color: 'info.main',
     }
   return {
-    icon: <SignalStrong sx={iconStyle} />,
+    icon: <SignalStrong style={iconStyle} />,
     text: t('settings.mini.statusDelayExcellent', { defaultValue: '延迟极佳' }),
     color: 'success.main',
   }
 }
```

---

## 3. Missing Table inside Proxy Node List View
* **Affected Files & Links**:
  - [src/components/proxy/use-render-list.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-render-list.ts#L82-L460)
* **Root Causes**:
  - **Layout Column Ignored**: In `use-render-list.ts`, `calculateColumns` completely ignored the user's configured column layout parameter (`_configCol`), hardcoding the return value to `3` (for widths > 285px). This prevented column counts of `1`, which renders the table-based single-column item list (`type: 2`).
  - **Collapsible Headers Missing**: The normal mode rendering logic in `use-render-list.ts` omitted pushing collapsible group headers (`type: 0`) and checking `headState.open`. This collapsed all proxy nodes into a single flat list without separators or group boundaries.
* **Proposed Diffs**:

#### `src/components/proxy/use-render-list.ts`
* **File Location**: [src/components/proxy/use-render-list.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/use-render-list.ts#L82-L460)
```diff
diff --git a/src/components/proxy/use-render-list.ts b/src/components/proxy/use-render-list.ts
--- a/src/components/proxy/use-render-list.ts
+++ b/src/components/proxy/use-render-list.ts
@@ -82,6 +82,6 @@
 const calculateColumns = (width: number, _configCol: number): number => {
   if (width <= 285) {
     return 1
   }
-  return 3
+  return _configCol
 }
@@ -420,40 +420,44 @@
-      ret.push({
-        type: 1,
-        key: `head-${group.name}`,
-        group,
-        headState,
-      })
-
-      if (!proxies.length) {
-        ret.push({
-          type: 3,
-          key: `empty-${group.name}`,
-          group,
-          headState,
-        })
-      } else if (col > 1) {
-        ret.push(
-          ...groupProxies(proxies, col).map((proxyCol, colIndex) => ({
-            type: 4 as const,
-            key: `col-${group.name}-${proxyCol[0]?.name ?? colIndex}`,
-            group,
-            headState,
-            col,
-            proxyCol,
-            provider: proxyCol[0]?.provider,
-            indexInGroup: colIndex,
-          })),
-        )
-      } else {
-        ret.push(
-          ...proxies.map((proxy, proxyIdx) => ({
-            type: 2 as const,
-            key: `${group.name}-${proxy?.name ?? proxyIdx}`,
-            group,
-            proxy,
-            headState,
-            provider: proxy.provider,
-            indexInGroup: proxyIdx,
-          })),
-        )
-      }
+      ret.push({
+        type: 0,
+        key: `group-${group.name}`,
+        group,
+        headState,
+      })
+
+      if (headState.open) {
+        ret.push({
+          type: 1,
+          key: `head-${group.name}`,
+          group,
+          headState,
+        })
+
+        if (!proxies.length) {
+          ret.push({
+            type: 3,
+            key: `empty-${group.name}`,
+            group,
+            headState,
+          })
+        } else if (col > 1) {
+          ret.push(
+            ...groupProxies(proxies, col).map((proxyCol, colIndex) => ({
+              type: 4 as const,
+              key: `col-${group.name}-${proxyCol[0]?.name ?? colIndex}`,
+              group,
+              headState,
+              col,
+              proxyCol,
+              provider: proxyCol[0]?.provider,
+              indexInGroup: colIndex,
+            })),
+          )
+        } else {
+          ret.push(
+            ...proxies.map((proxy, proxyIdx) => ({
+              type: 2 as const,
+              key: `${group.name}-${proxy?.name ?? proxyIdx}`,
+              group,
+              proxy,
+              headState,
+              provider: proxy.provider,
+              indexInGroup: proxyIdx,
+            })),
+          )
+        }
+      }
```

---

## 4. Colored Double-Border Outline around Proxy Node Table
* **Affected Files & Links**:
  - [src/components/proxy/proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L782-L792)
  - [src/components/proxy/proxy-render.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx#L84-L89)
  - [src/components/connection/connection-table.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx#L52-L73)
  - [src/assets/styles/index.scss](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L73-L84)
* **Root Causes**:
  - The outer wrapper box of the virtualized proxy groups uses `className="theme-panel"`.
  - In `src/assets/styles/index.scss`, the `.theme-panel` class enforces `border: 4px double var(--theme-border) !important` across all skins/themes.
  - Proxy cell renderers use `borderRight: '5px double var(--theme-border)'` internally for separation borders.
  - Connection tables use `border: '5px double var(--theme-border)'` and `borderBottom: '5px double var(--theme-border)'`.
* **Proposed Diffs**:

#### `src/components/proxy/proxy-groups.tsx`
* **File Location**: [src/components/proxy/proxy-groups.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L782-L792)
```diff
diff --git a/src/components/proxy/proxy-groups.tsx b/src/components/proxy/proxy-groups.tsx
--- a/src/components/proxy/proxy-groups.tsx
+++ b/src/components/proxy/proxy-groups.tsx
@@ -782,10 +782,11 @@
   return (
     <Box
-      className="theme-panel"
       sx={{
         height,
         display: 'flex',
         flexDirection: 'column',
         boxSizing: 'border-box',
         overflow: 'hidden',
+        border: '1px solid var(--theme-border)',
+        borderRadius: '8px',
       }}
     >
```

#### `src/components/proxy/proxy-render.tsx`
* **File Location**: [src/components/proxy/proxy-render.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx#L84-L89)
```diff
diff --git a/src/components/proxy/proxy-render.tsx b/src/components/proxy/proxy-render.tsx
--- a/src/components/proxy/proxy-render.tsx
+++ b/src/components/proxy/proxy-render.tsx
@@ -84,5 +84,5 @@
           ...(idx < (col || 3) - 1
             ? {
-                borderRight: '5px double var(--theme-border)',
+                borderRight: '1px solid var(--theme-border)',
               }
             : {}),
```

#### `src/components/connection/connection-table.tsx`
* **File Location**: [src/components/connection/connection-table.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx#L52-L73)
```diff
diff --git a/src/components/connection/connection-table.tsx b/src/components/connection/connection-table.tsx
--- a/src/components/connection/connection-table.tsx
+++ b/src/components/connection/connection-table.tsx
@@ -52,3 +52,3 @@
-  border: '5px double var(--theme-border)',
+  border: '1px solid var(--theme-border)',
   borderRadius: '4px',
 }
@@ -73,3 +73,3 @@
-  borderBottom: '5px double var(--theme-border)',
+  borderBottom: '1px solid var(--theme-border)',
   backgroundColor: (theme) => theme.palette.background.paper,
```

---

## 5. Plugin Upgrade and Build Script Audit

### 5.1 `tauri-plugin-mihomo` Upgrade Analysis
* **Git history & commit audit**: 
  - [crates/tauri-plugin-mihomo/Cargo.toml](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/Cargo.toml)
  - Commit `579532ad` bumped the local version of `tauri-plugin-mihomo` in `Cargo.toml`/`Cargo.lock` from `0.5.2` to `0.5.4`.
  - The bump was metadata-only. Its purpose was to satisfy validation checks in `verify.py` (which queries upstream versions via GitHub API). Bypassing `v_local < v_upstream` was required once the upstream release moved to `0.5.4`.
  - There are no layout/rendering style hooks introduced or modified in the local Rust code of the plugin in version `0.5.4`.
* **White Screen Connection**:
  - The previous upgrade to `0.5.2` (Commit `3b693842`) resolved a startup white screen bug.
  - Active layout/rendering bugs listed in `bug_list.md` (specifically `BUG-215` and `BUG-203`) reference WebView2 CSP issues blocking external font loading/dynamic styles, which are local frontend problems unrelated to the plugin version.

### 5.2 Patches, Scripts, and Bindings Verification
* **Deleted/Modified Patches**:
  - No build scripts, patches, or batch files were deleted.
  - Lowercase deserialization overrides for model enums (adding `#[ts(export, rename_all = "lowercase")]` / `#[serde(rename_all = "lowercase")]` to `LogLevel` and `FindProcessMode` in [crates/tauri-plugin-mihomo/src/models.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/models.rs#L411-L447)) are intact and correctly applied.
* **Manual Bindings and Compilation commands**:
  - The frontend accesses the plugin locally via a package link (`"tauri-plugin-mihomo-api": "link:./crates/tauri-plugin-mihomo"`).
  - If Rust models or guest-js bindings are modified, bindings must be compiled manually because there are no watchers on the plugin directory.
  - The execution commands to rebuild bindings are:
    ```bash
    # Run tests in the plugin crate to auto-generate TS model definitions
    cargo test --package tauri-plugin-mihomo
    
    # Install dependencies and build guest-js assets in the plugin directory
    cd crates/tauri-plugin-mihomo
    pnpm install
    pnpm build
    ```

---

## 6. Verification Protocol
To verify the audit findings:
1. Confirm that `python verify.py` passes without version warnings (verifies that `0.5.4` is correctly declared).
2. Inspect the lines specified in **Section 2, 3 and 4** for each affected file and confirm that local code matches the pre-patched states.
3. Validate that TypeScript compiler (`pnpm typecheck`) and linter (`pnpm lint`) are clean after applying the code diff proposals.
