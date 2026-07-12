import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useLockFn } from 'ahooks'

import { getVergeConfig, patchVergeConfig } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { getPreloadConfig, setPreloadConfig } from '@/services/preload'
import type { IVergeConfig } from '@/types/verge'

export const useVerge = () => {
  const qc = useQueryClient()
  const initialVergeConfig = getPreloadConfig()

  const { data: verge, refetch } = useQuery({
    queryKey: ['getVergeConfig'],
    queryFn: async () => {
      const config = await getVergeConfig()
      setPreloadConfig(config)
      return config
    },
    initialData: initialVergeConfig ?? undefined,
    staleTime: 5000,
  })

  const mutateVerge = async (
    updaterOrData?:
      | IVergeConfig
      | ((prev: IVergeConfig | undefined) => IVergeConfig | undefined)
      | undefined,
    revalidate?: boolean,
  ) => {
    if (updaterOrData === undefined) {
      await refetch()
      return
    }
    if (typeof updaterOrData === 'function') {
      const prev = qc.getQueryData<IVergeConfig>(['getVergeConfig'])
      const next = updaterOrData(prev)
      qc.setQueryData(['getVergeConfig'], next)
    } else {
      qc.setQueryData(['getVergeConfig'], updaterOrData)
    }
    if (revalidate !== false) {
      await refetch()
    }
  }

  const patchVerge = useLockFn(
    async (value: Partial<IVergeConfig>) => {
      try {
        await patchVergeConfig(value)
      } catch (err) {
        showNotice.error(err)
        throw err
      } finally {
        await mutateVerge()
      }
    },
  )

  return {
    verge,
    mutateVerge,
    patchVerge,
  }
}
