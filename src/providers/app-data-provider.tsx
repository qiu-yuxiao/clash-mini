import { useQuery } from '@tanstack/react-query'
import { listen } from '@tauri-apps/api/event'
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { useVerge } from '@/hooks/use-verge'
import {
  calcuProxies,
  calcuProxyProviders,
  getAppUptime,
  getRunningMode,
  getSystemProxy,
} from '@/services/cmds'
import { queryClient } from '@/services/query-client'
import type { IProxyItem, IProxyGroupItem } from '@/types/clash'
import { isDummyNode } from '@/utils/node'
import type { ProxyProvider } from 'tauri-plugin-mihomo-api'
import {
  getBaseConfig,
  getRuleProviders,
  getRules,
  getProxyByName,
} from 'tauri-plugin-mihomo-api'

import {
  ClashConfigContext,
  CoreDataStatusContext,
  ProxiesContext,
  RefreshersContext,
  RulesContext,
  SystemContext,
  UptimeContext,
} from './app-data-context'

const TQ_MIHOMO = {
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
  staleTime: 1500,
  retry: 3,
  retryDelay: (attempt: number) => Math.min(200 * 2 ** attempt, 3000),
} as const

const TQ_DEFAULTS = {
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
  staleTime: 5000,
  retry: 2,
} as const

function useStableFn<T extends (...args: any[]) => any>(fn: T): T {
  const ref = useRef(fn)
  ref.current = fn
  return useCallback((...args: Parameters<T>) => ref.current(...args), []) as T
}

// 全局数据提供者组件
export const AppDataProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const { verge } = useVerge()

  const [isMinimalWidth, setIsMinimalWidth] = useState(
    () => typeof window !== 'undefined' && window.innerWidth <= 285,
  )

  const [isMiniStatus, setIsMiniStatus] = useState(
    () =>
      typeof window !== 'undefined' &&
      window.innerWidth <= 285 &&
      window.innerHeight <= 135,
  )

  const [isSettingsOpen, setIsSettingsOpen] = useState(false)

  useEffect(() => {
    if (typeof window === 'undefined') return
    let rafId: number | null = null
    const handleResize = () => {
      if (rafId !== null) return
      rafId = requestAnimationFrame(() => {
        rafId = null
        setIsMinimalWidth((prev) => {
          const next = window.innerWidth <= 285
          return prev !== next ? next : prev
        })
        setIsMiniStatus((prev) => {
          const next = window.innerWidth <= 285 && window.innerHeight <= 135
          return prev !== next ? next : prev
        })
      })
    }
    const timer = setTimeout(handleResize, 0)
    window.addEventListener('resize', handleResize)
    window.addEventListener('focus', handleResize)
    document.addEventListener('visibilitychange', handleResize)
    return () => {
      clearTimeout(timer)
      if (rafId !== null) cancelAnimationFrame(rafId)
      window.removeEventListener('resize', handleResize)
      window.removeEventListener('focus', handleResize)
      document.removeEventListener('visibilitychange', handleResize)
    }
  }, [])

  const forceFullProxiesRef = useRef(false)

  const fetchProxies = async () => {
    const isMinimal = isMiniStatus
    const forceFull = forceFullProxiesRef.current
    forceFullProxiesRef.current = false

    if (isMinimal && !forceFull) {
      try {
        const groupProxy = await getProxyByName('PROXY')
        if (groupProxy) {
          const activeNodeName = groupProxy.now || ''
          let activeNode: any = null
          if (activeNodeName) {
            try {
              activeNode = await getProxyByName(activeNodeName)
            } catch (e) {
              console.warn('[AppDataProvider] Failed to fetch active node:', e)
            }
          }

          if (activeNode) {
            activeNode.provider =
              activeNode.providerName || activeNode.provider || ''
          }

          const groupItem = {
            name: 'PROXY',
            type: groupProxy.type,
            udp: false,
            xudp: false,
            tfo: false,
            mptcp: false,
            smux: false,
            history: [],
            now: activeNodeName,
            all: groupProxy.all
              ? groupProxy.all
                  .map((name: string) => {
                    if (name === activeNodeName && activeNode) {
                      return activeNode
                    }
                    return {
                      name,
                      type: name === 'DIRECT' ? 'DIRECT' : 'unknown',
                      udp: false,
                      xudp: false,
                      tfo: false,
                      mptcp: false,
                      smux: false,
                      history: [],
                      provider: '',
                    }
                  })
                  .filter((item: any) => !isDummyNode(item.name))
              : [],
          }

          return {
            global: {
              name: 'GLOBAL',
              type: 'Selector',
              udp: false,
              xudp: false,
              tfo: false,
              mptcp: false,
              smux: false,
              history: [],
              now: '',
              all: [],
            } as IProxyGroupItem,
            direct: {
              name: 'DIRECT',
              type: 'Direct',
              udp: true,
              xudp: false,
              tfo: false,
              mptcp: false,
              smux: false,
              history: [],
            } as IProxyItem,
            groups: [groupItem] as IProxyGroupItem[],
            records:
              activeNodeName && activeNode
                ? {
                    [activeNodeName]: activeNode,
                  }
                : {},
            proxies: [],
          }
        }
      } catch (err) {
        console.error('[AppDataProvider] Failed to fetch minimal proxies:', err)
      }
    }

    return calcuProxies()
  }

  const {
    data: proxiesData,
    isPending: isProxiesPending,
    refetch: _refetchProxy,
  } = useQuery({
    queryKey: ['getProxies'],
    queryFn: fetchProxies,
    refetchInterval: false,
    refetchIntervalInBackground: false,
    ...TQ_MIHOMO,
  })

  useEffect(() => {
    _refetchProxy()
  }, [isMiniStatus, _refetchProxy])

  const {
    data: clashConfig,
    isPending: isClashConfigPending,
    refetch: _refetchClashConfig,
  } = useQuery({
    queryKey: ['getClashConfig'],
    queryFn: getBaseConfig,
    ...TQ_MIHOMO,
  })

  const { data: proxyProviders, refetch: _refetchProxyProviders } = useQuery({
    queryKey: ['getProxyProviders'],
    queryFn: calcuProxyProviders,
    enabled: !isMinimalWidth,
    ...TQ_MIHOMO,
  })

  const { data: ruleProviders, refetch: _refetchRuleProviders } = useQuery({
    queryKey: ['getRuleProviders'],
    queryFn: getRuleProviders,
    enabled: !isMinimalWidth,
    ...TQ_MIHOMO,
  })

  const { data: rulesData, refetch: _refetchRules } = useQuery({
    queryKey: ['getRules'],
    queryFn: getRules,
    enabled: !isMinimalWidth,
    ...TQ_MIHOMO,
  })

  const { data: sysproxy, refetch: _refetchSysproxy } = useQuery({
    queryKey: ['getSystemProxy'],
    queryFn: getSystemProxy,
    enabled: isSettingsOpen,
    ...TQ_DEFAULTS,
  })

  const { data: runningMode } = useQuery({
    queryKey: ['getRunningMode'],
    queryFn: getRunningMode,
    enabled: isSettingsOpen,
    ...TQ_DEFAULTS,
  })

  const { data: uptimeData } = useQuery({
    queryKey: ['appUptime'],
    queryFn: getAppUptime,
    ...TQ_DEFAULTS,
    enabled: false,
    retry: 1,
  })

  const refreshProxy = useStableFn(
    async (options?: { forceFull?: boolean }) => {
      if (options?.forceFull) {
        forceFullProxiesRef.current = true
      }
      return await _refetchProxy()
    },
  )
  const refreshClashConfig = useStableFn(_refetchClashConfig)
  const refreshRules = useStableFn(_refetchRules)
  const refreshSysproxy = useStableFn(_refetchSysproxy)
  const refreshProxyProviders = useStableFn(_refetchProxyProviders)
  const refreshRuleProviders = useStableFn(_refetchRuleProviders)

  useEffect(() => {
    let active = true
    let unlistenProfile: (() => void) | null = null
    let unlistenProxy: (() => void) | null = null

    let lastProfileId: string | null = null
    let lastProfileUpdateTime = 0
    let lastProxyTimer: ReturnType<typeof setTimeout> | null = null
    const refreshThrottle = 800

    const handleProfileChanged = (event: { payload: string }) => {
      const newProfileId = event.payload
      const now = Date.now()
      if (
        lastProfileId === newProfileId &&
        now - lastProfileUpdateTime < refreshThrottle
      ) {
        return
      }
      lastProfileId = newProfileId
      lastProfileUpdateTime = now
      void queryClient.invalidateQueries({ queryKey: ['getProfiles'] })
      refreshRules().catch(() => console.warn('[app-data] refreshRules failed'))
      refreshRuleProviders().catch(() =>
        console.warn('[app-data] refreshRuleProviders failed'),
      )
    }

    const handleRefreshProxy = () => {
      if (lastProxyTimer) {
        clearTimeout(lastProxyTimer)
      }
      lastProxyTimer = setTimeout(() => {
        lastProxyTimer = null
        refreshProxy().catch(() =>
          console.warn('[app-data] refreshProxy failed'),
        )
      }, 200)
    }

    const initializeListeners = async () => {
      try {
        const uProfile = await listen<string>(
          'profile-changed',
          handleProfileChanged,
        )
        if (!active) {
          uProfile()
        } else {
          unlistenProfile = uProfile
        }
      } catch (error) {
        console.error('[AppDataProvider] 监听 Profile 事件失败:', error)
      }

      try {
        const uProxy = await listen(
          'verge://refresh-proxy-config',
          handleRefreshProxy,
        )
        if (!active) {
          uProxy()
        } else {
          unlistenProxy = uProxy
        }
      } catch (error) {
        console.warn('[AppDataProvider] 设置 Tauri 事件监听器失败:', error)
      }
    }

    void initializeListeners()

    return () => {
      active = false
      if (unlistenProfile) {
        unlistenProfile()
      }
      if (unlistenProxy) {
        unlistenProxy()
      }
      if (lastProxyTimer) {
        clearTimeout(lastProxyTimer)
      }
    }
  }, [refreshProxy, refreshRules, refreshRuleProviders])

  const refreshAll = useCallback(async () => {
    // WARN-001 修复：全量刷新时强制绕过 isMiniStatus 精简路径，确保代理列表完整加载
    forceFullProxiesRef.current = true
    await Promise.all([
      refreshProxy(),
      refreshClashConfig(),
      refreshRules(),
      refreshSysproxy(),
      refreshProxyProviders(),
      refreshRuleProviders(),
    ])
  }, [
    refreshProxy,
    refreshClashConfig,
    refreshRules,
    refreshSysproxy,
    refreshProxyProviders,
    refreshRuleProviders,
  ])

  const proxiesValue = useMemo(
    () => ({
      proxies: proxiesData,
      proxyProviders: (proxyProviders || {}) as unknown as Record<
        string,
        ProxyProvider | undefined
      >,
      isProxiesPending,
    }),
    [proxiesData, proxyProviders, isProxiesPending],
  )

  const rulesValue = useMemo(
    () => ({
      rules: rulesData?.rules ?? [],
      ruleProviders: ruleProviders?.providers || {},
    }),
    [rulesData, ruleProviders],
  )

  const clashConfigValue = useMemo(
    () => ({
      clashConfig,
      isClashConfigPending,
    }),
    [clashConfig, isClashConfigPending],
  )

  const systemValue = useMemo(() => {
    const calculateSystemProxyAddress = () => {
      if (!verge || !clashConfig) return '-'

      const isPacMode = verge.proxy_auto_config ?? false

      if (isPacMode) {
        // PAC模式：显示我们期望设置的代理地址
        const proxyHost = verge.proxy_host || '127.0.0.1'
        const proxyPort =
          verge.verge_mixed_port || clashConfig.mixedPort || 10801
        return `${proxyHost}:${proxyPort}`
      } else {
        // HTTP代理模式：优先使用系统地址，但如果格式不正确则使用期望地址
        const systemServer = sysproxy?.server
        if (
          systemServer &&
          systemServer !== '-' &&
          !systemServer.startsWith(':')
        ) {
          return systemServer
        } else {
          // 系统地址无效，返回期望的代理地址
          const proxyHost = verge.proxy_host || '127.0.0.1'
          const proxyPort =
            verge.verge_mixed_port || clashConfig.mixedPort || 10801
          return `${proxyHost}:${proxyPort}`
        }
      }
    }

    return {
      sysproxy,
      runningMode,
      systemProxyAddress: calculateSystemProxyAddress(),
      isSettingsOpen,
      setIsSettingsOpen,
    }
  }, [sysproxy, runningMode, verge, clashConfig, isSettingsOpen])

  const uptimeValue = useMemo(() => ({ uptime: uptimeData || 0 }), [uptimeData])

  const coreDataStatusValue = useMemo(
    () => ({ isCoreDataPending: isProxiesPending || isClashConfigPending }),
    [isProxiesPending, isClashConfigPending],
  )

  const refreshersValue = useMemo(
    () => ({
      refreshProxy,
      refreshClashConfig,
      refreshRules,
      refreshSysproxy,
      refreshProxyProviders,
      refreshRuleProviders,
      refreshAll,
    }),
    [
      refreshProxy,
      refreshClashConfig,
      refreshRules,
      refreshSysproxy,
      refreshProxyProviders,
      refreshRuleProviders,
      refreshAll,
    ],
  )

  return (
    <ProxiesContext value={proxiesValue}>
      <RulesContext value={rulesValue}>
        <ClashConfigContext value={clashConfigValue}>
          <SystemContext value={systemValue}>
            <UptimeContext value={uptimeValue}>
              <CoreDataStatusContext value={coreDataStatusValue}>
                <RefreshersContext value={refreshersValue}>
                  {children}
                </RefreshersContext>
              </CoreDataStatusContext>
            </UptimeContext>
          </SystemContext>
        </ClashConfigContext>
      </RulesContext>
    </ProxiesContext>
  )
}
