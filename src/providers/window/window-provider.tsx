import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import debounce from '@/utils/debounce'

import { WindowContext } from './window-context'

/** FEAT-003: Idle duration (ms) before chrome auto-hides */
const IDLE_HIDE_DELAY_MS = 10_000
/** Width threshold (CSS px) below which the window is in "traffic monitor" mode */
const MINIMAL_WIDTH_THRESHOLD = 290

export const WindowProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const currentWindow = useMemo(() => getCurrentWindow(), [])
  const [decorated, setDecorated] = useState<boolean | null>(null)
  const [maximized, setMaximized] = useState<boolean | null>(null)
  /** FEAT-003: true when we have hidden the native chrome via idle timer */
  const [isDecorationsHidden, setIsDecorationsHidden] = useState(false)

  const idleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isDecorationsHiddenRef = useRef(false)
  const isMinimalWidthRef = useRef(
    typeof window !== 'undefined'
      ? window.innerWidth <= MINIMAL_WIDTH_THRESHOLD
      : false,
  )

  // ── Drag-vs-click detection ─────────────────────────────────────────────────
  const mouseDownPosRef = useRef<{ x: number; y: number } | null>(null)

  const close = useCallback(async () => {
    await new Promise((resolve) => setTimeout(resolve, 20))
    await currentWindow.close()
  }, [currentWindow])

  const minimize = useCallback(async () => {
    await new Promise((resolve) => setTimeout(resolve, 10))
    await currentWindow.minimize()
  }, [currentWindow])

  // ── Resize listener: track maximized state + minimal width ──────────────────
  useEffect(() => {
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

        // Track minimal width state
        const wasMinimal = isMinimalWidthRef.current
        isMinimalWidthRef.current = window.innerWidth <= MINIMAL_WIDTH_THRESHOLD

        // If window is no longer minimal AND chrome is hidden → restore
        if (wasMinimal && !isMinimalWidthRef.current && isDecorationsHiddenRef.current) {
          restoreChrome()
        }

        const value = await currentWindow.isMaximized()
        setMaximized(value)
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
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentWindow])

  // ── Restore chrome ──────────────────────────────────────────────────────────
  const restoreChrome = useCallback(() => {
    if (!isDecorationsHiddenRef.current) return
    isDecorationsHiddenRef.current = false
    setIsDecorationsHidden(false)
    setDecorated(true)
    ;(async () => {
      try {
        await invoke('restore_window_chrome')
      } catch (err) {
        console.warn('[WindowProvider] restore_window_chrome failed:', err)
      }
    })()
  }, [])

  // ── Reset idle timer ────────────────────────────────────────────────────────
  const resetIdleTimer = useCallback(() => {
    if (idleTimerRef.current !== null) clearTimeout(idleTimerRef.current)

    idleTimerRef.current = setTimeout(async () => {
      // Only hide if currently at minimal width and not already hidden
      if (!isMinimalWidthRef.current || isDecorationsHiddenRef.current) return
      try {
        await invoke('hide_window_chrome')
        isDecorationsHiddenRef.current = true
        setIsDecorationsHidden(true)
        setDecorated(false)
      } catch (err) {
        console.warn('[WindowProvider] hide_window_chrome failed:', err)
      }
    }, IDLE_HIDE_DELAY_MS)
  }, [])

  // ── Activity listeners: reset timer on any user input ──────────────────────
  useEffect(() => {
    // Start the idle timer on mount
    resetIdleTimer()

    const handleMouseDown = (e: MouseEvent) => {
      mouseDownPosRef.current = { x: e.clientX, y: e.clientY }
      resetIdleTimer()

      // In stealth mode, start window drag for any non-interactive target
      // so the entire window is draggable (buttons/links still handle their own clicks)
      if (isDecorationsHiddenRef.current) {
        const target = e.target as HTMLElement
        if (!target.closest('button, a, input, select, textarea')) {
          currentWindow.startDragging().catch(() => {})
        }
      }
    }

    const handleClick = (e: MouseEvent) => {
      // Single click (not drag) in stealth mode → restore chrome
      if (isDecorationsHiddenRef.current && mouseDownPosRef.current) {
        const dx = e.clientX - mouseDownPosRef.current.x
        const dy = e.clientY - mouseDownPosRef.current.y
        const dist = Math.sqrt(dx * dx + dy * dy)
        if (dist < 5) {
          // It's a genuine click, not a drag
          restoreChrome()
        }
      }
      mouseDownPosRef.current = null
    }

    const handleActivity = () => {
      if (!isDecorationsHiddenRef.current) resetIdleTimer()
    }

    document.addEventListener('mousedown', handleMouseDown, { passive: true })
    document.addEventListener('click', handleClick, { capture: true })
    document.addEventListener('mousemove', handleActivity, { passive: true })
    document.addEventListener('keydown', handleActivity, { passive: true })

    return () => {
      if (idleTimerRef.current !== null) clearTimeout(idleTimerRef.current)
      document.removeEventListener('mousedown', handleMouseDown)
      document.removeEventListener('click', handleClick, { capture: true })
      document.removeEventListener('mousemove', handleActivity)
      document.removeEventListener('keydown', handleActivity)
    }
  }, [resetIdleTimer, restoreChrome, currentWindow])

  // ── Decorations init ────────────────────────────────────────────────────────
  const refreshDecorated = useCallback(async () => {
    const val = await currentWindow.isDecorated()
    setDecorated(val)
    return val
  }, [currentWindow])

  const toggleDecorations = useCallback(async () => {
    const currentVal = await currentWindow.isDecorated()
    await currentWindow.setDecorations(!currentVal)
    setDecorated(!currentVal)
  }, [currentWindow])

  const toggleMaximize = useCallback(async () => {
    if (await currentWindow.isMaximized()) {
      await currentWindow.unmaximize()
      setMaximized(false)
    } else {
      await currentWindow.maximize()
      setMaximized(true)
    }
  }, [currentWindow])

  const toggleFullscreen = useCallback(async () => {
    await currentWindow.setFullscreen(!(await currentWindow.isFullscreen()))
  }, [currentWindow])

  useEffect(() => {
    refreshDecorated()
    currentWindow.setMinimizable?.(true)
  }, [currentWindow, refreshDecorated])

  const contextValue = useMemo(
    () => ({
      decorated,
      maximized,
      isDecorationsHidden,
      toggleDecorations,
      refreshDecorated,
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
      toggleDecorations,
      refreshDecorated,
      minimize,
      close,
      toggleMaximize,
      toggleFullscreen,
      currentWindow,
    ],
  )

  return <WindowContext value={contextValue}>{children}</WindowContext>
}
