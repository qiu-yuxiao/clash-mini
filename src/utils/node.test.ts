import { describe, expect, it } from 'vitest'

import { isDummyNode } from './node'

// 本文件的断言必须与后端 `src-tauri/src/utils/node.rs` 的测试逐条对应。
// 前后端判定不一致会直接造成「后端能选中、前端却隐藏」的节点，属于高危一致性问题。
describe('isDummyNode 广告与说明假节点判定', () => {
  it('空字符串或无效名称判定为 dummy', () => {
    expect(isDummyNode('')).toBe(true)
    expect(isDummyNode(undefined as unknown as string)).toBe(true)
    expect(isDummyNode(null as unknown as string)).toBe(true)
  })

  it('中文广告短语在开头时判定为 dummy', () => {
    expect(isDummyNode('剩余流量：100G')).toBe(true)
    expect(isDummyNode('过期时间：2026-12-31')).toBe(true)
    expect(isDummyNode('套餐到期')).toBe(true)
    expect(isDummyNode('续费链接')).toBe(true)
    expect(isDummyNode('公告栏')).toBe(true)
    expect(isDummyNode('群：123456')).toBe(true)
  })

  it('整串被包裹符号包住或带「节点-」前缀时，归一化后能识别', () => {
    // 只有包裹符号**包住整串**才会被剥掉（与后端 normalize_dummy_name 一致）
    expect(isDummyNode('（购买入口）')).toBe(true)
    expect(isDummyNode('节点-购买入口')).toBe(true)
    expect(isDummyNode('【【过期时间】】')).toBe(true)
    // 已知共同漏判（前端与后端一致）：括号后面还有内容时不剥壳，广告短语不在开头
    // 就识别不到。修它需要按「块内内容」判定，属独立改进项，此处只锁住现状。
    expect(isDummyNode('【剩余流量】80GB')).toBe(false)
  })

  it('英文广告词与 URL 允许子串匹配（大小写不敏感）', () => {
    expect(isDummyNode('Account Expire in 10 days')).toBe(true)
    expect(isDummyNode('Remaining Traffic 50GB')).toBe(true)
    expect(isDummyNode('Official Website')).toBe(true)
    expect(isDummyNode('Subscribe Update')).toBe(true)
    expect(isDummyNode('防失联 http://example.com')).toBe(true)
    expect(isDummyNode('https://t.me/mygroup')).toBe(true)
  })

  it('特殊策略节点 DIRECT 不作为可选节点展示', () => {
    expect(isDummyNode('DIRECT')).toBe(true)
  })

  // ── 以下两组是「不能误杀真实节点」的回归用例（与后端测试一一对应） ──
  it('防误伤：广告短语出现在名字中间的真实节点不能被误判', () => {
    // 旧前端实现用 includes 子串匹配，会把下面这些真节点全部藏掉
    expect(isDummyNode('日本群组-A')).toBe(false)
    expect(isDummyNode('美国-网址专线')).toBe(false)
    expect(isDummyNode('上海-群节点-01')).toBe(false)
    expect(isDummyNode('HK-流量优化')).toBe(false)
    expect(isDummyNode('http-hk-01')).toBe(false)
    expect(isDummyNode('https-sg-02')).toBe(false)
  })

  // ── 以下为「有意漏判」，与后端保持一致，不要擅自改成 includes ──
  it('已知漏判：广告短语不在开头时按后端语义不判定为 dummy', () => {
    // 这些名字后端同样返回 false。改成子串匹配会连带误杀上一条里的真实节点，
    // 而漏判伪节点的代价很小：伪节点测不出延迟，不会被选点逻辑选中，后端还会兜底。
    expect(isDummyNode('最新官网发布页')).toBe(false)
    expect(isDummyNode('点击续费享受优惠')).toBe(false)
    expect(isDummyNode('官方交流电报群')).toBe(false)
    expect(isDummyNode('重要公告请阅读')).toBe(false)
  })

  it('常规真实代理节点判定为正常节点 (false)', () => {
    expect(isDummyNode('香港 01 [优质BGP]')).toBe(false)
    expect(isDummyNode('日本 02 [原生IP]')).toBe(false)
    expect(isDummyNode('新加坡 01 [Trojan]')).toBe(false)
    expect(isDummyNode('美国 05 [VLESS]')).toBe(false)
  })
})
