/**
 * 统一键盘快捷键收口治理模块（BUG-355）：
 * 在单一 `document` 级 `keydown` 监听器中统一处理：
 * 1. 屏蔽 WebView2 浏览器原生默认热键（原生 Ctrl+F 查找条、F5 刷新、Alt+左右导航等，绝不拦截 Alt+F4 / Ctrl+F4）；
 * 2. 在无任何弹窗/菜单遮挡、非输入编辑态、非 IME 合成且非长按连发时，安全派发业务快捷键（Ctrl+F / F4 / F1）。
 */
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

let activeOverlayGuard: (() => boolean) | null = null
let documentListenerInstalled = false

const handleUnifiedKeydown = (event: KeyboardEvent) => {
  const isDisabledBrowserShortcut =
    ['F5', 'F7'].includes(event.key) ||
    (event.altKey && ['ArrowLeft', 'ArrowRight'].includes(event.key)) ||
    ((event.ctrlKey || event.metaKey) &&
      ['F', 'G', 'H', 'J', 'P', 'Q', 'R', 'U'].includes(
        event.key.toUpperCase(),
      ))

  if (isDisabledBrowserShortcut) {
    event.preventDefault()
  }

  if (
    activeOverlayGuard === null ||
    event.isComposing ||
    event.repeat ||
    activeOverlayGuard() ||
    isEditableTarget(event.target)
  ) {
    return
  }

  if (
    (event.ctrlKey || event.metaKey) &&
    !event.altKey &&
    !event.shiftKey &&
    event.key.toLowerCase() === 'f'
  ) {
    event.preventDefault()
    window.dispatchEvent(
      new CustomEvent<AppShortcutAction>('clash-mini-shortcut', {
        detail: 'focus-filter',
      }),
    )
    return
  }

  if (!event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey) {
    if (event.key === 'F4') {
      event.preventDefault()
      window.dispatchEvent(
        new CustomEvent<AppShortcutAction>('clash-mini-shortcut', {
          detail: 'check-visible',
        }),
      )
    } else if (event.key === 'F1') {
      event.preventDefault()
      window.dispatchEvent(
        new CustomEvent<AppShortcutAction>('clash-mini-shortcut', {
          detail: 'check-full',
        }),
      )
    }
  }
}

const ensureDocumentShortcutListener = () => {
  if (documentListenerInstalled || typeof document === 'undefined') return
  document.addEventListener('keydown', handleUnifiedKeydown)
  documentListenerInstalled = true
}

export const disableWebViewShortcuts = () => {
  ensureDocumentShortcutListener()
  return () => {
    if (documentListenerInstalled && activeOverlayGuard === null) {
      document.removeEventListener('keydown', handleUnifiedKeydown)
      documentListenerInstalled = false
    }
  }
}

export const bindMainAppShortcuts = (
  isOverlayOpen: () => boolean,
): (() => void) => {
  ensureDocumentShortcutListener()
  activeOverlayGuard = isOverlayOpen
  return () => {
    if (activeOverlayGuard === isOverlayOpen) {
      activeOverlayGuard = null
    }
  }
}
