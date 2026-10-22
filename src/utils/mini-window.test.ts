import { describe, expect, it } from 'vitest'

import { isMiniByPhysical, isMinimalWidthByPhysical } from './mini-window'

/**
 * 极窄判定回归测试。
 *
 * 这组用例的存在理由：v3.0.0 ~ v3.0.3 连续两次「修了没效果」，
 * 全部栽在同一个 1 物理像素上——150% 缩放下最小窗内尺寸是 189 物理像素
 * （旧门槛 135 时代为 203、121 时代为 182），页面视口上报的是整数 CSS 像素，
 * 121 时代 182/1.5=121.33 会进位上报 122 而门槛是 121，判定必然失效。
 * 126 时代 150% 档恰为整数，但 125%/175% 档落点（158/221）仍带 0.5 边界，
 * 所以下面每个 DPI 都必须证明：**窗口落在最小内尺寸上时判定必然成立**。
 */
describe('isMiniByPhysical', () => {
  it('150% DPI：最小内尺寸 407×189 物理像素判定为极窄窗', () => {
    // 271×1.5 = 406.5，Windows 落点 407；126×1.5 = 189，恰为整数
    expect(isMiniByPhysical(407, 189, 1.5)).toBe(true)
  })

  it('150% DPI：189 精确落点、125% DPI：157 下偏落点都判定为极窄窗（容差覆盖取整方向）', () => {
    // 125% 档 126×1.25 = 157.5，Windows 半进位落 158；人为下偏 157 也必须成立
    expect(isMiniByPhysical(356, 158, 1.25)).toBe(true)
    expect(isMiniByPhysical(356, 157, 1.25)).toBe(true)
    expect(isMiniByPhysical(407, 189, 1.5)).toBe(true)
  })

  it('150% DPI：高度到 191 物理像素（≈127.3 CSS）即退出极窄窗', () => {
    // 126×1.5+1 = 190，190 及以内成立；191 起退出
    expect(isMiniByPhysical(407, 190, 1.5)).toBe(true)
    expect(isMiniByPhysical(407, 191, 1.5)).toBe(false)
  })

  it('150% DPI：宽度到 429 物理像素（≈286 CSS）即退出窄窗', () => {
    // 285×1.5+1 = 428.5 → 428 及以内算窄窗
    expect(isMiniByPhysical(428, 189, 1.5)).toBe(true)
    expect(isMiniByPhysical(429, 189, 1.5)).toBe(false)
  })

  it.each([
    // 每档取「阈值 × 缩放因子」的 Windows 实际物理落点（MulDiv 半进位取整）
    [1, 285, 126],
    [1.25, 356, 158],
    [1.5, 407, 189],
    [1.75, 499, 221],
    [2, 570, 252],
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
