import { getConnections, MihomoWebSocket, Traffic } from 'tauri-plugin-mihomo-api'

import { useMihomoWsSubscription } from './use-mihomo-ws-subscription'
import { useTrafficMonitorEnhanced } from './use-traffic-monitor'
import { useVisibility } from './use-visibility'

const FALLBACK_TRAFFIC: Traffic = { up: 0, down: 0 }
const DUPLICATE_TRAFFIC_WINDOW_MS = 50

let lastTrafficSignature = ''
let lastTrafficTimestamp = 0

const shouldSkipDuplicateTraffic = (traffic: Traffic) => {
  const now = Date.now()
  const signature = `${traffic.up}:${traffic.down}`

  if (
    signature === lastTrafficSignature &&
    now - lastTrafficTimestamp <= DUPLICATE_TRAFFIC_WINDOW_MS
  ) {
    return true
  }

  lastTrafficSignature = signature
  lastTrafficTimestamp = now
  return false
}

export const useTrafficData = (options?: { enabled?: boolean }) => {
  const enabled = options?.enabled ?? true
  const isVisible = useVisibility()
  const active = enabled && isVisible

  const {
    graphData: { appendData },
  } = useTrafficMonitorEnhanced({ subscribe: false, enabled: active })
  const { response, refresh } = useMihomoWsSubscription<ITrafficItem>({
    storageKey: 'mihomo_traffic_date',
    buildSubscriptKey: (date) => (active ? `getClashTraffic-${date}` : null),
    fallbackData: FALLBACK_TRAFFIC,
    connect: () => MihomoWebSocket.connect_traffic(),
    throttleMs: 200,
    setupHandlers: ({ next, scheduleReconnect }) => {
      let activeUpTotal = 0
      let activeDownTotal = 0
      let initialized = false

      const init = async () => {
        try {
          const res = await getConnections()
          activeUpTotal = res.uploadTotal ?? 0
          activeDownTotal = res.downloadTotal ?? 0
          initialized = true
        } catch (err) {
          console.warn('[useTrafficData] Failed to fetch initial connection totals:', err)
        }
      }

      init()

      return {
        handleMessage: (data) => {
          if (data.startsWith('Websocket error')) {
            next(data, FALLBACK_TRAFFIC)
            void scheduleReconnect()
            return
          }

          try {
            const parsed = JSON.parse(data) as Traffic
            if (shouldSkipDuplicateTraffic(parsed)) {
              return
            }

            activeUpTotal += parsed.up || 0
            activeDownTotal += parsed.down || 0

            const trafficWithTotals: ITrafficItem = {
              ...parsed,
              upTotal: activeUpTotal,
              downTotal: activeDownTotal,
            }

            appendData(trafficWithTotals)
            next(null, trafficWithTotals)
          } catch (error) {
            next(error, FALLBACK_TRAFFIC)
          }
        },
      }
    },
  })

  return { response, refreshGetClashTraffic: refresh }
}
