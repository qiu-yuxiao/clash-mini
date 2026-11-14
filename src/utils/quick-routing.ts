import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile, writeFile } from '@tauri-apps/plugin-fs'
import dayjs from 'dayjs'
import yaml from 'js-yaml'

import {
  beginLongOperation,
  endLongOperation,
  enhanceProfiles,
  readProfileFile,
  saveProfileFile,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

export type QuickRoutingTarget = 'DIRECT' | 'PROXY' | 'REJECT'
export type QuickRuleType =
  | 'DOMAIN'
  | 'DOMAIN-SUFFIX'
  | 'IP-CIDR'
  | 'IP-CIDR6'
  | 'PROCESS-NAME'

export interface ManagedMergeRule {
  raw: string
  ruleType: QuickRuleType
  /** Normalized pattern (lowercase domain/CIDR, or trimmed process name). */
  value: string
  /** Bare lookup key (strips trailing /32 or /128 on host IPs). */
  matchKey: string
  target: QuickRoutingTarget
  extra?: string
}

export const MERGE_RULES_CHANGED_EVENT = 'clash-mini-merge-rules-changed'

const SUPPORTED_RULE_TYPES = new Set<QuickRuleType>([
  'DOMAIN',
  'DOMAIN-SUFFIX',
  'IP-CIDR',
  'IP-CIDR6',
  'PROCESS-NAME',
])

const VALID_TARGETS = new Set<QuickRoutingTarget>(['DIRECT', 'PROXY', 'REJECT'])

let mergeFileLock = Promise.resolve()

async function withMergeFileLock<T>(fn: () => Promise<T>): Promise<T> {
  const result = mergeFileLock.then(() => fn())
  mergeFileLock = result.then(
    () => {},
    () => {},
  )
  return result
}

function readMergePrependRules(mergeObj: Record<string, unknown>): string[] {
  const raw = mergeObj['prepend-rules']
  if (!Array.isArray(raw)) return []
  return raw.filter((item): item is string => typeof item === 'string')
}

async function loadMergeObject(): Promise<Record<string, unknown>> {
  let mergeYaml: string
  try {
    mergeYaml = await readProfileFile('Merge')
  } catch (readErr) {
    console.warn(
      'Global Merge file not found or failed to read, initializing empty merge:',
      readErr,
    )
    mergeYaml = '{}'
  }
  const parsed = yaml.load(mergeYaml)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return {}
  }
  return parsed as Record<string, unknown>
}

function emitMergeRulesChanged(): void {
  if (
    typeof window !== 'undefined' &&
    typeof window.dispatchEvent === 'function'
  ) {
    window.dispatchEvent(new CustomEvent(MERGE_RULES_CHANGED_EVENT))
  }
}

export function isIPv4Address(value: string): boolean {
  const parts = value.split('.')
  if (parts.length !== 4) return false
  return parts.every((part) => {
    if (!/^\d{1,3}$/.test(part)) return false
    const num = Number(part)
    return num >= 0 && num <= 255
  })
}

export function isIPv6Address(value: string): boolean {
  if (!value || !value.includes(':') || value.includes('/')) return false
  // Require at least two colons (valid IPv6 literal) and hex/colon/dot chars only.
  const colonCount = value.split(':').length - 1
  if (colonCount < 2) return false
  return /^[0-9a-fA-F:.]+$/.test(value)
}

/**
 * Normalizes a host or IP string from Path Control without altering subdomains.
 * Strips IPv6 brackets, zone indices, optional single `:port`, leading `*.`/`.`, and trailing `.`.
 */
export function normalizeQuickTargetHost(raw: string): string {
  let host = raw.trim()
  if (!host) return ''

  // Strip surrounding IPv6 brackets and optional trailing `:port`, e.g. `[2001:db8::1]:443`.
  if (host.startsWith('[')) {
    const closeIdx = host.indexOf(']')
    if (closeIdx > 1) {
      host = host.slice(1, closeIdx)
    }
  } else {
    const firstColon = host.indexOf(':')
    const lastColon = host.lastIndexOf(':')
    // Only strip `:port` when there is a single colon followed strictly by digits.
    if (firstColon !== -1 && firstColon === lastColon) {
      const maybePort = host.slice(firstColon + 1)
      if (/^\d+$/.test(maybePort)) {
        host = host.slice(0, firstColon)
      }
    }
  }

  // Strip IPv6 zone suffix (e.g. `fe80::1%12` or `fe80::1%eth0`).
  const zoneIdx = host.indexOf('%')
  if (zoneIdx !== -1) {
    host = host.slice(0, zoneIdx)
  }

  host = host
    .replace(/^(\*\.|\.)+/, '')
    .replace(/\.+$/, '')
    .trim()
  return host.toLowerCase()
}

/**
 * Builds an exact Clash routing rule without widening scope or stripping subdomains.
 */
export function buildExactQuickRoutingRule(
  type: 'process' | 'domain',
  value: string,
  target: QuickRoutingTarget,
): string | null {
  if (type === 'process') {
    const proc = value.trim()
    return proc ? `PROCESS-NAME,${proc},${target}` : null
  }

  const host = normalizeQuickTargetHost(value)
  if (!host) return null

  if (isIPv4Address(host)) {
    return `IP-CIDR,${host}/32,${target}`
  }
  if (isIPv6Address(host)) {
    return `IP-CIDR6,${host}/128,${target}`
  }
  return `DOMAIN,${host},${target}`
}

/**
 * Builds a PROXY rule from user URL/domain/IP input.
 * Plain domains emit exact `DOMAIN`; explicit `*.domain` or `.domain` emits `DOMAIN-SUFFIX`.
 */
export function buildProxyRule(input: string): string | null {
  if (!input) return null

  let raw = input.trim()
  if (!raw) return null

  raw = raw.replace(/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//, '')

  // Detect explicit IPv4/IPv6 CIDR before path stripping removes `/prefix`.
  const cidrCandidate = raw.split('?')[0].split('#')[0].trim()
  const cidrParts = cidrCandidate.split('/')
  if (cidrParts.length === 2 && /^\d{1,3}$/.test(cidrParts[1])) {
    const prefix = Number(cidrParts[1])
    const baseIp = normalizeQuickTargetHost(cidrParts[0])
    if (isIPv4Address(baseIp) && prefix >= 0 && prefix <= 32) {
      return `IP-CIDR,${baseIp}/${prefix},PROXY`
    }
    if (isIPv6Address(baseIp) && prefix >= 0 && prefix <= 128) {
      return `IP-CIDR6,${baseIp}/${prefix},PROXY`
    }
  }

  // Strip path, query, hash while preserving IPv6 literals and optional `[ipv6]:port`.
  const hostPortRaw = raw.split('/')[0].split('?')[0].split('#')[0].trim()
  if (!hostPortRaw) return null

  const explicitWildcard =
    hostPortRaw.startsWith('*.') || hostPortRaw.startsWith('.')
  const cleaned = normalizeQuickTargetHost(hostPortRaw)
  if (!cleaned) return null

  if (isIPv4Address(cleaned)) {
    return `IP-CIDR,${cleaned}/32,PROXY`
  }
  if (isIPv6Address(cleaned)) {
    return `IP-CIDR6,${cleaned}/128,PROXY`
  }

  if (!cleaned.includes('.')) return null

  return explicitWildcard
    ? `DOMAIN-SUFFIX,${cleaned},PROXY`
    : `DOMAIN,${cleaned},PROXY`
}

/**
 * Parses a single rule line into a `ManagedMergeRule` if it is a managed Path Control rule.
 */
export function parseManagedMergeRuleLine(
  line: string,
): ManagedMergeRule | null {
  if (typeof line !== 'string') return null
  const trimmed = line.trim()
  if (!trimmed) return null

  const parts = trimmed.split(',').map((part) => part.trim())
  if (parts.length < 3) return null

  const ruleType = parts[0].toUpperCase() as QuickRuleType
  if (!SUPPORTED_RULE_TYPES.has(ruleType)) return null

  const target = parts[2].toUpperCase() as QuickRoutingTarget
  if (!VALID_TARGETS.has(target)) return null

  const rawPattern = parts[1]
  if (!rawPattern) return null

  const value =
    ruleType === 'PROCESS-NAME' ? rawPattern : rawPattern.toLowerCase()
  const matchKey =
    ruleType === 'IP-CIDR' && value.endsWith('/32')
      ? value.slice(0, -3)
      : ruleType === 'IP-CIDR6' && value.endsWith('/128')
        ? value.slice(0, -4)
        : ruleType === 'PROCESS-NAME'
          ? value.toLowerCase()
          : value

  const extra = parts.length > 3 ? parts.slice(3).join(',') : undefined

  return {
    raw: trimmed,
    ruleType,
    value,
    matchKey,
    target,
    extra,
  }
}

export function getRuleIdentityKey(ruleStr: string): string | null {
  const parsed = parseManagedMergeRuleLine(ruleStr)
  if (!parsed) return null
  const normalizedVal =
    parsed.ruleType === 'PROCESS-NAME'
      ? parsed.value.toLowerCase()
      : parsed.value
  return `${parsed.ruleType}:${normalizedVal}`
}

export function parseManagedMergeRulesFromYaml(
  mergeYaml: string,
): ManagedMergeRule[] {
  if (!mergeYaml || !mergeYaml.trim()) return []
  try {
    const doc = yaml.load(mergeYaml)
    if (!doc || typeof doc !== 'object' || Array.isArray(doc)) return []
    const lines = readMergePrependRules(doc as Record<string, unknown>)
    return lines
      .map((line) => parseManagedMergeRuleLine(line))
      .filter((item): item is ManagedMergeRule => item !== null)
  } catch (err) {
    console.warn('Failed to parse Merge prepend-rules:', err)
    return []
  }
}

export function upsertPrependRule(
  existing: string[],
  newRule: string,
  options?: { replaceRawRules?: string[] },
): string[] {
  const newKey = getRuleIdentityKey(newRule)
  const replaceSet = new Set(
    (options?.replaceRawRules ?? []).map((r) => r.trim()).filter(Boolean),
  )

  const filtered = existing.filter((rule) => {
    const trimmed = typeof rule === 'string' ? rule.trim() : ''
    if (!trimmed || trimmed === newRule) return false
    if (replaceSet.has(trimmed)) return false
    if (newKey && getRuleIdentityKey(trimmed) === newKey) return false
    return true
  })

  return [newRule, ...filtered]
}

export function removePrependRules(
  existing: string[],
  matchers: { rawRule?: string; identityKeys?: string[] },
): { nextRules: string[]; removed: string[] } {
  const targetRaw = matchers.rawRule?.trim()
  const keySet = new Set((matchers.identityKeys ?? []).filter(Boolean))

  const nextRules: string[] = []
  const removed: string[] = []

  for (const rule of existing) {
    const trimmed = typeof rule === 'string' ? rule.trim() : ''
    const ruleKey = trimmed ? getRuleIdentityKey(trimmed) : null
    const matchesRaw = Boolean(targetRaw && trimmed === targetRaw)
    const matchesKey = Boolean(ruleKey && keySet.has(ruleKey))
    if (matchesRaw || matchesKey) {
      removed.push(trimmed || rule)
    } else {
      nextRules.push(rule)
    }
  }

  return { nextRules, removed }
}

/**
 * Computes all candidate exact `getRuleIdentityKey` strings for a Path Control target.
 */
export function buildTargetIdentityKeys(
  type: 'process' | 'domain',
  value: string,
): string[] {
  const exactRule = buildExactQuickRoutingRule(type, value, 'DIRECT')
  if (!exactRule) return []
  const exactKey = getRuleIdentityKey(exactRule)
  return exactKey ? [exactKey] : []
}

export const readManagedMergeRules = async (): Promise<ManagedMergeRule[]> => {
  try {
    const mergeYaml = await readProfileFile('Merge')
    return parseManagedMergeRulesFromYaml(mergeYaml)
  } catch (err) {
    console.warn('Failed to read Merge profile rules:', err)
    return []
  }
}

/**
 * Adds a pinned exact rule to global Merge `prepend-rules`, reloads profiles,
 * and broadcasts the Merge rule change event.
 */
export const addQuickRoutingRule = async (
  type: 'process' | 'domain',
  value: string,
  target: QuickRoutingTarget,
  options?: { replaceRawRules?: string[] },
): Promise<string | null> => {
  const newRule = buildExactQuickRoutingRule(type, value, target)
  if (!newRule) return null

  try {
    await withMergeFileLock(async () => {
      const mergeObj = await loadMergeObject()
      const existing = readMergePrependRules(mergeObj)
      mergeObj['prepend-rules'] = upsertPrependRule(existing, newRule, options)
      await saveProfileFile('Merge', yaml.dump(mergeObj))
    })

    await enhanceProfiles()
    emitMergeRulesChanged()

    showNotice.success(
      'profiles.page.feedback.notifications.quickRuleAdded',
      { rule: newRule },
      2500,
    )
    return newRule
  } catch (e: unknown) {
    console.error('Failed to add quick routing rule:', e)
    showNotice.error(e instanceof Error ? e.message : String(e))
    return null
  }
}

/**
 * Removes a pinned rule from global Merge `prepend-rules`, reloads profiles,
 * and broadcasts the Merge rule change event.
 */
export const removeQuickRoutingRule = async (options: {
  rawRule?: string
  targetType?: 'process' | 'domain'
  targetValue?: string
}): Promise<string | null> => {
  const identityKeys =
    options.targetType && options.targetValue
      ? buildTargetIdentityKeys(options.targetType, options.targetValue)
      : []
  if (!options.rawRule && identityKeys.length === 0) return null

  try {
    let removedList: string[] = []
    await withMergeFileLock(async () => {
      const mergeObj = await loadMergeObject()
      const existing = readMergePrependRules(mergeObj)
      if (existing.length === 0) return

      const { nextRules, removed } = removePrependRules(existing, {
        rawRule: options.rawRule,
        identityKeys,
      })
      if (removed.length === 0) return

      removedList = removed
      mergeObj['prepend-rules'] = nextRules
      await saveProfileFile('Merge', yaml.dump(mergeObj))
    })

    if (removedList.length === 0) return null

    await enhanceProfiles()
    emitMergeRulesChanged()

    showNotice.success(
      'profiles.page.feedback.notifications.quickRuleRemoved',
      { rule: removedList[0] },
      2500,
    )
    return removedList[0]
  } catch (e: unknown) {
    console.error('Failed to remove quick routing rule:', e)
    showNotice.error(e instanceof Error ? e.message : String(e))
    return null
  }
}

/**
 * Parses multi-line/comma/semicolon user input into Clash PROXY rules and prepends to Merge.
 */
export const addQuickRoutingRules = async (
  rawText: string,
): Promise<number> => {
  const lines = (rawText || '')
    .split(/[\n,;]+/)
    .map((l) => l.trim())
    .filter(Boolean)

  if (lines.length === 0) return 0

  try {
    const rules = lines
      .map((line) => buildProxyRule(line))
      .filter((rule): rule is string => !!rule)

    if (rules.length === 0) return 0

    let addedCount = 0
    await withMergeFileLock(async () => {
      const mergeObj = await loadMergeObject()
      let current = readMergePrependRules(mergeObj)

      const added: string[] = []
      const seenKeys = new Set<string>()
      for (const rule of rules) {
        const key = getRuleIdentityKey(rule)
        if (!current.includes(rule) && (!key || !seenKeys.has(key))) {
          if (key) seenKeys.add(key)
          added.push(rule)
        }
      }

      if (added.length === 0) return

      for (const rule of [...added].reverse()) {
        current = upsertPrependRule(current, rule)
      }

      addedCount = added.length
      mergeObj['prepend-rules'] = current
      await saveProfileFile('Merge', yaml.dump(mergeObj))
    })

    if (addedCount === 0) return 0

    await enhanceProfiles()
    emitMergeRulesChanged()

    return addedCount
  } catch (e: unknown) {
    console.error('Failed to add bulk routing rules:', e)
    showNotice.error(e instanceof Error ? e.message : String(e))
    return 0
  }
}

interface CustomRoutingBackupPayload {
  version?: number
  exportedAt?: string
  'prepend-rules'?: unknown
  rules?: unknown
}

/**
 * Serializes managed Merge `prepend-rules` into a portable YAML backup string.
 */
export function serializeCustomRoutingBackup(
  rules: ReadonlyArray<string | { raw: string }>,
): string {
  const ruleLines: string[] = []
  const seenKeys = new Set<string>()

  for (const item of rules) {
    const raw = (typeof item === 'string' ? item : item.raw).trim()
    if (!raw || raw.startsWith('#')) continue
    const parsed = parseManagedMergeRuleLine(raw)
    if (!parsed) continue
    const dedupKey = getRuleIdentityKey(parsed.raw) ?? parsed.raw
    if (seenKeys.has(dedupKey)) continue
    seenKeys.add(dedupKey)
    ruleLines.push(parsed.raw)
  }

  const payload: CustomRoutingBackupPayload = {
    version: 1,
    exportedAt: new Date().toISOString(),
    'prepend-rules': ruleLines,
  }

  return `# Clash Mini Custom Routing Rules Backup\n${yaml.dump(payload, {
    noRefs: true,
    lineWidth: -1,
  })}`
}

/**
 * Parses a custom routing backup YAML (or plain rule list) and validates managed rules.
 */
export function parseCustomRoutingBackup(content: string): string[] {
  const trimmed = content.trim()
  if (!trimmed) return []

  let candidates: unknown[] = []
  try {
    const parsed = yaml.load(trimmed)
    if (Array.isArray(parsed)) {
      candidates = parsed
    } else if (parsed && typeof parsed === 'object') {
      const obj = parsed as CustomRoutingBackupPayload
      if (Array.isArray(obj['prepend-rules'])) {
        candidates = obj['prepend-rules']
      } else if (Array.isArray(obj.rules)) {
        candidates = obj.rules
      }
    }
  } catch {
    return []
  }

  const validRules: string[] = []
  const seenKeys = new Set<string>()
  for (const candidate of candidates) {
    if (typeof candidate !== 'string') continue
    const line = candidate.trim()
    if (!line || line.startsWith('#')) continue
    const parsedRule = parseManagedMergeRuleLine(line)
    if (!parsedRule) continue
    const dedupKey = getRuleIdentityKey(parsedRule.raw) ?? parsedRule.raw
    if (seenKeys.has(dedupKey)) continue
    seenKeys.add(dedupKey)
    validRules.push(parsedRule.raw)
  }

  return validRules
}

/**
 * Exports all custom Path Control routing rules from the Merge profile to a user-chosen YAML file.
 * Returns the exported rule count, or null if cancelled / failed.
 */
export async function exportCustomRoutingRules(): Promise<number | null> {
  try {
    const managedRules = await withMergeFileLock(() => readManagedMergeRules())
    if (managedRules.length === 0) {
      showNotice.info('settings.mini.routesExportEmpty')
      return 0
    }

    const filePath = await save({
      defaultPath: `clash-mini-routes-${dayjs().format('YYYY-MM-DD')}.yaml`,
      filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
    })
    if (!filePath) return null

    const content = serializeCustomRoutingBackup(managedRules)
    await writeFile(filePath, new TextEncoder().encode(content))

    showNotice.success('settings.mini.routesExportSuccess', {
      count: managedRules.length,
    })
    return managedRules.length
  } catch (e: unknown) {
    console.error('Failed to export custom routing rules:', e)
    showNotice.error(e instanceof Error ? e.message : String(e))
    return null
  }
}

/**
 * Imports custom Path Control routing rules from a user-selected backup file into the Merge profile.
 * Returns the number of imported rules, or null if cancelled / failed.
 */
export async function importCustomRoutingRules(): Promise<number | null> {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
    })
    const filePath = Array.isArray(selected) ? selected[0] : selected
    if (!filePath) return null

    const bytes = await readFile(filePath)
    const content = new TextDecoder('utf-8').decode(bytes)
    const importedRules = parseCustomRoutingBackup(content)

    if (importedRules.length === 0) {
      showNotice.info('settings.mini.routesImportEmpty')
      return 0
    }

    await withMergeFileLock(async () => {
      const mergeObj = await loadMergeObject()
      let nextPrependRules = readMergePrependRules(mergeObj)
      for (let i = importedRules.length - 1; i >= 0; i -= 1) {
        nextPrependRules = upsertPrependRule(nextPrependRules, importedRules[i])
      }
      mergeObj['prepend-rules'] = nextPrependRules
      await saveProfileFile(
        'Merge',
        yaml.dump(mergeObj, { noRefs: true, lineWidth: -1 }),
      )
    })

    beginLongOperation()
    try {
      await enhanceProfiles()
    } finally {
      endLongOperation()
    }
    emitMergeRulesChanged()

    showNotice.success('settings.mini.routesImportSuccess', {
      count: importedRules.length,
    })
    return importedRules.length
  } catch (e: unknown) {
    console.error('Failed to import custom routing rules:', e)
    showNotice.error(e instanceof Error ? e.message : String(e))
    return null
  }
}
