import { MINI_HEIGHT_THRESHOLD, MINI_WIDTH_THRESHOLD } from '@/constants'

/**
 * 极窄判定容差（物理像素）。
 *
 * 窗口最小内尺寸 135 在 150% 缩放下是 202.5 物理像素，实际落点可能是 202 也可能是 203；
 * 而页面视口上报的是**整数** CSS 像素（203 / 1.5 = 135.33 → 上报 136）。
 * 只要判定式是「数值恰好等于阈值」，这 1 个物理像素就会让整个极窄模式永久失效
 * （v3.0.0 ~ v3.0.3 连续两次「修了没效果」的根因）。
 * 因此判定一律在物理像素域做，并放宽 1 物理像素（≈0.67 CSS px，肉眼无感）。
 */
export const MINI_PHYS_SLACK = 1

/**
 * 极窄窗判定：宽 ≤ 285 CSS 且高 ≤ 135 CSS（按物理像素比较）。
 *
 * @param physW 窗口**内尺寸**宽度（物理像素，来自 onResized payload / innerSize()）
 * @param physH 窗口**内尺寸**高度（物理像素）
 * @param sf 窗口所在显示器缩放因子（150% → 1.5）
 */
export function isMiniByPhysical(
  physW: number,
  physH: number,
  sf: number,
): boolean {
  return (
    physW <= MINI_WIDTH_THRESHOLD * sf + MINI_PHYS_SLACK &&
    physH <= MINI_HEIGHT_THRESHOLD * sf + MINI_PHYS_SLACK
  )
}

/** 窄窗判定：宽 ≤ 285 CSS（按物理像素比较）；与极窄判定共用同一容差口径 */
export function isMinimalWidthByPhysical(physW: number, sf: number): boolean {
  return physW <= MINI_WIDTH_THRESHOLD * sf + MINI_PHYS_SLACK
}
