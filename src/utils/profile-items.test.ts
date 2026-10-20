import { describe, expect, it } from 'vitest'

import { hasRealProfileItem, isRealProfileItem } from './profile-items'

// 回归用例：首次引导曾用 `items.length === 0` 判断「还没添加订阅」，
// 但后端启动时会无条件追加 Merge / Script 两个内置项（items 恒 ≥ 2），
// 导致弹窗永不出现。此文件锁住「只认真实订阅」的语义。
describe('profile-items 真实订阅判定', () => {
  it('remote / local 才算真实订阅', () => {
    expect(isRealProfileItem({ type: 'remote' })).toBe(true)
    expect(isRealProfileItem({ type: 'local' })).toBe(true)
  })

  it('内置 merge / script 不算真实订阅', () => {
    expect(isRealProfileItem({ type: 'merge' })).toBe(false)
    expect(isRealProfileItem({ type: 'script' })).toBe(false)
  })

  it('空项 / 无 type / null 一律不算', () => {
    expect(isRealProfileItem({})).toBe(false)
    expect(isRealProfileItem(null)).toBe(false)
    expect(isRealProfileItem(undefined)).toBe(false)
  })

  it('典型全新安装（只有 Merge + Script）应判为「没有订阅」', () => {
    const freshInstall = [{ type: 'merge' }, { type: 'script' }]
    expect(freshInstall.length).toBeGreaterThan(0) // 说明不能用 length === 0
    expect(hasRealProfileItem(freshInstall)).toBe(false)
  })

  it('内置项与真实订阅混排时应判为「已有订阅」', () => {
    expect(
      hasRealProfileItem([
        { type: 'merge' },
        { type: 'script' },
        { type: 'remote' },
      ]),
    ).toBe(true)
  })

  it('空列表与 undefined 视为「没有订阅」', () => {
    expect(hasRealProfileItem([])).toBe(false)
    expect(hasRealProfileItem(undefined)).toBe(false)
    expect(hasRealProfileItem(null)).toBe(false)
  })
})
