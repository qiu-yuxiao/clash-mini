import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import debounce from '@/utils/debounce'

import { WindowContext } from './window-context'

/** FEAT-003: Idle duration (ms) before title bar auto-hides */
const IDLE_HIDE_DELAY_MS = 10_000

export const WindowProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const currentWindow = useMemo(() => getCurrentWindow(), [])
  const [decorated, setDecorated] = useState<boolean | null>(null)
  const [maximized, setMaximized] = useState<boolean | null>(null)
  /** FEAT-003: tracks whether we have actively hidden decorations via idle timer */
  const [isDecorationsHidden, setIsDecorationsHidden] = useState(false)
  const idleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isDecorationsHiddenRef = useRef(false)

  const close = useCallback(async () => {
    // Delay one frame so the UI can clear :hover before the window hides.
    await new Promise((resolve) => setTimeout(resolve, 20))
    await currentWindow.close()
  }, [currentWindow])

  const minimize = useCallback(async () => {
    // Delay one frame so the UI can clear :hover before the window hides.
    await new Promise((resolve) => setTimeout(resolve, 10))
    await currentWindow.minimize()
  }, [currentWindow])

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

  // ── FEAT-003: Idle auto-hide title bar ─────────────────────────────────────

  /** Restore title bar and reset idle timer */
  const restoreAndResetTimer = useCallback(() => {
    // Restore decorations if we hid them
    if (isDecorationsHiddenRef.current) {
      isDecorationsHiddenRef.current = false
      setIsDecorationsHidden(false)
      currentWindow.setDecorations(true).catch((err) =>
        console.warn('[WindowProvider] setDecorations(true) failed:', err),
      )
      setDecorated(true)
    }
    // Reset idle timer
    if (idleTimerRef.current !== null) {
      clearTimeout(idleTimerRef.current)
    }
    idleTimerRef.current = setTimeout(() => {
      // Only hide if window is currently decorated
      currentWindow.isDecorated().then((isDecorated) => {
        if (!isDecorated) return
        isDecorationsHiddenRef.current = true
        setIsDecorationsHidden(true)
        currentWindow.setDecorations(false).catch((err) =>
          console.warn('[WindowProvider] setDecorations(false) failed:', err),
        )
        setDecorated(false)
      })
    }, IDLE_HIDE_DELAY_MS)
  }, [currentWindow])

  useEffect(() => {
    // Start the idle timer immediately on mount
    restoreAndResetTimer()

    const handleActivity = () => restoreAndResetTimer()

    document.addEventListener('mousemove', handleActivity, { passive: true })
    document.addEventListener('mousedown', handleActivity, { passive: true })
    document.addEventListener('keydown', handleActivity, { passive: true })

    return () => {
      if (idleTimerRef.current !== null) clearTimeout(idleTimerRef.current)
      document.removeEventListener('mousemove', handleActivity)
      document.removeEventListener('mousedown', handleActivity)
      document.removeEventListener('keydown', handleActivity)
    }
  }, [restoreAndResetTimer])

  // ───────────────────────────────────────────────────────────────────────────

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
