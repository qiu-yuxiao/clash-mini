import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  isSupportedSubscriptionOrNodeInput,
  SUPPORTED_NODE_URI_SCHEMES,
} from '@/utils/node'

import { bindMainAppShortcuts } from './disable-webview-shortcuts'
import {
  isWindowResizing,
  RESIZE_ENDED_EVENT,
  setWindowResizing,
} from './window-resizing'

class MockHTMLElement {
  isContentEditable = false
}

class MockHTMLInputElement extends MockHTMLElement {}
class MockHTMLTextAreaElement extends MockHTMLElement {}
class MockHTMLSelectElement extends MockHTMLElement {}

describe('BUG-386: window-resizing 拖拽门控与 RESIZE_ENDED_EVENT 状态传递', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('setWindowResizing 在 true -> false 转换时派发携带尺寸与 DPI 缩放因子的结束事件', () => {
    const dispatched: Event[] = []
    const fakeWindow = {
      dispatchEvent: (ev: Event) => {
        dispatched.push(ev)
        return true
      },
    }
    vi.stubGlobal('window', fakeWindow)

    expect(isWindowResizing()).toBe(false)
    setWindowResizing(true)
    expect(isWindowResizing()).toBe(true)
    expect(dispatched).toHaveLength(0)

    const detail = { width: 612, height: 540, scaleFactor: 1.5 }
    setWindowResizing(false, detail)
    expect(isWindowResizing()).toBe(false)
    expect(dispatched).toHaveLength(1)
    const endedEvent = dispatched[0] as CustomEvent<typeof detail>
    expect(endedEvent.type).toBe(RESIZE_ENDED_EVENT)
    expect(endedEvent.detail).toEqual(detail)
  })

  it('未处于拖拽状态时重复 setWindowResizing(false) 不会误派发结束事件', () => {
    const dispatched: Event[] = []
    vi.stubGlobal('window', {
      dispatchEvent: (ev: Event) => {
        dispatched.push(ev)
        return true
      },
    })

    setWindowResizing(false)
    expect(isWindowResizing()).toBe(false)
    expect(dispatched).toHaveLength(0)
  })

  it('允许不带 detail 调用 setWindowResizing(false) 触发兜底回读对齐', () => {
    const dispatched: Event[] = []
    vi.stubGlobal('window', {
      dispatchEvent: (ev: Event) => {
        dispatched.push(ev)
        return true
      },
    })

    setWindowResizing(true)
    setWindowResizing(false)
    expect(dispatched).toHaveLength(1)
    expect((dispatched[0] as CustomEvent).detail).toBeNull()
  })
})

describe('BUG-389: 全局快捷键与首装全协议输入校验', () => {
  let keydownHandler: ((event: KeyboardEvent) => void) | null = null
  let dispatchedActions: string[] = []
  let overlayOpen = false
  let unbind: (() => void) | null = null

  beforeEach(() => {
    overlayOpen = false
    dispatchedActions = []
    vi.stubGlobal('HTMLElement', MockHTMLElement)
    vi.stubGlobal('HTMLInputElement', MockHTMLInputElement)
    vi.stubGlobal('HTMLTextAreaElement', MockHTMLTextAreaElement)
    vi.stubGlobal('HTMLSelectElement', MockHTMLSelectElement)
    vi.stubGlobal('document', {
      addEventListener: (
        _type: string,
        handler: (e: KeyboardEvent) => void,
      ) => {
        keydownHandler = handler
      },
      removeEventListener: vi.fn(),
    })
    vi.stubGlobal('window', {
      dispatchEvent: (ev: Event) => {
        const custom = ev as CustomEvent<string>
        if (custom.type === 'clash-mini-shortcut') {
          dispatchedActions.push(custom.detail)
        }
        return true
      },
    })
    unbind = bindMainAppShortcuts(() => overlayOpen)
  })

  afterEach(() => {
    unbind?.()
    unbind = null
    vi.unstubAllGlobals()
  })

  const fireKey = (init: {
    key: string
    target?: EventTarget | null
    ctrlKey?: boolean
    metaKey?: boolean
    altKey?: boolean
    shiftKey?: boolean
    isComposing?: boolean
    repeat?: boolean
  }) => {
    const preventDefault = vi.fn()
    const fakeEvent = {
      key: init.key,
      target: init.target ?? new MockHTMLElement(),
      ctrlKey: init.ctrlKey ?? false,
      metaKey: init.metaKey ?? false,
      altKey: init.altKey ?? false,
      shiftKey: init.shiftKey ?? false,
      isComposing: init.isComposing ?? false,
      repeat: init.repeat ?? false,
      preventDefault,
    } as unknown as KeyboardEvent
    keydownHandler?.(fakeEvent)
    return preventDefault
  }

  it('在输入框获焦时仍可直接触发 F4 与 F1，而 Ctrl+F 仅在非编辑态触发聚焦', () => {
    const input = new MockHTMLInputElement() as unknown as EventTarget
    const textarea = new MockHTMLTextAreaElement() as unknown as EventTarget

    const pdF4 = fireKey({ key: 'F4', target: input })
    const pdF1 = fireKey({ key: 'F1', target: textarea })
    expect(pdF4).toHaveBeenCalledTimes(1)
    expect(pdF1).toHaveBeenCalledTimes(1)
    expect(dispatchedActions).toEqual(['check-visible', 'check-full'])

    dispatchedActions = []
    const pdCtrlFInInput = fireKey({ key: 'f', ctrlKey: true, target: input })
    expect(pdCtrlFInInput).toHaveBeenCalledTimes(1)
    expect(dispatchedActions).toEqual([])

    const pdCtrlFOnBody = fireKey({
      key: 'f',
      ctrlKey: true,
      target: new MockHTMLElement() as unknown as EventTarget,
    })
    expect(pdCtrlFOnBody).toHaveBeenCalled()
    expect(dispatchedActions).toEqual(['focus-filter'])
  })

  it('弹窗或抽屉打开时屏蔽业务快捷键，且始终不拦截 Alt+F4', () => {
    overlayOpen = true
    fireKey({ key: 'F4' })
    fireKey({ key: 'F1' })
    fireKey({ key: 'f', ctrlKey: true })
    expect(dispatchedActions).toEqual([])

    overlayOpen = false
    const pdAltF4 = fireKey({ key: 'F4', altKey: true })
    expect(pdAltF4).not.toHaveBeenCalled()
    expect(dispatchedActions).toEqual([])
  })

  it('isSupportedSubscriptionOrNodeInput 支持 http(s) 与全部 11 种节点协议及多行节点 URI', () => {
    expect(SUPPORTED_NODE_URI_SCHEMES).toHaveLength(11)
    expect(
      isSupportedSubscriptionOrNodeInput('https://sub.example.com/a?b=1'),
    ).toBe(true)
    expect(
      isSupportedSubscriptionOrNodeInput('HTTP://127.0.0.1:8080/sub'),
    ).toBe(true)

    for (const scheme of SUPPORTED_NODE_URI_SCHEMES) {
      expect(isSupportedSubscriptionOrNodeInput(`${scheme}node-payload`)).toBe(
        true,
      )
      expect(
        isSupportedSubscriptionOrNodeInput(
          `${scheme.toUpperCase()}node-payload`,
        ),
      ).toBe(true)
    }

    const multiLineNodes = [
      'vmess://eyJhZGQiOiIxLjEuMS4xIn0=',
      'vless://uuid@1.2.3.4:443',
      'hysteria2://pass@1.2.3.4:443',
    ].join('\n')
    expect(isSupportedSubscriptionOrNodeInput(multiLineNodes)).toBe(true)

    expect(isSupportedSubscriptionOrNodeInput('')).toBe(false)
    expect(isSupportedSubscriptionOrNodeInput('   \n  ')).toBe(false)
    expect(isSupportedSubscriptionOrNodeInput('ftp://example.com/sub')).toBe(
      false,
    )
    expect(
      isSupportedSubscriptionOrNodeInput('https://a.com/1\nhttps://b.com/2'),
    ).toBe(false)
    expect(
      isSupportedSubscriptionOrNodeInput('https://a.com/sub with space'),
    ).toBe(false)
    expect(
      isSupportedSubscriptionOrNodeInput('vmess://ok\nnot-a-node-uri'),
    ).toBe(false)
  })
})
