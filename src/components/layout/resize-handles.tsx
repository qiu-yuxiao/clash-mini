import { PhysicalPosition, PhysicalSize } from '@tauri-apps/api/dpi'
import React, { useCallback } from 'react'

import { useWindow } from '@/hooks/use-window'
import { frontendLog } from '@/services/cmds'

const HANDLE_SIZE = 10
const MIN_WIDTH = 285
const MIN_HEIGHT = 135

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

export const ResizeHandles: React.FC = () => {
  const { maximized, currentWindow } = useWindow()

  const handlePointerDown = useCallback(
    (direction: Direction) => (e: React.PointerEvent) => {
      if (!currentWindow || e.button !== 0) return
      e.preventDefault()
      e.stopPropagation()

      const target = e.currentTarget as HTMLElement
      const pointerId = e.pointerId

      frontendLog(
        'info',
        `[ResizeHandle] pointerdown dir=${direction}, custom resize starting...`,
      )

      // 捕获指针：确保 pointermove/pointerup 即使鼠标移出窗口也能触发
      // 这替代了 startResizeDragging 的 Windows 模态 resize 循环
      try {
        target.setPointerCapture(pointerId)
      } catch (err) {
        frontendLog('error', `[ResizeHandle] setPointerCapture failed: ${err}`)
        return
      }

      // 立即设置全局 resize 标志，防止 onResized 回调中的 IPC 调用
      window.__isResizing = true

      const startScreenX = e.screenX
      const startScreenY = e.screenY

      // 异步初始化窗口状态（在 await 完成前 update 不会执行）
      let scaleFactor = 1
      let startWidth = 0
      let startHeight = 0
      let startX = 0
      let startY = 0
      let initialized = false

      let rafId: number | null = null
      let pendingEvent: PointerEvent | null = null
      let pendingUpdate = false
      let cleanup: () => void = () => {}

      const update = () => {
        rafId = null
        if (!pendingEvent || !initialized || pendingUpdate) return
        const ev = pendingEvent
        pendingEvent = null
        pendingUpdate = true

        const dx = (ev.screenX - startScreenX) * scaleFactor
        const dy = (ev.screenY - startScreenY) * scaleFactor

        let newWidth = startWidth
        let newHeight = startHeight
        let newX = startX
        let newY = startY

        if (direction.includes('e')) newWidth = startWidth + dx
        if (direction.includes('s')) newHeight = startHeight + dy
        if (direction.includes('w')) {
          newWidth = startWidth - dx
          newX = startX + dx
        }
        if (direction.includes('n')) {
          newHeight = startHeight - dy
          newY = startY + dy
        }

        // 最小尺寸约束
        if (newWidth < MIN_WIDTH) {
          if (direction.includes('w')) newX = startX + (startWidth - MIN_WIDTH)
          newWidth = MIN_WIDTH
        }
        if (newHeight < MIN_HEIGHT) {
          if (direction.includes('n'))
            newY = startY + (startHeight - MIN_HEIGHT)
          newHeight = MIN_HEIGHT
        }

        const hasPositionChange =
          direction.includes('w') || direction.includes('n')
        const promises: Promise<unknown>[] = []
        if (hasPositionChange) {
          promises.push(
            currentWindow.setPosition(new PhysicalPosition(newX, newY)),
          )
        }
        promises.push(
          currentWindow.setSize(new PhysicalSize(newWidth, newHeight)),
        )
        Promise.all(promises).finally(() => {
          pendingUpdate = false
        })
      }

      const handlePointerMove = (ev: PointerEvent) => {
        pendingEvent = ev
        if (rafId === null) {
          rafId = requestAnimationFrame(update)
        }
      }

      const handlePointerUp = () => {
        if (rafId !== null) {
          cancelAnimationFrame(rafId)
          rafId = null
        }
        cleanup()
        frontendLog(
          'info',
          `[ResizeHandle] pointerup dir=${direction}, custom resize ended`,
        )
      }

      cleanup = () => {
        target.removeEventListener('pointermove', handlePointerMove)
        target.removeEventListener('pointerup', handlePointerUp)
        try {
          target.releasePointerCapture(pointerId)
        } catch {
          // pointer capture may already be released
        }
        window.__isResizing = false
      }

      // 异步获取窗口初始状态
      void (async () => {
        try {
          scaleFactor = await currentWindow.scaleFactor()
          const size = await currentWindow.outerSize()
          const pos = await currentWindow.outerPosition()
          startWidth = size.width
          startHeight = size.height
          startX = pos.x
          startY = pos.y
          initialized = true
        } catch (err) {
          frontendLog('error', `[ResizeHandle] init failed: ${err}`)
          cleanup()
        }
      })()

      target.addEventListener('pointermove', handlePointerMove)
      target.addEventListener('pointerup', handlePointerUp)
    },
    [currentWindow],
  )

  if (maximized) return null

  return (
    <>
      {handles.map(({ direction, style }) => (
        <div
          key={direction}
          data-no-drag="true"
          onPointerDown={handlePointerDown(direction)}
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
