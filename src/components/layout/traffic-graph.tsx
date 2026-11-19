import { alpha, useTheme } from '@mui/material'
import {
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  type Ref,
} from 'react'

import { MINI_WIDTH_THRESHOLD } from '@/constants'
import {
  TRAFFIC_GRAPH_BUFFER_LEN,
  TRAFFIC_GRAPH_MIN_POINTS,
  computeTrafficGraphY,
  computeTrafficPointX,
  resolveTrafficVisiblePoints,
  sliceTrafficVisibleBuffer,
} from '@/utils/parse-traffic'
import { isWindowResizing } from '@/utils/window-resizing'
import { Traffic } from 'tauri-plugin-mihomo-api'

const refLineAlpha = 0.15
const wideGridAlpha = 0.18
const wideVerticalGridAlpha = 0.1

const refLineWidth = 1
const upLineAlpha = 0.8
const upLineWidth = 2.5
const downLineAlpha = 0.9
const downLineWidth = 2.5

const sampleIntervalMs = 1000
const frameIntervalMs = 1000 / 10
const animationDurationMs = sampleIntervalMs

const WIDE_SCALE_LEVELS = [
  { label: '1M', bps: 1_048_576 },
  { label: '10K', bps: 10_240 },
  { label: '100B', bps: 100 },
] as const

const zeroTraffic: Traffic = { up: 0, down: 0 }
const createDefaultList = () =>
  Array.from({ length: TRAFFIC_GRAPH_BUFFER_LEN }, () => ({ ...zeroTraffic }))

const hasTraffic = (traffic?: Traffic | null) =>
  (traffic?.up ?? 0) !== 0 || (traffic?.down ?? 0) !== 0

const hasRetainedTraffic = (list: Traffic[]) => list.some(hasTraffic)

const safeAlpha = (color: string, opacity: number): string => {
  try {
    return alpha(color, opacity)
  } catch {
    return 'transparent'
  }
}

export interface TrafficRef {
  appendData: (data: Traffic) => void
  toggleStyle: () => void
}

type TrafficValueKey = 'up' | 'down'

/**
 * 统一随窗宽自适应的实时滚动流量曲线组件：
 * - 跨 285px 断点不销毁重建、保留历史波形，同时兼顾手动拖拽与最大化切换；
 * - 窄窗（≤285px）呈极简 30s 双参考线；宽窗（>285px）自适应扩展至 60s 时间窗并展示对数刻度网格与渐变填充。
 */
export function TrafficGraph({
  ref,
  isMinimalWidth = true,
}: {
  ref?: Ref<TrafficRef>
  isMinimalWidth?: boolean
}) {
  const progressRef = useRef(0)
  const styleRef = useRef(true)
  const listRef = useRef<Traffic[]>(createDefaultList())
  const canvasRef = useRef<HTMLCanvasElement>(null!)
  const isMinimalWidthRef = useRef(isMinimalWidth)

  const cacheRef = useRef<Traffic | null>(null)
  const requestDrawRef = useRef<(animate?: boolean) => void>(() => {})
  const redrawImmediateRef = useRef<() => void>(() => {})
  const canvasMetricsRef = useRef<{ rect: DOMRect; dpr: number } | null>(null)

  const { palette } = useTheme()

  const handleToggleStyle = useCallback(() => {
    styleRef.current = !styleRef.current
    requestDrawRef.current(false)
  }, [])

  useImperativeHandle(
    ref,
    () => ({
      appendData: (data: Traffic) => {
        cacheRef.current = data
      },
      toggleStyle: handleToggleStyle,
    }),
    [handleToggleStyle],
  )

  useEffect(() => {
    isMinimalWidthRef.current = isMinimalWidth
    redrawImmediateRef.current()
  }, [isMinimalWidth])

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null
    let isPaused = false

    const handleData = () => {
      if (isPaused) return

      const data = cacheRef.current ?? zeroTraffic
      cacheRef.current = null

      const list = listRef.current
      const shouldAppend = hasTraffic(data) || hasRetainedTraffic(list)

      if (shouldAppend) {
        if (list.length >= TRAFFIC_GRAPH_BUFFER_LEN) list.shift()
        list.push(data)
        progressRef.current = 0
        requestDrawRef.current(true)
      }

      timer = setTimeout(handleData, sampleIntervalMs)
    }

    const handleVisibilityChange = () => {
      if (document.hidden) {
        isPaused = true
        if (timer) {
          clearTimeout(timer)
          timer = null
        }
      } else {
        isPaused = false
        handleData()
      }
    }

    handleData()
    document.addEventListener('visibilitychange', handleVisibilityChange)

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange)
      if (timer) clearTimeout(timer)
    }
  }, [])

  useEffect(() => {
    let raf = 0
    let frameTimer: ReturnType<typeof setTimeout> | null = null
    let resizeObserver: ResizeObserver | null = null
    let animationStart = 0
    let lastFrameTime = 0
    const canvas = canvasRef.current!

    if (!canvas) return

    const context = canvas.getContext('2d')!

    if (!context) return

    const refLineColor = palette.divider || 'rgba(0, 0, 0, 0.12)'
    const upLineColor = palette.secondary.main || '#9c27b0'
    const downLineColor = palette.primary.main || '#5b5c9d'
    const scaleTextColor = palette.text.secondary || refLineColor

    const cancelPendingDraw = () => {
      if (frameTimer !== null) {
        clearTimeout(frameTimer)
        frameTimer = null
      }

      if (raf) {
        cancelAnimationFrame(raf)
        raf = 0
      }
    }

    const updateCanvasMetrics = () => {
      const rect = canvas.getBoundingClientRect()
      const dpr = window.devicePixelRatio || 1
      canvasMetricsRef.current = { rect, dpr }
      return canvasMetricsRef.current
    }

    const drawGraph = (progress = progressRef.current) => {
      const list = listRef.current
      const lineStyle = styleRef.current

      const metrics = canvasMetricsRef.current || updateCanvasMetrics()
      const { rect, dpr } = metrics
      const cssWidth = rect.width
      const cssHeight = rect.height
      if (cssWidth <= 0 || cssHeight <= 0) return

      const pixelWidth = Math.max(1, Math.floor(cssWidth * dpr))
      const pixelHeight = Math.max(1, Math.floor(cssHeight * dpr))

      if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
        canvas.width = pixelWidth
        canvas.height = pixelHeight
      }

      context.save()
      context.setTransform(1, 0, 0, 1, 0, 0)
      context.scale(dpr, dpr)
      context.clearRect(0, 0, cssWidth, cssHeight)

      const width = cssWidth
      const height = cssHeight
      // 拖拽期间 React 侧 isMinimalWidth 按 BUG-386 刻意冻结，实时改用 innerWidth 判宽确保手拖边框即时切换形态；
      // 非拖拽期（含最大化/还原点击）以 WindowProvider 物理像素精确断点的 !isMinimalWidthRef.current 为准。
      const isWide = isWindowResizing()
        ? typeof window !== 'undefined' &&
          window.innerWidth > MINI_WIDTH_THRESHOLD
        : !isMinimalWidthRef.current
      const visiblePoints = isWide
        ? resolveTrafficVisiblePoints(width)
        : TRAFFIC_GRAPH_MIN_POINTS
      const points = sliceTrafficVisibleBuffer(list, visiblePoints)
      const dx = width / visiblePoints

      const traceCurvePath = (
        seriesPoints: Traffic[],
        valueKey: TrafficValueKey,
      ) => {
        if (seriesPoints.length === 0) return

        const firstX = computeTrafficPointX(0, dx, progress)
        const firstY = computeTrafficGraphY(
          seriesPoints[0]?.[valueKey] ?? 0,
          height,
        )
        context.moveTo(firstX, firstY)

        if (lineStyle) {
          for (let i = 1; i < seriesPoints.length; i++) {
            const p1x = computeTrafficPointX(i, dx, progress)
            const p1y = computeTrafficGraphY(
              seriesPoints[i]?.[valueKey] ?? 0,
              height,
            )

            const hasNext = i + 1 < seriesPoints.length
            const p2x = hasNext
              ? computeTrafficPointX(i + 1, dx, progress)
              : p1x
            const p2y = hasNext
              ? computeTrafficGraphY(
                  seriesPoints[i + 1]?.[valueKey] ?? 0,
                  height,
                )
              : p1y

            context.quadraticCurveTo(p1x, p1y, (p1x + p2x) / 2, (p1y + p2y) / 2)
          }
        } else {
          for (let i = 1; i < seriesPoints.length; i++) {
            context.lineTo(
              computeTrafficPointX(i, dx, progress),
              computeTrafficGraphY(seriesPoints[i]?.[valueKey] ?? 0, height),
            )
          }
        }
      }

      // 背景参考线与刻度网格
      context.beginPath()
      context.lineWidth = refLineWidth
      context.strokeStyle = refLineColor
      if (isWide) {
        context.globalAlpha = wideGridAlpha
        for (const level of WIDE_SCALE_LEVELS) {
          const y = computeTrafficGraphY(level.bps, height)
          context.moveTo(0, y)
          context.lineTo(width, y)
        }
        context.stroke()
        context.closePath()

        context.beginPath()
        context.globalAlpha = wideVerticalGridAlpha
        for (let s = 10; s < visiblePoints; s += 10) {
          const x = width - s * dx
          context.moveTo(x, 0)
          context.lineTo(x, height)
        }
        context.stroke()
        context.closePath()
      } else {
        const dy = height / 7
        const l1 = dy
        const l2 = dy * 4
        context.globalAlpha = refLineAlpha
        context.moveTo(0, l1)
        context.lineTo(width, l1)
        context.moveTo(0, l2)
        context.lineTo(width, l2)
        context.stroke()
        context.closePath()
      }

      // 宽窗下先填充面积渐变，再统一描边曲线，保持曲线前景清晰
      if (isWide && points.length > 0) {
        const baselineY = height - 2
        const firstX = computeTrafficPointX(0, dx, progress)
        const lastX = computeTrafficPointX(points.length - 1, dx, progress)

        const fillSeriesArea = (
          valueKey: TrafficValueKey,
          color: string,
          topOpacity: number,
        ) => {
          const gradient = context.createLinearGradient(0, 0, 0, height)
          gradient.addColorStop(0, safeAlpha(color, topOpacity))
          gradient.addColorStop(1, safeAlpha(color, 0))
          context.beginPath()
          traceCurvePath(points, valueKey)
          context.lineTo(lastX, baselineY)
          context.lineTo(firstX, baselineY)
          context.closePath()
          context.globalAlpha = 1
          context.fillStyle = gradient
          context.fill()
        }

        fillSeriesArea('down', downLineColor, 0.2)
        fillSeriesArea('up', upLineColor, 0.16)
      }

      context.beginPath()
      context.globalAlpha = upLineAlpha
      context.lineWidth = upLineWidth
      context.strokeStyle = upLineColor
      traceCurvePath(points, 'up')
      context.stroke()
      context.closePath()

      context.beginPath()
      context.globalAlpha = downLineAlpha
      context.lineWidth = downLineWidth
      context.strokeStyle = downLineColor
      traceCurvePath(points, 'down')
      context.stroke()
      context.closePath()

      // 宽窗下补充对数纵轴刻度标识与实时时间跨度角标
      if (isWide && height >= 60) {
        context.fillStyle = scaleTextColor
        context.font =
          "9px -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif"
        context.globalAlpha = 0.55
        context.textAlign = 'left'
        context.textBaseline = 'bottom'
        for (const level of WIDE_SCALE_LEVELS) {
          const y = computeTrafficGraphY(level.bps, height)
          context.fillText(level.label, 4, Math.max(11, y - 2))
        }

        context.font =
          "bold 9px -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif"
        context.globalAlpha = 0.6
        context.textAlign = 'right'
        context.textBaseline = 'top'
        context.fillText(`${visiblePoints}s`, width - 4, 3)
      }

      context.restore()
    }

    const drawAnimatedFrame = (timestamp: number) => {
      raf = 0

      const timeSinceLastFrame = timestamp - lastFrameTime
      if (timeSinceLastFrame < frameIntervalMs) {
        frameTimer = setTimeout(() => {
          frameTimer = null
          raf = requestAnimationFrame(drawAnimatedFrame)
        }, frameIntervalMs - timeSinceLastFrame)
        return
      }

      lastFrameTime = timestamp

      const progress = Math.min(
        (timestamp - animationStart) / animationDurationMs,
        1,
      )
      progressRef.current = progress
      drawGraph(progress)

      if (progress < 1) {
        raf = requestAnimationFrame(drawAnimatedFrame)
        return
      }

      progressRef.current = 1
    }

    const requestDraw = (animate = false) => {
      cancelPendingDraw()

      if (!animate) {
        raf = requestAnimationFrame(() => {
          raf = 0
          updateCanvasMetrics()
          drawGraph()
        })
        return
      }

      updateCanvasMetrics()
      animationStart = performance.now()
      lastFrameTime = animationStart - frameIntervalMs
      raf = requestAnimationFrame(drawAnimatedFrame)
    }

    const redrawImmediate = () => {
      canvasMetricsRef.current = null
      updateCanvasMetrics()
      drawGraph()
    }

    requestDrawRef.current = requestDraw
    redrawImmediateRef.current = redrawImmediate
    requestDraw(false)

    if (typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(redrawImmediate)
      resizeObserver.observe(canvas)
    }

    return () => {
      if (requestDrawRef.current === requestDraw) {
        requestDrawRef.current = () => {}
      }
      if (redrawImmediateRef.current === redrawImmediate) {
        redrawImmediateRef.current = () => {}
      }
      resizeObserver?.disconnect()
      cancelPendingDraw()
    }
  }, [
    palette.primary.main,
    palette.secondary.main,
    palette.divider,
    palette.text.secondary,
  ])

  return (
    <canvas
      ref={canvasRef}
      onClick={handleToggleStyle}
      style={{
        width: '100%',
        height: '100%',
        display: 'block',
        cursor: 'pointer',
      }}
    />
  )
}
