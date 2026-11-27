import type { IConnectionsItem } from '@/types/connection'
import {
  isIPv4Address,
  isIPv6Address,
  normalizeQuickTargetHost,
  type ManagedMergeRule,
} from '@/utils/quick-routing'

export type PathControlStatus = 'direct' | 'proxy' | 'reject' | 'disconnected'

export type PathControlViewMode = 'active' | 'closed'

export interface PartitionedPathControlConnections {
  active: PathControlConnectionItem[]
  closed: PathControlConnectionItem[]
}

export interface PathControlTarget {
  type: 'domain' | 'process'
  value: string
  /** Normalized lookup key: `domain:<h>` | `ipv4:<ip>` | `ipv6:<ip>` | `process:<p>`. */
  key: string
}

export interface DisconnectedTargetRecord {
  key: string
  type: 'domain' | 'process'
  value: string
  host: string
  destinationIP: string
  destinationPort: string
  network: string
  connType: string
  process: string
  processPath: string
  upload: number
  download: number
  start: string
  disconnectedAt: number
  disconnectedConnId: string
}

export interface PathControlConnectionItem extends IConnectionsItem {
  _controlStatus?: PathControlStatus
  _isSynthetic?: boolean
  _isClosed?: boolean
  _managedRule?: ManagedMergeRule
  _target?: PathControlTarget
  _disconnectedKey?: string
}

const DISCONNECTED_RECORDS_STORAGE_KEY =
  'clash_mini_path_control_disconnected_v1'
const DISCONNECTED_IDS_STORAGE_KEY =
  'clash_mini_path_control_disconnected_ids_v1'

const MAX_DISCONNECTED_RECORDS = 100
const MAX_DISCONNECTED_IDS = 500
const RECONNECT_CLOCK_SKEW_TOLERANCE_MS = 500

function getStorage(): Storage | null {
  try {
    if (typeof localStorage === 'undefined') return null
    return localStorage
  } catch {
    return null
  }
}

export function loadDisconnectedRecords(): DisconnectedTargetRecord[] {
  const storage = getStorage()
  if (!storage) return []
  try {
    const raw = storage.getItem(DISCONNECTED_RECORDS_STORAGE_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter(
        (item): item is DisconnectedTargetRecord =>
          Boolean(item) &&
          typeof item === 'object' &&
          typeof item.key === 'string' &&
          typeof item.value === 'string' &&
          typeof item.disconnectedAt === 'number',
      )
      .slice(0, MAX_DISCONNECTED_RECORDS)
  } catch {
    return []
  }
}

export function saveDisconnectedRecords(
  records: DisconnectedTargetRecord[],
): void {
  const storage = getStorage()
  if (!storage) return
  try {
    storage.setItem(
      DISCONNECTED_RECORDS_STORAGE_KEY,
      JSON.stringify(records.slice(0, MAX_DISCONNECTED_RECORDS)),
    )
  } catch (err) {
    console.warn('Failed to persist disconnected path control records:', err)
  }
}

export function loadDisconnectedIds(): string[] {
  const storage = getStorage()
  if (!storage) return []
  try {
    const raw = storage.getItem(DISCONNECTED_IDS_STORAGE_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter((id): id is string => typeof id === 'string' && id.length > 0)
      .slice(0, MAX_DISCONNECTED_IDS)
  } catch {
    return []
  }
}

export function saveDisconnectedIds(ids: string[]): void {
  const storage = getStorage()
  if (!storage) return
  try {
    storage.setItem(
      DISCONNECTED_IDS_STORAGE_KEY,
      JSON.stringify(ids.slice(0, MAX_DISCONNECTED_IDS)),
    )
  } catch (err) {
    console.warn('Failed to persist disconnected connection IDs:', err)
  }
}

function isDirectConnection(conn: { chains?: string[] }): boolean {
  const leaf = Array.isArray(conn.chains) ? conn.chains[0]?.trim() : ''
  return Boolean(leaf && leaf.toUpperCase() === 'DIRECT')
}

/**
 * Resolves the true destination IP of a connection.
 * Prefers `metadata.destinationIP`; falls back to `metadata.remoteDestination`
 * only for `DIRECT` connections because proxied connections populate
 * `remoteDestination` with the proxy server's IP.
 */
export function resolveConnectionDestinationIp(
  conn: Pick<IConnectionsItem, 'metadata'> & { chains?: string[] },
): string {
  const destinationIp = conn.metadata?.destinationIP?.trim()
  if (destinationIp) {
    return destinationIp
  }
  if (isDirectConnection(conn)) {
    return conn.metadata?.remoteDestination?.trim() || ''
  }
  return ''
}

function appendDestinationPort(base: string, port?: string): string {
  const trimmedPort = port?.trim()
  if (!base || base === '--' || !trimmedPort) {
    return base
  }
  return isIPv6Address(base)
    ? `[${base}]:${trimmedPort}`
    : `${base}:${trimmedPort}`
}

function targetFromDomainOrIp(rawHostOrIp: string): PathControlTarget | null {
  const norm = normalizeQuickTargetHost(rawHostOrIp)
  if (!norm) return null
  if (isIPv4Address(norm)) {
    return { type: 'domain', value: norm, key: `ipv4:${norm}` }
  }
  if (isIPv6Address(norm)) {
    return { type: 'domain', value: norm, key: `ipv6:${norm}` }
  }
  return { type: 'domain', value: norm, key: `domain:${norm}` }
}

/**
 * Extracts the exact Path Control target descriptor from a connection row.
 * Priority matches context-menu operations: `metadata.host` -> `metadata.destinationIP` -> `metadata.process`.
 */
export function extractConnectionTarget(
  conn: Pick<IConnectionsItem, 'metadata'> & { chains?: string[] },
  managedRule?: ManagedMergeRule,
): PathControlTarget | null {
  const rawHost = conn.metadata?.host?.trim()
  if (rawHost) {
    return targetFromDomainOrIp(rawHost)
  }

  const rawIp = resolveConnectionDestinationIp(conn)
  if (rawIp) {
    return targetFromDomainOrIp(rawIp)
  }

  const rawProc = conn.metadata?.process?.trim()
  if (rawProc) {
    return {
      type: 'process',
      value: rawProc,
      key: `process:${rawProc.toLowerCase()}`,
    }
  }

  if (managedRule) {
    if (managedRule.ruleType === 'PROCESS-NAME') {
      return {
        type: 'process',
        value: managedRule.value,
        key: `process:${managedRule.matchKey}`,
      }
    }
    return targetFromDomainOrIp(managedRule.matchKey)
  }

  return null
}

/**
 * Checks whether an active connection exactly matches the given target descriptor.
 */
export function matchesConnectionTarget(
  conn: Pick<IConnectionsItem, 'metadata'> & { chains?: string[] },
  target: PathControlTarget,
): boolean {
  if (target.type === 'process') {
    const proc = conn.metadata?.process?.trim().toLowerCase() ?? ''
    return proc.length > 0 && proc === target.value.toLowerCase()
  }

  const host = normalizeQuickTargetHost(conn.metadata?.host ?? '')
  if (target.key.startsWith('domain-suffix:')) {
    return (
      host.length > 0 &&
      (host === target.value || host.endsWith(`.${target.value}`))
    )
  }
  if (target.key.startsWith('domain:')) {
    return host === target.value
  }

  const destIp = normalizeQuickTargetHost(resolveConnectionDestinationIp(conn))
  return host === target.value || destIp === target.value
}

export function extractTargetKeyForManagedRule(rule: ManagedMergeRule): string {
  switch (rule.ruleType) {
    case 'DOMAIN':
      return `domain:${rule.matchKey}`
    case 'DOMAIN-SUFFIX':
      return `domain-suffix:${rule.matchKey}`
    case 'IP-CIDR':
      return isIPv4Address(rule.matchKey)
        ? `ipv4:${rule.matchKey}`
        : `cidr4:${rule.value}`
    case 'IP-CIDR6':
      return isIPv6Address(rule.matchKey)
        ? `ipv6:${rule.matchKey}`
        : `cidr6:${rule.value}`
    case 'PROCESS-NAME':
      return `process:${rule.matchKey}`
  }
}

export function findMatchingManagedRule(
  conn: Pick<IConnectionsItem, 'metadata'> & { chains?: string[] },
  managedRules: ManagedMergeRule[],
): ManagedMergeRule | null {
  if (managedRules.length === 0) return null

  const host = normalizeQuickTargetHost(conn.metadata?.host ?? '')
  const destIp = normalizeQuickTargetHost(resolveConnectionDestinationIp(conn))
  const proc = conn.metadata?.process?.trim().toLowerCase() ?? ''

  for (const rule of managedRules) {
    switch (rule.ruleType) {
      case 'DOMAIN':
        if (host && host === rule.matchKey) return rule
        break
      case 'DOMAIN-SUFFIX':
        if (
          host &&
          (host === rule.matchKey || host.endsWith(`.${rule.matchKey}`))
        ) {
          return rule
        }
        break
      case 'IP-CIDR':
      case 'IP-CIDR6':
        if (
          (destIp && destIp === rule.matchKey) ||
          (host && host === rule.matchKey)
        ) {
          return rule
        }
        break
      case 'PROCESS-NAME':
        if (proc && proc === rule.matchKey) return rule
        break
    }
  }

  return null
}

/**
 * Resolves the 4-state routing indicator for a row:
 * `direct` | `proxy` | `reject` | `disconnected`.
 */
export function resolveConnectionControlStatus(
  conn: PathControlConnectionItem,
  options?: {
    disconnectedIds?: Set<string>
    matchedRule?: ManagedMergeRule | null
  },
): PathControlStatus {
  if (conn._controlStatus) return conn._controlStatus
  if (options?.disconnectedIds?.has(conn.id)) return 'disconnected'

  const chains = Array.isArray(conn.chains) ? conn.chains : []
  if (
    chains.some((c) => {
      const upper = c.toUpperCase()
      return upper === 'REJECT' || upper === 'REJECT-DROP'
    })
  ) {
    return 'reject'
  }
  if (chains.some((c) => c.toUpperCase() === 'DIRECT')) {
    return 'direct'
  }

  const rule = options?.matchedRule ?? conn._managedRule
  if (rule?.target === 'REJECT') {
    return 'reject'
  }

  return 'proxy'
}

export function createDisconnectedRecordFromConnection(
  conn: PathControlConnectionItem,
  nowMs = Date.now(),
): DisconnectedTargetRecord | null {
  const target =
    conn._target ?? extractConnectionTarget(conn, conn._managedRule)
  if (!target) return null

  return {
    key: target.key,
    type: target.type,
    value: target.value,
    host: conn.metadata?.host ?? '',
    destinationIP: resolveConnectionDestinationIp(conn),
    destinationPort: conn.metadata?.destinationPort ?? '',
    network: conn.metadata?.network ?? 'tcp',
    connType: conn.metadata?.type ?? '',
    process: conn.metadata?.process ?? '',
    processPath: conn.metadata?.processPath ?? '',
    upload: typeof conn.upload === 'number' ? conn.upload : 0,
    download: typeof conn.download === 'number' ? conn.download : 0,
    start: conn.start || new Date(nowMs).toISOString(),
    disconnectedAt: nowMs,
    disconnectedConnId: conn.id,
  }
}

export function upsertDisconnectedRecord(
  records: DisconnectedTargetRecord[],
  record: DisconnectedTargetRecord,
): DisconnectedTargetRecord[] {
  const filtered = records.filter((item) => item.key !== record.key)
  return [record, ...filtered].slice(0, MAX_DISCONNECTED_RECORDS)
}

export function appendDisconnectedId(ids: string[], id: string): string[] {
  if (!id || id.startsWith('synthetic:')) return ids
  const filtered = ids.filter((item) => item !== id)
  return [id, ...filtered].slice(0, MAX_DISCONNECTED_IDS)
}

/**
 * Automatically drops disconnected markers once Mihomo establishes a newer live active socket
 * for the same target (`conn.start >= record.disconnectedAt - 500ms`).
 */
export function reconcileDisconnectedRecords(
  records: DisconnectedTargetRecord[],
  activeConnections: IConnectionsItem[],
): DisconnectedTargetRecord[] {
  if (records.length === 0 || activeConnections.length === 0) return records

  const latestActiveStartByKey = new Map<string, number>()
  for (const conn of activeConnections) {
    const target = extractConnectionTarget(conn)
    if (!target) continue
    const startMs = Date.parse(conn.start)
    if (!Number.isFinite(startMs)) continue
    const prev = latestActiveStartByKey.get(target.key)
    if (prev === undefined || startMs > prev) {
      latestActiveStartByKey.set(target.key, startMs)
    }
  }

  let changed = false
  const next = records.filter((record) => {
    const latestStart = latestActiveStartByKey.get(record.key)
    if (
      latestStart !== undefined &&
      latestStart >= record.disconnectedAt - RECONNECT_CLOCK_SKEW_TOLERANCE_MS
    ) {
      changed = true
      return false
    }
    return true
  })

  return changed ? next : records
}

function buildSyntheticDisconnectedRow(
  record: DisconnectedTargetRecord,
  matchedRule: ManagedMergeRule | null,
): PathControlConnectionItem {
  const target: PathControlTarget = {
    type: record.type,
    value: record.value,
    key: record.key,
  }
  return {
    id: `synthetic:disconnected:${record.key}`,
    metadata: {
      network: record.network || 'tcp',
      type: record.connType || 'Disconnected',
      host: record.host,
      sourceIP: '',
      sourcePort: '',
      destinationPort: record.destinationPort || '',
      destinationIP: record.destinationIP || undefined,
      process: record.process || undefined,
      processPath: record.processPath || undefined,
    },
    upload: record.upload,
    download: record.download,
    start: record.start,
    chains: ['DISCONNECTED'],
    rule: matchedRule?.ruleType ?? 'DISCONNECTED',
    rulePayload: matchedRule?.value ?? record.value,
    curUpload: 0,
    curDownload: 0,
    _controlStatus: 'disconnected',
    _isSynthetic: true,
    _managedRule: matchedRule ?? undefined,
    _target: target,
    _disconnectedKey: record.key,
  }
}

function buildSyntheticManagedRuleRow(
  rule: ManagedMergeRule,
): PathControlConnectionItem {
  const host =
    rule.ruleType === 'DOMAIN' || rule.ruleType === 'DOMAIN-SUFFIX'
      ? rule.matchKey
      : ''
  const destinationIP =
    rule.ruleType === 'IP-CIDR' || rule.ruleType === 'IP-CIDR6'
      ? rule.matchKey
      : undefined
  const process = rule.ruleType === 'PROCESS-NAME' ? rule.value : undefined

  const target: PathControlTarget =
    rule.ruleType === 'PROCESS-NAME'
      ? {
          type: 'process',
          value: rule.value,
          key: `process:${rule.matchKey}`,
        }
      : (targetFromDomainOrIp(rule.matchKey) ?? {
          type: 'domain',
          value: rule.matchKey,
          key: extractTargetKeyForManagedRule(rule),
        })

  return {
    id: `synthetic:rule:${rule.raw}`,
    metadata: {
      network: 'RULE',
      type: rule.ruleType,
      host,
      sourceIP: '',
      sourcePort: '',
      destinationPort: '',
      destinationIP,
      process,
    },
    upload: 0,
    download: 0,
    start: '',
    chains: [rule.target],
    rule: rule.ruleType,
    rulePayload: rule.value,
    curUpload: 0,
    curDownload: 0,
    _controlStatus: rule.target.toLowerCase() as PathControlStatus,
    _isSynthetic: true,
    _managedRule: rule,
    _target: target,
  }
}

function matchDisconnectedRecordRule(
  record: DisconnectedTargetRecord,
  managedRules: ManagedMergeRule[],
): ManagedMergeRule | null {
  return findMatchingManagedRule(
    {
      metadata: {
        network: record.network,
        type: record.connType,
        host: record.host,
        sourceIP: '',
        sourcePort: '',
        destinationPort: record.destinationPort,
        destinationIP: record.destinationIP,
        process: record.process,
      },
    },
    managedRules,
  )
}

/**
 * Partitions Path Control items into:
 * - `active`: genuinely live `direct` / `proxy` connections only
 * - `closed`: top-pinned `REJECT` rules, manually `disconnected` records,
 *   idle `direct`/`proxy` Merge rules, and deduplicated recent closed connections
 */
export function partitionPathControlConnections(params: {
  activeConnections: IConnectionsItem[]
  closedConnections?: IConnectionsItem[]
  managedRules: ManagedMergeRule[]
  disconnectedRecords: DisconnectedTargetRecord[]
  disconnectedIds?: Set<string>
}): PartitionedPathControlConnections {
  const {
    activeConnections,
    closedConnections = [],
    managedRules,
    disconnectedRecords,
    disconnectedIds,
  } = params

  const liveActiveTargetKeys = new Set<string>()
  const liveActiveRuleRaws = new Set<string>()
  const active: PathControlConnectionItem[] = []
  const transientInactiveSockets: IConnectionsItem[] = []

  for (const conn of activeConnections) {
    const target = extractConnectionTarget(conn)
    const matchedRule = findMatchingManagedRule(conn, managedRules)
    const status = resolveConnectionControlStatus(conn, {
      disconnectedIds,
      matchedRule,
    })

    if (status === 'direct' || status === 'proxy') {
      if (target) {
        liveActiveTargetKeys.add(target.key)
      }
      if (matchedRule) {
        liveActiveRuleRaws.add(matchedRule.raw)
      }
      active.push({
        ...conn,
        _target: target ?? undefined,
        _managedRule: matchedRule ?? undefined,
        _controlStatus: status,
      })
    } else {
      transientInactiveSockets.push(conn)
    }
  }

  const coveredKeys = new Set<string>(liveActiveTargetKeys)
  const coveredRuleRaws = new Set<string>(liveActiveRuleRaws)
  const syntheticRejected: PathControlConnectionItem[] = []
  const syntheticDisconnected: PathControlConnectionItem[] = []
  const syntheticIdlePinned: PathControlConnectionItem[] = []

  for (const rule of managedRules) {
    if (rule.target !== 'REJECT') continue
    const ruleKey = extractTargetKeyForManagedRule(rule)
    if (coveredRuleRaws.has(rule.raw) || coveredKeys.has(ruleKey)) {
      continue
    }
    coveredRuleRaws.add(rule.raw)
    coveredKeys.add(ruleKey)
    syntheticRejected.push(buildSyntheticManagedRuleRow(rule))
  }

  for (const record of disconnectedRecords) {
    if (coveredKeys.has(record.key)) continue
    const matchedRule = matchDisconnectedRecordRule(record, managedRules)
    if (matchedRule && coveredRuleRaws.has(matchedRule.raw)) {
      continue
    }
    if (matchedRule) {
      coveredRuleRaws.add(matchedRule.raw)
    }
    coveredKeys.add(record.key)
    syntheticDisconnected.push(
      buildSyntheticDisconnectedRow(record, matchedRule),
    )
  }

  for (const rule of managedRules) {
    if (rule.target === 'REJECT') continue
    const ruleKey = extractTargetKeyForManagedRule(rule)
    if (coveredRuleRaws.has(rule.raw) || coveredKeys.has(ruleKey)) {
      continue
    }
    coveredRuleRaws.add(rule.raw)
    coveredKeys.add(ruleKey)
    syntheticIdlePinned.push(buildSyntheticManagedRuleRow(rule))
  }

  // Merge transient inactive sockets and recent closed connections in reverse
  // chronological order, deduplicated by target key.
  const recentClosedRows: PathControlConnectionItem[] = []
  const appendClosedCandidate = (conn: IConnectionsItem) => {
    const target = extractConnectionTarget(conn)
    if (!target || coveredKeys.has(target.key)) {
      return
    }
    const matchedRule = findMatchingManagedRule(conn, managedRules)
    if (matchedRule && coveredRuleRaws.has(matchedRule.raw)) {
      return
    }
    coveredKeys.add(target.key)
    if (matchedRule) {
      coveredRuleRaws.add(matchedRule.raw)
    }
    recentClosedRows.push({
      ...conn,
      curUpload: 0,
      curDownload: 0,
      _isClosed: true,
      _target: target,
      _managedRule: matchedRule ?? undefined,
      _controlStatus: resolveConnectionControlStatus(conn, {
        disconnectedIds,
        matchedRule,
      }),
    })
  }

  for (let i = transientInactiveSockets.length - 1; i >= 0; i -= 1) {
    appendClosedCandidate(transientInactiveSockets[i])
  }
  for (let i = closedConnections.length - 1; i >= 0; i -= 1) {
    appendClosedCandidate(closedConnections[i])
  }

  return {
    active,
    closed: [
      ...syntheticRejected,
      ...syntheticDisconnected,
      ...syntheticIdlePinned,
      ...recentClosedRows,
    ],
  }
}

export function formatPathControlHostLabel(
  row: Pick<
    PathControlConnectionItem,
    'metadata' | '_managedRule' | '_isSynthetic'
  > & { chains?: string[] },
): string {
  const host = row.metadata?.host?.trim()
  if (host) {
    if (row._isSynthetic && row._managedRule?.ruleType === 'DOMAIN-SUFFIX') {
      return `*.${host}`
    }
    return host
  }
  const destIp = resolveConnectionDestinationIp(row)
  if (destIp) return destIp
  return row.metadata?.process?.trim() || '--'
}

export function formatPathControlAddress(
  row: Pick<
    PathControlConnectionItem,
    'metadata' | '_managedRule' | '_isSynthetic'
  > & { chains?: string[] },
): string {
  return appendDestinationPort(
    formatPathControlHostLabel(row),
    row.metadata?.destinationPort,
  )
}

export function formatPathControlCopyAddress(
  row: Pick<IConnectionsItem, 'metadata'> & { chains?: string[] },
): string {
  const base =
    row.metadata?.host?.trim() ||
    resolveConnectionDestinationIp(row) ||
    row.metadata?.process?.trim() ||
    ''
  if (!base) return ''
  return appendDestinationPort(base, row.metadata?.destinationPort)
}

export function matchesPathControlSearch(
  conn: PathControlConnectionItem,
  match: (content: string) => boolean,
): boolean {
  const displayAddress = formatPathControlAddress(conn)
  const host = conn.metadata?.host?.trim() || ''
  const destIp = resolveConnectionDestinationIp(conn)
  const process = conn.metadata?.process?.trim() || ''

  const parts: string[] = []
  if (displayAddress && displayAddress !== '--') {
    parts.push(displayAddress)
  }
  if (host && host !== displayAddress) {
    parts.push(host)
  }
  if (destIp && destIp !== displayAddress && destIp !== host) {
    parts.push(destIp)
  }
  if (process && process !== displayAddress) {
    parts.push(process)
  }

  return match(parts.join(' '))
}

/**
 * Orders Path Control rows so retained `reject` and `disconnected` synthetic rows stay
 * pinned at the top, followed by live traffic, idle `direct`/`proxy` Merge rows, and closed rows.
 */
export function prioritizePathControlConnections(
  rows: PathControlConnectionItem[],
  orderLiveFn?: (list: IConnectionsItem[]) => IConnectionsItem[],
): PathControlConnectionItem[] {
  const syntheticRejected: PathControlConnectionItem[] = []
  const syntheticDisconnected: PathControlConnectionItem[] = []
  const liveRows: PathControlConnectionItem[] = []
  const syntheticIdlePinned: PathControlConnectionItem[] = []
  const closedRows: PathControlConnectionItem[] = []

  for (const row of rows) {
    if (!row._isSynthetic && row._isClosed) {
      closedRows.push(row)
    } else if (!row._isSynthetic) {
      liveRows.push(row)
    } else if (row._controlStatus === 'reject') {
      syntheticRejected.push(row)
    } else if (row._controlStatus === 'disconnected') {
      syntheticDisconnected.push(row)
    } else {
      syntheticIdlePinned.push(row)
    }
  }

  const orderedLive = orderLiveFn
    ? (orderLiveFn([...liveRows]) as PathControlConnectionItem[])
    : liveRows

  return [
    ...syntheticRejected,
    ...syntheticDisconnected,
    ...orderedLive,
    ...syntheticIdlePinned,
    ...closedRows,
  ]
}

/**
 * 判断某条活跃连接在前台用户显式切换至 `targetNode` 后是否应被立即断开（BUG-396）：
 * - 与后端 `module/monitor/connections.rs::should_close_stale_proxy_connection` 1:1 同源；
 * - 仅匹配经过 `groupName`（默认 `'PROXY'`）的连接，绝不触碰 `DIRECT` / `REJECT` 流量；
 * - 若连接已位于 `targetNode` 上，返回 `false` 保持零中断；
 * - 若连接仍挂在任意非 `targetNode` 的旧代理节点（含多跳前残留）上，返回 `true`。
 */
export function shouldCloseStaleProxyConnection(
  chains: string[],
  targetNode: string,
  groupName: string = 'PROXY',
): boolean {
  const target = targetNode.trim()
  const group = groupName.trim()
  if (!target || !group) return false
  const upperTarget = target.toUpperCase()
  if (
    upperTarget === 'DIRECT' ||
    upperTarget === 'REJECT' ||
    upperTarget === 'GLOBAL'
  ) {
    return false
  }
  return chains.includes(group) && !chains.includes(target)
}

export interface ConnectionNodeDisplay {
  kind: 'proxy' | 'direct' | 'reject' | 'none'
  nodeName: string
  isStaleProxyNode: boolean
}

/**
 * 解析路径控制左键详情弹窗中的实际出口节点展示状态（BUG-396）。
 */
export function resolveConnectionNodeDisplay(params: {
  chains?: string[]
  isSynthetic?: boolean
  isClosed?: boolean
  currentProxyNow?: string
}): ConnectionNodeDisplay {
  const {
    chains = [],
    isSynthetic = false,
    isClosed = false,
    currentProxyNow,
  } = params
  if (chains.length === 0) {
    return { kind: 'none', nodeName: '--', isStaleProxyNode: false }
  }
  const leaf = chains[0]?.trim() || ''
  const upperLeaf = leaf.toUpperCase()
  if (!leaf || upperLeaf === 'DISCONNECTED') {
    return { kind: 'none', nodeName: '--', isStaleProxyNode: false }
  }
  if (upperLeaf === 'DIRECT') {
    return { kind: 'direct', nodeName: 'DIRECT', isStaleProxyNode: false }
  }
  if (upperLeaf === 'REJECT' || upperLeaf === 'REJECT-DROP') {
    return { kind: 'reject', nodeName: leaf, isStaleProxyNode: false }
  }
  if (isSynthetic || upperLeaf === 'PROXY' || upperLeaf === 'GLOBAL') {
    return { kind: 'none', nodeName: '--', isStaleProxyNode: false }
  }

  const activeNow = currentProxyNow?.trim() || ''
  const upperActive = activeNow.toUpperCase()
  const isStaleProxyNode =
    !isClosed &&
    Boolean(activeNow) &&
    upperActive !== 'DIRECT' &&
    upperActive !== 'REJECT' &&
    upperActive !== 'GLOBAL' &&
    upperActive !== 'PROXY' &&
    leaf !== activeNow

  return {
    kind: 'proxy',
    nodeName: leaf,
    isStaleProxyNode,
  }
}
