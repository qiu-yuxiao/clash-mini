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

      // patchVergeConfig 成功即说明后端已生效，refetch 只为同步前端状态。
      // mihomo reload 期间 IPC 通道阻塞必然 timeout，重试会反复弹 notice
      // 并加剧 IPC 风暴（TUN/系统代理切换时的卡死根因）。改为失败 1 次静默放弃，
      // 依赖后端 verge://refresh-verge-config 事件或下次用户操作自然同步状态。
      try {
        await mutateVerge()
      } catch (refetchErr) {
        console.error('[useVerge] refetch 失败，等待后端事件同步:', refetchErr)
      }
    },
  )

  return {
    verge,
    mutateVerge,
    patchVerge,
  }
}
