/**
 * 统一键盘快捷键收口治理模块（BUG-355）：
 * 1. `disableWebViewShortcuts`: 屏蔽 WebView2 浏览器原生默认热键（含原生 Ctrl+F 查找条、F5 刷新等）
 * 2. `bindMainAppShortcuts`: 在无弹窗遮挡时将业务快捷键安全分发给主界面，严格校验修饰键与输入焦点，绝不误伤 Alt+F4 / Ctrl+F4
 */
export const disableWebViewShortcuts = () => {
  const handleKeydown = (event: KeyboardEvent) => {
    const disabledShortcuts =
      ['F5', 'F7'].includes(event.key) ||
      (event.altKey && ['ArrowLeft', 'ArrowRight'].includes(event.key)) ||
      ((event.ctrlKey || event.metaKey) &&
        ['F', 'G', 'H', 'J', 'P', 'Q', 'R', 'U'].includes(
          event.key.toUpperCase(),
        ))

    if (disabledShortcuts) {
      event.preventDefault()
    }
  }

  document.addEventListener('keydown', handleKeydown)

  return () => document.removeEventListener('keydown', handleKeydown)
}

export type AppShortcutAction = 'focus-filter' | 'check-visible' | 'check-full'

const isEditableTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    target.isContentEditable
  )
}

export const bindMainAppShortcuts = (
  isOverlayOpen: () => boolean,
): (() => void) => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.isComposing || isOverlayOpen()) return

    if (
      (e.ctrlKey || e.metaKey) &&
      !e.altKey &&
      !e.shiftKey &&
      e.key.toLowerCase() === 'f'
    ) {
      e.preventDefault()
      window.dispatchEvent(
        new CustomEvent<AppShortcutAction>('clash-mini-shortcut', {
          detail: 'focus-filter',
        }),
      )
      return
    }

    if (
      !e.altKey &&
      !e.ctrlKey &&
      !e.metaKey &&
      !e.shiftKey &&
      !e.repeat &&
      !isEditableTarget(e.target)
    ) {
      if (e.key === 'F4') {
        e.preventDefault()
        window.dispatchEvent(
          new CustomEvent<AppShortcutAction>('clash-mini-shortcut', {
            detail: 'check-visible',
          }),
        )
      } else if (e.key === 'F1') {
        e.preventDefault()
        window.dispatchEvent(
          new CustomEvent<AppShortcutAction>('clash-mini-shortcut', {
            detail: 'check-full',
          }),
        )
      }
    }
  }

  window.addEventListener('keydown', handleKeyDown)
  return () => window.removeEventListener('keydown', handleKeyDown)
}
