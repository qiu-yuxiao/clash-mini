import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type {
  GithubRelease,
  CoreUpgradeProgressPayload,
} from '@/pages/_layout/types'
import { isSameVersion } from '@/pages/_layout/utils/style-helpers'
import { withIpcTimeout } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

interface UseCoreUpdateParams {
  coreVersion: string
  mutateVersion: () => void
  setHelpAnchorEl: (el: HTMLElement | null) => void
}

export function useCoreUpdate({
  coreVersion,
  mutateVersion,
  setHelpAnchorEl,
}: UseCoreUpdateParams) {
  const { t } = useTranslation()
  const [coreUpdateOpen, setCoreUpdateOpen] = useState(false)
  const [coreUpdateRelease, setCoreUpdateRelease] =
    useState<GithubRelease | null>(null)
  const [coreUpgradeStatus, setCoreUpgradeStatus] = useState<string>('idle')
  const [coreUpgradeProgress, setCoreUpgradeProgress] = useState<number>(0)
  const [coreUpgradeMessage, setCoreUpgradeMessage] = useState<string>('')
  const [coreCheckLoading, setCoreCheckLoading] = useState(false)

  useEffect(() => {
    let active = true
    const unlistenPromise = listen<CoreUpgradeProgressPayload>(
      'core-upgrade-progress',
      (event) => {
        if (!active) return
        const payload = event.payload
        setCoreUpgradeStatus(payload.status)
        setCoreUpgradeProgress(payload.progress ?? 0)
        setCoreUpgradeMessage(payload.message ?? '')
        if (payload.status === 'done') {
          showNotice.success('layout.notices.coreUpdateSuccess')
          mutateVersion()
        } else if (payload.status === 'error') {
          showNotice.error('layout.notices.coreUpdateFailed', {
            message: payload.message,
          })
        }
      },
    )
    return () => {
      active = false
      unlistenPromise
        .then((unlisten) => unlisten())
        .catch((err) =>
          console.warn('Failed to unlisten from core-upgrade-progress:', err),
        )
    }
  }, [mutateVersion])

  const handleCoreCheck = async () => {
    setCoreCheckLoading(true)
    setHelpAnchorEl(null)
    try {
      const release = await invoke<GithubRelease>('check_core_update')
      if (isSameVersion(coreVersion, release.tag_name)) {
        showNotice.info('layout.notices.coreUpToDate')
        return
      }
      setCoreUpdateRelease(release)
      setCoreUpdateOpen(true)
      setCoreUpgradeStatus('idle')
      setCoreUpgradeProgress(0)
      setCoreUpgradeMessage('')
    } catch (err: unknown) {
      console.error('Failed to check for core update:', err)
      showNotice.error('layout.notices.coreCheckFailed', {
        message: err instanceof Error ? err.message : String(err),
      })
    } finally {
      setCoreCheckLoading(false)
    }
  }

  const handleCoreUpgrade = async () => {
    if (
      coreUpdateRelease &&
      isSameVersion(coreVersion, coreUpdateRelease.tag_name)
    ) {
      showNotice.info('layout.notices.coreUpToDate')
      setCoreUpdateOpen(false)
      return
    }
    if (!coreUpdateRelease) return
    setCoreUpgradeStatus('checking')
    setCoreUpgradeProgress(0)
    setCoreUpgradeMessage(t('layout.components.dialogs.progress.coreStarting'))
    try {
      await withIpcTimeout(
        invoke('start_core_upgrade', { release: coreUpdateRelease }),
        5 * 60 * 1000,
        'start_core_upgrade',
      )
    } catch (err: unknown) {
      console.error('Failed to start core upgrade:', err)
      setCoreUpgradeStatus('error')
      const errMsg = err instanceof Error ? err.message : String(err)
      setCoreUpgradeMessage(
        t('layout.components.dialogs.progress.coreStartFailed', {
          message: errMsg,
        }),
      )
      showNotice.error('layout.notices.coreUpgradeStartFailed', {
        message: errMsg,
      })
    }
  }

  return {
    coreUpdateOpen,
    setCoreUpdateOpen,
    coreUpdateRelease,
    coreUpgradeStatus,
    coreUpgradeProgress,
    coreUpgradeMessage,
    coreCheckLoading,
    handleCoreCheck,
    handleCoreUpgrade,
  }
}
