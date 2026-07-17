import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { MINI_WIDTH_THRESHOLD, MINI_HEIGHT_THRESHOLD } from '@/constants'
import { frontendLog, withIpcTimeout } from '@/services/cmds'
import debounce from '@/utils/debounce'
import getSystem from '@/utils/get-system'

import { WindowContext } from './window-context'

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
  /** 窗口宽度 ≤ 285px（窄窗口）：控制路由表格、流量面板布局 */
  const [isMinimalWidth, setIsMinimalWidth] = useState(
    () =>
      typeof window !== 'undefined' &&
      window.innerWidth <= MINI_WIDTH_THRESHOLD,
  )
  /** 窗口宽 ≤ 285 且 高 ≤ 135（最小窗口）：触发数据精简 + DOM卸载 */
  const [isMiniStatus, setIsMiniStatus] = useState(
    () =>
      typeof window !== 'undefined' &&
      window.innerWidth <= MINI_WIDTH_THRESHOLD &&
      window.innerHeight <= MINI_HEIGHT_THRESHOLD,
  )

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

        // 同步窄窗口 / 最小窗口状态到 React state（供消费方订阅）
        const nextIsMinimalWidth = isMinimalWidthRef.current
        const nextIsMiniStatus =
          nextIsMinimalWidth && window.innerHeight <= MINI_HEIGHT_THRESHOLD
        setIsMinimalWidth((prev) =>
          prev !== nextIsMinimalWidth ? nextIsMinimalWidth : prev,
        )
        setIsMiniStatus((prev) =>
          prev !== nextIsMiniStatus ? nextIsMiniStatus : prev,
        )

        if (
          wasMinimal &&
          !isMinimalWidthRef.current &&
          isDecorationsHiddenRef.current
        ) {
          restoreChrome()
        }

        const t0 = performance.now()
        frontendLog(
          'info',
          `[WindowProvider] onResized -> isMaximized() calling...`,
        )
        try {
          const value = await withIpcTimeout(
            currentWindow.isMaximized(),
            5000,
            'isMaximized',
          )
          frontendLog(
            'info',
            `[WindowProvider] isMaximized() resolved = ${value}, took ${Math.round(performance.now() - t0)}ms`,
          )
          if (!isUnmounted) {
            setMaximized(value)
          }
        } catch (err) {
          frontendLog(
            'error',
            `[WindowProvider] isMaximized FAILED took ${Math.round(performance.now() - t0)}ms: ${err}`,
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

    // L-16: mousemove 事件触发非常频繁，用 throttle 限制 resetIdleTimer 调用频率
    // 100ms 节流对用户体验无感知，但能显著减少定时器重置次数
    let lastMouseMoveTime = 0
    const THROTTLE_MS = 100
    const throttledResetIdle = () => {
      const now = Date.now()
      if (now - lastMouseMoveTime >= THROTTLE_MS) {
        lastMouseMoveTime = now
        resetIdleTimer()
      }
    }

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDecorationsHiddenRef.current) {
        throttledResetIdle()
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
            )
          ) {
            dragStartedRef.current = true
            currentWindow?.startDragging().catch(() => {
              console.warn('[window] startDragging failed')
              dragStartedRef.current = false
            })
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

  // L-17: TOCTOU（检查-使用时间差）问题——isMaximized 状态与操作之间
  // 可能有窗口状态变化。影响很小，try-catch 已能安全处理边界情况。
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

  useEffect(() => {
    if (!currentWindow) return
    currentWindow.setMinimizable?.(true)
  }, [currentWindow])

  const contextValue = useMemo(
    () => ({
      decorated,
      maximized,
      isDecorationsHidden,
      isMinimalWidth,
      isMiniStatus,
      minimize,
      close,
      toggleMaximize,
      currentWindow,
    }),
    [
      decorated,
      maximized,
      isDecorationsHidden,
      isMinimalWidth,
      isMiniStatus,
      minimize,
      close,
      toggleMaximize,
      currentWindow,
    ],
  )

  return <WindowContext value={contextValue}>{children}</WindowContext>
}
