import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'

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
          showNotice.success('Mihomo 内核更新成功')
          mutateVersion()
        } else if (payload.status === 'error') {
          showNotice.error(`内核更新失败: ${payload.message}`)
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
        showNotice.info('当前内核已是最新版本')
        return
      }
      setCoreUpdateRelease(release)
      setCoreUpdateOpen(true)
      setCoreUpgradeStatus('idle')
      setCoreUpgradeProgress(0)
      setCoreUpgradeMessage('')
    } catch (err: unknown) {
      console.error('Failed to check for core update:', err)
      showNotice.error(
        `检查内核更新失败: ${err instanceof Error ? err.message : String(err)}`,
      )
    } finally {
      setCoreCheckLoading(false)
    }
  }

  const handleCoreUpgrade = async () => {
    if (
      coreUpdateRelease &&
      isSameVersion(coreVersion, coreUpdateRelease.tag_name)
    ) {
      showNotice.info('当前内核已是最新版本')
      setCoreUpdateOpen(false)
      return
    }
    if (!coreUpdateRelease) return
    setCoreUpgradeStatus('checking')
    setCoreUpgradeProgress(0)
    setCoreUpgradeMessage('正在启动内核升级任务...')
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
      setCoreUpgradeMessage(`启动失败: ${errMsg}`)
      showNotice.error(`启动内核升级失败: ${errMsg}`)
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
