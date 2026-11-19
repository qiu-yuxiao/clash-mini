// 窗口尺寸与布局断点常量（logical px，客户区/内容口径）。
// 几何下限须与后端 `src-tauri/src/utils/resolve/window.rs` 一致，禁止在别处硬编码。
// - MIN_WINDOW_WIDTH (271) < MINI_WIDTH_THRESHOLD (285)：关闭无框阴影后 outer == inner，
//   保证缩至最小时进入 1 列窄窗排版，同时保持 272-285 区间窄窗布局不变。
// - MIN_WINDOW_HEIGHT (126)：与 150% DPI 对齐为整数物理像素 (189px)。
export const MIN_WINDOW_WIDTH = 271
export const MIN_WINDOW_HEIGHT = 126
export const MAX_WINDOW_WIDTH = 640
export const MAX_WINDOW_HEIGHT = 860

/** 自定义标题栏固定高度（px），所有窄窗弹窗、遮罩与顶部通知不得覆盖此区域 */
export const CUSTOM_TITLEBAR_HEIGHT = 30

/** 设置抽屉关闭按钮层级（高于设置层 `100` 与底栏 `200`，严格低于模态弹窗 `1300` 与通知条 `2200`） */
export const SETTINGS_CLOSE_BUTTON_Z_INDEX = 250

/** 流量统计 Web Worker 在无活跃引用后的闲置销毁延迟（ms，对齐 React Query 30s gcTime） */
export const TRAFFIC_WORKER_DESTROY_DELAY_MS = 30_000

/** 窄窗判定断点：内容宽度 ≤ 该值走 1 列紧凑布局，> 该值走 3 列宽窗布局 */
export const MINI_WIDTH_THRESHOLD = 285
/**
 * 极小窗口判定断点：内容宽 ≤ MINI_WIDTH_THRESHOLD 且 高 ≤ 该值，触发数据精简 + DOM 卸载。
 * 当前与 MIN_WINDOW_HEIGHT 同值 126，但语义不同（高度下限 vs 极小窗口门槛），故不合并；
 * 改动其一必须核对另一处是否也要跟着改。
 */
export const MINI_HEIGHT_THRESHOLD = 126
