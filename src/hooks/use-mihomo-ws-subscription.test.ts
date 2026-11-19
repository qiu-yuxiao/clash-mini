import { beforeEach, describe, expect, it, vi } from 'vitest'

import {
  acquireSharedSubscriptionEntry,
  computeWsReconnectDelay,
  flushAllWsReconnectBackoff,
  MAX_RECONNECT_DELAY_MS,
  RECONNECT_DELAY_MS,
  REFRESH_CLASH_DEBOUNCE_MS,
  releaseSharedSubscriptionEntry,
  scheduleFlushWsReconnectBackoff,
} from '@/hooks/use-mihomo-ws-subscription'
import {
  batchTestWithFirstBatchSelect,
  isProxyHistoryFreshAndValid,
  triggerAutoSelectAndRefresh,
} from '@/pages/_layout/utils/profile-coordination'
import * as cmds from '@/services/cmds'
import { getDelayManager, HISTORY_FALLBACK_MAX_AGE_MS } from '@/services/delay'
import * as mihomoApi from '@/services/mihomo-api'
import type { MihomoWebSocket } from 'tauri-plugin-mihomo-api'

vi.mock('@/services/cmds', () => ({
  calcuProxies: vi.fn(),
  getProfiles: vi.fn(),
  notifyNodeSwitched: vi.fn(),
  triggerAutoSelect: vi.fn(),
}))

vi.mock('@/services/mihomo-api', () => ({
  getProxyByNameWithTimeout: vi.fn(),
}))

describe('BUG-347: computeWsReconnectDelay 指数退避、严格上限与快速重连冲刷', () => {
  it('第 0 次重连基准为 1000ms 且落在 ±20% 抖动区间 [800, 1200]', () => {
    for (let i = 0; i < 30; i++) {
      const delay = computeWsReconnectDelay(0)
      expect(delay).toBeGreaterThanOrEqual(Math.round(RECONNECT_DELAY_MS * 0.8))
      expect(delay).toBeLessThanOrEqual(Math.round(RECONNECT_DELAY_MS * 1.2))
    }
  })

  it('随重连次数指数递增且严格封顶于 MAX_RECONNECT_DELAY_MS (15000ms)，绝不因正抖动越界', () => {
    // attempt=2 -> base=4000 -> [3200, 4800]
    for (let i = 0; i < 20; i++) {
      const delay = computeWsReconnectDelay(2)
      expect(delay).toBeGreaterThanOrEqual(3200)
      expect(delay).toBeLessThanOrEqual(4800)
    }

    // attempt=10 -> 严格落在 [12000, 15000]，绝不超过 MAX_RECONNECT_DELAY_MS
    for (let i = 0; i < 40; i++) {
      const delay = computeWsReconnectDelay(10)
      expect(delay).toBeGreaterThanOrEqual(
        Math.round(MAX_RECONNECT_DELAY_MS * 0.8),
      )
      expect(delay).toBeLessThanOrEqual(MAX_RECONNECT_DELAY_MS)
    }
  })

  it('内核重启就绪时调用 flushAllWsReconnectBackoff 立即打断退避等待并发起即时重连', async () => {
    const fakeSocket = {
      addListener: vi.fn(),
      close: vi.fn().mockResolvedValue(undefined),
    } as unknown as MihomoWebSocket

    const connectMock = vi
      .fn<() => Promise<MihomoWebSocket>>()
      .mockRejectedValueOnce(new Error('pipe unavailable'))
      .mockResolvedValue(fakeSocket)

    const cacheKey = '$sub$test-ws-fast-reconnect'
    const entry = acquireSharedSubscriptionEntry(cacheKey, connectMock)
    await entry.connectWs()

    expect(connectMock).toHaveBeenCalledTimes(1)
    expect(entry.reconnectAttempt).toBe(1)
    expect(entry.reconnectTimer).not.toBeNull()

    // 无需等待退避定时器，触发冲刷后立即清零退避计数并发起第 2 次连接
    flushAllWsReconnectBackoff()
    await vi.waitFor(() => {
      expect(connectMock).toHaveBeenCalledTimes(2)
    })
    expect(entry.reconnectAttempt).toBe(0)
    expect(entry.reconnectTimer).toBeNull()

    releaseSharedSubscriptionEntry(cacheKey, entry)
  })

  it('内核重启时即便通道仍持有陈旧 ws 句柄，flushAllWsReconnectBackoff 也会主动关闭并立即重连，且 250ms 防抖合并连发事件', async () => {
    const staleClose = vi.fn().mockResolvedValue(undefined)
    const staleSocket = {
      addListener: vi.fn(),
      close: staleClose,
    } as unknown as MihomoWebSocket
    const freshSocket = {
      addListener: vi.fn(),
      close: vi.fn().mockResolvedValue(undefined),
    } as unknown as MihomoWebSocket

    const connectMock = vi
      .fn<() => Promise<MihomoWebSocket>>()
      .mockResolvedValueOnce(staleSocket)
      .mockResolvedValueOnce(freshSocket)

    const cacheKey = '$sub$test-ws-stale-socket'
    const entry = acquireSharedSubscriptionEntry(cacheKey, connectMock)
    await entry.connectWs()
    expect(entry.ws).toBe(staleSocket)

    // 模拟后端 reload 连发 3 次 refresh-clash-config：250ms 防抖合并为单次冲刷
    scheduleFlushWsReconnectBackoff()
    scheduleFlushWsReconnectBackoff()
    scheduleFlushWsReconnectBackoff()

    await new Promise((r) => setTimeout(r, REFRESH_CLASH_DEBOUNCE_MS + 60))
    await vi.waitFor(() => {
      expect(staleClose).toHaveBeenCalledTimes(1)
      expect(connectMock).toHaveBeenCalledTimes(2)
      expect(entry.ws).toBe(freshSocket)
    })

    releaseSharedSubscriptionEntry(cacheKey, entry)
  })
})

describe('BUG-321: batchTestWithFirstBatchSelect 严格传递过滤节点子集', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    getDelayManager().clearCache()
  })

  it('将过滤后的节点数组原样传给 triggerAutoSelect，禁止传 undefined 越界选点，且返回空数组时回滚 -2 占位', async () => {
    vi.mocked(cmds.getProfiles).mockResolvedValue({
      current: 'uid-test-1',
      items: [],
    } as Awaited<ReturnType<typeof cmds.getProfiles>>)
    vi.mocked(cmds.triggerAutoSelect).mockResolvedValue([])

    const originalTs = Date.now() - 90_000
    getDelayManager().restoreDelaySnapshot('日本 01', 'PROXY', {
      delay: 64,
      updatedAt: originalTs,
    })

    const subset = ['日本 01', '日本 02']
    await batchTestWithFirstBatchSelect(subset, true)

    expect(cmds.triggerAutoSelect).toHaveBeenCalledTimes(1)
    expect(cmds.triggerAutoSelect).toHaveBeenCalledWith(
      'uid-test-1',
      subset,
      0,
      true,
    )
    // 返回空数组（NotExecuted）时必须回滚 -2，且保留原始 updatedAt
    expect(getDelayManager().getDelayUpdate('日本 01', 'PROXY')).toEqual({
      delay: 64,
      updatedAt: originalTs,
    })
    expect(getDelayManager().getDelayUpdate('日本 02', 'PROXY')).toBeUndefined()
  })

  it('后端异常时自动回滚仍处于 -2 测试态的节点快照，防止永久转圈或虚增 TTL', async () => {
    vi.mocked(cmds.getProfiles).mockResolvedValue({
      current: 'uid-test-2',
      items: [],
    } as Awaited<ReturnType<typeof cmds.getProfiles>>)
    vi.mocked(cmds.triggerAutoSelect).mockRejectedValue(new Error('ipc busy'))

    const oldTs = Date.now() - 180_000
    getDelayManager().restoreDelaySnapshot('新加坡 01', 'PROXY', {
      delay: 88,
      updatedAt: oldTs,
    })
    await batchTestWithFirstBatchSelect(['新加坡 01', '新加坡 02'], false)

    expect(getDelayManager().getDelayUpdate('新加坡 01', 'PROXY')).toEqual({
      delay: 88,
      updatedAt: oldTs,
    })
    expect(
      getDelayManager().getDelayUpdate('新加坡 02', 'PROXY'),
    ).toBeUndefined()
  })
})

describe('BUG-402: 窗口唤醒新鲜延迟免拨测、传参对齐与移除 10s 二次补测器', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    getDelayManager().clearCache()
  })

  it('isProxyHistoryFreshAndValid 严格校验 alive、合法延迟区间与 HISTORY_FALLBACK_MAX_AGE_MS 时效', () => {
    const nowMs = Date.now()
    expect(
      isProxyHistoryFreshAndValid(
        {
          alive: true,
          history: [
            { time: new Date(nowMs - 60_000).toISOString(), delay: 88 },
          ],
        },
        nowMs,
      ),
    ).toBe(true)

    expect(
      isProxyHistoryFreshAndValid(
        {
          alive: false,
          history: [
            { time: new Date(nowMs - 60_000).toISOString(), delay: 88 },
          ],
        },
        nowMs,
      ),
    ).toBe(false)

    expect(
      isProxyHistoryFreshAndValid(
        {
          alive: true,
          history: [
            {
              time: new Date(
                nowMs - HISTORY_FALLBACK_MAX_AGE_MS - 1,
              ).toISOString(),
              delay: 88,
            },
          ],
        },
        nowMs,
      ),
    ).toBe(false)

    expect(
      isProxyHistoryFreshAndValid(
        {
          alive: true,
          history: [{ time: new Date(nowMs).toISOString(), delay: 0 }],
        },
        nowMs,
      ),
    ).toBe(false)

    expect(
      isProxyHistoryFreshAndValid({ alive: true, history: [] }, nowMs),
    ).toBe(false)
  })

  it('轻量唤醒开启 skipProbeIfFresh 且当前活跃出口持有新鲜有效延迟时，零拨测直接跳过 triggerAutoSelect', async () => {
    vi.mocked(cmds.getProfiles).mockResolvedValue({
      current: 'uid-fresh-1',
      items: [],
    } as Awaited<ReturnType<typeof cmds.getProfiles>>)

    const freshProxyA = {
      name: '日本 01',
      type: 'AnyTLS',
      udp: true,
      alive: true,
      history: [{ time: new Date().toISOString(), delay: 58 }],
    }
    const freshProxyB = {
      name: '日本 02',
      type: 'AnyTLS',
      udp: true,
      alive: true,
      history: [{ time: new Date().toISOString(), delay: 75 }],
    }
    vi.mocked(cmds.calcuProxies).mockResolvedValue({
      groups: [
        {
          name: 'PROXY',
          type: 'select',
          now: '日本 01',
          all: [freshProxyA, freshProxyB],
        },
      ],
      records: { '日本 01': freshProxyA, '日本 02': freshProxyB },
      proxies: [freshProxyA, freshProxyB],
    } as unknown as Awaited<ReturnType<typeof cmds.calcuProxies>>)
    vi.mocked(mihomoApi.getProxyByNameWithTimeout).mockResolvedValue({
      name: 'PROXY',
      now: '日本 01',
      all: ['日本 01', '日本 02'],
    } as Awaited<ReturnType<typeof mihomoApi.getProxyByNameWithTimeout>>)

    await triggerAutoSelectAndRefresh(
      vi.fn().mockResolvedValue(undefined),
      undefined,
      'uid-fresh-1',
      { skipProbeIfFresh: true },
    )

    expect(cmds.triggerAutoSelect).not.toHaveBeenCalled()
  })

  it('当前出口延迟缺失或陈旧时触发单次测速，且 15s 内不再调度历史遗留 10s 二次补测', async () => {
    vi.useFakeTimers()
    try {
      vi.mocked(cmds.getProfiles).mockResolvedValue({
        current: 'uid-stale-1',
        items: [],
      } as Awaited<ReturnType<typeof cmds.getProfiles>>)
      const staleProxyA = {
        name: '日本 01',
        type: 'AnyTLS',
        udp: true,
        alive: false,
        history: [],
      }
      vi.mocked(cmds.calcuProxies).mockResolvedValue({
        groups: [
          {
            name: 'PROXY',
            type: 'select',
            now: '日本 01',
            all: [staleProxyA],
          },
        ],
        records: { '日本 01': staleProxyA },
        proxies: [staleProxyA],
      } as unknown as Awaited<ReturnType<typeof cmds.calcuProxies>>)
      vi.mocked(mihomoApi.getProxyByNameWithTimeout).mockResolvedValue({
        name: 'PROXY',
        now: '日本 01',
        all: ['日本 01'],
      } as Awaited<ReturnType<typeof mihomoApi.getProxyByNameWithTimeout>>)
      vi.mocked(cmds.triggerAutoSelect).mockResolvedValue([['日本 01', 62]])

      const p = triggerAutoSelectAndRefresh(
        vi.fn().mockResolvedValue(undefined),
        undefined,
        'uid-stale-1',
        { skipProbeIfFresh: true },
      )
      await vi.advanceTimersByTimeAsync(15_000)
      await p

      expect(cmds.triggerAutoSelect).toHaveBeenCalledTimes(1)
      expect(cmds.triggerAutoSelect).toHaveBeenCalledWith(
        'uid-stale-1',
        ['日本 01'],
        0,
        false,
      )
    } finally {
      vi.useRealTimers()
    }
  })
})
