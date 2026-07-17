import { use } from 'react'

import { WindowContext, type WindowContextType } from '@/providers/window'

export const useWindow = () => {
  const context = use(WindowContext)
  if (context === undefined) {
    throw new Error('useWindow must be used within WindowProvider')
  }
  return context
}

export const useWindowControls = () => {
  const { maximized, minimize, toggleMaximize, close, currentWindow } =
    useWindow()
  return {
    maximized,
    minimize,
    toggleMaximize,
    close,
    currentWindow,
  } satisfies Pick<
    WindowContextType,
    'maximized' | 'minimize' | 'toggleMaximize' | 'close' | 'currentWindow'
  >
}

export const useWindowDecorations = () => {
  const { decorated, isDecorationsHidden, isMinimalWidth, isMiniStatus } =
    useWindow()
  return {
    decorated,
    isDecorationsHidden,
    isMinimalWidth,
    isMiniStatus,
  } satisfies Pick<
    WindowContextType,
    'decorated' | 'isDecorationsHidden' | 'isMinimalWidth' | 'isMiniStatus'
  >
}
