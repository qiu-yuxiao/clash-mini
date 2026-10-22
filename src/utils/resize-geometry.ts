// 【口径】本文件的尺寸一律是 logical px、**内尺寸/客户区**口径。
// 后端用 min_inner_size / max_inner_size 约束，窗口又以 shadow(false) 创建（outer == inner），
// 所以这里绝不能混入 outer 尺寸。边界值不在此处定义，统一从 @/constants 取（唯一事实来源）。
import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
} from '@/constants'

type ResizeDirection = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

/**
 * 纯函数：根据拖拽起始几何与鼠标位移，算出窗口的新几何。
 *
 * 【入参口径】`startInnerW/H` 必须是**内尺寸（客户区）**物理像素（来自 `innerSize()`），
 * `startPhysX/Y` 必须是**外位置**物理像素（来自 `outerPosition()`）。
 * 因为落地分别是 `setSize()`（Tauri 语义 = 内尺寸）与 `setPosition()`（Tauri 语义 = 外位置）。
 * 两者混用会让每帧尺寸多出一个「外框内缩量」：历史上曾用 outerSize 作基准 + setSize 落地，
 * 在 150% DPI 下每帧凭空放大 22×13 物理像素，最小宽度时表现为「按下左键窗口就跳宽」。
 *
 * 【出参口径】除 `width` / `height` 是 clamp 后的 logical 尺寸外，
 * `physW` / `physH` / `physX` / `physY` 全是**整数物理像素**，可直接构造函数下发。
 * 落地时不再做 logical → physical 换算 —— 那一趟往返每帧要取整两次，正是 1px 抖动来源之一。
 *
 * 【锚定边反推】西/北方向拖拽时，位置不由「起点 + 位移」推导，而由**固定的对边**反推：
 *   physX = startPhysX + startInnerW - physW
 * 这样即使尺寸被 min/max clamp，右边（下边）也绝对不动，不会出现
 * 「尺寸取整一次、位置取整一次，两边各差半像素」的 ±1px 边缘抖动。
 * （`startPhysX + startInnerW` 即窗口右边：本项目 shadow(false)，外尺寸恒等于内尺寸。）
 *
 * 尺寸 clamp 到 [MIN, MAX]（logical 口径，与后端 min/max_inner_size 一致）。
 */
export function computeResizeGeometry(input: {
  direction: ResizeDirection
  startInnerW: number
  startInnerH: number
  startPhysX: number
  startPhysY: number
  dx: number
  dy: number
  scaleFactor: number
}): {
  width: number
  height: number
  physW: number
  physH: number
  physX: number | null
  physY: number | null
} {
  const {
    direction,
    startInnerW,
    startInnerH,
    startPhysX,
    startPhysY,
    dx,
    dy,
  } = input
  const sf = input.scaleFactor

  let newW = startInnerW
  let newH = startInnerH

  if (direction.includes('e')) newW = startInnerW + dx
  if (direction.includes('s')) newH = startInnerH + dy
  if (direction.includes('w')) newW = startInnerW - dx
  if (direction.includes('n')) newH = startInnerH - dy

  // 只对「本次被拖拽的维度」做 clamp，另一维原样透传（不重新 clamp）。
  // 否则会出现：窗口停在最小宽度（150% 下物理 406 = 270.67 logical）时拖上下边，
  // 宽度被重新 clamp 到 271 → 物理 407，凭空宽出 1 像素。
  const horiz = direction.includes('e') || direction.includes('w')
  const vert = direction.includes('n') || direction.includes('s')

  const clampedW = horiz
    ? Math.max(MIN_WINDOW_WIDTH, Math.min(MAX_WINDOW_WIDTH, newW / sf))
    : startInnerW / sf
  const clampedH = vert
    ? Math.max(MIN_WINDOW_HEIGHT, Math.min(MAX_WINDOW_HEIGHT, newH / sf))
    : startInnerH / sf

  // 全程唯一一次取整：logical → physical
  const physW = Math.round(clampedW * sf)
  const physH = Math.round(clampedH * sf)

  // 锚定边反推（东/南方向位置不变，故为 null）
  const physX = direction.includes('w')
    ? startPhysX + startInnerW - physW
    : null
  const physY = direction.includes('n')
    ? startPhysY + startInnerH - physH
    : null

  return { width: clampedW, height: clampedH, physW, physH, physX, physY }
}
