import {
  check,
  type Update,
  type DownloadEvent,
} from '@tauri-apps/plugin-updater'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { isSameVersion } from '@/pages/_layout/utils/style-helpers'
import { withIpcTimeout } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

interface UseClientUpdateParams {
  appVersion: string
  setHelpAnchorEl: (el: HTMLElement | null) => void
}

export function useClientUpdate({
  appVersion,
  setHelpAnchorEl,
}: UseClientUpdateParams) {
  const { t } = useTranslation()
  const [clientUpdateOpen, setClientUpdateOpen] = useState(false)
  const [clientUpdateObj, setClientUpdateObj] = useState<Update | null>(null)
  const [clientStatus, setClientStatus] = useState<
    'idle' | 'downloading' | 'error' | 'done'
  >('idle')
  const [clientProgress, setClientProgress] = useState(0)
  const [clientProgressMessage, setClientProgressMessage] = useState('')
  const [clientCheckLoading, setClientCheckLoading] = useState(false)

  const handleClientCheck = async () => {
    setClientCheckLoading(true)
    setHelpAnchorEl(null)
    try {
      const update = await check()
      if (update) {
        setClientUpdateObj(update)
        setClientUpdateOpen(true)
        setClientStatus('idle')
        setClientProgress(0)
        setClientProgressMessage('')
      } else {
        showNotice.info('layout.notices.clientUpToDate')
      }
    } catch (err: unknown) {
      console.error('Failed to check for client update:', err)
      showNotice.error('layout.notices.clientCheckFailed', {
        message: err instanceof Error ? err.message : String(err),
      })
    } finally {
      setClientCheckLoading(false)
    }
  }

  const handleClientUpgrade = async () => {
    if (clientUpdateObj && isSameVersion(appVersion, clientUpdateObj.version)) {
      showNotice.info('layout.notices.clientUpToDate')
      setClientUpdateOpen(false)
      return
    }
    if (!clientUpdateObj) return
    setClientStatus('downloading')
    setClientProgress(0)
    setClientProgressMessage(
      t('layout.components.dialogs.progress.clientDownloading'),
    )

    try {
      let downloaded = 0
      let total = 0
      const downloadPromise = clientUpdateObj.downloadAndInstall(
        (progressEvent: DownloadEvent) => {
          if (progressEvent.event === 'Started') {
            total = progressEvent.data.contentLength || 0
            setClientProgressMessage(
              t('layout.components.dialogs.progress.clientDownloadStarted'),
            )
          } else if (progressEvent.event === 'Progress') {
            downloaded += progressEvent.data.chunkLength
            if (total > 0) {
              const pct = Math.round((downloaded / total) * 100)
              setClientProgress(pct)
              setClientProgressMessage(
                t('layout.components.dialogs.progress.clientDownloadProgress', {
                  percent: pct,
                  downloaded: (downloaded / 1024 / 1024).toFixed(2),
                  total: (total / 1024 / 1024).toFixed(2),
                }),
              )
            } else {
              setClientProgressMessage(
                t(
                  'layout.components.dialogs.progress.clientDownloadProgressNoTotal',
                  { downloaded: (downloaded / 1024 / 1024).toFixed(2) },
                ),
              )
            }
          } else if (progressEvent.event === 'Finished') {
            setClientProgress(100)
            setClientStatus('done')
            setClientProgressMessage(
              t('layout.components.dialogs.progress.clientDownloadFinished'),
            )
          }
        },
      )
      await withIpcTimeout(
        downloadPromise,
        10 * 60 * 1000,
        'downloadAndInstall',
      )
      showNotice.success('layout.notices.clientInstallCompleted')
    } catch (err: unknown) {
      console.error('Client update error:', err)
      setClientStatus('error')
      const errMsg = err instanceof Error ? err.message : String(err)
      setClientProgressMessage(
        t('layout.notices.clientUpdateFailed', { message: errMsg }),
      )
      showNotice.error('layout.notices.clientUpdateFailed', {
        message: errMsg,
      })
    }
  }

  return {
    clientUpdateOpen,
    setClientUpdateOpen,
    clientUpdateObj,
    clientStatus,
    clientProgress,
    clientProgressMessage,
    clientCheckLoading,
    handleClientCheck,
    handleClientUpgrade,
  }
}
