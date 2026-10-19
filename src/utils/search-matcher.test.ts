import { describe, expect, it } from 'vitest'

import { compileStringMatcher } from './search-matcher'

// 覆盖「节点子集过滤」语义：这是选点范围的唯一来源。
// 错在这里会直接导致选点越界（如在「日本」筛选下切到香港节点），必须锁住。
describe('compileStringMatcher 节点子集过滤', () => {
  it('空查询匹配全部节点', () => {
    const { matcher, isValid } = compileStringMatcher('')
    expect(isValid).toBe(true)
    expect(matcher('日本aw6')).toBe(true)
    expect(matcher('v5-香港00|1x|v')).toBe(true)
  })

  it('纯空白查询视同空查询（不能把全部节点过滤掉）', () => {
    const { matcher } = compileStringMatcher('   ')
    expect(matcher('任意节点')).toBe(true)
  })

  it('按地域子串过滤，且不越界', () => {
    const { matcher } = compileStringMatcher('日本')
    expect(matcher('日本aw6 (050410)')).toBe(true)
    expect(matcher('v4-日本01|1x|v (050410)')).toBe(true)
    expect(matcher('v5-香港00|1x|v (050410)')).toBe(false)
    expect(matcher('新新加坡3 (050410)')).toBe(false)
  })

  it('大小写不敏感', () => {
    const { matcher } = compileStringMatcher('JP')
    expect(matcher('jp01.hy2use.net')).toBe(true)
    expect(matcher('JP-AW6')).toBe(true)
    expect(matcher('hk01')).toBe(false)
  })

  it('查询两端空白会被裁剪', () => {
    const { matcher } = compileStringMatcher('  日本  ')
    expect(matcher('日本aw6')).toBe(true)
    expect(matcher('香港00')).toBe(false)
  })

  it('非空查询不会匹配空节点名', () => {
    const { matcher } = compileStringMatcher('日本')
    expect(matcher('')).toBe(false)
  })
})
