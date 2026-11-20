import {
  check,
  type DownloadEvent,
  type Update,
} from '@tauri-apps/plugin-updater'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useClashInfo } from '@/hooks/use-clash'
import { useVerge } from '@/hooks/use-verge'
import { isSameVersion } from '@/pages/_layout/utils/style-helpers'
import { frontendLog, withIpcTimeout } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import {
  checkClientUpdateWithFallback,
  downloadAndInstallClientUpdateWithRetry,
  resolveLocalMixedProxyUrl,
} from '@/utils/client-update'

interface UseClientUpdateParams {
  appVersion: string
  setHelpAnchorEl: (el: HTMLElement | null) => void
}

export function useClientUpdate({
  appVersion,
  setHelpAnchorEl,
}: UseClientUpdateParams) {
  const { t } = useTranslation()
  const { verge } = useVerge()
  const { clashInfo } = useClashInfo()
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
      const localProxyUrl = resolveLocalMixedProxyUrl(
        verge?.verge_mixed_port,
        clashInfo?.mixed_port,
      )
      const update = await checkClientUpdateWithFallback({
        localProxyUrl,
        checkFn: (options) => check(options),
        logError: (msg) => frontendLog('error', msg),
      })
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
      const downloadPromise = downloadAndInstallClientUpdateWithRetry({
        initialUpdate: clientUpdateObj,
        onAttemptReset: () => {
          downloaded = 0
          total = 0
          setClientProgress(0)
          setClientProgressMessage(
            t('layout.components.dialogs.progress.clientDownloading'),
          )
        },
        onEvent: (progressEvent: DownloadEvent) => {
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
        recheckDirectFn: () => check({ timeout: 20_000 }),
        logError: (msg) => frontendLog('error', msg),
      })
      await withIpcTimeout(
        downloadPromise,
        10 * 60 * 1000,
        'downloadAndInstall',
      )
      showNotice.success('layout.notices.clientInstallCompleted')
    } catch (err: unknown) {
      console.error('Client update error:', err)
      const errMsg = err instanceof Error ? err.message : String(err)
      frontendLog('error', `[ClientUpdate] 客户端升级失败: ${errMsg}`)
      setClientStatus('error')
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
