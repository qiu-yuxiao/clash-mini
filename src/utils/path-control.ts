import type { IConnectionsItem } from '@/types/connection'
import {
  isIPv4Address,
  isIPv6Address,
  normalizeQuickTargetHost,
  type ManagedMergeRule,
} from '@/utils/quick-routing'

export type PathControlStatus = 'direct' | 'proxy' | 'reject' | 'disconnected'

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
  conn: Pick<IConnectionsItem, 'metadata'>,
  managedRule?: ManagedMergeRule,
): PathControlTarget | null {
  const rawHost = conn.metadata?.host?.trim()
  if (rawHost) {
    return targetFromDomainOrIp(rawHost)
  }

  const rawIp =
    conn.metadata?.destinationIP?.trim() ||
    conn.metadata?.remoteDestination?.trim()
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
  conn: Pick<IConnectionsItem, 'metadata'>,
  target: PathControlTarget,
): boolean {
  if (target.type === 'process') {
    const proc = conn.metadata?.process?.trim().toLowerCase() ?? ''
    return proc.length > 0 && proc === target.value.toLowerCase()
  }

  const host = normalizeQuickTargetHost(conn.metadata?.host ?? '')
  if (target.key.startsWith('domain:')) {
    return host === target.value
  }

  const destIp = normalizeQuickTargetHost(
    conn.metadata?.destinationIP || conn.metadata?.remoteDestination || '',
  )
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
  conn: Pick<IConnectionsItem, 'metadata'>,
  managedRules: ManagedMergeRule[],
): ManagedMergeRule | null {
  if (managedRules.length === 0) return null

  const host = normalizeQuickTargetHost(conn.metadata?.host ?? '')
  const destIp = normalizeQuickTargetHost(
    conn.metadata?.destinationIP || conn.metadata?.remoteDestination || '',
  )
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
    destinationIP: conn.metadata?.destinationIP ?? '',
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
 * Merges live Mihomo connections with persisted disconnected items and Merge override rules
 * so all 4 states stay visible and directly operable inside Path Control.
 */
export function composeActivePathControlConnections(params: {
  activeConnections: IConnectionsItem[]
  managedRules: ManagedMergeRule[]
  disconnectedRecords: DisconnectedTargetRecord[]
  disconnectedIds?: Set<string>
}): PathControlConnectionItem[] {
  const {
    activeConnections,
    managedRules,
    disconnectedRecords,
    disconnectedIds,
  } = params

  const liveTargetKeys = new Set<string>()
  const liveMatchedRuleRaws = new Set<string>()

  const enrichedLive: PathControlConnectionItem[] = activeConnections.map(
    (conn) => {
      const target = extractConnectionTarget(conn)
      const matchedRule = findMatchingManagedRule(conn, managedRules)
      if (target) {
        liveTargetKeys.add(target.key)
      }
      if (matchedRule) {
        liveMatchedRuleRaws.add(matchedRule.raw)
      }
      return {
        ...conn,
        _target: target ?? undefined,
        _managedRule: matchedRule ?? undefined,
        _controlStatus: resolveConnectionControlStatus(conn, {
          disconnectedIds,
          matchedRule,
        }),
      }
    },
  )

  const coveredKeys = new Set<string>(liveTargetKeys)
  const coveredRuleRaws = new Set<string>(liveMatchedRuleRaws)
  const syntheticRows: PathControlConnectionItem[] = []

  for (const record of disconnectedRecords) {
    if (liveTargetKeys.has(record.key)) continue
    const matchedRule = matchDisconnectedRecordRule(record, managedRules)
    if (matchedRule) {
      coveredRuleRaws.add(matchedRule.raw)
    }
    coveredKeys.add(record.key)
    syntheticRows.push(buildSyntheticDisconnectedRow(record, matchedRule))
  }

  for (const rule of managedRules) {
    const ruleKey = extractTargetKeyForManagedRule(rule)
    if (coveredRuleRaws.has(rule.raw) || coveredKeys.has(ruleKey)) {
      continue
    }
    coveredRuleRaws.add(rule.raw)
    coveredKeys.add(ruleKey)
    syntheticRows.push(buildSyntheticManagedRuleRow(rule))
  }

  return [...enrichedLive, ...syntheticRows]
}

/**
 * Enriches closed/history connections with 4-state resolution (`已断开` when manually disconnected)
 * and matching Merge rule metadata.
 */
export function enrichClosedPathControlConnections(params: {
  closedConnections: IConnectionsItem[]
  managedRules: ManagedMergeRule[]
  disconnectedIds?: Set<string>
}): PathControlConnectionItem[] {
  const { closedConnections, managedRules, disconnectedIds } = params
  return closedConnections.map((conn) => {
    const target = extractConnectionTarget(conn)
    const matchedRule = findMatchingManagedRule(conn, managedRules)
    return {
      ...conn,
      _target: target ?? undefined,
      _managedRule: matchedRule ?? undefined,
      _controlStatus: resolveConnectionControlStatus(conn, {
        disconnectedIds,
        matchedRule,
      }),
    }
  })
}

export function formatPathControlHostLabel(
  row: Pick<
    PathControlConnectionItem,
    'metadata' | '_managedRule' | '_isSynthetic'
  >,
): string {
  const host = row.metadata?.host?.trim()
  if (host) {
    if (row._isSynthetic && row._managedRule?.ruleType === 'DOMAIN-SUFFIX') {
      return `*.${host}`
    }
    return host
  }
  const destIp =
    row.metadata?.remoteDestination?.trim() ||
    row.metadata?.destinationIP?.trim()
  if (destIp) return destIp
  return row.metadata?.process?.trim() || '--'
}

/**
 * Orders Path Control rows so retained `disconnected` and `reject` synthetic rows stay
 * immediately visible above live traffic, while idle `direct`/`proxy` Merge rows follow live rows.
 */
export function prioritizePathControlConnections(
  rows: PathControlConnectionItem[],
  orderLiveFn?: (list: IConnectionsItem[]) => IConnectionsItem[],
): PathControlConnectionItem[] {
  const syntheticDisconnected: PathControlConnectionItem[] = []
  const syntheticRejected: PathControlConnectionItem[] = []
  const liveRows: PathControlConnectionItem[] = []
  const syntheticIdlePinned: PathControlConnectionItem[] = []

  for (const row of rows) {
    if (!row._isSynthetic) {
      liveRows.push(row)
    } else if (row._controlStatus === 'disconnected') {
      syntheticDisconnected.push(row)
    } else if (row._controlStatus === 'reject') {
      syntheticRejected.push(row)
    } else {
      syntheticIdlePinned.push(row)
    }
  }

  const orderedLive = orderLiveFn
    ? (orderLiveFn([...liveRows]) as PathControlConnectionItem[])
    : liveRows

  return [
    ...syntheticDisconnected,
    ...syntheticRejected,
    ...orderedLive,
    ...syntheticIdlePinned,
  ]
}
