import {
  Box,
  List,
  Menu,
  MenuItem,
  Paper,
  SvgIcon,
  ThemeProvider,
  Typography,
  IconButton,
  Button,
  ButtonGroup,
  Chip,
  CircularProgress,
  TextField,
  Dialog,
  Select,
  ListItem,
  ListItemText,
} from '@mui/material'
import { alpha } from '@mui/material'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useLocation, useNavigate } from 'react-router'

import { BaseErrorBoundary, BaseSearchBox, BaseEmpty, Switch } from '@/components/base'
import { WindowControls } from '@/components/layout/window-controller'
import { useI18n } from '@/hooks/use-i18n'
import { useVerge } from '@/hooks/use-verge'
import { useWindowDecorations } from '@/hooks/use-window'
import { useThemeMode } from '@/services/states'
import getSystem from '@/utils/get-system'

import {
  useCustomTheme,
  useLayoutEvents,
  useLoadingOverlay,
} from './_layout/hooks'
import { handleNoticeMessage } from './_layout/utils'
import { NoticeManager } from '@/components/layout/notice-manager'

import { useProfiles } from '@/hooks/use-profiles'
import { useProxiesData, useClashConfigData, useAppRefreshers } from '@/providers/app-data-context'
import { useSystemProxyState } from '@/hooks/use-system-proxy-state'
import { useSystemState } from '@/hooks/use-system-state'
import { useServiceInstaller } from '@/hooks/use-service-installer'
import { useClashInfo } from '@/hooks/use-clash'
import { useConnectionData } from '@/hooks/use-connection-data'
import { useConnectionSetting } from '@/hooks/use-connection-setting'
import { useTrafficData } from '@/hooks/use-traffic-data'

import {
  importProfile,
  updateProfile,
  deleteProfile,
  enhanceProfiles,
  isPortInUse,
  patchClashMode,
} from '@/services/cmds'
import { healthcheckProxyProvider, closeAllConnections, selectNodeForGroup } from 'tauri-plugin-mihomo-api'
import delayManager from '@/services/delay'
import parseTraffic from '@/utils/parse-traffic'

import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { ConnectionTable } from '@/components/connection/connection-table'
import { ConnectionDetail } from '@/components/connection/connection-detail'
import { EnhancedCanvasTrafficGraph } from '@/components/home/enhanced-canvas-traffic-graph'
import { useVisibility } from '@/hooks/use-visibility'
import { showNotice } from '@/services/notice-service'

import {
  ArrowDownwardRounded,
  ArrowUpwardRounded,
  SettingsRounded as SettingsRoundedIcon,
  CloseRounded,
  RefreshRounded,
  DeleteRounded,
  Shuffle as ShuffleIcon,
  WifiOff as SignalError,
  SignalWifi3Bar as SignalGood,
  SignalWifi2Bar as SignalMedium,
  SignalWifi0Bar as SignalNone,
  SignalWifi4Bar as SignalStrong,
  SignalWifi1Bar as SignalWeak,
  SaveRounded,
} from '@mui/icons-material'
import LogsPage from './logs'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)

const OS = getSystem()

export const portableFlag = false

// Delay Helpers
function getSignalIcon(delay: number) {
  if (delay === -2) return { icon: <SignalNone />, text: '测试中', color: 'text.secondary' }
  if (delay === -1) return { icon: <SignalNone />, text: '未测试', color: 'text.secondary' }
  if (delay > 1e5) return { icon: <SignalError />, text: '错误', color: 'error.main' }
  if (delay === 0 || delay >= 10000) return { icon: <SignalError />, text: '超时', color: 'error.main' }
  if (delay >= 500) return { icon: <SignalWeak />, text: '延迟较高', color: 'error.main' }
  if (delay >= 300) return { icon: <SignalMedium />, text: '延迟中等', color: 'warning.main' }
  if (delay >= 200) return { icon: <SignalGood />, text: '延迟良好', color: 'info.main' }
  return { icon: <SignalStrong />, text: '延迟极佳', color: 'success.main' }
}

function convertDelayColor(delayValue: number): 'success' | 'warning' | 'error' | 'primary' | 'default' {
  const colorStr = delayManager.formatDelayColor(delayValue)
  if (!colorStr) return 'default'
  const mainColor = colorStr.split('.')[0]
  switch (mainColor) {
    case 'success': return 'success'
    case 'warning': return 'warning'
    case 'error': return 'error'
    case 'primary': return 'primary'
    default: return 'default'
  }
}

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
      list.sort((a, b) => b.curUpload! - a.curUpload!),
  },
  {
    id: 'downloadSpeed',
    labelKey: 'connections.components.order.downloadSpeed',
    fn: (list: IConnectionsItem[]) =>
      list.sort((a, b) => b.curDownload! - a.curDownload!),
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

// Active Node Card
const ActiveNodeStatusCard = () => {
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const { decorated } = useWindowDecorations()
  
  const primaryGroup = useMemo(() => {
    const groups = proxies?.groups || []
    const primaryKeywords = ['auto', 'select', 'proxy', '节点选择', '自动选择']
    return groups.find((group: any) =>
      primaryKeywords.some((keyword) => group.name.toLowerCase().includes(keyword.toLowerCase()))
    ) || groups.filter((g: any) => g.name !== 'GLOBAL')[0] || groups[0]
  }, [proxies])

  const activeNodeName = primaryGroup?.now || ''
  const activeNodeRecord = proxies?.records?.[activeNodeName]

  const delay = useMemo(() => {
    if (!activeNodeName || !primaryGroup?.name || !proxies?.records?.[activeNodeName]) return -1
    return delayManager.getDelayFix(proxies.records[activeNodeName], primaryGroup.name)
  }, [proxies, activeNodeName, primaryGroup])

  const [testing, setTesting] = useState(false)
  const handleTestDelay = async (e: React.MouseEvent) => {
    e.stopPropagation()
    if (!activeNodeName || !primaryGroup?.name) return
    setTesting(true)
    try {
      if (activeNodeRecord?.provider) {
        await healthcheckProxyProvider(activeNodeRecord.provider)
      } else {
        await delayManager.checkDelay(activeNodeName, primaryGroup.name, 10000)
      }
      refreshProxy()
    } catch (err) {
      console.error(err)
    } finally {
      setTesting(false)
    }
  }

  const signalInfo = getSignalIcon(delay)
  const delayColor = convertDelayColor(delay)

  return (
    <Paper
      className="theme-crystal-card"
      sx={{
        m: 1,
        mb: 0.5,
        mr: decorated ? '44px' : 1, // Avoid overlap with Settings gear button when decorated
        p: '6px 12px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 2,
        background: (theme) => `${theme.palette.background.paper} !important`,
        border: (theme) => `1px solid ${theme.palette.divider} !important`,
        boxShadow: 'none !important',
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <Typography variant="caption" color="text.secondary" sx={{ fontSize: '11px' }}>
          当前活跃出口节点：
        </Typography>
        <Typography variant="body2" sx={{ fontWeight: 'bold', fontSize: '12px', color: 'text.primary' }}>
          {activeNodeName || '未选择节点 (直接连接)'}
        </Typography>
      </Box>
      {activeNodeName && (
        <Chip
          size="small"
          icon={testing ? <CircularProgress size={10} color="inherit" /> : signalInfo.icon}
          label={testing ? '测试中...' : delayManager.formatDelay(delay)}
          color={delayColor}
          onClick={handleTestDelay}
          sx={{
            fontSize: '11px',
            height: '20px',
            fontWeight: 600,
            cursor: 'pointer',
            bgcolor: testing ? undefined : alpha(signalInfo.color === 'success.main' ? '#4caf50' : signalInfo.color === 'warning.main' ? '#ff9800' : '#f44336', 0.12),
            color: signalInfo.color === 'text.secondary' ? 'text.secondary' : signalInfo.color,
            '& .MuiChip-icon': {
              color: 'inherit',
              fontSize: '12px',
            }
          }}
        />
      )}
    </Paper>
  )
}

// Mini Traffic Panel
const MiniTrafficPanel = () => {
  const mode = useThemeMode()
  const { t } = useTranslation()
  const pageVisible = useVisibility()
  const { response: { data: traffic } } = useTrafficData({ enabled: pageVisible })
  const { response: { data: connections } } = useConnectionData()
  const trafficRef = useRef<any>(null)

  useEffect(() => {
    if (trafficRef.current && traffic) {
      trafficRef.current.appendData({
        up: traffic.up || 0,
        down: traffic.down || 0,
      })
    }
  }, [traffic])

  const [upVal, upUnit] = parseTraffic(traffic?.up || 0)
  const [downVal, downUnit] = parseTraffic(traffic?.down || 0)
  const [upTotalVal, upTotalUnit] = parseTraffic(connections?.uploadTotal || 0)
  const [downTotalVal, downTotalUnit] = parseTraffic(connections?.downloadTotal || 0)

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', width: '100%', height: '100%', boxSizing: 'border-box', overflow: 'hidden' }}>
      {/* Traffic Graph (Full Width) */}
      <Box sx={{ flex: 1, width: '100%', minHeight: 0, position: 'relative' }}>
        <EnhancedCanvasTrafficGraph ref={trafficRef} />
      </Box>

      {/* Metrics Row (Single Line Below Graph - Raised 3D Button style) */}
      <Box sx={{ 
        display: 'flex', 
        width: '100%', 
        height: '32px', 
        alignItems: 'center', 
        justifyContent: 'space-between', 
        borderTop: (theme) => `1px solid ${theme.palette.divider}`,
        mt: 1,
        pt: 1,
        px: 1,
        gap: 1,
        boxSizing: 'border-box'
      }}>
        {/* Upload Group */}
        <Box sx={{ display: 'flex', flex: 1, minWidth: 0 }}>
          {/* Upload Speed */}
          <Box sx={{ 
            display: 'flex', 
            alignItems: 'center', 
            justifyContent: 'center',
            flex: 1,
            gap: 0.5,
            height: '24px',
            px: 1.25,
            borderTopLeftRadius: '6px',
            borderBottomLeftRadius: '6px',
            borderTopRightRadius: 0,
            borderBottomRightRadius: 0,
            border: `1px solid ${mode === 'light' ? 'rgba(212, 175, 55, 0.35)' : 'rgba(212, 175, 55, 0.5)'}`,
            borderRight: 'none',
            bgcolor: mode === 'light' ? 'rgba(212, 175, 55, 0.08)' : 'rgba(212, 175, 55, 0.15)',
            boxShadow: mode === 'light'
              ? '0 1.5px 2px rgba(212, 175, 55, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.6)'
              : '0 1.5px 2px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.05)',
            whiteSpace: 'nowrap'
          }}>
            <ArrowUpwardRounded sx={{ color: '#D4AF37', fontSize: 13 }} />
            <Typography sx={{ fontSize: '10px', color: mode === 'light' ? '#8c7010' : '#e5c158', fontWeight: 'bold', whiteSpace: 'nowrap' }}>上传:</Typography>
            <Typography sx={{ fontWeight: 'bold', fontSize: '13px', color: '#D4AF37', whiteSpace: 'nowrap' }}>
              {upVal} <span style={{ fontSize: '9px', fontWeight: 'normal', color: mode === 'light' ? '#8c7010' : '#b29645' }}>{upUnit}/s</span>
            </Typography>
          </Box>

          {/* Upload Total */}
          <Box sx={{ 
            display: 'flex', 
            alignItems: 'center', 
            justifyContent: 'center',
            flex: 1,
            gap: 0.5,
            height: '24px',
            px: 1.25,
            borderTopRightRadius: '6px',
            borderBottomRightRadius: '6px',
            borderTopLeftRadius: 0,
            borderBottomLeftRadius: 0,
            border: `1px solid ${mode === 'light' ? 'rgba(212, 175, 55, 0.35)' : 'rgba(212, 175, 55, 0.5)'}`,
            bgcolor: mode === 'light' ? 'rgba(212, 175, 55, 0.08)' : 'rgba(212, 175, 55, 0.15)',
            boxShadow: mode === 'light'
              ? '0 1.5px 2px rgba(212, 175, 55, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.6)'
              : '0 1.5px 2px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.05)',
            whiteSpace: 'nowrap'
          }}>
            <Typography sx={{ fontSize: '10px', color: mode === 'light' ? '#8c7010' : '#e5c158', fontWeight: 'bold', whiteSpace: 'nowrap' }}>总量:</Typography>
            <Typography sx={{ fontSize: '13px', fontWeight: 'bold', color: '#D4AF37', whiteSpace: 'nowrap' }}>
              {upTotalVal} <span style={{ fontSize: '9px', color: mode === 'light' ? '#8c7010' : '#b29645', fontWeight: 'normal' }}>{upTotalUnit}</span>
            </Typography>
          </Box>
        </Box>

        {/* Download Group */}
        <Box sx={{ display: 'flex', flex: 1, minWidth: 0 }}>
          {/* Download Speed */}
          <Box sx={{ 
            display: 'flex', 
            alignItems: 'center', 
            justifyContent: 'center',
            flex: 1,
            gap: 0.5,
            height: '24px',
            px: 1.25,
            borderTopLeftRadius: '6px',
            borderBottomLeftRadius: '6px',
            borderTopRightRadius: 0,
            borderBottomRightRadius: 0,
            border: `1px solid ${mode === 'light' ? 'rgba(0, 132, 255, 0.3)' : 'rgba(0, 132, 255, 0.5)'}`,
            borderRight: 'none',
            bgcolor: mode === 'light' ? 'rgba(0, 132, 255, 0.06)' : 'rgba(0, 132, 255, 0.12)',
            boxShadow: mode === 'light'
              ? '0 1.5px 2px rgba(0, 132, 255, 0.1), inset 0 1px 0 rgba(255, 255, 255, 0.6)'
              : '0 1.5px 2px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.05)',
            whiteSpace: 'nowrap'
          }}>
            <ArrowDownwardRounded sx={{ color: '#0084FF', fontSize: 13 }} />
            <Typography sx={{ fontSize: '10px', color: mode === 'light' ? '#0052a3' : '#66b2ff', fontWeight: 'bold', whiteSpace: 'nowrap' }}>下载:</Typography>
            <Typography sx={{ fontWeight: 'bold', fontSize: '13px', color: '#0084FF', whiteSpace: 'nowrap' }}>
              {downVal} <span style={{ fontSize: '9px', fontWeight: 'normal', color: mode === 'light' ? '#0052a3' : '#8cd9ff' }}>{downUnit}/s</span>
            </Typography>
          </Box>

          {/* Download Total */}
          <Box sx={{ 
            display: 'flex', 
            alignItems: 'center', 
            justifyContent: 'center',
            flex: 1,
            gap: 0.5,
            height: '24px',
            px: 1.25,
            borderTopRightRadius: '6px',
            borderBottomRightRadius: '6px',
            borderTopLeftRadius: 0,
            borderBottomLeftRadius: 0,
            border: `1px solid ${mode === 'light' ? 'rgba(0, 132, 255, 0.3)' : 'rgba(0, 132, 255, 0.5)'}`,
            bgcolor: mode === 'light' ? 'rgba(0, 132, 255, 0.06)' : 'rgba(0, 132, 255, 0.12)',
            boxShadow: mode === 'light'
              ? '0 1.5px 2px rgba(0, 132, 255, 0.1), inset 0 1px 0 rgba(255, 255, 255, 0.6)'
              : '0 1.5px 2px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.05)',
            whiteSpace: 'nowrap'
          }}>
            <Typography sx={{ fontSize: '10px', color: mode === 'light' ? '#0052a3' : '#66b2ff', fontWeight: 'bold', whiteSpace: 'nowrap' }}>总量:</Typography>
            <Typography sx={{ fontWeight: 'bold', fontSize: '13px', color: '#0084FF', whiteSpace: 'nowrap' }}>
              {downTotalVal} <span style={{ fontSize: '9px', color: mode === 'light' ? '#0052a3' : '#8cd9ff', fontWeight: 'normal' }}>{downTotalUnit}</span>
            </Typography>
          </Box>
        </Box>
      </Box>
    </Box>
  )
}

const Layout = () => {
  const mode = useThemeMode()
  const { t } = useTranslation()
  const { theme } = useCustomTheme()
  const { verge, mutateVerge, patchVerge } = useVerge()
  const { language } = verge ?? {}
  const { switchLanguage } = useI18n()
  const { decorated } = useWindowDecorations()
  const { pathname } = useLocation()
  const windowControlsRef = useRef<any>(null)

  // Drawer Toggle State
  const [drawerOpen, setDrawerOpen] = useState(false)

  // Profiles State
  const [url, setUrl] = useState('')
  const [profileLoading, setProfileLoading] = useState(false)
  const { profiles = {}, mutateProfiles, activateSelected, patchProfiles } = useProfiles()
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

  // Takeover Mode States
  const { indicator: systemProxyIndicator, toggleSystemProxy } = useSystemProxyState()
  const { isTunModeAvailable, mutateSystemState } = useSystemState()
  const { installServiceAndRestartCore } = useServiceInstaller()
  const { enable_tun_mode } = verge ?? {}
  const currentMode = enable_tun_mode ? 'tun' : systemProxyIndicator ? 'system' : 'manual'
  const activeIndex = currentMode === 'tun' ? 2 : currentMode === 'system' ? 1 : 0

  // Port State
  const { clashInfo, patchInfo } = useClashInfo()
  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()

  const modeKey = clashConfig?.mode?.toLowerCase() || 'rule'
  const policyActiveIndex = modeKey === 'direct' ? 2 : modeKey === 'global' ? 1 : 0

  const themeModeVal = verge?.theme_mode || 'system'
  const themeActiveIndex = themeModeVal === 'dark' ? 2 : themeModeVal === 'light' ? 1 : 0

  const [mixedPortVal, setMixedPortVal] = useState(verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? 10801)

  // Minimal Settings States
  const [notificationsEnabled, setNotificationsEnabled] = useState(
    () => localStorage.getItem('clash-verge-enable-notification') !== 'false'
  )

  // Connections manager states
  const [match, setMatch] = useState<(input: string) => boolean>(() => () => true)
  const [curOrderOpt, setCurOrderOpt] = useState<OrderKey>('default')
  const [connectionsType, setConnectionsType] = useState<'active' | 'closed'>('active')
  const { response: { data: connectionsData }, clearClosedConnections } = useConnectionData()
  const [isColumnManagerOpen, setIsColumnManagerOpen] = useState(false)
  const detailRef = useRef<any>(null)
  
  const filterConn = useMemo(() => {
    const orderFunc = orderFunctionMap[curOrderOpt]
    const conns = (connectionsType === 'active' ? connectionsData?.activeConnections : connectionsData?.closedConnections) ?? []
    let matchConns = conns.filter((conn) => {
      const { host, destinationIP, process } = conn.metadata
      return match(host || '') || match(destinationIP || '') || match(process || '')
    })
    if (orderFunc) matchConns = orderFunc(matchConns)
    return matchConns
  }, [connectionsData, connectionsType, match, curOrderOpt])

  const handleSearch = useCallback((match: (content: string) => boolean) => {
    setMatch(() => match)
  }, [])

  // Sync mixed port val
  useEffect(() => {
    if (verge?.verge_mixed_port) {
      setMixedPortVal(verge.verge_mixed_port)
    } else if (clashInfo?.mixed_port) {
      setMixedPortVal(clashInfo.mixed_port)
    }
  }, [verge?.verge_mixed_port, clashInfo?.mixed_port])

  useEffect(() => {
    if (language) {
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguage(language)
    }
  }, [language, switchLanguage])

  // Automatically enhance profile when it is loaded or switched (flatten to single PROXY group)
  useEffect(() => {
    if (currentProfileUid) {
      enhanceProfiles()
        .then(() => {
          console.log(`[Layout] Enhanced active profile: ${currentProfileUid}`);
          activateSelected();
        })
        .catch((err) => {
          console.error(`[Layout] Failed to enhance profile ${currentProfileUid}:`, err);
        });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentProfileUid]);

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
    if (!/^https?:\/\//i.test(url)) {
      showNotice.error('profiles.page.feedback.errors.invalidUrl')
      return
    }
    setProfileLoading(true)
    try {
      await importProfile(url)
      showNotice.success('shared.feedback.notifications.importSuccess')
      setUrl('')
      await mutateProfiles()
    } catch (err) {
      try {
        await importProfile(url, { with_proxy: false, self_proxy: true })
        showNotice.success('shared.feedback.notifications.importWithClashProxy')
        setUrl('')
        await mutateProfiles()
      } catch (retryErr) {
        showNotice.error('profiles.page.feedback.notifications.importFail', String(retryErr))
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
      showNotice.success('profiles.page.feedback.notifications.profileSwitched', 1000)
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

  // Takeover actions
  const handleTakeoverModeChange = async (targetMode: 'manual' | 'system' | 'tun') => {
    const currentMode = enable_tun_mode ? 'tun' : systemProxyIndicator ? 'system' : 'manual'
    if (targetMode === currentMode) return

    if (targetMode === 'manual') {
      try {
        if (systemProxyIndicator) {
          await toggleSystemProxy(false)
        }
        if (enable_tun_mode) {
          await patchVerge({ enable_tun_mode: false })
        }
        showNotice.success('已切换至手动模式')
      } catch (err) {
        showNotice.error(err)
      }
    } else if (targetMode === 'system') {
      try {
        if (enable_tun_mode) {
          await patchVerge({ enable_tun_mode: false })
        }
        if (!systemProxyIndicator) {
          await toggleSystemProxy(true)
        }
        showNotice.success('已开启系统代理')
      } catch (err) {
        showNotice.error(err)
      }
    } else if (targetMode === 'tun') {
      if (systemProxyIndicator) {
        try {
          await toggleSystemProxy(false)
        } catch (err) {
          showNotice.error(err)
          return
        }
      }

      if (!isTunModeAvailable) {
        try {
          showNotice.info('正在自动安装/配置虚拟网卡系统服务...')
          await installServiceAndRestartCore()
          await mutateSystemState()
        } catch (err) {
          showNotice.error('TUN 模式服务配置失败，请尝试以管理员身份运行。')
          return
        }
      }

      try {
        await patchVerge({ enable_tun_mode: true })
        showNotice.success('已开启 TUN 模式')
      } catch (err) {
        showNotice.error(err)
      }
    }
  }

  // Port update
  const handleSavePort = async () => {
    if (mixedPortVal === verge?.verge_mixed_port && mixedPortVal === clashInfo?.mixed_port) return
    try {
      const inUse = await isPortInUse(mixedPortVal)
      if (inUse) {
        showNotice.error('settings.modals.clashPort.messages.portInUse', { port: mixedPortVal })
        return
      }
      await Promise.all([
        patchInfo({ 'mixed-port': mixedPortVal }),
        patchVerge({ verge_mixed_port: mixedPortVal })
      ])
      showNotice.success('代理端口已保存并重载')
    } catch (err) {
      showNotice.error(err)
    }
  }

  // Logs dialog state
  const [logsOpen, setLogsOpen] = useState(false)

  // Custom Titlebar Render
  const customTitlebar = useMemo(
    () =>
      !decorated ? (
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
              flex: '1 1 auto',
            }}
          />
          
          <IconButton
            size="small"
            onClick={() => setDrawerOpen(!drawerOpen)}
            sx={{
              color: drawerOpen ? 'primary.main' : 'text.primary',
              p: 0.5,
              mr: 1,
              borderRadius: '6px',
              border: (theme) => drawerOpen ? `1px solid ${alpha(theme.palette.primary.main, 0.5)}` : '1px solid transparent',
              background: (theme) => drawerOpen ? `${alpha(theme.palette.primary.main, 0.15)} !important` : 'transparent',
              '&:hover': {
                background: 'rgba(255, 255, 255, 0.2) !important',
              }
            }}
          >
            {drawerOpen ? <CloseRounded fontSize="small" /> : <SettingsRoundedIcon fontSize="small" />}
          </IconButton>
          
          <WindowControls ref={windowControlsRef} />
        </div>
      ) : null,
    [decorated, drawerOpen],
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

        {/* Double-Pane Dashboard */}
        <div
          className="layout-content"
          style={{
            display: 'flex',
            flexDirection: 'column',
            height: decorated ? '100vh' : 'calc(100vh - 36px)',
            width: '100vw',
            overflow: 'hidden',
            position: 'relative',
          }}
        >
          {/* Settings button when decorated is true */}
          {decorated && (
            <IconButton
              size="small"
              onClick={() => setDrawerOpen(!drawerOpen)}
              sx={{
                position: 'absolute',
                top: 0,
                right: 0,
                width: '36px',
                height: '36px',
                zIndex: 101,
                borderRadius: 0,
                color: drawerOpen ? 'primary.main' : 'text.primary',
                border: (theme) => drawerOpen ? `1px solid ${alpha(theme.palette.primary.main, 0.5)}` : '1px solid transparent',
                background: (theme) => drawerOpen ? `${alpha(theme.palette.primary.main, 0.15)} !important` : 'transparent',
                '&:hover': {
                  background: 'rgba(255, 255, 255, 0.2) !important',
                }
              }}
            >
              {drawerOpen ? <CloseRounded fontSize="small" /> : <SettingsRoundedIcon fontSize="small" />}
            </IconButton>
          )}

          {/* Upper Pane: Node Selection (80%) */}
          <div
            style={{
              flex: '80 0 0%',
              height: '80%',
              position: 'relative',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
            }}
          >
            {/*置顶当前节点*/}
            <ActiveNodeStatusCard />

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
                transition: 'transform 0.4s cubic-bezier(0.1, 0.9, 0.2, 1), opacity 0.3s ease-in-out',
                transform: drawerOpen ? 'translate(0, 0) scale(1)' : 'translate(100%, -100%) scale(0.95)',
                opacity: drawerOpen ? 1 : 0,
                pointerEvents: drawerOpen ? 'auto' : 'none',
                boxSizing: 'border-box',
                padding: '12px',
                gap: '12px',
                background: mode === 'light' ? '#f0f5ff' : '#1e2438',
                border: '4px double var(--divider-color)',
              }}
            >
              {/* Left Settings Column (200px width) */}
              <Box
                sx={{
                  flex: '0 0 200px',
                  width: '200px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 1,
                  overflowY: 'auto',
                  pr: 1,
                  borderRight: (theme) => `1px solid ${theme.palette.divider}`,
                }}
              >
                {/* Section 1: Subscriptions Import */}
                <Box sx={{ p: 1, border: '1px solid', borderColor: 'divider', borderRadius: '8px' }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 'bold', mb: 0.75, fontSize: '11px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    订阅与机场配置
                    {profileLoading && <CircularProgress size={10} />}
                  </Typography>
                  <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5, mb: 1 }}>
                    <TextField
                      placeholder="填入订阅链接 (YAML)"
                      size="small"
                      value={url}
                      onChange={(e) => setUrl(e.target.value)}
                      slotProps={{ htmlInput: { style: { paddingTop: '4px', paddingBottom: '4px', fontSize: '11px', height: '26px', boxSizing: 'border-box' } } }}
                      sx={{ width: '100%' }}
                    />
                    <Box sx={{ display: 'flex', justifyContent: 'center', width: '100%' }}>
                      <Button
                        variant="contained"
                        color="primary"
                        onClick={handleImportProfile}
                        sx={{
                          fontSize: 11,
                          height: 26,
                          textTransform: 'none',
                          px: 2,
                        }}
                        disabled={profileLoading}
                      >
                        导入订阅链接（YAML）
                      </Button>
                    </Box>
                  </Box>
                  {/* Profiles List */}
                  <Box sx={{ maxHeight: 110, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 0.5 }}>
                    {profileItems.map((item) => {
                      const isActive = item.uid === currentProfileUid
                      const extra = item.extra
                      const hasExtra = !!extra
                      const { upload = 0, download = 0, total = 0 } = extra ?? {}
                      const progress = total > 0 ? Math.min(Math.round(((download + upload) * 100) / (total + 0.01)), 100) : 0
                      
                      const formatTraffic = (num?: number) => {
                        if (typeof num !== 'number') return '-'
                        const [val, unit] = parseTraffic(num)
                        return `${val}${unit}`
                      }

                      const formatExpire = (expire?: number) => {
                        if (!expire) return '-'
                        return dayjs(expire * 1000).format('YYYY-MM-DD')
                      }

                      return (
                        <Box
                          key={item.uid}
                          onClick={() => handleSelectProfile(item.uid)}
                          sx={{
                            display: 'flex',
                            flexDirection: 'column',
                            p: '6px 8px',
                            mb: 0.5,
                            borderRadius: '6px',
                            cursor: 'pointer',
                            bgcolor: (theme) => theme.palette.mode === 'light' ? '#ffffff' : '#282A36',
                            borderLeft: (theme) => `3px solid ${isActive ? theme.palette.primary.main : 'transparent'}`,
                            boxShadow: '0 1px 3px rgba(0,0,0,0.05)',
                            transition: 'all 0.2s',
                            '&:hover': {
                              bgcolor: 'action.hover',
                            }
                          }}
                        >
                          {/* Line 1: Title & Actions */}
                          <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
                            <Typography
                              variant="body2"
                              sx={{
                                fontSize: '11px',
                                fontWeight: isActive ? 600 : 400,
                                color: isActive ? 'primary.main' : 'text.primary',
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap',
                                maxWidth: '70%'
                              }}
                              title={item.name}
                            >
                              {item.name || '未命名配置'}
                            </Typography>
                            <Box sx={{ display: 'flex', gap: 0.25, alignItems: 'center' }}>
                              {item.type === 'remote' && (
                                <IconButton
                                  size="small"
                                  onClick={(e) => handleUpdateProfile(item.uid, e)}
                                  sx={{ p: 0.1, color: isActive ? 'primary.main' : 'text.secondary' }}
                                >
                                  <RefreshRounded sx={{ fontSize: 12 }} />
                                </IconButton>
                              )}
                              <IconButton
                                size="small"
                                onClick={(e) => handleDeleteProfile(item.uid, e)}
                                sx={{ p: 0.1, color: 'error.main' }}
                              >
                                <DeleteRounded sx={{ fontSize: 12 }} />
                              </IconButton>
                            </Box>
                          </Box>

                          {/* Line 2: Traffic & Expiration (Only for remote with extra data) */}
                          {item.type === 'remote' && hasExtra && (
                            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mt: 0.25, fontSize: '9px', color: 'text.secondary' }}>
                              <span>
                                {formatTraffic(upload + download)} / {formatTraffic(total)}
                              </span>
                              <span>
                                {extra?.expire ? formatExpire(extra.expire) : '-'}
                              </span>
                            </Box>
                          )}

                          {/* Line 3: Traffic progress bar (Only if total traffic > 0) */}
                          {item.type === 'remote' && total > 0 && (
                            <Box sx={{ width: '100%', height: 2, bgcolor: 'action.hover', borderRadius: 1, mt: 0.5, overflow: 'hidden' }}>
                              <Box sx={{ width: `${progress}%`, height: '100%', bgcolor: 'primary.main' }} />
                            </Box>
                          )}
                        </Box>
                      )
                    })}
                  </Box>
                </Box>

                {/* Section 2: Takeover Mode (三态互斥单选) */}
                <Box sx={{ p: 1, border: '1px solid', borderColor: 'divider', borderRadius: '8px' }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 'bold', mb: 0.75, fontSize: '11px' }}>
                    代理模式
                  </Typography>
                  <Box
                    sx={{
                      position: 'relative',
                      display: 'flex',
                      alignItems: 'center',
                      bgcolor: 'action.hover',
                      borderRadius: '6px',
                      p: '2px',
                      mb: 1,
                      height: 28,
                      userSelect: 'none',
                    }}
                  >
                    {/* Sliding Background Indicator */}
                    <Box
                      sx={{
                        position: 'absolute',
                        left: 0,
                        top: 0,
                        width: '33.333%',
                        height: '100%',
                        zIndex: 0,
                        transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                        transform: `translate3d(${activeIndex * 100}%, 0, 0)`,
                      }}
                    >
                      <Box
                        sx={{
                          height: 'calc(100% - 4px)',
                          margin: '2px',
                          bgcolor: 'primary.main',
                          borderRadius: '4px',
                          boxShadow: '0px 1px 3px rgba(0, 0, 0, 0.15)',
                        }}
                      />
                    </Box>

                    {/* Manual Mode Option */}
                    <Box
                      onClick={() => handleTakeoverModeChange('manual')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: activeIndex === 0 ? 'primary.contrastText' : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: activeIndex === 0 ? 'bold' : 'normal',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      手动模式
                    </Box>

                    {/* System Proxy Option */}
                    <Box
                      onClick={() => handleTakeoverModeChange('system')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: activeIndex === 1 ? 'primary.contrastText' : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: activeIndex === 1 ? 'bold' : 'normal',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      系统代理
                    </Box>

                    {/* TUN Mode Option */}
                    <Box
                      onClick={() => handleTakeoverModeChange('tun')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: activeIndex === 2 ? 'primary.contrastText' : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: activeIndex === 2 ? 'bold' : 'normal',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      TUN 模式
                    </Box>
                  </Box>
                  <Typography variant="subtitle2" sx={{ fontWeight: 'bold', mb: 0.75, fontSize: '11px' }}>
                    代理策略
                  </Typography>
                  <ButtonGroup fullWidth size="small" sx={{ display: 'none' }}>
                    {/* Keep old ButtonGroup hidden to avoid refactor side-effects if any */}
                  </ButtonGroup>
                  <Box
                    sx={{
                      position: 'relative',
                      display: 'flex',
                      alignItems: 'center',
                      bgcolor: 'action.hover',
                      borderRadius: '6px',
                      p: '2px',
                      height: 28,
                      userSelect: 'none',
                    }}
                  >
                    {/* Sliding Background Indicator */}
                    <Box
                      sx={{
                        position: 'absolute',
                        left: 0,
                        top: 0,
                        width: '33.333%',
                        height: '100%',
                        zIndex: 0,
                        transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                        transform: `translate3d(${policyActiveIndex * 100}%, 0, 0)`,
                      }}
                    >
                      <Box
                        sx={{
                          height: 'calc(100% - 4px)',
                          margin: '2px',
                          bgcolor: 'primary.main',
                          borderRadius: '4px',
                          boxShadow: '0px 1px 3px rgba(0, 0, 0, 0.15)',
                        }}
                      />
                    </Box>

                    {/* Rule Option */}
                    <Box
                      onClick={async () => {
                        await patchClashMode('rule')
                        refreshClashConfig()
                      }}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: policyActiveIndex === 0 ? 'primary.contrastText' : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: policyActiveIndex === 0 ? 'bold' : 'normal',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      规则模式
                    </Box>

                    {/* Global Option */}
                    <Box
                      onClick={async () => {
                        await patchClashMode('global')
                        try {
                          await selectNodeForGroup('GLOBAL', 'PROXY')
                        } catch (e) {
                          console.error('Failed to set GLOBAL target to PROXY:', e)
                        }
                        refreshClashConfig()
                      }}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: policyActiveIndex === 1 ? 'primary.contrastText' : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: policyActiveIndex === 1 ? 'bold' : 'normal',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      全局代理
                    </Box>

                    {/* Direct Option */}
                    <Box
                      onClick={async () => {
                        await patchClashMode('direct')
                        refreshClashConfig()
                      }}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: policyActiveIndex === 2 ? 'primary.contrastText' : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: policyActiveIndex === 2 ? 'bold' : 'normal',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      全局直连
                    </Box>
                  </Box>
                </Box>

                {/* Section 3: Minimal Settings */}
                <Box sx={{ p: 1, border: '1px solid', borderColor: 'divider', borderRadius: '8px' }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 'bold', mb: 0.5, fontSize: '11px' }}>
                    基础设置
                  </Typography>
                  <List dense sx={{ py: 0 }}>
                    <ListItem sx={{ py: 0.1, px: 0.5, display: 'flex', justifyContent: 'space-between' }}>
                      <Typography variant="caption" sx={{ fontSize: '11px' }}>开机自动启动</Typography>
                      <Switch
                        size="small"
                        checked={verge?.enable_auto_launch ?? false}
                        onChange={(_, checked: boolean) => patchVerge({ enable_auto_launch: checked })}
                        sx={{ transform: 'scale(0.75)', transformOrigin: 'right center' }}
                      />
                    </ListItem>
                    <ListItem sx={{ py: 0.1, px: 0.5, display: 'flex', justifyContent: 'space-between' }}>
                      <Typography variant="caption" sx={{ fontSize: '11px' }}>启动时最小化</Typography>
                      <Switch
                        size="small"
                        checked={verge?.enable_silent_start ?? false}
                        onChange={(_, checked: boolean) => patchVerge({ enable_silent_start: checked })}
                        sx={{ transform: 'scale(0.75)', transformOrigin: 'right center' }}
                      />
                    </ListItem>
                    <ListItem sx={{ py: 0.1, px: 0.5, display: 'flex', justifyContent: 'space-between' }}>
                      <Typography variant="caption" sx={{ fontSize: '11px' }}>通知弹窗显示</Typography>
                      <Switch
                        size="small"
                        checked={notificationsEnabled}
                        onChange={(_, checked: boolean) => {
                          setNotificationsEnabled(checked)
                          localStorage.setItem('clash-verge-enable-notification', checked ? 'true' : 'false')
                        }}
                        sx={{ transform: 'scale(0.75)', transformOrigin: 'right center' }}
                      />
                    </ListItem>
                    <ListItem sx={{ py: 0.25, px: 0.5, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                      <Typography variant="caption" sx={{ fontSize: '11px' }}>Mixed Port</Typography>
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                        <TextField
                          size="small"
                          type="text"
                          value={mixedPortVal}
                          onChange={(e) => setMixedPortVal(e.target.value ? parseInt(e.target.value, 10) || 0 : 0)}
                          slotProps={{ htmlInput: { style: { paddingTop: '2px', paddingBottom: '2px', paddingLeft: '4px', paddingRight: '4px', width: '60px', fontSize: '11px', textAlign: 'center' } } }}
                        />
                        <IconButton size="small" onClick={handleSavePort} sx={{ p: 0.2 }}>
                          <SaveRounded sx={{ fontSize: 13 }} />
                        </IconButton>
                      </Box>
                    </ListItem>
                    <ListItem sx={{ py: 0.1, px: 0.5, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                      <Typography variant="caption" sx={{ fontSize: '11px', flexShrink: 0 }}>主题模式</Typography>
                      <Box
                        sx={{
                          position: 'relative',
                          display: 'flex',
                          alignItems: 'center',
                          bgcolor: 'action.hover',
                          borderRadius: '4px',
                          p: '1px',
                          width: '120px',
                          height: 22,
                          userSelect: 'none',
                        }}
                      >
                        {/* Sliding Background Indicator */}
                        <Box
                          sx={{
                            position: 'absolute',
                            left: 0,
                            top: 0,
                            width: '33.333%',
                            height: '100%',
                            zIndex: 0,
                            transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                            transform: `translate3d(${themeActiveIndex * 100}%, 0, 0)`,
                          }}
                        >
                          <Box
                            sx={{
                              height: 'calc(100% - 2px)',
                              margin: '1px',
                              bgcolor: 'primary.main',
                              borderRadius: '3px',
                              boxShadow: '0px 1px 2px rgba(0, 0, 0, 0.15)',
                            }}
                          />
                        </Box>

                        {/* System Option */}
                        <Box
                          onClick={() => patchVerge({ theme_mode: 'system' })}
                          sx={{
                            flex: 1,
                            height: '100%',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            color: themeActiveIndex === 0 ? 'primary.contrastText' : 'text.secondary',
                            fontSize: '10px',
                            fontWeight: themeActiveIndex === 0 ? 'bold' : 'normal',
                            cursor: 'pointer',
                            zIndex: 1,
                            transition: 'color 0.2s ease',
                          }}
                        >
                          系统
                        </Box>

                        {/* Light Option */}
                        <Box
                          onClick={() => patchVerge({ theme_mode: 'light' })}
                          sx={{
                            flex: 1,
                            height: '100%',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            color: themeActiveIndex === 1 ? 'primary.contrastText' : 'text.secondary',
                            fontSize: '10px',
                            fontWeight: themeActiveIndex === 1 ? 'bold' : 'normal',
                            cursor: 'pointer',
                            zIndex: 1,
                            transition: 'color 0.2s ease',
                          }}
                        >
                          浅色
                        </Box>

                        {/* Dark Option */}
                        <Box
                          onClick={() => patchVerge({ theme_mode: 'dark' })}
                          sx={{
                            flex: 1,
                            height: '100%',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            color: themeActiveIndex === 2 ? 'primary.contrastText' : 'text.secondary',
                            fontSize: '10px',
                            fontWeight: themeActiveIndex === 2 ? 'bold' : 'normal',
                            cursor: 'pointer',
                            zIndex: 1,
                            transition: 'color 0.2s ease',
                          }}
                        >
                          深色
                        </Box>
                      </Box>
                    </ListItem>
                  </List>
                  
                  {/* Centered Troubleshooting Button */}
                  <Box sx={{ display: 'flex', justifyContent: 'center', width: '100%', mt: 1 }}>
                    <Button
                      variant="contained"
                      color="primary"
                      size="small"
                      onClick={() => setLogsOpen(true)}
                      sx={{
                        fontSize: 11,
                        height: 26,
                        textTransform: 'none',
                        px: 2,
                      }}
                    >
                      系统调试运行日志
                    </Button>
                  </Box>
                </Box>
              </Box>

              {/* Right Connections column (自适应 flex: 1) */}
              <Box
                sx={{
                  flex: 1,
                  minWidth: 0,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 0.5,
                  height: '100%',
                  minHeight: 0,
                  overflow: 'hidden',
                }}
              >
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 0.5, width: '100%', justifyContent: 'flex-start' }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 'bold', fontSize: '11px', whiteSpace: 'nowrap' }}>
                    路径控制（右键点击链接）
                  </Typography>
                  <ButtonGroup size="small" color="primary" sx={{ transform: 'scale(0.85)', transformOrigin: 'left center' }}>
                    <Button
                      variant={connectionsType === 'active' ? 'contained' : 'outlined'}
                      onClick={() => setConnectionsType('active')}
                      sx={{
                        fontSize: 10,
                        height: 22,
                      }}
                    >
                      活跃 ({connectionsData?.activeConnections.length || 0})
                    </Button>
                    <Button
                      variant={connectionsType === 'closed' ? 'contained' : 'outlined'}
                      onClick={() => setConnectionsType('closed')}
                      sx={{
                        fontSize: 10,
                        height: 22,
                      }}
                    >
                      历史 ({connectionsData?.closedConnections.length || 0})
                    </Button>
                  </ButtonGroup>
                </Box>

                {/* Search and Action Row */}
                <Box sx={{ display: 'flex', gap: 1, alignItems: 'center', px: 0.5, mt: 0.5 }}>
                  <Box sx={{ flex: 1 }}>
                    <BaseSearchBox onSearch={handleSearch} />
                  </Box>
                  <Button
                    size="small"
                    variant="outlined"
                    onClick={() => closeAllConnections()}
                    sx={{ fontSize: 10, height: 24, px: 1, minWidth: 'auto' }}
                  >
                    断开全部
                  </Button>
                  {connectionsType === 'closed' && (
                    <Button
                      size="small"
                      variant="outlined"
                      onClick={() => clearClosedConnections()}
                      sx={{ fontSize: 10, height: 24, px: 1, minWidth: 'auto' }}
                    >
                      清空历史
                    </Button>
                  )}
                </Box>

                {/* Connection Table Container */}
                <Box sx={{ flex: 1, minHeight: 0, mt: 0.5 }}>
                  {filterConn.length === 0 ? (
                    <BaseEmpty />
                  ) : (
                    <ConnectionTable
                      connections={filterConn}
                      onShowDetail={(detail, el) =>
                        detailRef.current?.open(detail, connectionsType === 'closed', el)
                      }
                      columnManagerOpen={isColumnManagerOpen}
                      onCloseColumnManager={() => setIsColumnManagerOpen(false)}
                    />
                  )}
                </Box>
              </Box>
            </div>
          </div>

          {/* Lower Pane: Constant Traffic Dashboard (Fixed Height) */}
          <div
            style={{
              flex: '0 0 178px',
              height: '178px',
              borderTop: '1px solid',
              borderTopColor: 'var(--divider-color, rgba(0,0,0,0.12))',
              background: 'inherit',
              padding: '8px 12px',
              display: 'flex',
              gap: '12px',
              overflow: 'hidden',
              boxSizing: 'border-box',
            }}
          >
            <MiniTrafficPanel />
          </div>
        </div>
      </Paper>

      {/* Popups & dialogs */}
      <ConnectionDetail ref={detailRef} />

      {/* Logs View Dialog */}
      <Dialog
        open={logsOpen}
        onClose={() => setLogsOpen(false)}
        maxWidth="md"
        fullWidth
        slotProps={{
          paper: {
            sx: {
              background: mode === 'light' ? 'var(--background-color)' : '#1e1f27',
              height: '480px',
              position: 'relative',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
              boxSizing: 'border-box',
              '& .base-page > header': {
                pr: 6,
              },
            }
          }
        }}
      >
        <IconButton
          size="small"
          onClick={() => setLogsOpen(false)}
          sx={{
            position: 'absolute',
            top: 14,
            right: 14,
            zIndex: 10,
            color: 'text.secondary',
          }}
        >
          <CloseRounded fontSize="small" />
        </IconButton>
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'hidden' }}>
          <LogsPage />
        </Box>
      </Dialog>
    </ThemeProvider>
  )
}

export default Layout
