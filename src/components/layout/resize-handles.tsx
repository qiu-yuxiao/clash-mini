import { LogicalPosition, LogicalSize } from '@tauri-apps/api/window'
import React, { useCallback, useRef } from 'react'

import { useWindow } from '@/hooks/use-window'
import { frontendLog } from '@/services/cmds'

const HANDLE_SIZE = 10

// 窗口最小/最大尺寸（logical px），与后端 resolve/window.rs 对齐：
// MINIMAL_WIDTH=285.0, MINIMAL_HEIGHT=135.0, MAX_WIDTH=640.0, MAX_HEIGHT=860.0
const MIN_WIDTH = 285
const MIN_HEIGHT = 135
const MAX_WIDTH = 640
const MAX_HEIGHT = 860

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

  const handlePointerDown = useCallback(
    (direction: Direction) => async (e: React.PointerEvent) => {
      if (!currentWindow || e.button !== 0) return
      e.preventDefault()
      e.stopPropagation()

      // 读取窗口当前 outer 位置/尺寸（physical px），所有读取并行发 IPC 减少往返
      const [pos, size, monitor] = await Promise.all([
        currentWindow.outerPosition(),
        currentWindow.outerSize(),
        import('@tauri-apps/api/window').then((m) => m.currentMonitor()),
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

      // 设置全局 resizing 标志：use-layout-events 等订阅处会跳过 IPC 调用
      // 避免 onResized 触发的 updateWindowState 等 IPC 与拖拽的 setSize/setPosition 排队
      if (typeof window !== 'undefined') {
        ;(window as unknown as { __isResizing?: boolean }).__isResizing = true
      }

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

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      const session = sessionRef.current
      if (!session || e.pointerId !== session.pointerId) return
      // 计算鼠标在 physical 坐标系下的位移
      const dx = e.clientX * session.scaleFactor - session.startPointerX
      const dy = e.clientY * session.scaleFactor - session.startPointerY

      // 起始尺寸/位置（physical）
      const startW = session.startPhysW
      const startH = session.startPhysH
      const startX = session.startPhysX
      const startY = session.startPhysY

      // 计算新尺寸（physical，先按方向算 raw，再 clamp 到 min/max）
      let newW = startW
      let newH = startH
      let newX = startX
      let newY = startY

      const dir = session.direction
      if (dir.includes('e')) newW = startW + dx
      if (dir.includes('s')) newH = startH + dy
      if (dir.includes('w')) {
        newW = startW - dx
        newX = startX + dx
      }
      if (dir.includes('n')) {
        newH = startH - dy
        newY = startY + dy
      }

      // physical → logical 用于 clamp 比较（后端 max_inner_size 也是 logical）
      const sf = session.scaleFactor
      const logicalW = newW / sf
      const logicalH = newH / sf

      // clamp 尺寸到 [MIN, MAX]
      const clampedW = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, logicalW))
      const clampedH = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, logicalH))

      // 若尺寸被 clamp（例如达到最小宽度），需要相应回退位置以保持对边不动
      // 推导：右边缘 = newX + newW，clamp 后 newW 变成 clampedW*sf，
      //   为保持右边缘不动，newX 需减去 (newW - clampedW*sf) = -dwLogical*sf
      //   即 newX += dwLogical*sf（dwLogical 为负时往左推，正时往右推）
      // 下方 dwLogical = clampedW - logicalW，符号与上面推导对齐
      const dwLogical = clampedW - logicalW
      const dhLogical = clampedH - logicalH
      if (dwLogical !== 0 && dir.includes('w')) {
        newX -= dwLogical * sf
      }
      if (dhLogical !== 0 && dir.includes('n')) {
        newY -= dhLogical * sf
      }

      // 写入 pending（rAF 节流后统一应用），尺寸用 clamped 后的 logical
      session.pendingW = clampedW
      session.pendingH = clampedH
      // 只有当方向包含 w/n 时才会改位置
      if (dir.includes('w') || dir.includes('n')) {
        session.pendingX = newX / sf
        session.pendingY = newY / sf
      } else {
        session.pendingX = null
        session.pendingY = null
      }

      scheduleApply(session)
    },
    [scheduleApply],
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

      // 取消尚未触发的 rAF，立即应用最后一帧
      if (session.rafId !== null) {
        cancelAnimationFrame(session.rafId)
        session.rafId = null
      }
      void applyPending(session)

      // 清除全局 resizing 标志
      if (typeof window !== 'undefined') {
        ;(window as unknown as { __isResizing?: boolean }).__isResizing = false
      }

      sessionRef.current = null
    },
    [applyPending],
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
