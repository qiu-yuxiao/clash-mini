// 窗口尺寸与布局断点常量（logical px，**内容/客户区**口径）。
// 与后端 src-tauri/src/utils/resolve/window.rs 的 MINIMAL_WIDTH / MINIMAL_HEIGHT / MAX_WIDTH /
// MAX_HEIGHT 必须保持一致；断点类常量只存在于前端（后端不需要知道排版断点）。
//
// 【本文件是唯一事实来源】布局断点、缩放拖拽钳制、窗口默认值都从这里取，
// 禁止在别处重复硬编码（重复的魔数正是 BUG-297 的成因之一）。
//
// ⚠️ 「窗口几何下限」与「排版断点」是两件不同的事，取值也必须分开看：
//
// · MIN_WINDOW_WIDTH = 271：窗口能达到的最小/默认宽度。
//   旧版窗口带系统阴影，客户区比窗口外框窄 22 物理像素（150% DPI 下，左右各 11），
//   即外框 285 时内容实际只有约 270.67 CSS px。关闭阴影后 outer == inner，
//   若沿用 285 会让窄窗内容凭空变宽约 14px、既有排版变松，故取 271（= 285 − 14）。
//
// · MIN_WINDOW_HEIGHT = 121：窗口能达到的最小高度。
//   旧值 135 是带系统阴影时代的余量（与宽度同理：阴影占位 14px）。关闭阴影后
//   outer == inner，高度侧同样按 −14 对称修正（135 − 14 = 121），否则最小高度下
//   底部凭空多出一段没有归属的空白，外观怪异（BUG-300）。
//
// · MINI_WIDTH_THRESHOLD = 285：窄窗/宽窗的**排版断点**，比较对象是内容宽度。
//   旧版就是这个值，必须保持不变 —— 否则内容宽度 272–285 这一段会从「1 列窄窗」
//   变成「3 列宽窗」，把三列卡片硬挤进 272px，属于可见的排版事故。
//   它比最小宽度大，是有意为之：窗口最小时必然落在窄窗模式，且 272–285 区间行为与历史一致。
export const MIN_WINDOW_WIDTH = 271
export const MIN_WINDOW_HEIGHT = 121
export const MAX_WINDOW_WIDTH = 640
export const MAX_WINDOW_HEIGHT = 860

/** 窄窗判定断点：内容宽度 ≤ 该值走 1 列紧凑布局，> 该值走 3 列宽窗布局 */
export const MINI_WIDTH_THRESHOLD = 285
/**
 * 极小窗口判定断点：内容宽 ≤ MINI_WIDTH_THRESHOLD 且 高 ≤ 该值，触发数据精简 + DOM 卸载。
 * 当前与 MIN_WINDOW_HEIGHT 同值 121，但语义不同（高度下限 vs 极小窗口门槛），故不合并；
 * 改动其一必须核对另一处是否也要跟着改。
 */
export const MINI_HEIGHT_THRESHOLD = 121
