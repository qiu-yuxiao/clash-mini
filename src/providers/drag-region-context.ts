import { createContext } from 'react'

// 跨组件共享拖拽区域启用状态，替代 querySelectorAll 全量扫描
export const DragRegionContext = createContext<{
  enabled: boolean
  setEnabled: (v: boolean) => void
}>({ enabled: true, setEnabled: () => {} })
