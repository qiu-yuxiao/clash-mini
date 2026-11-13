import { describe, expect, it } from 'vitest'

import type { IConnectionsItem } from '@/types/connection'
import { parseManagedMergeRuleLine } from '@/utils/quick-routing'

import {
  composeActivePathControlConnections,
  createDisconnectedRecordFromConnection,
  enrichClosedPathControlConnections,
  extractConnectionTarget,
  formatPathControlHostLabel,
  matchesConnectionTarget,
  prioritizePathControlConnections,
  reconcileDisconnectedRecords,
  resolveConnectionControlStatus,
} from './path-control'

function makeConn(
  overrides: Partial<IConnectionsItem> & {
    host?: string
    destinationIP?: string
    process?: string
  } = {},
): IConnectionsItem {
  const {
    host = 'play.googleapis.com',
    destinationIP = '142.250.190.46',
    process = 'chrome.exe',
    ...rest
  } = overrides
  return {
    id: rest.id ?? 'conn-1',
    metadata: rest.metadata ?? {
      network: 'tcp',
      type: 'TLS',
      host,
      sourceIP: '127.0.0.1',
      sourcePort: '50000',
      destinationPort: '443',
      destinationIP,
      process,
    },
    upload: rest.upload ?? 100,
    download: rest.download ?? 200,
    start: rest.start ?? '2026-11-13T10:00:00.000Z',
    chains: rest.chains ?? ['PROXY', 'HK-01'],
    rule: rest.rule ?? 'Match',
    rulePayload: rest.rulePayload ?? '',
  }
}

describe('path-control 四态展示与留存闭环（BUG-367）', () => {
  it('四态解析精确区分 直连 / 代理 / 封锁 / 已断开', () => {
    expect(
      resolveConnectionControlStatus(makeConn({ chains: ['DIRECT'] })),
    ).toBe('direct')
    expect(
      resolveConnectionControlStatus(makeConn({ chains: ['HK-01', 'PROXY'] })),
    ).toBe('proxy')
    expect(
      resolveConnectionControlStatus(makeConn({ chains: ['REJECT'] })),
    ).toBe('reject')
    expect(
      resolveConnectionControlStatus(
        makeConn({ id: 'conn-manual', chains: ['PROXY'] }),
        {
          disconnectedIds: new Set(['conn-manual']),
        },
      ),
    ).toBe('disconnected')
  })

  it('封锁 play.googleapis.com 后即使无活跃 socket 也必须在表中保留可恢复行', () => {
    const rejectRule = parseManagedMergeRuleLine(
      'DOMAIN,play.googleapis.com,REJECT',
    )!
    const directRule = parseManagedMergeRuleLine('IP-CIDR,1.2.3.4/32,DIRECT')!

    const rows = composeActivePathControlConnections({
      activeConnections: [
        makeConn({
          id: 'live-daily',
          host: 'daily-cloudcode-pa.googleapis.com',
          chains: ['PROXY'],
        }),
      ],
      managedRules: [rejectRule, directRule],
      disconnectedRecords: [],
    })

    expect(rows).toHaveLength(3)
    expect(rows[0].metadata.host).toBe('daily-cloudcode-pa.googleapis.com')
    expect(rows[0]._controlStatus).toBe('proxy')
    expect(rows[0]._managedRule).toBeUndefined()

    expect(rows[1]._isSynthetic).toBe(true)
    expect(rows[1]._controlStatus).toBe('reject')
    expect(rows[1].metadata.host).toBe('play.googleapis.com')
    expect(rows[1]._managedRule?.raw).toBe('DOMAIN,play.googleapis.com,REJECT')

    expect(rows[2]._isSynthetic).toBe(true)
    expect(rows[2]._controlStatus).toBe('direct')
    expect(rows[2].metadata.destinationIP).toBe('1.2.3.4')
  })

  it('历史遗留 DOMAIN-SUFFIX 规则在表内显式呈现 *.domain 且不干扰精确目标复制', () => {
    const legacyRule = parseManagedMergeRuleLine(
      'DOMAIN-SUFFIX,googleapis.com,REJECT',
    )!
    const rows = composeActivePathControlConnections({
      activeConnections: [],
      managedRules: [legacyRule],
      disconnectedRecords: [],
    })

    expect(rows).toHaveLength(1)
    expect(formatPathControlHostLabel(rows[0])).toBe('*.googleapis.com')
    expect(rows[0].metadata.host).toBe('googleapis.com')
    expect(rows[0]._controlStatus).toBe('reject')
  })

  it('手动断开连接后表内保留「已断开」条目，且重连新流到达时自动消除标记', () => {
    const conn = makeConn({
      id: 'conn-play-1',
      host: 'play.googleapis.com',
      start: '2026-11-13T10:00:00.000Z',
    })
    const disconnectedAt = Date.parse('2026-11-13T10:01:00.000Z')
    const record = createDisconnectedRecordFromConnection(conn, disconnectedAt)!

    // Mihomo 移除活跃连接后，表中依然保留该已断开链接
    const afterDisconnect = composeActivePathControlConnections({
      activeConnections: [],
      managedRules: [],
      disconnectedRecords: [record],
    })
    expect(afterDisconnect).toHaveLength(1)
    expect(afterDisconnect[0]._controlStatus).toBe('disconnected')
    expect(afterDisconnect[0]._isSynthetic).toBe(true)
    expect(afterDisconnect[0].metadata.host).toBe('play.googleapis.com')

    // 若同时存在旧连接（早于断开时间），不误清除断开状态
    const withOlderConn = reconcileDisconnectedRecords(
      [record],
      [
        makeConn({
          id: 'conn-old',
          host: 'play.googleapis.com',
          start: '2026-11-13T09:59:00.000Z',
        }),
      ],
    )
    expect(withOlderConn).toHaveLength(1)

    // 客户端建立新活跃连接（start >= disconnectedAt - 500ms）后，自动清除已断开标记
    const reconciled = reconcileDisconnectedRecords(
      [record],
      [
        makeConn({
          id: 'conn-new',
          host: 'play.googleapis.com',
          start: '2026-11-13T10:01:02.000Z',
        }),
      ],
    )
    expect(reconciled).toHaveLength(0)
  })

  it('历史标签页对手动断开的连接显示「已断开」', () => {
    const enriched = enrichClosedPathControlConnections({
      closedConnections: [
        makeConn({ id: 'manual-closed', chains: ['PROXY'] }),
        makeConn({ id: 'normal-closed', chains: ['DIRECT'] }),
      ],
      managedRules: [],
      disconnectedIds: new Set(['manual-closed']),
    })

    expect(enriched[0]._controlStatus).toBe('disconnected')
    expect(enriched[1]._controlStatus).toBe('direct')
  })

  it('matchesConnectionTarget 严格精确匹配目标，不误关闭子域名或父域名连接', () => {
    const target = extractConnectionTarget(
      makeConn({ host: 'play.googleapis.com' }),
    )!
    expect(target.key).toBe('domain:play.googleapis.com')

    expect(
      matchesConnectionTarget(
        makeConn({ host: 'play.googleapis.com' }),
        target,
      ),
    ).toBe(true)
    expect(
      matchesConnectionTarget(
        makeConn({ host: 'sub.play.googleapis.com' }),
        target,
      ),
    ).toBe(false)
    expect(
      matchesConnectionTarget(
        makeConn({ host: 'daily-cloudcode-pa.googleapis.com' }),
        target,
      ),
    ).toBe(false)
  })

  it('prioritizePathControlConnections 置顶保留 已断开 与 封锁 虚拟行，闲置直连/代理固定规则置于活跃流之后', () => {
    const disconn = {
      ...makeConn({ id: 'd' }),
      _isSynthetic: true,
      _controlStatus: 'disconnected' as const,
    }
    const rej = {
      ...makeConn({ id: 'r' }),
      _isSynthetic: true,
      _controlStatus: 'reject' as const,
    }
    const liveNew = makeConn({ id: 'l2', start: '2026-11-13T10:02:00.000Z' })
    const liveOld = makeConn({ id: 'l1', start: '2026-11-13T10:00:00.000Z' })
    const idleProxy = {
      ...makeConn({ id: 'p' }),
      _isSynthetic: true,
      _controlStatus: 'proxy' as const,
    }

    const ordered = prioritizePathControlConnections(
      [idleProxy, liveOld, rej, liveNew, disconn],
      (list) =>
        list.sort(
          (a, b) => Date.parse(b.start || '0') - Date.parse(a.start || '0'),
        ),
    )

    expect(ordered.map((r) => r.id)).toEqual(['d', 'r', 'l2', 'l1', 'p'])
  })
})
