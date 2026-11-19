const UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']

export const TRAFFIC_GRAPH_MIN_POINTS = 30
export const TRAFFIC_GRAPH_MAX_POINTS = 60
/** 保留 60s（外加首尾视口外锚点）历史点位，从 271px 窄窗到 640px 宽窗满幅铺开且无右端缺口 */
export const TRAFFIC_GRAPH_BUFFER_LEN = TRAFFIC_GRAPH_MAX_POINTS + 3

const NARROW_CANVAS_WIDTH = 275
const MAXIMIZED_CANVAS_WIDTH = 616

/** 根据画布 CSS 宽度平滑映射可见时间窗口（≤275px 显示 30s，随拉宽或最大化线性扩展至 60s） */
export const resolveTrafficVisiblePoints = (cssWidth: number): number => {
  if (!Number.isFinite(cssWidth) || cssWidth <= NARROW_CANVAS_WIDTH) {
    return TRAFFIC_GRAPH_MIN_POINTS
  }
  const ratio = Math.min(
    1,
    Math.max(
      0,
      (cssWidth - NARROW_CANVAS_WIDTH) /
        (MAXIMIZED_CANVAS_WIDTH - NARROW_CANVAS_WIDTH),
    ),
  )
  return Math.round(
    TRAFFIC_GRAPH_MIN_POINTS +
      ratio * (TRAFFIC_GRAPH_MAX_POINTS - TRAFFIC_GRAPH_MIN_POINTS),
  )
}

/** 截取右侧最新 `visiblePoints` 秒视窗（保留 3 个边缘插值锚点），左端随宽度扩展自然揭露更早缓冲 */
export const sliceTrafficVisibleBuffer = <T>(
  buffer: readonly T[],
  visiblePoints: number,
): T[] => {
  const clampedVisible = Math.min(
    TRAFFIC_GRAPH_MAX_POINTS,
    Math.max(TRAFFIC_GRAPH_MIN_POINTS, Math.round(visiblePoints)),
  )
  const needed = clampedVisible + 3
  return buffer.length > needed
    ? buffer.slice(buffer.length - needed)
    : buffer.slice()
}

/** 分段对数纵轴坐标映射，钳制于 [2, height - 2] 防止 2.5px 线宽在底边/顶边被裁切 */
export const computeTrafficGraphY = (v: number, height: number): number => {
  if (height <= 4) return Math.max(0, height / 2)
  if (v <= 0) return height - 2

  const dy = height / 7
  let rawY: number
  if (v <= 10) rawY = height - (v / 10) * dy
  else if (v <= 100) rawY = height - (v / 100 + 1) * dy
  else if (v <= 1024) rawY = height - (v / 1024 + 2) * dy
  else if (v <= 10240) rawY = height - (v / 10240 + 3) * dy
  else if (v <= 102400) rawY = height - (v / 102400 + 4) * dy
  else if (v <= 1048576) rawY = height - (v / 1048576 + 5) * dy
  else if (v <= 10485760) rawY = height - (v / 10485760 + 6) * dy
  else rawY = 2

  return Math.min(height - 2, Math.max(2, rawY))
}

/** 归一化横向点位计算：消除宽窄切换间的整数取整锯齿与右端留白 */
export const computeTrafficPointX = (
  index: number,
  dx: number,
  progress: number,
): number => dx * (index - 1) - progress * dx

const parseTraffic = (num?: number) => {
  if (typeof num !== 'number') return ['NaN', '']
  const exp =
    num < 1 ? 0 : Math.min(Math.floor(Math.log2(num) / 10), UNITS.length - 1)
  const dat = num / Math.pow(1024, exp)
  const ret = dat >= 1000 ? dat.toFixed(0) : dat.toPrecision(3)
  const unit = UNITS[exp]

  return [ret, unit]
}

export default parseTraffic
