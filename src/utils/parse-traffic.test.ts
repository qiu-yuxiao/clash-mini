import { describe, expect, it } from 'vitest'

import parseTraffic from './parse-traffic'

// 锁住流量显示的进位与精度行为。这些数值直接出现在界面上，
// 单位进位错了比显示错更难被发现（用户会以为自己流量用超了）。
describe('parseTraffic 流量单位换算', () => {
  it('非数字入参返回 NaN 占位，不抛异常', () => {
    expect(parseTraffic(undefined)).toEqual(['NaN', ''])
    expect(parseTraffic('abc' as unknown as number)).toEqual(['NaN', ''])
  })

  it('0 字节', () => {
    expect(parseTraffic(0)).toEqual(['0.00', 'B'])
  })

  it('字节级不升位', () => {
    expect(parseTraffic(500)).toEqual(['500', 'B'])
  })

  it('满 1024 进位到 KB', () => {
    expect(parseTraffic(1024)).toEqual(['1.00', 'KB'])
    expect(parseTraffic(2048)).toEqual(['2.00', 'KB'])
  })

  it('MB / GB 进位正确', () => {
    expect(parseTraffic(1024 ** 2)).toEqual(['1.00', 'MB'])
    expect(parseTraffic(1024 ** 3)).toEqual(['1.00', 'GB'])
  })

  it('大数值降精度避免溢出显示（≥1000 取整）', () => {
    // 1000 字节 < 1024 不升位，但数值 ≥1000 → 走 toFixed(0)
    expect(parseTraffic(1000)).toEqual(['1000', 'B'])
  })
})
