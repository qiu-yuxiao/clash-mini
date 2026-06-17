import { useLocalStorage } from 'foxact/use-local-storage'

import type { IConnectionSetting } from '@/types/connection'

const defaultConnectionSetting: IConnectionSetting = { layout: 'table' }

export const useConnectionSetting = () =>
  useLocalStorage<IConnectionSetting>(
    'connections-setting',
    defaultConnectionSetting,
    {
      serializer: JSON.stringify,
      deserializer: JSON.parse,
    },
  )
