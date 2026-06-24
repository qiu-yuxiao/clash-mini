# Codebase Layout-Clipping Design Agreements Analysis

## Executive Summary
This analysis identifies the locations, contexts, and details of intentional layout-clipping and responsive height cutoff designs marked with warning comments in the Clash Mini project. These behaviors are documented in `clash_mini_agreements.md` and explicitly commented in the source files to prevent auditors or AI agents from mistakenly flagging or changing them as layout bugs.

---

## 1. Connections Panel Clipping Design Agreement
Under narrow window layouts (such as the default 270px width), the connections panel is designed to be squeezed to 0px and physically clipped off-screen rather than wrapping or expanding.

### Occurrence A: Settings Sliding Drawer Wrapper
* **File Path**: `src/pages/_layout.tsx`
* **Line Range**: 1735 - 1738
* **Context**: Applied inline as style properties for the Settings Sliding Drawer (`theme-panel` wrapper).
* **Verbatim Comments & Code**:
  ```tsx
  // WARNING [FOR AI AGENTS / AUDITORS]:
  // This flex layout must remain as row direction and MUST NOT wrap. In default 270px width,
  // the connections panel is intentionally squeezed to 0px (hidden) and physically clipped
  // off-screen, per the design agreement. Widening the window will slide it into view.
  display: 'flex',
  ```

### Occurrence B: Connections Panel Root Layout
* **File Path**: `src/pages/_layout/components/connections-panel.tsx`
* **Line Range**: 45 - 48
* **Context**: Styled within the root `<Box>` component for the connections panel component.
* **Verbatim Comments & Code**:
  ```tsx
  // WARNING [FOR AI AGENTS / AUDITORS]:
  // This minWidth must remain 0 and MUST NOT be set to a fixed minimum width.
  // Shrinking to 0px is the INTENDED behavior for 270px narrow width mode to hide
  // the panel by sliding/clipping it off-screen, per the design agreement.
  minWidth: 0,
  ```

---

## 2. Media Query Height Hiding Design Agreement
Elements such as theme/skin selectors and debug/system menu buttons must remain hidden under a height threshold of 830px (e.g. the default 680px window height). This is responsive height cutoff behavior mandated by `clash_mini_agreements.md` to prevent layout overlaps.

### Occurrence A: Bottom-Left Dropdown Select Button
* **File Path**: `src/pages/_layout.tsx`
* **Line Range**: 1901 - 1904
* **Context**: Styled on a bottom-left menu selector button component.
* **Verbatim Comments & Code**:
  ```tsx
  // WARNING [FOR AI AGENTS / AUDITORS]:
  // The 830px max-height display:none check is the INTENDED responsive height cutoff
  // rule mandated by clash_mini_agreements.md. Under default 680px height, these
  // elements must remain hidden. DO NOT change this threshold to make them visible.
  '@media (max-height: 830px)': {
    display: 'none',
  },
  ```

### Occurrence B: Excel Selector Row Container
* **File Path**: `src/pages/_layout.tsx`
* **Line Range**: 1962 - 1965
* **Context**: Styled inside the container Box for the Excel / Skin selector row.
* **Verbatim Comments & Code**:
  ```tsx
  // WARNING [FOR AI AGENTS / AUDITORS]:
  // The 830px max-height display:none check is the INTENDED responsive height cutoff
  // rule mandated by clash_mini_agreements.md. Under default 680px height, these
  // elements must remain hidden. DO NOT change this threshold to make them visible.
  '@media (max-height: 830px)': {
    display: 'none',
  },
  ```

### Occurrence C: Help Menu Button Responsive Hiding
* **File Path**: `src/pages/_layout/components/help-menu-button.tsx`
* **Line Range**: 62 - 64
* **Context**: Section 17 of `clash_mini_agreements.md` dictates that the help button must be hidden using the same responsive height cutoff rule.
* **Verbatim Code**:
  ```tsx
  '@media (max-height: 830px)': {
    display: 'none',
  },
  ```

---

## 3. Related Design Agreement Documentation
* **Document**: `clash_mini_agreements.md` (root directory)
* **Section 17 (Help Button) Details** (Lines 1368):
  > **响应式隐藏**：与换肤选择器保持一致的响应式高度裁剪规则，当窗口高度较小时（由媒体查询 `@media (max-height: 830px)` 触发）自动隐藏，防止纵向空间狭窄时产生重叠。
* **Section II.3/Section II.5 details** (Lines 781-783):
  > 整个选择器固定总宽度为 `462.5px`。最左侧格子（`Trump-3D`）固定宽度为 `92.5px`，起点定位在偏左 `177.5px`，确保在 `270px` 默认窄窗口下该格精准露出窗口右下角（177.5px 至 270px 区间内）；右侧 5 个格子平分其余 `370px` 空间，每个格子固定宽度为 `74px`。
  > 物理裁剪揭示：在 `270px` 默认窄窗口下，`Trump-3D` 格精准露在窗口右下角，其余 5 个按钮被完全遮挡；拉宽窗口至最大 `640px` 时，其余 5 个按钮向右滑动展示被逐步揭露。
