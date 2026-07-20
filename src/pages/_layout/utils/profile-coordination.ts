import { filterSort } from '@/components/proxy/use-filter-sort'
import type { HeadState } from '@/components/proxy/use-head-state'
import { calcuProxies, getProfiles, triggerAutoSelect } from '@/services/cmds'
import {
  getDelayManager,
  NODE_DELAY_MIN_MS,
  NODE_DELAY_MAX_MS,
} from '@/services/delay'
import { getProxyByNameWithTimeout } from '@/services/mihomo-api'
import { showNotice } from '@/services/notice-service'
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
    showNotice.info(
      t('shared.feedback.notifications.clashNotReady') ||
        'Clash 内核加载超时，自动选点可能不准确',
    )
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

    let filterText = ''
    let sortType = 0

    try {
      const headStateStr = localStorage.getItem('proxy-head-state')
      if (headStateStr && currentUid) {
        const headStateStorage = JSON.parse(headStateStr)
        const groupState = headStateStorage?.[currentUid]?.[groupName]
        if (groupState) {
          filterText = groupState.filterText || ''
          sortType = groupState.sortType || 0
        }
      }
    } catch {}

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

  for (const name of names) {
    getDelayManager().setDelay(name, groupName, -2)
  }
  getDelayManager().queueGroupNotification(groupName)

  try {
    await triggerAutoSelect(currentUid, undefined, 0, select)
  } catch (err) {
    console.error('[Layout] 后端批量测速/选点失败:', err)
  }
}

export async function triggerAutoSelectAndRefresh(
  refreshProxy: (opts?: { forceFull?: boolean }) => Promise<unknown>,
  fallbackTimerRef: { current: number | null },
  autoSelectTimerRef: { current: number | null },
  setHeadState?: (groupName: string, patch: Partial<HeadState>) => void,
  profileUid?: string,
): Promise<void> {
  try {
    await refreshProxy({ forceFull: true })
  } catch (err) {
    console.warn('[Layout] refreshProxy after profile change failed:', err)
  }

  if (setHeadState) {
    setHeadState('PROXY', { sortType: 1 })
  }

  if (autoSelectTimerRef.current !== null) {
    clearTimeout(autoSelectTimerRef.current)
    autoSelectTimerRef.current = null
  }

  autoSelectTimerRef.current = setTimeout(async () => {
    autoSelectTimerRef.current = null
    try {
      if (profileUid) {
        const currentUid = (await getProfiles())?.current || ''
        if (currentUid !== profileUid) {
          return
        }
      }
      const names = await getFilteredNodeNames()
      if (names.length === 0) return
      await batchTestWithFirstBatchSelect(names, true)
      if (setHeadState) {
        setHeadState('PROXY', { sortType: 1 })
      }
    } catch (err) {
      console.warn('[Layout] 延迟批量测速失败:', err)
    }
  }, 0)

  if (fallbackTimerRef.current) clearTimeout(fallbackTimerRef.current)
  fallbackTimerRef.current = setTimeout(async () => {
    const timerId = fallbackTimerRef.current
    try {
      if (profileUid) {
        const currentUid = (await getProfiles())?.current || ''
        if (currentUid !== profileUid) {
          return
        }
      }
      const proxyGroup = await getProxyByNameWithTimeout('PROXY')
      const nowNodeName = proxyGroup?.now || ''
      if (!nowNodeName) return
      const nowNode = await getProxyByNameWithTimeout(nowNodeName)
      const history = nowNode?.history || []
      const latestDelay =
        history.length > 0 ? history[history.length - 1].delay : -1
      const hasHealth =
        latestDelay >= NODE_DELAY_MIN_MS && latestDelay < NODE_DELAY_MAX_MS
      if (!hasHealth) {
        const names = await getFilteredNodeNames()
        if (names.length === 0) return
        await batchTestWithFirstBatchSelect(names, true)
        if (setHeadState) {
          setHeadState('PROXY', { sortType: 1 })
        }
      }
    } catch (fbErr) {
      console.error('[Layout] Fallback 逻辑异常:', fbErr)
    } finally {
      if (fallbackTimerRef.current === timerId) {
        fallbackTimerRef.current = null
      }
    }
  }, 10_000)
}
