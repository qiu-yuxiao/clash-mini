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
} from '@mui/material'
import { alpha } from '@mui/material'
import { getVersion as getAppVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import { check, type Update } from '@tauri-apps/plugin-updater'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useLocation } from 'react-router'

import { ConnectionDetail } from '@/components/connection/connection-detail'
import { GlowBorder } from '@/components/glow-border'
import { NoticeManager } from '@/components/layout/notice-manager'
import { WindowControls } from '@/components/layout/window-controller'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { useClashInfo, useClash } from '@/hooks/use-clash'
import { useConnectionData } from '@/hooks/use-connection-data'
import { useI18n } from '@/hooks/use-i18n'
import { useProfiles } from '@/hooks/use-profiles'
import { useProxySelection } from '@/hooks/use-proxy-selection'
import { useServiceInstaller } from '@/hooks/use-service-installer'
import { useSystemState } from '@/hooks/use-system-state'
import { useVerge } from '@/hooks/use-verge'
import { useWindowDecorations } from '@/hooks/use-window'
import {
  useProxiesData,
  useClashConfigData,
  useAppRefreshers,
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
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import type { IConnectionsItem } from '@/types/connection'
import {
  get3DButtonStyle,
} from '@/utils/button-styles'
import {
  closeAllConnections,
} from 'tauri-plugin-mihomo-api'


// Sub-components
import { ActiveNodeStatusCard } from './_layout/components/active-node-card'
import { BasicSettingsCard } from './_layout/components/basic-settings-card'
import { ConnectionsPanel } from './_layout/components/connections-panel'
import { HelpMenuButton } from './_layout/components/help-menu-button'
import { LayoutDialogs } from './_layout/components/layout-dialogs'
import { MiniTrafficPanel } from './_layout/components/mini-traffic-panel'
import { ProfileImportCard } from './_layout/components/profile-import-card'
import { RoutingPreferenceCard } from './_layout/components/routing-preference-card'
import { TakeoverModeCard } from './_layout/components/takeover-mode-card'
import {
  useCustomTheme,
  useLayoutEvents,
  useLoadingOverlay,
} from './_layout/hooks'
import { handleNoticeMessage } from './_layout/utils'

// Style Helpers
import {
  OS,
  getMenuItemHoverStyle,
  isSameVersion,
} from './_layout/utils/style-helpers'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)

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

const orderFunctionMap = ORDER_OPTIONS.reduce<Record<OrderKey, any>>(
  (acc, option) => {
    acc[option.id] = option.fn
    return acc
  },
  {} as Record<OrderKey, any>,
)

interface GithubAsset {
  name: string
  browser_download_url: string
}

interface GithubRelease {
  tag_name: string
  assets: GithubAsset[]
}

const Layout = () => {
  // Active Skin State
  const [controlSkin, setControlSkin] = useState(() => {
    return typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d'
  })

  // Dual Sliders State (Depth & Vibrancy Factors)
  const [depthFactor, setDepthFactor] = useState<number>(() => {
    const skin =
      typeof window !== 'undefined'
        ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
        : 'retro-3d'
    const saved = localStorage.getItem(`clash-mini-${skin}-val1`)
    if (saved !== null) return parseFloat(saved)
    if (skin === 'retro-3d') {
      const oldSaved = localStorage.getItem('clash-mini-depth-factor')
      return oldSaved !== null ? parseFloat(oldSaved) : 1.0
    }
    return 1.0
  })

  const [vibrancyFactor, setVibrancyFactor] = useState<number>(() => {
    const skin =
      typeof window !== 'undefined'
        ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
        : 'retro-3d'
    const saved = localStorage.getItem(`clash-mini-${skin}-val2`)
    if (saved !== null) return parseFloat(saved)
    if (skin === 'retro-3d') {
      const oldSaved = localStorage.getItem('clash-mini-vibrancy-factor')
      return oldSaved !== null ? parseFloat(oldSaved) : 1.0
    }
    return 1.0
  })

  useEffect(() => {
    const handleSkinChanged = () => {
      const newSkin =
        localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      setControlSkin(newSkin)
      const val1 = localStorage.getItem(`clash-mini-${newSkin}-val1`)
      const val2 = localStorage.getItem(`clash-mini-${newSkin}-val2`)
      if (val1 !== null) {
        setDepthFactor(parseFloat(val1))
      } else if (newSkin === 'retro-3d') {
        const oldSaved = localStorage.getItem('clash-mini-depth-factor')
        setDepthFactor(oldSaved !== null ? parseFloat(oldSaved) : 1.0)
      } else {
        setDepthFactor(1.0)
      }

      if (val2 !== null) {
        setVibrancyFactor(parseFloat(val2))
      } else if (newSkin === 'retro-3d') {
        const oldSaved = localStorage.getItem('clash-mini-vibrancy-factor')
        setVibrancyFactor(oldSaved !== null ? parseFloat(oldSaved) : 1.0)
      } else {
        setVibrancyFactor(1.0)
      }
    }
    window.addEventListener('clash-mini-skin-changed', handleSkinChanged)
    return () => {
      window.removeEventListener('clash-mini-skin-changed', handleSkinChanged)
    }
  }, [])

  // Update States
  const [helpAnchorEl, setHelpAnchorEl] = useState<null | HTMLElement>(null)
  const [appVersion, setAppVersion] = useState<string>('')

  // Client Update states
  const [clientUpdateOpen, setClientUpdateOpen] = useState(false)
  const [clientUpdateObj, setClientUpdateObj] = useState<Update | null>(null)
  const [clientStatus, setClientStatus] = useState<
    'idle' | 'downloading' | 'error' | 'done'
  >('idle')
  const [clientProgress, setClientProgress] = useState(0)
  const [clientProgressMessage, setClientProgressMessage] = useState('')
  const [clientCheckLoading, setClientCheckLoading] = useState(false)

  // Core Update states
  const [coreUpdateOpen, setCoreUpdateOpen] = useState(false)
  const [coreUpdateRelease, setCoreUpdateRelease] = useState<GithubRelease | null>(null)
  const [coreUpgradeStatus, setCoreUpgradeStatus] = useState<string>('idle')
  const [coreUpgradeProgress, setCoreUpgradeProgress] = useState<number>(0)
  const [coreUpgradeMessage, setCoreUpgradeMessage] = useState<string>('')
  const [coreCheckLoading, setCoreCheckLoading] = useState(false)

  const [isMinimalWidth, setIsMinimalWidth] = useState(() => {
    if (typeof window !== 'undefined') {
      return window.innerWidth <= 285
    }
    return false
  })

  useEffect(() => {
    if (typeof window === 'undefined') return
    const handleResize = () => {
      setIsMinimalWidth(window.innerWidth <= 285)
    }
    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [])

  const handleDepthFactorChange = (val: number) => {
    setDepthFactor(val)
    localStorage.setItem(`clash-mini-${controlSkin}-val1`, val.toString())
    if (controlSkin === 'retro-3d') {
      localStorage.setItem('clash-mini-depth-factor', val.toString())
    }
  }

  const handleVibrancyFactorChange = (val: number) => {
    setVibrancyFactor(val)
    localStorage.setItem(`clash-mini-${controlSkin}-val2`, val.toString())
    if (controlSkin === 'retro-3d') {
      localStorage.setItem('clash-mini-vibrancy-factor', val.toString())
    }
  }

  useEffect(() => {
    document.documentElement.style.setProperty(
      '--depth-factor',
      depthFactor.toString(),
    )
    document.documentElement.style.setProperty(
      '--control-skin-val1',
      depthFactor.toString(),
    )
  }, [depthFactor, controlSkin])

  useEffect(() => {
    document.documentElement.style.setProperty(
      '--vibrancy-factor',
      vibrancyFactor.toString(),
    )
    document.documentElement.style.setProperty(
      '--control-skin-val2',
      vibrancyFactor.toString(),
    )
  }, [vibrancyFactor, controlSkin])

  const getSlider1Label = () => {
    switch (controlSkin) {
      case 'retro-3d':
        return 'Depth'
      case 'original':
        return 'Radius'
      case 'modern-flat':
        return 'Roundness'
      case 'frosted-glass':
        return 'Opacity'
      case 'cyberpunk':
        return 'Glow'
      case 'monochrome':
        return 'Radius'
      default:
        return 'Depth'
    }
  }

  const getSlider2Label = () => {
    switch (controlSkin) {
      case 'retro-3d':
        return 'Vibrancy'
      case 'original':
        return 'Accent'
      case 'modern-flat':
        return 'Shadow'
      case 'frosted-glass':
        return 'Blur'
      case 'cyberpunk':
        return 'Speed'
      case 'monochrome':
        return 'Border'
      default:
        return 'Vibrancy'
    }
  }

  const mode = useThemeMode()
  const { t } = useTranslation() as any
  const { theme } = useCustomTheme()
  if (theme) {
    ;(theme as any).controlSkin = controlSkin
  }
  const isRetro3DDark =
    controlSkin === 'retro-3d' && theme?.palette?.mode === 'dark'
  const { verge, patchVerge } = useVerge()
  const { language } = verge ?? {}
  const { switchLanguage, currentLanguage } = useI18n()
  const { decorated, isDecorationsHidden } = useWindowDecorations()
  const { pathname } = useLocation()
  const windowControlsRef = useRef<any>(null)

  // Language Sync Ref to prevent deadlock/rollback loops
  const lastLanguageRef = useRef<string | undefined>(undefined)
  const switchLanguageRef = useRef(switchLanguage)
  useEffect(() => {
    switchLanguageRef.current = switchLanguage
  }, [switchLanguage])

  // Drawer Toggle State
  const [drawerOpen, setDrawerOpen] = useState(false)

  // Profiles State
  const [url, setUrl] = useState('')
  const [profileLoading, setProfileLoading] = useState(false)

  // Context Menu State for Import Subscription Input Field (BUG-091)
  const [importInputContextMenu, setImportInputContextMenu] = useState<{
    mouseX: number
    mouseY: number
  } | null>(null)
  const importInputRef = useRef<HTMLInputElement>(null)

  const handleImportInputPaste = async () => {
    setImportInputContextMenu(null)
    let textToPaste = ''
    try {
      textToPaste = await readText()
    } catch (err) {
      try {
        textToPaste = await navigator.clipboard.readText()
      } catch (e) {
        console.error('Failed to read from clipboard:', e)
      }
    }

    if (!textToPaste) return

    const input = importInputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const currentVal = url || ''
      const newValue =
        currentVal.substring(0, start) + textToPaste + currentVal.substring(end)
      setUrl(newValue)
      setTimeout(() => {
        input.focus()
        const newCursorPos = start + textToPaste.length
        input.setSelectionRange(newCursorPos, newCursorPos)
      }, 0)
    } else {
      setUrl(textToPaste)
    }
  }

  const handleImportInputCopy = async () => {
    setImportInputContextMenu(null)
    const input = importInputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const selectedText = (url || '').substring(start, end)
      if (selectedText) {
        try {
          await writeText(selectedText)
        } catch (err) {
          try {
            await navigator.clipboard.writeText(selectedText)
          } catch (e) {
            console.error('Failed to copy to clipboard:', e)
          }
        }
      }
    }
  }

  const handleImportInputCut = async () => {
    setImportInputContextMenu(null)
    const input = importInputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const currentVal = url || ''
      const selectedText = currentVal.substring(start, end)
      if (selectedText) {
        try {
          await writeText(selectedText)
        } catch (err) {
          try {
            await navigator.clipboard.writeText(selectedText)
          } catch (e) {
            console.error('Failed to copy to clipboard:', e)
          }
        }
        const newValue =
          currentVal.substring(0, start) + currentVal.substring(end)
        setUrl(newValue)
        setTimeout(() => {
          input.focus()
          input.setSelectionRange(start, start)
        }, 0)
      }
    }
  }

  const handleImportInputSelectAll = () => {
    setImportInputContextMenu(null)
    const input = importInputRef.current
    if (input) {
      input.focus()
      input.setSelectionRange(0, (url || '').length)
    }
  }

  const handleImportInputClear = () => {
    setImportInputContextMenu(null)
    setUrl('')
    const input = importInputRef.current
    if (input) {
      input.focus()
    }
  }

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

  const primaryBtn3DStyle = useMemo(() => {
    const btnStyle = get3DButtonStyle(theme, 'contained', 'primary')
    const styleWithImportant: any = {}
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
    return styleWithImportant
  }, [theme])

  const defaultBtn3DStyle = useMemo(() => {
    const btnStyle = get3DButtonStyle(theme, 'contained', 'default')
    const styleWithImportant: any = {}
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
    return styleWithImportant
  }, [theme])
  const {
    profiles = {},
    mutateProfiles,
    activateSelected,
    patchProfiles,
  } = useProfiles()
  const { changeProxy } = useProxySelection()
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
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
  const { version: coreVersion, mutateVersion } = useClash()
  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()

  useEffect(() => {
    getAppVersion()
      .then(setAppVersion)
      .catch((err) => console.error('Failed to get app version:', err))
  }, [])

  useEffect(() => {
    let active = true
    const unlistenPromise = listen<any>('core-upgrade-progress', (event) => {
      if (!active) return
      const payload = event.payload
      setCoreUpgradeStatus(payload.status)
      setCoreUpgradeProgress(payload.progress)
      setCoreUpgradeMessage(payload.message)
      if (payload.status === 'done') {
        showNotice.success('Mihomo 内核更新成功')
        mutateVersion()
      } else if (payload.status === 'error') {
        showNotice.error(`内核更新失败: ${payload.message}`)
      }
    })
    return () => {
      active = false
      unlistenPromise.then((unlisten) => unlisten())
    }
  }, [mutateVersion])

  const handleHelpClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setHelpAnchorEl(event.currentTarget)
  }

  const handleHelpClose = () => {
    setHelpAnchorEl(null)
  }

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
    } catch (err: any) {
      console.error('Failed to check for client update:', err)
      showNotice.error(`检查更新失败: ${err.message || err}`)
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
      await clientUpdateObj.downloadAndInstall((progressEvent: any) => {
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
      })
      showNotice.success('更新安装完毕，请重启应用以应用更改')
    } catch (err: any) {
      console.error('Client update error:', err)
      setClientStatus('error')
      setClientProgressMessage(`更新失败: ${err.message || err}`)
      showNotice.error(`更新失败: ${err.message || err}`)
    }
  }

  const handleCoreCheck = async () => {
    setCoreCheckLoading(true)
    setHelpAnchorEl(null)
    try {
      const release = await invoke<any>('check_core_update')
      if (isSameVersion(coreVersion, release.tag_name)) {
        showNotice.info('当前内核已是最新版本')
        return
      }
      setCoreUpdateRelease(release)
      setCoreUpdateOpen(true)
      setCoreUpgradeStatus('idle')
      setCoreUpgradeProgress(0)
      setCoreUpgradeMessage('')
    } catch (err: any) {
      console.error('Failed to check for core update:', err)
      showNotice.error(`检查内核更新失败: ${err.message || err}`)
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
      await invoke('start_core_upgrade', { release: coreUpdateRelease })
    } catch (err: any) {
      console.error('Failed to start core upgrade:', err)
      setCoreUpgradeStatus('error')
      setCoreUpgradeMessage(`启动失败: ${err.message || err}`)
      showNotice.error(`启动内核升级失败: ${err.message || err}`)
    }
  }

  const policyActiveIndex =
    verge?.rule_fallback === 'direct'
      ? 0
      : verge?.rule_fallback === 'adjustable'
        ? 1
        : 2

  const handleRuleFallbackChange = async (
    fallback: 'direct' | 'adjustable' | 'proxy',
  ) => {
    try {
      await patchVerge({ rule_fallback: fallback })
      await patchClashMode('rule')
      await enhanceProfiles()
      await activateSelected()
      await refreshClashConfig()
    } catch (err: any) {
      showNotice.error(err?.message || err)
    }
  }

  const themeModeVal = verge?.theme_mode || 'system'
  const themeActiveIndex =
    themeModeVal === 'dark' ? 2 : themeModeVal === 'light' ? 1 : 0

  const [mixedPortVal, setMixedPortVal] = useState(
    verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? 10801,
  )

  // Minimal Settings Actions
  const handleAllowLanChange = async (checked: boolean) => {
    try {
      await patchClashConfig({ 'allow-lan': checked })
      await refreshClashConfig()
    } catch (err: any) {
      showNotice.error(err?.message || err)
    }
  }

  // Connections manager states
  const [match, setMatch] = useState<(input: string) => boolean>(
    () => () => true,
  )
  // eslint-disable-next-line unused-imports/no-unused-vars
  const [curOrderOpt, setCurOrderOpt] = useState<OrderKey>('default')
  const [connectionsType, setConnectionsType] = useState<'active' | 'closed'>(
    'active',
  )
  const {
    response: { data: connectionsData },
    clearClosedConnections,
  } = useConnectionData({ enabled: drawerOpen })
  const [isColumnManagerOpen, setIsColumnManagerOpen] = useState(false)
  const detailRef = useRef<any>(null)

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
      Promise.resolve().then(() => setMixedPortVal(vPort))
    } else if (cPort !== undefined && cPort !== null) {
      Promise.resolve().then(() => setMixedPortVal(cPort))
    }
  }, [verge?.verge_mixed_port, clashInfo?.mixed_port])

  useEffect(() => {
    if (language && language !== lastLanguageRef.current) {
      lastLanguageRef.current = language
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguageRef.current(language)
    }
  }, [language])

  const lastEnhancedProfileRef = useRef<string | null>(null)

  // Automatically enhance profile when it is loaded or switched (flatten to single PROXY group)
  useEffect(() => {
    if (
      currentProfileUid &&
      lastEnhancedProfileRef.current !== currentProfileUid
    ) {
      lastEnhancedProfileRef.current = currentProfileUid
      enhanceProfiles()
        .then(async () => {
          console.log(`[Layout] Enhanced active profile: ${currentProfileUid}`)
          await activateSelectedRef.current()
        })
        .catch((err) => {
          console.error(
            `[Layout] Failed to enhance profile ${currentProfileUid}:`,
            err,
          )
          lastEnhancedProfileRef.current = null
        })
    }
  }, [currentProfileUid])

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
    setProfileLoading(true)
    try {
      await importProfile(url)
      showNotice.success('shared.feedback.notifications.importSuccess')
      setUrl('')

      const freshConfig = await getProfiles()
      const newProfile = freshConfig?.items?.find((p: any) => p.url === trimmed)
      let targetUid = currentProfileUid
      if (newProfile) {
        await patchProfiles({ current: newProfile.uid })
        targetUid = newProfile.uid
      }

      await mutateProfiles()

      // Real-time compilation and reload
      await enhanceProfiles()
      await refreshProxy()
      if (targetUid) {
        invoke('trigger_auto_select', { isManual: false })
      }
    } catch {
      try {
        await importProfile(url, { with_proxy: false, self_proxy: true })
        showNotice.success('shared.feedback.notifications.importWithClashProxy')
        setUrl('')

        const freshConfig = await getProfiles()
        const newProfile = freshConfig?.items?.find(
          (p: any) => p.url === trimmed,
        )
        let targetUid = currentProfileUid
        if (newProfile) {
          await patchProfiles({ current: newProfile.uid })
          targetUid = newProfile.uid
        }

        await mutateProfiles()

        // Real-time compilation and reload
        await enhanceProfiles()
        await refreshProxy()
        if (targetUid) {
          invoke('trigger_auto_select', { isManual: false })
        }
      } catch (retryErr) {
        showNotice.error(
          'profiles.page.feedback.notifications.importFail',
          String(retryErr),
        )
      }
    } finally {
      setProfileLoading(false)
    }
  }

  const handleSelectProfile = async (uid: string) => {
    if (currentProfileUid === uid) return
    try {
      await patchProfiles({ current: uid })
      await mutateProfiles()
      closeAllConnections()
      showNotice.success(
        'profiles.page.feedback.notifications.profileSwitched',
        1000,
      )
    } catch (err) {
      showNotice.error(err)
    }
  }

  const handleUpdateProfile = async (uid: string, e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      showNotice.info('正在更新订阅...')
      await updateProfile(uid)
      if (uid === currentProfileUid) {
        await enhanceProfiles()
        invoke('trigger_auto_select', { isManual: false })
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
    } as any)
  }

  const handleDeleteProfileClick = async () => {
    setProfileMenuAnchorPosition(null)
    if (!contextMenuProfileUid) return
    await handleDeleteProfile(contextMenuProfileUid, {
      stopPropagation: () => {},
    } as any)
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
      await enhanceProfiles()
      await refreshProxy()
    } catch (err) {
      showNotice.error(String(err))
    }
  }

  // Takeover actions
  const handleTakeoverModeChange = async (
    targetMode: 'manual' | 'system' | 'tun',
  ) => {
    if (targetMode === currentMode) return

    if (targetMode === 'manual') {
      try {
        await patchVerge({ enable_system_proxy: false, enable_tun_mode: false })
        if (verge?.auto_close_connection) {
          await closeAllConnections().catch(() => {})
        }
        showNotice.success('已切换至手动模式')
      } catch (err) {
        showNotice.error(err)
      }
    } else if (targetMode === 'system') {
      try {
        await patchVerge({ enable_system_proxy: true, enable_tun_mode: false })
        showNotice.success('已开启系统代理')
      } catch (err) {
        showNotice.error(err)
      }
    } else if (targetMode === 'tun') {
      if (!isTunModeAvailable) {
        try {
          showNotice.info('正在自动安装/配置虚拟网卡系统服务...')
          await installServiceAndRestartCore()
          await mutateSystemState()
        } catch {
          showNotice.error('TUN 模式服务配置失败，请尝试以管理员身份运行。')
          return
        }
      }

      try {
        await patchVerge({ enable_system_proxy: false, enable_tun_mode: true })
        showNotice.success('已开启 TUN 模式')
      } catch (err) {
        showNotice.error(err)
      }
    }
  }

  // Port update
  const handleSavePort = async () => {
    if (
      mixedPortVal === verge?.verge_mixed_port &&
      mixedPortVal === clashInfo?.mixed_port
    )
      return
    try {
      const inUse = await isPortInUse(mixedPortVal)
      if (inUse) {
        showNotice.error('settings.modals.clashPort.messages.portInUse', {
          port: mixedPortVal,
        })
        setMixedPortVal(
          verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? 10801,
        )
        return
      }
      await Promise.all([
        patchInfo({ 'mixed-port': mixedPortVal }),
        patchVerge({ verge_mixed_port: mixedPortVal }),
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
          style={{
            width: '100%',
            display: 'flex',
            justifyContent: 'flex-end',
            alignItems: 'center',
            padding: '10px',
            boxSizing: 'border-box',
            height: '36px',
            borderBottom: '1px solid var(--divider-color)',
            background: 'var(--background-color)',
            gap: '8px',
            userSelect: 'none',
          }}
        >
          <div
            className="the_titlebar-drag-region"
            data-tauri-drag-region="true"
            style={{
              alignSelf: 'stretch',
              flex: '1 1 0%',
              minWidth: 0,
            }}
          />

          {!drawerOpen && (
            <IconButton
              size="small"
              onClick={() =>
                patchVerge({
                  enable_always_on_top: !verge?.enable_always_on_top,
                })
              }
              sx={{
                flexShrink: 0,
                color: verge?.enable_always_on_top
                  ? 'primary.main'
                  : 'text.primary',
                width: '28px',
                height: '28px',
                p: 0,
                mr: 0.5,
                borderRadius: '6px',
                border: (theme) =>
                  verge?.enable_always_on_top
                    ? `1px solid ${alpha(theme.palette.primary.main, 0.5)}`
                    : '1px solid transparent',
                background: (theme) =>
                  verge?.enable_always_on_top
                    ? `${alpha(theme.palette.primary.main, 0.15)} !important`
                    : 'transparent',
                boxShadow: (theme) =>
                  verge?.enable_always_on_top
                    ? `0 0 calc(8px * var(--vibrancy-factor, 1.0)) ${alpha(theme.palette.primary.main, 0.6)}`
                    : 'none',
                transition: 'all 0.2s ease',
                '&:hover': {
                  background: 'rgba(255, 255, 255, 0.2) !important',
                },
              }}
            >
              <PushPinRounded
                sx={{
                  fontSize: '20px',
                  color: verge?.enable_always_on_top ? '#FF3B30' : '#888888',
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
          )}

          <IconButton
            size="small"
            onClick={() => setDrawerOpen(!drawerOpen)}
            sx={{
              flexShrink: 0,
              color: drawerOpen ? 'primary.main' : 'text.primary',
              width: '28px',
              height: '28px',
              p: 0,
              mr: 1,
              borderRadius: '6px',
              border: (theme) =>
                drawerOpen
                  ? `1px solid ${alpha(theme.palette.primary.main, 0.5)}`
                  : '1px solid transparent',
              background: (theme) =>
                drawerOpen
                  ? `${alpha(theme.palette.primary.main, 0.15)} !important`
                  : 'transparent',
              '&:hover': {
                background: 'rgba(255, 255, 255, 0.2) !important',
              },
            }}
          >
            {drawerOpen ? (
              <CloseRounded sx={{ fontSize: '20px' }} />
            ) : (
              <SettingsRoundedIcon sx={{ fontSize: '20px' }} />
            )}
          </IconButton>

          <WindowControls ref={windowControlsRef} />
        </div>
      ) : null,
    [
      decorated,
      isDecorationsHidden,
      drawerOpen,
      patchVerge,
      verge?.enable_always_on_top,
    ],
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
            <Outlet />
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
        {customTitlebar}

        {/* FEAT-003: GlowBorder — only visible in stealth mode, replaces native chrome */}
        {isDecorationsHidden && <GlowBorder />}

        <div
          className="layout-content"
          style={{
            display: 'flex',
            flexDirection: 'column',
            height:
              decorated || isDecorationsHidden ? '100vh' : 'calc(100vh - 36px)',
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
            {/* 右上角独立控制按钮（齿轮/关闭） */}
            {(decorated || isDecorationsHidden) && (
              <div
                data-no-drag="true"
                style={{
                  position: 'absolute',
                  top: '3px',
                  right: '8px',
                  zIndex: 120,
                }}
              >
                <IconButton
                  size="small"
                  onClick={() => setDrawerOpen(!drawerOpen)}
                  sx={{
                    color: drawerOpen ? 'primary.main' : 'text.primary',
                    width: '28px',
                    height: '28px',
                    p: 0,
                    borderRadius: '6px',
                    border: (theme) =>
                      drawerOpen
                        ? `1px solid ${alpha(theme.palette.primary.main, 0.5)}`
                        : '1px solid transparent',
                    background: (theme) =>
                      drawerOpen
                        ? `${alpha(theme.palette.primary.main, 0.15)} !important`
                        : 'transparent',
                    '&:hover': {
                      background: 'rgba(255, 255, 255, 0.2) !important',
                    },
                  }}
                >
                  {drawerOpen ? (
                    <CloseRounded sx={{ fontSize: '20px' }} />
                  ) : (
                    <SettingsRoundedIcon sx={{ fontSize: '20px' }} />
                  )}
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
                  padding: '3px 44px 2px 8px',
                  position: 'relative',
                  zIndex: 110,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <ActiveNodeStatusCard />
                </div>
                {(decorated || isDecorationsHidden) && (
                  <IconButton
                    size="small"
                    onClick={() =>
                      patchVerge({
                        enable_always_on_top: !verge?.enable_always_on_top,
                      })
                    }
                    sx={{
                      flexShrink: 0,
                      color: verge?.enable_always_on_top
                        ? 'primary.main'
                        : 'text.primary',
                      width: '28px',
                      height: '28px',
                      p: 0,
                      borderRadius: '6px',
                      border: (theme) =>
                        verge?.enable_always_on_top
                          ? `1px solid ${alpha(theme.palette.primary.main, 0.5)}`
                          : '1px solid transparent',
                      background: (theme) =>
                        verge?.enable_always_on_top
                          ? `${alpha(theme.palette.primary.main, 0.15)} !important`
                          : 'transparent',
                      boxShadow: (theme) =>
                        verge?.enable_always_on_top
                          ? `0 0 calc(8px * var(--vibrancy-factor, 1.0)) ${alpha(theme.palette.primary.main, 0.6)}`
                          : 'none',
                      transition: 'all 0.2s ease',
                      '&:hover': {
                        background: 'rgba(255, 255, 255, 0.2) !important',
                      },
                    }}
                  >
                    <PushPinRounded
                      sx={{
                        fontSize: '20px',
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
                )}
              </div>
            )}

            {/*节点组选择列表*/}
            <div style={{ flex: 1, overflow: 'hidden' }}>
              <ProxyGroups
                mode={clashConfig?.mode?.toLowerCase() || 'rule'}
                isChainMode={false}
                chainConfigData={null}
              />
            </div>

            {/* Settings Sliding Drawer (slides internal left-downwards) */}
            <div
              className="theme-panel"
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: '100%',
                zIndex: 100,
                display: 'flex',
                transition:
                  'transform 0.4s cubic-bezier(0.1, 0.9, 0.2, 1), opacity 0.3s ease-in-out',
                transform: drawerOpen
                  ? 'translate(0, 0) scale(1)'
                  : 'translate(100%, -100%) scale(0.95)',
                opacity: drawerOpen ? 1 : 0,
                pointerEvents: drawerOpen ? 'auto' : 'none',
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
                  gap: 1,
                  overflow: 'hidden',
                  pr: 1,
                  borderRight: (theme) => `1px solid ${theme.palette.divider}`,
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

                {/* Section 2: Takeover Mode (三态互斥单选) */}
                <TakeoverModeCard
                  activeIndex={activeIndex}
                  language={language}
                  handleTakeoverModeChange={handleTakeoverModeChange}
                />

                {/* Section 2b: Routing Preference */}
                <RoutingPreferenceCard
                  policyActiveIndex={policyActiveIndex}
                  language={language}
                  handleRuleFallbackChange={handleRuleFallbackChange}
                />

                {/* Section 3: Minimal Settings */}
                <BasicSettingsCard
                  verge={verge}
                  clashConfig={clashConfig}
                  patchVerge={patchVerge}
                  handleAllowLanChange={handleAllowLanChange}
                  mixedPortVal={mixedPortVal}
                  setMixedPortVal={setMixedPortVal}
                  handleSavePort={handleSavePort}
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
              {/* Right Connections column (自适应 flex: 1) */}
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
                  left: '70px',
                  width: '97.5px',
                  height: '24px',
                  zIndex: 200,
                  boxSizing: 'border-box',
                  fontSize: '11px',
                  fontWeight: 'bold',
                  fontFamily: 'var(--control-font-family)',
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
          </div>

          {/* Lower Pane: Constant Traffic Dashboard (Fixed Height - 30px) */}
          <div
            style={{
              flex: isMinimalWidth ? '0 0 100px' : '0 0 135px',
              height: isMinimalWidth ? '100px' : '135px',
              background: 'inherit',
              padding: isMinimalWidth ? '3px 6px 2px 6px' : '8px 12px 2px 12px',
              display: 'flex',
              gap: isMinimalWidth ? '6px' : '12px',
              overflow: 'hidden',
              boxSizing: 'border-box',
            }}
          >
            <MiniTrafficPanel isMinimalWidth={isMinimalWidth} />
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
              backgroundColor: 'transparent',
              backgroundImage: 'none',
              boxShadow: 'none',
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
              backgroundColor: 'transparent',
              backgroundImage: 'none',
              boxShadow: 'none',
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
