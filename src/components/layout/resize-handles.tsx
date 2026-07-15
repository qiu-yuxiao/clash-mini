import React, { useCallback } from 'react'

import { useWindow } from '@/hooks/use-window'

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

export const ResizeHandles: React.FC = () => {
  const { maximized, currentWindow } = useWindow()

  const handleMouseDown = useCallback(
    (direction: Direction) => (e: React.MouseEvent) => {
      if (!currentWindow || e.button !== 0) return
      e.preventDefault()
      e.stopPropagation()
      currentWindow.startResizeDragging(direction as any).catch(() => {})
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
