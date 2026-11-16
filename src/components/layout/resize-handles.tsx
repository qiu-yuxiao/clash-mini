import type React from 'react'
import { useCallback, useEffect, useRef } from 'react'

import { useWindow } from '@/hooks/use-window'
import {
  beginWindowResize,
  endWindowResize,
  frontendLog,
  type ResizeDirection,
  stepWindowResize,
} from '@/services/cmds'
import { setWindowResizing } from '@/utils/window-resizing'

const HANDLE_SIZE = 10

const handles: { direction: ResizeDirection; style: React.CSSProperties }[] = [
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

interface ActiveGestureSession {
  seq: number
  pointerId: number
  ending: boolean
}

export const ResizeHandles: React.FC = () => {
  const { isLargeMode } = useWindow()
  const isMountedRef = useRef(true)
  const gestureSeqRef = useRef(0)
  const activeSessionRef = useRef<ActiveGestureSession | null>(null)
  const beginPromiseRef = useRef<{
    seq: number
    promise: Promise<boolean>
  } | null>(null)
  const rafIdRef = useRef<number | null>(null)
  const inFlightRef = useRef(false)
  const needsStepRef = useRef(false)
  const scheduleStepRef = useRef<((seq: number) => void) | null>(null)

  const runStep = useCallback(async (seq: number) => {
    rafIdRef.current = null
    const active = activeSessionRef.current
    if (!active || active.ending || active.seq !== seq) return
    if (inFlightRef.current) {
      needsStepRef.current = true
      return
    }

    inFlightRef.current = true
    needsStepRef.current = false
    try {
      await stepWindowResize(seq)
    } finally {
      inFlightRef.current = false
      const stillActive = activeSessionRef.current
      if (
        isMountedRef.current &&
        stillActive &&
        !stillActive.ending &&
        stillActive.seq === seq &&
        needsStepRef.current
      ) {
        needsStepRef.current = false
        scheduleStepRef.current?.(seq)
      }
    }
  }, [])

  const scheduleStep = useCallback(
    (seq: number) => {
      const active = activeSessionRef.current
      if (!active || active.ending || active.seq !== seq) return
      if (inFlightRef.current) {
        needsStepRef.current = true
        return
      }
      if (rafIdRef.current !== null) return
      rafIdRef.current = requestAnimationFrame(() => {
        void runStep(seq)
      })
    },
    [runStep],
  )

  scheduleStepRef.current = scheduleStep

  const finishGesture = useCallback(
    async (pointerId?: number, target?: HTMLElement) => {
      const active = activeSessionRef.current
      if (!active || active.ending) return
      if (pointerId !== undefined && active.pointerId !== pointerId) return

      active.ending = true
      const finishingSeq = active.seq
      activeSessionRef.current = null
      needsStepRef.current = false
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current)
        rafIdRef.current = null
      }

      if (target && pointerId !== undefined) {
        try {
          if (target.hasPointerCapture?.(pointerId)) {
            target.releasePointerCapture(pointerId)
          }
        } catch {
          // 捕获可能已被系统释放，忽略
        }
      }

      const pendingBegin = beginPromiseRef.current
      if (pendingBegin && pendingBegin.seq === finishingSeq) {
        await pendingBegin.promise.catch(() => false)
        if (beginPromiseRef.current?.seq === finishingSeq) {
          beginPromiseRef.current = null
        }
      }

      const snapshot = await endWindowResize(finishingSeq)
      if (!activeSessionRef.current) {
        setWindowResizing(false, snapshot ?? undefined)
      }
    },
    [],
  )

  useEffect(() => {
    isMountedRef.current = true
    return () => {
      isMountedRef.current = false
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current)
        rafIdRef.current = null
      }
      const active = activeSessionRef.current
      activeSessionRef.current = null
      needsStepRef.current = false
      if (active && !active.ending) {
        void endWindowResize(active.seq)
      }
      setWindowResizing(false)
    }
  }, [])

  const handlePointerDown = useCallback(
    (direction: ResizeDirection) => async (e: React.PointerEvent) => {
      if (e.button !== 0) return
      e.preventDefault()
      e.stopPropagation()

      // 必须在任何 await 之前同步建立指针捕获，防止 SyntheticEvent.currentTarget 置空
      const target = e.currentTarget as HTMLElement
      try {
        target.setPointerCapture(e.pointerId)
      } catch (err) {
        frontendLog('warn', `[ResizeHandle] setPointerCapture failed: ${err}`)
      }

      const prev = activeSessionRef.current
      if (prev && !prev.ending) {
        prev.ending = true
        void endWindowResize(prev.seq)
      }

      const seq = gestureSeqRef.current + 1
      gestureSeqRef.current = seq
      needsStepRef.current = false
      inFlightRef.current = false
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current)
        rafIdRef.current = null
      }

      activeSessionRef.current = {
        seq,
        pointerId: e.pointerId,
        ending: false,
      }
      setWindowResizing(true)

      const beginPromise = beginWindowResize(direction, seq)
      beginPromiseRef.current = { seq, promise: beginPromise }
      const ok = await beginPromise
      if (beginPromiseRef.current?.seq === seq) {
        beginPromiseRef.current = null
      }

      const currentActive = activeSessionRef.current
      if (!ok) {
        if (
          currentActive &&
          currentActive.seq === seq &&
          !currentActive.ending
        ) {
          activeSessionRef.current = null
          try {
            if (target.hasPointerCapture?.(e.pointerId)) {
              target.releasePointerCapture(e.pointerId)
            }
          } catch {
            // ignore
          }
          setWindowResizing(false)
        }
        return
      }

      if (
        isMountedRef.current &&
        currentActive &&
        currentActive.seq === seq &&
        !currentActive.ending &&
        needsStepRef.current
      ) {
        needsStepRef.current = false
        scheduleStep(seq)
      }
    },
    [scheduleStep],
  )

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      void finishGesture(e.pointerId, e.currentTarget as HTMLElement)
    },
    [finishGesture],
  )

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      const active = activeSessionRef.current
      if (!active || active.ending || e.pointerId !== active.pointerId) return

      if ((e.buttons & 1) === 0) {
        void finishGesture(e.pointerId, e.currentTarget as HTMLElement)
        return
      }

      if (beginPromiseRef.current?.seq === active.seq) {
        needsStepRef.current = true
        return
      }

      scheduleStep(active.seq)
    },
    [finishGesture, scheduleStep],
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
