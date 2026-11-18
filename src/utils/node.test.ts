import { describe, expect, it } from 'vitest'

import {
  hasEffectiveFilterTerm,
  hasRegionHint,
  isDummyNode,
  isIncompleteFilterInput,
  matchNodeFilter,
  resolveBatchCheckIntent,
} from './node'

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

  it('结构形态横幅识别（不含地区暗示时的冒号配额/日期/内嵌域名，BUG-383）', () => {
    expect(isDummyNode('距离下次重置剩余：24 天 (045017)')).toBe(true)
    expect(isDummyNode('永久官网:666.boostqz.com (045017)')).toBe(true)
    expect(isDummyNode('有效期至：2027-02-09')).toBe(true)
    expect(isDummyNode('更新日期：2026年11月')).toBe(true)
    expect(isDummyNode('Remaining: 30 days')).toBe(true)
    expect(isDummyNode('Quota: 167.26 GB')).toBe(true)
    expect(isDummyNode('剩余流量：100\u{3000}GB')).toBe(true)
    expect(isDummyNode('Contact us: boostqz.com')).toBe(true)
    expect(hasRegionHint('永久官网:abc.hk')).toBe(false)
    expect(isDummyNode('永久官网:abc.hk')).toBe(true)

    expect(hasRegionHint('🇭🇰 HK-01')).toBe(true)
    expect(hasRegionHint('香港1 (045017)')).toBe(true)
    expect(hasRegionHint('us-01 [VLESS]')).toBe(true)
    expect(hasRegionHint('CN2-GIA-01')).toBe(true)
    expect(hasRegionHint('About us: boostqz.com')).toBe(false)

    // 防误杀真实节点名
    expect(isDummyNode('vps1.myserver.com')).toBe(false)
    expect(isDummyNode('新加坡：备用')).toBe(false)
    expect(isDummyNode('专线：10Gbps')).toBe(false)
  })
})

describe('matchNodeFilter 节点多关键词与排除过滤（与后端逐条对齐）', () => {
  it('空值、纯空白及孤立符号全部放行', () => {
    for (const empty of ['', '   ', '-', '!', '！', '|', '｜', '  -  ']) {
      expect(matchNodeFilter('香港1 (005024)', empty)).toBe(true)
    }
    expect(matchNodeFilter('日本 01 [IPLC]', '日本 -')).toBe(true)
  })

  it('单关键词大小写不敏感匹配', () => {
    expect(matchNodeFilter('HK-BGP-01', 'hk')).toBe(true)
    expect(matchNodeFilter('日本 01 [IPLC]', '日本')).toBe(true)
    expect(matchNodeFilter('新加坡 01', '日本')).toBe(false)
  })

  it('多关键词 OR 匹配（半角 |、全角 ｜、两侧空格及尾部悬空 |）', () => {
    for (const q of ['日本|新加坡|台湾', '日本 | 新加坡 ｜ 台湾']) {
      expect(matchNodeFilter('日本 01', q)).toBe(true)
      expect(matchNodeFilter('新加坡-专线', q)).toBe(true)
      expect(matchNodeFilter('台湾-原生', q)).toBe(true)
      expect(matchNodeFilter('香港1 (005024)', q)).toBe(false)
    }
    expect(matchNodeFilter('日本 01', '日本|')).toBe(true)
    expect(matchNodeFilter('香港1 (005024)', '日本|')).toBe(false)
  })

  it('纯排除语法（- / ! / ！）与多排除', () => {
    expect(matchNodeFilter('香港1 (005024)', '-香港')).toBe(false)
    expect(matchNodeFilter('日本 01', '-香港')).toBe(true)
    expect(matchNodeFilter('HK-01', '!hk')).toBe(false)
    expect(matchNodeFilter('高倍率 3x', '！倍率')).toBe(false)
    expect(matchNodeFilter('香港-专线', '-香港 -英国')).toBe(false)
    expect(matchNodeFilter('英国-01', '-香港|-英国')).toBe(false)
    expect(matchNodeFilter('新加坡-01', '-香港|-英国')).toBe(true)
  })

  it('组合语法：OR + AND + 排除，且兼容含内部连字符与空格的节点名', () => {
    const combo = '日本 | 新加坡 -倍率 -香港'
    expect(matchNodeFilter('日本 专线 01', combo)).toBe(true)
    expect(matchNodeFilter('新加坡 BGP 02', combo)).toBe(true)
    expect(matchNodeFilter('日本 2x高倍率', combo)).toBe(false)
    expect(matchNodeFilter('香港-日本中转', combo)).toBe(false)

    expect(matchNodeFilter('HK BGP IPLC-01', 'hk bgp -倍率')).toBe(true)
    expect(matchNodeFilter('HK BGP 3倍率', 'hk bgp -倍率')).toBe(false)

    expect(matchNodeFilter('英国-3倍率', '英国-3倍率')).toBe(true)
    expect(matchNodeFilter('香港1 (005024)', '香港1 (005024)')).toBe(true)
  })
})

describe('BUG-392: hasEffectiveFilterTerm 与 isIncompleteFilterInput 过滤输入完整性判定', () => {
  it('空串、纯空白及孤立操作符不含有效检索词，恒判为未完成态', () => {
    for (const emptyOrBare of [
      '',
      '   ',
      '-',
      '!',
      '！',
      '|',
      '｜',
      '- | !',
      '  -  ',
    ]) {
      expect(hasEffectiveFilterTerm(emptyOrBare)).toBe(false)
      expect(isIncompleteFilterInput(emptyOrBare)).toBe(true)
    }
  })

  it('结尾带悬空排除符或悬空 OR 符的输入包含前序有效词，但仍属半输入态', () => {
    for (const trailing of [
      '日本 -',
      '-香港 -',
      '-香港|-',
      '日本|',
      '日本 ｜',
      '日本 !',
      '日本 ！',
    ]) {
      expect(hasEffectiveFilterTerm(trailing)).toBe(true)
      expect(isIncompleteFilterInput(trailing)).toBe(true)
    }
  })

  it('完整过滤词及合法连字符前缀词条均放行（不被误拦）', () => {
    for (const complete of [
      '-香港',
      '!hk',
      '！倍率',
      'hk-',
      '英国-3倍率',
      '日本 | 新加坡 -倍率',
    ]) {
      expect(hasEffectiveFilterTerm(complete)).toBe(true)
      expect(isIncompleteFilterInput(complete)).toBe(false)
    }
  })
})

describe('BUG-397: resolveBatchCheckIntent 批量测速入口意图边界', () => {
  it('闪电图标与 F4 仅做纯测速：既不切节点也不清理存量连接', () => {
    expect(resolveBatchCheckIntent('bolt')).toEqual({
      select: false,
      cleanStale: false,
    })
    expect(resolveBatchCheckIntent('f4')).toEqual({
      select: false,
      cleanStale: false,
    })
  })

  it('过滤框输入与 F1 全量优选触发切节点并精准清理非新目标旧代理连接', () => {
    expect(resolveBatchCheckIntent('filter')).toEqual({
      select: true,
      cleanStale: true,
    })
    expect(resolveBatchCheckIntent('f1')).toEqual({
      select: true,
      cleanStale: true,
    })
  })
})
