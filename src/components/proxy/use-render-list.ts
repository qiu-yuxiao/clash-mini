import { useCallback, useEffect, useMemo, useReducer, useRef } from 'react'

import { useWindowDecorations } from '@/hooks/use-window'
import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'
import delayManager, { NODE_DELAY_MAX_MS } from '@/services/delay'
import type { IProxyItem } from '@/types/clash'

import { filterSort } from './use-filter-sort'
import {
  DEFAULT_STATE,
  useHeadStateNew,
  type HeadState,
} from './use-head-state'

// 代理组类型
type ProxyGroup = {
  name: string
  type: string
  udp: boolean
  xudp: boolean
  tfo: boolean
  mptcp: boolean
  smux: boolean
  history: {
    time: string
    delay: number
  }[]
  now?: string
  all: IProxyItem[]
  hidden?: boolean
  icon?: string
  testUrl?: string
  provider?: string
}

export interface IRenderItem {
  // 组 | head | item | empty | item col
  type: 0 | 1 | 2 | 3 | 4
  key: string
  group: ProxyGroup
  proxy?: IProxyItem
  col?: number
  proxyCol?: IProxyItem[]
  headState?: HeadState
  // 新增支持图标和其他元数据
  icon?: string
  provider?: string
  testUrl?: string
  indexInGroup?: number
}

type GroupCache = {
  now?: string
  all: IProxyItem[]
  headState: HeadState
  col: number
  latencyTimeout: number | undefined
  delayBump: number
  items: IRenderItem[]
  groupRef: ProxyGroup
}

// 优化分组逻辑
const groupProxies = <T = unknown>(list: T[], size: number): T[][] => {
  return list.reduce((acc, item) => {
    const lastGroup = acc[acc.length - 1]
    if (!lastGroup || lastGroup.length >= size) {
      acc.push([item])
    } else {
      lastGroup.push(item)
    }
    return acc
  }, [] as T[][])
}

export const useRenderList = (mode: string) => {
  // 使用全局数据提供者
  const { proxies: proxiesData } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const { isMinimalWidth } = useWindowDecorations()
  const [headStates, setHeadState] = useHeadStateNew()
  const latencyTimeout = NODE_DELAY_MAX_MS
  const bumpTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // 延迟更新计数器，每次组级通知递增，驱动 useMemo 重新计算排序
  const [delayBump, bumpDelay] = useReducer((c: number) => c + 1, 0)

  // 计算列数：窄窗口（≤285）单列，宽/大尺寸模式固定 3 列
  // 复用全 app 唯一的窗口尺寸状态源（window-provider），避免与浏览器 resize 链路脱钩
  const col = useMemo(() => (isMinimalWidth ? 1 : 3), [isMinimalWidth])

  // 确保代理数据加载
  // L-19: 限制 refreshProxy 最大重试次数（10次），防止无限轮询
  const refreshRetryCountRef = useRef(0)
  const MAX_REFRESH_RETRIES = 10
  useEffect(() => {
    if (!proxiesData) return
    const { groups, proxies } = proxiesData

    if (
      (mode === 'rule' && !groups?.length) ||
      (mode === 'global' && (proxies?.length ?? 0) < 2)
    ) {
      if (refreshRetryCountRef.current >= MAX_REFRESH_RETRIES) {
        console.warn('[useRenderList] 达到最大重试次数，停止自动刷新')
        return
      }
      refreshRetryCountRef.current += 1
      const handle = setTimeout(() => refreshProxy(), 500)
      return () => clearTimeout(handle)
    } else {
      // 数据加载成功，重置重试计数
      refreshRetryCountRef.current = 0
    }
  }, [proxiesData, mode, refreshProxy])

  // 注册 PROXY 组监听器：测速/延迟结果回写会高频触发组级通知，
  // 合并到 120ms 窗口内只递增一次 delayBump，避免主线程被连续 renderList 重算打满
  //（与窗口 resize / 开抽屉叠加时表现为卡死）。
  const scheduleBump = useCallback(() => {
    if (bumpTimerRef.current) return
    bumpTimerRef.current = setTimeout(() => {
      bumpTimerRef.current = null
      bumpDelay()
    }, 120)
  }, [bumpDelay])

  useEffect(() => {
    delayManager.setGroupListener('PROXY', scheduleBump)
    return () => {
      delayManager.removeGroupListener('PROXY', scheduleBump)
      if (bumpTimerRef.current) {
        clearTimeout(bumpTimerRef.current)
        bumpTimerRef.current = null
      }
    }
  }, [scheduleBump])

  const groupCacheRef = useRef<Map<string, GroupCache>>(new Map())

  // 处理渲染列表
  const renderList: IRenderItem[] = useMemo(() => {
    if (!proxiesData) return []

    // 【核心架构约定 - 切勿误判为多组架构】
    // Clash Mini 把所有上游代理组的节点合并到唯一一个 PROXY 组中。
    // calcuProxies() 返回的 groups 数组仅含 PROXY 一个元素，直接取 groups[0] 即可。
    // Mini 的所有节点选择/恢复/切换/测速逻辑只针对 PROXY 组，不应遍历多组。
    // 如需修改此处，请先确认 Mini 单组架构约定（见 project_memory.md）。
    const group = proxiesData.groups?.[0]
    if (!group) return []

    const cache = groupCacheRef.current

    const headState = headStates[group.name] || DEFAULT_STATE
    const cached = cache.get(group.name)

    if (
      cached &&
      cached.now === group.now &&
      cached.all === group.all &&
      cached.headState === headState &&
      cached.col === col &&
      cached.latencyTimeout === latencyTimeout &&
      cached.delayBump === delayBump &&
      cached.groupRef === group
    ) {
      return cached.items
    }

    const ret: IRenderItem[] = []

    const proxies = filterSort(
      group.all,
      group.name,
      headState.filterText,
      headState.sortType,
      latencyTimeout,
    )

    if (!proxies.length) {
      ret.push({
        type: 3,
        key: `empty-${group.name}`,
        group,
        headState,
      })
    } else {
      // 统一为分组结构（type:4）：列数 col 仅决定「每行几个节点」，
      // 行 key 用 colIndex（不依赖具体节点名）。窗口在 285 宽上下切换使 col 1↔3 时，
      // 同行索引的 ProxyItem 实例可被 React 复用，不再像过去
      // 「type:2 平铺 ↔ type:4 分组」结构切换那样整批卸载+重挂载
      //（群发测速时拖拽窗口卡死的根因）。
      ret.push(
        ...groupProxies(proxies, col).map((proxyCol, colIndex) => ({
          type: 4 as const,
          key: `col-${group.name}-${colIndex}`,
          group,
          headState,
          col,
          proxyCol,
          provider: proxyCol[0]?.provider,
          indexInGroup: colIndex,
        })),
      )
    }

    cache.set(group.name, {
      now: group.now,
      all: group.all,
      headState,
      col,
      latencyTimeout,
      delayBump,
      items: ret,
      groupRef: group,
    })

    const filtered = ret.filter((item: IRenderItem) => !item.group?.hidden)

    return filtered
  }, [headStates, proxiesData, col, delayBump, latencyTimeout])

  return {
    renderList,
    onHeadState: setHeadState,
    headStates,
  }
}

// 优化建议：如有大数据量，建议用虚拟滚动（已在 ProxyGroups 组件中实现），此处无需额外处理。
