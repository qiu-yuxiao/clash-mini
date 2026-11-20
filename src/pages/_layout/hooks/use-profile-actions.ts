import { useLockFn } from 'ahooks'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useHeadStateNew } from '@/components/proxy/use-head-state'
import { useProfiles } from '@/hooks/use-profiles'
import { useAppRefreshers } from '@/providers/app-data-context'
import {
  deleteProfile,
  enhanceProfiles,
  getProfiles,
  importProfile,
  patchProfile,
  updateProfile,
  viewProfile,
} from '@/services/cmds'
import { getDelayManager } from '@/services/delay'
import { closeAllConnectionsWithTimeout } from '@/services/mihomo-api'
import { showNotice } from '@/services/notice-service'
import type { IProfileItem } from '@/types/profile'
import { hasRealProfileItem } from '@/utils/profile-items'

import { FIRST_RUN_GUIDE_STORAGE_KEY } from '../components/first-run-guide-dialog'
import {
  triggerAutoSelectAndRefresh,
  waitForClashReady,
} from '../utils/profile-coordination'

export function useProfileActions() {
  const { t } = useTranslation()
  const {
    profiles = {},
    mutateProfiles,
    activateSelected,
    patchProfiles,
  } = useProfiles()
  const { refreshProxy } = useAppRefreshers()
  const [, setHeadStateForSort] = useHeadStateNew()

  const [url, setUrl] = useState('')
  const [profileLoading, setProfileLoading] = useState(false)
  const [profileMenuAnchorPosition, setProfileMenuAnchorPosition] = useState<{
    top: number
    left: number
  } | null>(null)
  const [contextMenuProfileUid, setContextMenuProfileUid] = useState<
    string | null
  >(null)
  const [deleteConfirmProfile, setDeleteConfirmProfile] = useState<{
    uid: string
    name: string
  } | null>(null)

  const [editProfileOpen, setEditProfileOpen] = useState(false)
  const [editProfileUid, setEditProfileUid] = useState<string | null>(null)
  const [editProfileName, setEditProfileName] = useState('')
  const [editProfileUrl, setEditProfileUrl] = useState('')
  const [editProfileInterval, setEditProfileInterval] = useState(0)
  const [firstRunGuideOpen, setFirstRunGuideOpen] = useState(false)

  const profileItems = useMemo(
    () =>
      (profiles.items || []).filter(
        (item) =>
          item &&
          ['local', 'remote'].includes(item.type || '') &&
          item.name &&
          item.name.trim() !== '',
      ),
    [profiles],
  )
  const currentProfileUid = profiles.current

  const contextMenuTargetItem = useMemo(
    () => profileItems.find((p) => p.uid === contextMenuProfileUid),
    [profileItems, contextMenuProfileUid],
  )
  const isContextMenuLocal = contextMenuTargetItem?.type === 'local'

  const editProfileTargetItem = useMemo(
    () => profileItems.find((p) => p.uid === editProfileUid),
    [profileItems, editProfileUid],
  )
  const isEditProfileLocal = editProfileTargetItem?.type === 'local'

  const activateSelectedRef = useRef(activateSelected)
  useEffect(() => {
    activateSelectedRef.current = activateSelected
  }, [activateSelected])

  const refreshProxyRef = useRef(refreshProxy)
  refreshProxyRef.current = refreshProxy
  const setHeadStateForSortRef = useRef(setHeadStateForSort)
  setHeadStateForSortRef.current = setHeadStateForSort
  const tRef = useRef(t)
  tRef.current = t

  // 首次运行向导检测：无任何真实订阅且未完成向导时，自动展示引导弹窗
  useEffect(() => {
    const guided = localStorage.getItem(FIRST_RUN_GUIDE_STORAGE_KEY) === 'true'
    if (guided || !profiles) return
    if (hasRealProfileItem(profiles.items)) return

    const timer = setTimeout(() => {
      setFirstRunGuideOpen(true)
    }, 500)
    return () => clearTimeout(timer)
  }, [profiles])

  const [profileRefreshCounter, setProfileRefreshCounter] = useState(0)
  const lastProcessedRef = useRef<{ uid: string | null; counter: number }>({
    uid: null,
    counter: -1,
  })
  const startupRetryCountRef = useRef(0)
  const isImportingRef = useRef(false)

  // 订阅加载/切换时自动增强（收敛为单一 PROXY 组）；轻量唤醒跳过 enhance，仅按需刷新延迟
  useEffect(() => {
    if (!currentProfileUid) return

    const enhancedUid = localStorage.getItem('clash-mini-last-enhanced-uid')
    const isNewProfile = enhancedUid !== currentProfileUid
    const isRefreshTriggered =
      lastProcessedRef.current.uid !== null &&
      lastProcessedRef.current.counter !== profileRefreshCounter

    if (!isNewProfile && !isRefreshTriggered) {
      lastProcessedRef.current = {
        uid: currentProfileUid,
        counter: profileRefreshCounter,
      }

      let cancelled = false
      ;(async () => {
        try {
          if (cancelled || isImportingRef.current) return
          await activateSelectedRef.current()
          if (cancelled || isImportingRef.current) return
          await waitForClashReady(tRef.current)
          if (cancelled || isImportingRef.current) return
          await triggerAutoSelectAndRefresh(
            refreshProxyRef.current,
            setHeadStateForSortRef.current,
            currentProfileUid,
            { skipProbeIfFresh: true },
          )
          startupRetryCountRef.current = 0
        } catch (err) {
          if (!cancelled) {
            console.error('[Layout] 唤醒后恢复节点选择失败:', err)
          }
        }
      })()
      return () => {
        cancelled = true
      }
    }

    lastProcessedRef.current = {
      uid: currentProfileUid,
      counter: profileRefreshCounter,
    }
    const uid = currentProfileUid
    let cancelled = false
    let timerId: ReturnType<typeof setTimeout> | null = null
    enhanceProfiles()
      .then(async (success) => {
        if (!success) {
          throw new Error('Profile configuration validation failed')
        }
        if (cancelled || isImportingRef.current) return
        localStorage.setItem('clash-mini-last-enhanced-uid', uid)
        await activateSelectedRef.current()
        if (cancelled || isImportingRef.current) return
        await waitForClashReady(tRef.current)
        if (cancelled || isImportingRef.current) return
        await triggerAutoSelectAndRefresh(
          refreshProxyRef.current,
          setHeadStateForSortRef.current,
          uid,
        )
        startupRetryCountRef.current = 0
      })
      .catch((err) => {
        if (cancelled) return
        console.error(`[Layout] Failed to enhance profile ${uid}:`, err)
        lastProcessedRef.current.uid = null

        if (startupRetryCountRef.current < 3) {
          startupRetryCountRef.current += 1
          const retryDelay = 2000 * startupRetryCountRef.current
          timerId = setTimeout(() => {
            if (!cancelled) {
              setProfileRefreshCounter((c) => c + 1)
            }
          }, retryDelay)
        } else {
          showNotice.error('layout.notices.profileActivationFailed')
        }
      })
    return () => {
      cancelled = true
      if (timerId) {
        clearTimeout(timerId)
      }
      lastProcessedRef.current.uid = null
    }
  }, [currentProfileUid, profileRefreshCounter])

  const handleImportProfile = async (inputUrl?: string): Promise<boolean> => {
    const rawUrl = typeof inputUrl === 'string' ? inputUrl : url
    if (!rawUrl) return false
    const trimmed = rawUrl.trim()
    if (!trimmed || isImportingRef.current) return false

    const finalizeImportedProfile = async (
      noticeKey:
        | 'shared.feedback.notifications.importSuccess'
        | 'shared.feedback.notifications.importWithClashProxy',
    ) => {
      showNotice.success(noticeKey)
      if (!inputUrl) setUrl('')

      const freshConfig = await getProfiles()
      const newProfile = freshConfig?.items?.find(
        (p: IProfileItem) => p.url === trimmed,
      )
      let targetUid = currentProfileUid
      if (newProfile) {
        if (freshConfig.current !== newProfile.uid) {
          const enhanced = await patchProfiles({ current: newProfile.uid })
          if (enhanced) {
            localStorage.setItem('clash-mini-last-enhanced-uid', newProfile.uid)
            lastProcessedRef.current = {
              uid: newProfile.uid,
              counter: profileRefreshCounter,
            }
          }
        } else {
          localStorage.setItem('clash-mini-last-enhanced-uid', newProfile.uid)
          lastProcessedRef.current = {
            uid: newProfile.uid,
            counter: profileRefreshCounter,
          }
        }
        targetUid = newProfile.uid
      }

      isImportingRef.current = false
      await mutateProfiles()

      if (targetUid && targetUid === currentProfileUid) {
        lastProcessedRef.current.uid = null
        setProfileRefreshCounter((c) => c + 1)
      }
    }

    isImportingRef.current = true
    setProfileLoading(true)
    try {
      await importProfile(trimmed)
      await finalizeImportedProfile(
        'shared.feedback.notifications.importSuccess',
      )
      return true
    } catch (err) {
      console.error(
        '[handleImportProfile] 首次导入失败，尝试 Clash 代理重试:',
        err,
      )
      try {
        await importProfile(trimmed, { with_proxy: false, self_proxy: true })
        await finalizeImportedProfile(
          'shared.feedback.notifications.importWithClashProxy',
        )
        return true
      } catch (retryErr) {
        showNotice.error(
          'profiles.page.feedback.notifications.importFail',
          String(retryErr),
        )
        return false
      }
    } finally {
      isImportingRef.current = false
      setProfileLoading(false)
    }
  }

  const handleSelectProfile = useLockFn(async (uid: string) => {
    if (currentProfileUid === uid) return
    try {
      startupRetryCountRef.current = 0
      getDelayManager().clearCache()
      const enhanced = await patchProfiles({ current: uid })
      if (enhanced) {
        localStorage.setItem('clash-mini-last-enhanced-uid', uid)
        lastProcessedRef.current = { uid, counter: profileRefreshCounter }
      }
      await mutateProfiles()
      closeAllConnectionsWithTimeout().catch((err) =>
        console.warn('[Layout] closeAllConnectionsWithTimeout failed:', err),
      )
      showNotice.success(
        'profiles.page.feedback.notifications.profileSwitched',
        1000,
      )
    } catch (err) {
      showNotice.error(err)
    }
  })

  const handleUpdateProfile = async (uid: string, e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      showNotice.info('layout.notices.updatingSubscription')
      await updateProfile(uid)
      if (uid === currentProfileUid) {
        lastProcessedRef.current.uid = null
        setProfileRefreshCounter((c) => c + 1)
      }
      await mutateProfiles()
      showNotice.success('layout.notices.subscriptionUpdated')
    } catch (err) {
      showNotice.error(err)
    }
  }

  const handleDeleteProfile = (uid: string, e: React.MouseEvent) => {
    e.stopPropagation()
    const targetItem = profileItems.find((p) => p.uid === uid)
    setDeleteConfirmProfile({
      uid,
      name: targetItem?.name || uid,
    })
  }

  const handleConfirmDeleteProfile = async () => {
    if (!deleteConfirmProfile) return
    const { uid } = deleteConfirmProfile
    setDeleteConfirmProfile(null)
    try {
      await deleteProfile(uid)
      await mutateProfiles()
      showNotice.success('shared.feedback.notifications.common.deleteSuccess')
    } catch (err) {
      showNotice.error(err)
    }
  }

  const handleEditProfileClick = () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    const targetItem = profileItems.find((p) => p.uid === contextMenuProfileUid)
    if (!targetItem) return
    setEditProfileUid(contextMenuProfileUid)
    setEditProfileName(targetItem.name || '')
    setEditProfileUrl(targetItem.url || '')
    setEditProfileInterval(targetItem.option?.update_interval || 0)
    setEditProfileOpen(true)
  }

  const handleEditProfileFileClick = async () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    try {
      await viewProfile(contextMenuProfileUid)
    } catch (err) {
      showNotice.error(String(err))
    }
  }

  const handleCopyProfileLinkClick = async () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    const targetItem = profileItems.find((p) => p.uid === contextMenuProfileUid)
    if (!targetItem?.url) return
    try {
      await navigator.clipboard.writeText(targetItem.url)
      showNotice.success('layout.notices.profileLinkCopied')
    } catch {
      showNotice.error('layout.notices.copyFailed')
    }
  }

  const handleUpdateProfileClick = async () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    await handleUpdateProfile(contextMenuProfileUid, {
      stopPropagation: () => {},
    } as React.MouseEvent)
  }

  const handleDeleteProfileClick = () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    handleDeleteProfile(contextMenuProfileUid, {
      stopPropagation: () => {},
    } as React.MouseEvent)
  }

  const handleSaveProfile = async () => {
    if (!editProfileUid) return
    try {
      const targetItem = profileItems.find((p) => p.uid === editProfileUid)
      const origOption = targetItem?.option || {}
      await patchProfile(editProfileUid, {
        name: editProfileName,
        url: editProfileUrl,
        option: {
          ...origOption,
          update_interval: Number(editProfileInterval) || 0,
        },
      })
      showNotice.success('layout.notices.profileSaved')
      setEditProfileOpen(false)
      await mutateProfiles()
      if (editProfileUid === currentProfileUid) {
        lastProcessedRef.current.uid = null
        setProfileRefreshCounter((c) => c + 1)
      }
    } catch (err) {
      showNotice.error(String(err))
    }
  }

  return {
    url,
    setUrl,
    profileLoading,
    profileItems,
    currentProfileUid,
    activateSelected,
    profileMenuAnchorPosition,
    setProfileMenuAnchorPosition,
    contextMenuTargetItem,
    isContextMenuLocal,
    setContextMenuProfileUid,
    deleteConfirmProfile,
    setDeleteConfirmProfile,
    editProfileOpen,
    setEditProfileOpen,
    editProfileName,
    setEditProfileName,
    editProfileUrl,
    setEditProfileUrl,
    editProfileInterval,
    setEditProfileInterval,
    isEditProfileLocal,
    firstRunGuideOpen,
    setFirstRunGuideOpen,
    handleImportProfile,
    handleSelectProfile,
    handleUpdateProfile,
    handleDeleteProfile,
    handleConfirmDeleteProfile,
    handleEditProfileClick,
    handleEditProfileFileClick,
    handleCopyProfileLinkClick,
    handleUpdateProfileClick,
    handleDeleteProfileClick,
    handleSaveProfile,
  }
}
