import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import {
  closeConnectionWithTimeout,
  getConnectionsWithTimeout,
} from '@/services/mihomo-api'
import type { IConnectionsItem } from '@/types/connection'
import {
  appendDisconnectedId,
  composeActivePathControlConnections,
  createDisconnectedRecordFromConnection,
  enrichClosedPathControlConnections,
  loadDisconnectedIds,
  loadDisconnectedRecords,
  matchesConnectionTarget,
  reconcileDisconnectedRecords,
  saveDisconnectedIds,
  saveDisconnectedRecords,
  upsertDisconnectedRecord,
  type DisconnectedTargetRecord,
  type PathControlConnectionItem,
  type PathControlTarget,
} from '@/utils/path-control'
import {
  MERGE_RULES_CHANGED_EVENT,
  readManagedMergeRules,
  type ManagedMergeRule,
} from '@/utils/quick-routing'

interface UsePathControlInput {
  activeConnections: IConnectionsItem[]
  closedConnections: IConnectionsItem[]
  enabled: boolean
}

export interface PathControlActions {
  markConnectionDisconnected: (conn: PathControlConnectionItem) => void
  clearDisconnectedTarget: (targetKey?: string, connId?: string) => void
  closeMatchingActiveConnections: (target: PathControlTarget) => Promise<void>
  clearAllDisconnectedRecords: () => void
  refreshManagedRules: () => Promise<void>
}

export function usePathControl({
  activeConnections,
  closedConnections,
  enabled,
}: UsePathControlInput) {
  const [managedRules, setManagedRules] = useState<ManagedMergeRule[]>([])
  const [disconnectedRecords, setDisconnectedRecords] = useState<
    DisconnectedTargetRecord[]
  >(() => loadDisconnectedRecords())
  const [disconnectedIds, setDisconnectedIds] = useState<string[]>(() =>
    loadDisconnectedIds(),
  )

  const activeConnectionsRef = useRef(activeConnections)
  activeConnectionsRef.current = activeConnections

  const refreshManagedRules = useCallback(async () => {
    const rules = await readManagedMergeRules()
    setManagedRules(rules)
  }, [])

  // Load Merge rules when enabled and whenever Merge or Clash config refreshes.
  useEffect(() => {
    if (!enabled) return
    void refreshManagedRules()

    const onMergeChanged = () => {
      void refreshManagedRules()
    }
    window.addEventListener(MERGE_RULES_CHANGED_EVENT, onMergeChanged)

    let unlistenConfig: UnlistenFn | null = null
    let disposed = false
    void listen('verge://refresh-clash-config', () => {
      void refreshManagedRules()
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten()
        } else {
          unlistenConfig = unlisten
        }
      })
      .catch((err) => {
        console.warn(
          'Failed to listen refresh-clash-config in path control:',
          err,
        )
      })

    return () => {
      disposed = true
      window.removeEventListener(MERGE_RULES_CHANGED_EVENT, onMergeChanged)
      unlistenConfig?.()
    }
  }, [enabled, refreshManagedRules])

  const effectiveDisconnectedRecords = useMemo(
    () => reconcileDisconnectedRecords(disconnectedRecords, activeConnections),
    [disconnectedRecords, activeConnections],
  )

  // Persist pruned disconnected list to localStorage without triggering extra React renders.
  useEffect(() => {
    if (effectiveDisconnectedRecords !== disconnectedRecords) {
      saveDisconnectedRecords(effectiveDisconnectedRecords)
    }
  }, [effectiveDisconnectedRecords, disconnectedRecords])

  const disconnectedIdSet = useMemo(
    () => new Set(disconnectedIds),
    [disconnectedIds],
  )

  const activeControlConnections = useMemo(
    () =>
      composeActivePathControlConnections({
        activeConnections,
        managedRules,
        disconnectedRecords: effectiveDisconnectedRecords,
        disconnectedIds: disconnectedIdSet,
      }),
    [
      activeConnections,
      managedRules,
      effectiveDisconnectedRecords,
      disconnectedIdSet,
    ],
  )

  const closedControlConnections = useMemo(
    () =>
      enrichClosedPathControlConnections({
        closedConnections,
        managedRules,
        disconnectedIds: disconnectedIdSet,
      }),
    [closedConnections, managedRules, disconnectedIdSet],
  )

  const markConnectionDisconnected = useCallback(
    (conn: PathControlConnectionItem) => {
      const record = createDisconnectedRecordFromConnection(conn)
      if (record) {
        setDisconnectedRecords((prev) => {
          const pruned = reconcileDisconnectedRecords(
            prev,
            activeConnectionsRef.current,
          )
          const next = upsertDisconnectedRecord(pruned, record)
          saveDisconnectedRecords(next)
          return next
        })
      }
      if (conn.id && !conn.id.startsWith('synthetic:')) {
        setDisconnectedIds((prev) => {
          const next = appendDisconnectedId(prev, conn.id)
          saveDisconnectedIds(next)
          return next
        })
      }
    },
    [],
  )

  const clearDisconnectedTarget = useCallback(
    (targetKey?: string, connId?: string) => {
      if (targetKey) {
        setDisconnectedRecords((prev) => {
          const next = prev.filter((item) => item.key !== targetKey)
          if (next.length !== prev.length) {
            saveDisconnectedRecords(next)
          }
          return next
        })
      }
      if (connId) {
        setDisconnectedIds((prev) => {
          const next = prev.filter((id) => id !== connId)
          if (next.length !== prev.length) {
            saveDisconnectedIds(next)
          }
          return next
        })
      }
    },
    [],
  )

  const clearAllDisconnectedRecords = useCallback(() => {
    setDisconnectedRecords([])
    saveDisconnectedRecords([])
    setDisconnectedIds([])
    saveDisconnectedIds([])
  }, [])

  const closeMatchingActiveConnections = useCallback(
    async (target: PathControlTarget) => {
      let candidates = activeConnectionsRef.current
      try {
        const snapshot = await getConnectionsWithTimeout()
        if (Array.isArray(snapshot?.connections)) {
          candidates = snapshot.connections
        }
      } catch {
        // Fall back to latest WebSocket active connection snapshot.
      }

      const matchingIds = candidates
        .filter((conn) => matchesConnectionTarget(conn, target))
        .map((conn) => conn.id)
        .filter((id) => id && !id.startsWith('synthetic:'))

      if (matchingIds.length === 0) return

      await Promise.allSettled(
        matchingIds.map((id) => closeConnectionWithTimeout(id)),
      )
    },
    [],
  )

  const actions = useMemo<PathControlActions>(
    () => ({
      markConnectionDisconnected,
      clearDisconnectedTarget,
      closeMatchingActiveConnections,
      clearAllDisconnectedRecords,
      refreshManagedRules,
    }),
    [
      markConnectionDisconnected,
      clearDisconnectedTarget,
      closeMatchingActiveConnections,
      clearAllDisconnectedRecords,
      refreshManagedRules,
    ],
  )

  return {
    managedRules,
    activeControlConnections,
    closedControlConnections,
    actions,
  }
}
