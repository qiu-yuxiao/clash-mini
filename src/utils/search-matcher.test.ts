import { describe, expect, it } from 'vitest'

import {
  compileStringMatcher,
  extractPositiveSearchTerms,
} from './search-matcher'

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

describe('BUG-424: compileStringMatcher 与/或/排除多关键词过滤及正向高亮提取', () => {
  it('支持空格 AND、| / ｜ OR（含两侧空格）与 - / ! / ！ 排除', () => {
    const andMatcher = compileStringMatcher('google chrome').matcher
    expect(andMatcher('play.googleapis.com chrome.exe')).toBe(true)
    expect(andMatcher('play.googleapis.com curl.exe')).toBe(false)

    const orMatcher = compileStringMatcher('google | oracle').matcher
    expect(orMatcher('cloud.oracle.com')).toBe(true)
    expect(orMatcher('play.googleapis.com')).toBe(true)
    expect(orMatcher('github.com')).toBe(false)

    const fullwidthOrMatcher = compileStringMatcher('oracle｜lastpass').matcher
    expect(fullwidthOrMatcher('pollserver.lastpass.com')).toBe(true)
    expect(fullwidthOrMatcher('github.com')).toBe(false)

    const pureExclude = compileStringMatcher('-play').matcher
    expect(pureExclude('github.com')).toBe(true)
    expect(pureExclude('play.googleapis.com')).toBe(false)

    const multiExclude = compileStringMatcher(
      '-香港|-倍率 !台湾 ！广告',
    ).matcher
    expect(multiExclude('日本 专线 01')).toBe(true)
    expect(multiExclude('香港 01')).toBe(false)
    expect(multiExclude('日本 2x倍率')).toBe(false)
    expect(multiExclude('台湾 01')).toBe(false)
    expect(multiExclude('广告节点')).toBe(false)
  })

  it('孤立操作符放行全部内容，内部连字符词条按完整词匹配', () => {
    for (const op of ['-', '!', '！', '|', '｜', '- | !']) {
      expect(compileStringMatcher(op).matcher('任意文本')).toBe(true)
      expect(compileStringMatcher(op).matcher('')).toBe(true)
    }

    expect(
      compileStringMatcher('daily-cloudcode').matcher(
        'daily-cloudcode-pa.googleapis.com',
      ),
    ).toBe(true)
    expect(compileStringMatcher('英国-3倍率').matcher('英国-3倍率 (01)')).toBe(
      true,
    )
    expect(
      compileStringMatcher('google | oracle -play').matcher(
        'daily-cloudcode-pa.googleapis.com',
      ),
    ).toBe(true)
    expect(
      compileStringMatcher('google | oracle -play').matcher(
        'play.googleapis.com',
      ),
    ).toBe(false)
  })

  it('extractPositiveSearchTerms 仅提取去重后的正向关键词并按长度降序排列', () => {
    expect(extractPositiveSearchTerms('')).toEqual([])
    expect(extractPositiveSearchTerms('-dns !timeout')).toEqual([])
    expect(
      extractPositiveSearchTerms('dial | timeout dial_long -direct ！cn'),
    ).toEqual(['dial_long', 'timeout', 'dial'])
  })
})
