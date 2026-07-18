/**
 * 拖拽调整窗口大小期间的全局标志。
 *
 * resize-handles 在 pointerdown 时置为 true、pointerup 时置为 false。
 * window-provider (onResized) 与 use-visibility (updateWindowState) 在发起 IPC 前
 * 读取此标志，拖拽期间跳过 IPC，避免与拖拽自身的 setSize/setPosition 抢占连接、
 * 以及 resize 每像素触发一次的 IPC 洪水。
 *
 * 放在 window 上是因为这两个订阅者分属不同 React 子树，无法共享 ref/context。
 */
interface ResizingFlag {
  __isResizing?: boolean
}

export function isWindowResizing(): boolean {
  return (
    typeof window !== 'undefined' &&
    (window as unknown as ResizingFlag).__isResizing === true
  )
}

export function setWindowResizing(value: boolean): void {
  if (typeof window !== 'undefined') {
    ;(window as unknown as ResizingFlag).__isResizing = value
  }
}
