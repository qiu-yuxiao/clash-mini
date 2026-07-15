import '@fontsource/outfit/300.css'
import '@fontsource/outfit/400.css'
import '@fontsource/outfit/600.css'
import '@fontsource/outfit/700.css'
import './assets/styles/index.scss'

import createCache from '@emotion/cache'
import { CacheProvider } from '@emotion/react'
import { ResizeObserver } from '@juggle/resize-observer'
import { QueryClientProvider } from '@tanstack/react-query'
import { ComposeContextProvider } from 'foxact/compose-context-provider'
import React from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router'

import { MihomoWebSocket } from 'tauri-plugin-mihomo-api'

import { BaseErrorBoundary } from './components/base'
import { router } from './pages/_routers'
import { AppDataProvider } from './providers/app-data-provider'
import { WindowProvider } from './providers/window'
import { FALLBACK_LANGUAGE, initializeLanguage } from './services/i18n'
import {
  preloadAppData,
  resolveThemeMode,
  getPreloadConfig,
} from './services/preload'
import { queryClient } from './services/query-client'
import { ThemeModeProvider } from './services/states'
import { disableWebViewShortcuts } from './utils/disable-webview-shortcuts'

if (!window.ResizeObserver) {
  window.ResizeObserver = ResizeObserver
}

// 读取 Tauri 2.0 构建时注入的 CSP nonce
// Tauri 会在 CSP 的 style-src/script-src 中自动添加 'nonce-xxx'，
// 同时给 HTML 中的 inline <script>/<style> 标签添加 nonce 属性。
// 但 emotion 运行时动态创建的 <style> 标签没有 nonce，会被 CSP 阻断。
// 这里读取 Tauri 注入的 nonce，传给 emotion cache，让动态样式带 nonce 通过 CSP。
function getCspNonce(): string | undefined {
  const meta = document.querySelector(
    'meta[name="csp-nonce"]',
  ) as HTMLMetaElement | null
  if (meta?.content) return meta.content

  const script = document.querySelector(
    'script[nonce]',
  ) as HTMLScriptElement | null
  if (script) {
    const nonce = script.nonce || script.getAttribute('nonce')
    if (nonce) return nonce
  }

  const style = document.querySelector(
    'style[nonce]',
  ) as HTMLStyleElement | null
  if (style) {
    const nonce = style.nonce || style.getAttribute('nonce')
    if (nonce) return nonce
  }

  return undefined
}

const emotionCache = createCache({
  key: 'mui',
  nonce: getCspNonce(),
})

const mainElementId = 'root'
const container = document.getElementById(mainElementId)

if (!container) {
  throw new Error(`No container '${mainElementId}' found to render application`)
}

disableWebViewShortcuts()

const initializeApp = (initialThemeMode: 'light' | 'dark') => {
  // 清除"本会话已 enhance 过的 profile uid"标志
  // 仅在程序启动时执行（main.tsx 不会被窗口销毁/重建触发），确保：
  // - 程序首次启动：标志被清除，_layout 的 useEffect 会触发 enhance
  // - 轻量模式唤醒：main.tsx 不执行，localStorage 中的 uid 得以保留，
  //   _layout 的 useEffect 检测到 uid 未变，跳过 enhance 避免重置内核
  localStorage.removeItem('clash-mini-last-enhanced-uid')

  const contexts = [
    <ThemeModeProvider key="theme" initialState={initialThemeMode} />,
  ]

  const root = createRoot(container)
  root.render(
    <React.StrictMode>
      <CacheProvider value={emotionCache}>
        <ComposeContextProvider contexts={contexts}>
          <BaseErrorBoundary>
            <QueryClientProvider client={queryClient}>
              <WindowProvider>
                <AppDataProvider>
                  <RouterProvider router={router} />
                </AppDataProvider>
              </WindowProvider>
            </QueryClientProvider>
          </BaseErrorBoundary>
        </ComposeContextProvider>
      </CacheProvider>
    </React.StrictMode>,
  )
}

const bootstrap = async () => {
  const { initialThemeMode } = await preloadAppData()
  initializeApp(initialThemeMode)
}

bootstrap().catch((error) => {
  console.error(
    '[main.tsx] App bootstrap failed, falling back to default language:',
    error,
  )
  initializeLanguage(FALLBACK_LANGUAGE)
    .catch((fallbackError) => {
      console.error(
        '[main.tsx] Fallback language initialization failed:',
        fallbackError,
      )
    })
    .finally(() => {
      initializeApp(resolveThemeMode(getPreloadConfig()))
    })
})

// Error handling
window.addEventListener('error', (event) => {
  console.error('[main.tsx] Global error:', event.error)
})

window.addEventListener('unhandledrejection', (event) => {
  console.error('[main.tsx] Unhandled promise rejection:', event.reason)
})

// Page close/refresh events
window.addEventListener('beforeunload', () => {
  // Clean up all WebSocket instances to prevent memory leaks
  MihomoWebSocket.cleanupAll()
})

// Page loaded event
window.addEventListener('DOMContentLoaded', () => {
  // Clean up all WebSocket instances to prevent memory leaks
  MihomoWebSocket.cleanupAll()
})
