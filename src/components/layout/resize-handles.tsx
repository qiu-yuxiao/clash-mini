import React, { useCallback } from 'react'

import { useWindow } from '@/hooks/use-window'
import { frontendLog } from '@/services/cmds'

const HANDLE_SIZE = 10

type Direction = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

// Tauri ResizeDirection type (not exported from @tauri-apps/api/window, mirrored here)
type ResizeDirection =
  | 'East'
  | 'North'
  | 'NorthEast'
  | 'NorthWest'
  | 'South'
  | 'SouthEast'
  | 'SouthWest'
  | 'West'

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

      // 映射为 Tauri v2 标准的拉伸方向参数
      const mapDir = (dir: Direction): string => {
        switch (dir) {
          case 'n':
            return 'Top'
          case 's':
            return 'Bottom'
          case 'e':
            return 'Right'
          case 'w':
            return 'Left'
          case 'ne':
            return 'TopRight'
          case 'nw':
            return 'TopLeft'
          case 'se':
            return 'BottomRight'
          case 'sw':
            return 'BottomLeft'
        }
      }

      // 直接调用 Tauri 原生的无损拖拽调整窗口大小 API
      // 这会开启底层的 Windows 硬件拉伸消息循环，零前端 pointermove 逻辑与 IPC 堆积
      currentWindow
        .startResizeDragging(mapDir(direction) as unknown as ResizeDirection)
        .catch((err) => {
          frontendLog(
            'error',
            `[ResizeHandle] startResizeDragging failed: ${err}`,
          )
        })
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
