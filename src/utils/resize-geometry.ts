// 窗口最小/最大尺寸（logical px），与后端 resolve/window.rs 对齐：
// MINIMAL_WIDTH=285.0, MINIMAL_HEIGHT=135.0, MAX_WIDTH=640.0, MAX_HEIGHT=860.0
const MIN_WIDTH = 285
const MIN_HEIGHT = 135
const MAX_WIDTH = 640
const MAX_HEIGHT = 860

type ResizeDirection = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

/**
 * 纯函数：根据拖拽起始几何与鼠标位移，算出窗口的新几何（logical px）。
 * 尺寸 clamp 到 [MIN, MAX]；在达到边界时回退对边位置以保持不动。
 * 抽成纯函数便于单测，也避免 handlePointerMove 里塞满数学。
 */
export function computeResizeGeometry(input: {
  direction: ResizeDirection
  startPhysW: number
  startPhysH: number
  startPhysX: number
  startPhysY: number
  dx: number
  dy: number
  scaleFactor: number
}): { width: number; height: number; x: number | null; y: number | null } {
  const { direction, startPhysW, startPhysH, startPhysX, startPhysY, dx, dy } =
    input
  const sf = input.scaleFactor

  let newW = startPhysW
  let newH = startPhysH
  let newX = startPhysX
  let newY = startPhysY

  if (direction.includes('e')) newW = startPhysW + dx
  if (direction.includes('s')) newH = startPhysH + dy
  if (direction.includes('w')) {
    newW = startPhysW - dx
    newX = startPhysX + dx
  }
  if (direction.includes('n')) {
    newH = startPhysH - dy
    newY = startPhysY + dy
  }

  // physical → logical 用于 clamp 比较（后端 max_inner_size 也是 logical）
  const logicalW = newW / sf
  const logicalH = newH / sf

  // clamp 尺寸到 [MIN, MAX]
  const clampedW = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, logicalW))
  const clampedH = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, logicalH))

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
    x: hasWestOrNorth ? newX / sf : null,
    y: hasWestOrNorth ? newY / sf : null,
  }
}
