import { getCurrentWindow } from '@tauri-apps/api/window'
import { createContext } from 'react'

export interface WindowContextType {
  decorated: boolean
  maximized: boolean | null
  /** True when the custom titlebar is hidden by the idle auto-hide timer (FEAT-003 stealth mode) */
  isDecorationsHidden: boolean
  minimize: () => Promise<void>
  close: () => Promise<void>
  toggleMaximize: () => Promise<void>
  toggleFullscreen: () => Promise<void>
  currentWindow: ReturnType<typeof getCurrentWindow> | null
}

export const WindowContext = createContext<WindowContextType | undefined>(
  undefined,
)
