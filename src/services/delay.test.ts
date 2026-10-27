import { describe, expect, it } from 'vitest'

import type { IProxyItem } from '@/types/clash'

import {
  formatDelay,
  formatDelayColor,
  getDelayManager,
  HISTORY_FALLBACK_MAX_AGE_MS,
  NODE_DELAY_MAX_MS,
  resolveSignalDelayColor,
} from './delay'

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
    expect(formatDelay(6000)).toBe('Timeout')
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
  /**
   * BUG-304 回归（2026-10-26）：后端 monitor.rs 判死阈值已 2000→5000，
   * 前端单一真源若不同步，2000~5000ms 的「拥塞期慢而可用」节点会被
   * formatDelay/formatDelayColor/排序/联动逻辑误判为 Timeout（红字 + 沉底），
   * 重现 BUG-304 要根除的「活节点全员 Time out」。
   */
  describe('BUG-304 阈值同步：2000~5000ms 是有效延迟而非 Timeout', () => {
    it('NODE_DELAY_MAX_MS 必须为 5000（与后端 monitor.rs 单一真源一致）', () => {
      expect(NODE_DELAY_MAX_MS).toBe(5000)
    })

    it('拥塞期实测延迟 (3000ms) 按有效值呈现，不得显示 Timeout', () => {
      expect(formatDelay(3000)).toBe('3000')
      expect(formatDelayColor(3000)).not.toBe('error.main')
    })

    it('拥塞期实测延迟 (4999ms) 仍为有效值，恰好 5000ms 才判 Timeout', () => {
      expect(formatDelay(4999)).toBe('4999')
      expect(formatDelay(5000)).toBe('Timeout')
    })
  })

  it('未测速与测试中 (< 0) 返回空样式', () => {
    expect(formatDelayColor(-1)).toBe('')
    expect(formatDelayColor(-2)).toBe('')
  })

  it('超时与失败 (0 或 >= timeout) 返回 error.main', () => {
    expect(formatDelayColor(0)).toBe('error.main')
    expect(formatDelayColor(NODE_DELAY_MAX_MS)).toBe('error.main')
    expect(formatDelayColor(6500)).toBe('error.main')
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

/**
 * 2026-10-24 实测事故回归：节点已断流（活跃节点栏 Timeout），
 * 节点列表却仍显示绿色 106ms（history 里最后一次成功值）。
 */
describe('getDelayFix 回落规则：禁止陈旧成功值冒充当前延迟', () => {
  const NODE = '回归-日本aw6'
  const makeProxy = (over: Partial<IProxyItem> = {}) =>
    ({
      name: NODE,
      type: 'AnyTLS',
      udp: true,
      history: [],
      ...over,
    }) as unknown as IProxyItem
  const freshHistory = () => [
    { time: new Date(Date.now() - 30_000).toISOString(), delay: 106 },
  ]

  it('内核已判死 (alive=false) 时按 Timeout(0) 呈现，不回落 history', () => {
    const dm = getDelayManager()
    dm.clearCache()
    const proxy = makeProxy({ alive: false, history: freshHistory() })
    expect(dm.getDelayFix(proxy)).toBe(0)
    expect(dm.formatDelay(dm.getDelayFix(proxy))).toBe('Timeout')
  })

  it('存活且 history 新鲜时正常回落真实延迟', () => {
    const dm = getDelayManager()
    dm.clearCache()
    const proxy = makeProxy({ alive: true, history: freshHistory() })
    expect(dm.getDelayFix(proxy)).toBe(106)
  })

  it('history 记录超过容忍时长后不再冒充当前延迟', () => {
    const dm = getDelayManager()
    dm.clearCache()
    const stale = [
      {
        time: new Date(
          Date.now() - HISTORY_FALLBACK_MAX_AGE_MS - 60_000,
        ).toISOString(),
        delay: 88,
      },
    ]
    expect(dm.getDelayFix(makeProxy({ alive: true, history: stale }))).toBe(0)
  })

  it('时间无法解析时保守回落原值，不误判为 Timeout', () => {
    const dm = getDelayManager()
    dm.clearCache()
    const proxy = makeProxy({
      alive: true,
      history: [{ time: 'not-a-time', delay: 77 }],
    })
    expect(dm.getDelayFix(proxy)).toBe(77)
  })

  it('完全无 history 且无缓存时返回 -1（未测速占位）', () => {
    const dm = getDelayManager()
    dm.clearCache()
    expect(dm.getDelayFix(makeProxy({ alive: true }))).toBe(-1)
  })

  it('有本次测速结论时以缓存为准（不被内核 alive 标记覆盖）', () => {
    // 依据 agreements 5.8：测速链路失真时内核会把全组 alive 置 false，
    // 此时前端已有的真实测量结论必须优先，否则会把所有节点误显示为 Timeout。
    const dm = getDelayManager()
    dm.clearCache()
    dm.setDelay(NODE, 'PROXY', 132)
    expect(dm.getDelayFix(makeProxy({ alive: false }))).toBe(132)
  })
})

describe('节点延迟监听：同一节点支持多订阅者', () => {
  const NODE = '回归-多订阅节点'

  it('活跃节点栏与列表项同时监听同一节点时都能收到更新', async () => {
    const dm = getDelayManager()
    dm.clearCache()
    dm.removeListener(NODE, 'PROXY')

    const upper: number[] = []
    const lower: number[] = []
    dm.setListener(NODE, 'PROXY', (u) => upper.push(u.delay))
    dm.setListener(NODE, 'PROXY', (u) => lower.push(u.delay))

    dm.injectBatchResults('PROXY', [[NODE, 0]])
    await new Promise((r) => setTimeout(r, 50))

    expect(upper).toEqual([0])
    expect(lower).toEqual([0])

    dm.removeListener(NODE, 'PROXY')
  })

  it('移除指定 listener 不会带走同节点的其它订阅者', async () => {
    const dm = getDelayManager()
    dm.clearCache()
    dm.removeListener(NODE, 'PROXY')

    const kept: number[] = []
    const removed: number[] = []
    const keptFn = (u: { delay: number }) => kept.push(u.delay)
    const removedFn = (u: { delay: number }) => removed.push(u.delay)

    dm.setListener(NODE, 'PROXY', keptFn)
    dm.setListener(NODE, 'PROXY', removedFn)
    dm.removeListener(NODE, 'PROXY', removedFn)

    dm.injectBatchResults('PROXY', [[NODE, 42]])
    await new Promise((r) => setTimeout(r, 50))

    expect(kept).toEqual([42])
    expect(removed).toEqual([])

    dm.removeListener(NODE, 'PROXY')
  })
})

describe('resolveSignalDelayColor 与 formatDelayColor 延迟颜色标准完全一致', () => {
  const samples = [
    0, 35, 180, 220, 249, 250, 280, 350, 399, 400, 450, 600, 3000, 5000, 120000,
  ]

  it('在浅色与深色主题下，顶部活跃节点栏颜色均与节点表格保持 100% 一致', () => {
    for (const isDark of [false, true]) {
      for (const delay of samples) {
        const expected = formatDelayColor(delay, NODE_DELAY_MAX_MS, isDark)
        expect(resolveSignalDelayColor(delay, NODE_DELAY_MAX_MS, isDark)).toBe(
          expected,
        )
      }
    }
  })

  it('未测速 (-1) 与测试中 (-2) 返回 text.secondary', () => {
    expect(resolveSignalDelayColor(-1)).toBe('text.secondary')
    expect(resolveSignalDelayColor(-2)).toBe('text.secondary')
  })
})
