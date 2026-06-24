# Handoff Report — Explorer M1 Phase 2

## 1. Observation

Direct observations made in the codebase:
- **Active Node row layout definition (`src/pages/_layout.tsx` lines 1662–1676):**
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
  ```
- **Active Node status card wrapper styles (`src/pages/_layout/components/active-node-card.tsx` lines 222–232):**
  ```tsx
  <Paper
    sx={{
      m: 0,
      p: '0 12px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 1.5,
      height: '28px',
      ...get3DCardStyle(theme, 'default'),
    }}
  >
  ```
- **Active Node name typography component style (`src/pages/_layout/components/active-node-card.tsx` lines 269–288):**
  ```tsx
  <Tooltip title="点击轮换下一个节点">
    <Typography
      variant="body2"
      onClick={handleCycleNode}
      sx={{
        fontWeight: 'bold',
        fontSize: '12px',
        color: isRetro3DDark ? '#2C1F03' : 'text.primary',
        maxWidth: { xs: '120px', sm: '240px', md: '360px' },
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap',
        cursor: 'pointer',
        transition: 'color 0.2s',
        ...
  ```
- **Responsive media queries for hiding text components (`src/pages/_layout/components/active-node-card.tsx` lines 237–242, 260–263, 351–354):**
  - Label:
    ```tsx
    '@media (max-width: 580px)': {
      display: 'none',
    },
    ```
  - Protocol chip:
    ```tsx
    '@media (max-width: 480px)': {
      display: 'none',
    },
    ```
  - Node address:
    ```tsx
    '@media (max-width: 400px)': {
      display: 'none',
    },
    ```
- **Latency display chip (`src/pages/_layout/components/active-node-card.tsx` lines 315–340):**
  ```tsx
  sx={{
    fontSize: '11px',
    height: '20px',
    fontWeight: 600,
    cursor: 'pointer',
    ...
    '& .MuiChip-icon': {
      color: 'inherit',
      fontSize: '12px',
    },
  }}
  ```
- **SCSS Styles (`src/assets/styles/*.scss`):**
  - Direct grep search for `ActiveNodeStatusCard`, `MuiChip-icon`, `latency`, or `delay` yielded zero matches. SCSS files do not format this component.

---

## 2. Logic Chain

1. **Active Node Card parent container** has `flex: 1, minWidth: 0` in `_layout.tsx`, allowing the container to shrink without forcing the parent flex row to expand.
2. However, the inner `<Paper>` card component in `active-node-card.tsx` uses `display: 'flex'` with no `overflow: 'hidden'` or `maxWidth` bounds.
3. In Flexbox, children default to `min-width: auto`. The node name `Typography` element uses `white-space: nowrap` for ellipsis formatting but lacks `minWidth: 0`.
4. As a result, the browser calculates the `min-width` of the `Typography` element based on the full length of the proxy node name. In narrow layout environments (like the 270px width mode where the card container is squeezed to ~154px), the node name element will refuse to shrink below its text width.
5. Furthermore, the hide rules on other elements (label, protocol, address) are tied to viewport width media queries (`@media (max-width: ...)`). Under a wide window width where the layout is squeezed, these elements remain visible (`display: flex`).
6. Because the total width of these unhidden elements exceeds the squeezed container's width, and because `<Paper>` lacks `overflow: 'hidden'`, the active connection node card's contents will overflow the card boundaries, leading to a layout collapse and visual overlap with absolute-positioned elements (such as the settings gear button).

---

## 3. Caveats

- We did not dynamically mount the app in a browser to inspect runtime CSS layout computations, as this is a read-only investigation. All layout behavior is inferred from static source code analysis.

---

## 4. Conclusion

The active connection node and latency display row formatting is defined via inline styles, MUI `sx` props, and helper-derived CSS classes, without SCSS definitions. The layout collapse risk in narrow/squeezed modes is caused by:
1. **The lack of `minWidth: 0` (or another min-width override)** on the `Typography` element presenting the node name, preventing text-clipping from taking effect in flex sizing.
2. **The reliance on viewport-based `@media` queries** instead of container-based/state-based queries to hide secondary layout components, leaving them visible when the container is squeezed in a wide window.

---

## 5. Verification Method

- **Files to Inspect:**
  - `src/pages/_layout.tsx` (specifically the row container starting at line 1662)
  - `src/pages/_layout/components/active-node-card.tsx` (specifically the styling props on `Paper`, `Typography`, and `Chip`)
- **Invalidation Condition:**
  - If a code change is implemented that replaces the viewport `@media` queries with a React state-based visibility check or adds `minWidth: 0` to the typography child elements, layout collapse under squeezed container sizes will be resolved.
