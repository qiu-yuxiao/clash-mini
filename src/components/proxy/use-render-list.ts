import { useEffect, useMemo, useReducer, useRef } from 'react'

import { MINI_WIDTH_THRESHOLD } from '@/constants'
import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'
import delayManager, { NODE_DELAY_MAX_MS } from '@/services/delay'
import type { IProxyItem } from '@/types/clash'

import { filterSort } from './use-filter-sort'
import {
  DEFAULT_STATE,
  useHeadStateNew,
  type HeadState,
} from './use-head-state'
import { useWindowWidth } from './use-window-width'

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

// 优化列布局计算
const calculateColumns = (width: number): number => {
  if (width <= MINI_WIDTH_THRESHOLD) {
    return 1
  }
  return 3 // 依据 Agreement 第九条，宽屏下固定为 3 列排版
}

// 优化分组逻辑
const groupProxies = <T = any>(list: T[], size: number): T[][] => {
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
  const { width } = useWindowWidth()
  const [headStates, setHeadState] = useHeadStateNew()
  const latencyTimeout = NODE_DELAY_MAX_MS

  // 延迟更新计数器，每次组级通知递增，驱动 useMemo 重新计算排序
  const [delayBump, bumpDelay] = useReducer((c: number) => c + 1, 0)

  // 计算列数
  const col = useMemo(() => calculateColumns(width), [width])

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

  // 注册 PROXY 组监听器，单点测速完成后驱动列表重排
  // 注意：批量测速已通过 checkListDelay 内部 queueGroupNotification 触发，此监听器同时覆盖两类场景
  useEffect(() => {
    delayManager.setGroupListener('PROXY', bumpDelay)
    return () => {
      delayManager.removeGroupListener('PROXY', bumpDelay)
    }
  }, [bumpDelay])

  const groupCacheRef = useRef<Map<string, GroupCache>>(new Map())
  const prevListRef = useRef<IRenderItem[]>([])

  // 处理渲染列表
  const renderList: IRenderItem[] = useMemo(() => {
    if (!proxiesData) return []

    // 【核心架构约定 - 切勿误判为多组架构】
    // Clash Mini 把所有上游代理组的节点合并到唯一一个 PROXY 组中。
    // calcuProxies() 返回的 groups 数组里虽然可能包含 GLOBAL 等其他组（上游遗留的数据结构），
    // 但 Mini 的所有节点选择/恢复/切换/测速逻辑只针对 PROXY 组，不应遍历多组。
    // 如需修改此处，请先确认 Mini 单组架构约定（见 project_memory.md）。
    const renderGroups = proxiesData.groups || []

    const cache = groupCacheRef.current
    let anyChanged = false

    const retList = renderGroups.flatMap((group: ProxyGroup) => {
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

      anyChanged = true
      const ret: IRenderItem[] = []

      const proxies = filterSort(
        group.all,
        group.name,
        headState.filterText,
        headState.sortType,
        latencyTimeout,
      )

      ret.push({
        type: 1,
        key: `head-${group.name}`,
        group,
        headState,
      })

      if (!proxies.length) {
        ret.push({
          type: 3,
          key: `empty-${group.name}`,
          group,
          headState,
        })
      } else if (col > 1) {
        ret.push(
          ...groupProxies(proxies, col).map((proxyCol, colIndex) => ({
            type: 4 as const,
            key: `col-${group.name}-${proxyCol[0]?.name ?? colIndex}`,
            group,
            headState,
            col,
            proxyCol,
            provider: proxyCol[0]?.provider,
            indexInGroup: colIndex,
          })),
        )
      } else {
        ret.push(
          ...proxies.map((proxy, proxyIdx) => ({
            type: 2 as const,
            key: `${group.name}-${proxy?.name ?? proxyIdx}`,
            group,
            proxy,
            headState,
            provider: proxy.provider,
            indexInGroup: proxyIdx,
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
      return ret
    })

    // L-18: 清理不再存在的组的缓存，防止 profile 切换后内存泄漏
    const existingGroupNames = new Set(
      renderGroups.map((g: ProxyGroup) => g.name),
    )
    cache.forEach((_, key) => {
      if (!existingGroupNames.has(key)) {
        cache.delete(key)
      }
    })

    const filtered = retList.filter((item: IRenderItem) => !item.group?.hidden)

    if (!anyChanged && prevListRef.current.length === filtered.length) {
      return prevListRef.current
    }
    prevListRef.current = filtered
    return filtered
  }, [headStates, proxiesData, col, delayBump, latencyTimeout])

  return {
    renderList,
    onProxies: refreshProxy,
    onHeadState: setHeadState,
    currentColumns: col,
  }
}

// 优化建议：如有大数据量，建议用虚拟滚动（已在 ProxyGroups 组件中实现），此处无需额外处理。
