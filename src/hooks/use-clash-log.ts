import { useLocalStorage } from 'foxact/use-local-storage'

import type { IClashLog } from '@/types/clash'

const defaultClashLog: IClashLog = {
  enable: true,
  logLevel: 'info',
  logFilter: 'all',
  logOrder: 'asc',
}

export const useClashLog = () =>
  useLocalStorage<IClashLog>('clash-log', defaultClashLog, {
    serializer: JSON.stringify,
    deserializer: JSON.parse,
  })
