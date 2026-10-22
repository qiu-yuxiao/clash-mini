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
 * 纯函数：根据拖拽起始几何与鼠标位移，算出窗口的新几何（logical px）。
 *
 * 【口径铁律】`startInnerW` / `startInnerH` 必须是**内尺寸（客户区）**物理像素（来自 `innerSize()`），
 * `startPhysX` / `startPhysY` 必须是**外位置**物理像素（来自 `outerPosition()`）。
 * 因为落地分别是 `setSize()`（Tauri 语义 = 内尺寸）与 `setPosition()`（Tauri 语义 = 外位置）。
 *
 * 两者混用会让每帧尺寸多出一个「外框内缩量」：历史上曾用 outerSize 作基准 + setSize 落地，
 * 在 150% DPI 下每帧凭空放大 22×13 物理像素，最小宽度时表现为「按下左键窗口就跳宽」。
 *
 * 位置换算（保持对边不动）：newOuterX = startOuterX + (startInnerW - newInnerW)。
 * 外框左右/上下内缩量在这条式子中精确抵消，所以即使将来外框重新出现内缩也不会漂移。
 *
 * 尺寸 clamp 到 [MIN, MAX]；被 clamp 时回退对边位置以保持不动。
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
}): { width: number; height: number; x: number | null; y: number | null } {
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
  let newX = startPhysX
  let newY = startPhysY

  if (direction.includes('e')) newW = startInnerW + dx
  if (direction.includes('s')) newH = startInnerH + dy
  if (direction.includes('w')) {
    newW = startInnerW - dx
    newX = startPhysX + dx
  }
  if (direction.includes('n')) {
    newH = startInnerH - dy
    newY = startPhysY + dy
  }

  // physical → logical 用于 clamp 比较（后端 min/max_inner_size 也是 logical）
  const logicalW = newW / sf
  const logicalH = newH / sf

  // clamp 尺寸到 [MIN, MAX]
  const clampedW = Math.max(
    MIN_WINDOW_WIDTH,
    Math.min(MAX_WINDOW_WIDTH, logicalW),
  )
  const clampedH = Math.max(
    MIN_WINDOW_HEIGHT,
    Math.min(MAX_WINDOW_HEIGHT, logicalH),
  )

  // 若尺寸被 clamp（例如达到最小宽度），需要相应回退位置以保持对边不动
  // 推导：右边缘 = newX + newW，clamp 后 newW 变成 clampedW*sf，
  //   为保持右边缘不动，newX 需减去 (newW - clampedW*sf) = -dwLogical*sf
  //   即 newX += dwLogical*sf（dwLogical 为负时往左推，正时往右推）
  const dwLogical = clampedW - logicalW
  const dhLogical = clampedH - logicalH
  if (dwLogical !== 0 && direction.includes('w')) {
    newX -= dwLogical * sf
  }
  if (dhLogical !== 0 && direction.includes('n')) {
    newY -= dhLogical * sf
  }

  const hasWestOrNorth = direction.includes('w') || direction.includes('n')
  return {
    width: clampedW,
    height: clampedH,
    x: hasWestOrNorth ? Math.round(newX / sf) : null,
    y: hasWestOrNorth ? Math.round(newY / sf) : null,
  }
}
