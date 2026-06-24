# Layout and Styling Audit Report

## 1. Observation
This read-only audit investigated styling, themes, scrollbars, and layout configurations under different screen scaling, viewport boundaries, and WebView2 runtime environments. Below are the key findings observed in the codebase:

### Finding 1: CSS Specificity Conflict on Dialog/Menu/Popover/Autocomplete Backgrounds under Frosted Glass Skin
* **Location**:
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L109-L115`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L184-L190`
* **Direct Code Observation**:
  Lines 109–115:
  ```scss
  .MuiDialog-paper.theme-panel,
  .MuiMenu-paper.theme-panel,
  .MuiPopover-paper.theme-panel,
  .MuiAutocomplete-paper.theme-panel {
    background: rgb(var(--theme-panel-base-rgb, 255, 255, 255)) !important;
    opacity: 1 !important;
  }
  ```
  Lines 184–190:
  ```scss
  html[data-control-skin="frosted-glass"] {
    .theme-panel {
      background: rgba(var(--theme-panel-base-rgb, 255, 255, 255), calc(0.02 + 0.1 * var(--depth-factor, 1.0))) !important;
      backdrop-filter: blur(calc(16px * var(--vibrancy-factor, 1.0))) !important;
      border: 4px double rgba(var(--theme-glass-border-rgb, 255, 255, 255), 0.15) !important;
    }
  ```

### Finding 2: Hardcoded Opaque Background Colors Overriding Theme Skins
* **Locations**:
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-page.tsx#L37`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/base/base-page.tsx#L41`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/unlock.tsx#L380`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L780`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/layout-dialogs.tsx#L536`
* **Direct Code Observation**:
  * `base-page.tsx` (Lines 37, 41):
    ```tsx
    style={{ backgroundColor: isDark ? '#1e1f27' : '#ffffff' }}
    style={{ backgroundColor: isDark ? '#1e1f27' : 'var(--background-color)' }}
    ```
  * `unlock.tsx` (Line 380):
    ```tsx
    backgroundColor: isDark ? '#282a36' : '#ffffff',
    ```
  * `proxy-groups.tsx` (Line 780):
    ```tsx
    const stickyBackground = theme.palette.mode === 'dark' ? '#1e1f27' : 'var(--background-color)'
    ```
  * `layout-dialogs.tsx` (Line 535–536):
    ```tsx
    background: mode === 'light' ? 'var(--background-color)' : '#1e1f27',
    ```

### Finding 3: Scrollbar Hiding Overrides and Layout Conflict in Standard Scrollbar Property
* **Locations**:
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/connection/connection-table.tsx#L45`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L589`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/index.scss#L34`
* **Direct Code Observation**:
  * `connection-table.tsx` (Line 45):
    ```tsx
    scrollbarWidth: 'none',
    ```
  * `use-custom-theme.ts` (Line 589):
    ```scss
    * { scrollbar-width: thin !important; }
    ```
  * `index.scss` (Line 34):
    ```scss
    * { scrollbar-width: thin !important; }
    ```

### Finding 4: Inactive Scrollbar Variables Set in CSS but Unused
* **Locations**:
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L430-L437`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/hooks/use-custom-theme.ts#L591-L605`
* **Direct Code Observation**:
  In `use-custom-theme.ts` (Lines 430–437), the properties `--scrollbar-bg` and `--scrollbar-thumb` are set:
  ```tsx
  rootEle.style.setProperty('--scrollbar-bg', mode === 'light' ? '#f1f1f1' : '#2E303D')
  rootEle.style.setProperty('--scrollbar-thumb', mode === 'light' ? '#c1c1c1' : '#555555')
  ```
  However, the injected scrollbar styles (Lines 591–605) use:
  ```scss
  ::-webkit-scrollbar-track {
    background: transparent !important;
  }
  ::-webkit-scrollbar-thumb {
    background: var(--primary-main, #1976d2) !important;
    ...
  }
  ```

### Finding 5: Vertical Clipping Hazard in Compact Proxy Columns under High Font Scaling
* **Locations**:
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-render.tsx#L239`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-item.tsx#L101-L102`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/proxy/proxy-groups.tsx#L198-L205`
* **Direct Code Observation**:
  * `proxy-render.tsx` (Line 239):
    ```tsx
    height: '20px',
    ```
  * `proxy-item.tsx` (Lines 101–102):
    ```tsx
    height: '20px',
    minHeight: '20px',
    ```
  * `proxy-groups.tsx` (Lines 198–205):
    ```tsx
    estimateSize: (index) => {
      const item = filteredRenderList[index]
      if (item?.type === 0) return 56
      if (item?.type === 2) return 20
      if (item?.type === 3) return 80
      if (item?.type === 4) return 20
      return 56
    },
    ```

### Finding 6: Navigation Menu Scrollbar Truncation
* **Location**:
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/layout.scss#L89-L94`
  * `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/assets/styles/layout.scss#L275-L279`
* **Direct Code Observation**:
  Lines 89–94:
  ```scss
        scrollbar-width: none;

        &::-webkit-scrollbar {
          width: 0;
          height: 0;
        }
  ```

---

## 2. Logic Chain

### Finding 1: Specificity Conflict
1. The global opaque safety rule for dialogs (`.MuiDialog-paper.theme-panel`) uses two class selectors (specificity `0, 2, 0` or 20).
2. The skin-specific rule `html[data-control-skin="frosted-glass"] .theme-panel` uses an attribute selector, a class selector, and a tag selector (specificity `0, 2, 1` or 21).
3. Because 21 > 20, the skin's translucent background property takes priority over the dialog's opaque safety rule.
4. Consequently, dialog papers in Frosted Glass mode are rendered with high transparency, allowing text underneath to bleed through, creating overlaps and rendering anomalies.

### Finding 2: Hardcoded Backgrounds
1. Custom skins (e.g., Frosted Glass, Cyberpunk) specify unique background colors (or transparencies) via CSS variables such as `--theme-panel-bg` or `--theme-bg`.
2. Several React components (`BasePage`, `UnlockPage` cards, `ProxyVirtualList` sticky headers, Logs dialog) override these variables in inline `style` or `sx` props by hardcoding colors like `#1e1f27` or `#282a36` when `theme.palette.mode === 'dark'`.
3. As a result, when these custom skins are activated in dark mode, these components fail to adopt the skin styles (e.g. glass transparency or cyberpunk dark tones) and instead leak `#1e1f27` / `#282a36` blocks, breaking theme coherence.

### Finding 3: Connection Table Scrollbar Leak
1. `index.scss` and `use-custom-theme.ts` both apply `* { scrollbar-width: thin !important; }` globally.
2. The connection table scroll container in `connection-table.tsx` attempts to hide scrollbars using `scrollbarWidth: 'none'`.
3. In WebView2 (or standard Web browsers) supporting the standard `scrollbar-width` property, the global rule (which has `!important`) overrides the table's inline `scrollbarWidth: 'none'` (which lacks `!important`).
4. This results in an unintended thin scrollbar displaying in the connection table list.

### Finding 4: Dead Code in Scrollbar Color Variables
1. `--scrollbar-bg` and `--scrollbar-thumb` are dynamically configured in JS based on theme settings.
2. However, the injected CSS rules target `::-webkit-scrollbar-thumb` and track with hardcoded colors (`transparent`, `var(--primary-main)`).
3. Since the CSS variables are never actually used in the CSS definitions, customization settings targeting scrollbar aesthetics are ignored.

### Finding 5: Vertical Clipping under Scaling
1. Proxy row items and list elements are styled with a locked height of `20px` and `overflow: hidden`.
2. The virtualization height estimation is also hardcoded to `20` in JS.
3. Under high OS text scaling (e.g. 150%) or custom system fonts, a 12px font scales up and exceeds the 20px limit.
4. Due to `overflow: hidden` and absolute height limits, text and badge contents are clipped vertically.

### Finding 6: Navigation Menu Scrollbar Truncation
1. Hiding the scrollbar on `.the-menu` eliminates visual feedback.
2. In narrow/short viewports or where a user has many custom tabs, they cannot determine if the menu is scrollable, causing hidden actions/truncation.

---

## 3. Caveats
* **Accessibility Settings**: The severity of the clipping issue (Finding 5) depends on whether the system accessibility font scaling is applied or if custom user CSS is injected. Standard DPI setups at 100% or 125% do not immediately show severe clipping, but it becomes pronounced at 150%+ text scaling.
* **WebView2 Updates**: Standard CSS `scrollbar-width` support (Finding 3) only applies to systems running modern WebView2 runtimes (Chromium 121+). In older environments, only `-webkit-scrollbar` is processed, which is successfully hidden.
* **Tauri Shell Navigation**: In `theme-settings-card.tsx`, the `mailto` anchor tag is treated as a standard link. Under some Tauri configurations, this link could cause navigation issues within the WebView if shell opening is not intercepted or handled correctly.

---

## 4. Conclusion
The layout audit reveals several styling regressions and WebView2 compatibility issues:
1. Specificity conflicts render Frosted Glass dialogs and menus semi-transparent, causing severe readability/overlapping issues.
2. Inline style color hardcoding overrides the dynamic skin system, causing visual leakage in Dark Mode for custom skins.
3. An `!important` omission on `scrollbarWidth` in the Connection Table allows scrollbars to leak in newer WebView2 runtimes.
4. Proxy list layout heights are too rigid (20px), rendering them vulnerable to text clipping when font scaling is applied.

---

## 5. Verification Method
To verify these findings, developers should:
1. **Specificity Conflict & Hardcoded Backgrounds**:
   - Inspect `src/assets/styles/index.scss` at lines 109-115 and 184-190.
   - Switch the application skin to `Frosted Glass` in dark mode, and open the `Edit Profile` dialog; check if the background is highly transparent and text is overlapping.
   - Open `BasePage` and `UnlockPage` in Cyberpunk mode, and verify that the page background and card elements render as solid dark gray `#1e1f27` / `#282a36` instead of the cyberpunk theme colors.
2. **Scrollbar Leak**:
   - Inspect `src/components/connection/connection-table.tsx` line 45.
   - In a browser or newer WebView2 container, verify if a scrollbar is displayed on the connection table.
3. **MUI and TypeScript integrity check**:
   - Propose running the following command to check for any linting/compilation failures:
     ```powershell
     npm run lint
     npm run typecheck
     ```
