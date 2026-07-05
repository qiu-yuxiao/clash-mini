import React, { useCallback, useEffect, useState } from 'react'

import DelayManager from '@/services/delay'
import { useWindow } from '@/hooks/use-window'

const HANDLE_SIZE = 6

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
  const [canResize, setCanResize] = useState(true)

  // Notify WindowProvider when a resize starts/ends
  const setResizeActive = useCallback((active: boolean) => {
    if (typeof window !== 'undefined' && (window as any).__setResizeActive) {
      ;(window as any).__setResizeActive(active)
    }
  }, [])

  useEffect(() => {
    const handleMouseUp = () => {
      setResizeActive(false)
    }
    document.addEventListener('mouseup', handleMouseUp)
    return () => {
      document.removeEventListener('mouseup', handleMouseUp)
    }
  }, [setResizeActive])

  useEffect(() => {
    if (!currentWindow) return
    currentWindow
      .isResizable()
      .then(setCanResize)
      .catch(() => {})
  }, [currentWindow])

  const handleMouseDown = useCallback(
    (direction: Direction) => (e: React.MouseEvent) => {
      if (!currentWindow || e.button !== 0) return
      if (DelayManager.isBatchTesting) return
      e.preventDefault()
      e.stopPropagation()
      setResizeActive(true)
      currentWindow
        .startResizeDragging(direction as any)
        .finally(() => {
          // 缩放模态循环退出后，切换 resizable 状态切断 Windows 可能立即发起的原生 move 操作
          currentWindow.setResizable(false).then(() => currentWindow.setResizable(true))
        })
        .catch(() => {})
    },
    [currentWindow, setResizeActive],
  )

  if (maximized || !canResize) return null

  return (
    <>
      {handles.map(({ direction, style }) => (
        <div
          key={direction}
          data-no-drag="true"
          onMouseDown={handleMouseDown(direction)}
          style={{
            position: 'absolute',
            zIndex: 9999,
            userSelect: 'none',
            ...style,
          }}
        />
      ))}
    </>
  )
}
