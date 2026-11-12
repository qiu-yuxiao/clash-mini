import { useQuery, useQueryClient } from '@tanstack/react-query'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useLocalStorage } from 'foxact/use-local-storage'
import { type MutableRefObject, useCallback, useEffect, useRef } from 'react'

import { type Message, type MihomoWebSocket } from 'tauri-plugin-mihomo-api'

export const RECONNECT_DELAY_MS = 1000
export const MAX_RECONNECT_DELAY_MS = 15_000
export const REFRESH_CLASH_DEBOUNCE_MS = 250

/**
 * BUG-347: 指数退避 + 抖动，且严格保证上界不超过 MAX_RECONNECT_DELAY_MS (15000ms)，
 * 抑制内核异常期前端 WebSocket 1000ms 死循环重连风暴。
 */
export const computeWsReconnectDelay = (attempt: number): number => {
  const cappedExp = Math.min(Math.max(attempt, 0), 4)
  const base = Math.min(
    RECONNECT_DELAY_MS * Math.pow(2, cappedExp),
    MAX_RECONNECT_DELAY_MS,
  )
  const minDelay = Math.round(base * 0.8)
  const maxDelay = Math.min(Math.round(base * 1.2), MAX_RECONNECT_DELAY_MS)
  return Math.round(minDelay + Math.random() * (maxDelay - minDelay))
}

interface SharedSubscriptionOwner {
  handleMessage: (data: string) => void
  onConnected?: (ws: MihomoWebSocket) => Promise<void> | void
  cleanup?: () => void
  isMounted: () => boolean
}

interface SharedSubscriptionEntry {
  refs: number
  ws: MihomoWebSocket | null
  reconnectTimer: ReturnType<typeof setTimeout> | null
  reconnectAttempt: number
  connecting: boolean
  refHolders: Set<MutableRefObject<MihomoWebSocket | null>>
  owners: Set<SharedSubscriptionOwner>
  activeOwner: SharedSubscriptionOwner | null
  closed: boolean
  connectWs: () => Promise<void>
  scheduleReconnect: () => Promise<void>
}

const sharedSubscriptions = new Map<string, SharedSubscriptionEntry>()
let refreshClashUnlisten: UnlistenFn | null = null
let refreshClashListening = false
let refreshClashListenerEpoch = 0
let refreshClashDebounceTimer: ReturnType<typeof setTimeout> | null = null

/**
 * 当后端完成内核重启或配置重载发出 `verge://refresh-clash-config` 时，
 * 立即清空所有活动 WS 通道的退避定时器与退避计数；对已持有陈旧 socket 句柄的通道先主动关闭再立即重连，
 * 彻底消除最长 15s 陈旧等待窗与半死句柄滞留（BUG-347）。
 */
export const flushAllWsReconnectBackoff = () => {
  for (const entry of sharedSubscriptions.values()) {
    if (entry.closed) continue
    if (entry.reconnectTimer) {
      clearTimeout(entry.reconnectTimer)
      entry.reconnectTimer = null
    }
    entry.reconnectAttempt = 0
    if (entry.ws) {
      void (async () => {
        await closeSharedSocket(entry)
        if (!entry.closed && !entry.connecting && !entry.ws) {
          await entry.connectWs()
        }
      })()
    } else if (!entry.connecting) {
      void entry.connectWs()
    }
  }
}

export const scheduleFlushWsReconnectBackoff = () => {
  if (refreshClashDebounceTimer !== null) return
  refreshClashDebounceTimer = setTimeout(() => {
    refreshClashDebounceTimer = null
    flushAllWsReconnectBackoff()
  }, REFRESH_CLASH_DEBOUNCE_MS)
}

const ensureRefreshClashListener = () => {
  if (
    refreshClashListening ||
    refreshClashUnlisten !== null ||
    typeof window === 'undefined' ||
    !('__TAURI_INTERNALS__' in window)
  ) {
    return
  }
  refreshClashListening = true
  const epoch = ++refreshClashListenerEpoch
  listen('verge://refresh-clash-config', () => {
    scheduleFlushWsReconnectBackoff()
  })
    .then((unlisten) => {
      refreshClashListening = false
      if (
        epoch !== refreshClashListenerEpoch ||
        sharedSubscriptions.size === 0
      ) {
        unlisten()
      } else {
        refreshClashUnlisten = unlisten
      }
    })
    .catch(() => {
      refreshClashListening = false
    })
}

const cleanupRefreshClashListenerIfIdle = () => {
  if (sharedSubscriptions.size === 0) {
    refreshClashListenerEpoch += 1
    refreshClashListening = false
    if (refreshClashDebounceTimer !== null) {
      clearTimeout(refreshClashDebounceTimer)
      refreshClashDebounceTimer = null
    }
    if (refreshClashUnlisten) {
      const unlisten = refreshClashUnlisten
      refreshClashUnlisten = null
      unlisten()
    }
  }
}

export const acquireSharedSubscriptionEntry = (
  subscriptionCacheKey: string,
  connect: () => Promise<MihomoWebSocket>,
): SharedSubscriptionEntry => {
  let entry = sharedSubscriptions.get(subscriptionCacheKey)
  if (!entry) {
    entry = createSharedSubscriptionEntry(connect)
    sharedSubscriptions.set(subscriptionCacheKey, entry)
  }
  ensureRefreshClashListener()
  entry.refs += 1
  return entry
}

export const releaseSharedSubscriptionEntry = (
  subscriptionCacheKey: string,
  entry: SharedSubscriptionEntry,
) => {
  entry.refs -= 1
  if (entry.refs <= 0) {
    entry.closed = true
    if (entry.reconnectTimer) {
      clearTimeout(entry.reconnectTimer)
      entry.reconnectTimer = null
    }
    sharedSubscriptions.delete(subscriptionCacheKey)
    cleanupRefreshClashListenerIfIdle()
    void closeSharedSocket(entry)
  }
}

const syncSharedWsRefs = (entry: SharedSubscriptionEntry) => {
  entry.refHolders.forEach((ref) => {
    ref.current = entry.ws
  })
}

const pickActiveOwner = (entry: SharedSubscriptionEntry) => {
  if (entry.activeOwner?.isMounted()) return entry.activeOwner

  for (const owner of entry.owners) {
    if (owner.isMounted()) {
      entry.activeOwner = owner
      return owner
    }
  }

  entry.activeOwner = null
  return null
}

const closeSharedSocket = async (entry: SharedSubscriptionEntry) => {
  const ws = entry.ws
  if (!ws) return

  entry.ws = null
  syncSharedWsRefs(entry)
  await ws.close()
}

const createSharedSubscriptionEntry = (
  connect: () => Promise<MihomoWebSocket>,
): SharedSubscriptionEntry => {
  const entry: SharedSubscriptionEntry = {
    refs: 0,
    ws: null,
    reconnectTimer: null,
    reconnectAttempt: 0,
    connecting: false,
    refHolders: new Set(),
    owners: new Set(),
    activeOwner: null,
    closed: false,
    connectWs: async () => {},
    scheduleReconnect: async () => {},
  }

  const clearReconnectTimer = () => {
    if (entry.reconnectTimer) {
      clearTimeout(entry.reconnectTimer)
      entry.reconnectTimer = null
    }
  }

  entry.connectWs = async () => {
    if (entry.closed || entry.connecting || entry.ws) return

    entry.connecting = true
    try {
      const ws = await connect()
      if (entry.closed) {
        await ws.close()
        return
      }

      entry.ws = ws
      entry.reconnectAttempt = 0
      syncSharedWsRefs(entry)
      clearReconnectTimer()

      const owner = pickActiveOwner(entry)
      if (owner?.onConnected) {
        await owner.onConnected(ws)
        if (entry.closed) {
          await ws.close()
          return
        }
      }

      ws.addListener((msg: Message) => {
        if (msg.type !== 'Text') return
        const activeOwner = pickActiveOwner(entry)
        if (!activeOwner) {
          if (
            msg.data.startsWith('Websocket error') &&
            !entry.closed &&
            entry.ws === ws
          ) {
            void entry.scheduleReconnect()
          }
          return
        }

        activeOwner.handleMessage(msg.data)
      })
    } catch (err) {
      const delayMs = computeWsReconnectDelay(entry.reconnectAttempt)
      entry.reconnectAttempt += 1
      console.warn(
        `[WS] 连接失败，将在 ${delayMs}ms 后退避重试:`,
        err instanceof Error ? err.message : String(err),
      )
      if (!entry.closed && !entry.ws) {
        clearReconnectTimer()
        entry.reconnectTimer = setTimeout(entry.connectWs, delayMs)
      }
    } finally {
      entry.connecting = false
    }
  }

  entry.scheduleReconnect = async () => {
    if (entry.closed) return

    clearReconnectTimer()
    await closeSharedSocket(entry)
    if (!entry.closed) {
      const delayMs = computeWsReconnectDelay(entry.reconnectAttempt)
      entry.reconnectAttempt += 1
      entry.reconnectTimer = setTimeout(entry.connectWs, delayMs)
    }
  }

  return entry
}

/**
 * Mirrors SWR's MutatorCallback: consumers can pass either a plain value or a
 * functional updater `(current?: T) => T`.  The functional form is resolved
 * against the current cache entry before calling `queryClient.setQueryData`.
 */
type NextFn<T> = (
  error?: unknown,
  data?: T | ((current?: T) => T | undefined),
) => void

interface HandlerContext<T> {
  next: NextFn<T>
  scheduleReconnect: () => Promise<void>
  isMounted: () => boolean
}

interface HandlerResult {
  handleMessage: (data: string) => void
  onConnected?: (ws: MihomoWebSocket) => Promise<void> | void
  cleanup?: () => void
}

interface UseMihomoWsSubscriptionOptions<T> {
  storageKey: string
  buildSubscriptKey: (date: number) => string | null
  buildCacheKey?: (date: number) => string
  fallbackData: T
  connect: () => Promise<MihomoWebSocket>
  /**
   * When > 0, coalesce rapid WebSocket messages by wrapping the `next`
   * function passed to `setupHandlers`.  Only the most recent value is
   * flushed, at most once per `throttleMs` milliseconds.
   *
   * Uses `setTimeout` (not `requestAnimationFrame`) so it keeps working
   * when the window is backgrounded or minimized.
   */
  throttleMs?: number
  setupHandlers: (ctx: HandlerContext<T>) => HandlerResult
}

export const useMihomoWsSubscription = <T>(
  options: UseMihomoWsSubscriptionOptions<T>,
) => {
  const {
    storageKey,
    buildSubscriptKey,
    buildCacheKey,
    fallbackData,
    connect,
    throttleMs,
    setupHandlers,
  } = options

  // eslint-disable-next-line @eslint-react/purity
  const [date, setDate] = useLocalStorage(storageKey, Date.now())
  const subscriptKey = buildSubscriptKey(date)
  const subscriptionCacheKey = subscriptKey ? `$sub$${subscriptKey}` : null
  const lastSubscriptionCacheKeyRef = useRef<string | null>(null)
  if (subscriptionCacheKey) {
    lastSubscriptionCacheKeyRef.current = subscriptionCacheKey
  }
  const responseCacheKey =
    subscriptionCacheKey ??
    lastSubscriptionCacheKeyRef.current ??
    (buildCacheKey ? `$sub$${buildCacheKey(date)}` : null)

  const queryClient = useQueryClient()

  const wsRef = useRef<MihomoWebSocket | null>(null)

  const resolveNextData = useCallback(
    (
      data: T | ((current?: T) => T | undefined) | undefined,
      cacheKey: string,
    ): T => {
      if (typeof data === 'function') {
        const updater = data as (current?: T) => T | undefined
        const current = queryClient.getQueryData<T>([cacheKey])
        return updater(current) ?? fallbackData
      }
      return data ?? fallbackData
    },
    [queryClient, fallbackData],
  )

  const response = useQuery<T>({
    queryKey: responseCacheKey ? [responseCacheKey] : ['$sub$__disabled__'],
    queryFn: () =>
      responseCacheKey
        ? (queryClient.getQueryData<T>([responseCacheKey]) ?? fallbackData)
        : fallbackData,
    initialData: () =>
      queryClient.getQueryData<T>([responseCacheKey ?? '$sub$__disabled__']) ??
      fallbackData,
    staleTime: Infinity,
    gcTime: 30_000,
    enabled: subscriptionCacheKey !== null,
  })

  const connectRef = useRef(connect)
  connectRef.current = connect
  const setupHandlersRef = useRef(setupHandlers)
  setupHandlersRef.current = setupHandlers

  useEffect(() => {
    if (!subscriptionCacheKey) return

    let isMounted = true
    const entry = acquireSharedSubscriptionEntry(subscriptionCacheKey, () =>
      connectRef.current(),
    )
    entry.refHolders.add(wsRef)
    wsRef.current = entry.ws

    let throttleCleanup: (() => void) | undefined
    let wrappedNext: NextFn<T>

    const baseNext: NextFn<T> = (error, data) => {
      if (error !== undefined && error !== null) {
        return
      }
      if (data === undefined) return
      const resolved = resolveNextData(data, subscriptionCacheKey)
      queryClient.setQueryData<T>([subscriptionCacheKey], resolved)
    }

    if (throttleMs && throttleMs > 0) {
      let pendingData: T | ((current?: T) => T | undefined) | undefined
      let hasPending = false
      let timerId: ReturnType<typeof setTimeout> | null = null

      const flush = () => {
        timerId = null
        if (hasPending) {
          const data = pendingData
          pendingData = undefined
          hasPending = false
          baseNext(undefined, data)
        }
      }

      // L-31: 节流采用"前缘触发 + 后缘合并"模式：
      // - 首条消息立即 flush（保证初始数据/状态变化的及时性）
      // - 后续在 throttleMs 窗口内的消息合并，只保留最新值
      // - 窗口结束时 flush 最新值（保证最终一致性）
      // 这对于 WebSocket 订阅场景是合理的：既保证响应速度，又减少重复渲染。
      wrappedNext = (
        error?: unknown,
        data?: T | ((current?: T) => T | undefined),
      ) => {
        if (error !== undefined && error !== null) {
          baseNext(error, data)
          return
        }
        if (!timerId) {
          baseNext(undefined, data)
          timerId = setTimeout(flush, throttleMs)
        } else {
          pendingData = data
          hasPending = true
        }
      }

      throttleCleanup = () => {
        if (timerId) {
          clearTimeout(timerId)
          timerId = null
        }
      }
    } else {
      wrappedNext = baseNext
    }

    const {
      handleMessage: handleTextMessage,
      onConnected,
      cleanup,
    } = setupHandlersRef.current({
      next: wrappedNext,
      scheduleReconnect: entry.scheduleReconnect,
      isMounted: () => isMounted,
    })

    const owner: SharedSubscriptionOwner = {
      handleMessage: handleTextMessage,
      onConnected,
      cleanup: () => {
        throttleCleanup?.()
        cleanup?.()
      },
      isMounted: () => isMounted,
    }

    entry.owners.add(owner)
    if (!entry.activeOwner) {
      entry.activeOwner = owner
    }
    void entry.connectWs()

    return () => {
      isMounted = false
      entry.refHolders.delete(wsRef)
      wsRef.current = null
      entry.owners.delete(owner)
      owner.cleanup?.()

      if (entry.activeOwner === owner) {
        entry.activeOwner = null
        const nextOwner = pickActiveOwner(entry)
        if (entry.ws && nextOwner?.onConnected) {
          void nextOwner.onConnected(entry.ws)
        }
      }

      releaseSharedSubscriptionEntry(subscriptionCacheKey, entry)
    }
    // eslint-disable-next-line react-compiler/react-compiler
    // eslint-disable-next-line react-hooks/exhaustive-deps, @eslint-react/exhaustive-deps
  }, [subscriptionCacheKey])

  const refresh = useCallback(() => {
    if (subscriptionCacheKey) {
      queryClient.removeQueries({ queryKey: [subscriptionCacheKey] })
    }
    setDate(Date.now())
  }, [queryClient, subscriptionCacheKey, setDate])

  return { response, refresh, subscriptionCacheKey: responseCacheKey, wsRef }
}
