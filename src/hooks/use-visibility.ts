import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'

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

  useEffect(() => {
    let active = true
    let unlistenResized: (() => void) | null = null
    let unlistenFocus: (() => void) | null = null

    const updateWindowState = async () => {
      try {
        const currentWindow = getCurrentWindow()
        const [minimized, visible] = await Promise.all([
          currentWindow.isMinimized(),
          currentWindow.isVisible(),
        ])
        if (active) {
          setIsMinimized(minimized)
          setIsWindowVisible(visible)
        }
      } catch {
        // ignore
      }
    }

    const initTauri = async () => {
      try {
        const currentWindow = getCurrentWindow()
        await updateWindowState()

        const unR = await currentWindow.onResized(async () => {
          await updateWindowState()
        })
        if (active) {
          unlistenResized = unR
        } else {
          unR()
        }

        const unF = await currentWindow.onFocusChanged(async () => {
          await updateWindowState()
        })
        if (active) {
          unlistenFocus = unF
        } else {
          unF()
        }
      } catch {
        // Fallback for non-Tauri
      }
    }

    initTauri()

    return () => {
      active = false
      if (unlistenResized) {
        unlistenResized()
      }
      if (unlistenFocus) {
        unlistenFocus()
      }
    }
  }, [])

  return debouncedVisible
}
