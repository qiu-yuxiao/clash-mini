import { describe, expect, it } from 'vitest'

import parseTraffic, {
  TRAFFIC_GRAPH_BUFFER_LEN,
  TRAFFIC_GRAPH_MAX_POINTS,
  TRAFFIC_GRAPH_MIN_POINTS,
  computeTrafficGraphY,
  computeTrafficPointX,
  resolveTrafficVisiblePoints,
  sliceTrafficVisibleBuffer,
} from './parse-traffic'

// 锁住流量显示的进位与精度行为。这些数值直接出现在界面上，
// 单位进位错了比显示错更难被发现（用户会以为自己流量用超了）。
describe('parseTraffic 流量单位换算', () => {
  it('非数字入参返回 NaN 占位，不抛异常', () => {
    expect(parseTraffic(undefined)).toEqual(['NaN', ''])
    expect(parseTraffic('abc' as unknown as number)).toEqual(['NaN', ''])
  })

  it('0 字节', () => {
    expect(parseTraffic(0)).toEqual(['0.00', 'B'])
  })

  it('字节级不升位', () => {
    expect(parseTraffic(500)).toEqual(['500', 'B'])
  })

  it('满 1024 进位到 KB', () => {
    expect(parseTraffic(1024)).toEqual(['1.00', 'KB'])
    expect(parseTraffic(2048)).toEqual(['2.00', 'KB'])
  })

  it('MB / GB 进位正确', () => {
    expect(parseTraffic(1024 ** 2)).toEqual(['1.00', 'MB'])
    expect(parseTraffic(1024 ** 3)).toEqual(['1.00', 'GB'])
  })

  it('大数值降精度避免溢出显示（≥1000 取整）', () => {
    // 1000 字节 < 1024 不升位，但数值 ≥1000 → 走 toFixed(0)
    expect(parseTraffic(1000)).toEqual(['1000', 'B'])
  })
})

describe('BUG-410: TrafficGraph 宽窄窗统一曲线几何与随宽自适应不变式', () => {
  it('零流量与超 10MB/s 峰值在 80px/99px 高度下均严格约束于 [2, height - 2] 内，避免 2.5px 描边在底/顶边裁切', () => {
    for (const height of [80, 99]) {
      expect(computeTrafficGraphY(0, height)).toBe(height - 2)
      expect(computeTrafficGraphY(10_485_760, height)).toBe(2)
      expect(computeTrafficGraphY(50 * 1024 * 1024, height)).toBe(2)

      for (const bps of [0, 1, 10, 512, 10240, 102400, 1048576, 104857600]) {
        const y = computeTrafficGraphY(bps, height)
        expect(y).toBeGreaterThanOrEqual(2)
        expect(y).toBeLessThanOrEqual(height - 2)
      }
    }
  })

  it('分段对数纵轴对中小流量仍具备明显垂直抬升（不被历史高峰压扁为贴底直线）', () => {
    const height = 99
    const baselineY = computeTrafficGraphY(0, height)
    const smallBpsY = computeTrafficGraphY(512, height)
    const tenKbY = computeTrafficGraphY(10 * 1024, height)

    expect(baselineY - smallBpsY).toBeGreaterThan(25)
    expect(smallBpsY - tenKbY).toBeGreaterThan(10)
  })

  it('时间跨度随窗口宽度由 30s 平滑扩展至 60s（兼顾手动拖宽与点击最大化），且截取始终锚定最新样本', () => {
    expect(TRAFFIC_GRAPH_MIN_POINTS).toBe(30)
    expect(TRAFFIC_GRAPH_MAX_POINTS).toBe(60)
    expect(TRAFFIC_GRAPH_BUFFER_LEN).toBe(63)

    expect(resolveTrafficVisiblePoints(259)).toBe(30)
    expect(resolveTrafficVisiblePoints(275)).toBe(30)

    const w400 = resolveTrafficVisiblePoints(400)
    const w500 = resolveTrafficVisiblePoints(500)
    expect(w400).toBeGreaterThan(30)
    expect(w400).toBeLessThan(w500)
    expect(w500).toBeLessThan(60)
    expect(resolveTrafficVisiblePoints(616)).toBe(60)
    expect(resolveTrafficVisiblePoints(800)).toBe(60)

    const buffer = Array.from(
      { length: TRAFFIC_GRAPH_BUFFER_LEN },
      (_, idx) => idx,
    )
    const narrowSlice = sliceTrafficVisibleBuffer(buffer, 30)
    const wideSlice = sliceTrafficVisibleBuffer(buffer, 60)

    expect(narrowSlice).toHaveLength(33)
    expect(wideSlice).toHaveLength(63)
    expect(narrowSlice[narrowSlice.length - 1]).toBe(62)
    expect(wideSlice[wideSlice.length - 1]).toBe(62)
    expect(narrowSlice[0]).toBe(30)
    expect(wideSlice[0]).toBe(0)
  })

  it('窄窗(259px)、手动拖宽(440px)与最大化(616px)在任意滚动进度 [0, 1] 下横向点阵始终覆盖全宽且无右端缺口', () => {
    const buffer = Array.from(
      { length: TRAFFIC_GRAPH_BUFFER_LEN },
      (_, idx) => idx,
    )
    const scenarios = [
      { width: 259, visiblePoints: TRAFFIC_GRAPH_MIN_POINTS },
      { width: 440, visiblePoints: resolveTrafficVisiblePoints(440) },
      { width: 616, visiblePoints: resolveTrafficVisiblePoints(616) },
    ]

    for (const { width, visiblePoints } of scenarios) {
      const sliced = sliceTrafficVisibleBuffer(buffer, visiblePoints)
      const dx = width / visiblePoints
      for (const progress of [0, 0.5, 1]) {
        const firstX = computeTrafficPointX(0, dx, progress)
        const lastX = computeTrafficPointX(sliced.length - 1, dx, progress)
        expect(firstX).toBeLessThanOrEqual(0)
        expect(lastX).toBeGreaterThanOrEqual(width - 1e-9)
      }
    }
  })
})
