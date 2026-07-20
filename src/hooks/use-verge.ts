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
      }

      // refetch 确保前端状态与后端一致；失败时重试 3 次（间隔 1s/2s/4s）
      for (let attempt = 0; attempt < 3; attempt++) {
        try {
          await mutateVerge()
          return
        } catch (refetchErr) {
          if (attempt < 2) {
            await new Promise((r) => setTimeout(r, 1000 << attempt))
          } else {
            console.error('[useVerge] refetch 3 次均失败:', refetchErr)
            showNotice.error('配置已更新但刷新失败，状态可能不一致，请重启应用')
          }
        }
      }
    },
  )

  return {
    verge,
    mutateVerge,
    patchVerge,
  }
}
