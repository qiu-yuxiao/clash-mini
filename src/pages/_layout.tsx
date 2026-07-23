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
import { Outlet, useLocation } from 'react-router'

import AppIcon from '@/assets/image/app-icon.png'
import { AreaErrorFallback } from '@/components/base/base-error-boundary'
import { ConnectionDetail } from '@/components/connection/connection-detail'
import type { ConnectionDetailRef } from '@/components/connection/connection-detail'
import { GlowBorder } from '@/components/glow-border'
import { NoticeManager } from '@/components/layout/notice-manager'
import { ResizeHandles } from '@/components/layout/resize-handles'
import { WindowControls } from '@/components/layout/window-controller'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { useHeadStateNew } from '@/components/proxy/use-head-state'
import { useClashInfo, useClash } from '@/hooks/use-clash'
import { useConnectionData } from '@/hooks/use-connection-data'
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
  isPortInUse,
  patchClashMode,
  getProfiles,
  patchClashConfig,
  patchProfile,
  viewProfile,
  restartCore,
  triggerAutoSelect,
} from '@/services/cmds'
import { getDelayManager } from '@/services/delay'
import { closeAllConnectionsWithTimeout } from '@/services/mihomo-api'
import { showNotice, hideNotice } from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import type { IConnectionsItem } from '@/types/connection'
import type { IProfileItem } from '@/types/profile'
import { get3DButtonStyle, get3DCardStyle } from '@/utils/button-styles'

// Sub-components
import { ActiveNodeStatusCard } from './_layout/components/active-node-card'
import { AddUrlDialog } from './_layout/components/add-url-dialog'
import { BasicSettingsCard } from './_layout/components/basic-settings-card'
import { ConnectionsPanel } from './_layout/components/connections-panel'
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
import { useSkinControls } from './_layout/hooks/use-skin-controls'
import { handleNoticeMessage } from './_layout/utils'
import {
  waitForClashReady,
  getFilteredNodeNames,
  triggerAutoSelectAndRefresh,
} from './_layout/utils/profile-coordination'
import { OS, getMenuItemHoverStyle } from './_layout/utils/style-helpers'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)

// 底部常驻流量条(MiniTrafficPanel)的固定高度(px)。
// 这是面板自身的显示尺寸，与窗口极小高度开关阈值(MINI_HEIGHT_THRESHOLD)无关，
// 宽窗高度数值恰好同为 135，纯属巧合，二者不应耦合。
const TRAFFIC_PANE_HEIGHT_WIDE = 135
const TRAFFIC_PANE_HEIGHT_MINIMAL = 100

// Connections order
const ORDER_OPTIONS = [
  {
    id: 'default',
    labelKey: 'connections.components.order.default',
    fn: (list: IConnectionsItem[]) =>
      list.sort(
        (a, b) =>
          new Date(b.start || '0').getTime()! -
          new Date(a.start || '0').getTime()!,
      ),
  },
  {
    id: 'uploadSpeed',
    labelKey: 'connections.components.order.uploadSpeed',
    fn: (list: IConnectionsItem[]) =>
      list.sort((a, b) => (b.curUpload ?? 0) - (a.curUpload ?? 0)),
  },
  {
    id: 'downloadSpeed',
    labelKey: 'connections.components.order.downloadSpeed',
    fn: (list: IConnectionsItem[]) =>
      list.sort((a, b) => (b.curDownload ?? 0) - (a.curDownload ?? 0)),
  },
] as const

type OrderKey = (typeof ORDER_OPTIONS)[number]['id']

type OrderFn = (list: IConnectionsItem[]) => IConnectionsItem[]

const orderFunctionMap = ORDER_OPTIONS.reduce<Record<OrderKey, OrderFn>>(
  (acc, option) => {
    acc[option.id] = option.fn
    return acc
  },
  {} as Record<OrderKey, OrderFn>,
)

const Layout = () => {
  // Active Skin State — managed by useSkinControls
  const {
    controlSkin,
    setControlSkin,
    depthFactor,
    vibrancyFactor,
    handleDepthFactorChange,
    handleVibrancyFactorChange,
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
  const { theme } = useCustomTheme()
  const { verge, patchVerge } = useVerge()
  const { language } = verge ?? {}
  const { switchLanguage, currentLanguage } = useI18n()
  const { decorated, isDecorationsHidden, isMinimalWidth, isMiniStatus } =
    useWindowDecorations()
  const { pathname } = useLocation()

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

  // Edit Profile Dialog State (BUG-072)
  const [editProfileOpen, setEditProfileOpen] = useState(false)
  const [editProfileUid, setEditProfileUid] = useState<string | null>(null)
  const [editProfileName, setEditProfileName] = useState('')
  const [editProfileUrl, setEditProfileUrl] = useState('')
  const [editProfileInterval, setEditProfileInterval] = useState(0)

  const primaryBtn3DStyle = useMemo<SxProps<Theme>>(() => {
    const btnStyle = get3DButtonStyle(theme, 'contained', 'primary')
    const styleWithImportant: Record<string, unknown> = {}
    for (const [key, val] of Object.entries(btnStyle)) {
      if (
        ['background', 'border', 'borderColor', 'boxShadow', 'color'].includes(
          key,
        )
      ) {
        styleWithImportant[key] = `${val} !important`
      } else {
        styleWithImportant[key] = val
      }
    }
    if (btnStyle.background) {
      styleWithImportant.backgroundColor = `${btnStyle.background} !important`
    }
    return styleWithImportant as SxProps<Theme>
  }, [theme])

  const defaultBtn3DStyle = useMemo<SxProps<Theme>>(() => {
    const btnStyle = get3DButtonStyle(theme, 'contained', 'default')
    const styleWithImportant: Record<string, unknown> = {}
    for (const [key, val] of Object.entries(btnStyle)) {
      if (
        ['background', 'border', 'borderColor', 'boxShadow', 'color'].includes(
          key,
        )
      ) {
        styleWithImportant[key] = `${val} !important`
      } else {
        styleWithImportant[key] = val
      }
    }
    if (btnStyle.background) {
      styleWithImportant.backgroundColor = `${btnStyle.background} !important`
    }
    return styleWithImportant as SxProps<Theme>
  }, [theme])
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

  // Takeover Mode States
  const { isTunModeAvailable, mutateSystemState } = useSystemState()
  const { installServiceAndRestartCore } = useServiceInstaller()
  const { enable_tun_mode, enable_system_proxy } = verge ?? {}
  const currentMode = enable_tun_mode
    ? 'tun'
    : enable_system_proxy
      ? 'system'
      : 'manual'
  const activeIndex =
    currentMode === 'tun' ? 2 : currentMode === 'system' ? 1 : 0

  // Port State
  const { clashInfo, patchInfo } = useClashInfo()
  // coreVersion/mutateVersion 已在前面通过 useClash() 提前获取（供 useCoreUpdate 使用）
  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()

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
  ) => {
    try {
      await patchVerge({ rule_fallback: fallback })
      await patchClashMode('rule')
      await enhanceProfiles()
      await activateSelected()
      await refreshClashConfig()
    } catch (err: unknown) {
      showNotice.error(err instanceof Error ? err.message : String(err))
    }
  }

  const [addUrlOpen, setAddUrlOpen] = useState(false)
  const handleAddUrlClick = async () => {
    await handleRuleFallbackChange('addurl')
    setAddUrlOpen(true)
  }

  const themeModeVal = verge?.theme_mode || 'system'
  const themeActiveIndex =
    themeModeVal === 'dark' ? 2 : themeModeVal === 'light' ? 1 : 0

  const [mixedPortVal, setMixedPortVal] = useState(
    verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? 10801,
  )

  // Minimal Settings Actions
  const handleClashBoolChange = (field: string) => async (checked: boolean) => {
    const waitId = showNotice.info('正在调整，请稍候…', 0)
    try {
      await patchClashConfig({ [field]: checked })
      await refreshClashConfig()
      hideNotice(waitId)
      showNotice.success('已更新')
    } catch (err: unknown) {
      hideNotice(waitId)
      showNotice.error(err instanceof Error ? err.message : String(err))
    }
  }

  // Connections manager states
  const [match, setMatch] = useState<(input: string) => boolean>(
    () => () => true,
  )
  const curOrderOpt: OrderKey = 'default'
  const [connectionsType, setConnectionsType] = useState<'active' | 'closed'>(
    'active',
  )
  // 右侧连接/路由表的"挂载"与"数据订阅"共用同一宽度开关（isMinimalWidth）：
  // 大窗口(宽>285) 同时开，窄窗口(宽=285) 同时关，杜绝"看得见/看不见"与"订阅/不订阅"脱节。
  const {
    response: { data: connectionsData },
    clearClosedConnections,
  } = useConnectionData({ enabled: drawerOpen && !isMinimalWidth })
  const [isColumnManagerOpen, setIsColumnManagerOpen] = useState(false)
  const detailRef = useRef<ConnectionDetailRef | null>(null)

  const filterConn = useMemo(() => {
    const orderFunc = orderFunctionMap[curOrderOpt]
    const conns =
      (connectionsType === 'active'
        ? connectionsData?.activeConnections
        : connectionsData?.closedConnections) ?? []
    let matchConns = conns.filter((conn) => {
      const { host, destinationIP, process } = conn.metadata ?? {}
      return (
        match(host || '') || match(destinationIP || '') || match(process || '')
      )
    })
    if (orderFunc) matchConns = orderFunc(matchConns)
    return matchConns
  }, [connectionsData, connectionsType, match, curOrderOpt])

  const handleSearch = useCallback((match: (content: string) => boolean) => {
    setMatch(() => match)
  }, [])

  // Sync mixed port val
  useEffect(() => {
    const vPort = verge?.verge_mixed_port
    const cPort = clashInfo?.mixed_port
    if (vPort !== undefined && vPort !== null) {
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setMixedPortVal(vPort)
    } else if (cPort !== undefined && cPort !== null) {
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setMixedPortVal(cPort)
    }
  }, [verge?.verge_mixed_port, clashInfo?.mixed_port])

  useEffect(() => {
    if (language && language !== lastLanguageRef.current) {
      lastLanguageRef.current = language
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguageRef.current(language)
    }
  }, [language])

  // eslint-disable-next-line @eslint-react/no-unused-state
  const [profileRefreshCounter, setProfileRefreshCounter] = useState(0)
  const lastProcessedRef = useRef<{ uid: string | null; counter: number }>({
    uid: null,
    counter: -1,
  })
  const startupRetryCountRef = useRef(0)
  const fallbackTimerRef = useRef<number | null>(null)
  // H-13: autoSelect timer ref，组件卸载时清理
  const autoSelectTimerRef = useRef<number | null>(null)
  // H-14: wakeup test timer ref，组件卸载时清理
  const wakeupTestTimerRef = useRef<number | null>(null)
  // H-15: 唤醒测速防重入互斥锁
  const isWakeupTestingRef = useRef(false)
  // 解决 BUG-118: handleImportProfile 与 useEffect 双链竞态
  // 当 handleImportProfile 正在处理时，设置此标志让 useEffect 跳过自动选点
  const isImportingRef = useRef(false)

  // 协议要求：自动选点后排序置顶（sortType: 1 = 按延迟排序）
  const [, setHeadStateForSort] = useHeadStateNew()

  const refreshProxyRef = useRef(refreshProxy)
  refreshProxyRef.current = refreshProxy
  const refreshAllRef = useRef(refreshAll)
  refreshAllRef.current = refreshAll
  const setHeadStateForSortRef = useRef(setHeadStateForSort)
  setHeadStateForSortRef.current = setHeadStateForSort
  const tRef = useRef(t)
  tRef.current = t

  const isStartingUpRef = useRef(true)
  const lastFullTestTimeRef = useRef<number>(0)
  const isMountedRef = useRef(true)

  useEffect(() => {
    isMountedRef.current = true
    return () => {
      isMountedRef.current = false
      if (wakeupTestTimerRef.current !== null) {
        clearTimeout(wakeupTestTimerRef.current)
        wakeupTestTimerRef.current = null
      }
    }
  }, [])

  const triggerWakeupLatencyTest = useCallback(async () => {
    // H-15: 防重入互斥锁，快速Alt+Tab两次不会并发执行
    if (isWakeupTestingRef.current) return

    // M-33: 原子化冷却检查+设置，避免竞态条件
    // 冷却 5 分钟：用户频繁 Alt+Tab 切换窗口时避免高频测速，
    // 防止订阅服务器把高频测速视作攻击导致节点封锁
    const now = Date.now()
    if (now - lastFullTestTimeRef.current < 5 * 60 * 1000) {
      return
    }

    if (isStartingUpRef.current) {
      return
    }

    isWakeupTestingRef.current = true
    lastFullTestTimeRef.current = now

    try {
      await refreshAllRef.current()

      const names = await getFilteredNodeNames()
      if (names.length === 0) {
        isWakeupTestingRef.current = false
        return
      }

      // H-14: 清理前一次的 wakeup test timer，防止泄漏
      if (wakeupTestTimerRef.current !== null) {
        clearTimeout(wakeupTestTimerRef.current)
        wakeupTestTimerRef.current = null
      }

      // M2-08: 捕获当前 Profile UID，setTimeout 回调内校验
      const capturedUid = (await getProfiles())?.current || ''

      // 用 setTimeout 让出主线程给浏览器完成当前帧渲染，避免测速的 36 路并发 IPC
      // 与 React 的 layout/paint 争抢主线程导致 UI 冻结
      wakeupTestTimerRef.current = setTimeout(async () => {
        wakeupTestTimerRef.current = null // setTimeout 已触发，清除ID
        if (!isMountedRef.current) return

        // M2-08: 校验 Profile 未切换，避免对旧 Profile 节点执行测速
        const currentUid = (await getProfiles())?.current || ''
        if (currentUid !== capturedUid || !isMountedRef.current) {
          return
        }
        try {
          // 视觉占位：立即将 PROXY 全节点标记为「测速中」以触发流光动画
          for (const name of names) {
            getDelayManager().setDelay(name, 'PROXY', -2)
          }
          getDelayManager().queueGroupNotification('PROXY')
          // 委托后端静默测速填充缓存（不传子集=全量测速）；select=false 严禁切换用户当前节点
          if (currentUid && isMountedRef.current) {
            await triggerAutoSelect(currentUid, undefined, 0, false)
          }
          if (isMountedRef.current) {
            await refreshAllRef.current()
          }
        } catch (err) {
          console.error('[Layout] 唤醒后后台测速异常:', err)
        } finally {
          isWakeupTestingRef.current = false // H-15: 释放互斥锁
        }
      }, 0)
    } catch (err) {
      console.error('[Layout] 唤醒刷新与测速失败:', err)
      isWakeupTestingRef.current = false // H-15: 异常时也要释放锁
    }
  }, [])

  const triggerWakeupLatencyTestRef = useRef(triggerWakeupLatencyTest)
  triggerWakeupLatencyTestRef.current = triggerWakeupLatencyTest

  // WARNING: DO NOT remove this unified hook or replace it with ad-hoc reload chains in other methods (like handleImportProfile).
  // The state-driven approach prevents race conditions during import/activation.
  // The 3-attempt auto-retry block in .catch resolves startup timing issues where the core/socket is temporarily busy.
  // Refer to BUG-171/BUG-172 agreements.
  // Automatically enhance profile when it is loaded or switched (flatten to single PROXY group)
  useEffect(() => {
    if (!currentProfileUid) return

    // 通过 localStorage 跨窗口重建持久化"已 enhance 过的 uid"，避免轻量模式唤醒时误判为 profile 切换
    // （useRef 在 React 重新挂载时会重置，无法区分"窗口重建"和"profile 切换"）
    const enhancedUid = localStorage.getItem('clash-mini-last-enhanced-uid')
    const isNewProfile = enhancedUid !== currentProfileUid
    // 组件重新挂载时 lastProcessedRef.current.uid 为 null，
    // 此时 isRefreshTriggered 必为 false，防止轻量模式唤醒误触发 enhance
    const isRefreshTriggered =
      lastProcessedRef.current.uid !== null &&
      lastProcessedRef.current.counter !== profileRefreshCounter

    if (!isNewProfile && !isRefreshTriggered) {
      // 轻量模式唤醒场景：profile 未变，跳过 enhance（避免触发内核 force=true 重置）
      // 仅恢复节点选择 + 自动选点刷新前端
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
            fallbackTimerRef,
            autoSelectTimerRef,
            setHeadStateForSortRef.current,
            currentProfileUid,
          )
          startupRetryCountRef.current = 0
          lastFullTestTimeRef.current = Date.now()
        } catch (err) {
          if (!cancelled) {
            console.error('[Layout] 唤醒后恢复节点选择失败:', err)
          }
        } finally {
          isStartingUpRef.current = false
        }
      })()
      return () => {
        cancelled = true
      }
    }

    // profile 切换 / 首次启动 / 手动刷新场景：走完整 enhance 流程
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
        // enhance 成功后才写入 localStorage，防止失败时下次跳过
        localStorage.setItem('clash-mini-last-enhanced-uid', uid)
        await activateSelectedRef.current()
        if (cancelled || isImportingRef.current) return
        // 等待 Clash 内核就绪（最多 10 秒），然后触发自动选点并刷新前端
        await waitForClashReady(tRef.current)
        if (cancelled || isImportingRef.current) return
        await triggerAutoSelectAndRefresh(
          refreshProxyRef.current,
          fallbackTimerRef,
          autoSelectTimerRef,
          setHeadStateForSortRef.current,
          uid,
        )
        // Success: reset retry counter
        startupRetryCountRef.current = 0
        // WARN-002 修复：记录启动完成时间，防止首次窗口聚焦时触发冗余的二次全量刷新
        lastFullTestTimeRef.current = Date.now()
      })
      .catch((err) => {
        if (cancelled) return
        console.error(`[Layout] Failed to enhance profile ${uid}:`, err)
        // Reset to allow retry/reload
        lastProcessedRef.current.uid = null

        // Auto-retry up to 3 times on failure/startup
        if (startupRetryCountRef.current < 3) {
          startupRetryCountRef.current += 1
          const retryDelay = 2000 * startupRetryCountRef.current
          timerId = setTimeout(() => {
            if (!cancelled) {
              setProfileRefreshCounter((c) => c + 1)
            }
          }, retryDelay)
        } else {
          showNotice.error(
            'Profile activation failed after 3 retries. Please check your network or subscription.',
          )
        }
      })
      .finally(() => {
        // 无论 enhanceProfiles 成功、失败还是重试耗尽，都必须打开门闩，
        // 否则唤醒测速和 TUN 自动关闭功能永久阻塞。
        isStartingUpRef.current = false
      })
    return () => {
      cancelled = true
      if (timerId) {
        clearTimeout(timerId)
      }
      // Reset last processed to allow retry/reload on next mount/run if cancelled before completion
      lastProcessedRef.current.uid = null
    }
  }, [currentProfileUid, profileRefreshCounter])

  // 组件卸载时清理 Fallback、自动选优及唤醒测速定时器，防止内存泄漏
  useEffect(() => {
    return () => {
      if (fallbackTimerRef.current) {
        clearTimeout(fallbackTimerRef.current)
        fallbackTimerRef.current = null
      }
      // H-13: 清理 autoSelect timer
      if (autoSelectTimerRef.current !== null) {
        clearTimeout(autoSelectTimerRef.current)
        autoSelectTimerRef.current = null
      }
      // H-14: 清理 wakeup test timer
      if (wakeupTestTimerRef.current !== null) {
        clearTimeout(wakeupTestTimerRef.current)
        wakeupTestTimerRef.current = null
      }
    }
  }, [])

  // 监听窗口可见度变化，窗口从隐藏/后台恢复时触发全节点测速刷新
  // 注意：不监听 focus 事件，因为 WebView2 在任何鼠标点击（包括标题栏拖动）时都会触发 focus，
  // 导致每次点击都触发全节点批量测速，造成 UI 冻结
  // 窗口尺寸变化由 AppDataProvider 统一管理并通过 SystemContext 下发，此处不再监听 resize
  useEffect(() => {
    if (typeof window === 'undefined') return
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        triggerWakeupLatencyTestRef.current()
      }
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
  const handleImportProfile = async () => {
    if (!url) return
    const trimmed = url.trim()
    if (!trimmed) return
    if (isImportingRef.current) return
    isImportingRef.current = true
    setProfileLoading(true)
    try {
      await importProfile(url)
      showNotice.success('shared.feedback.notifications.importSuccess')
      setUrl('')

      const freshConfig = await getProfiles()
      const newProfile = freshConfig?.items?.find(
        (p: IProfileItem) => p.url === trimmed,
      )
      let targetUid = currentProfileUid
      if (newProfile) {
        // 后端 import_profile 在首次导入（current 之前为空）时会自动激活并刷新内核，
        // 这里若再调 patchProfiles 会与后端 update_config_forced 竞争验证锁导致超时。
        // 仅在 current 未指向 newProfile 时才显式切换。
        if (freshConfig.current !== newProfile.uid) {
          await patchProfiles({ current: newProfile.uid })
        }
        targetUid = newProfile.uid
      }

      await mutateProfiles()

      if (targetUid) {
        if (targetUid === currentProfileUid) {
          lastProcessedRef.current.uid = null
          setProfileRefreshCounter((c) => c + 1)
        }
      }
    } catch (err) {
      console.error(
        '[handleImportProfile] 首次导入失败，尝试 Clash 代理重试:',
        err,
      )
      try {
        await importProfile(url, { with_proxy: false, self_proxy: true })
        showNotice.success('shared.feedback.notifications.importWithClashProxy')
        setUrl('')

        const freshConfig = await getProfiles()
        const newProfile = freshConfig?.items?.find(
          (p: IProfileItem) => p.url === trimmed,
        )
        let targetUid = currentProfileUid
        if (newProfile) {
          // 同主分支：避免与后端自动激活的 update_config_forced 竞争验证锁
          if (freshConfig.current !== newProfile.uid) {
            await patchProfiles({ current: newProfile.uid })
          }
          targetUid = newProfile.uid
        }

        await mutateProfiles()

        if (targetUid) {
          if (targetUid === currentProfileUid) {
            lastProcessedRef.current.uid = null
            setProfileRefreshCounter((c) => c + 1)
          }
        }
      } catch (retryErr) {
        showNotice.error(
          'profiles.page.feedback.notifications.importFail',
          String(retryErr),
        )
      }
    } finally {
      isImportingRef.current = false
      setProfileLoading(false)
    }
  }

  const handleSelectProfile = useLockFn(async (uid: string) => {
    if (currentProfileUid === uid) return
    try {
      isStartingUpRef.current = true
      startupRetryCountRef.current = 0
      // M2-10: 切换 Profile 时清理旧的测试 URL 缓存和节点延迟缓存
      // 延迟缓存不清理会导致同名节点在新 Profile 中显示旧 Profile 的延迟，干扰自动选点
      getDelayManager().clearUrlMap()
      getDelayManager().clearCache()
      await patchProfiles({ current: uid })
      await mutateProfiles()
      closeAllConnectionsWithTimeout()
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
      showNotice.info('正在更新订阅...')
      await updateProfile(uid)
      if (uid === currentProfileUid) {
        lastProcessedRef.current.uid = null
        setProfileRefreshCounter((c) => c + 1)
      }
      await mutateProfiles()
      showNotice.success('订阅更新成功')
    } catch (err) {
      showNotice.error(err)
    }
  }

  const handleDeleteProfile = async (uid: string, e: React.MouseEvent) => {
    e.stopPropagation()
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
    if (!targetItem || !targetItem.url) return
    try {
      await navigator.clipboard.writeText(targetItem.url)
      showNotice.success('链接已复制到剪贴板')
    } catch (ignoreErr) {
      showNotice.error('复制失败')
    }
  }

  const handleUpdateProfileClick = async () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    await handleUpdateProfile(contextMenuProfileUid, {
      stopPropagation: () => {},
    } as React.MouseEvent)
  }

  const handleDeleteProfileClick = async () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    await handleDeleteProfile(contextMenuProfileUid, {
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
      showNotice.success('配置修改成功')
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

  // Takeover actions
  const handleTakeoverModeChange = async (
    targetMode: 'manual' | 'system' | 'tun',
  ) => {
    if (targetMode === currentMode) return

    const waitId = showNotice.info('正在调整，请稍候…', 0)

    if (targetMode === 'manual') {
      try {
        await patchVerge({ enable_system_proxy: false, enable_tun_mode: false })
        if (verge?.auto_close_connection) {
          await closeAllConnectionsWithTimeout().catch(() =>
            console.warn('[layout] closeAllConnectionsWithTimeout failed'),
          )
        }
        hideNotice(waitId)
        showNotice.success('已切换至手动模式')
      } catch (err) {
        hideNotice(waitId)
        showNotice.error(err)
      }
    } else if (targetMode === 'system') {
      try {
        await patchVerge({ enable_system_proxy: true, enable_tun_mode: false })
        hideNotice(waitId)
        showNotice.success('已开启系统代理')
      } catch (err) {
        hideNotice(waitId)
        showNotice.error(err)
      }
    } else if (targetMode === 'tun') {
      if (!isTunModeAvailable) {
        try {
          await installServiceAndRestartCore()
          await mutateSystemState()
        } catch {
          hideNotice(waitId)
          showNotice.error('TUN 模式服务配置失败，请尝试以管理员身份运行。')
          return
        }
      }

      try {
        await patchVerge({ enable_system_proxy: false, enable_tun_mode: true })
        await restartCore()
        hideNotice(waitId)
        showNotice.success('已开启 TUN 模式')
      } catch (err) {
        hideNotice(waitId)
        showNotice.error(err)
      }
    }
  }

  // Port update
  const handleSavePort = async (port: number) => {
    if (port === verge?.verge_mixed_port && port === clashInfo?.mixed_port)
      return
    try {
      const inUse = await isPortInUse(port)
      if (inUse) {
        showNotice.error('settings.modals.clashPort.messages.portInUse', {
          port: port,
        })
        setMixedPortVal(
          verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? 10801,
        )
        return
      }
      await Promise.all([
        patchInfo({ 'mixed-port': port }),
        patchVerge({ verge_mixed_port: port }),
      ])
      showNotice.success('代理端口已保存并重载')
    } catch (err) {
      showNotice.error(err)
      setMixedPortVal(verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? 10801)
    }
  }

  // Logs dialog state
  const [logsOpen, setLogsOpen] = useState(false)

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
            height: '30px',
            minHeight: '30px',
            maxHeight: '30px',
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
              height: '30px',
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

  // Handle Unlock page rendering
  if (pathname === '/unlock') {
    return (
      <ThemeProvider theme={theme}>
        <NoticeManager position={verge?.notice_position} />
        <Paper square elevation={0} className={`${OS} layout`}>
          {customTitlebar}
          <div className="layout-content" style={{ padding: 20 }}>
            <ErrorBoundary FallbackComponent={AreaErrorFallback}>
              <Outlet />
            </ErrorBoundary>
          </div>
        </Paper>
      </ThemeProvider>
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
            OS === 'windows' &&
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

        {/* FEAT-003: GlowBorder — only visible in stealth mode, replaces native chrome */}
        {isDecorationsHidden && <GlowBorder />}

        <div
          className="layout-content"
          style={{
            display: 'flex',
            flexDirection: 'column',
            height:
              decorated || isDecorationsHidden ? '100vh' : 'calc(100vh - 30px)',
            width: '100vw',
            overflow: 'hidden',
            position: 'relative',
            boxSizing: 'border-box',
          }}
        >
          {/* Upper Pane: Node Selection (80% + 30px) */}
          <div
            style={{
              flex: isMinimalWidth ? '1 1 0%' : '80 0 calc(0% + 30px)',
              height: isMinimalWidth ? 'auto' : 'calc(80% + 30px)',
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
                  zIndex: 9999,
                }}
              >
                <IconButton
                  size="small"
                  aria-label={t('layout.a11y.closeSettings')}
                  onClick={() => setDrawerOpen(false)}
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
                    })
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
                  onClick={() => setDrawerOpen(true)}
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

            {/*节点组选择列表*/}
            {!isMiniStatus && (
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
                  // WARNING [FOR AI AGENTS / AUDITORS]:
                  // This flex layout must remain as row direction and MUST NOT wrap. In narrow window
                  // (width = 285px, i.e. isMinimalWidth) the right ConnectionsPanel is NOT mounted at all
                  // (see conditional render `{!isMinimalWidth && <ConnectionsPanel/>}`), so the squeeze-to-0px
                  // path is never reached there. In large window the panel mounts and this row layout applies.
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
                    importInputContextMenu={importInputContextMenu}
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
                    <AddUrlDialog
                      open={addUrlOpen}
                      onClose={() => setAddUrlOpen(false)}
                    />
                  </Box>

                  {/* Section 3: Minimal Settings */}
                  <BasicSettingsCard
                    verge={verge ?? null}
                    clashConfig={clashConfig ?? null}
                    patchVerge={patchVerge}
                    handleAllowLanChange={handleClashBoolChange('allow-lan')}
                    handleIpv6Change={handleClashBoolChange('ipv6')}
                    mixedPortVal={mixedPortVal}
                    handleSavePort={handleSavePort}
                  />

                  {/* Section 4: Theme Settings */}
                  <ThemeSettingsCard
                    verge={verge ?? null}
                    patchVerge={patchVerge}
                    themeActiveIndex={themeActiveIndex}
                    depthFactor={depthFactor}
                    handleDepthFactorChange={handleDepthFactorChange}
                    vibrancyFactor={vibrancyFactor}
                    handleVibrancyFactorChange={handleVibrancyFactorChange}
                    controlSkin={controlSkin}
                    setLogsOpen={setLogsOpen}
                    mode={mode}
                  />
                </Box>
                {/* Right Connections column (自适应 flex: 1) — 仅大窗口(宽>285)挂载；
                      窄窗口(宽=285)下不挂载、不渲染、不订阅数据，与上方 useConnectionData 共用 isMinimalWidth 信号 */}
                {!isMinimalWidth && (
                  <ErrorBoundary FallbackComponent={AreaErrorFallback}>
                    <ConnectionsPanel
                      connectionsType={connectionsType}
                      setConnectionsType={setConnectionsType}
                      connectionsData={connectionsData}
                      handleSearch={handleSearch}
                      filterConn={filterConn}
                      detailRef={detailRef}
                      isColumnManagerOpen={isColumnManagerOpen}
                      setIsColumnManagerOpen={setIsColumnManagerOpen}
                      clearClosedConnections={clearClosedConnections}
                    />
                  </ErrorBoundary>
                )}
                {/* Help Button */}
                <HelpMenuButton
                  helpAnchorEl={helpAnchorEl}
                  handleHelpClick={handleHelpClick}
                  handleHelpClose={handleHelpClose}
                  handleClientCheck={handleClientCheck}
                  handleCoreCheck={handleCoreCheck}
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
                      paper: {
                        sx: {
                          maxHeight: 640,
                        },
                      },
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
                    // WARNING [FOR AI AGENTS / AUDITORS]:
                    // The 830px max-height display:none check is the INTENDED responsive height cutoff
                    // rule mandated by clash_mini_agreements.md. Under default 680px height, these
                    // elements must remain hidden. DO NOT change this threshold to make them visible.
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
                    // WARNING [FOR AI AGENTS / AUDITORS]:
                    // The 830px max-height display:none check is the INTENDED responsive height cutoff
                    // rule mandated by clash_mini_agreements.md. Under default 680px height, these
                    // elements must remain hidden. DO NOT change this threshold to make them visible.
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

          {/* Lower Pane: Constant Traffic Dashboard (Fixed Height - 30px) */}
          <div
            style={{
              flex: isMinimalWidth
                ? `0 0 ${TRAFFIC_PANE_HEIGHT_MINIMAL}px`
                : `0 0 ${TRAFFIC_PANE_HEIGHT_WIDE}px`,
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

      {/* Popups & dialogs */}
      <ConnectionDetail ref={detailRef} />

      {/* Profile Card Context Menu (BUG-072) */}
      <Menu
        anchorReference="anchorPosition"
        anchorPosition={
          profileMenuAnchorPosition !== null
            ? {
                top: profileMenuAnchorPosition.top,
                left: profileMenuAnchorPosition.left,
              }
            : undefined
        }
        open={profileMenuAnchorPosition !== null}
        onClose={() => setProfileMenuAnchorPosition(null)}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              minWidth: '160px',
              borderRadius: '6px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              '& .MuiList-root': {
                padding: '4px 0',
              },
            },
          },
        }}
      >
        <MenuItem
          onClick={handleEditProfileClick}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📝 编辑
        </MenuItem>
        <MenuItem
          onClick={handleEditProfileFileClick}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📄 编辑文件
        </MenuItem>
        <Divider sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }} />
        <MenuItem
          onClick={handleCopyProfileLinkClick}
          disabled={isContextMenuLocal || !contextMenuTargetItem?.url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🔗 复制链接
        </MenuItem>
        <MenuItem
          onClick={handleUpdateProfileClick}
          disabled={isContextMenuLocal}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🔄 更新
        </MenuItem>
        <Divider sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }} />
        <MenuItem
          onClick={handleDeleteProfileClick}
          sx={{
            ...getMenuItemHoverStyle(theme, controlSkin),
            color: 'error.main',
          }}
        >
          ❌ 删除
        </MenuItem>
      </Menu>

      {/* Import Input Context Menu (BUG-091) */}
      <Menu
        anchorReference="anchorPosition"
        anchorPosition={
          importInputContextMenu !== null
            ? {
                top: importInputContextMenu.mouseY,
                left: importInputContextMenu.mouseX,
              }
            : undefined
        }
        open={importInputContextMenu !== null}
        onClose={() => setImportInputContextMenu(null)}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              minWidth: '160px',
              borderRadius: '6px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              '& .MuiList-root': {
                padding: '4px 0',
              },
            },
          },
        }}
      >
        <MenuItem
          onClick={handleImportInputCut}
          disabled={
            !importInputRef.current ||
            importInputRef.current.selectionStart ===
              importInputRef.current.selectionEnd
          }
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          ✂️ 剪切
        </MenuItem>
        <MenuItem
          onClick={handleImportInputCopy}
          disabled={
            !importInputRef.current ||
            importInputRef.current.selectionStart ===
              importInputRef.current.selectionEnd
          }
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📋 复制
        </MenuItem>
        <MenuItem
          onClick={handleImportInputPaste}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📥 粘贴
        </MenuItem>
        <MenuItem
          onClick={handleImportInputSelectAll}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🔍 全选
        </MenuItem>
        <Divider sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }} />
        <MenuItem
          onClick={handleImportInputClear}
          disabled={!url}
          sx={{
            ...getMenuItemHoverStyle(theme, controlSkin),
            color: 'error.main',
          }}
        >
          🧹 清空
        </MenuItem>
      </Menu>

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
        mode={mode}
      />
    </ThemeProvider>
  )
}

export default Layout
