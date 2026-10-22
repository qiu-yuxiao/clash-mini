import { describe, expect, it } from 'vitest'

import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
} from '@/constants'

import { computeResizeGeometry } from './resize-geometry'

// 西/北向拖拽由锚定边反推保证 physX/physY 非空；测试里显式判空收窄而非 `!` 断言，
// 失败时给出可读信息（同时规避 noNonNullAssertion）。
function rightEdgeOf(geo: ReturnType<typeof computeResizeGeometry>): number {
  if (geo.physX === null) {
    throw new Error('physX 应非空：w/nw/sw 向拖拽必有锚定边反推')
  }
  return geo.physX + geo.physW
}

function bottomEdgeOf(geo: ReturnType<typeof computeResizeGeometry>): number {
  if (geo.physY === null) {
    throw new Error('physY 应非空：n/ne/nw 向拖拽必有锚定边反推')
  }
  return geo.physY + geo.physH
}

describe('computeResizeGeometry 窗口尺寸与位置重算', () => {
  const defaultBase = {
    startInnerW: 300,
    startInnerH: 200,
    startPhysX: 500,
    startPhysY: 300,
    scaleFactor: 1.0,
  }

  describe('东侧拖拽 (direction: e)', () => {
    it('向右拉伸增加宽度，高度与原点位置保持不变 (physX/physY 为 null)', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'e',
        dx: 50,
        dy: 0,
      })

      expect(geo.width).toBe(350)
      expect(geo.height).toBe(200)
      expect(geo.physW).toBe(350)
      expect(geo.physH).toBe(200)
      expect(geo.physX).toBeNull()
      expect(geo.physY).toBeNull()
    })

    it('向左收缩受 MIN_WINDOW_WIDTH (271) 钳制', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'e',
        dx: -100, // 300 - 100 = 200 < 271
        dy: 0,
      })

      expect(geo.width).toBe(MIN_WINDOW_WIDTH)
      expect(geo.physW).toBe(MIN_WINDOW_WIDTH)
      expect(geo.physX).toBeNull()
    })

    it('向右拉伸受 MAX_WINDOW_WIDTH (640) 钳制', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'e',
        dx: 500, // 300 + 500 = 800 > 640
        dy: 0,
      })

      expect(geo.width).toBe(MAX_WINDOW_WIDTH)
      expect(geo.physW).toBe(MAX_WINDOW_WIDTH)
    })
  })

  describe('南侧拖拽 (direction: s)', () => {
    it('向下拉伸增加高度，宽度与原点位置保持不变', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 's',
        dx: 0,
        dy: 60,
      })

      expect(geo.width).toBe(300)
      expect(geo.height).toBe(260)
      expect(geo.physX).toBeNull()
      expect(geo.physY).toBeNull()
    })

    it('向上收缩受 MIN_WINDOW_HEIGHT (121) 钳制', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 's',
        dx: 0,
        dy: -100, // 200 - 100 = 100 < 121
      })

      expect(geo.height).toBe(MIN_WINDOW_HEIGHT)
      expect(geo.physY).toBeNull()
    })

    it('向下拉伸受 MAX_WINDOW_HEIGHT (860) 钳制', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 's',
        dx: 0,
        dy: 800, // 200 + 800 = 1000 > 860
      })

      expect(geo.height).toBe(MAX_WINDOW_HEIGHT)
    })
  })

  describe('西侧拖拽 (direction: w) - 右边缘绝对锁定', () => {
    it('向左拉伸增加宽度并向左平移，右边缘坐标恒定', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'w',
        dx: -50,
        dy: 0,
      })

      expect(geo.width).toBe(350)
      expect(geo.physW).toBe(350)
      expect(geo.physX).toBe(450)
      expect(rightEdgeOf(geo)).toBe(
        defaultBase.startPhysX + defaultBase.startInnerW,
      )
    })

    it('向右挤压达到 MIN_WINDOW_WIDTH (271) 时，右边缘依然保持锁定不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'w',
        dx: 80, // 300 - 80 = 220 < 271，触发 clamp
        dy: 0,
      })

      expect(geo.width).toBe(MIN_WINDOW_WIDTH)
      expect(rightEdgeOf(geo)).toBe(
        defaultBase.startPhysX + defaultBase.startInnerW,
      )
    })

    it('向左拉伸达到 MAX_WINDOW_WIDTH (640) 时，右边缘依然保持锁定不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'w',
        dx: -400, // 300 - (-400) = 700 > 640，触发 clamp
        dy: 0,
      })

      expect(geo.width).toBe(MAX_WINDOW_WIDTH)
      expect(rightEdgeOf(geo)).toBe(
        defaultBase.startPhysX + defaultBase.startInnerW,
      )
    })
  })

  describe('北侧拖拽 (direction: n) - 底边缘绝对锁定', () => {
    it('向上拉伸增加高度并向上平移，底边缘坐标恒定', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'n',
        dx: 0,
        dy: -40,
      })

      expect(geo.height).toBe(240)
      expect(geo.physY).toBe(260)
      expect(bottomEdgeOf(geo)).toBe(
        defaultBase.startPhysY + defaultBase.startInnerH,
      )
    })

    it('向下挤压达到 MIN_WINDOW_HEIGHT (121) 时，底边缘依然保持锁定不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'n',
        dx: 0,
        dy: 90, // 200 - 90 = 110 < 121
      })

      expect(geo.height).toBe(MIN_WINDOW_HEIGHT)
      expect(bottomEdgeOf(geo)).toBe(
        defaultBase.startPhysY + defaultBase.startInnerH,
      )
    })

    it('向上拉伸达到 MAX_WINDOW_HEIGHT (860) 时，底边缘依然保持锁定不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'n',
        dx: 0,
        dy: -800, // 200 - (-800) = 1000 > 860
      })

      expect(geo.height).toBe(MAX_WINDOW_HEIGHT)
      expect(bottomEdgeOf(geo)).toBe(
        defaultBase.startPhysY + defaultBase.startInnerH,
      )
    })
  })

  describe('对角线与高 DPI 缩放场景', () => {
    it('西北角拖拽 (direction: nw) 同时保持右边与底边不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'nw',
        dx: -30,
        dy: -20,
      })

      expect(geo.width).toBe(330)
      expect(geo.height).toBe(220)
      expect(rightEdgeOf(geo)).toBe(
        defaultBase.startPhysX + defaultBase.startInnerW,
      )
      expect(bottomEdgeOf(geo)).toBe(
        defaultBase.startPhysY + defaultBase.startInnerH,
      )
    })

    it('150% DPI 下 physW/physH/physX/physY 全为整数物理像素，可直接下发', () => {
      const scaleFactor = 1.5
      const geo = computeResizeGeometry({
        startInnerW: Math.round(300 * scaleFactor), // 450
        startInnerH: Math.round(200 * scaleFactor), // 300
        startPhysX: 103, // 故意取非 1.5 整数倍的坐标
        startPhysY: 207,
        direction: 'nw',
        dx: 17, // 奇数像素移动
        dy: 11,
        scaleFactor,
      })

      expect(Number.isInteger(geo.physW)).toBe(true)
      expect(Number.isInteger(geo.physH)).toBe(true)
      expect(Number.isInteger(geo.physX)).toBe(true)
      expect(Number.isInteger(geo.physY)).toBe(true)
      expect(geo.physX).toBe(120) // 103 + 450 - 433
      expect(geo.physY).toBe(218) // 207 + 300 - 289
      // 精确锁定：不经 logical 往返，右边与底边一像素都不许动
      expect(rightEdgeOf(geo)).toBe(103 + 450)
      expect(bottomEdgeOf(geo)).toBe(207 + 300)
    })

    it('125% DPI 下西侧 clamp 到最小宽度后，右边缘仍然精确锁定（0 误差）', () => {
      const scaleFactor = 1.25
      const startInnerW = Math.round(300 * scaleFactor) // 375
      const startPhysX = 250
      const geo = computeResizeGeometry({
        startInnerW,
        startInnerH: 250,
        startPhysX,
        startPhysY: 100,
        direction: 'w',
        dx: 100, // 375 - 100 = 275 / 1.25 = 220 < 271，被 clamp
        dy: 0,
        scaleFactor,
      })

      expect(geo.width).toBe(MIN_WINDOW_WIDTH)
      expect(geo.physW).toBe(Math.round(MIN_WINDOW_WIDTH * scaleFactor)) // 339
      expect(rightEdgeOf(geo)).toBe(startPhysX + startInnerW)
    })

    it('150% DPI 下连续 100 帧小步拖拽，右边缘累积误差为 0（无漂移）', () => {
      const scaleFactor = 1.5
      const startInnerW = 450
      const startPhysX = 103
      const rightEdge = startPhysX + startInnerW

      for (let i = 1; i <= 100; i += 1) {
        const geo = computeResizeGeometry({
          startInnerW,
          startInnerH: 300,
          startPhysX,
          startPhysY: 207,
          direction: 'w',
          dx: i, // 每帧挪 1 物理像素
          dy: 0,
          scaleFactor,
        })
        expect(rightEdgeOf(geo)).toBe(rightEdge)
      }
    })

    it('非拖拽维度原样透传：最小宽度下拖上下边，宽度不得被重新 clamp 顶大 1px', () => {
      const scaleFactor = 1.5
      // 窗口停在最小宽度：OS 的最小跟踪尺寸 271 * 1.5 = 406.5，实际截断为 406 物理像素
      const startInnerW = 406
      expect(startInnerW / scaleFactor).toBeLessThan(MIN_WINDOW_WIDTH) // 270.67

      const geo = computeResizeGeometry({
        startInnerW,
        startInnerH: 300,
        startPhysX: 100,
        startPhysY: 200,
        direction: 'n',
        dx: 0,
        dy: -50,
        scaleFactor,
      })

      expect(geo.physW).toBe(startInnerW) // 不被 clamp 顶成 407
      expect(geo.physH).toBe(350) // 300 - (-50)
      expect(bottomEdgeOf(geo)).toBe(200 + 300)
    })

    it('非拖拽维度原样透传：最小高度下拖左右边，高度不得被顶大 1px', () => {
      const scaleFactor = 1.25
      const startInnerH = 151 // 121 * 1.25 = 151.25 → 实际 151，回算 120.8 < 121
      expect(startInnerH / scaleFactor).toBeLessThan(MIN_WINDOW_HEIGHT)

      const geo = computeResizeGeometry({
        startInnerW: 400,
        startInnerH,
        startPhysX: 100,
        startPhysY: 200,
        direction: 'e',
        dx: 30,
        dy: 0,
        scaleFactor,
      })

      expect(geo.physH).toBe(startInnerH)
      expect(geo.physW).toBe(430)
      expect(geo.physX).toBeNull()
    })
  })
})
