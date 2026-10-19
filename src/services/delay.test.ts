import { describe, expect, it } from 'vitest'

import { formatDelay, formatDelayColor, NODE_DELAY_MAX_MS } from './delay'

describe('formatDelay 延迟文本格式化', () => {
  it('未测速 (-1) 返回占位横线', () => {
    expect(formatDelay(-1)).toBe('-')
  })

  it('正在测速 (-2) 返回 testing', () => {
    expect(formatDelay(-2)).toBe('testing')
  })

  it('超时 (0 或 >= timeout 且 <= 1e5) 返回 Timeout', () => {
    expect(formatDelay(0)).toBe('Timeout')
    expect(formatDelay(NODE_DELAY_MAX_MS)).toBe('Timeout')
    expect(formatDelay(2500)).toBe('Timeout')
    expect(formatDelay(1e5)).toBe('Timeout')
  })

  it('异常 (> 1e5) 返回 Error', () => {
    expect(formatDelay(100001)).toBe('Error')
    expect(formatDelay(200000)).toBe('Error')
  })

  it('正常有效延迟返回数字字符串', () => {
    expect(formatDelay(35)).toBe('35')
    expect(formatDelay(180)).toBe('180')
    expect(formatDelay(1999)).toBe('1999')
  })

  it('支持自定义 timeout 阈值', () => {
    expect(formatDelay(1500, 1000)).toBe('Timeout')
    expect(formatDelay(800, 1000)).toBe('800')
  })
})

describe('formatDelayColor 延迟颜色映射', () => {
  it('未测速与测试中 (< 0) 返回空样式', () => {
    expect(formatDelayColor(-1)).toBe('')
    expect(formatDelayColor(-2)).toBe('')
  })

  it('超时与失败 (0 或 >= timeout) 返回 error.main', () => {
    expect(formatDelayColor(0)).toBe('error.main')
    expect(formatDelayColor(2000)).toBe('error.main')
    expect(formatDelayColor(3500)).toBe('error.main')
  })

  it('较慢延迟 (>= 400 且 < timeout) 在浅色与深色主题下返回对应 warning 色', () => {
    expect(formatDelayColor(400, 2000, false)).toBe('warning.dark')
    expect(formatDelayColor(400, 2000, true)).toBe('warning.main')
    expect(formatDelayColor(1200, 2000, false)).toBe('warning.dark')
    expect(formatDelayColor(1200, 2000, true)).toBe('warning.main')
  })

  it('中等延迟 (>= 250 且 < 400) 返回 primary.main', () => {
    expect(formatDelayColor(250)).toBe('primary.main')
    expect(formatDelayColor(399)).toBe('primary.main')
  })

  it('优秀极速延迟 (< 250 且 > 0) 返回 success.main', () => {
    expect(formatDelayColor(30)).toBe('success.main')
    expect(formatDelayColor(120)).toBe('success.main')
    expect(formatDelayColor(249)).toBe('success.main')
  })
})
