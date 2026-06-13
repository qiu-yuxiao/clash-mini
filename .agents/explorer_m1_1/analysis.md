# Technical Analysis: Frontend Visibility State & Tauri Window Events Optimization

## 1. Executive Summary
This report analyzes the current implementation of `src/hooks/use-visibility.ts` and details a robust, performance-optimized strategy to integrate Tauri's window event listeners and state queries. The goal is to ensure that the hook `useVisibility` accurately returns `false` when the window is minimized or hidden, which in turn suspends CPU-intensive and high-frequency IPC/WebSocket operations (such as traffic monitor graphs, connections, and log subscriptions).

## 2. Current Implementation Analysis of `use-visibility.ts`
The current implementation of the `useVisibility` hook is purely DOM-based:
- It initializes state by checking if `document.visibilityState === 'visible'`.
- It registers three DOM event listeners on the `document`:
  - `visibilitychange`: updates the state based on `document.visibilityState === 'visible'`.
  - `focus`: sets the state to `true`.
  - `pointerdown`: sets the state to `true`.
- It cleans up these listeners on unmount.

### Key Limitations:
1. **Lack of Blur Handling**: There is no DOM `blur` event listener to set visibility to `false`. Once the window loses focus, it may remain marked as "visible" (which is appropriate if the window is side-by-side but out of focus). However, it does not handle OS-level minimization or hiding actions gracefully if they do not trigger a DOM `visibilitychange` event (which can happen under certain Tauri webview platforms or when minimized to the system tray).
2. **Ignorance of Tauri Window State**: The hook has no knowledge of whether the Tauri native window container itself is minimized (`isMinimized`) or hidden (`isHidden`). In Tauri apps, the webview container's DOM visibility state does not always sync reliably with the host OS window minimization, causing background resources (like WebSockets) to continue polling/subscribing.

## 3. Tauri Window APIs and Events Investigation
Under `@tauri-apps/api/window` (Tauri v2), we have APIs to query and listen to window state:

### A. Window Event Listeners:
1. `getCurrentWindow().onResized(handler)`:
   - Emits whenever the native window is resized.
   - **Relevance**: On Windows and other OSs, minimizing a window changes its size (usually to 0x0 or a specialized minimized state), which fires a resize event. Listening to resize changes allows us to recheck if the window was minimized.
2. `getCurrentWindow().onFocusChanged(handler)`:
   - Emits a boolean payload (`true` if focused, `false` if blurred).
   - Under the hood, this API listens to both the `tauri://focus` and `tauri://blur` Tauri events and normalizes the payload.
   - **Relevance**: Minimizing or hiding the window always changes its focus state.

### B. State Query APIs:
1. `getCurrentWindow().isMinimized()`:
   - Returns a `Promise<boolean>`.
   - **Relevance**: Directly queries the OS/window manager to determine if the window is minimized.
2. `getCurrentWindow().isFocused()`:
   - Returns a `Promise<boolean>`.
   - **Relevance**: Directly queries if the window currently holds focus.

## 4. Robust Visibility Strategy
Per the interface contracts in `SCOPE.md` (Milestone 1) and the request:
- `useVisibility()` must return `true` if and only if `document.visibilityState === 'visible'` AND the Tauri window is not minimized.
- Optionally, we can check for focus, but doing so strictly (e.g. marking visible as `false` when blurred) would disconnect WebSockets when the user interacts with another window (e.g. side-by-side monitoring). Therefore, the recommended robust strategy focuses on **document visibility state and Tauri window minimized state**, while using focus events merely as triggers to re-evaluate the state.

### Proposed Code for `src/hooks/use-visibility.ts`
Below is the proposed implementation of the optimized `useVisibility` hook:

```typescript
import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

export const useVisibility = () => {
  const [visible, setVisible] = useState(() =>
    typeof document === 'undefined'
      ? true
      : document.visibilityState === 'visible',
  )

  useEffect(() => {
    let isUnmounted = false
    let unlistenResize: (() => void) | null = null
    let unlistenFocus: (() => void) | null = null

    // Asynchronously queries the document and Tauri window states to determine visibility
    const checkVisibility = async () => {
      if (isUnmounted) return

      const isDocVisible = typeof document !== 'undefined'
        ? document.visibilityState === 'visible'
        : true

      let isMin = false
      try {
        const currentWindow = getCurrentWindow()
        isMin = await currentWindow.isMinimized()
      } catch (err) {
        // Fallback to false if the Tauri API is unavailable (e.g. during tests or browser preview)
        console.warn('[useVisibility] Failed to query minimized state:', err)
      }

      const nextVisible = isDocVisible && !isMin

      if (!isUnmounted) {
        setVisible(nextVisible)
      }
    }

    // Run immediately on mount to sync the state asynchronously
    checkVisibility()

    // DOM event listeners
    const handleVisibilityChange = () => {
      checkVisibility()
    }
    const handleFocus = () => {
      checkVisibility()
    }
    const handlePointerDown = () => {
      checkVisibility()
    }

    document.addEventListener('visibilitychange', handleVisibilityChange)
    document.addEventListener('focus', handleFocus)
    document.addEventListener('pointerdown', handlePointerDown)

    // Tauri window event listeners
    try {
      const currentWindow = getCurrentWindow()

      // Listen to window resize events (triggered during minimization/restoration)
      currentWindow
        .onResized(() => {
          checkVisibility()
        })
        .then((unlisten) => {
          if (isUnmounted) {
            unlisten()
          } else {
            unlistenResize = unlisten
          }
        })
        .catch((err) => {
          console.warn('[useVisibility] Failed to bind onResized:', err)
        })

      // Listen to focus changes (triggered when window is focused/blurred/minimized)
      currentWindow
        .onFocusChanged(() => {
          checkVisibility()
        })
        .then((unlisten) => {
          if (isUnmounted) {
            unlisten()
          } else {
            unlistenFocus = unlisten
          }
        })
        .catch((err) => {
          console.warn('[useVisibility] Failed to bind onFocusChanged:', err)
        })
    } catch (err) {
      console.warn('[useVisibility] Tauri API not available, using DOM events only:', err)
    }

    // Cleanup listeners
    return () => {
      isUnmounted = true
      document.removeEventListener('visibilitychange', handleVisibilityChange)
      document.removeEventListener('focus', handleFocus)
      document.removeEventListener('pointerdown', handlePointerDown)
      if (unlistenResize) unlistenResize()
      if (unlistenFocus) unlistenFocus()
    }
  }, [])

  return visible
}
```

### Breakdown of State Transitions:
1. **Minimize Event**: 
   - Fires `onFocusChanged(false)` and `onResized`. 
   - `checkVisibility` runs asynchronously, checks `currentWindow.isMinimized()`, gets `true`. State updates to `false`.
2. **Restore Event**: 
   - Fires `onResized` and `onFocusChanged(true)`. 
   - `checkVisibility` queries `isMinimized()`, gets `false`. State updates to `true`.
3. **Tray Hide / Show**: 
   - Hiding changes document visibility state and window focus.
   - Evaluation of `document.visibilityState === 'visible'` correctly determines visibility.
4. **DOM Focus/PointerDown**:
   - Re-evaluates visibility state asynchronously to ensure consistency when clicking/interacting.

## 5. Potential Issues & Mitigation Strategy
1. **Tauri Environment Availability**: In web-only development modes or unit tests, `getCurrentWindow` will throw or return an invalid object. We mitigate this by wrapping the Tauri calls in a `try...catch` block and defaulting `isMin = false`, ensuring the hook remains functional in standard web browsers and testing environments.
2. **Race Conditions during Unmount**: Since `onResized` and `onFocusChanged` return promises, the component might unmount before they resolve. We mitigate this by setting an `isUnmounted` flag, checking it inside the `.then()` callbacks, and immediately calling the resolved unlisten function if the component has already unmounted.
3. **Redundant Render Cycles**: React's state updater (`setVisible`) automatically skips re-rendering if the new boolean state matches the previous state. This ensures that multiple event triggers (like both a resize and a focus event firing during minimize) do not cause unnecessary component updates.
