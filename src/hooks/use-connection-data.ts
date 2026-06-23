import { useQueryClient } from '@tanstack/react-query'

import { useVisibility } from '@/hooks/use-visibility'
import type { IConnectionsItem } from '@/types/connection'
import { MihomoWebSocket } from 'tauri-plugin-mihomo-api'

import { useMihomoWsSubscription } from './use-mihomo-ws-subscription'

const MAX_CLOSED_CONNS_NUM = 500

export const initConnData: ConnectionMonitorData = {
  uploadTotal: 0,
  downloadTotal: 0,
  activeConnections: [],
  closedConnections: [],
}

export interface ConnectionMonitorData {
  uploadTotal: number
  downloadTotal: number
  activeConnections: IConnectionsItem[]
  closedConnections: IConnectionsItem[]
}

export const useConnectionData = (options?: { enabled?: boolean }) => {
  const enabled = options?.enabled ?? true
  const isVisible = useVisibility()
  const queryClient = useQueryClient()

  const isWsActive = enabled && isVisible

  const { response, refresh, subscriptionCacheKey } =
    useMihomoWsSubscription<ConnectionMonitorData>({
      storageKey: 'mihomo_connection_date',
      buildSubscriptKey: (date) =>
        isWsActive ? `getClashConnection-${date}` : null,
      buildCacheKey: (date) => `getClashConnection-${date}`,
      fallbackData: initConnData,
      connect: () => MihomoWebSocket.connect_connections(),
      throttleMs: 1000,
      setupHandlers: ({ next, scheduleReconnect }) => {
        let currentEpochId: string | null = null
        let lastSequenceId = -1

        return {
          handleMessage: (data) => {
            if (data.startsWith('Websocket error')) {
              next(data)
              void scheduleReconnect()
              return
            }

            try {
              const msg = JSON.parse(data)

              next(null, (old = initConnData) => {
                if (msg.type === 'snapshot') {
                  const data = msg.data ?? {}
                  currentEpochId = data.epochId
                  lastSequenceId = data.sequenceId

                  return {
                    uploadTotal: data.uploadTotal,
                    downloadTotal: data.downloadTotal,
                    activeConnections: (data.connections ?? []).map(
                      (conn: any) => ({
                        ...conn,
                        curUpload: 0,
                        curDownload: 0,
                      }),
                    ),
                    closedConnections: old.closedConnections,
                  }
                }

                if (msg.type === 'delta') {
                  const delta = msg.data ?? {}

                  // Sequence & Epoch Validation
                  if (
                    delta.epochId !== currentEpochId ||
                    delta.sequenceId !== lastSequenceId + 1
                  ) {
                    console.warn(
                      'Sequence mismatch or epoch change. Triggering connection resync.',
                    )
                    currentEpochId = null
                    lastSequenceId = -1
                    void scheduleReconnect()
                    return old
                  }

                  lastSequenceId = delta.sequenceId

                  const previousActive = old.activeConnections ?? []
                  const previousClosed = old.closedConnections ?? []
                  const activeMap = new Map<string, IConnectionsItem>()

                  for (let i = 0; i < previousActive.length; i++) {
                    activeMap.set(previousActive[i].id, {
                      ...previousActive[i],
                    })
                  }

                  // 1. Process Removals
                  const dropped: IConnectionsItem[] = []
                  const removed = delta.removed ?? []
                  for (let i = 0; i < removed.length; i++) {
                    const id = removed[i]
                    const conn = activeMap.get(id)
                    if (conn) {
                      activeMap.delete(id)
                      dropped.push(conn)
                    }
                  }

                  const updatedSet = new Set<string>()

                  // 2. Process Updates (Flat 1D layout: [id1, up1, down1, id2, up2, down2, ...])
                  const updated = delta.updated ?? []
                  for (let i = 0; i < updated.length; i += 3) {
                    const id = updated[i] as string
                    const upload = updated[i + 1] as number
                    const download = updated[i + 2] as number
                    const conn = activeMap.get(id)
                    if (conn) {
                      conn.curUpload = upload - conn.upload
                      conn.curDownload = download - conn.download
                      conn.upload = upload
                      conn.download = download
                      updatedSet.add(id)
                    }
                  }

                  for (let i = 0; i < previousActive.length; i++) {
                    const id = previousActive[i].id
                    if (!updatedSet.has(id)) {
                      const conn = activeMap.get(id)
                      if (conn) {
                        conn.curUpload = 0
                        conn.curDownload = 0
                      }
                    }
                  }

                  // 3. Process Additions
                  const added = delta.added ?? []
                  for (let i = 0; i < added.length; i++) {
                    const conn = added[i]
                    activeMap.set(conn.id, {
                      ...conn,
                      curUpload: 0,
                      curDownload: 0,
                    })
                  }

                  // 4. Optimize Reference Stability to prevent unnecessary React re-renders
                  const activeConnections: IConnectionsItem[] = []
                  for (let i = 0; i < previousActive.length; i++) {
                    const prev = previousActive[i]
                    const nextConn = activeMap.get(prev.id)
                    if (nextConn) {
                      if (
                        prev.upload === nextConn.upload &&
                        prev.download === nextConn.download &&
                        prev.curUpload === 0 &&
                        nextConn.curUpload === 0 &&
                        prev.curDownload === 0 &&
                        nextConn.curDownload === 0
                      ) {
                        activeConnections.push(prev)
                      } else {
                        activeConnections.push(nextConn)
                      }
                      activeMap.delete(prev.id)
                    }
                  }

                  for (const conn of activeMap.values()) {
                    activeConnections.push(conn)
                  }

                  // 5. Merge Closed Connections History
                  const rawClosedLen = previousClosed.length + dropped.length
                  let closedConnections: IConnectionsItem[]
                  if (rawClosedLen <= MAX_CLOSED_CONNS_NUM) {
                    closedConnections = previousClosed.concat(dropped)
                  } else {
                    const skipPrev = rawClosedLen - MAX_CLOSED_CONNS_NUM
                    closedConnections =
                      skipPrev >= previousClosed.length
                        ? dropped.slice(skipPrev - previousClosed.length)
                        : previousClosed.slice(skipPrev).concat(dropped)
                  }

                  return {
                    uploadTotal: delta.uploadTotal ?? 0,
                    downloadTotal: delta.downloadTotal ?? 0,
                    activeConnections,
                    closedConnections,
                  }
                }

                return old
              })
            } catch (err) {
              console.error('Failed to parse connections diff:', err)
              next(err)
            }
          },
        }
      },
    })



  const clearClosedConnections = () => {
    if (!subscriptionCacheKey) return
    queryClient.setQueryData<ConnectionMonitorData>([subscriptionCacheKey], {
      uploadTotal: response.data?.uploadTotal ?? 0,
      downloadTotal: response.data?.downloadTotal ?? 0,
      activeConnections: response.data?.activeConnections ?? [],
      closedConnections: [],
    })
  }

  return {
    response,
    refreshGetClashConnection: refresh,
    clearClosedConnections,
  }
}
