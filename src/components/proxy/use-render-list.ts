import { useEffect, useMemo, useReducer, useRef } from 'react'

import { MINI_WIDTH_THRESHOLD } from '@/constants'
import { useRuntimeConfig } from '@/hooks/use-clash'
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

export const useRenderList = (
  mode: string,
  isChainMode?: boolean,
  selectedGroup?: string | null,
) => {
  // 使用全局数据提供者
  const { proxies: proxiesData } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const { width } = useWindowWidth()
  const [headStates, setHeadState] = useHeadStateNew()
  const latencyTimeout = NODE_DELAY_MAX_MS

  // 延迟更新计数器，每次组级通知递增，驱动 useMemo 重新计算排序
  const [delayBump, bumpDelay] = useReducer((c: number) => c + 1, 0)

  // 获取运行时配置用于链式代理模式
  const { data: runtimeConfig } = useRuntimeConfig(!!isChainMode)

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

  // 非链式模式下注册 PROXY 组监听器，单点测速完成后驱动列表重排
  // 注意：批量测速已通过 checkListDelay 内部 queueGroupNotification 触发，此监听器同时覆盖两类场景
  useEffect(() => {
    if (isChainMode) return
    delayManager.setGroupListener('PROXY', bumpDelay)
    return () => {
      delayManager.removeGroupListener('PROXY', bumpDelay)
    }
  }, [isChainMode, bumpDelay])

  const groupCacheRef = useRef<Map<string, GroupCache>>(new Map())
  const prevListRef = useRef<IRenderItem[]>([])

  // 处理渲染列表
  const renderList: IRenderItem[] = useMemo(() => {
    if (!proxiesData) return []

    // 链式代理模式下，显示代理组和其节点
    if (isChainMode && runtimeConfig && mode === 'rule') {
      // 使用正常的规则模式代理组
      const allGroups = proxiesData.groups?.length
        ? proxiesData.groups
        : proxiesData.global
          ? [proxiesData.global]
          : []

      // 如果选择了特定代理组，只显示该组的节点
      if (selectedGroup) {
        const targetGroup = allGroups.find((g: any) => g.name === selectedGroup)
        if (targetGroup) {
          const proxies = filterSort(
            targetGroup.all,
            targetGroup.name,
            '',
            0,
            latencyTimeout,
          )

          if (col > 1) {
            return groupProxies(proxies, col).map((proxyCol, colIndex) => ({
              type: 4,
              key: `chain-col-${selectedGroup}-${colIndex}`,
              group: targetGroup,
              headState: DEFAULT_STATE,
              col,
              proxyCol,
              provider: proxyCol[0]?.provider,
            }))
          } else {
            return proxies.map((proxy, proxyIdx) => ({
              type: 2,
              key: `chain-${selectedGroup}-${proxy?.name ?? proxyIdx}`,
              group: targetGroup,
              proxy,
              headState: DEFAULT_STATE,
              provider: proxy.provider,
              indexInGroup: proxyIdx,
            }))
          }
        }
        return []
      }

      // 如果没有选择特定组，显示第一个组的节点（如果有组的话）
      if (allGroups.length > 0) {
        const firstGroup = allGroups[0]
        const proxies = filterSort(
          firstGroup.all,
          firstGroup.name,
          '',
          0,
          latencyTimeout,
        )

        if (col > 1) {
          return groupProxies(proxies, col).map((proxyCol, colIndex) => ({
            type: 4,
            key: `chain-col-first-${colIndex}`,
            group: firstGroup,
            headState: DEFAULT_STATE,
            col,
            proxyCol,
            provider: proxyCol[0]?.provider,
          }))
        } else {
          return proxies.map((proxy, proxyIdx) => ({
            type: 2,
            key: `chain-first-${proxy?.name ?? proxyIdx}`,
            group: firstGroup,
            proxy,
            headState: DEFAULT_STATE,
            provider: proxy.provider,
            indexInGroup: proxyIdx,
          }))
        }
      }

      // 如果没有组，显示所有节点
      const allProxies: IProxyItem[] = allGroups.flatMap(
        (group: any) => group?.all ?? [],
      )

      // 为每个节点获取延迟信息
      const proxiesWithDelay = allProxies.map((proxy) => {
        const delay = delayManager.getDelay(proxy.name, 'chain-mode')
        return {
          ...proxy,
          // 如果delayManager有延迟数据，更新history
          history:
            delay >= 0
              ? [{ time: new Date().toISOString(), delay }]
              : proxy.history || [],
        }
      })

      // 创建一个虚拟的组来容纳所有节点
      const virtualGroup: ProxyGroup = {
        name: 'All Proxies',
        type: 'Selector',
        udp: false,
        xudp: false,
        tfo: false,
        mptcp: false,
        smux: false,
        history: [],
        now: '',
        all: proxiesWithDelay,
      }

      if (col > 1) {
        return groupProxies(proxiesWithDelay, col).map(
          (proxyCol, colIndex) => ({
            type: 4,
            key: `chain-col-all-${colIndex}`,
            group: virtualGroup,
            headState: DEFAULT_STATE,
            col,
            proxyCol,
            provider: proxyCol[0]?.provider,
          }),
        )
      } else {
        return proxiesWithDelay.map((proxy, proxyIdx) => ({
          type: 2,
          key: `chain-all-${proxy.name}`,
          group: virtualGroup,
          proxy,
          headState: DEFAULT_STATE,
          provider: proxy.provider,
          indexInGroup: proxyIdx,
        }))
      }
    }

    // 链式代理模式下的其他模式（如global）仍显示所有节点
    if (isChainMode && runtimeConfig) {
      // 从运行时配置直接获取 proxies 列表
      const allProxies: IProxyItem[] = Object.values(
        (runtimeConfig as { proxies?: Record<string, IProxyItem> }).proxies ||
          {},
      )

      // 为每个节点获取延迟信息
      const proxiesWithDelay = allProxies.map((proxy) => {
        const delay = delayManager.getDelay(proxy.name, 'chain-mode')
        return {
          ...proxy,
          // 如果delayManager有延迟数据，更新history
          history:
            delay >= 0
              ? [{ time: new Date().toISOString(), delay }]
              : proxy.history || [],
        }
      })

      // 创建一个虚拟的组来容纳所有节点
      const virtualGroup: ProxyGroup = {
        name: 'All Proxies',
        type: 'Selector',
        udp: false,
        xudp: false,
        tfo: false,
        mptcp: false,
        smux: false,
        history: [],
        now: '',
        all: proxiesWithDelay,
      }

      // 返回节点列表（不显示组头）
      if (col > 1) {
        return groupProxies(proxiesWithDelay, col).map(
          (proxyCol, colIndex) => ({
            type: 4,
            key: `chain-col-${colIndex}`,
            group: virtualGroup,
            headState: DEFAULT_STATE,
            col,
            proxyCol,
            provider: proxyCol[0]?.provider,
          }),
        )
      } else {
        return proxiesWithDelay.map((proxy, proxyIdx) => ({
          type: 2,
          key: `chain-${proxy.name}`,
          group: virtualGroup,
          proxy,
          headState: DEFAULT_STATE,
          provider: proxy.provider,
          indexInGroup: proxyIdx,
        }))
      }
    }

    // 正常模式的渲染逻辑
    const renderGroups = proxiesData.groups?.length
      ? proxiesData.groups.filter((group) => group.name === 'PROXY')
      : []

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

      if (group.name !== 'PROXY') {
        ret.push({
          type: 0,
          key: `group-${group.name}`,
          group,
          headState,
        })
      }

      const isOpen = group.name === 'PROXY' ? true : headState.open

      if (isOpen) {
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
  }, [
    headStates,
    proxiesData,
    mode,
    col,
    isChainMode,
    runtimeConfig,
    selectedGroup,
    delayBump,
    latencyTimeout,
  ])

  return {
    renderList,
    onProxies: refreshProxy,
    onHeadState: setHeadState,
    currentColumns: col,
  }
}

// 优化建议：如有大数据量，建议用虚拟滚动（已在 ProxyGroups 组件中实现），此处无需额外处理。
