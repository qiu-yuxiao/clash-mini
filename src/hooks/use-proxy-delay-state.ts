import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useReducer } from 'react'

import delayManager, { type DelayUpdate } from '@/services/delay'
import { getPreloadConfig } from '@/services/preload'
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
  const timeout = getPreloadConfig()?.default_latency_timeout || 10000

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

  const onDelay = useLockFn(async () => {
    if (!proxy) return
    setDelayState({ delay: -2, updatedAt: Date.now() })
    const currentTimeout = getPreloadConfig()?.default_latency_timeout || 10000
    setDelayState(await delayManager.checkDelay(proxy.name, groupName, currentTimeout))
  })

  return {
    delayState,
    delayValue: delayState.delay,
    isPreset,
    timeout,
    onDelay,
  }
}
