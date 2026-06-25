import type { TrafficWorkerRequestMessage } from '@/types/traffic'

import { TrafficDataSampler } from '../utils/traffic-sampler'

let sampler: TrafficDataSampler | null = null
let config = {
  rawDataMinutes: 10,
  compressedDataMinutes: 60,
  compressionRatio: 5,
  snapshotIntervalMs: 3000,
  defaultRangeMinutes: 10,
}
let currentRange = 10
let throttleTimer: ReturnType<typeof setTimeout> | null = null
let lastTimestamp: number | undefined

const emitSnapshot = (reason: string) => {
  if (!sampler) return
  const dataPoints = sampler.getDataForTimeRange(currentRange)
  const availableDataPoints = sampler.getDataForTimeRange(
    config.compressedDataMinutes,
  )

  postMessage({
    type: 'snapshot',
    dataPoints,
    availableDataPoints,
    samplerStats: sampler.getStats(),
    rangeMinutes: currentRange,
    lastTimestamp,
    reason,
  })
}

const scheduleSnapshot = (reason: string) => {
  if (throttleTimer !== null) return
  throttleTimer = setTimeout(() => {
    throttleTimer = null
    emitSnapshot(reason)
  }, config.snapshotIntervalMs)
}

self.onmessage = (event) => {
  const message = event.data as TrafficWorkerRequestMessage
  if (!message) return

  switch (message.type) {
    case 'init': {
      config = { ...message.config }
      sampler = new TrafficDataSampler(config)
      currentRange = message.config.defaultRangeMinutes
      emitSnapshot('init')
      break
    }
    case 'append': {
      if (!sampler) return
      const timestamp = message.payload.timestamp ?? Date.now()
      const dataPoint = {
        up: message.payload.up || 0,
        down: message.payload.down || 0,
        timestamp,
        name: `${new Date(timestamp).getHours().toString().padStart(2, '0')}:${new Date(timestamp).getMinutes().toString().padStart(2, '0')}:${new Date(timestamp).getSeconds().toString().padStart(2, '0')}`,
      }

      lastTimestamp = timestamp
      sampler.addDataPoint(dataPoint)
      scheduleSnapshot('append-throttle')
      break
    }
    case 'clear': {
      if (sampler) {
        sampler.clear()
      }
      lastTimestamp = undefined
      emitSnapshot('clear')
      break
    }
    case 'setRange': {
      if (currentRange !== message.minutes) {
        currentRange = message.minutes
        emitSnapshot('range-change')
      }
      break
    }
    case 'requestSnapshot': {
      emitSnapshot('request')
      break
    }
  }
}
