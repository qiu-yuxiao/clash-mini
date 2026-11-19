import { filterSort } from '@/components/proxy/use-filter-sort'
import {
  getGroupHeadState,
  type HeadState,
} from '@/components/proxy/use-head-state'
import { calcuProxies, getProfiles, triggerAutoSelect } from '@/services/cmds'
import {
  getDelayManager,
  HISTORY_FALLBACK_MAX_AGE_MS,
  NODE_DELAY_MIN_MS,
  NODE_DELAY_MAX_MS,
} from '@/services/delay'
import { getProxyByNameWithTimeout } from '@/services/mihomo-api'
import { showNotice } from '@/services/notice-service'
import type { IProxyItem } from '@/types/clash'
import { isDummyNode } from '@/utils/node'

type TFunc = (key: string, options?: Record<string, unknown>) => string

export async function waitForClashReady(t: TFunc): Promise<boolean> {
  const MAX_WAIT_MS = 10_000
  const POLL_INTERVAL_MS = 500
  const startedAt = Date.now()

  while (Date.now() - startedAt < MAX_WAIT_MS) {
    try {
      const proxyGroup = await getProxyByNameWithTimeout('PROXY')
      const hasRealNodes = (proxyGroup?.all || []).some(
        (name: string) => !isDummyNode(name),
      )
      if (hasRealNodes) {
        return true
      }
    } catch {
      // 内核还在加载中，继续等待
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS))
  }

  console.warn(
    `[waitForClashReady] 等待 Clash 内核就绪超时（${MAX_WAIT_MS}ms），继续触发自动选点`,
  )
  try {
    showNotice.info(t('shared.feedback.notifications.clashNotReady'))
  } catch {
    // showNotice 不可用则静默失败
  }
  return false
}

export async function getFilteredNodeNames(): Promise<string[]> {
  const groupName = 'PROXY'
  try {
    const allData = await calcuProxies()
    const group = allData.groups?.[0]
    const allProxies = group?.all || []

    let currentUid = ''
    try {
      const profiles = await getProfiles()
      currentUid = profiles?.current || ''
    } catch {}

    const groupState = currentUid
      ? getGroupHeadState(currentUid, groupName)
      : undefined
    const filterText = groupState?.filterText || ''
    const sortType = groupState?.sortType ?? 0

    const filtered = filterSort(
      allProxies,
      groupName,
      filterText,
      sortType as 0 | 1 | 2,
    )

    return filtered
      .map((p) => p.name)
      .filter((name) => name && !isDummyNode(name))
  } catch (err) {
    console.warn('[Layout] 获取过滤节点列表失败，回退到全部节点:', err)
    try {
      const proxyGroup = await getProxyByNameWithTimeout(groupName)
      return (proxyGroup?.all || []).filter(
        (name: string) => !isDummyNode(name),
      )
    } catch {
      return []
    }
  }
}

export async function batchTestWithFirstBatchSelect(
  names: string[],
  select = true,
): Promise<void> {
  const groupName = 'PROXY'
  if (names.length === 0) return

  const currentUid = (await getProfiles())?.current || ''
  if (!currentUid) return

  const rollbackPendingNodes = getDelayManager().beginBatchCheck(
    names,
    groupName,
  )

  try {
    // 严格传入过滤子集 names，避免越过当前筛选条件选到子集外节点
    const results = await triggerAutoSelect(currentUid, names, 0, select)
    if (results.length === 0) {
      rollbackPendingNodes()
    }
  } catch (err) {
    console.error('[Layout] 后端批量测速/选点失败:', err)
    rollbackPendingNodes()
  }
}

/**
 * 最高方针「连续性优先」（agreements §5 / Sync-22）：判断当前 `PROXY.now` 是否仍是
 * 结构性有效的用户持久出口，供启动 / 唤醒 / Profile 切换的初始化自动选点决定是否允许切点。
 *
 * - 有效（非空、非伪节点、仍在当前过滤子集内）→ 只做全组测速刷新，**不换节点**；
 * - 失效（空 / 伪节点 / 不在新配置与过滤范围内）→ 才允许重选，保证有可用出口。
 *
 * 刻意不检查 alive / history：冷启动时内核尚未拨测，这些数据不可靠；「没有健康延迟」
 * 不等于「节点已死」，不构成切换理由。真死节点由后台监测经连续 2 次确认 + 死节点门检自愈，
 * 不在此抢占。读取组信息失败一律返回 false（保守允许选点），避免退化成无出口。
 */
export async function hasUsablePersistedExit(
  names: string[],
): Promise<boolean> {
  try {
    const group = await getProxyByNameWithTimeout('PROXY')
    const now = group?.now || ''
    if (!now || isDummyNode(now)) return false
    return names.includes(now)
  } catch {
    return false
  }
}

export function isProxyHistoryFreshAndValid(
  proxy: Pick<IProxyItem, 'alive' | 'history'> | null | undefined,
  nowMs = Date.now(),
): boolean {
  if (proxy?.alive !== true) return false
  const history = proxy.history
  if (!Array.isArray(history) || history.length === 0) return false
  const lastRecord = history[history.length - 1]
  const delay = lastRecord?.delay
  if (
    typeof delay !== 'number' ||
    delay < NODE_DELAY_MIN_MS ||
    delay >= NODE_DELAY_MAX_MS
  ) {
    return false
  }
  const recordedAt = Date.parse(lastRecord.time)
  return (
    !Number.isNaN(recordedAt) &&
    nowMs - recordedAt <= HISTORY_FALLBACK_MAX_AGE_MS
  )
}

/**
 * 判断当前 `PROXY.now` 节点是否在当前过滤子集 `names` 内且已持有有效未过期延迟
 * （优先读 `DelayManager` 有效延迟缓存，再回退内核最新 `alive + history`），
 * 供轻量唤醒及窗口可见恢复跳过不必要的全组探针（BUG-402）。
 */
export async function hasFreshActiveExitLatency(
  names: string[],
): Promise<boolean> {
  if (names.length === 0) return false
  try {
    const allData = await calcuProxies()
    const now = allData.groups?.[0]?.now || ''
    if (!now || isDummyNode(now) || !names.includes(now)) return false

    const cached = getDelayManager().getDelayUpdate(now, 'PROXY')
    if (
      cached &&
      cached.delay >= NODE_DELAY_MIN_MS &&
      cached.delay < NODE_DELAY_MAX_MS
    ) {
      return true
    }

    return isProxyHistoryFreshAndValid(allData.records?.[now])
  } catch {
    return false
  }
}

export async function triggerAutoSelectAndRefresh(
  refreshProxy: (opts?: { forceFull?: boolean }) => Promise<unknown>,
  setHeadState?: (groupName: string, patch: Partial<HeadState>) => void,
  profileUid?: string,
  options?: { skipProbeIfFresh?: boolean },
): Promise<void> {
  try {
    await refreshProxy({ forceFull: true })
  } catch (err) {
    console.warn('[Layout] refreshProxy after profile change failed:', err)
  }

  if (setHeadState) {
    setHeadState('PROXY', { sortType: 1 })
  }

  // 让出主线程一帧以先渲染当前界面与排序状态
  await new Promise((resolve) => setTimeout(resolve, 0))

  try {
    if (profileUid) {
      const currentUid = (await getProfiles())?.current || ''
      if (currentUid !== profileUid) {
        return
      }
    }
    const names = await getFilteredNodeNames()
    if (names.length === 0) return
    // 连续性优先：持久出口有效时仅刷新延迟不切换；开启 skipProbeIfFresh 且延迟新鲜时跳过拨测
    const keepExit = await hasUsablePersistedExit(names)
    if (
      options?.skipProbeIfFresh &&
      keepExit &&
      (await hasFreshActiveExitLatency(names))
    ) {
      return
    }
    await batchTestWithFirstBatchSelect(names, !keepExit)
    if (setHeadState) {
      setHeadState('PROXY', { sortType: 1 })
    }
  } catch (err) {
    console.warn('[Layout] 批量测速失败:', err)
  }
}
