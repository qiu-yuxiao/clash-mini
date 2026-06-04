import { useEffect, useRef } from 'react'
import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window'
import { PhysicalPosition } from '@tauri-apps/api/dpi'

interface MonitorInfo {
  position: { x: number; y: number }
  size: { width: number; height: number }
  scaleFactor: number
}

interface WindowSizeInfo {
  width: number
  height: number
}

export const useWindowSnap = (active: boolean = true) => {
  const currentWindow = getCurrentWindow()

  // Cache for monitor and window size
  const cacheRef = useRef<{
    monitor: MonitorInfo | null
    windowSize: WindowSizeInfo | null
    fetchPromise: Promise<void> | null
  }>({ monitor: null, windowSize: null, fetchPromise: null })

  const lastSnapRef = useRef<{ x: number; y: number } | null>(null)
  const isSnappingRef = useRef<boolean>(false)
  const breakCooldownRef = useRef<{
    left?: number
    right?: number
    top?: number
    bottom?: number
  }>({})

  // Helper to fetch dimensions and cache them
  const cacheDimensions = () => {
    if (cacheRef.current.fetchPromise) return cacheRef.current.fetchPromise

    const promise = (async () => {
      try {
        const monitor = await currentMonitor()
        const size = await currentWindow.outerSize()
        if (monitor && size) {
          cacheRef.current.monitor = {
            position: { x: monitor.position.x, y: monitor.position.y },
            size: { width: monitor.size.width, height: monitor.size.height },
            scaleFactor: monitor.scaleFactor || 1,
          }
          cacheRef.current.windowSize = {
            width: size.width,
            height: size.height,
          }
        }
      } catch (err) {
        console.warn('[Snap] Pre-cache failed:', err)
      } finally {
        cacheRef.current.fetchPromise = null
      }
    })()

    cacheRef.current.fetchPromise = promise
    return promise
  }

  useEffect(() => {
    if (!active) return

    // Register mousedown/mouseup to pre-cache dimensions
    const handleMouseDown = () => {
      cacheDimensions()
    }

    const handleMouseUp = () => {
      // Clear cache when drag finishes
      cacheRef.current = { monitor: null, windowSize: null, fetchPromise: null }
      lastSnapRef.current = null
      breakCooldownRef.current = {}
    }

    document.addEventListener('mousedown', handleMouseDown, { passive: true })
    document.addEventListener('mouseup', handleMouseUp, { passive: true })

    let isUnmounted = false
    let unlistenPromise: Promise<() => void> | null = null

    // Register window moved listener
    unlistenPromise = currentWindow.onMoved(async (event) => {
      if (isUnmounted || isSnappingRef.current) return

      const { x, y } = event.payload

      try {
        // Ensure cache is populated
        if (!cacheRef.current.monitor || !cacheRef.current.windowSize) {
          await cacheDimensions()
        }

        const monitor = cacheRef.current.monitor
        const windowSize = cacheRef.current.windowSize
        if (!monitor || !windowSize) return

        const scaleFactor = monitor.scaleFactor || 1
        const snapThreshold = 18 * scaleFactor
        const breakThreshold = 24 * scaleFactor // breakaway threshold

        const screenLeft = monitor.position.x
        const screenTop = monitor.position.y
        const screenRight = screenLeft + monitor.size.width
        const screenBottom = screenTop + monitor.size.height

        const windowWidth = windowSize.width
        const windowHeight = windowSize.height

        // If window is moved completely out of cached monitor bounds, update cache
        if (
          x < screenLeft - 150 ||
          x > screenRight + 150 ||
          y < screenTop - 150 ||
          y > screenBottom + 150
        ) {
          cacheRef.current = { monitor: null, windowSize: null, fetchPromise: null }
          await cacheDimensions()
          return
        }

        let targetX = x
        let targetY = y
        let snapX = false
        let snapY = false

        // Check if we are currently breaking away from a snapped state
        if (lastSnapRef.current) {
          const dx = Math.abs(x - lastSnapRef.current.x)
          const dy = Math.abs(y - lastSnapRef.current.y)

          // If user pulled window beyond break threshold, break the snap
          if (dx > breakThreshold || dy > breakThreshold) {
            lastSnapRef.current = null
            // Set cooldown to prevent immediate re-snapping
            breakCooldownRef.current = {
              left: Math.abs(x - screenLeft) < snapThreshold ? Date.now() + 600 : 0,
              right: Math.abs(x + windowWidth - screenRight) < snapThreshold ? Date.now() + 600 : 0,
              top: Math.abs(y - screenTop) < snapThreshold ? Date.now() + 600 : 0,
              bottom: Math.abs(y + windowHeight - screenBottom) < snapThreshold ? Date.now() + 600 : 0,
            }
            return
          }
        }

        const now = Date.now()
        const cooldown = breakCooldownRef.current

        // Snap Horizontally
        if (Math.abs(x - screenLeft) < snapThreshold) {
          if (!cooldown.left || now > cooldown.left) {
            targetX = screenLeft
            snapX = true
          }
        } else if (Math.abs(x + windowWidth - screenRight) < snapThreshold) {
          if (!cooldown.right || now > cooldown.right) {
            targetX = screenRight - windowWidth
            snapX = true
          }
        }

        // Snap Vertically
        if (Math.abs(y - screenTop) < snapThreshold) {
          if (!cooldown.top || now > cooldown.top) {
            targetY = screenTop
            snapY = true
          }
        } else if (Math.abs(y + windowHeight - screenBottom) < snapThreshold) {
          if (!cooldown.bottom || now > cooldown.bottom) {
            targetY = screenBottom - windowHeight
            snapY = true
          }
        }

        if (snapX || snapY) {
          // If already snapped here, do nothing
          if (
            lastSnapRef.current &&
            lastSnapRef.current.x === targetX &&
            lastSnapRef.current.y === targetY
          ) {
            return
          }

          lastSnapRef.current = { x: targetX, y: targetY }
          isSnappingRef.current = true

          await currentWindow.setPosition(new PhysicalPosition(targetX, targetY))

          setTimeout(() => {
            isSnappingRef.current = false
          }, 150)
        } else {
          lastSnapRef.current = null
        }
      } catch (e) {
        console.error('[Snap] Snap calculation failed:', e)
      }
    })

    return () => {
      isUnmounted = true
      document.removeEventListener('mousedown', handleMouseDown)
      document.removeEventListener('mouseup', handleMouseUp)
      unlistenPromise
        .then((unlisten) => unlisten())
        .catch((err) => console.warn('[Snap] Failed to clear listener:', err))
    }
  }, [currentWindow, active])
}
