import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useReducer, useRef } from 'react'

import delayManager, { NODE_DELAY_MAX_MS, type DelayUpdate } from '@/services/delay'
import type { IProxyItem } from '@/types/clash'

const PRESET_PROXY_NAMES = [
  'DIRECT',
  'REJECT',
  'REJECT-DROP',
  'PASS',
  'COMPATIBLE',
]

const identity = (_: DelayUpdate, next: DelayUpdate): DelayUpdate => next

const INITIAL_DELAY: DelayUpdate = { delay: -1, updatedAt: 0 }

export interface UseProxyDelayState {
  delayState: DelayUpdate
  delayValue: number
  isPreset: boolean
  timeout: number
  onDelay: () => Promise<void>
}

export function useProxyDelayState(
  proxy: IProxyItem,
  groupName: string,
): UseProxyDelayState {
  const isPreset = proxy ? PRESET_PROXY_NAMES.includes(proxy.name) : false
  const [delayState, setDelayState] = useReducer(identity, INITIAL_DELAY)
  const isMountedRef = useRef(true)
  const abortControllerRef = useRef<AbortController | null>(null)

  useEffect(() => {
    isMountedRef.current = true
    return () => {
      isMountedRef.current = false
      if (abortControllerRef.current) {
        abortControllerRef.current.abort()
      }
    }
  }, [])
  // 死活着色阈值统一为 NODE_DELAY_MAX_MS(2000)，与后端死节点判定一致
  const timeout = NODE_DELAY_MAX_MS

  useEffect(() => {
    if (isPreset || !proxy) return
    delayManager.setListener(proxy.name, groupName, setDelayState)
    return () => {
      delayManager.removeListener(proxy.name, groupName)
    }
  }, [proxy, groupName, isPreset])

  const updateDelay = useCallback(() => {
    if (!proxy) return
    const cachedUpdate = delayManager.getDelayUpdate(proxy.name, groupName)
    if (cachedUpdate) {
      setDelayState({ ...cachedUpdate })
      return
    }

    const fallbackDelay = delayManager.getDelayFix(proxy, groupName)
    if (fallbackDelay === -1) {
      setDelayState({ delay: -1, updatedAt: 0 })
      return
    }

    let updatedAt = 0
    const history = proxy?.history
    if (history && history.length > 0) {
      const lastRecord = history[history.length - 1]
      const parsed = Date.parse(lastRecord.time)
      if (!Number.isNaN(parsed)) {
        updatedAt = parsed
      }
    }

    setDelayState({ delay: fallbackDelay, updatedAt })
  }, [proxy, groupName])

  useEffect(() => {
    updateDelay()
  }, [updateDelay])

  // L-28: abortControllerRef 的清理逻辑说明：
  // 1. useLockFn 保证 onDelay 不会并发执行，同一时间只有一个活跃的测速请求
  // 2. 每次调用开始时，如果存在旧的 abortController，先 abort 它
  // 3. finally 中：如果是被 abort 的，保留引用（因为刚被新请求 abort，新请求已接管）；
  //    如果正常完成，清空引用
  // 4. 组件卸载时，useEffect 清理函数会 abort 正在进行的请求
  const onDelay = useLockFn(async () => {
    if (!proxy) return
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
    }
    abortControllerRef.current = new AbortController()

    setDelayState({ delay: -2, updatedAt: Date.now() })
    const currentTimeout = NODE_DELAY_MAX_MS
    try {
      const result = await delayManager.checkDelay(
        proxy.name,
        groupName,
        currentTimeout,
        abortControllerRef.current.signal,
      )
      if (isMountedRef.current) {
        setDelayState(result)
        // 单点测速完成后通知组级监听，驱动 useRenderList 重排
        delayManager.queueGroupNotification(groupName)
      }
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') {
        return
      }
      console.error(err)
    } finally {
      if (abortControllerRef.current?.signal.aborted) {
        // do nothing
      } else {
        abortControllerRef.current = null
      }
    }
  })

  return {
    delayState,
    delayValue: delayState.delay,
    isPreset,
    timeout,
    onDelay,
  }
}
