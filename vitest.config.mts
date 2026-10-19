import path from 'node:path'

import { defineConfig } from 'vitest/config'

// 纯逻辑单测专用配置。
//
// 刻意**不加载** vite.config.mts 里的 Tauri/Vite 插件：这里只覆盖无副作用、不碰
// WebView/Tauri runtime 的工具函数，保证在无 GUI 环境与 CI 里都能直接跑。
// 需要 DOM 的组件测试再单独用 environment=jsdom（当前未启用）。
export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve('./src'),
      '@root': path.resolve('.'),
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
    reporters: 'dot',
  },
})
