import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'

export const useVisibility = () => {
  const [documentVisible, setDocumentVisible] = useState(() =>
    typeof document === 'undefined'
      ? true
      : document.visibilityState === 'visible',
  )
  const [isMinimized, setIsMinimized] = useState(false)

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

    const initTauri = async () => {
      try {
        const currentWindow = getCurrentWindow()
        const minimized = await currentWindow.isMinimized()
        if (active) {
          setIsMinimized(minimized)
        }

        const unR = await currentWindow.onResized(async () => {
          try {
            const min = await currentWindow.isMinimized()
            if (active) {
              setIsMinimized(min)
            }
          } catch {
            // ignore
          }
        })
        if (active) {
          unlistenResized = unR
        } else {
          unR()
        }

        const unF = await currentWindow.onFocusChanged(async () => {
          try {
            const min = await currentWindow.isMinimized()
            if (active) {
              setIsMinimized(min)
            }
          } catch {
            // ignore
          }
        })
        if (active) {
          unlistenFocus = unF
        } else {
          unF()
        }
      } catch {
        // Fallback for non-Tauri / browser / testing environments
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

  return documentVisible && !isMinimized
}
