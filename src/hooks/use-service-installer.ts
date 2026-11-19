import { useCallback } from 'react'

import { installService } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import { useSystemState } from './use-system-state'

export interface ServiceInstallerOptions {
  silentError?: boolean
}

const executeWithErrorHandling = async (
  operation: () => Promise<void>,
  loadingKey: string,
  successKey?: string,
  options?: ServiceInstallerOptions,
) => {
  try {
    showNotice.info(loadingKey)
    await operation()
    if (successKey) {
      showNotice.success(successKey)
    }
  } catch (err) {
    if (!options?.silentError) {
      showNotice.error(err)
    }
    throw err
  }
}

export const useServiceInstaller = () => {
  const { mutateSystemState } = useSystemState()

  // 仅负责安装服务并同步状态，不再提前重启 Sidecar，由随后的 patchVerge 原子完成 Sidecar -> Service 切换（BUG-408）
  const installServiceAndSync = useCallback(
    async (options?: ServiceInstallerOptions) => {
      await executeWithErrorHandling(
        () => installService(),
        'settings.statuses.clashService.installing',
        'settings.feedback.notifications.clashService.installSuccess',
        options,
      )

      await mutateSystemState()
    },
    [mutateSystemState],
  )
  return { installServiceAndSync }
}
