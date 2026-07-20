import { useEffect } from 'react'

import { useListen } from '@/hooks/use-listen'
import { getDelayManager } from '@/services/delay'
import { queryClient } from '@/services/query-client'

export const useLayoutEvents = (
  handleNotice: (payload: [string, string]) => void,
) => {
  const { addListener } = useListen()

  useEffect(() => {
    const unlisteners: Array<() => void> = []
    let disposed = false
    const revalidateKeys = (keys: readonly string[]) => {
      keys.forEach((key) => {
        queryClient.invalidateQueries({ queryKey: [key] })
      })
    }

    const register = (
      maybeUnlisten: void | (() => void) | Promise<void | (() => void)>,
    ) => {
      if (!maybeUnlisten) return

      if (typeof maybeUnlisten === 'function') {
        unlisteners.push(maybeUnlisten)
        return
      }

      maybeUnlisten
        .then((unlisten) => {
          if (!unlisten) return
          if (disposed) {
            unlisten()
          } else {
            unlisteners.push(unlisten)
          }
        })
        .catch((error) =>
          console.error('[Event Listener] Registration failed:', error),
        )
    }

    register(
      addListener('verge://refresh-clash-config', async () => {
        revalidateKeys([
          'getProxies',
          'getVersion',
          'getClashConfig',
          'getProxyProviders',
        ])
      }),
    )

    register(
      addListener('verge://refresh-verge-config', () => {
        revalidateKeys([
          'getVergeConfig',
          'getSystemProxy',
          'getAutoProxy',
          'getRunningMode',
          'isServiceAvailable',
          'getSystemState',
        ])
      }),
    )

    register(
      addListener('verge://notice-message', ({ payload }) =>
        handleNotice(payload as [string, string]),
      ),
    )

    register(
      addListener<{ group: string; results: Array<[string, number]> }>(
        'verge://backend-delay-results',
        ({ payload }) => {
          if (payload.results && payload.results.length > 0) {
            getDelayManager().injectBatchResults(payload.group, payload.results)
          }
        },
      ),
    )

    return () => {
      disposed = true
      const errors: Error[] = []

      unlisteners.forEach((unlisten) => {
        try {
          unlisten()
        } catch (error) {
          errors.push(error instanceof Error ? error : new Error(String(error)))
        }
      })

      if (errors.length > 0) {
        console.error(
          `[Event Listener] Encountered ${errors.length} errors during cleanup:`,
          errors,
        )
      }

      unlisteners.length = 0
    }
  }, [addListener, handleNotice])
}
