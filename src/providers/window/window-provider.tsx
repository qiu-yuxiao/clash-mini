import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import debounce from '@/utils/debounce'
import getSystem from '@/utils/get-system'

import { WindowContext } from './window-context'

/** FEAT-003: Idle duration (ms) before chrome auto-hides */
const IDLE_HIDE_DELAY_MS = 10_000
/** Width threshold (CSS px) below which the window is in "traffic monitor" mode */
const MINIMAL_WIDTH_THRESHOLD = 290

export const WindowProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  // Guard: in non-Tauri environment (e.g. browser dev server), skip window operations
  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__
  const currentWindow = useMemo(() => (isTauri ? getCurrentWindow() : null), [isTauri])
  const [decorated, setDecorated] = useState<boolean | null>(() => {
    if (typeof window === 'undefined') return true
    const isTauriEnv = !!(window as any).__TAURI_INTERNALS__
    if (!isTauriEnv) return false // Render custom titlebar in browser dev server for testing
    const OS = getSystem()
    return OS === 'linux' ? false : true
  })
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
  /** FEAT-003: true once startDragging() has been called for the current press */
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

        // Track minimal width state
        const wasMinimal = isMinimalWidthRef.current
        isMinimalWidthRef.current = window.innerWidth <= MINIMAL_WIDTH_THRESHOLD

        // If window is no longer minimal AND chrome is hidden → restore
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
          console.warn('[WindowProvider] checkMaximized isMaximized failed:', err)
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
      dragStartedRef.current = false
      resetIdleTimer()
      // NOTE: Do NOT call startDragging() here.
      // Calling it on mousedown causes the OS to swallow subsequent
      // click/mouseup events, permanently breaking click-to-restore.
      // We defer to handleMouseMove and only start dragging after 5px.
    }

    const handleMouseMove = (e: MouseEvent) => {
      // When chrome is visible, just reset idle timer
      if (!isDecorationsHiddenRef.current) {
        resetIdleTimer()
        return
      }
      // In stealth mode: start dragging once threshold is exceeded
      if (
        mouseDownPosRef.current &&
        !dragStartedRef.current &&
        (e.buttons & 1) !== 0 // left button held
      ) {
        const dx = e.clientX - mouseDownPosRef.current.x
        const dy = e.clientY - mouseDownPosRef.current.y
        if (Math.sqrt(dx * dx + dy * dy) > 5) {
          const target = e.target as HTMLElement
          if (
            !target.closest(
              'button, a, input, select, textarea, [data-no-drag]',
            )
          ) {
            dragStartedRef.current = true
            currentWindow?.startDragging().catch(() => {})
          } else {
            // Interactive target: abort drag tracking
            mouseDownPosRef.current = null
          }
        }
      }
    }

    const handleMouseUp = (e: MouseEvent) => {
      // Click = mousedown + mouseup without drag → restore chrome
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

  // ── Decorations init ────────────────────────────────────────────────────────
  const refreshDecorated = useCallback(async () => {
    if (!currentWindow) return false
    try {
      const val = await currentWindow.isDecorated()
      setDecorated(val)
      return val
    } catch (err) {
      console.warn('[WindowProvider] refreshDecorated failed:', err)
      return false
    }
  }, [currentWindow])

  const toggleDecorations = useCallback(async () => {
    if (!currentWindow) return
    try {
      const currentVal = await currentWindow.isDecorated()
      await currentWindow.setDecorations(!currentVal)
      setDecorated(!currentVal)
    } catch (err) {
      console.warn('[WindowProvider] toggleDecorations failed:', err)
    }
  }, [currentWindow])

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
