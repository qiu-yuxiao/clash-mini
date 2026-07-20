import { useState } from 'react'
import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater'
import { showNotice, withIpcTimeout } from '@/services/cmds'
import { isSameVersion } from '@/pages/_layout/utils/style-helpers'

interface UseClientUpdateParams {
  appVersion: string
  setHelpAnchorEl: (el: HTMLElement | null) => void
}

export function useClientUpdate({ appVersion, setHelpAnchorEl }: UseClientUpdateParams) {
  const [clientUpdateOpen, setClientUpdateOpen] = useState(false)
  const [clientUpdateObj, setClientUpdateObj] = useState<Update | null>(null)
  const [clientStatus, setClientStatus] = useState<'idle' | 'downloading' | 'error' | 'done'>('idle')
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
        showNotice.info('当前已是最新版本')
      }
    } catch (err: unknown) {
      console.error('Failed to check for client update:', err)
      showNotice.error(
        `检查更新失败: ${err instanceof Error ? err.message : String(err)}`,
      )
    } finally {
      setClientCheckLoading(false)
    }
  }

  const handleClientUpgrade = async () => {
    if (clientUpdateObj && isSameVersion(appVersion, clientUpdateObj.version)) {
      showNotice.info('当前已是最新版本')
      setClientUpdateOpen(false)
      return
    }
    if (!clientUpdateObj) return
    setClientStatus('downloading')
    setClientProgress(0)
    setClientProgressMessage('正在下载更新...')

    try {
      let downloaded = 0
      let total = 0
      const downloadPromise = clientUpdateObj.downloadAndInstall(
        (progressEvent: DownloadEvent) => {
          if (progressEvent.event === 'Started') {
            total = progressEvent.data.contentLength || 0
            setClientProgressMessage('开始下载软件更新包...')
          } else if (progressEvent.event === 'Progress') {
            downloaded += progressEvent.data.chunkLength
            if (total > 0) {
              const pct = Math.round((downloaded / total) * 100)
              setClientProgress(pct)
              setClientProgressMessage(
                `已下载 ${pct}% (${(downloaded / 1024 / 1024).toFixed(2)} MB / ${(total / 1024 / 1024).toFixed(2)} MB)`,
              )
            } else {
              setClientProgressMessage(
                `已下载 ${(downloaded / 1024 / 1024).toFixed(2)} MB`,
              )
            }
          } else if (progressEvent.event === 'Finished') {
            setClientProgress(100)
            setClientStatus('done')
            setClientProgressMessage('下载完成，正在准备安装并重启...')
          }
        },
      )
      await withIpcTimeout(
        downloadPromise,
        10 * 60 * 1000,
        'downloadAndInstall',
      )
      showNotice.success('更新安装完毕，请重启应用以应用更改')
    } catch (err: unknown) {
      console.error('Client update error:', err)
      setClientStatus('error')
      const errMsg = err instanceof Error ? err.message : String(err)
      setClientProgressMessage(`更新失败: ${errMsg}`)
      showNotice.error(`更新失败: ${errMsg}`)
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
