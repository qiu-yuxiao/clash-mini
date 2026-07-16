import { useCallback, useMemo, useRef } from 'react'

import { useProfiles } from '@/hooks/use-profiles'
import { useVerge } from '@/hooks/use-verge'
import {
  closeConnectionWithTimeout,
  getConnectionsWithTimeout,
  selectNodeForGroupWithTimeout,
} from '@/services/mihomo-api'
import { debugLog } from '@/utils/debug'

// 缓存连接清理
const cleanupConnections = async (previousProxy: string) => {
  try {
    const { connections } = await getConnectionsWithTimeout()
    const cleanupPromises = (connections ?? [])
      .filter((conn) => conn.chains.includes(previousProxy))
      .map((conn) => closeConnectionWithTimeout(conn.id))

    if (cleanupPromises.length > 0) {
      await Promise.allSettled(cleanupPromises)
      debugLog(`[ProxySelection] 清理了 ${cleanupPromises.length} 个连接`)
    }
  } catch (error) {
    console.warn('[ProxySelection] 连接清理失败:', error)
  }
}

interface ProxySelectionOptions {
  onSuccess?: () => void
  onError?: (error: any) => void
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
  const { verge } = useVerge()
  const pendingRequestRef = useRef<ProxyChangeRequest | null>(null)
  const isProcessingRef = useRef(false)

  const { onSuccess, onError, enableConnectionCleanup = true } = options

  // 缓存
  const config = useMemo(
    () => ({
      autoCloseConnection: verge?.auto_close_connection ?? false,
      enableConnectionCleanup,
    }),
    [verge?.auto_close_connection, enableConnectionCleanup],
  )

  const persistSelection = useCallback(
    (proxyName: string, skipConfigSave: boolean) => {
      if (!current || skipConfigSave) return
      // Mini 单组架构：selected 数组仅含 PROXY 一条
      patchCurrent({ selected: [{ name: 'PROXY', now: proxyName }] }).catch((error) => {
        console.error('[ProxySelection] 保存代理选择失败:', error)
      })
    },
    [current, patchCurrent],
  )

  const executeChange = useCallback(
    async (request: ProxyChangeRequest) => {
      const { groupName, proxyName, previousProxy, skipConfigSave } = request
      debugLog(`[ProxySelection] 代理切换: ${groupName} -> ${proxyName}`)

      try {
        await selectNodeForGroupWithTimeout(groupName, proxyName)
        onSuccess?.()
        persistSelection(proxyName, skipConfigSave)
        debugLog(
          `[ProxySelection] 代理和状态同步完成: ${groupName} -> ${proxyName}`,
        )

        if (
          config.enableConnectionCleanup &&
          config.autoCloseConnection &&
          previousProxy
        ) {
          void cleanupConnections(previousProxy)
        }
      } catch (error) {
        console.error(
          `[ProxySelection] 代理切换失败: ${groupName} -> ${proxyName}`,
          error,
        )
        onError?.(error)
      }
    },
    [config, onError, onSuccess, persistSelection],
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
