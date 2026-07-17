import { getCurrentWindow } from '@tauri-apps/api/window'
import { createContext } from 'react'

export interface WindowContextType {
  decorated: boolean
  maximized: boolean | null
  /** True when the custom titlebar is hidden by the idle auto-hide timer (FEAT-003 stealth mode) */
  isDecorationsHidden: boolean
  /** 窗口宽度 ≤ 285px（窄窗口）：控制路由表格、流量面板布局 */
  isMinimalWidth: boolean
  /** 窗口宽 ≤ 285 且 高 ≤ 135（最小窗口）：触发数据精简 + DOM卸载 */
  isMiniStatus: boolean
  minimize: () => Promise<void>
  close: () => Promise<void>
  toggleMaximize: () => Promise<void>
  currentWindow: ReturnType<typeof getCurrentWindow> | null
}

export const WindowContext = createContext<WindowContextType | undefined>(
  undefined,
)
