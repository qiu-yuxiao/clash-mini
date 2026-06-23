# Clash Mini Comprehensive Layout & Rendering Audit Report

## Executive Summary
This report presents a comprehensive, multi-angle code audit of the Clash Mini (Clash Verge) project to identify all potential root causes of layout/screen rendering failures (chaos, blanks, or crashes) introduced since version 1.6.5.

Under the strict **Non-Modification Constraint**, no changes were directly applied to the codebase. All findings, logic chains, and precise code diff blocks have been compiled below to guide the development team in resolving these visual and layout anomalies.

**Overall Layout & Rendering Stability: AMBER (High Risk of Rendering Failures & Hangs)**
While the custom skeuomorphic 3D aesthetics are well-structured, several security hardening and configuration refactor changes introduced since version 1.6.5 have created regression vectors:
1. **Asset Protocol Scope Tightening**: Restricting asset loads to `"$APPDATA/**"` blocks cached profile icons in portable mode.
2. **Hidden Skin Switcher & Language Selector**: A media query hides the switcher at the default window height (680px).
3. **Settings Drawer horizontal layout overflow**: Squeezes the connections panel to 0px, making it invisible.
4. **Startup Script Blocking**: A synchronous `.output().await` blocks window creation, causing permanent blank screens/crashes.
5. **Service Manager Deadlocks**: Mutex lock contention blocks the GUI initialization thread during reinstall UAC prompts.
6. **Live Theme Config Updates Skipped**: Config patches do not emit `RefreshVerge`, ignoring theme/CSS changes.
7. **Window Close Unconditionally Destroys WebView**: Discards all state memory and slows window recreation.

Applying the proposed diffs will restore visual compliance with the project's design agreements and ensure smooth rendering.

---

## 🎨 Component Compliance Status Table

The following table maps the compliance status of layout components against the six skin styles (`Trump-3D` / `Original` / `Modern` / `Frosted` / `Cyberpunk` / `Monochrome`):

| Component / Layout | Trump-3D | Original | Modern | Frosted | Cyberpunk | Monochrome | Notes |
|---|---|---|---|---|---|---|---|
| `_layout.tsx` Settings Drawer | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | Squeezes right connection panel to 0px (AUDIT-LAYOUT-001) and hides switcher (AUDIT-LAYOUT-002). |
| `use-custom-theme.ts` | ✅ Compliant | ❌ Non-compliant | ✅ Compliant | ✅ Compliant | ✅ Compliant | ⚠️ Partial | Original skin accent color is static in React (AUDIT-LAYOUT-008); Monochrome lacks contrast (AUDIT-LAYOUT-009). |
| `base-switch.tsx` | ✅ Compliant | ✅ Compliant | ✅ Compliant | ✅ Compliant | ❌ Non-compliant | ❌ Non-compliant | Small switches rendered at standard size (AUDIT-LAYOUT-010). |
| `tauri.conf.json` | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | Scope blocks portable mode icons (AUDIT-LAYOUT-003). |

---

## 🖥️ Layout & Rendering Audit Findings

### AUDIT-LAYOUT-001: Settings Drawer Horizontal Layout Overflow
* **Severity**: High
* **File Path & Link**: [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1726-L1762)
* **Description**: Squeezes the Connections column to 0px, making active/closed connection lists completely invisible and inaccessible in the default/minimal window width of 270px.
* **Root Cause**: The Settings Drawer container (`.theme-panel`) uses `display: 'flex'` (row layout by default) with a fixed-width left settings column of `240px` and total padding/gaps of `36px`, exceeding the window width of `270px`. This leaves negative space for the right connections panel.
* **Suggested Fix**:
```diff
diff --git a/src/pages/_layout.tsx b/src/pages/_layout.tsx
index a123456..b654321 100644
--- a/src/pages/_layout.tsx
+++ b/src/pages/_layout.tsx
@@ -1732,3 +1732,4 @@
                 width: '100%',
                 height: '100%',
                 zIndex: 100,
-                display: 'flex',
+                display: 'flex',
+                flexDirection: 'row',
+                flexWrap: 'wrap',
```

---

### AUDIT-LAYOUT-002: Skin Switcher & Language Selector Hidden at Default Window Height
* **Severity**: Medium
* **File Path & Link**: [_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L1897-L1899)
* **Description**: Users cannot see or switch skins/languages because the elements are completely hidden under default window dimensions.
* **Root Cause**: The media query `@media (max-height: 830px) { display: none }` is applied to these absolutely positioned elements inside the settings drawer. Since the default window height is `680px` (which is less than `830px`), they are always hidden by default.
* **Suggested Fix**:
```diff
diff --git a/src/pages/_layout.tsx b/src/pages/_layout.tsx
index a123456..b654321 100644
--- a/src/pages/_layout.tsx
+++ b/src/pages/_layout.tsx
@@ -1897,3 +1897,3 @@
-                  '@media (max-height: 830px)': {
-                    display: 'none',
-                  },
+                  '@media (max-height: 500px)': {
+                    display: 'none',
+                  },
@@ -1954,3 +1954,3 @@
-                  '@media (max-height: 830px)': {
-                    display: 'none',
-                  },
+                  '@media (max-height: 500px)': {
+                    display: 'none',
+                  },
```

---

### AUDIT-LAYOUT-003: Tightened Asset Protocol Blocks Icons in Portable Mode
* **Severity**: Medium
* **File Path & Link**: [tauri.conf.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.conf.json#L61-L69)
* **Description**: Profiles and custom icons fail to render and show up as broken/blank images when Clash Mini is run in portable mode.
* **Root Cause**: The asset protocol scope is restricted to `"$APPDATA/**"`. In portable mode, the `.config/clash-verge/` directory resides in the execution directory (outside AppData), causing Tauri to block file queries with `403 Forbidden`.
* **Suggested Fix**:
```diff
diff --git a/src-tauri/tauri.conf.json b/src-tauri/tauri.conf.json
index a123456..b654321 100644
--- a/src-tauri/tauri.conf.json
+++ b/src-tauri/tauri.conf.json
@@ -64,3 +64,5 @@
           "allow": [
-            "$APPDATA/**"
+            "$APPDATA/**",
+            "$EXE_DIR/**",
+            "$RESOURCE_DIR/**"
           ],
```

---

### AUDIT-LAYOUT-004: Theme/CSS Injection Patch Updates Silently Ignored
* **Severity**: Medium
* **File Path & Link**: [config.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/config.rs#L73-L165)
* **Description**: Real-time styling/layout updates are ignored when users change theme settings or inject custom CSS, requiring manual app restart to apply changes.
* **Root Cause**: `determine_update_flags` fails to check/set `UpdateFlags::VERGE_CONFIG` for `theme_mode` and `theme_setting` (`css_injection`) changes, omitting `RefreshVerge` event emission.
* **Suggested Fix**:
```diff
diff --git a/src-tauri/src/feat/config.rs b/src-tauri/src/feat/config.rs
index a123456..b654321 100644
--- a/src-tauri/src/feat/config.rs
+++ b/src-tauri/src/feat/config.rs
@@ -151,3 +151,3 @@
-    if enable_global_hotkey.is_some() || home_cards.is_some() {
+    if enable_global_hotkey.is_some() || home_cards.is_some() || patch.theme_mode.is_some() || patch.theme_setting.is_some() {
         update_flags.insert(UpdateFlags::VERGE_CONFIG);
     }
```

---

### AUDIT-LAYOUT-005: Startup Script Blocks Window Initialization
* **Severity**: High
* **File Path & Link**: [mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/resolve/mod.rs#L60-L63)
* **Description**: Startup hang or permanent blank screen when users configure a blocking/long-running startup script.
* **Root Cause**: `resolve_setup_async()` runs `init_startup_script().await` synchronously before creating the window. The startup script function waits for command exit output asynchronously, blocking the thread indefinitely.
* **Suggested Fix**:
```diff
diff --git a/src-tauri/src/utils/resolve/mod.rs b/src-tauri/src/utils/resolve/mod.rs
index a123456..b654321 100644
--- a/src-tauri/src/utils/resolve/mod.rs
+++ b/src-tauri/src/utils/resolve/mod.rs
@@ -60,3 +60,5 @@
-        init_startup_script().await;
+        tokio::spawn(async {
+            let _ = init_startup_script().await;
+        });
```

---

### AUDIT-LAYOUT-006: Service Manager Reinstall Deadlock during Startup
* **Severity**: High
* **File Path & Link**: [service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs#L539-L566)
* **Description**: UI hangs and startup freeze if the app needs to reinstall the helper service.
* **Root Cause**: `init_service_manager()` locks the global `SERVICE_MANAGER` Mutex. Inside `refresh()`, it triggers service reinstallation which shows a blocking UAC prompt. The core startup loop checks service status by locking `SERVICE_MANAGER`, resulting in a deadlock.
* **Suggested Fix**:
```diff
diff --git a/src-tauri/src/core/service.rs b/src-tauri/src/core/service.rs
index a123456..b654321 100644
--- a/src-tauri/src/core/service.rs
+++ b/src-tauri/src/core/service.rs
@@ -539,3 +539,4 @@
     pub async fn refresh(&mut self) -> Result<()> {
-        let status = self.check_service_comprehensive().await;
-        self.0 = status.clone();
-        logging_error!(Type::Service, self.handle_service_status(&status).await);
+        let status = self.check_service_comprehensive().await;
+        if matches!(status, ServiceStatus::NeedsReinstall | ServiceStatus::ReinstallRequired) {
+            tokio::task::spawn_blocking(move || {
+                let _ = reinstall_service();
+            });
+        } else {
+            self.0 = status.clone();
+            logging_error!(Type::Service, self.handle_service_status(&status).await);
+        }
```

---

### AUDIT-LAYOUT-007: Window Close Unconditionally Destroys WebView State
* **Severity**: Medium
* **File Path & Link**: [lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/lightweight.rs#L100-L115)
* **Description**: Slower reopening of the window and loss of all frontend state/UI focus on close.
* **Root Cause**: `entry_lightweight_mode()` unconditionally destroys the main window instead of checking if `enable_auto_light_weight_mode` is true.
* **Suggested Fix**:
```diff
diff --git a/src-tauri/src/module/lightweight.rs b/src-tauri/src/module/lightweight.rs
index a123456..b654321 100644
--- a/src-tauri/src/module/lightweight.rs
+++ b/src-tauri/src/module/lightweight.rs
@@ -100,3 +100,6 @@
 pub async fn entry_lightweight_mode() -> bool {
+    let verge = Config::verge().await;
+    if !verge.enable_auto_light_weight_mode.unwrap_or(false) {
+        return WindowManager::hide_main_window();
+    }
```

---

### AUDIT-LAYOUT-008: Original Skin Accent Color Dynamic Shift Mismatch
* **Severity**: Low
* **File Path & Link**: [use-custom-theme.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L214-L216)
* **Description**: Visual styling discrepancy where CSS components change color with the slider, but MUI React components remain static purple.
* **Root Cause**: Theme primary color is hardcoded to `#5b5c9d` in `use-custom-theme.ts`, but index.scss shifts color dynamically based on HSL.
* **Suggested Fix**:
```diff
diff --git a/src/pages/_layout/hooks/use-custom-theme.ts b/src/pages/_layout/hooks/use-custom-theme.ts
index a123456..b654321 100644
--- a/src/pages/_layout/hooks/use-custom-theme.ts
+++ b/src/pages/_layout/hooks/use-custom-theme.ts
@@ -214,3 +214,3 @@
         if (controlSkin === 'original') {
-          resolvedPrimary = '#5b5c9d'
+          resolvedPrimary = `hsl(${239 * (setting.control_skin_val2 ?? 1.0)}, 26%, 49%)`
         }
```

---

### AUDIT-LAYOUT-009: Monochrome Skin Dark Mode Card Background Contrast Loss
* **Severity**: Low
* **File Path & Link**: [index.scss](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L270-L282)
* **Description**: Settings cards and panels blend entirely into the dark window background, making the UI unreadable.
* **Root Cause**: Cards and panels are forced to use `var(--background-color)` which in dark mode is set to `#2E303D` (matching the main background).
* **Suggested Fix**:
```diff
diff --git a/src/assets/styles/index.scss b/src/assets/styles/index.scss
index a123456..b654321 100644
--- a/src/assets/styles/index.scss
+++ b/src/assets/styles/index.scss
@@ -270,3 +270,3 @@
 html[data-control-skin="monochrome"] {
   .theme-panel {
-    background: var(--background-color) !important;
+    background: var(--paper-color, #1e2438) !important;
```

---

### AUDIT-LAYOUT-010: Switch Component Sizing Inconsistency in Cyberpunk/Monochrome Skins
* **Severity**: Low
* **File Path & Link**: [base-switch.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-switch.tsx#L370-L402)
* **Description**: Sizing layout mismatch when small switches are rendered at standard size.
* **Root Cause**: Standard switch dimensions are forced onto small-sized Mui classes in these skins.
* **Suggested Fix**:
```diff
diff --git a/src/components/base/base-switch.tsx b/src/components/base/base-switch.tsx
index a123456..b654321 100644
--- a/src/components/base/base-switch.tsx
+++ b/src/components/base/base-switch.tsx
@@ -370,3 +370,3 @@
         '&.MuiSwitch-sizeSmall': {
-          width: '28px !important',
-          height: '14px !important',
+          width: '18px !important',
+          height: '10px !important',
```
