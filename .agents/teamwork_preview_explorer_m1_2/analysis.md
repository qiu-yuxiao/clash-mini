# Active Connection Node and Latency Display Styling & Layout Audit

## Summary of Findings
The active connection node and latency display are styled using a hybrid of React Inline CSS, MUI `sx` (Emotion) properties, and external theme-based styling helpers (`src/utils/button-styles.ts` and `src/pages/_layout/utils/style-helpers.tsx`). Key risks for layout collapse include the lack of `minWidth: 0` on flex children (specifically the active node name text component) and viewport-based responsive hiding rules that fail when the container is squeezed independently of viewport resizing.

---

## 1. Styling Files & Applied Styles

### A. Styling Files
There are **no external SCSS/CSS rules** directly targeting the active connection node or latency display. All styles are defined in React component files and helper files:
- **Component definition & layout mapping:** `src/pages/_layout/components/active-node-card.tsx`
- **Render position & parent structure:** `src/pages/_layout.tsx`
- **Design Tokens (Skins):** `src/utils/button-styles.ts` (specifically `get3DCardStyle`)
- **Latency Icons & Colors:** `src/pages/_layout/utils/style-helpers.tsx` (specifically `getSignalIcon` and `convertDelayColor`)

---

### B. Styling Applied to Active Node Card Container
The active node container is rendered as a `<Paper>` component inside `<ActiveNodeStatusCard>`:

* **HTML Structure & Grandparent Layout (defined in `src/pages/_layout.tsx`):**
  - **Grandparent flex row container** (lines 1662–1673):
    ```tsx
    display: 'flex',
    alignItems: 'center',
    width: '100%',
    boxSizing: 'border-box',
    padding: '3px 36px 2px 8px',
    position: 'relative',
    zIndex: 110,
    ```
    *Note: The right padding of `36px` is explicitly calculated to leave space for the absolute-positioned settings gear button.*
  - **Direct parent wrapper** (line 1674):
    ```tsx
    flex: 1,
    minWidth: 0,
    ```
* **Active Node Card (defined in `src/pages/_layout/components/active-node-card.tsx` at lines 222–233):**
  - **MUI Paper styles:**
    ```tsx
    m: 0,
    p: '0 12px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 1.5,
    height: '28px',
    ```
  - **Dynamic Theme Styles:** Extracted via `get3DCardStyle(theme, 'default')` which returns the box-shadows, borders, font-families, and gradients corresponding to the active skin (`retro-3d`, `original`, `modern-flat`, `frosted-glass`, `cyberpunk`, `monochrome`).

---

### C. Styling Applied to Latency Display
The latency display is rendered as a `<Chip>` component in `<ActiveNodeStatusCard>` (lines 298–342):

* **MUI Chip styles:**
  ```tsx
  fontSize: '11px',
  height: '20px',
  fontWeight: 600,
  cursor: 'pointer',
  bgcolor: testing ? undefined : alpha(...),
  color: signalInfo.color === 'text.secondary' ? (isRetro3DDark ? '#2C1F03' : 'text.secondary') : signalInfo.color,
  '& .MuiChip-icon': {
    color: 'inherit',
    fontSize: '12px',
  },
  ```
* **Latency Icon mapping (from `src/pages/_layout/utils/style-helpers.tsx`):**
  - Standard MUI SVG Icons (`SignalWifi3Bar` / `SignalWifi2Bar` / `SignalWifi0Bar` / `SignalWifi4Bar` / `SignalWifi1Bar` / `WifiOff`) are selected based on delay value (e.g. `<= 200ms` for excellent, `>= 500ms` for high, `0` or `>= 10000ms` for timeout).
  - The icon size is scaled down via the selector `& .MuiChip-icon` to `fontSize: '12px'`.

---

## 2. Layout Collapse & Size Inflation Risks

During our audit, we identified two main layout vulnerabilities that could lead to layout collapse or overflow under narrow display modes (such as the 270px width mode):

### Symptom 1: Viewport Media Queries vs. Container Squeeze
Several typography components inside `ActiveNodeStatusCard` are hidden responsively based on viewport width:
- **Active Node Label (`settings.mini.activeNodeLabel`):** Hides at viewport `< 580px` (`@media (max-width: 580px) { display: 'none' }`).
- **Protocol Chip:** Hides at viewport `< 480px` (`@media (max-width: 480px) { display: 'none' }`).
- **Node Address (`nodeAddr`):** Hides at viewport `< 400px` (`@media (max-width: 400px) { display: 'none' }`).

**The Bug:** Viewport media queries (`@media`) only inspect the window size, not the container's available width. If the container is squeezed independently of viewport resizing (e.g., when other layout elements expand, or when rendering inside a narrow flex container at a medium window width), these elements will remain visible and refuse to hide. Since the parent `<Paper>` card does **not** have `overflow: 'hidden'`, the un-hidden elements will overflow the card horizontally, causing layout distortion or overlapping with the settings gear icon.

### Symptom 2: Flexbox Minimum Content Width Inflation
The active node name is displayed inside a `Typography` element:
```tsx
maxWidth: { xs: '120px', sm: '240px', md: '360px' },
overflow: 'hidden',
textOverflow: 'ellipsis',
whiteSpace: 'nowrap',
```
**The Bug:** In CSS Flexbox, a flex child has a default `min-width` of `auto`. When `white-space: nowrap` is set, the browser computes the minimum width of the element based on the *entire un-truncated text*, ignoring the `maxWidth` property inside flex sizing calculations unless `minWidth: 0` is explicitly set on the flex item.
- Without `minWidth: 0` on the `Typography` component, a long proxy node name will force the `Typography` element to maintain its full un-truncated width under flex constraints, causing the outer card and the top row layout to inflate, preventing proper shrinking.

### Symptom 3: Icon Size Inflation Risk
- **MUI Icons:** Standard SvgIcons default to `width: 1em` and `height: 1em` and scale with the element's `font-size`. Inside `<Chip>`, Emotion applies `.MuiChip-icon { font-size: 12px }` which correctly restricts the icon size to 12px.
- **CircularProgress:** The loading spinner explicitly sets `size={10}` directly on the component.
- **Risk:** There is no global SVG style override that would cause these icons to inflate to the entire screen. However, if the selector `& .MuiChip-icon` is ever bypassed or if a non-MUI custom SVG is passed without explicit width/height properties, the icon will lack size bounds and expand to fill its container.

---

## 3. Evidence Chain

1. **Active Node row rendering and layout context:**
   - File: `src/pages/_layout.tsx` at lines 1662–1676.
2. **Component implementation and styling properties:**
   - File: `src/pages/_layout/components/active-node-card.tsx` at lines 221–359.
3. **Card design token skin configurations:**
   - File: `src/utils/button-styles.ts` (specifically `get3DCardStyle` helper).
4. **Latency icon classes and helper methods:**
   - File: `src/pages/_layout/utils/style-helpers.tsx` at lines 19–87.
