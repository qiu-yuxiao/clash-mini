import { describe, expect, it } from 'vitest'

import { isDummyNode } from './node'

describe('isDummyNode 广告与说明假节点判定', () => {
  it('空字符串或无效名称判定为 dummy', () => {
    expect(isDummyNode('')).toBe(true)
    expect(isDummyNode(undefined as unknown as string)).toBe(true)
    expect(isDummyNode(null as unknown as string)).toBe(true)
  })

  it('中文关键词匹配：流量、过期时间、官网、剩余等公告节点', () => {
    expect(isDummyNode('剩余流量：100G')).toBe(true)
    expect(isDummyNode('套餐到期时间：2026-12-31')).toBe(true)
    expect(isDummyNode('最新官网发布页')).toBe(true)
    expect(isDummyNode('点击续费享受优惠')).toBe(true)
    expect(isDummyNode('官方交流电报群')).toBe(true)
    expect(isDummyNode('套餐购买链接')).toBe(true)
    expect(isDummyNode('重要公告请阅读')).toBe(true)
  })

  it('英文关键词匹配：expire, traffic, website, subscribe（大小写不敏感）', () => {
    expect(isDummyNode('Account Expire in 10 days')).toBe(true)
    expect(isDummyNode('Remaining Traffic 50GB')).toBe(true)
    expect(isDummyNode('Official Website')).toBe(true)
    expect(isDummyNode('Subscribe Update')).toBe(true)
  })

  it('特殊策略节点匹配：DIRECT', () => {
    expect(isDummyNode('DIRECT')).toBe(true)
  })

  it('包含 URL 协议头的广告文本节点判定为 dummy', () => {
    expect(isDummyNode('防失联 http://example.com')).toBe(true)
    expect(isDummyNode('https://t.me/mygroup')).toBe(true)
  })

  it('防误伤：以 http- 或 https- 开头的真实节点名称不能被误判为 dummy', () => {
    // 关键回归用例：旧版本若仅按 "http" 匹配会误伤 http 协议的真实节点
    expect(isDummyNode('http-hk-01')).toBe(false)
    expect(isDummyNode('https-sg-02')).toBe(false)
    expect(isDummyNode('http-relay-us-03')).toBe(false)
  })

  it('常规真实代理节点判定为正常节点 (false)', () => {
    expect(isDummyNode('香港 01 [优质BGP]')).toBe(false)
    expect(isDummyNode('日本 02 [原生IP]')).toBe(false)
    expect(isDummyNode('新加坡 01 [Trojan]')).toBe(false)
    expect(isDummyNode('美国 05 [VLESS]')).toBe(false)
  })
})
