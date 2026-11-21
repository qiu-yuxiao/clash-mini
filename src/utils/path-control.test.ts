import { describe, expect, it } from 'vitest'

import type { IConnectionsItem } from '@/types/connection'
import { parseManagedMergeRuleLine } from '@/utils/quick-routing'

import {
  createDisconnectedRecordFromConnection,
  extractConnectionTarget,
  formatPathControlHostLabel,
  matchesConnectionTarget,
  partitionPathControlConnections,
  prioritizePathControlConnections,
  reconcileDisconnectedRecords,
  resolveConnectionControlStatus,
  resolveConnectionNodeDisplay,
  shouldCloseStaleProxyConnection,
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

  it('活跃视图仅保留实时 直连/代理 连接，封锁规则及无活跃流的自定义规则归入已关闭视图且封锁置顶', () => {
    const rejectRule = parseManagedMergeRuleLine(
      'DOMAIN,play.googleapis.com,REJECT',
    )!
    const directRule = parseManagedMergeRuleLine('IP-CIDR,1.2.3.4/32,DIRECT')!
    const liveDirectRule = parseManagedMergeRuleLine(
      'DOMAIN,live-direct.example.com,DIRECT',
    )!

    const { active, closed } = partitionPathControlConnections({
      activeConnections: [
        makeConn({
          id: 'live-daily',
          host: 'daily-cloudcode-pa.googleapis.com',
          chains: ['PROXY'],
        }),
        makeConn({
          id: 'live-direct',
          host: 'live-direct.example.com',
          chains: ['DIRECT'],
        }),
        makeConn({
          id: 'transient-reject',
          host: 'ads.example.com',
          chains: ['REJECT'],
        }),
        makeConn({
          id: 'transient-disconnected',
          host: 'manual-drop.example.com',
          chains: ['PROXY'],
        }),
      ],
      closedConnections: [
        makeConn({
          id: 'closed-normal',
          host: 'finished.example.com',
          chains: ['DIRECT'],
        }),
      ],
      managedRules: [rejectRule, directRule, liveDirectRule],
      disconnectedRecords: [],
      disconnectedIds: new Set(['transient-disconnected']),
    })

    expect(active).toHaveLength(2)
    expect(active.map((r) => r.id)).toEqual(['live-daily', 'live-direct'])
    expect(active.map((r) => r._controlStatus)).toEqual(['proxy', 'direct'])
    expect(active[1]._managedRule?.raw).toBe(
      'DOMAIN,live-direct.example.com,DIRECT',
    )

    // 封锁规则置顶，其次闲置直连规则（正在活跃的 liveDirectRule 不重复展示），随后瞬态死链与历史关闭连接
    expect(closed.map((r) => r._controlStatus)).toEqual([
      'reject',
      'direct',
      'disconnected',
      'reject',
      'direct',
    ])
    expect(closed[0]._isSynthetic).toBe(true)
    expect(closed[0].metadata.host).toBe('play.googleapis.com')
    expect(closed[0]._managedRule?.raw).toBe(
      'DOMAIN,play.googleapis.com,REJECT',
    )
    expect(closed[1]._isSynthetic).toBe(true)
    expect(closed[1].metadata.destinationIP).toBe('1.2.3.4')
    expect(closed[2].id).toBe('transient-disconnected')
    expect(closed[2]._isClosed).toBe(true)
    expect(closed[3].id).toBe('transient-reject')
    expect(closed[3]._isClosed).toBe(true)
    expect(closed[4].id).toBe('closed-normal')
    expect(closed[4]._isClosed).toBe(true)
  })

  it('历史遗留 DOMAIN-SUFFIX 规则在已关闭视图显式呈现 *.domain 且不干扰精确目标复制', () => {
    const legacyRule = parseManagedMergeRuleLine(
      'DOMAIN-SUFFIX,googleapis.com,REJECT',
    )!
    const { active, closed } = partitionPathControlConnections({
      activeConnections: [],
      managedRules: [legacyRule],
      disconnectedRecords: [],
    })

    expect(active).toHaveLength(0)
    expect(closed).toHaveLength(1)
    expect(formatPathControlHostLabel(closed[0])).toBe('*.googleapis.com')
    expect(closed[0].metadata.host).toBe('googleapis.com')
    expect(closed[0]._controlStatus).toBe('reject')
  })

  it('手动断开连接后已关闭视图保留「已断开」条目，且重连新流到达时自动消除标记', () => {
    const conn = makeConn({
      id: 'conn-play-1',
      host: 'play.googleapis.com',
      start: '2026-11-13T10:00:00.000Z',
    })
    const disconnectedAt = Date.parse('2026-11-13T10:01:00.000Z')
    const record = createDisconnectedRecordFromConnection(conn, disconnectedAt)!

    // Mihomo 移除活跃连接后，已关闭视图依然保留该已断开链接，活跃视图保持纯净
    const afterDisconnect = partitionPathControlConnections({
      activeConnections: [],
      managedRules: [],
      disconnectedRecords: [record],
    })
    expect(afterDisconnect.active).toHaveLength(0)
    expect(afterDisconnect.closed).toHaveLength(1)
    expect(afterDisconnect.closed[0]._controlStatus).toBe('disconnected')
    expect(afterDisconnect.closed[0]._isSynthetic).toBe(true)
    expect(afterDisconnect.closed[0].metadata.host).toBe('play.googleapis.com')

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

  it('已关闭视图对手动断开的历史连接显示「已断开」', () => {
    const { active, closed } = partitionPathControlConnections({
      activeConnections: [],
      closedConnections: [
        makeConn({
          id: 'normal-closed',
          host: 'a.example.com',
          chains: ['DIRECT'],
        }),
        makeConn({
          id: 'manual-closed',
          host: 'b.example.com',
          chains: ['PROXY'],
        }),
      ],
      managedRules: [],
      disconnectedRecords: [],
      disconnectedIds: new Set(['manual-closed']),
    })

    expect(active).toHaveLength(0)
    expect(closed.map((r) => r.id)).toEqual(['manual-closed', 'normal-closed'])
    expect(closed[0]._controlStatus).toBe('disconnected')
    expect(closed[1]._controlStatus).toBe('direct')
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

    const suffixTarget = {
      type: 'domain' as const,
      value: 'oraclecloud.com',
      key: 'domain-suffix:oraclecloud.com',
    }
    expect(
      matchesConnectionTarget(
        makeConn({ host: 'oraclecloud.com' }),
        suffixTarget,
      ),
    ).toBe(true)
    expect(
      matchesConnectionTarget(
        makeConn({ host: 'console.ap-tokyo-1.oraclecloud.com' }),
        suffixTarget,
      ),
    ).toBe(true)
    expect(
      matchesConnectionTarget(
        makeConn({ host: 'fakeoraclecloud.com' }),
        suffixTarget,
      ),
    ).toBe(false)
  })

  it('prioritizePathControlConnections 置顶保留 封锁(REJECT) 与 已断开 虚拟行，闲置直连/代理固定规则置于活跃流之后', () => {
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
    const closedRow = {
      ...makeConn({ id: 'c' }),
      _isClosed: true,
    }

    const ordered = prioritizePathControlConnections(
      [closedRow, idleProxy, liveOld, disconn, liveNew, rej],
      (list) =>
        list.sort(
          (a, b) => Date.parse(b.start || '0') - Date.parse(a.start || '0'),
        ),
    )

    expect(ordered.map((r) => r.id)).toEqual(['r', 'd', 'l2', 'l1', 'p', 'c'])
  })

  it('partitionPathControlConnections 隔离活跃与已关闭连接，置顶封锁/已断开/闲置规则并将近期关闭连接按目标去重逆序排列', () => {
    const active: IConnectionsItem[] = [
      {
        id: 'live-1',
        metadata: {
          network: 'tcp',
          type: 'HTTPS',
          sourceIP: '127.0.0.1',
          destinationIP: '1.1.1.1',
          sourcePort: '1',
          destinationPort: '443',
          host: 'live.example.com',
          processPath: '',
        },
        upload: 100,
        download: 200,
        start: '2026-01-01T00:00:05Z',
        chains: ['Proxy'],
        rule: 'Match',
        rulePayload: '',
        curUpload: 10,
        curDownload: 20,
      },
    ]

    const closed: IConnectionsItem[] = [
      {
        id: 'closed-old-dup',
        metadata: {
          network: 'tcp',
          type: 'HTTPS',
          sourceIP: '127.0.0.1',
          destinationIP: '2.2.2.2',
          sourcePort: '2',
          destinationPort: '443',
          host: 'dup.example.com',
          processPath: '',
        },
        upload: 50,
        download: 60,
        start: '2026-01-01T00:00:01Z',
        chains: ['DIRECT'],
        rule: 'Match',
        rulePayload: '',
        curUpload: 99,
        curDownload: 88,
      },
      {
        id: 'closed-live-dup',
        metadata: {
          network: 'tcp',
          type: 'HTTPS',
          sourceIP: '127.0.0.1',
          destinationIP: '1.1.1.1',
          sourcePort: '3',
          destinationPort: '443',
          host: 'live.example.com',
          processPath: '',
        },
        upload: 10,
        download: 20,
        start: '2026-01-01T00:00:02Z',
        chains: ['Proxy'],
        rule: 'Match',
        rulePayload: '',
        curUpload: 5,
        curDownload: 6,
      },
      {
        id: 'closed-pinned-dup',
        metadata: {
          network: 'tcp',
          type: 'HTTPS',
          sourceIP: '127.0.0.1',
          destinationIP: '3.3.3.3',
          sourcePort: '4',
          destinationPort: '443',
          host: 'pinned.example.com',
          processPath: '',
        },
        upload: 10,
        download: 20,
        start: '2026-01-01T00:00:03Z',
        chains: ['DIRECT'],
        rule: 'DOMAIN',
        rulePayload: 'pinned.example.com',
        curUpload: 1,
        curDownload: 1,
      },
      {
        id: 'closed-older-uniq',
        metadata: {
          network: 'tcp',
          type: 'HTTPS',
          sourceIP: '127.0.0.1',
          destinationIP: '4.4.4.4',
          sourcePort: '5',
          destinationPort: '443',
          host: 'older.example.com',
          processPath: '',
        },
        upload: 30,
        download: 40,
        start: '2026-01-01T00:00:04Z',
        chains: ['Proxy'],
        rule: 'Match',
        rulePayload: '',
        curUpload: 7,
        curDownload: 8,
      },
      {
        id: 'closed-new-dup',
        metadata: {
          network: 'tcp',
          type: 'HTTPS',
          sourceIP: '127.0.0.1',
          destinationIP: '2.2.2.2',
          sourcePort: '6',
          destinationPort: '443',
          host: 'dup.example.com',
          processPath: '',
        },
        upload: 70,
        download: 80,
        start: '2026-01-01T00:00:00Z',
        chains: ['DIRECT'],
        rule: 'Match',
        rulePayload: '',
        curUpload: 123,
        curDownload: 456,
      },
    ]

    const record = createDisconnectedRecordFromConnection(
      makeConn({ id: 'disc-1', host: 'dropped.example.com' }),
      Date.parse('2026-01-01T00:00:06Z'),
    )!

    const partitioned = partitionPathControlConnections({
      activeConnections: active,
      closedConnections: closed,
      managedRules: [
        {
          raw: 'DOMAIN,pinned.example.com,DIRECT',
          ruleType: 'DOMAIN',
          value: 'pinned.example.com',
          matchKey: 'pinned.example.com',
          target: 'DIRECT',
        },
        {
          raw: 'DOMAIN,blocked.example.com,REJECT',
          ruleType: 'DOMAIN',
          value: 'blocked.example.com',
          matchKey: 'blocked.example.com',
          target: 'REJECT',
        },
      ],
      disconnectedRecords: [record],
    })

    expect(partitioned.active.map((r) => r.id)).toEqual(['live-1'])

    const closedRows = partitioned.closed.filter((r) => r._isClosed)
    expect(closedRows.map((r) => r.id)).toEqual([
      'closed-new-dup',
      'closed-older-uniq',
    ])
    expect(closedRows[0]?.curUpload).toBe(0)
    expect(closedRows[0]?.curDownload).toBe(0)

    const prioritizedClosed = prioritizePathControlConnections(
      partitioned.closed,
    )
    expect(prioritizedClosed.map((r) => r.id)).toEqual([
      'synthetic:rule:DOMAIN,blocked.example.com,REJECT',
      'synthetic:disconnected:domain:dropped.example.com',
      'synthetic:rule:DOMAIN,pinned.example.com,DIRECT',
      'closed-new-dup',
      'closed-older-uniq',
    ])
  })

  it('shouldCloseStaleProxyConnection 仅清理非目标节点的 PROXY 存量连接，保护直连与同目标连接 (BUG-396)', () => {
    expect(
      shouldCloseStaleProxyConnection(
        ['香港1 (181201)', 'PROXY'],
        '日本3 (181201)',
      ),
    ).toBe(true)
    expect(
      shouldCloseStaleProxyConnection(
        ['新加坡4 (181201)', 'PROXY'],
        '日本3 (181201)',
      ),
    ).toBe(true)
    expect(
      shouldCloseStaleProxyConnection(
        ['日本3 (181201)', 'PROXY'],
        '日本3 (181201)',
      ),
    ).toBe(false)
    expect(shouldCloseStaleProxyConnection(['DIRECT'], '日本3 (181201)')).toBe(
      false,
    )
    expect(shouldCloseStaleProxyConnection(['REJECT'], '日本3 (181201)')).toBe(
      false,
    )
    expect(
      shouldCloseStaleProxyConnection(['香港1 (181201)', 'PROXY'], 'DIRECT'),
    ).toBe(false)
    expect(
      shouldCloseStaleProxyConnection(['香港1 (181201)', 'PROXY'], ''),
    ).toBe(false)
  })

  it('resolveConnectionNodeDisplay 在详情弹窗准确解析出口节点与旧节点滞留提示 (BUG-396)', () => {
    expect(
      resolveConnectionNodeDisplay({
        chains: ['日本3 (181201)', 'PROXY'],
        currentProxyNow: '日本3 (181201)',
      }),
    ).toEqual({
      kind: 'proxy',
      nodeName: '日本3 (181201)',
      isStaleProxyNode: false,
    })

    expect(
      resolveConnectionNodeDisplay({
        chains: ['香港1 (181201)', 'PROXY'],
        currentProxyNow: '日本3 (181201)',
      }),
    ).toEqual({
      kind: 'proxy',
      nodeName: '香港1 (181201)',
      isStaleProxyNode: true,
    })

    expect(
      resolveConnectionNodeDisplay({
        chains: ['香港1 (181201)', 'PROXY'],
        isClosed: true,
        currentProxyNow: '日本3 (181201)',
      }),
    ).toEqual({
      kind: 'proxy',
      nodeName: '香港1 (181201)',
      isStaleProxyNode: false,
    })

    expect(
      resolveConnectionNodeDisplay({
        chains: ['DIRECT'],
        currentProxyNow: '日本3 (181201)',
      }),
    ).toEqual({
      kind: 'direct',
      nodeName: 'DIRECT',
      isStaleProxyNode: false,
    })

    expect(
      resolveConnectionNodeDisplay({
        chains: ['REJECT'],
        currentProxyNow: '日本3 (181201)',
      }),
    ).toEqual({
      kind: 'reject',
      nodeName: 'REJECT',
      isStaleProxyNode: false,
    })

    expect(
      resolveConnectionNodeDisplay({
        chains: ['PROXY'],
        isSynthetic: true,
        currentProxyNow: '日本3 (181201)',
      }),
    ).toEqual({
      kind: 'none',
      nodeName: '--',
      isStaleProxyNode: false,
    })

    expect(
      resolveConnectionNodeDisplay({
        chains: ['DISCONNECTED'],
        isSynthetic: true,
        currentProxyNow: '日本3 (181201)',
      }),
    ).toEqual({
      kind: 'none',
      nodeName: '--',
      isStaleProxyNode: false,
    })
  })
})
