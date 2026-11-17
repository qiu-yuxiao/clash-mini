import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useRef } from 'react'

import { useProfiles } from '@/hooks/use-profiles'
import { frontendLog, notifyNodeSwitched } from '@/services/cmds'
import {
  closeStaleProxyConnections,
  selectNodeForGroupWithTimeout,
} from '@/services/mihomo-api'
import { debugLog } from '@/utils/debug'

interface ProxySelectionOptions {
  onSuccess?: () => void
  onError?: (error: unknown) => void
  enableConnectionCleanup?: boolean
}

interface ProxyChangeRequest {
  groupName: string
  proxyName: string
  previousProxy?: string
  skipConfigSave: boolean
}

// 代理选择 Hook
export const useProxySelection = (options: ProxySelectionOptions = {}) => {
  const { current, patchCurrent } = useProfiles()
  const queryClient = useQueryClient()
  const pendingRequestRef = useRef<ProxyChangeRequest | null>(null)
  const isProcessingRef = useRef(false)

  const { onSuccess, onError, enableConnectionCleanup = true } = options
  // BUG-337: 用 ref 持有回调与最新配置，保证 changeProxy / handleProxyGroupChange 引用绝对稳定，
  // 防止父级重绘击穿 ProxyRender 的 useMemo 与千节点列表的 React.memo
  const onSuccessRef = useRef(onSuccess)
  onSuccessRef.current = onSuccess
  const onErrorRef = useRef(onError)
  onErrorRef.current = onError
  const enableConnectionCleanupRef = useRef(enableConnectionCleanup)
  enableConnectionCleanupRef.current = enableConnectionCleanup

  const persistSelection = useCallback(
    (proxyName: string, skipConfigSave: boolean) => {
      if (!current || skipConfigSave) return
      // Mini 单组架构：selected 数组仅含 PROXY 一条
      patchCurrent({ selected: [{ name: 'PROXY', now: proxyName }] }).catch(
        (error) => {
          const msg = `[ProxySelection] 保存代理选择失败: ${error instanceof Error ? error.message : String(error)}`
          console.error(msg)
          // 与 use-profiles.ts 诊断体系一致：写入后端 latest.log 便于 UI 卡死时仍可诊断
          frontendLog('error', msg)
        },
      )
    },
    [current, patchCurrent],
  )
  const persistSelectionRef = useRef(persistSelection)
  persistSelectionRef.current = persistSelection

  const executeChange = useCallback(
    async (request: ProxyChangeRequest) => {
      const { groupName, proxyName, skipConfigSave } = request
      debugLog(`[ProxySelection] 代理切换: ${groupName} -> ${proxyName}`)

      // BUG-356: 0ms 乐观高亮更新，点击瞬间即在 ['getProxies'] 缓存切至目标节点，失败自动回滚
      const previousSnapshot = queryClient.getQueryData(['getProxies'])
      queryClient.setQueryData(
        ['getProxies'],
        (old: Record<string, unknown> | undefined) => {
          if (!old || typeof old !== 'object') return old
          const next: Record<string, unknown> = { ...old }
          if (Array.isArray(old.groups)) {
            next.groups = old.groups.map((g: Record<string, unknown>) =>
              g && g.name === groupName ? { ...g, now: proxyName } : g,
            )
          }
          return next
        },
      )

      try {
        await selectNodeForGroupWithTimeout(groupName, proxyName)
        notifyNodeSwitched()
        onSuccessRef.current?.()
        persistSelectionRef.current(proxyName, skipConfigSave)
        void queryClient.invalidateQueries({ queryKey: ['getProxies'] })
        debugLog(
          `[ProxySelection] 代理和状态同步完成: ${groupName} -> ${proxyName}`,
        )

        // BUG-396: 前台用户显式切换节点时，立即清理仍挂在非当前目标节点上的存量代理长连接，
        // 避免 HTTP/2 多路复用连接粘滞在旧节点上导致地区策略不生效；直连流量保持零干扰。
        if (enableConnectionCleanupRef.current) {
          void closeStaleProxyConnections(proxyName, groupName)
        }
      } catch (error) {
        if (previousSnapshot !== undefined) {
          queryClient.setQueryData(['getProxies'], previousSnapshot)
        } else {
          void queryClient.invalidateQueries({ queryKey: ['getProxies'] })
        }
        console.error(
          `[ProxySelection] 代理切换失败: ${groupName} -> ${proxyName}`,
          error,
        )
        onErrorRef.current?.(error)
      }
    },
    [queryClient],
  )

  // L-27: flushChangeQueue 看起来有递归调用（finally 中可能再次调用自己），
  // 但实际上递归深度是可控的：
  // 1. isProcessingRef 作为互斥锁，同一时间只有一个执行流
  // 2. while 循环会持续消费队列，直到队列为空才退出
  // 3. finally 中的递归调用仅在执行过程中又有新请求入队时才触发
  // 4. 每次递归都处理一批新请求，不会无限循环（请求不会无限快速产生）
  const flushChangeQueue = useCallback(async () => {
    if (isProcessingRef.current) return
    isProcessingRef.current = true

    try {
      while (pendingRequestRef.current) {
        const request = pendingRequestRef.current
        pendingRequestRef.current = null
        await executeChange(request)
      }
    } finally {
      isProcessingRef.current = false
      if (pendingRequestRef.current) {
        void flushChangeQueue()
      }
    }
  }, [executeChange])

  const changeProxy = useCallback(
    (
      groupName: string,
      proxyName: string,
      previousProxy?: string,
      skipConfigSave: boolean = false,
    ) => {
      pendingRequestRef.current = {
        groupName,
        proxyName,
        previousProxy,
        skipConfigSave,
      }
      void flushChangeQueue()
    },
    [flushChangeQueue],
  )

  const handleProxyGroupChange = useCallback(
    (group: { name: string; now?: string }, proxy: { name: string }) => {
      changeProxy(group.name, proxy.name, group.now)
    },
    [changeProxy],
  )

  return {
    changeProxy,
    handleProxyGroupChange,
  }
}
