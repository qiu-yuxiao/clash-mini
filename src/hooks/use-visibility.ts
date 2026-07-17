import { getCurrentWindow } from '@tauri-apps/api/window'
import { useCallback, useEffect, useRef, useState } from 'react'

import { frontendLog } from '@/services/cmds'

/**
 * useVisibility
 *
 * Hook to track whether the window is visible to the user.
 * Dis disconnection/reconnection behavior will only trigger if the window is minimized or hidden.
 * Pausing on window focus loss (Focused(false)) is prohibited to support multi-monitor setups.
 * State updates are debounced by 1000ms to prevent connection thrashing.
 */
export const useVisibility = () => {
  const [documentVisible, setDocumentVisible] = useState(() =>
    typeof document === 'undefined'
      ? true
      : document.visibilityState === 'visible',
  )
  const [isMinimized, setIsMinimized] = useState(false)
  const [isWindowVisible, setIsWindowVisible] = useState(true)

  const rawVisible = documentVisible && isWindowVisible && !isMinimized
  const [debouncedVisible, setDebouncedVisible] = useState(rawVisible)

  const isMountedRef = useRef(true)
  // L2-05: 第二个 effect 使用独立的 mounted ref，避免跨 effect 共享导致的状态错乱
  const tauriMountedRef = useRef(true)

  useEffect(() => {
    isMountedRef.current = true
    return () => {
      isMountedRef.current = false
    }
  }, [])

  useEffect(() => {
    if (rawVisible) {
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setDebouncedVisible(true)
    } else {
      const timer = setTimeout(() => {
        setDebouncedVisible(false)
      }, 1000)

      return () => clearTimeout(timer)
    }
  }, [rawVisible])

  useEffect(() => {
    const handleVisibilityChange = () => {
      setDocumentVisible(document.visibilityState === 'visible')
    }

    const handleFocus = () => setDocumentVisible(true)
    const handlePointerDown = () => setDocumentVisible(true)

    document.addEventListener('focus', handleFocus)
    document.addEventListener('pointerdown', handlePointerDown)
    document.addEventListener('visibilitychange', handleVisibilityChange)

    return () => {
      document.removeEventListener('focus', handleFocus)
      document.removeEventListener('pointerdown', handlePointerDown)
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [])

  // L-29: updateWindowState 使用 useCallback 包装并通过 isMountedRef 判断挂载状态
  // 避免闭包变量带来的潜在问题，同时确保回调引用稳定（虽然 useEffect 依赖为空本就只注册一次）
  const inFlightRef = useRef(false)
  const updateWindowState = useCallback(async () => {
    // 防止并发调用堆积：如果上一次 IPC 调用尚未返回，直接跳过
    // 这能防止 resize 期间的 IPC 洪水（onResized 每像素触发一次）
    if (inFlightRef.current) return
    inFlightRef.current = true
    try {
      const currentWindow = getCurrentWindow()
      const [minimized, visible] = await Promise.all([
        currentWindow.isMinimized(),
        currentWindow.isVisible(),
      ])
      if (isMountedRef.current) {
        setIsMinimized(minimized)
        setIsWindowVisible(visible)
      }
    } catch (err) {
      frontendLog('error', `[useVisibility] updateWindowState FAILED: ${err}`)
    } finally {
      inFlightRef.current = false
    }
  }, [])

  useEffect(() => {
    let unlistenResized: (() => void) | null = null
    let unlistenFocus: (() => void) | null = null

    const initTauri = async () => {
      try {
        const currentWindow = getCurrentWindow()
        await updateWindowState()

        const unR = await currentWindow.onResized(async () => {
          await updateWindowState()
        })
        if (tauriMountedRef.current) {
          unlistenResized = unR
        } else {
          unR()
        }

        const unF = await currentWindow.onFocusChanged(async () => {
          await updateWindowState()
        })
        if (tauriMountedRef.current) {
          unlistenFocus = unF
        } else {
          unF()
        }
      } catch {
        // Fallback for non-Tauri
      }
    }

    initTauri()

    tauriMountedRef.current = true
    return () => {
      tauriMountedRef.current = false
      if (unlistenResized) {
        unlistenResized()
      }
      if (unlistenFocus) {
        unlistenFocus()
      }
    }
  }, [updateWindowState])

  return debouncedVisible
}
