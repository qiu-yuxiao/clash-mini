import { useCallback } from 'react'

import { installService, restartCore } from '@/services/cmds'
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

  const installServiceAndRestartCore = useCallback(
    async (options?: ServiceInstallerOptions) => {
      await executeWithErrorHandling(
        () => installService(),
        'settings.statuses.clashService.installing',
        'settings.feedback.notifications.clashService.installSuccess',
        options,
      )

      await executeWithErrorHandling(
        () => restartCore(),
        'settings.statuses.clash.restarting',
        'settings.feedback.notifications.clash.restartSuccess',
        options,
      )

      await mutateSystemState()
    },
    [mutateSystemState],
  )
  return { installServiceAndRestartCore }
}
