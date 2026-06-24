# Handoff Report: Proxy Node List and Double Border Analysis

## 1. Observation

### Observation 1: Ignore Column Configurations in `use-render-list.ts`
In `src/components/proxy/use-render-list.ts` (lines 82–87):
```typescript
// 优化列布局计算
const calculateColumns = (width: number, _configCol: number): number => {
  if (width <= 285) {
    return 1
  }
  return 3
}
```
In `src/components/proxy/use-render-list.ts` (lines 119–122):
```typescript
  // 计算列数
  const col = useMemo(
    () => calculateColumns(width, verge?.proxy_layout_column || 6),
    [width, verge?.proxy_layout_column],
  )
```

### Observation 2: Filtering Out Proxy Headers in `proxy-groups.tsx`
In `src/components/proxy/proxy-groups.tsx` (lines 131–133):
```typescript
  const filteredRenderList = useMemo(() => {
    return renderList.filter((item) => item.type !== 1)
  }, [renderList])
```

### Observation 3: Group Headers (type: 0) Not Added to Normal Mode List
In `src/components/proxy/use-render-list.ts` (lines 389–470), there is no entry pushing `{ type: 0, ... }` into the list.

### Observation 4: Double Borders in Stylesheet (`index.scss`)
In `src/assets/styles/index.scss`:
- Line 75:
  ```scss
  .theme-panel {
    border: 4px double var(--theme-border) !important;
  ```
- Line 170:
  ```scss
    border: 4px double var(--theme-border) !important; /* Keep double border */
  ```
- Line 274:
  ```scss
    border: 4px double var(--theme-border) !important; /* Keep double border */
  ```
- Line 124 (Retro-3D):
  ```scss
    border: 4px double #B8860B !important;
  ```

### Observation 5: Double Borders in Components (`proxy-groups.tsx` and `proxy-render.tsx`)
In `src/components/proxy/proxy-groups.tsx` (lines 783–785):
```typescript
  return (
    <Box
      className="theme-panel"
```
In `src/components/proxy/proxy-render.tsx` (lines 84–89):
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

### Observation 6: Double Borders in Connection Table
In `src/components/connection/connection-table.tsx` (lines 52, 73):
```typescript
  border: '5px double var(--theme-border)',
```
```typescript
  borderBottom: '5px double var(--theme-border)',
```

---

## 2. Logic Chain

1. **Why the table view is missing**:
   - The user configures layout columns in settings (`verge?.proxy_layout_column`), which translates to `_configCol`.
   - When columns is `1`, `useRenderList` generates `type: 2` items (`ProxyItem` list row items). This represents the "table" layout of proxies.
   - However, `calculateColumns` completely ignores `_configCol` (Observation 1) and returns hardcoded `3` (when `width > 285`). Thus, the app is forced into grid column mode (`type: 4`) and cannot display the 1-column list/table layout.
   - Additionally, group control headers (`type: 1`) are filtered out of the rendered list (Observation 2), and collapsible group headers (`type: 0`) are never added to `renderList` in normal mode (Observation 3). This removes the headers, titles, and test toolbars from all proxy groups, hiding their table/group structures.

2. **Why there is a colored double-border outline around the proxy node table**:
   - The outer container box of the virtualized proxy list uses `className="theme-panel"` (Observation 5).
   - The CSS class `.theme-panel` defines an `important` 4px double border outline: `border: 4px double var(--theme-border) !important` (Observation 4).
   - In addition, the internal column cells have an explicit right double border: `borderRight: '5px double var(--theme-border)'` (Observation 5).
   - The color is resolved dynamically via the CSS custom property `--theme-border`, which is configured in `use-custom-theme.ts` based on light/dark mode and active skin/mode (e.g. golden for retro-3d).

---

## 3. Caveats

- We did not investigate whether any external plugins or user-injected custom CSS (`verge?.css_injection`) could alter the border behavior, but the local source code is explicitly forcing these double borders.
- We assume that the user's "missing table" refers to the single-column list/table view for proxy nodes being inaccessible.

---

## 4. Conclusion

- **Missing Table View**: Caused by `calculateColumns` hardcoding the column count to `3` (ignoring `_configCol`), `filteredRenderList` removing `type: 1` (`ProxyHead`) headers, and normal mode `renderList` omitting `type: 0` group collapsible headers.
- **Double Borders**: Caused by `.theme-panel` styling in `index.scss` declaring `4px double var(--theme-border) !important`, and `ProxyRender` rendering right cell borders using `5px double var(--theme-border)`.

---

## 5. Verification Method

To verify these findings:
1. Open `src/components/proxy/use-render-list.ts` and inspect lines 82–87 to confirm `_configCol` is unused.
2. Open `src/components/proxy/proxy-groups.tsx` and inspect line 132 to confirm `type === 1` is filtered out.
3. Open `src/assets/styles/index.scss` and search for `.theme-panel` to inspect the `border` properties.
4. Run the React dev server / build:
   - Check page styles using developer tools (F12) to inspect the computed border styles on `.theme-panel` and proxy cells.
