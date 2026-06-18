import { getCurrentWindow } from '@tauri-apps/api/window'
import { createContext } from 'react'

export interface WindowContextType {
  decorated: boolean | null
  maximized: boolean | null
  /** True when the title bar is currently hidden by the idle auto-hide timer (FEAT-003) */
  isDecorationsHidden: boolean
  toggleDecorations: () => Promise<void>
  refreshDecorated: () => Promise<boolean>
  minimize: () => Promise<void>
  close: () => Promise<void>
  toggleMaximize: () => Promise<void>
  toggleFullscreen: () => Promise<void>
  currentWindow: ReturnType<typeof getCurrentWindow> | null
}

export const WindowContext = createContext<WindowContextType | undefined>(
  undefined,
)
