import { describe, expect, it } from 'vitest'

import { isMiniByPhysical, isMinimalWidthByPhysical } from './mini-window'

/**
 * 极窄判定回归测试。
 *
 * 这组用例的存在理由：v3.0.0 ~ v3.0.3 连续两次「修了没效果」，
 * 全部栽在同一个 1 物理像素上——150% 缩放下最小窗内尺寸是 203 物理像素，
 * 页面视口上报 136 CSS像素（203/1.5=135.33 取整进位），而门槛是 135。
 * 所以下面每个 DPI 都必须证明：**窗口落在最小内尺寸上时判定必然成立**。
 */
describe('isMiniByPhysical', () => {
  it('150% DPI：最小内尺寸 407×203 物理像素判定为极窄窗（历史失效点）', () => {
    // 271×1.5 = 406.5，Windows 落点 407；135×1.5 = 202.5，落点 203
    expect(isMiniByPhysical(407, 203, 1.5)).toBe(true)
  })

  it('150% DPI：202 / 203 两种物理落点都判定为极窄窗（容差覆盖取整的两个方向）', () => {
    // 135×1.5 = 202.5——向下取整落 202、向上取整落 203，两者都必须成立
    expect(isMiniByPhysical(407, 202, 1.5)).toBe(true)
    expect(isMiniByPhysical(407, 203, 1.5)).toBe(true)
  })

  it('150% DPI：高度到 204 物理像素（136 CSS）即退出极窄窗', () => {
    expect(isMiniByPhysical(407, 204, 1.5)).toBe(false)
  })

  it('150% DPI：宽度到 429 物理像素（≈286 CSS）即退出窄窗', () => {
    // 285×1.5+1 = 428.5 → 428 及以内算窄窗
    expect(isMiniByPhysical(428, 203, 1.5)).toBe(true)
    expect(isMiniByPhysical(429, 203, 1.5)).toBe(false)
  })

  it.each([
    // 每档取「阈值 × 缩放因子」四舍五入后的真实物理落点
    [1, 285, 135],
    [1.25, 356, 169],
    [1.5, 407, 203],
    [1.75, 499, 236],
    [2, 570, 270],
  ])('%s 倍缩放下最小内尺寸 %s×%s 物理像素必然判定为极窄窗', (sf, physW, physH) => {
    expect(isMiniByPhysical(physW, physH, sf)).toBe(true)
  })

  it('窗口被拉高后不再是极窄窗', () => {
    expect(isMiniByPhysical(407, 600, 1.5)).toBe(false)
  })
})

describe('isMinimalWidthByPhysical', () => {
  it('150% DPI：最小宽度 407 物理像素判定为窄窗', () => {
    expect(isMinimalWidthByPhysical(407, 1.5)).toBe(true)
  })

  it('150% DPI：宽度阈值边界 428 成立、429 不成立', () => {
    expect(isMinimalWidthByPhysical(428, 1.5)).toBe(true)
    expect(isMinimalWidthByPhysical(429, 1.5)).toBe(false)
  })

  it('窄窗判定不看高度（只看宽度）', () => {
    expect(isMinimalWidthByPhysical(407, 1.5)).toBe(true)
  })
})
