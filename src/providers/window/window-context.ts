import type { Window as TauriWindow } from '@tauri-apps/api/window'
import { createContext } from 'react'

export interface WindowContextType {
  decorated: boolean
  /** 大尺寸模式：点击「最大化」按钮后窗口为 640×860（程序最大尺寸），替代原生 maximize */
  isLargeMode: boolean
  /** True when the custom titlebar is hidden by the idle auto-hide timer (FEAT-003 stealth mode) */
  isDecorationsHidden: boolean
  /** 窗口宽度 ≤ 285px（窄窗口）：控制路由表格、流量面板布局 */
  isMinimalWidth: boolean
  /** 窗口宽 ≤ 285 且 高 ≤ 135（最小窗口）：触发数据精简 + DOM卸载 */
  isMiniStatus: boolean
  minimize: () => Promise<void>
  close: () => Promise<void>
  toggleMaximize: () => Promise<void>
  currentWindow: TauriWindow | null
}

export const WindowContext = createContext<WindowContextType | undefined>(
  undefined,
)
