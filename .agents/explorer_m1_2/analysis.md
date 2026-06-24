# Analysis Report: Proxy Node List and Border Outline Issues

This analysis documents the investigations into two issues: the missing table inside the proxy node list view and the colored double-border outline around the proxy node table.

---

## 1. The Missing Table inside the Proxy Node List View

### Cause A: Hardcoded Column Calculations
In `src/components/proxy/use-render-list.ts`, the `calculateColumns` function is defined as:
```typescript
// 优化列布局计算
const calculateColumns = (width: number, _configCol: number): number => {
  if (width <= 285) {
    return 1
  }
  return 3
}
```
* **Problem**: The configured columns parameter `_configCol` (which represents `verge?.proxy_layout_column`) is prefixed with an underscore and completely ignored. It hardcodes returning `3` columns for any viewport width greater than `285px`.
* **Impact**: The user's settings for proxy layout columns (e.g. 1 column, which would render as a single-column list/table layout) are ignored. When `col === 1`, `useRenderList` generates `type: 2` items (`ProxyItem` full-width list items), which behave like rows in a table. Because columns are forced to `3`, it always renders `type: 4` items (grid-based rows), preventing the table/list layout from being displayed.

### Cause B: Proxy Headers/Toolbars Omitted (Filtered Out)
In `src/components/proxy/proxy-groups.tsx`:
```typescript
  const filteredRenderList = useMemo(() => {
    return renderList.filter((item) => item.type !== 1)
  }, [renderList])
```
* **Problem**: `type === 1` refers to `ProxyHead` (the toolbar that contains the speed test url, delay test button, sort order, and filter box). Filtering out `type !== 1` removes the headers for all proxy groups in normal mode.
* **Impact**: In normal mode, only the first group's `ProxyHead` is manually extracted as `activeGroupHeadItem` and rendered outside the virtualized list at the top. For all other groups, their respective control toolbars are completely missing, collapsing all groups into a single un-headered list.

### Cause C: Missing Group Header (Collapsible Headers)
In `src/components/proxy/use-render-list.ts` (normal mode block, lines 389–470):
* **Problem**: The normal mode rendering logic only pushes `type: 1` (`ProxyHead`) and proxy items (`type: 2` or `type: 4`) into `renderList`. It never pushes `type: 0` (collapsible group header, which is rendered as a `ListItemButton` in `proxy-render.tsx`).
* **Impact**: The collapsible group header rows (to open/close groups) are entirely missing from the normal mode view.

---

## 2. The Colored Double-Border Outline around the Proxy Node Table

The double-border styles are defined across several TSX and SCSS files:

### CSS / SCSS Definitions
In `src/assets/styles/index.scss`:
* **`.theme-panel` Double Borders**:
  ```scss
  .theme-panel {
    border: 4px double var(--theme-border) !important;
    border-radius: var(--border-radius, 8px);
    ...
  }
  ```
* **Skin-Specific Overrides**:
  * `html[data-control-skin="modern-flat"]` (line 170):
    ```scss
    border: 4px double var(--theme-border) !important; /* Keep double border */
    ```
  * `html[data-control-skin="original"]` (line 274):
    ```scss
    border: 4px double var(--theme-border) !important; /* Keep double border */
    ```
  * `html[data-control-skin="retro-3d"]` (line 124):
    ```scss
    border: 4px double #B8860B !important;
    ```

### Container Class Application
In `src/components/proxy/proxy-groups.tsx`, the `ProxyVirtualList` container applies the class `theme-panel`:
```typescript
  return (
    <Box
      className="theme-panel"
      sx={{
        height,
        display: 'flex',
        flexDirection: 'column',
        boxSizing: 'border-box',
        overflow: 'hidden',
      }}
    >
```
* **Impact**: This applies the `4px double` border around the entire proxy node list container.

### Internal Grid Column Borders
In `src/components/proxy/proxy-render.tsx` (lines 84–89), inside `proxyColItemsMemo` for grid layout (`type === 4`):
```typescript
        sx={{
          py: 0,
          pl: 0,
          ...(idx < (col || 3) - 1
            ? {
                borderRight: '5px double var(--theme-border)',
              }
            : {}),
        }}
```
* **Impact**: This adds a `5px double` border on the right of each column item in a row (except the last column), creating nested double borders within the table.

### Connection Table Borders
For comparison, `src/components/connection/connection-table.tsx` (lines 52, 73) also uses the same double-border style:
```typescript
  border: '5px double var(--theme-border)',
  ...
  borderBottom: '5px double var(--theme-border)',
```

---

## 3. Theme Border Color Variables
The `--theme-border` variable is set dynamically based on theme mode and skin in `src/pages/_layout/hooks/use-custom-theme.ts`:
* **Modern Flat Skin**:
  * Light Mode: `#e2e8f0` (gray)
  * Dark Mode: `#333333` (dark gray)
* **Original Skin**:
  * Light Mode: `#d0d7de`
  * Dark Mode: `#30363d`
* **Retro-3D Skin**:
  * Uses `#B8860B` (dark goldenrod)
