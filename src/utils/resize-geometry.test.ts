import { describe, expect, it } from 'vitest'

import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
} from '@/constants'

import { computeResizeGeometry } from './resize-geometry'

describe('computeResizeGeometry 窗口尺寸与位置重算', () => {
  const defaultBase = {
    startInnerW: 300,
    startInnerH: 200,
    startPhysX: 500,
    startPhysY: 300,
    scaleFactor: 1.0,
  }

  describe('东侧拖拽 (direction: e)', () => {
    it('向右拉伸增加宽度，高度与原点位置保持不变 (x/y 为 null)', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'e',
        dx: 50,
        dy: 0,
      })

      expect(geo.width).toBe(350)
      expect(geo.height).toBe(200)
      expect(geo.x).toBeNull()
      expect(geo.y).toBeNull()
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
      expect(geo.x).toBeNull()
    })

    it('向右拉伸受 MAX_WINDOW_WIDTH (640) 钳制', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'e',
        dx: 500, // 300 + 500 = 800 > 640
        dy: 0,
      })

      expect(geo.width).toBe(MAX_WINDOW_WIDTH)
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
      expect(geo.x).toBeNull()
      expect(geo.y).toBeNull()
      expect(geo.physX).toBeNull()
      expect(geo.physY).toBeNull()
    })

    it('向上收缩受 MIN_WINDOW_HEIGHT (135) 钳制', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 's',
        dx: 0,
        dy: -100, // 200 - 100 = 100 < 135
      })

      expect(geo.height).toBe(MIN_WINDOW_HEIGHT)
      expect(geo.y).toBeNull()
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

  describe('西侧拖拽 (direction: w) - 保持右边缘不动', () => {
    it('向左拉伸增加宽度并向左平移，右边缘坐标恒定', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'w',
        dx: -50,
        dy: 0,
      })

      expect(geo.width).toBe(350)
      expect(geo.physX).toBe(450)
      // 右边缘：physX + width * sf 必须等于 startPhysX + startInnerW
      expect(geo.physX! + geo.width * defaultBase.scaleFactor).toBe(
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
      // 验证右边缘锁定
      const rightEdge = geo.physX! + geo.width * defaultBase.scaleFactor
      expect(rightEdge).toBe(defaultBase.startPhysX + defaultBase.startInnerW)
    })

    it('向左拉伸达到 MAX_WINDOW_WIDTH (640) 时，右边缘依然保持锁定不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'w',
        dx: -400, // 300 - (-400) = 700 > 640，触发 clamp
        dy: 0,
      })

      expect(geo.width).toBe(MAX_WINDOW_WIDTH)
      const rightEdge = geo.physX! + geo.width * defaultBase.scaleFactor
      expect(rightEdge).toBe(defaultBase.startPhysX + defaultBase.startInnerW)
    })
  })

  describe('北侧拖拽 (direction: n) - 保持底边缘不动', () => {
    it('向上拉伸增加高度并向上平移，底边缘坐标恒定', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'n',
        dx: 0,
        dy: -40,
      })

      expect(geo.height).toBe(240)
      expect(geo.physY).toBe(260)
      // 底边缘：physY + height * sf 必须等于 startPhysY + startInnerH
      expect(geo.physY! + geo.height * defaultBase.scaleFactor).toBe(
        defaultBase.startPhysY + defaultBase.startInnerH,
      )
    })

    it('向下挤压达到 MIN_WINDOW_HEIGHT (135) 时，底边缘依然保持锁定不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'n',
        dx: 0,
        dy: 90, // 200 - 90 = 110 < 135
      })

      expect(geo.height).toBe(MIN_WINDOW_HEIGHT)
      const bottomEdge = geo.physY! + geo.height * defaultBase.scaleFactor
      expect(bottomEdge).toBe(defaultBase.startPhysY + defaultBase.startInnerH)
    })

    it('向上拉伸达到 MAX_WINDOW_HEIGHT (860) 时，底边缘依然保持锁定不动', () => {
      const geo = computeResizeGeometry({
        ...defaultBase,
        direction: 'n',
        dx: 0,
        dy: -800, // 200 - (-800) = 1000 > 860
      })

      expect(geo.height).toBe(MAX_WINDOW_HEIGHT)
      const bottomEdge = geo.physY! + geo.height * defaultBase.scaleFactor
      expect(bottomEdge).toBe(defaultBase.startPhysY + defaultBase.startInnerH)
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
      expect(geo.physX! + geo.width * defaultBase.scaleFactor).toBe(
        defaultBase.startPhysX + defaultBase.startInnerW,
      )
      expect(geo.physY! + geo.height * defaultBase.scaleFactor).toBe(
        defaultBase.startPhysY + defaultBase.startInnerH,
      )
    })

    it('150% DPI 缩放下，physX 和 physY 始终为规整的物理像素整数，杜绝浮点抖动', () => {
      const scaleFactor = 1.5
      // 模拟物理像素初值与位移
      const geo = computeResizeGeometry({
        startInnerW: Math.round(300 * scaleFactor), // 450
        startInnerH: Math.round(200 * scaleFactor), // 300
        startPhysX: 103, // 任意非 1.5 整数倍的坐标
        startPhysY: 207,
        direction: 'w',
        dx: 17, // 奇数像素移动
        dy: 0,
        scaleFactor,
      })

      expect(Number.isInteger(geo.physX)).toBe(true)
      expect(Number.isInteger(geo.physY)).toBe(true)
      expect(geo.physX).toBe(120) // 103 + 17 = 120
    })

    it('125% DPI 缩放下，西侧 clamp 到最小宽度后右边缘无跳动', () => {
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
      // 物理右边缘：physX + Math.round(clampedW * sf) 与 startPhysX + startInnerW 误差不超过 1px (四舍五入规整)
      const expectedRight = startPhysX + startInnerW
      const actualRight = geo.physX! + Math.round(geo.width * scaleFactor)
      expect(Math.abs(actualRight - expectedRight)).toBeLessThanOrEqual(1)
    })
  })
})
