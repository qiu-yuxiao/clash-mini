import { useCallback, useEffect, useReducer } from 'react'

import {
  getDelayManager,
  NODE_DELAY_MAX_MS,
  type DelayUpdate,
} from '@/services/delay'
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

function computeInitialDelay(
  proxy: IProxyItem | undefined,
  groupName: string,
): DelayUpdate {
  if (!proxy) return INITIAL_DELAY
  const cachedUpdate = getDelayManager().getDelayUpdate(proxy.name, groupName)
  if (cachedUpdate) {
    return { ...cachedUpdate }
  }

  const fallbackDelay = getDelayManager().getDelayFix(proxy, groupName)
  if (fallbackDelay === -1) {
    return INITIAL_DELAY
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

  return { delay: fallbackDelay, updatedAt }
}

export interface UseProxyDelayState {
  delayState: DelayUpdate
  delayValue: number
  isPreset: boolean
  timeout: number
}

export function useProxyDelayState(
  proxy: IProxyItem,
  groupName: string,
): UseProxyDelayState {
  const isPreset = proxy ? PRESET_PROXY_NAMES.includes(proxy.name) : false
  // 惰性初始化：首帧直接从 DelayManager 缓存或 history 回落值计算初始延迟，消除挂载时的双重渲染
  const [delayState, setDelayState] = useReducer(
    identity,
    { proxy, groupName },
    ({ proxy: p, groupName: g }) => computeInitialDelay(p, g),
  )

  // 死活着色阈值统一为 NODE_DELAY_MAX_MS(5000)，与后端死节点判定一致
  const timeout = NODE_DELAY_MAX_MS

  useEffect(() => {
    if (isPreset || !proxy) return
    getDelayManager().setListener(proxy.name, groupName, setDelayState)
    return () => {
      // 必须带上具体 listener：同节点可能有多个订阅者（活跃节点栏也在监听），
      // 无参调用会把它一并摘掉，导致那边永久收不到延迟更新。
      getDelayManager().removeListener(proxy.name, groupName, setDelayState)
    }
  }, [proxy, groupName, isPreset])

  const updateDelay = useCallback(() => {
    if (!proxy) return
    setDelayState(computeInitialDelay(proxy, groupName))
  }, [proxy, groupName])

  useEffect(() => {
    updateDelay()
  }, [updateDelay])

  return {
    delayState,
    delayValue: delayState.delay,
    isPreset,
    timeout,
  }
}
