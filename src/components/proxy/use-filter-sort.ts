import { getDelayManager, NODE_DELAY_MAX_MS } from '@/services/delay'
import type { IProxyItem } from '@/types/clash'

// default | delay | alphabet
export type ProxySortType = 0 | 1 | 2

export function filterSort(
  proxies: IProxyItem[],
  groupName: string,
  filterText: string,
  sortType: ProxySortType,
  latencyTimeout?: number,
) {
  const fp = filterProxies(proxies, groupName, filterText)
  const sp = sortProxies(fp, groupName, sortType, latencyTimeout)
  return sp
}

/**
 * filter the proxy
 * according to simple lowercase includes search with trim
 */
function filterProxies(
  proxies: IProxyItem[],
  _groupName: string,
  filterText: string,
) {
  if (!proxies) return []
  const query = (filterText || '').trim()
  if (!query) return proxies

  const target = query.toLowerCase()
  return proxies.filter((p) => (p?.name ?? '').toLowerCase().includes(target))
}

/**
 * sort the proxy
 */
function sortProxies(
  proxies: IProxyItem[],
  groupName: string,
  sortType: ProxySortType,
  latencyTimeout?: number,
) {
  if (!proxies) return []
  if (sortType === 0) return proxies

  const list = proxies.slice()
  const effectiveTimeout =
    typeof latencyTimeout === 'number' && latencyTimeout > 0
      ? latencyTimeout
      : NODE_DELAY_MAX_MS

  if (sortType === 1) {
    const categorizeDelay = (delay: number): [number, number] => {
      if (!Number.isFinite(delay)) return [3, Number.MAX_SAFE_INTEGER]
      if (delay > 1e5) return [4, delay]
      if (delay === 0 || (delay >= effectiveTimeout && delay <= 1e5)) {
        return [3, delay || effectiveTimeout]
      }
      if (delay < 0) {
        // sentinel delays (-1, -2 测试中) 降级为 timeout 同级，防止因临时标记导致排序错位
        return [3, Number.MAX_SAFE_INTEGER]
      }
      return [0, delay]
    }

    list.sort((a, b) => {
      const ad = getDelayManager().getDelayFix(a, groupName)
      const bd = getDelayManager().getDelayFix(b, groupName)
      const [ar, av] = categorizeDelay(ad)
      const [br, bv] = categorizeDelay(bd)

      if (ar !== br) return ar - br
      return av - bv
    })
  } else {
    list.sort((a, b) => (a?.name ?? '').localeCompare(b?.name ?? ''))
  }

  return list
}
