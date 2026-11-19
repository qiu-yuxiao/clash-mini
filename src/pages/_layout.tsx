// Copyright (c) 2026 秋雨潇潇 <qiuyuxiao@gmail.com> (Portions relating to modifications)
// SPDX-License-Identifier: GPL-3.0-only

import {
  SettingsRounded as SettingsRoundedIcon,
  CloseRounded,
  PushPinRounded,
} from '@mui/icons-material'
import {
  Box,
  Paper,
  ThemeProvider,
  IconButton,
  Select,
  MenuItem,
  Menu,
  Divider,
  type SxProps,
  type Theme,
} from '@mui/material'
import { getVersion as getAppVersion } from '@tauri-apps/api/app'
import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ErrorBoundary } from 'react-error-boundary'
import { useTranslation } from 'react-i18next'

import AppIcon from '@/assets/image/app-icon.png'
import { AreaErrorFallback } from '@/components/base/base-error-boundary'
import { NoticeManager } from '@/components/layout/notice-manager'
import { ResizeHandles } from '@/components/layout/resize-handles'
import { WindowControls } from '@/components/layout/window-controller'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { useHeadStateNew } from '@/components/proxy/use-head-state'
import {
  CUSTOM_TITLEBAR_HEIGHT,
  SETTINGS_CLOSE_BUTTON_Z_INDEX,
} from '@/constants'
import { useClashInfo, useClash } from '@/hooks/use-clash'
import { useI18n } from '@/hooks/use-i18n'
import { useProfiles } from '@/hooks/use-profiles'
import { useServiceInstaller } from '@/hooks/use-service-installer'
import { useSystemState } from '@/hooks/use-system-state'
import { useVerge } from '@/hooks/use-verge'
import { useWindowDecorations } from '@/hooks/use-window'
import {
  useClashConfigData,
  useAppRefreshers,
  useSystemData,
} from '@/providers/app-data-context'
import {
  importProfile,
  updateProfile,
  deleteProfile,
  enhanceProfiles,
  patchClashMode,
  getProfiles,
  patchProfile,
  viewProfile,
  beginLongOperation,
  endLongOperation,
  openWebUrl,
} from '@/services/cmds'
import { getDelayManager } from '@/services/delay'
import { closeAllConnectionsWithTimeout } from '@/services/mihomo-api'
import {
  showNotice,
  markLoadingNotice,
  settleLoadingNotice,
} from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import type { IProfileItem } from '@/types/profile'
import { get3DButtonStyle, get3DCardStyle } from '@/utils/button-styles'
import { bindMainAppShortcuts } from '@/utils/disable-webview-shortcuts'
import {
  getPopupMenuItemStyle,
  getPopupMenuPaperProps,
} from '@/utils/popup-styles'
import { hasRealProfileItem } from '@/utils/profile-items'

// Sub-components
import { ActiveNodeStatusCard } from './_layout/components/active-node-card'
import { AddUrlDialog } from './_layout/components/add-url-dialog'
import { BasicSettingsCard } from './_layout/components/basic-settings-card'
import { ConnectionsPanel } from './_layout/components/connections-panel'
import {
  FirstRunGuideDialog,
  CLASH_MINI_GITHUB_URL,
  CLASH_MINI_WIKI_URL,
  FIRST_RUN_GUIDE_STORAGE_KEY,
} from './_layout/components/first-run-guide-dialog'
import { HelpMenuButton } from './_layout/components/help-menu-button'
import { LayoutDialogs } from './_layout/components/layout-dialogs'
import { MiniTrafficPanel } from './_layout/components/mini-traffic-panel'
import { ProfileImportCard } from './_layout/components/profile-import-card'
import { RoutingPreferenceCard } from './_layout/components/routing-preference-card'
import { TakeoverModeCard } from './_layout/components/takeover-mode-card'
import { ThemeSettingsCard } from './_layout/components/theme-settings-card'
import {
  useCustomTheme,
  useLayoutEvents,
  useLoadingOverlay,
} from './_layout/hooks'
import { useClientUpdate } from './_layout/hooks/use-client-update'
import { useCoreUpdate } from './_layout/hooks/use-core-update'
import { useImportContextMenu } from './_layout/hooks/use-import-context-menu'
import { useMinimalSettings } from './_layout/hooks/use-minimal-settings'
import { useSkinControls } from './_layout/hooks/use-skin-controls'
import { handleNoticeMessage } from './_layout/utils'
import {
  waitForClashReady,
  triggerAutoSelectAndRefresh,
} from './_layout/utils/profile-coordination'
import { OS, buildImportant3DButtonStyle } from './_layout/utils/style-helpers'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)

// 底部常驻流量条(MiniTrafficPanel)的固定高度(px)。
// 这是面板自身的显示尺寸，与窗口极小高度开关阈值(MINI_HEIGHT_THRESHOLD=126)无关，
// 不应耦合（历史上曾与旧阈值 135 同值纯属巧合，窗口侧先后修正为 121、126 后两值已分离）。
const TRAFFIC_PANE_HEIGHT_WIDE = 135
const TRAFFIC_PANE_HEIGHT_MINIMAL = 100

const Layout = () => {
  // Active Skin State — managed by useSkinControls
  const {
    controlSkin,
    setControlSkin,
    depthFactor,
    vibrancyFactor,
    handleDepthFactorChange,
    commitDepthFactorChange,
    handleVibrancyFactorChange,
    commitVibrancyFactorChange,
  } = useSkinControls()

  // Update States
  const [helpAnchorEl, setHelpAnchorEl] = useState<null | HTMLElement>(null)
  const [appVersion, setAppVersion] = useState<string>('')

  // 必须在 useCoreUpdate 之前调用，以提供 coreVersion/mutateVersion
  const { version: coreVersion, mutateVersion } = useClash()

  const {
    clientUpdateOpen,
    setClientUpdateOpen,
    clientUpdateObj,
    clientStatus,
    clientProgress,
    clientProgressMessage,
    clientCheckLoading,
    handleClientCheck,
    handleClientUpgrade,
  } = useClientUpdate({ appVersion, setHelpAnchorEl })

  const {
    coreUpdateOpen,
    setCoreUpdateOpen,
    coreUpdateRelease,
    coreUpgradeStatus,
    coreUpgradeProgress,
    coreUpgradeMessage,
    coreCheckLoading,
    handleCoreCheck,
    handleCoreUpgrade,
  } = useCoreUpdate({ coreVersion, mutateVersion, setHelpAnchorEl })

  const mode = useThemeMode()
  const { t } = useTranslation()
  const { theme } = useCustomTheme(controlSkin)
  const { verge, patchVerge } = useVerge()
  const { language } = verge ?? {}
  const { switchLanguage, currentLanguage } = useI18n()
  const { decorated, isDecorationsHidden, isMinimalWidth, isMiniStatus } =
    useWindowDecorations()

  // Language Sync Ref to prevent deadlock/rollback loops
  const lastLanguageRef = useRef<string | undefined>(undefined)
  const switchLanguageRef = useRef(switchLanguage)
  useEffect(() => {
    switchLanguageRef.current = switchLanguage
  }, [switchLanguage])

  // Drawer Toggle State
  const { isSettingsOpen: drawerOpen, setIsSettingsOpen: setDrawerOpen } =
    useSystemData()

  // 设置抽屉打开状态下缩到最小窗口时自动关闭（避免遮挡主内容）
  useEffect(() => {
    if (drawerOpen && isMiniStatus) {
      setDrawerOpen(false)
    }
  }, [drawerOpen, isMiniStatus, setDrawerOpen])

  // Profiles State
  const [url, setUrl] = useState('')
  const [profileLoading, setProfileLoading] = useState(false)

  const {
    importInputContextMenu,
    setImportInputContextMenu,
    importInputRef,
    handlePaste: handleImportInputPaste,
    handleCopy: handleImportInputCopy,
    handleCut: handleImportInputCut,
    handleSelectAll: handleImportInputSelectAll,
    handleClear: handleImportInputClear,
  } = useImportContextMenu({ url, setUrl })

  // Context Menu State for Profile Card (BUG-072)
  const [profileMenuAnchorPosition, setProfileMenuAnchorPosition] = useState<{
    top: number
    left: number
  } | null>(null)
  const [contextMenuProfileUid, setContextMenuProfileUid] = useState<
    string | null
  >(null)

  // Delete Profile Confirmation Dialog State (BUG-390)
  const [deleteConfirmProfile, setDeleteConfirmProfile] = useState<{
    uid: string
    name: string
  } | null>(null)

  // BUG-385: 通过派生可见态与显式开关处理器统一收口抽屉内浮层菜单锚点，
  // 避免在 useEffect 中同步 setState，且在极小窗自动收起或手动关闭设置时同步清理。
  const isDrawerVisible = drawerOpen && !isMiniStatus
  const activeHelpAnchorEl = isDrawerVisible ? helpAnchorEl : null
  const activeProfileMenuAnchorPosition = isDrawerVisible
    ? profileMenuAnchorPosition
    : null
  const activeImportInputContextMenu = isDrawerVisible
    ? importInputContextMenu
    : null

  const resetDrawerMenuAnchors = useCallback(() => {
    setHelpAnchorEl(null)
    setProfileMenuAnchorPosition(null)
    setImportInputContextMenu(null)
  }, [setImportInputContextMenu])

  const openSettingsDrawer = useCallback(() => {
    resetDrawerMenuAnchors()
    setDrawerOpen(true)
  }, [resetDrawerMenuAnchors, setDrawerOpen])

  const closeSettingsDrawer = useCallback(() => {
    resetDrawerMenuAnchors()
    setDeleteConfirmProfile(null)
    setDrawerOpen(false)
  }, [resetDrawerMenuAnchors, setDrawerOpen])

  // Edit Profile Dialog State (BUG-072)
  const [editProfileOpen, setEditProfileOpen] = useState(false)
  const [editProfileUid, setEditProfileUid] = useState<string | null>(null)
  const [editProfileName, setEditProfileName] = useState('')
  const [editProfileUrl, setEditProfileUrl] = useState('')
  const [editProfileInterval, setEditProfileInterval] = useState(0)

  // First-run Onboarding Guide Dialog State
  const [firstRunGuideOpen, setFirstRunGuideOpen] = useState(false)

  const primaryBtn3DStyle = useMemo<SxProps<Theme>>(
    () => buildImportant3DButtonStyle(theme, 'primary'),
    [theme],
  )

  const defaultBtn3DStyle = useMemo<SxProps<Theme>>(
    () => buildImportant3DButtonStyle(theme, 'default'),
    [theme],
  )
  const {
    profiles = {},
    mutateProfiles,
    activateSelected,
    patchProfiles,
  } = useProfiles()
  const { refreshProxy, refreshAll } = useAppRefreshers()
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

  // Memoized profile and button styles to avoid IIFE syntax issues in JSX (BUG-072)
  const contextMenuTargetItem = useMemo(() => {
    return profileItems.find((p) => p.uid === contextMenuProfileUid)
  }, [profileItems, contextMenuProfileUid])
  const isContextMenuLocal = contextMenuTargetItem?.type === 'local'

  const editProfileTargetItem = useMemo(() => {
    return profileItems.find((p) => p.uid === editProfileUid)
  }, [profileItems, editProfileUid])
  const isEditProfileLocal = editProfileTargetItem?.type === 'local'

  // Stable reference to activateSelected to avoid infinite loops and satisfy ESLint / React Compiler
  const activateSelectedRef = useRef(activateSelected)
  useEffect(() => {
    activateSelectedRef.current = activateSelected
  }, [activateSelected])

  // 首次运行向导检测：无任何**真实订阅**且未完成向导时，自动展示引导弹窗
  useEffect(() => {
    const guided = localStorage.getItem(FIRST_RUN_GUIDE_STORAGE_KEY) === 'true'
    if (guided || !profiles) return

    // 通过 hasRealProfileItem 判空：内置 Merge/Script 项不视为真实订阅
    if (hasRealProfileItem(profiles.items)) return

    const timer = setTimeout(() => {
      setFirstRunGuideOpen(true)
    }, 500)
    return () => clearTimeout(timer)
  }, [profiles])

  // Takeover Mode States
  const { isTunModeAvailable, mutateSystemState } = useSystemState()
  const { installServiceAndSync } = useServiceInstaller()
  const { enable_tun_mode, enable_system_proxy } = verge ?? {}
  const currentMode = enable_tun_mode
    ? 'tun'
    : enable_system_proxy
      ? 'system'
      : 'manual'
  const activeIndex =
    currentMode === 'tun' ? 2 : currentMode === 'system' ? 1 : 0

  const { clashInfo } = useClashInfo()
  const { clashConfig } = useClashConfigData()

  useEffect(() => {
    getAppVersion()
      .then(setAppVersion)
      .catch((err) => console.error('Failed to get app version:', err))
  }, [])

  const handleHelpClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setHelpAnchorEl(event.currentTarget)
  }

  const handleHelpClose = () => {
    setHelpAnchorEl(null)
  }

  const policyActiveIndex =
    verge?.rule_fallback === 'proxy'
      ? 0
      : verge?.rule_fallback === 'addurl'
        ? 2
        : 1

  const handleRuleFallbackChange = async (
    fallback: 'direct' | 'proxy' | 'addurl',
  ): Promise<boolean> => {
    if (verge?.rule_fallback === fallback) return true

    const waitId = showNotice.info('layout.notices.adjustingProxyMode', 0)
    markLoadingNotice(waitId)

    // 标记长操作并抑制 IPC 超时风暴，配置重生成由后端 update_config_checked 完成
    beginLongOperation()
    try {
      const applied = await patchVerge(
        { rule_fallback: fallback },
        { silentError: true },
      )
      if (!applied) return false

      await patchClashMode('rule')
      await activateSelected()
      if (fallback === 'proxy') {
        showNotice.success('layout.notices.switchedToGlobalProxy')
      } else if (fallback === 'direct') {
        showNotice.success('layout.notices.switchedToGfwlist')
      } else {
        showNotice.success('layout.notices.switchedToAddUrl')
      }
      return true
    } catch (err: unknown) {
      showNotice.error(err)
      return false
    } finally {
      settleLoadingNotice(waitId)
      endLongOperation()
    }
  }

  const [addUrlOpen, setAddUrlOpen] = useState(false)
  const handleAddUrlClick = async () => {
    if (verge?.rule_fallback === 'addurl') {
      setAddUrlOpen(true)
      return
    }
    const ok = await handleRuleFallbackChange('addurl')
    if (ok) {
      setAddUrlOpen(true)
    }
  }

  const themeModeVal = verge?.theme_mode || 'system'
  const themeActiveIndex =
    themeModeVal === 'dark' ? 2 : themeModeVal === 'light' ? 1 : 0

  // 端口编辑态 + allow-lan/ipv6 切换已抽到 use-minimal-settings（详见该 hook 注释）
  const { mixedPortVal, handleSavePort, handleClashBoolChange } =
    useMinimalSettings()

  useEffect(() => {
    if (language && language !== lastLanguageRef.current) {
      lastLanguageRef.current = language
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguageRef.current(language)
    }
  }, [language])

  const [profileRefreshCounter, setProfileRefreshCounter] = useState(0)
  const lastProcessedRef = useRef<{ uid: string | null; counter: number }>({
    uid: null,
    counter: -1,
  })
  const startupRetryCountRef = useRef(0)
  // 当 handleImportProfile 处理中时跳过 useEffect 自动选点，避免双链竞态
  const isImportingRef = useRef(false)
  const lastVisibilityRefreshAtRef = useRef(0)

  // 自动选点后按延迟排序置顶（sortType: 1）
  const [, setHeadStateForSort] = useHeadStateNew()

  const refreshProxyRef = useRef(refreshProxy)
  refreshProxyRef.current = refreshProxy
  const refreshAllRef = useRef(refreshAll)
  refreshAllRef.current = refreshAll
  const setHeadStateForSortRef = useRef(setHeadStateForSort)
  setHeadStateForSortRef.current = setHeadStateForSort
  const tRef = useRef(t)
  tRef.current = t

  // 订阅加载/切换时自动增强（收敛为单一 PROXY 组）；轻量唤醒跳过 enhance，仅按需刷新延迟
  useEffect(() => {
    if (!currentProfileUid) return

    // 通过 localStorage 持久化已增强的 uid，防止轻量模式窗口重建误判为订阅切换
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

  // 窗口恢复可见时仅同步前端状态展示（节点探测与自愈由后端 monitor 统一驱动）
  useEffect(() => {
    if (typeof window === 'undefined') return
    const handleVisibilityChange = () => {
      if (document.visibilityState !== 'visible') return
      const now = Date.now()
      if (now - lastVisibilityRefreshAtRef.current < 3000) return
      lastVisibilityRefreshAtRef.current = now
      void refreshAllRef.current()
    }
    document.addEventListener('visibilitychange', handleVisibilityChange)
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [])

  const themeReady = useMemo(() => Boolean(theme), [theme])
  useLoadingOverlay(themeReady)

  const handleNotice = useCallback(
    (payload: [string, string]) => {
      const [status, msg] = payload
      try {
        handleNoticeMessage(status, msg, t, () => {})
      } catch (error) {
        console.error('[通知处理] 失败:', error)
      }
    },
    [t],
  )
  useLayoutEvents(handleNotice)

  // Profile actions
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
          // 仅在后端确认 enhance 成功时打标记；返回 Busy 时交由 useEffect 重试路径兜底
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

      // 必须在 mutateProfiles 触发 currentProfileUid 变更前释放门闩，使新订阅执行首次自动选点
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
      // M2-10: 切换 Profile 时清理节点延迟缓存
      // 延迟缓存不清理会导致同名节点在新 Profile 中显示旧 Profile 的延迟，干扰自动选点
      getDelayManager().clearCache()
      const enhanced = await patchProfiles({ current: uid })
      // 后端 patch_profiles_config 已同步 update_config_forced（含 generate+validate+reload），
      // 等效于 enhance。仅当后端确认 enhance 成功（返回值 valid）才同步标记，
      // 避免后端因并发持锁返回 Busy（未真正 enhance）时谎报，导致 useEffect 永久跳过
      // enhanceProfiles —— 那样会漏掉单 PROXY 压平，需重启才自愈。
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

  const handleTakeoverModeChange = async (
    targetMode: 'manual' | 'system' | 'tun',
  ) => {
    if (targetMode === currentMode) return

    const waitId = showNotice.info('layout.notices.adjusting', 0)
    markLoadingNotice(waitId)
    beginLongOperation()
    try {
      if (targetMode === 'tun' && !isTunModeAvailable) {
        try {
          await installServiceAndSync({ silentError: true })
        } catch {
          showNotice.error('layout.notices.tunServiceConfigFailed')
          return
        }
      }

      const modeConfig = {
        manual: {
          patch: { enable_system_proxy: false, enable_tun_mode: false },
          notice: 'layout.notices.switchedToManualMode' as const,
        },
        system: {
          patch: { enable_system_proxy: true, enable_tun_mode: false },
          notice: 'layout.notices.systemProxyEnabled' as const,
        },
        tun: {
          patch: { enable_system_proxy: false, enable_tun_mode: true },
          notice: 'layout.notices.tunModeEnabled' as const,
        },
      }[targetMode]

      const applied = await patchVerge(modeConfig.patch, { silentError: true })
      if (!applied) return
      await mutateSystemState()
      if (targetMode === 'manual' && verge?.auto_close_connection) {
        await closeAllConnectionsWithTimeout().catch(() =>
          console.warn('[layout] closeAllConnectionsWithTimeout failed'),
        )
      }
      showNotice.success(modeConfig.notice)
    } catch (err) {
      showNotice.error(err)
    } finally {
      settleLoadingNotice(waitId)
      endLongOperation()
    }
  }

  // Logs dialog state
  const [logsOpen, setLogsOpen] = useState(false)

  // BUG-355: 统一经 disable-webview-shortcuts 单一监听器绑定主界面快捷键（覆盖全部弹窗/菜单排除、编辑态保护与防误伤 Alt+F4）
  const isAnyOverlayOpen =
    drawerOpen ||
    logsOpen ||
    editProfileOpen ||
    Boolean(deleteConfirmProfile) ||
    addUrlOpen ||
    firstRunGuideOpen ||
    clientUpdateOpen ||
    coreUpdateOpen ||
    Boolean(activeHelpAnchorEl)
  const isAnyOverlayOpenRef = useRef(isAnyOverlayOpen)
  useEffect(() => {
    isAnyOverlayOpenRef.current = isAnyOverlayOpen
  }, [isAnyOverlayOpen])
  useEffect(() => bindMainAppShortcuts(() => isAnyOverlayOpenRef.current), [])

  // Custom Titlebar Render
  const customTitlebar = useMemo(
    () =>
      !decorated && !isDecorationsHidden ? (
        <div
          className="the_titlebar"
          data-tauri-drag-region="true"
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            padding: '0 8px 0 10px',
            boxSizing: 'border-box',
            height: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            minHeight: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            maxHeight: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            borderBottom: '1px solid var(--divider-color)',
            background: 'var(--background-color)',
            userSelect: 'none',
            flexShrink: 0,
            gap: '8px',
            overflow: 'hidden',
          }}
        >
          <img
            src={AppIcon}
            alt=""
            draggable={false}
            data-tauri-drag-region="true"
            style={{
              width: '16px',
              height: '16px',
              flexShrink: 0,
              pointerEvents: 'none',
            }}
          />
          <span
            data-tauri-drag-region="true"
            style={{
              fontSize: '12px',
              fontWeight: 500,
              color: 'var(--text-primary-color)',
              flexShrink: 0,
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              pointerEvents: 'none',
            }}
          >
            Clash Mini Ver.{appVersion}
          </span>

          <div
            data-tauri-no-drag="true"
            style={{
              marginLeft: 'auto',
              flexShrink: 0,
              display: 'flex',
              alignItems: 'center',
              height: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            }}
          >
            <WindowControls />
          </div>
        </div>
      ) : null,
    [decorated, isDecorationsHidden, appVersion],
  )

  if (!themeReady) {
    return (
      <div
        style={{
          width: '100vw',
          height: '100vh',
          background: mode === 'light' ? '#fff' : '#181a1b',
          transition: 'background 0.2s',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: mode === 'light' ? '#333' : '#fff',
        }}
      ></div>
    )
  }

  return (
    <ThemeProvider theme={theme}>
      <NoticeManager position={verge?.notice_position} />

      <Paper
        square
        elevation={0}
        className={`${OS} layout`}
        {...(isDecorationsHidden ? { 'data-tauri-drag-region': 'true' } : {})}
        style={{
          width: '100vw',
          height: '100vh',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          background: 'transparent',
          position: 'relative',
        }}
        onContextMenu={(e) => {
          if (
            !['input', 'textarea'].includes(
              e.currentTarget.tagName.toLowerCase(),
            ) &&
            !e.currentTarget.isContentEditable
          ) {
            e.preventDefault()
          }
        }}
      >
        <ResizeHandles />
        {customTitlebar}

        <div
          className="layout-content"
          style={{
            display: 'flex',
            flexDirection: 'column',
            height:
              decorated || isDecorationsHidden
                ? '100vh'
                : `calc(100vh - ${CUSTOM_TITLEBAR_HEIGHT}px)`,
            width: '100vw',
            overflow: 'hidden',
            position: 'relative',
            boxSizing: 'border-box',
          }}
        >
          {/* Upper Pane: Node Selection */}
          <div
            style={{
              flex: '1 1 0%',
              minHeight: 0,
              position: 'relative',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
            }}
          >
            {/* 右上角独立控制按钮（仅在设置面板打开时渲染，以允许关闭设置） */}
            {drawerOpen && (
              <div
                data-no-drag="true"
                style={{
                  position: 'absolute',
                  top: '3px',
                  right: '8px',
                  zIndex: SETTINGS_CLOSE_BUTTON_Z_INDEX,
                }}
              >
                <IconButton
                  size="small"
                  data-close-settings="true"
                  aria-label={t('layout.a11y.closeSettings')}
                  onClick={closeSettingsDrawer}
                  sx={(theme) => ({
                    ...get3DButtonStyle(theme, 'contained', 'primary'),
                    width: '28px',
                    height: '28px',
                    p: 0,
                  })}
                >
                  <CloseRounded
                    aria-hidden="true"
                    sx={{ fontSize: '20px', width: '20px', height: '20px' }}
                  />
                </IconButton>
              </div>
            )}

            {/*置顶当前节点与快捷控制栏（仅在未打开设置时渲染）*/}
            {!drawerOpen && (
              <div
                data-no-drag="true"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  width: '100%',
                  boxSizing: 'border-box',
                  padding: '0px 8px 2px 8px',
                  height: '30px',
                  position: 'relative',
                  zIndex: 9998,
                  gap: 0,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <ErrorBoundary FallbackComponent={AreaErrorFallback}>
                    <ActiveNodeStatusCard />
                  </ErrorBoundary>
                </div>

                <IconButton
                  size="small"
                  aria-label={t('layout.a11y.pinWindow')}
                  onClick={() =>
                    patchVerge({
                      enable_always_on_top: !verge?.enable_always_on_top,
                    }).catch((err) =>
                      console.error('[Layout] toggle pinWindow failed:', err),
                    )
                  }
                  sx={(theme) => ({
                    ...get3DButtonStyle(
                      theme,
                      'contained',
                      verge?.enable_always_on_top ? 'primary' : 'default',
                    ),
                    flexShrink: 0,
                    width: '28px',
                    height: '28px',
                    p: 0,
                  })}
                >
                  <PushPinRounded
                    aria-hidden="true"
                    sx={{
                      fontSize: '20px',
                      width: '20px',
                      height: '20px',
                      color: verge?.enable_always_on_top
                        ? '#FF3B30'
                        : '#888888',
                      filter: verge?.enable_always_on_top
                        ? 'drop-shadow(0 0 3px rgba(255, 59, 48, 0.85)) drop-shadow(0 1px 1px rgba(255, 255, 255, 0.45))'
                        : 'none',
                      transform: verge?.enable_always_on_top
                        ? 'rotate(45deg)'
                        : 'none',
                      transition:
                        'transform 0.2s ease, color 0.2s ease, filter 0.2s ease',
                    }}
                  />
                </IconButton>

                <IconButton
                  size="small"
                  aria-label={t('layout.a11y.openSettings')}
                  onClick={openSettingsDrawer}
                  sx={(theme) => ({
                    ...get3DButtonStyle(theme, 'contained', 'default'),
                    flexShrink: 0,
                    width: '28px',
                    height: '28px',
                    p: 0,
                  })}
                >
                  <SettingsRoundedIcon
                    aria-hidden="true"
                    sx={{ fontSize: '20px', width: '20px', height: '20px' }}
                  />
                </IconButton>
              </div>
            )}

            {/*节点组选择列表（设置抽屉打开被全屏遮挡或极小窗时彻底卸载，释放节点列表内存与 Timer）*/}
            {!drawerOpen && !isMiniStatus && (
              <div style={{ flex: 1, overflow: 'hidden' }}>
                <ErrorBoundary FallbackComponent={AreaErrorFallback}>
                  <ProxyGroups
                    mode={clashConfig?.mode?.toLowerCase() || 'rule'}
                  />
                </ErrorBoundary>
              </div>
            )}

            {/* Settings Drawer (Conditionally mounted; unmounting avoids idle WebSocket/resource drain) */}
            {drawerOpen && !isMiniStatus && (
              <div
                className="theme-panel"
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: '100%',
                  zIndex: 100,
                  // 仅宽窗 (>285px) 会同时挂载右侧 ConnectionsPanel，须保持不换行横排
                  display: 'flex',
                  boxSizing: 'border-box',
                  padding: '12px',
                  gap: '12px',
                  overflow: 'hidden',
                }}
              >
                {/* Left Settings Column (240px width) */}
                <Box
                  sx={{
                    flex: '0 0 240px',
                    width: '240px',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '5px',
                    overflow: 'hidden',
                    pr: 1,
                    borderRight: (theme) =>
                      `1px solid ${theme.palette.divider}`,
                    pb: { xs: 0, '@media (min-height: 831px)': '30px' },
                  }}
                >
                  {/* Section 1: Subscriptions Import */}
                  <ProfileImportCard
                    url={url}
                    setUrl={setUrl}
                    profileLoading={profileLoading}
                    profileItems={profileItems}
                    currentProfileUid={currentProfileUid}
                    importInputRef={importInputRef}
                    importInputContextMenu={activeImportInputContextMenu}
                    setImportInputContextMenu={setImportInputContextMenu}
                    handleImportProfile={handleImportProfile}
                    handleSelectProfile={handleSelectProfile}
                    handleUpdateProfile={handleUpdateProfile}
                    handleDeleteProfile={handleDeleteProfile}
                    setProfileMenuAnchorPosition={setProfileMenuAnchorPosition}
                    setContextMenuProfileUid={setContextMenuProfileUid}
                  />

                  {/* Section 2: Takeover Mode + Routing Preference (合并为同一卡片) */}
                  <Box
                    sx={{
                      p: 1,
                      flexShrink: 0,
                      ...get3DCardStyle(theme, 'default'),
                      '&:hover': {
                        transform: 'none',
                        boxShadow: get3DCardStyle(theme, 'default')
                          .boxShadow as string,
                      },
                    }}
                  >
                    <TakeoverModeCard
                      activeIndex={activeIndex}
                      language={language}
                      handleTakeoverModeChange={handleTakeoverModeChange}
                      disableCardBorder
                    />

                    <RoutingPreferenceCard
                      policyActiveIndex={policyActiveIndex}
                      language={language}
                      handleRuleFallbackChange={handleRuleFallbackChange}
                      onAddUrlClick={handleAddUrlClick}
                      disableCardBorder
                    />
                  </Box>

                  {/* Section 3: Minimal Settings */}
                  <BasicSettingsCard
                    verge={verge ?? null}
                    clashConfig={clashConfig ?? null}
                    clashInfo={clashInfo ?? null}
                    patchVerge={patchVerge}
                    handleAllowLanChange={handleClashBoolChange('allow-lan')}
                    handleIpv6Change={handleClashBoolChange('ipv6')}
                    mixedPortVal={mixedPortVal}
                    handleSavePort={handleSavePort}
                  />

                  {/* Section 4: Theme Settings */}
                  <ThemeSettingsCard
                    verge={verge ?? null}
                    patchVerge={async (val) => {
                      await patchVerge(val)
                    }}
                    themeActiveIndex={themeActiveIndex}
                    depthFactor={depthFactor}
                    handleDepthFactorChange={handleDepthFactorChange}
                    commitDepthFactorChange={commitDepthFactorChange}
                    vibrancyFactor={vibrancyFactor}
                    handleVibrancyFactorChange={handleVibrancyFactorChange}
                    commitVibrancyFactorChange={commitVibrancyFactorChange}
                    controlSkin={controlSkin}
                    setLogsOpen={setLogsOpen}
                    mode={mode}
                  />
                </Box>
                {/* Right Connections column (自适应 flex: 1) — 仅大窗口(宽>285)随设置抽屉挂载；
                      窄窗口或关闭设置后彻底卸载，连接数据/路径控制/列管理/详情弹窗随之销毁 */}
                {!isMinimalWidth && (
                  <ErrorBoundary FallbackComponent={AreaErrorFallback}>
                    <ConnectionsPanel />
                  </ErrorBoundary>
                )}
                {/* Help Button */}
                <HelpMenuButton
                  helpAnchorEl={activeHelpAnchorEl}
                  handleHelpClick={handleHelpClick}
                  handleHelpClose={handleHelpClose}
                  handleClientCheck={handleClientCheck}
                  handleCoreCheck={handleCoreCheck}
                  onReopenFirstRunGuide={() => setFirstRunGuideOpen(true)}
                  clientCheckLoading={clientCheckLoading}
                  coreCheckLoading={coreCheckLoading}
                  appVersion={appVersion}
                  coreVersion={coreVersion}
                  controlSkin={controlSkin}
                  primaryBtn3DStyle={primaryBtn3DStyle}
                />
                {/* Language Selector */}
                <Select
                  value={currentLanguage || 'zh'}
                  onChange={(e) => switchLanguage(e.target.value as string)}
                  displayEmpty
                  // 移除未使用的 @ts-expect-error，MUI v9 aria-label 类型问题已修复
                  inputProps={{
                    'aria-label': t('layout.a11y.selectLanguage'),
                  }}
                  renderValue={() => 'Language'}
                  size="small"
                  variant="outlined"
                  MenuProps={{
                    anchorOrigin: {
                      vertical: 'top',
                      horizontal: 'left',
                    },
                    transformOrigin: {
                      vertical: 'bottom',
                      horizontal: 'left',
                    },
                    slotProps: {
                      paper: getPopupMenuPaperProps(theme, { maxHeight: 640 }),
                    },
                  }}
                  sx={{
                    position: 'absolute',
                    bottom: '0',
                    left: '80px',
                    width: '97.5px',
                    height: '24px',
                    zIndex: 200,
                    boxSizing: 'border-box',
                    fontSize: '11px',
                    fontWeight: 'bold',
                    fontFamily: 'var(--control-font-family)',
                    // 高度 ≤ 830px 时遵循紧凑视口规范隐藏底栏语言选择
                    '@media (max-height: 830px)': {
                      display: 'none',
                    },
                    ...primaryBtn3DStyle,
                    '& .MuiSelect-select': {
                      paddingTop: 0,
                      paddingBottom: 0,
                      paddingLeft: '12px',
                      paddingRight: '24px',
                      height: '100%',
                      display: 'flex',
                      alignItems: 'center',
                      color: 'inherit',
                      boxSizing: 'border-box',
                    },
                    '& .MuiOutlinedInput-notchedOutline': {
                      border: 'none !important',
                    },
                    '&.Mui-focused .MuiOutlinedInput-notchedOutline': {
                      border: 'none !important',
                    },
                    '& .MuiSelect-icon': {
                      color: 'inherit',
                      right: '4px',
                    },
                    '&:before, &:after': {
                      display: 'none !important',
                    },
                  }}
                >
                  <MenuItem value="zh">简体中文</MenuItem>
                  <MenuItem value="en">English</MenuItem>
                  <MenuItem value="ru">Русский</MenuItem>
                  <MenuItem value="fa">فارسی</MenuItem>
                  <MenuItem value="tt">Татарча</MenuItem>
                  <MenuItem value="id">Bahasa Indonesia</MenuItem>
                  <MenuItem value="ar">العربية</MenuItem>
                  <MenuItem value="ko">한국어</MenuItem>
                  <MenuItem value="tr">Türkçe</MenuItem>
                  <MenuItem value="de">Deutsch</MenuItem>
                  <MenuItem value="es">Español</MenuItem>
                  <MenuItem value="jp">日本語</MenuItem>
                  <MenuItem value="zhtw">繁體中文</MenuItem>
                </Select>

                {/* Excel Selector Row */}
                <Box
                  sx={{
                    position: 'absolute',
                    bottom: '0',
                    left: '177.5px',
                    width: '462.5px',
                    height: '24px',
                    display: 'flex',
                    alignItems: 'stretch',
                    zIndex: 200,
                    boxSizing: 'border-box',
                    // 高度 ≤ 830px 时遵循紧凑视口规范隐藏底栏皮肤切换条
                    '@media (max-height: 830px)': {
                      display: 'none',
                    },
                  }}
                >
                  {[
                    {
                      key: 'retro-3d',
                      label: 'Trump-3D',
                      font: 'Trebuchet MS, SimHei, sans-serif',
                    },
                    {
                      key: 'original',
                      label: 'Original',
                      font: 'Segoe UI, Microsoft YaHei, sans-serif',
                    },
                    {
                      key: 'modern-flat',
                      label: 'Modern',
                      font: 'Outfit, DengXian, sans-serif',
                    },
                    {
                      key: 'frosted-glass',
                      label: 'Frosted',
                      font: 'Segoe UI Light, Microsoft YaHei Light, sans-serif',
                    },
                    {
                      key: 'cyberpunk',
                      label: 'Cyberpunk',
                      font: 'Consolas, NSimSun, monospace',
                    },
                    {
                      key: 'monochrome',
                      label: 'Monochrome',
                      font: 'Georgia, KaiTi, serif',
                    },
                  ].map((item, index) => {
                    const isSelected = controlSkin === item.key
                    const handleSelect = () => {
                      localStorage.setItem('clash-mini-control-skin', item.key)
                      setControlSkin(item.key)
                      window.dispatchEvent(new Event('clash-mini-skin-changed'))
                    }

                    return (
                      <Box
                        key={item.key}
                        onClick={handleSelect}
                        sx={(theme) => {
                          const isLight = theme.palette.mode === 'light'
                          const cellWidth = index === 0 ? '92.5px' : '74px'
                          const unselectedStyle = {
                            width: cellWidth,
                            height: '100%',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            cursor: 'pointer',
                            fontSize: '11px',
                            fontWeight: 'bold',
                            color: isLight ? '#555555' : '#aaaaaa',
                            backgroundColor: isLight ? '#f3f3f3' : '#1e1e1e',
                            border: `1px solid ${isLight ? '#d0d0d0' : '#404040'}`,
                            borderLeft:
                              index === 0
                                ? `1px solid ${isLight ? '#d0d0d0' : '#404040'}`
                                : 'none',
                            boxSizing: 'border-box',
                            fontFamily: item.font,
                            transition: 'background-color 0.1s ease',
                            '&:hover': {
                              backgroundColor: isLight ? '#e5e5e5' : '#2d2d2d',
                            },
                          }

                          if (isSelected) {
                            const btnStyle = get3DButtonStyle(
                              theme,
                              'contained',
                              'primary',
                            )
                            return {
                              width: cellWidth,
                              height: '100%',
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'center',
                              cursor: 'pointer',
                              fontSize: '11px',
                              fontWeight: 'bold',
                              boxSizing: 'border-box',
                              fontFamily: item.font,
                              ...btnStyle,
                              borderRadius: btnStyle.borderRadius || '0px',
                              margin: 0,
                            }
                          } else {
                            return unselectedStyle
                          }
                        }}
                      >
                        {item.label}
                      </Box>
                    )
                  })}
                </Box>
              </div>
            )}
          </div>

          {/* Lower Pane: Constant Traffic Dashboard */}
          <div
            style={{
              flex: isMinimalWidth
                ? `0 0 ${TRAFFIC_PANE_HEIGHT_MINIMAL}px`
                : `0 1 ${TRAFFIC_PANE_HEIGHT_WIDE}px`,
              maxHeight: '100%',
              height: isMinimalWidth
                ? `${TRAFFIC_PANE_HEIGHT_MINIMAL}px`
                : `${TRAFFIC_PANE_HEIGHT_WIDE}px`,
              background: 'inherit',
              padding: isMinimalWidth ? '3px 6px 2px 6px' : '8px 12px 2px 12px',
              display: 'flex',
              gap: isMinimalWidth ? '6px' : 0,
              overflow: 'hidden',
              boxSizing: 'border-box',
            }}
          >
            <ErrorBoundary FallbackComponent={AreaErrorFallback}>
              <MiniTrafficPanel isMinimalWidth={isMinimalWidth} />
            </ErrorBoundary>
          </div>
        </div>
      </Paper>

      {/* Popups & dialogs：单个弹窗渲染异常只做局部降级，不拖垮整页 */}
      {/* Profile Card Context Menu (BUG-072) */}
      <Menu
        anchorReference="anchorPosition"
        anchorPosition={
          activeProfileMenuAnchorPosition !== null
            ? {
                top: activeProfileMenuAnchorPosition.top,
                left: activeProfileMenuAnchorPosition.left,
              }
            : undefined
        }
        open={activeProfileMenuAnchorPosition !== null}
        onClose={() => setProfileMenuAnchorPosition(null)}
        slotProps={{
          paper: getPopupMenuPaperProps(theme, { minWidth: '160px' }),
        }}
      >
        <MenuItem onClick={handleEditProfileClick}>
          📝 {t('shared.actions.edit')}
        </MenuItem>
        <MenuItem onClick={handleEditProfileFileClick}>
          📄 {t('profiles.components.menu.editFile')}
        </MenuItem>
        <Divider />
        <MenuItem
          onClick={handleCopyProfileLinkClick}
          disabled={isContextMenuLocal || !contextMenuTargetItem?.url}
        >
          🔗 {t('profiles.components.menu.copyLink')}
        </MenuItem>
        <MenuItem
          onClick={handleUpdateProfileClick}
          disabled={isContextMenuLocal}
        >
          🔄 {t('profiles.components.menu.update')}
        </MenuItem>
        <Divider />
        <MenuItem
          onClick={handleDeleteProfileClick}
          sx={getPopupMenuItemStyle(theme, { danger: true })}
        >
          ❌ {t('shared.actions.delete')}
        </MenuItem>
      </Menu>

      {/* Import Input Context Menu (BUG-091) */}
      <Menu
        anchorReference="anchorPosition"
        anchorPosition={
          activeImportInputContextMenu !== null
            ? {
                top: activeImportInputContextMenu.mouseY,
                left: activeImportInputContextMenu.mouseX,
              }
            : undefined
        }
        open={activeImportInputContextMenu !== null}
        onClose={() => setImportInputContextMenu(null)}
        slotProps={{
          paper: getPopupMenuPaperProps(theme, { minWidth: '160px' }),
        }}
      >
        <MenuItem
          onClick={handleImportInputCut}
          disabled={
            !importInputRef.current ||
            importInputRef.current.selectionStart ===
              importInputRef.current.selectionEnd
          }
        >
          ✂️ {t('shared.actions.cut')}
        </MenuItem>
        <MenuItem
          onClick={handleImportInputCopy}
          disabled={
            !importInputRef.current ||
            importInputRef.current.selectionStart ===
              importInputRef.current.selectionEnd
          }
        >
          📋 {t('shared.actions.copy')}
        </MenuItem>
        <MenuItem onClick={handleImportInputPaste}>
          📥 {t('shared.actions.paste')}
        </MenuItem>
        <MenuItem onClick={handleImportInputSelectAll} disabled={!url}>
          🔍 {t('shared.actions.selectAll')}
        </MenuItem>
        <Divider />
        <MenuItem
          onClick={handleImportInputClear}
          disabled={!url}
          sx={getPopupMenuItemStyle(theme, { danger: true })}
        >
          🧹 {t('shared.actions.clearInput')}
        </MenuItem>
      </Menu>

      <ErrorBoundary FallbackComponent={AreaErrorFallback}>
        <LayoutDialogs
          editProfileOpen={editProfileOpen}
          setEditProfileOpen={setEditProfileOpen}
          editProfileName={editProfileName}
          setEditProfileName={setEditProfileName}
          editProfileUrl={editProfileUrl}
          setEditProfileUrl={setEditProfileUrl}
          editProfileInterval={editProfileInterval}
          setEditProfileInterval={setEditProfileInterval}
          isEditProfileLocal={isEditProfileLocal}
          handleSaveProfile={handleSaveProfile}
          defaultBtn3DStyle={defaultBtn3DStyle}
          primaryBtn3DStyle={primaryBtn3DStyle}
          clientUpdateOpen={clientUpdateOpen}
          setClientUpdateOpen={setClientUpdateOpen}
          clientStatus={clientStatus}
          appVersion={appVersion}
          clientUpdateObj={clientUpdateObj}
          clientProgress={clientProgress}
          clientProgressMessage={clientProgressMessage}
          handleClientUpgrade={handleClientUpgrade}
          coreUpdateOpen={coreUpdateOpen}
          setCoreUpdateOpen={setCoreUpdateOpen}
          coreUpgradeStatus={coreUpgradeStatus}
          coreVersion={coreVersion}
          coreUpdateRelease={coreUpdateRelease}
          coreUpgradeProgress={coreUpgradeProgress}
          coreUpgradeMessage={coreUpgradeMessage}
          handleCoreUpgrade={handleCoreUpgrade}
          logsOpen={logsOpen}
          setLogsOpen={setLogsOpen}
          deleteConfirmProfile={deleteConfirmProfile}
          onCancelDeleteProfile={() => setDeleteConfirmProfile(null)}
          onConfirmDeleteProfile={handleConfirmDeleteProfile}
        />
        <FirstRunGuideDialog
          open={firstRunGuideOpen}
          onClose={() => setFirstRunGuideOpen(false)}
          onImport={handleImportProfile}
          onOpenGithub={() => {
            openWebUrl(CLASH_MINI_GITHUB_URL).catch((e) => {
              console.error('[FirstRunGuide] 打开 GitHub 失败:', e)
            })
          }}
          onOpenWiki={() => {
            openWebUrl(CLASH_MINI_WIKI_URL).catch((e) => {
              console.error('[FirstRunGuide] 打开 Wiki 失败:', e)
            })
          }}
          onLater={() => {
            setFirstRunGuideOpen(false)
            openSettingsDrawer()
          }}
        />
        <AddUrlDialog open={addUrlOpen} onClose={() => setAddUrlOpen(false)} />
      </ErrorBoundary>
    </ThemeProvider>
  )
}

export default Layout
