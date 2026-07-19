import {
  currentMonitor,
  LogicalPosition,
  LogicalSize,
} from '@tauri-apps/api/window'
import React, { useCallback, useEffect, useRef } from 'react'

import { useWindow } from '@/hooks/use-window'
import { frontendLog } from '@/services/cmds'
import { computeResizeGeometry } from '@/utils/resize-geometry'
import { setWindowResizing } from '@/utils/window-resizing'

const HANDLE_SIZE = 10

type Direction = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

const handles: { direction: Direction; style: React.CSSProperties }[] = [
  {
    direction: 'n',
    style: {
      top: 0,
      left: HANDLE_SIZE,
      right: HANDLE_SIZE,
      height: HANDLE_SIZE,
      cursor: 'ns-resize',
    },
  },
  {
    direction: 's',
    style: {
      bottom: 0,
      left: HANDLE_SIZE,
      right: HANDLE_SIZE,
      height: HANDLE_SIZE,
      cursor: 'ns-resize',
    },
  },
  {
    direction: 'e',
    style: {
      top: HANDLE_SIZE,
      right: 0,
      bottom: HANDLE_SIZE,
      width: HANDLE_SIZE,
      cursor: 'ew-resize',
    },
  },
  {
    direction: 'w',
    style: {
      top: HANDLE_SIZE,
      left: 0,
      bottom: HANDLE_SIZE,
      width: HANDLE_SIZE,
      cursor: 'ew-resize',
    },
  },
  {
    direction: 'ne',
    style: {
      top: 0,
      right: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nesw-resize',
    },
  },
  {
    direction: 'nw',
    style: {
      top: 0,
      left: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nwse-resize',
    },
  },
  {
    direction: 'se',
    style: {
      bottom: 0,
      right: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nwse-resize',
    },
  },
  {
    direction: 'sw',
    style: {
      bottom: 0,
      left: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nesw-resize',
    },
  },
]

interface ResizeSession {
  direction: Direction
  // 拖拽起始时窗口的 outer 位置与尺寸（physical px）
  startPhysX: number
  startPhysY: number
  startPhysW: number
  startPhysH: number
  // 拖拽起始鼠标屏幕坐标（physical px，与 outerPosition 同坐标系）
  startPointerX: number
  startPointerY: number
  // 缩放因子，用于 physical ↔ logical 转换
  scaleFactor: number
  // 拖拽期间使用的 pointerId，用于 setPointerCapture/releasePointerCapture
  pointerId: number
  // 当前帧待应用的 logical 尺寸/位置（rAF 节流）
  pendingW: number | null
  pendingH: number | null
  pendingX: number | null
  pendingY: number | null
  // rAF 调度句柄；非 null 表示已有帧排队或正在执行
  rafId: number | null
  // applyPending 是否在飞行中（IPC 未完成），防止下一帧 rAF 触发并发 IPC 调用堆积
  inFlight: boolean
}

export const ResizeHandles: React.FC = () => {
  const { isLargeMode, currentWindow } = useWindow()
  // session ref 持有整个拖拽期间的不可变快照，pointermove/up 通过闭包读取
  const sessionRef = useRef<ResizeSession | null>(null)
  // applyPending ref：用于打破 scheduleApply ↔ applyPending 循环依赖
  const applyPendingRef = useRef<
    ((session: ResizeSession) => Promise<void>) | null
  >(null)

  const scheduleApply = useCallback((session: ResizeSession) => {
    // 已有帧排队 或 IPC 飞行中：跳过，避免并发 IPC 堆积
    if (session.rafId !== null || session.inFlight) return
    session.rafId = requestAnimationFrame(() => {
      void applyPendingRef.current?.(session)
    })
  }, [])

  const applyPending = useCallback(
    async (session: ResizeSession) => {
      // 取出本帧的待应用值，立即清空，避免下一帧重复应用
      const w = session.pendingW
      const h = session.pendingH
      const x = session.pendingX
      const y = session.pendingY
      session.pendingW = null
      session.pendingH = null
      session.pendingX = null
      session.pendingY = null
      session.rafId = null
      session.inFlight = true
      try {
        if (!currentWindow) return
        // 任一维度需要更新才发 IPC，减少调用次数
        const needsSize = w !== null || h !== null
        const needsPos = x !== null || y !== null
        if (!needsSize && !needsPos) return
        if (needsSize) {
          await currentWindow.setSize(
            new LogicalSize(
              w ?? session.startPhysW / session.scaleFactor,
              h ?? session.startPhysH / session.scaleFactor,
            ),
          )
        }
        if (needsPos) {
          await currentWindow.setPosition(
            new LogicalPosition(
              x ?? session.startPhysX / session.scaleFactor,
              y ?? session.startPhysY / session.scaleFactor,
            ),
          )
        }
      } catch (err) {
        frontendLog(
          'error',
          `[ResizeHandle] setSize/setPosition failed: ${err}`,
        )
      } finally {
        session.inFlight = false
        // IPC 期间若又有新的 pending，立即再调度一帧应用，避免遗漏最后一帧
        if (
          session.pendingW !== null ||
          session.pendingH !== null ||
          session.pendingX !== null ||
          session.pendingY !== null
        ) {
          scheduleApply(session)
        }
      }
    },
    [currentWindow, scheduleApply],
  )

  // 每次 render 同步 ref，供 scheduleApply 在 rAF 回调中调用最新版本
  applyPendingRef.current = applyPending

  // 组件卸载（含拖拽中途因切模式等卸载）时复位 resizing 标志，
  // 避免标志残留为 true 导致后续 onResized/updateWindowState 永远跳过 IPC。
  useEffect(
    () => () => {
      setWindowResizing(false)
    },
    [],
  )

  const handlePointerDown = useCallback(
    (direction: Direction) => async (e: React.PointerEvent) => {
      if (!currentWindow || e.button !== 0) return
      e.preventDefault()
      e.stopPropagation()

      // 读取窗口当前 outer 位置/尺寸（physical px），所有读取并行发 IPC 减少往返
      const [pos, size, monitor] = await Promise.all([
        currentWindow.outerPosition(),
        currentWindow.outerSize(),
        currentMonitor(),
      ]).catch((err) => {
        frontendLog('error', `[ResizeHandle] read window state failed: ${err}`)
        return [null, null, null]
      })
      if (!pos || !size) return
      const scaleFactor = monitor?.scaleFactor || window.devicePixelRatio || 1

      // 锁定指针，确保 pointermove/up 即使移出 handle 也能收到
      const target = e.currentTarget as HTMLElement
      try {
        target.setPointerCapture(e.pointerId)
      } catch (err) {
        frontendLog('warn', `[ResizeHandle] setPointerCapture failed: ${err}`)
      }

      // 设置全局 resizing 标志：window-provider (onResized) 与 use-visibility
      // (updateWindowState) 在发起 IPC 前读取此标志，拖拽期间跳过 IPC，
      // 避免与拖拽自身的 setSize/setPosition 抢占连接。
      setWindowResizing(true)

      sessionRef.current = {
        direction,
        startPhysX: pos.x,
        startPhysY: pos.y,
        startPhysW: size.width,
        startPhysH: size.height,
        // pointer events 的 clientX/Y 是 CSS 像素（logical），
        // 转换为 physical 以与 outerPosition 同坐标系
        startPointerX: e.clientX * scaleFactor,
        startPointerY: e.clientY * scaleFactor,
        scaleFactor,
        pointerId: e.pointerId,
        pendingW: null,
        pendingH: null,
        pendingX: null,
        pendingY: null,
        rafId: null,
        inFlight: false,
      }
    },
    [currentWindow],
  )

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      const session = sessionRef.current
      if (!session || e.pointerId !== session.pointerId) return

      // 释放指针捕获
      const target = e.currentTarget as HTMLElement
      try {
        target.releasePointerCapture(session.pointerId)
      } catch (err) {
        frontendLog(
          'warn',
          `[ResizeHandle] releasePointerCapture failed: ${err}`,
        )
      }

      // 拖拽结束：先清除全局 resizing 标志，再应用最后一帧。
      // 必须在 applyPending 之前清除，否则收尾的 setSize/setPosition 触发的
      // onResized 仍会看到 __isResizing=true 而被跳过，导致最终尺寸状态
      // （最小/大窗口判定）不刷新。
      setWindowResizing(false)

      // 取消尚未触发的 rAF，立即应用最后一帧
      if (session.rafId !== null) {
        cancelAnimationFrame(session.rafId)
        session.rafId = null
      }
      void applyPending(session)

      sessionRef.current = null
    },
    [applyPending],
  )

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      const session = sessionRef.current
      if (!session || e.pointerId !== session.pointerId) return

      // 🛡️ 终极防线：如果检测到鼠标左键已释放（e.buttons 不含左键 1 掩码），
      // 说明 pointerup 已经丢失。立即在此主动触发收尾清理，斩断假拖动状态。
      if ((e.buttons & 1) === 0) {
        handlePointerUp(e)
        return
      }

      // 计算鼠标在 physical 坐标系下的位移
      const dx = e.clientX * session.scaleFactor - session.startPointerX
      const dy = e.clientY * session.scaleFactor - session.startPointerY

      // 计算 clamp 后的新几何（logical），写入 pending（rAF 节流后统一应用）
      const geo = computeResizeGeometry({
        direction: session.direction,
        startPhysW: session.startPhysW,
        startPhysH: session.startPhysH,
        startPhysX: session.startPhysX,
        startPhysY: session.startPhysY,
        dx,
        dy,
        scaleFactor: session.scaleFactor,
      })
      session.pendingW = geo.width
      session.pendingH = geo.height
      session.pendingX = geo.x
      session.pendingY = geo.y

      scheduleApply(session)
    },
    [scheduleApply, handlePointerUp],
  )

  if (isLargeMode) return null

  return (
    <>
      {handles.map(({ direction, style }) => (
        <div
          key={direction}
          data-no-drag="true"
          onPointerDown={handlePointerDown(direction)}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerUp}
          onLostPointerCapture={handlePointerUp}
          style={{
            position: 'absolute',
            zIndex: 9999,
            userSelect: 'none',
            touchAction: 'none',
            ...style,
          }}
        />
      ))}
    </>
  )
}
