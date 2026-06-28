# ClashVerge Post-v1.9.5 Changes Comprehensive Code Review & Audit Report

**Date/Time:** 2026-06-28T17:35:00+08:00  
**Audit Range:** Tag `v1.9.5` (commit `d6fe39ca`) to current HEAD (commit `67b661e9`)  
**Methodology:** Read-only static analysis, commit-by-commit diff inspection, compiler check, and linter check verification. All findings are documented below; no modifications have been made to the workspace codebase.

---

## 📋 Table of Contents
1. [Audited Commits Index](#1-audited-commits-index)
2. [Automated Diagnostics Verification](#2-automated-diagnostics-verification)
3. [Findings Summary by Severity](#3-findings-summary-by-severity)
4. [Detailed Findings & Proposed Gits Diffs](#4-detailed-findings--proposed-gits-diffs)
    - [Major Findings](#major-findings)
    - [Minor Findings](#minor-findings)
    - [Info Findings](#info-findings)
5. [Conclusion & Next Steps](#5-conclusion--next-steps)

---

## 1. Audited Commits Index

A total of 16 commits introduced since tag `v1.9.5` up to current HEAD have been audited:

| # | Commit Hash | Author | Commit Message |
|---|---|---|---|
| 1 | `67b661e9` | qiu-yuxiao | fix: prevent concurrent service reinstall loops in refresh() |
| 2 | `4e16c92f` | qiu-yuxiao | fix: increase column gap in basic settings 2x2 grid |
| 3 | `b36ff876` | qiu-yuxiao | feat: add jsDelivr CDN as fallback updater endpoint |
| 4 | `0f1c5e0e` | qiu-yuxiao | chore: revert app-update.json to v1.9.5 |
| 5 | `f27421db` | qiu-yuxiao | chore: update app-update.json for v1.9.6 [skip ci] |
| 6 | `05115857` | qiu-yuxiao | release: bump version to 1.9.6 |
| 7 | `09d5a3db` | qiu-yuxiao | chore: turn off no-explicit-any eslint rule, fix remaining warnings, remove security report |
| 8 | `8ec1fe7a` | qiu-yuxiao | fix: JS sandbox name embedding + MUI v9 type errors |
| 9 | `5b689a69` | qiu-yuxiao | fix: restore useTranslation after reverting a11y label commit |
| 10 | `cb74520a` | qiu-yuxiao | Revert "fix(a11y): add proper labels to all TextFields replacing hiddenLabel" |
| 11 | `cec42102` | qiu-yuxiao | fix(a11y): improve delay color contrast to meet WCAG 1.4.3 AA |
| 12 | `ad521378` | qiu-yuxiao | fix(a11y): add ARIA list semantics to virtual proxy node list |
| 13 | `67fd30bb` | qiu-yuxiao | fix(a11y): add aria-label to Select components |
| 14 | `a15ec861` | qiu-yuxiao | fix(a11y): add proper labels to all TextFields replacing hiddenLabel |
| 15 | `e5ad370b` | qiu-yuxiao | fix(a11y): add aria-label to all IconButtons for screen reader support |
| 16 | `d6fe39ca` | qiu-yuxiao | chore: update app-update.json for v1.9.5 [skip ci] |

---

## 2. Automated Diagnostics Verification

Static analysis and verification tools were executed on current HEAD to detect regressions, lint warnings, or compile errors:

- **Frontend Typechecker (`tsc --noEmit`)**: **PASS**
  - No type errors or TypeScript compilation failures were found in the frontend.
- **Frontend Linter (`eslint`)**: **PASS**
  - Completed successfully with 0 warnings and 0 errors.
- **Frontend Formatter (`biome format`)**: **WARNING**
  - Biome flagged minor line-ending and trailing-newline mismatches on `package.json` and `updater/app-update.json` but otherwise completed successfully.
- **Backend Cargo Check (`cargo check --workspace`)**: **PASS**
  - Checked all crates in the workspace (`clash-mini`, `tauri-plugin-mihomo`, `clash-verge-logging`, etc.). Completed successfully with 0 errors or warnings.
- **Backend Cargo Clippy (`cargo clippy --workspace`)**: **PASS**
  - Successfully linted all crates; reported 0 warnings and 0 errors.

---

## 3. Findings Summary by Severity

### 🔴 Critical (Must Fix)
- *None.* No immediate crashes, critical remote execution vulnerabilities, or data loss vectors were introduced in this commit range.

### 🟡 Major (Bugs & Structural Vulnerabilities)
1. **Global Mutex Lock Held During Heavy Asynchronous Operation** (`src-tauri/src/core/service.rs`)
   - The service `refresh()` method holds the global `SERVICE_MANAGER` Mutex lock during the entire asynchronous installer wait (including UAC prompt latency), which freezes other threads trying to inspect service status.
2. **Hardcoded Chinese Strings in International Locale Files** (multiple files under `src/locales/`)
   - Accessibility strings added in commit `e5ad370b` (e.g. `"置顶窗口"`, `"打开设置"`) were hardcoded in Chinese for all languages (Russian, German, Korean, etc.), breaking screen reader usage for international users.
3. **Accessibility Headless Form Controls After TextField Revert** (`src/pages/_layout/components/layout-dialogs.tsx`)
   - Reverting the TextField label migration in `cb74520a` broke programmatic associations between `<Typography>` label elements and `<TextField>` inputs, violating WCAG 3.3.2.
4. **Updater CDN Fallback Limitation via GitHub Hardcoded Assets** (`src-tauri/tauri.conf.json` & `updater/app-update.json`)
   - While adding a jsDelivr fallback endpoints is helpful, the fetched `app-update.json` still points to GitHub Releases. If GitHub is blocked, the download will still fail. Also, jsDelivr branch references cache for up to 12 hours.

### 🟢 Minor (Code Quality, Accessibility, Style)
1. **Brittle MUI v9 Type Suppressions via @ts-expect-error** (`src/pages/_layout.tsx`)
   - Disabling TypeScript errors directly with `@ts-expect-error` risks future compilation breakage when packages are updated.
2. **Contrast Color Risk on Dark Backgrounds in Dark Mode** (`src/services/delay.ts` & `src/components/proxy/proxy-head.tsx`)
   - Migrating colors from `warning.main` to `warning.dark` to meet contrast checks against light backgrounds degrades accessibility contrast on dark backgrounds in dark mode.
3. **ESLint Global Rule Disabling** (`eslint.config.ts`)
   - Turning off the `no-explicit-any` ESLint rule globally weakens TypeScript validation, hiding potential type safety regressions.

### 🔵 Info (Chore & Code Style Suggestions)
1. **Biome Line Ending Mismatches** (`package.json`, `updater/app-update.json`)
   - Carriage returns (CRLF) and missing trailing newlines are present in the package files.
2. **Updater JSON Chore Commits**
   - Several commits bumps (`0f1c5e0e`, `f27421db`, `05115857`) represent correct versioning updates.

---

## 4. Detailed Findings & Proposed Gits Diffs

### Major Findings

#### 1. Global Mutex Lock Held During Heavy Asynchronous Operation
- **File Path**: `[service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/service.rs#L540)`
- **Analysis**: 
  In commit `67b661e9`, the `refresh` method was modified to await the asynchronous `reinstall_service` task:
  ```rust
  let result = tokio::task::spawn_blocking(reinstall_service).await;
  ```
  However, `refresh(&mut self)` is an async method on the `ServiceManager` struct. Any caller locks the global static `Lazy<Mutex<ServiceManager>>` wrapper (`SERVICE_MANAGER`) before calling it (e.g. in `run_core_by_service`, `init_service_manager`, etc.). Holding this lock across the heavy `spawn_blocking` call blocks other async tasks (like tray updates or status checks) for the entire duration of the service installation, which involves waiting for user UAC elevation approval.
- **Proposed Fix**: 
  Make `refresh` a static async function that locks the `SERVICE_MANAGER` internally only when writing state, similar to the pattern implemented in `handle_service_operation`. This releases the lock during the heavy installer invocation.
- **Proposed Git Diff**:
```rust
diff --git a/src-tauri/src/core/service.rs b/src-tauri/src/core/service.rs
index cb5c3b39..ef1a2b3c 100644
--- a/src-tauri/src/core/service.rs
+++ b/src-tauri/src/core/service.rs
@@ -536,25 +536,36 @@ impl ServiceManager {
     pub fn current(&self) -> ServiceStatus {
         self.0.clone()
     }
 
-    pub async fn refresh(&mut self) -> Result<()> {
-        let status = self.check_service_comprehensive().await;
+    // Acquire Mutex locks internally to release it during the spawn_blocking task
+    pub async fn refresh() -> Result<()> {
+        let status = {
+            let manager = SERVICE_MANAGER.lock().await;
+            manager.check_service_comprehensive().await
+        };
+
         if matches!(status, ServiceStatus::NeedsReinstall | ServiceStatus::ReinstallRequired) {
-            // 先更新状态为"重装中"，防止重复触发
-            self.0 = ServiceStatus::Reinstalling;
-            // 等待重装完成，避免 fire-and-forget 导致并发重装
+            {
+                let mut manager = SERVICE_MANAGER.lock().await;
+                manager.0 = ServiceStatus::Reinstalling;
+            }
+
             let result = tokio::task::spawn_blocking(reinstall_service).await;
+
+            let mut manager = SERVICE_MANAGER.lock().await;
             match result {
                 Ok(Ok(())) => {
-                    // 重装成功，重新检查状态
-                    let new_status = self.check_service_comprehensive().await;
-                    self.0 = new_status.clone();
-                    logging_error!(Type::Service, self.handle_service_status(&new_status).await);
+                    let new_status = manager.check_service_comprehensive().await;
+                    manager.0 = new_status.clone();
+                    logging_error!(Type::Service, manager.handle_service_status(&new_status).await);
                 }
                 Ok(Err(e)) => {
                     logging!(error, Type::Service, "重装服务失败: {}", e);
-                    self.0 = ServiceStatus::NeedsReinstall;
+                    manager.0 = ServiceStatus::NeedsReinstall;
                 }
                 Err(e) => {
                     logging!(error, Type::Service, "重装服务任务失败: {}", e);
-                    self.0 = ServiceStatus::NeedsReinstall;
+                    manager.0 = ServiceStatus::NeedsReinstall;
                 }
             }
         } else {
-            self.0 = status.clone();
-            logging_error!(Type::Service, self.handle_service_status(&status).await);
+            let mut manager = SERVICE_MANAGER.lock().await;
+            manager.0 = status.clone();
+            logging_error!(Type::Service, manager.handle_service_status(&status).await);
         }
         Ok(())
     }
```

---

#### 2. Hardcoded Chinese Strings in International Locale Files
- **File Paths**: 
  - `[ar/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/ar/layout.json#L24)`
  - `[de/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/de/layout.json#L24)`
  - `[es/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/es/layout.json#L24)`
  - `[fa/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/fa/layout.json#L24)`
  - `[id/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/id/layout.json#L24)`
  - `[jp/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/jp/layout.json#L24)`
  - `[ko/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/ko/layout.json#L24)`
  - `[ru/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/ru/layout.json#L24)`
  - `[tr/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/tr/layout.json#L24)`
  - `[tt/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/tt/layout.json#L24)`
  - `[zhtw/layout.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/locales/zhtw/layout.json#L24)`
- **Analysis**:
  In commit `e5ad370b` (which added accessibility labels for screen readers), the localization blocks in non-Chinese translation files were populated by copy-pasting the Chinese template strings verbatim. Consequently, screen readers used by German, Russian, Arabic, or Japanese users will read out Chinese texts like `"置顶窗口"`, `"打开设置"`, and `"关闭设置"`.
- **Proposed Fix**:
  Translate the `"a11y"` block values into their respective languages to match the structure defined in `en/layout.json`.
- **Proposed Git Diff (Example for `ru/layout.json`)**:
```json
diff --git a/src/locales/ru/layout.json b/src/locales/ru/layout.json
index f76b4cde..c76891ab 100644
--- a/src/locales/ru/layout.json
+++ b/src/locales/ru/layout.json
@@ -24,7 +24,9 @@
   "a11y": {
-    "pinWindow": "置顶窗口",
-    "openSettings": "打开设置",
-    "closeSettings": "关闭设置",
-    "closeLogsDialog": "关闭日志对话框"
+    "pinWindow": "Закрепить окно сверху",
+    "unpinWindow": "Открепить окно",
+    "openSettings": "Открыть настройки",
+    "closeSettings": "Закрыть настройки",
+    "closeLogsDialog": "Закрыть диалог логов",
+    "selectLanguage": "Выбрать язык"
   }
 }
```

---

#### 3. Accessibility Headless Form Controls After TextField Revert
- **File Path**: `[layout-dialogs.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/layout-dialogs.tsx#L151)`
- **Analysis**:
  Commit `cb74520a` reverted the TextField label migration to preserve the 3D visual style of the edit profile dialog. While this restored the custom `<Typography>` styling, it stripped the `label` prop from the `<TextField>` component. Because the `<Typography>` labels are plain text elements with no programmatic link to the `<TextField>` components, screen readers will treat the fields as unlabeled textboxes, failing WCAG 3.3.2 (Labels or Instructions) and WCAG 1.3.1 (Info and Relationships).
- **Proposed Fix**:
  Keep the 3D-styled typography labels, but programmatically associate them by assigning unique `id` values to the `<Typography>` components and setting `aria-labelledby` on the `<TextField>` components.
- **Proposed Git Diff**:
```tsx
diff --git a/src/pages/_layout/components/layout-dialogs.tsx b/src/pages/_layout/components/layout-dialogs.tsx
index 706abca2..0a6b11bc 100644
--- a/src/pages/_layout/components/layout-dialogs.tsx
+++ b/src/pages/_layout/components/layout-dialogs.tsx
@@ -148,15 +148,17 @@ export const LayoutDialogs: React.FC<LayoutDialogsProps> = ({
         <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mb: 3 }}>
           <Box>
             <Typography
+              id="profile-name-label"
               variant="body2"
               sx={{
                 mb: 0.5,
                 fontWeight: 'bold',
                 fontFamily: 'var(--control-font-family)',
               }}
             >
               配置名称
             </Typography>
             <TextField
+              aria-labelledby="profile-name-label"
               fullWidth
               size="small"
               value={editProfileName}
@@ -171,15 +173,17 @@ export const LayoutDialogs: React.FC<LayoutDialogsProps> = ({
           <Box>
             <Typography
+              id="profile-url-label"
               variant="body2"
               sx={{
                 mb: 0.5,
                 fontWeight: 'bold',
                 fontFamily: 'var(--control-font-family)',
               }}
             >
               订阅地址
             </Typography>
             <TextField
+              aria-labelledby="profile-url-label"
               fullWidth
               size="small"
               value={editProfileUrl}
@@ -192,15 +196,17 @@ export const LayoutDialogs: React.FC<LayoutDialogsProps> = ({
           <Box>
             <Typography
+              id="profile-interval-label"
               variant="body2"
               sx={{
                 mb: 0.5,
                 fontWeight: 'bold',
                 fontFamily: 'var(--control-font-family)',
               }}
             >
               更新周期 (单位: 小时, 设为 0 禁用)
             </Typography>
             <TextField
+              aria-labelledby="profile-interval-label"
               fullWidth
               size="small"
               type="number"
```

---

#### 4. Updater CDN Fallback Limitation via GitHub Hardcoded Assets
- **File Path**: `[tauri.conf.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.conf.json#L41)` & `[app-update.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/updater/app-update.json#L8)`
- **Analysis**:
  Commit `b36ff876` introduces `https://cdn.jsdelivr.net/gh/qiu-yuxiao/clash-mini@dev/updater/app-update.json` as a fallback. However:
  1. **CDN Latency**: Branch references on jsDelivr cache for up to 12 hours. Using `@dev` means clients will fetch stale updates.
  2. **Hardcoded GitHub Links**: The downloaded update manifest (`app-update.json`) contains a hardcoded URL pointing to GitHub releases:
     `"url": "https://github.com/qiu-yuxiao/clash-mini/releases/download/..."`
     If GitHub is blocked in the user's region (which is the primary reason for adding CDN fallbacks), downloading the manifest succeeds but the installer download itself will fail.
- **Proposed Fix**:
  Use a versioned tag or serverless function to avoid caching issues, and consider writing a custom updater parser on the client side that replaces the download URL with a mirror link (e.g. FastGit, jsDelivr releases, or custom proxy) if the primary GitHub connection fails.

---

### Minor Findings

#### 1. Brittle MUI v9 Type Suppressions via `@ts-expect-error`
- **File Path**: `[_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L2009)`
- **Analysis**:
  Commit `8ec1fe7a` added `// @ts-expect-error MUI v9 aria-label type issue` above `<Select>` components. TypeScript raises errors if a `@ts-expect-error` directive ceases to suppress an error. This will break compilation if the underlying MUI types are updated in the future.
- **Proposed Fix**:
  Pass accessibility attributes via `inputProps` (which targets the underlying DOM element) rather than directly onto the wrapper `Select` component.
- **Proposed Git Diff**:
```tsx
diff --git a/src/pages/_layout.tsx b/src/pages/_layout.tsx
index e01d6a80..2e10f44d 100644
--- a/src/pages/_layout.tsx
+++ b/src/pages/_layout.tsx
@@ -2008,4 +2008,3 @@ const Layout = () => {
                 displayEmpty
-                // @ts-expect-error MUI v9 aria-label type issue
-                aria-label={t('layout.a11y.selectLanguage')}
+                inputProps={{ 'aria-label': t('layout.a11y.selectLanguage') }}
                 renderValue={() => 'Language'}
```

---

#### 2. Contrast Color Risk on Dark Backgrounds in Dark Mode
- **File Paths**: 
  - `[delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L388)`
  - `[proxy-head.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-head.tsx#L122)`
- **Analysis**:
  In commit `cec42102`, delay color mappings were changed from `warning.main` to `warning.dark` to satisfy WCAG 1.4.3 minimums (4.5:1 ratio) on light backgrounds. However, `warning.dark` is a deep orange/brown shade which reduces the contrast ratio against dark backgrounds. In dark mode, this can drop the contrast ratio below the WCAG 1.4.3 AA minimum.
- **Proposed Fix**:
  Use theme-aware color mapping instead of a static color. Retrieve the current theme mode and return `warning.main` for dark mode and `warning.dark` for light mode.
- **Proposed Git Diff**:
```typescript
diff --git a/src/services/delay.ts b/src/services/delay.ts
index e9ca0a81..b82bc0e1 100644
--- a/src/services/delay.ts
+++ b/src/services/delay.ts
@@ -384,7 +384,7 @@ export class DelayManager {
   formatDelayColor(delay: number, timeout = 10000, isDarkMode = false) {
     if (delay < 0) return ''
     if (delay === 0 || delay >= timeout) return 'error.main'
     if (delay >= 10000) return 'error.main'
-    if (delay >= 400) return 'warning.dark'
+    if (delay >= 400) return isDarkMode ? 'warning.main' : 'warning.dark'
     if (delay >= 250) return 'primary.main'
     return 'success.main'
   }
```

---

#### 3. ESLint Global Rule Disabling
- **File Path**: `[eslint.config.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/eslint.config.ts#L52)`
- **Analysis**:
  Commit `09d5a3db` disabled the `no-explicit-any` ESLint rule globally. This compromises type safety by allowing developers to use `any` without lint checks, potentially letting type bugs bypass static verification.
- **Proposed Fix**:
  Re-enable `no-explicit-any` globally, and use inline comments (`/* eslint-disable-next-line @typescript-eslint/no-explicit-any */`) only for files where using `any` is strictly necessary (e.g. low-level bridge commands or YAML parsers).

---

### Info Findings

#### 1. Biome Line Ending Mismatches
- **Analysis**:
  Biome formatter checks output minor warnings concerning trailing newlines and CRLF (carriage return) line endings in package configuration files:
  - `package.json`
  - `updater/app-update.json`
- **Proposed Fix**:
  Configure `.editorconfig` or run `npx biome format --write package.json updater/app-update.json` to unify line endings.

#### 2. Updater JSON Chore Commits
- **Analysis**:
  Commits `0f1c5e0e`, `f27421db`, `05115857`, and `d6fe39ca` are version chore and releases updates. The changes in `app-update.json` are correct and present no logic issues.

---

## 5. Conclusion & Next Steps

The post-v1.9.5 changes are highly targeted and generally improve accessibility and loop preventions in the application. However, resolving the **global Mutex lock blocking** in `service.rs`, correcting the **hardcoded Chinese translations**, and securing **unlabeled form textfields** are important next steps to elevate the software to production-ready status.

It is recommended to apply the proposed git diffs sequentially in the next development cycle to resolve these architectural and accessibility gaps.
