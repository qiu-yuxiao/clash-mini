import { createContext } from 'react'

// 跨组件共享拖拽区域启用状态，替代 querySelectorAll 全量扫描
// L-20: 提供默认值以避免 Provider 未挂载时的未定义行为
// 默认启用拖拽，setEnabled 为空函数（不执行任何操作）
// 开发环境下会在控制台输出警告，提示开发者检查 Provider 是否正确配置
const isDev =
  typeof import.meta !== 'undefined' &&
  import.meta.env &&
  (import.meta.env as any).DEV

export const DragRegionContext = createContext<{
  enabled: boolean
  setEnabled: (v: boolean) => void
}>({
  enabled: true,
  setEnabled: () => {
    if (isDev) {
      console.warn(
        '[DragRegionContext] 使用了默认的 setEnabled，可能 DragRegionProvider 未正确挂载',
      )
    }
  },
})
