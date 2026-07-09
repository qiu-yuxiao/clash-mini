import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import DelayManager from '@/services/delay'
import debounce from '@/utils/debounce'
import getSystem from '@/utils/get-system'

import { WindowContext } from './window-context'
import { MINI_WIDTH_THRESHOLD, MINI_HEIGHT_THRESHOLD } from '@/constants'

/** FEAT-003: Idle duration (ms) before chrome auto-hides */
const IDLE_HIDE_DELAY_MS = 10_000

const OS = getSystem()
const IS_MACOS = OS === 'macos'

export const WindowProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const isTauri = typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__
  const currentWindow = useMemo(
    () => (isTauri ? getCurrentWindow() : null),
    [isTauri],
  )

  /**
   * Whether the window has native decorations (title bar + borders).
   * Windows/Linux: always frameless (custom titlebar).
   * macOS: always uses native titlebar.
   */
  const decorated: boolean = IS_MACOS

  const [maximized, setMaximized] = useState<boolean | null>(null)
  /** FEAT-003: true when custom titlebar is hidden by idle auto-hide timer (stealth mode) */
  const [isDecorationsHidden, setIsDecorationsHidden] = useState(false)

  const idleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isDecorationsHiddenRef = useRef(false)
  const isMinimalWidthRef = useRef(
    typeof window !== 'undefined'
      ? window.innerWidth <= MINI_WIDTH_THRESHOLD
      : false,
  )
  // ── Drag-vs-click detection (for stealth mode) ────────────────────────────
  const mouseDownPosRef = useRef<{ x: number; y: number } | null>(null)
  const dragStartedRef = useRef(false)

  const close = useCallback(async () => {
    if (!currentWindow) return
    await new Promise((resolve) => setTimeout(resolve, 20))
    await currentWindow.close()
  }, [currentWindow])

  const minimize = useCallback(async () => {
    if (!currentWindow) return
    await new Promise((resolve) => setTimeout(resolve, 10))
    await currentWindow.minimize()
  }, [currentWindow])

  // ── Restore chrome (show custom titlebar) ─────────────────────────────────
  const restoreChrome = useCallback(() => {
    if (!isDecorationsHiddenRef.current) return
    isDecorationsHiddenRef.current = false
    setIsDecorationsHidden(false)
  }, [])

  // ── Resize listener: track maximized state + minimal width ──────────────────
  useEffect(() => {
    if (!currentWindow) return
    let isUnmounted = false
    let lastWidth = -1
    let lastHeight = -1

    const checkMaximized = debounce(
      async (event: { payload: { width: number; height: number } }) => {
        if (isUnmounted) return
        const { width, height } = event.payload
        if (width === lastWidth && height === lastHeight) return
        lastWidth = width
        lastHeight = height

        const wasMinimal = isMinimalWidthRef.current
        isMinimalWidthRef.current = window.innerWidth <= MINI_WIDTH_THRESHOLD

        if (
          wasMinimal &&
          !isMinimalWidthRef.current &&
          isDecorationsHiddenRef.current
        ) {
          restoreChrome()
        }

        try {
          const value = await currentWindow.isMaximized()
          if (!isUnmounted) {
            setMaximized(value)
          }
        } catch (err) {
          console.warn(
            '[WindowProvider] checkMaximized isMaximized failed:',
            err,
          )
        }
      },
      300,
    )

    const unlistenPromise = currentWindow.onResized(checkMaximized)

    return () => {
      isUnmounted = true
      unlistenPromise
        .then((unlisten) => unlisten())
        .catch((err) => console.warn('[WindowProvider] 清理监听器失败:', err))
    }
  }, [currentWindow, restoreChrome])

  // ── Reset idle timer ────────────────────────────────────────────────────────
  const resetIdleTimer = useCallback(() => {
    if (idleTimerRef.current !== null) clearTimeout(idleTimerRef.current)

    if (typeof window !== 'undefined') {
      isMinimalWidthRef.current = window.innerWidth <= MINI_WIDTH_THRESHOLD
    }

    idleTimerRef.current = setTimeout(() => {
      const currentIsMinimal =
        typeof window !== 'undefined'
          ? window.innerWidth <= MINI_WIDTH_THRESHOLD
          : false
      const currentIsMinimalHeight =
        typeof window !== 'undefined'
          ? window.innerHeight <= MINI_HEIGHT_THRESHOLD
          : false
      if (
        !currentIsMinimal ||
        !currentIsMinimalHeight ||
        isDecorationsHiddenRef.current
      )
        return
      isDecorationsHiddenRef.current = true
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setIsDecorationsHidden(true)
    }, IDLE_HIDE_DELAY_MS)
  }, [])

  // ── Activity listeners: reset timer on any user input ──────────────────────
  useEffect(() => {
    resetIdleTimer()

    const handleMouseDown = (e: MouseEvent) => {
      mouseDownPosRef.current = { x: e.clientX, y: e.clientY }
      dragStartedRef.current = false
      resetIdleTimer()
    }

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDecorationsHiddenRef.current) {
        resetIdleTimer()
        return
      }
      if (
        mouseDownPosRef.current &&
        !dragStartedRef.current &&
        (e.buttons & 1) !== 0
      ) {
        const dx = e.clientX - mouseDownPosRef.current.x
        const dy = e.clientY - mouseDownPosRef.current.y
        if (Math.sqrt(dx * dx + dy * dy) > 5) {
          const target = e.target as HTMLElement
          if (
            !target.closest(
              'button, a, input, select, textarea, [data-no-drag]',
            ) &&
            !DelayManager.isBatchTesting
          ) {
            dragStartedRef.current = true
            currentWindow
              ?.startDragging()
              .catch(() => console.warn('[window] startDragging failed'))
          } else {
            mouseDownPosRef.current = null
          }
        }
      }
    }

    const handleMouseUp = (e: MouseEvent) => {
      if (
        isDecorationsHiddenRef.current &&
        mouseDownPosRef.current &&
        !dragStartedRef.current
      ) {
        const dx = e.clientX - mouseDownPosRef.current.x
        const dy = e.clientY - mouseDownPosRef.current.y
        if (Math.sqrt(dx * dx + dy * dy) < 5) {
          const target = e.target as HTMLElement
          if (!target.closest('[data-no-drag]')) {
            restoreChrome()
          }
        }
      }
      mouseDownPosRef.current = null
      dragStartedRef.current = false
    }

    const handleKeyActivity = () => {
      if (!isDecorationsHiddenRef.current) resetIdleTimer()
    }

    document.addEventListener('mousedown', handleMouseDown, { passive: true })
    document.addEventListener('mousemove', handleMouseMove, { passive: true })
    document.addEventListener('mouseup', handleMouseUp, { passive: true })
    document.addEventListener('keydown', handleKeyActivity, { passive: true })

    return () => {
      if (idleTimerRef.current !== null) clearTimeout(idleTimerRef.current)
      document.removeEventListener('mousedown', handleMouseDown)
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.removeEventListener('keydown', handleKeyActivity)
    }
  }, [resetIdleTimer, restoreChrome, currentWindow])

  const toggleMaximize = useCallback(async () => {
    if (!currentWindow) return
    try {
      if (await currentWindow.isMaximized()) {
        await currentWindow.unmaximize()
        setMaximized(false)
      } else {
        await currentWindow.maximize()
        setMaximized(true)
      }
    } catch (err) {
      console.warn('[WindowProvider] toggleMaximize failed:', err)
    }
  }, [currentWindow])

  const toggleFullscreen = useCallback(async () => {
    if (!currentWindow) return
    try {
      await currentWindow.setFullscreen(!(await currentWindow.isFullscreen()))
    } catch (err) {
      console.warn('[WindowProvider] toggleFullscreen failed:', err)
    }
  }, [currentWindow])

  useEffect(() => {
    if (!currentWindow) return
    currentWindow.setMinimizable?.(true)
  }, [currentWindow])

  const contextValue = useMemo(
    () => ({
      decorated,
      maximized,
      isDecorationsHidden,
      minimize,
      close,
      toggleMaximize,
      toggleFullscreen,
      currentWindow,
    }),
    [
      decorated,
      maximized,
      isDecorationsHidden,
      minimize,
      close,
      toggleMaximize,
      toggleFullscreen,
      currentWindow,
    ],
  )

  return <WindowContext value={contextValue}>{children}</WindowContext>
}
