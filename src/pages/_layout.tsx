import {
  ArrowDownwardRounded,
  ArrowUpwardRounded,
  SettingsRounded as SettingsRoundedIcon,
  CloseRounded,
  RefreshRounded,
  DeleteRounded,
  WifiOff as SignalError,
  SignalWifi3Bar as SignalGood,
  SignalWifi2Bar as SignalMedium,
  SignalWifi0Bar as SignalNone,
  SignalWifi4Bar as SignalStrong,
  SignalWifi1Bar as SignalWeak,
  PushPinRounded,
} from '@mui/icons-material'
import {
  Box,
  List,
  Paper,
  ThemeProvider,
  Typography,
  IconButton,
  Button,
  Chip,
  CircularProgress,
  TextField,
  Dialog,
  ListItem,
  Slider,
  useTheme,
} from '@mui/material'
import { alpha } from '@mui/material'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useLocation } from 'react-router'
import {
  healthcheckProxyProvider,
  closeAllConnections,
} from 'tauri-plugin-mihomo-api'

import { BaseSearchBox, BaseEmpty, Switch } from '@/components/base'
import { ConnectionDetail } from '@/components/connection/connection-detail'
import { ConnectionTable } from '@/components/connection/connection-table'
import { EnhancedCanvasTrafficGraph } from '@/components/home/enhanced-canvas-traffic-graph'
import { TrafficGraph } from '@/components/layout/traffic-graph'
import { NoticeManager } from '@/components/layout/notice-manager'
import { WindowControls } from '@/components/layout/window-controller'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { filterSort } from '@/components/proxy/use-filter-sort'
import { useClashInfo } from '@/hooks/use-clash'
import { useConnectionData } from '@/hooks/use-connection-data'
import { useI18n } from '@/hooks/use-i18n'
import { useProfiles } from '@/hooks/use-profiles'
import { useProxySelection } from '@/hooks/use-proxy-selection'
import { useServiceInstaller } from '@/hooks/use-service-installer'
import { useSystemState } from '@/hooks/use-system-state'
import { useTrafficData } from '@/hooks/use-traffic-data'
import { useVerge } from '@/hooks/use-verge'
import { useVisibility } from '@/hooks/use-visibility'
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
  getProxyAddr,
  cmdGetProxyDelay,
} from '@/services/cmds'
import delayManager from '@/services/delay'
import { showNotice } from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import {
  get3DButtonStyle,
  get3DInputStyle,
  get3DSegmentedContainerStyle,
  get3DSegmentedActiveStyle,
  get3DCardStyle,
} from '@/utils/button-styles'
import getSystem from '@/utils/get-system'
import parseTraffic from '@/utils/parse-traffic'

import {
  useCustomTheme,
  useLayoutEvents,
  useLoadingOverlay,
} from './_layout/hooks'
import { handleNoticeMessage } from './_layout/utils'
import LogsPage from './logs'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)

const OS = getSystem()

export const portableFlag = false

// Delay Helpers
function getSignalIcon(delay: number) {
  if (delay === -2)
    return { icon: <SignalNone />, text: '测试中', color: 'text.secondary' }
  if (delay === -1)
    return { icon: <SignalNone />, text: '未测试', color: 'text.secondary' }
  if (delay > 1e5)
    return { icon: <SignalError />, text: '错误', color: 'error.main' }
  if (delay === 0 || delay >= 10000)
    return { icon: <SignalError />, text: '超时', color: 'error.main' }
  if (delay >= 500)
    return { icon: <SignalWeak />, text: '延迟较高', color: 'error.main' }
  if (delay >= 300)
    return { icon: <SignalMedium />, text: '延迟中等', color: 'warning.main' }
  if (delay >= 200)
    return { icon: <SignalGood />, text: '延迟良好', color: 'info.main' }
  return { icon: <SignalStrong />, text: '延迟极佳', color: 'success.main' }
}

function convertDelayColor(
  delayValue: number,
): 'success' | 'warning' | 'error' | 'primary' | 'default' {
  const colorStr = delayManager.formatDelayColor(delayValue)
  if (!colorStr) return 'default'
  const mainColor = colorStr.split('.')[0]
  switch (mainColor) {
    case 'success':
      return 'success'
    case 'warning':
      return 'warning'
    case 'error':
      return 'error'
    case 'primary':
      return 'primary'
    default:
      return 'default'
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
const getFriendlyProtocolName = (type?: string) => {
  if (!type) return ''
  const map: Record<string, string> = {
    ss: 'Shadowsocks',
    ssr: 'ShadowsocksR',
    vmess: 'VMess',
    vless: 'VLESS',
    trojan: 'Trojan',
    hysteria: 'Hysteria',
    hysteria2: 'Hysteria 2',
    tuic: 'TUIC',
    wireguard: 'WireGuard',
    shadowsocks: 'Shadowsocks',
    shadowsocksr: 'ShadowsocksR',
  }
  const low = type.toLowerCase()
  return map[low] || type.toUpperCase()
}

const ActiveNodeStatusCard = () => {
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()

  const primaryGroup = useMemo(() => {
    const groups = proxies?.groups || []
    const primaryKeywords = ['auto', 'select', 'proxy', '节点选择', '自动选择']
    return (
      groups.find((group: any) =>
        primaryKeywords.some((keyword) =>
          group.name.toLowerCase().includes(keyword.toLowerCase()),
        ),
      ) ||
      groups.filter((g: any) => g.name !== 'GLOBAL')[0] ||
      groups[0]
    )
  }, [proxies])

  const activeNodeName = primaryGroup?.now || ''

  const activeNodeRecord = useMemo(() => {
    if (!activeNodeName) return null
    if (proxies?.records?.[activeNodeName]) {
      return proxies.records[activeNodeName]
    }
    const groups = proxies?.groups || []
    for (const group of groups) {
      if (group?.all) {
        const found = group.all.find(
          (node: any) => node?.name === activeNodeName,
        )
        if (found) return found
      }
    }
    if (proxies?.global?.all) {
      const found = proxies.global.all.find(
        (node: any) => node?.name === activeNodeName,
      )
      if (found) return found
    }
    return null
  }, [proxies, activeNodeName])

  const delay = useMemo(() => {
    if (!activeNodeName || !primaryGroup?.name || !activeNodeRecord) return -1
    return delayManager.getDelayFix(activeNodeRecord, primaryGroup.name)
  }, [activeNodeName, primaryGroup, activeNodeRecord])

  const [testing, setTesting] = useState(false)
  const [nodeAddr, setNodeAddr] = useState<string>('')

  useEffect(() => {
    if (!activeNodeName) {
      Promise.resolve().then(() => setNodeAddr(''))
      return
    }
    getProxyAddr(activeNodeName, activeNodeRecord?.provider)
      .then((res) => {
        if (res) {
          setNodeAddr(`${res[0]}:${res[1]}`)
        } else {
          setNodeAddr('')
        }
      })
      .catch((err) => {
        console.error('Failed to get proxy address:', err)
        setNodeAddr('')
      })
  }, [activeNodeName, activeNodeRecord?.provider])
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
  const theme = useTheme()

  return (
    <Paper
      sx={{
        m: 0,
        p: '0 12px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 1.5,
        height: '28px',
        ...get3DCardStyle(theme, 'default'),
      }}
    >
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{
          fontSize: '11px',
          '@media (max-width: 580px)': {
            display: 'none',
          },
        }}
      >
        当前活跃出口节点：
      </Typography>

      <Typography
        variant="caption"
        sx={{
          fontWeight: 'bold',
          fontSize: '11px',
          color: 'primary.main',
          border: '1px solid',
          borderColor: 'primary.main',
          borderRadius: '4px',
          px: 0.8,
          py: 0.2,
          '@media (max-width: 480px)': {
            display: 'none',
          },
        }}
      >
        {getFriendlyProtocolName(activeNodeRecord?.type) || 'Direct'}
      </Typography>

      <Typography
        variant="body2"
        sx={{
          fontWeight: 'bold',
          fontSize: '12px',
          color: 'text.primary',
          maxWidth: { xs: '120px', sm: '240px', md: '360px' },
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
      >
        {activeNodeName || '未选择节点 (直接连接)'}
      </Typography>

      {activeNodeName && (
        <Chip
          size="small"
          icon={
            testing ? (
              <CircularProgress size={10} color="inherit" />
            ) : (
              signalInfo.icon
            )
          }
          label={testing ? '测试中...' : delayManager.formatDelay(delay)}
          color={delayColor}
          onClick={handleTestDelay}
          sx={{
            fontSize: '11px',
            height: '20px',
            fontWeight: 600,
            cursor: 'pointer',
            bgcolor: testing
              ? undefined
              : alpha(
                  signalInfo.color === 'success.main'
                    ? '#4caf50'
                    : signalInfo.color === 'warning.main'
                      ? '#ff9800'
                      : '#f44336',
                  0.12,
                ),
            color:
              signalInfo.color === 'text.secondary'
                ? 'text.secondary'
                : signalInfo.color,
            '& .MuiChip-icon': {
              color: 'inherit',
              fontSize: '12px',
            },
          }}
        />
      )}

      {nodeAddr && (
        <Typography
          variant="caption"
          sx={{
            fontWeight: 'bold',
            fontSize: '11px',
            color: 'text.secondary',
            '@media (max-width: 400px)': {
              display: 'none',
            },
          }}
        >
          ({nodeAddr})
        </Typography>
      )}
    </Paper>
  )
}

// Mini Traffic Panel
const MiniTrafficPanel = ({ isMinimalWidth }: { isMinimalWidth: boolean }) => {
  const mode = useThemeMode()
  const theme = useTheme()
  useTranslation()
  const pageVisible = useVisibility()
  const {
    response: { data: traffic },
  } = useTrafficData({ enabled: pageVisible })
  const {
    response: { data: connections },
  } = useConnectionData()
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
  const [downTotalVal, downTotalUnit] = parseTraffic(
    connections?.downloadTotal || 0,
  )

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        width: '100%',
        height: '100%',
        boxSizing: 'border-box',
        overflow: 'hidden',
      }}
    >
      {/* Traffic Graph (Full Width) */}
      <Box sx={{ flex: 1, width: '100%', minHeight: 0, position: 'relative' }}>
        {isMinimalWidth ? (
          <TrafficGraph ref={trafficRef} />
        ) : (
          <EnhancedCanvasTrafficGraph ref={trafficRef} />
        )}
      </Box>

      {/* Metrics Row (Single Line Below Graph - Raised 3D Button style) */}
      <Box
        sx={{
          display: 'flex',
          width: '100%',
          height: '22px',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderTop: (theme) => `1px solid ${theme.palette.divider}`,
          mt: 0.5,
          pt: 0.5,
          px: 1,
          gap: 1,
          boxSizing: 'border-box',
          '@media (max-width: 560px)': {
            height: 'auto',
            flexDirection: 'column',
            gap: 0,
            pt: 0.5,
          },
        }}
      >
        {/* Download Group */}
        <Box sx={{ display: 'flex', flex: 1, minWidth: 0, width: '100%' }}>
          {/* Download Speed */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'download'),
              borderTopRightRadius: 0,
              borderBottomRightRadius: 0,
              borderRight: 'none',
              whiteSpace: 'nowrap',
            }}
          >
            <ArrowDownwardRounded sx={{ color: mode === 'light' ? '#0084FF' : '#80D8FF', fontSize: 14 }} />
            <Typography
              sx={{
                fontSize: '9px',
                color: mode === 'light' ? '#006064' : '#80D8FF',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              下载:
            </Typography>
            <Typography
              sx={{
                fontWeight: 'bold',
                fontSize: '13px',
                color: mode === 'light' ? '#00363A' : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {downVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  fontWeight: 'normal',
                  color: mode === 'light' ? '#006064' : '#E0F7FA',
                }}
              >
                {downUnit}/s
              </span>
            </Typography>
          </Box>

          {/* Download Total */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'download'),
              borderTopLeftRadius: 0,
              borderBottomLeftRadius: 0,
              whiteSpace: 'nowrap',
            }}
          >
            <Typography
              sx={{
                fontSize: '9px',
                color: mode === 'light' ? '#006064' : '#80D8FF',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              总量:
            </Typography>
            <Typography
              sx={{
                fontWeight: 'bold',
                fontSize: '13px',
                color: mode === 'light' ? '#00363A' : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {downTotalVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  color: mode === 'light' ? '#006064' : '#E0F7FA',
                  fontWeight: 'normal',
                }}
              >
                {downTotalUnit}
              </span>
            </Typography>
          </Box>
        </Box>

        {/* Upload Group */}
        <Box sx={{ display: 'flex', flex: 1, minWidth: 0, width: '100%' }}>
          {/* Upload Speed */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'upload'),
              borderTopRightRadius: 0,
              borderBottomRightRadius: 0,
              borderRight: 'none',
              whiteSpace: 'nowrap',
            }}
          >
            <ArrowUpwardRounded sx={{ color: mode === 'light' ? '#E65100' : '#FFD54F', fontSize: 14 }} />
            <Typography
              sx={{
                fontSize: '9px',
                color: mode === 'light' ? '#7B5200' : '#FFD54F',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              上传:
            </Typography>
            <Typography
              sx={{
                fontWeight: 'bold',
                fontSize: '13px',
                color: mode === 'light' ? '#3E2723' : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {upVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  fontWeight: 'normal',
                  color: mode === 'light' ? '#7B5200' : '#FFECB3',
                }}
              >
                {upUnit}/s
              </span>
            </Typography>
          </Box>

          {/* Upload Total */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'upload'),
              borderTopLeftRadius: 0,
              borderBottomLeftRadius: 0,
              whiteSpace: 'nowrap',
            }}
          >
            <Typography
              sx={{
                fontSize: '9px',
                color: mode === 'light' ? '#7B5200' : '#FFD54F',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              总量:
            </Typography>
            <Typography
              sx={{
                fontSize: '13px',
                fontWeight: 'bold',
                color: mode === 'light' ? '#3E2723' : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {upTotalVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  color: mode === 'light' ? '#7B5200' : '#FFECB3',
                  fontWeight: 'normal',
                }}
              >
                {upTotalUnit}
              </span>
            </Typography>
          </Box>
        </Box>
      </Box>
    </Box>
  )
}

const get3DSliderStyle = (theme: any, mode: 'light' | 'dark') => {
  const isLight = mode === 'light'
  return {
    py: 0.5,
    '& .MuiSlider-rail': {
      height: 10,
      opacity: 0.85,
      bgcolor: isLight ? 'rgba(212, 175, 55, 0.15)' : 'rgba(212, 175, 55, 0.08)',
      boxShadow: isLight
        ? 'inset 0 3px 5px rgba(0,0,0,0.3), 0 1px 0 rgba(255,255,255,0.8)'
        : 'inset 0 3px 5px rgba(0,0,0,0.75), 0 1px 0 rgba(255,255,255,0.08)',
      border: `1px solid ${isLight ? 'rgba(212, 175, 55, 0.25)' : 'rgba(212, 175, 55, 0.12)'}`,
      borderRadius: 5,
    },
    '& .MuiSlider-track': {
      height: 10,
      border: 'none',
      borderRadius: 5,
      background: isLight
        ? `linear-gradient(to bottom, #FFA000 0%, #E65100 100%)`
        : `linear-gradient(to bottom, rgba(255, 160, 0, 0.8) 0%, rgba(230, 81, 0, 0.9) 100%)`,
      boxShadow: isLight
        ? 'inset 0 1px 0 rgba(255,255,255,0.4)'
        : 'inset 0 1px 0 rgba(255,255,255,0.08)',
    },
    '& .MuiSlider-thumb': {
      width: 14,
      height: 14,
      top: '50%',
      transform: 'translate(-50%, -50%)',
      border: `1px solid ${isLight ? '#9E670B' : '#6E4302'}`,
      background: isLight
        ? `radial-gradient(circle at 35% 35%, #ffffff 0%, #FFD54F 55%, #FFA000 100%)`
        : `radial-gradient(circle at 35% 35%, #ffffff 0%, #FFA000 55%, #E65100 100%)`,
      boxShadow: isLight
        ? `0 calc(3px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0,0,0,0.25),
           inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.8),
           0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.25 * var(--vibrancy-factor, 1.0)))`
        : `0 calc(4px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) rgba(0,0,0,0.6),
           inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.4),
           0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.3 * var(--vibrancy-factor, 1.0)))`,
      transition:
        'transform 0.1s ease-out, box-shadow 0.1s ease-out, filter 0.1s ease-out',
      '&:hover, &.Mui-focusVisible': {
        transform: 'translate(-50%, -50%) scale(1.2)',
        filter: 'brightness(1.15)',
        boxShadow: isLight
          ? `0 calc(5px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.9),
             0 0 calc(12px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.35 * var(--vibrancy-factor, 1.0)))`
          : `0 calc(6px * var(--depth-factor, 1.0)) calc(12px * var(--depth-factor, 1.0)) rgba(0,0,0,0.7),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.5),
             0 0 calc(14px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.4 * var(--vibrancy-factor, 1.0)))`,
      },
      '&.Mui-active': {
        transform: 'translate(-50%, -50%) scale(0.92)',
        boxShadow: isLight
          ? `inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0,0,0,0.15)`
          : `inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.5)`,
      },
    },
  }
}

const Layout = () => {
  // Dual Sliders State (Depth & Vibrancy Factors)
  const [depthFactor, setDepthFactor] = useState<number>(() => {
    const saved = localStorage.getItem('clash-mini-depth-factor')
    return saved !== null ? parseFloat(saved) : 1.0
  })

  const [vibrancyFactor, setVibrancyFactor] = useState<number>(() => {
    const saved = localStorage.getItem('clash-mini-vibrancy-factor')
    return saved !== null ? parseFloat(saved) : 1.0
  })

  const [isMinimalWidth, setIsMinimalWidth] = useState(() => {
    if (typeof window !== 'undefined') {
      return window.innerWidth <= 270
    }
    return false
  })

  useEffect(() => {
    if (typeof window === 'undefined') return
    const handleResize = () => {
      setIsMinimalWidth(window.innerWidth <= 270)
    }
    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [])

  const handleDepthFactorChange = (val: number) => {
    setDepthFactor(val)
    localStorage.setItem('clash-mini-depth-factor', val.toString())
  }

  const handleVibrancyFactorChange = (val: number) => {
    setVibrancyFactor(val)
    localStorage.setItem('clash-mini-vibrancy-factor', val.toString())
  }

  useEffect(() => {
    document.documentElement.style.setProperty(
      '--depth-factor',
      depthFactor.toString(),
    )
  }, [depthFactor])

  useEffect(() => {
    document.documentElement.style.setProperty(
      '--vibrancy-factor',
      vibrancyFactor.toString(),
    )
  }, [vibrancyFactor])

  const mode = useThemeMode()
  const { t } = useTranslation()
  const { theme } = useCustomTheme()
  const { verge, patchVerge } = useVerge()
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
  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()

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

  // Minimal Settings States
  const [notificationsEnabled, setNotificationsEnabled] = useState(
    () => localStorage.getItem('clash-verge-enable-notification') !== 'false',
  )

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
  } = useConnectionData()
  const [isColumnManagerOpen, setIsColumnManagerOpen] = useState(false)
  const detailRef = useRef<any>(null)

  const filterConn = useMemo(() => {
    const orderFunc = orderFunctionMap[curOrderOpt]
    const conns =
      (connectionsType === 'active'
        ? connectionsData?.activeConnections
        : connectionsData?.closedConnections) ?? []
    let matchConns = conns.filter((conn) => {
      const { host, destinationIP, process } = conn.metadata
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
    if (language) {
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguage(language)
    }
  }, [language, switchLanguage])

  const triggerAutoSelectFastestNode = useCallback(
    async (profileUid: string) => {
      if (!profileUid) return
      console.log(
        `[BUG-034] Profile UID changed to ${profileUid}, scheduling auto select fastest...`,
      )

      pollSessionRef.current += 1
      const currentSession = pollSessionRef.current

      // 1. Wait a bit for Clash core to reload and populate proxies (with retry loop)
      const groupName = 'PROXY'
      let group: any = null
      let proxiesData: any = null
      const findStartTime = Date.now()

      while (Date.now() - findStartTime < 5000) {
        if (pollSessionRef.current !== currentSession) return
        const freshProxies = await refreshProxy()
        proxiesData = freshProxies?.data || proxies
        group = proxiesData?.groups?.find((g: any) => g.name === groupName)
        if (group && group.all && group.all.length > 0) {
          break
        }
        await new Promise((resolve) => setTimeout(resolve, 500))
      }

      if (!group || !group.all || group.all.length === 0) {
        console.warn('[BUG-034] PROXY group not found or empty after 5 seconds')
        return
      }

      const nodeNames: string[] = group.all
        .map((n: any) => n?.name)
        .filter(Boolean)
      console.log(
        `[BUG-034] Found PROXY group with ${group.all.length} nodes (${nodeNames.length} mapped), starting auto-latency test...`,
      )

      // 2. Set sorting state for this group in local storage to "Latency Sort" (sortType: 1)
      try {
        const item = localStorage.getItem('proxy-head-state')
        let data = (item ? JSON.parse(item) : {}) as Record<string, any>
        if (!data || typeof data !== 'object') data = {}
        if (!data[profileUid]) data[profileUid] = {}
        if (!data[profileUid][groupName]) data[profileUid][groupName] = {}
        data[profileUid][groupName].sortType = 1 // 1 = latency sort
        localStorage.setItem('proxy-head-state', JSON.stringify(data))
        console.log(
          `[BUG-034] Set proxy-head-state sortType to 1 for profile ${profileUid}`,
        )
      } catch (e) {
        console.error('[BUG-034] Failed to set auto-sort in localStorage:', e)
      }

      // 3. Trigger latency tests
      try {
        // Check unique providers across nodes in the PROXY group
        const uniqueProviders = new Set<string>()
        for (const node of group.all) {
          if (node?.provider) {
            uniqueProviders.add(node.provider)
          }
        }

        if (uniqueProviders.size > 0) {
          console.log(
            `[BUG-034] Triggering healthcheck for providers: ${Array.from(uniqueProviders).join(', ')}`,
          )
          await Promise.all(
            Array.from(uniqueProviders).map((provider) =>
              healthcheckProxyProvider(provider).catch((err) => {
                console.error(
                  `[BUG-034] provider healthcheck failed for ${provider}:`,
                  err,
                )
              }),
            ),
          )
        } else {
          console.log(
            `[BUG-034] Triggering delay test for all nodes: ${nodeNames.length}`,
          )
          const timeout = verge?.default_latency_timeout || 10000
          await delayManager
            .checkListDelay(nodeNames, groupName, timeout)
            .catch((err) => {
              console.error('[BUG-034] checkListDelay failed:', err)
            })
        }

        const isDummyNode = (name: string): boolean => {
          const lower = name.toLowerCase()
          return (
            lower.includes('流量') ||
            lower.includes('过期时间') ||
            lower.includes('网址') ||
            lower.includes('官网') ||
            lower.includes('剩余') ||
            lower.includes('expire') ||
            lower.includes('traffic') ||
            lower.includes('website') ||
            lower.includes('http') ||
            lower.includes('https')
          )
        }

        const startTime = Date.now()
        let hasSelected = false

        while (!hasSelected) {
          if (pollSessionRef.current !== currentSession) {
            console.log('[BUG-034] Session invalidated, stopping poll.')
            return
          }

          const elapsed = (Date.now() - startTime) / 1000

          // Fetch fresh proxy records
          const testedProxies = await refreshProxy()
          if (pollSessionRef.current !== currentSession) return

          const latestData = testedProxies?.data || proxiesData
          const currentGroup =
            latestData?.groups?.find((g: any) => g.name === groupName) || group

          // Read filter and sort from localStorage for the active profile & PROXY group
          let filterText = ''
          let useRegex = false
          let matchCase = false
          let matchWholeWord = false
          try {
            const item = localStorage.getItem('proxy-head-state')
            if (item) {
              const data = JSON.parse(item)
              const currentProfile = profiles?.current || latestData?.current || ''
              const groupState = data[currentProfile]?.[groupName]
              if (groupState) {
                filterText = groupState.filterText || ''
                useRegex = !!groupState.filterUseRegularExpression
                matchCase = !!groupState.filterMatchCase
                matchWholeWord = !!groupState.filterMatchWholeWord
              }
            }
          } catch (e) {
            console.error('[BUG-034] Error parsing proxy-head-state:', e)
          }

          // Filter nodes to match the active homepage filter
          const filteredAll = filterSort(
            currentGroup.all || [],
            groupName,
            filterText,
            0,
            verge?.default_latency_timeout,
            {
              matchCase,
              matchWholeWord,
              useRegularExpression: useRegex,
            }
          )

          // Collect healthy scanned nodes
          const healthyNodes: { name: string; delay: number }[] = []
          for (const node of filteredAll) {
            const name = node?.name
            if (!name || isDummyNode(name)) continue
            const d = delayManager.getDelayFix(node, groupName)
            if (d > 0) {
              healthyNodes.push({ name, delay: d })
            }
          }

          // Sort by delay ascending
          healthyNodes.sort((a, b) => a.delay - b.delay)

          console.log(
            `[BUG-034] Polling: elapsed=${elapsed.toFixed(1)}s, healthyNodes count=${healthyNodes.length}`,
          )

          // Rule A: If elapsed < 30s and healthyNodes count >= 5, pick the fastest and connect
          if (elapsed < 30 && healthyNodes.length >= 5) {
            const targetNode = healthyNodes[0].name
            const targetDelay = healthyNodes[0].delay
            changeProxy(groupName, targetNode, currentGroup.now)
            showNotice.success(
              `自动测速完成，已切换至最快节点: ${targetNode} (${targetDelay}ms)`,
            )
            hasSelected = true
            break
          }

          // Rule B & C: If elapsed >= 30s and < 60s
          if (elapsed >= 30 && elapsed < 60) {
            if (healthyNodes.length >= 1) {
              const targetNode = healthyNodes[0].name
              const targetDelay = healthyNodes[0].delay
              changeProxy(groupName, targetNode, currentGroup.now)
              showNotice.success(
                `自动测速超时降级，已切换至可用最快节点: ${targetNode} (${targetDelay}ms)`,
              )
              hasSelected = true
              break
            }
          }

          // Rule D: If 60 seconds have elapsed and still no healthy nodes
          if (elapsed >= 60) {
            showNotice.error(
              <span style={{ fontSize: '18px', fontWeight: 'bold' }}>
                所有线路都繁忙，请耐心等待。
              </span>,
            )
            hasSelected = true
            break
          }

          // Wait 500ms before next poll
          await new Promise((resolve) => setTimeout(resolve, 500))
        }
      } catch (err) {
        console.error('[BUG-034] Error during auto speed test and select:', err)
      }
    },
    [refreshProxy, proxies, verge?.default_latency_timeout, changeProxy],
  )

  const lastEnhancedProfileRef = useRef<string | null>(null)
  const pollSessionRef = useRef<number>(0)

  useEffect(() => {
    return () => {
      pollSessionRef.current += 1
    }
  }, [])

  // Background monitor for the active proxy node
  const consecutiveFailRef = useRef<number>(0)
  const lastActiveNodeRef = useRef<string | null>(null)

  useEffect(() => {
    if (!currentProfileUid) return

    consecutiveFailRef.current = 0
    lastActiveNodeRef.current = null
    let timerId: ReturnType<typeof setTimeout> | null = null

    const checkNode = async () => {
      const activeNodeName = proxies?.groups?.find((g: any) => g.name === 'PROXY')?.now
      if (!activeNodeName || activeNodeName === 'DIRECT' || activeNodeName === 'REJECT') {
        timerId = setTimeout(checkNode, 60000)
        return
      }

      // Reset counters if node manually or automatically switched
      if (lastActiveNodeRef.current !== activeNodeName) {
        lastActiveNodeRef.current = activeNodeName
        consecutiveFailRef.current = 0
      }

      let isHealthy = false
      try {
        const timeout = 5000
        const testUrl = delayManager.getUrl('PROXY')
        const result = await cmdGetProxyDelay(activeNodeName, timeout, testUrl)
        const delay = result?.delay ?? 1e6

        if (delay < 1500) {
          isHealthy = true
        } else {
          console.log(`[NodeMonitor] Active node ${activeNodeName} is unhealthy (delay: ${delay}ms)`)
        }
      } catch (err) {
        console.error('[NodeMonitor] Failed to check active node latency:', err)
      }

      if (isHealthy) {
        consecutiveFailRef.current = 0
        timerId = setTimeout(checkNode, 60000)
      } else {
        consecutiveFailRef.current += 1
        console.log(
          `[NodeMonitor] Consecutive unhealthy count for ${activeNodeName} = ${consecutiveFailRef.current}`,
        )

        if (consecutiveFailRef.current >= 3) {
          consecutiveFailRef.current = 0
          console.log(`[NodeMonitor] Node ${activeNodeName} failed 3 times consecutively. Triggering auto select.`)
          showNotice.info(`检测到当前线路连接超时或缓慢，正在自动为您切换至最快线路...`)
          triggerAutoSelectFastestNode(currentProfileUid)
          timerId = setTimeout(checkNode, 60000)
        } else {
          // Failure occurred: fast retry in 5 seconds
          timerId = setTimeout(checkNode, 5000)
        }
      }
    }

    timerId = setTimeout(checkNode, 60000)

    return () => {
      if (timerId) {
        clearTimeout(timerId)
      }
    }
  }, [currentProfileUid, proxies, triggerAutoSelectFastestNode])

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
          // Trigger the auto speed-test and select fastest node chain
          triggerAutoSelectFastestNode(currentProfileUid)
        })
        .catch((err) => {
          console.error(
            `[Layout] Failed to enhance profile ${currentProfileUid}:`,
            err,
          )
          lastEnhancedProfileRef.current = null
        })
    }
  }, [currentProfileUid, triggerAutoSelectFastestNode])

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
    } catch {
      try {
        await importProfile(url, { with_proxy: false, self_proxy: true })
        showNotice.success('shared.feedback.notifications.importWithClashProxy')
        setUrl('')
        await mutateProfiles()
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

          {!drawerOpen && (
            <IconButton
              size="small"
              onClick={() =>
                patchVerge({
                  enable_always_on_top: !verge?.enable_always_on_top,
                })
              }
              sx={{
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
    [decorated, drawerOpen, patchVerge, verge?.enable_always_on_top],
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
          {/* Upper Pane: Node Selection (80%) */}
          <div
            style={{
              flex: isMinimalWidth ? '0 0 33px' : '80 0 0%',
              height: isMinimalWidth ? '33px' : '80%',
              position: 'relative',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
            }}
          >
            {/* 右上角独立控制按钮（齿轮/关闭） */}
            {decorated && !isMinimalWidth && (
              <div
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
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  width: '100%',
                  boxSizing: 'border-box',
                  padding: isMinimalWidth ? '3px 8px 2px 8px' : '3px 44px 2px 8px', // 右侧留出 44px 避让右上角绝对定位按钮
                  position: 'relative',
                  zIndex: 110,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <ActiveNodeStatusCard />
                </div>
                {decorated && !isMinimalWidth && (
                  <IconButton
                    size="small"
                    onClick={() =>
                      patchVerge({
                        enable_always_on_top: !verge?.enable_always_on_top,
                      })
                    }
                    sx={{
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
            {!isMinimalWidth && (
              <div style={{ flex: 1, overflow: 'hidden' }}>
                <ProxyGroups
                  mode={clashConfig?.mode?.toLowerCase() || 'rule'}
                  isChainMode={false}
                  chainConfigData={null}
                />
              </div>
            )}

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
                }}
              >
                {/* Section 1: Subscriptions Import */}
                <Box
                  sx={{
                    p: 1,
                    flexShrink: 0,
                    ...get3DCardStyle(theme, 'default'),
                    '&:hover': {
                      transform: 'none',
                      boxShadow: get3DCardStyle(theme, 'default').boxShadow,
                    },
                  }}
                >
                  <Typography
                    variant="subtitle2"
                    sx={{
                      fontWeight: 'bold',
                      mb: 0.75,
                      fontSize: '13px',
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                    }}
                  >
                    订阅与机场配置
                    {profileLoading && <CircularProgress size={10} />}
                  </Typography>
                  <Box
                    sx={{
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 0.5,
                      mb: 1,
                    }}
                  >
                    <TextField
                      placeholder="填入订阅链接 (YAML)"
                      size="small"
                      value={url}
                      onChange={(e) => setUrl(e.target.value)}
                      slotProps={{
                        htmlInput: {
                          style: {
                            paddingTop: '4px',
                            paddingBottom: '4px',
                            fontSize: '13px',
                            height: '30px',
                            boxSizing: 'border-box',
                          },
                        },
                      }}
                      sx={{ width: '100%', ...get3DInputStyle(theme) }}
                    />
                    <Box
                      sx={{
                        display: 'flex',
                        justifyContent: 'center',
                        width: '100%',
                      }}
                    >
                      <Button
                        variant="contained"
                        color="primary"
                        onClick={handleImportProfile}
                        sx={{
                          fontSize: 12,
                          height: 24,
                          px: 2,
                          ...get3DButtonStyle(theme, 'contained', 'primary'),
                        }}
                        disabled={profileLoading}
                      >
                        导入订阅链接（YAML）
                      </Button>
                    </Box>
                  </Box>
                  {/* Profiles List */}
                  <Box
                    sx={{
                      maxHeight: 110,
                      overflowY: 'auto',
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 0.5,
                    }}
                  >
                    {profileItems.map((item) => {
                      const isActive = item.uid === currentProfileUid
                      const extra = item.extra
                      const hasExtra = !!extra
                      const {
                        upload = 0,
                        download = 0,
                        total = 0,
                      } = extra ?? {}
                      const progress =
                        total > 0
                          ? Math.min(
                              Math.round(
                                ((download + upload) * 100) / (total + 0.01),
                              ),
                              100,
                            )
                          : 0

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
                            bgcolor: (theme) =>
                              theme.palette.mode === 'light'
                                ? '#ffffff'
                                : '#282A36',
                            borderLeft: (theme) =>
                              `3px solid ${isActive ? theme.palette.primary.main : 'transparent'}`,
                            boxShadow: '0 1px 3px rgba(0,0,0,0.05)',
                            transition: 'all 0.2s',
                            '&:hover': {
                              bgcolor: 'action.hover',
                            },
                          }}
                        >
                          {/* Line 1: Title & Actions */}
                          <Box
                            sx={{
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'space-between',
                              width: '100%',
                            }}
                          >
                            <Typography
                              variant="body2"
                              sx={{
                                fontSize: '13px',
                                fontWeight: isActive ? 600 : 400,
                                color: isActive
                                  ? 'primary.main'
                                  : 'text.primary',
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap',
                                maxWidth: '70%',
                              }}
                              title={item.name}
                            >
                              {item.name || '未命名配置'}
                            </Typography>
                            <Box
                              sx={{
                                display: 'flex',
                                gap: 0.25,
                                alignItems: 'center',
                              }}
                            >
                              {item.type === 'remote' && (
                                <IconButton
                                  size="small"
                                  onClick={(e) =>
                                    handleUpdateProfile(item.uid, e)
                                  }
                                  sx={{
                                    p: 0.1,
                                    color: isActive
                                      ? 'primary.main'
                                      : 'text.secondary',
                                  }}
                                >
                                  <RefreshRounded sx={{ fontSize: 12 }} />
                                </IconButton>
                              )}
                              <IconButton
                                size="small"
                                onClick={(e) =>
                                  handleDeleteProfile(item.uid, e)
                                }
                                sx={{ p: 0.1, color: 'error.main' }}
                              >
                                <DeleteRounded sx={{ fontSize: 12 }} />
                              </IconButton>
                            </Box>
                          </Box>

                          {/* Line 2: Traffic & Expiration (Only for remote with extra data) */}
                          {item.type === 'remote' && hasExtra && (
                            <Box
                              sx={{
                                display: 'flex',
                                justifyContent: 'space-between',
                                alignItems: 'center',
                                mt: 0.25,
                                fontSize: '11px',
                                color: 'text.secondary',
                              }}
                            >
                              <span>
                                {formatTraffic(upload + download)} /{' '}
                                {formatTraffic(total)}
                              </span>
                              <span>
                                {extra?.expire
                                  ? formatExpire(extra.expire)
                                  : '-'}
                              </span>
                            </Box>
                          )}

                          {/* Line 3: Traffic progress bar (Only if total traffic > 0) */}
                          {item.type === 'remote' && total > 0 && (
                            <Box
                              sx={{
                                width: '100%',
                                height: 2,
                                bgcolor: 'action.hover',
                                borderRadius: 1,
                                mt: 0.5,
                                overflow: 'hidden',
                              }}
                            >
                              <Box
                                sx={{
                                  width: `${progress}%`,
                                  height: '100%',
                                  bgcolor: 'primary.main',
                                }}
                              />
                            </Box>
                          )}
                        </Box>
                      )
                    })}
                  </Box>
                </Box>

                {/* Section 2: Takeover Mode (三态互斥单选) */}
                <Box
                  sx={{
                    p: 1,
                    flexShrink: 0,
                    ...get3DCardStyle(theme, 'default'),
                    '&:hover': {
                      transform: 'none',
                      boxShadow: get3DCardStyle(theme, 'default').boxShadow,
                    },
                  }}
                >
                  <Typography
                    variant="subtitle2"
                    sx={{ fontWeight: 'bold', mb: 0.75, fontSize: '13px' }}
                  >
                    流量接管模式
                  </Typography>
                  <Box
                    sx={(theme) => ({
                      position: 'relative',
                      display: 'flex',
                      alignItems: 'center',
                      bgcolor: 'action.hover',
                      borderRadius: '4px',
                      p: '1px',
                      mb: 1,
                      height: 26,
                      userSelect: 'none',
                      ...get3DSegmentedContainerStyle(
                        theme.palette.mode === 'light',
                      ),
                    })}
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
                        transition:
                          'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                        transform: `translate3d(${activeIndex * 100}%, 0, 0)`,
                      }}
                    >
                      <Box sx={(theme) => get3DSegmentedActiveStyle(theme)} />
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
                        color:
                          activeIndex === 0
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '13px',
                        fontWeight: 'bold',
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
                        color:
                          activeIndex === 1
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '13px',
                        fontWeight: 'bold',
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
                        color:
                          activeIndex === 2
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '13px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      TUN 模式
                    </Box>
                  </Box>
                  <Typography
                    variant="subtitle2"
                    sx={{ fontWeight: 'bold', mb: 0.75, fontSize: '13px' }}
                  >
                    分流策略倾向
                  </Typography>
                  <Box
                    sx={(theme) => ({
                      position: 'relative',
                      display: 'flex',
                      alignItems: 'center',
                      bgcolor: 'action.hover',
                      borderRadius: '4px',
                      p: '1px',
                      height: 26,
                      userSelect: 'none',
                      ...get3DSegmentedContainerStyle(
                        theme.palette.mode === 'light',
                      ),
                    })}
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
                        transition:
                          'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                        transform: `translate3d(${policyActiveIndex * 100}%, 0, 0)`,
                      }}
                    >
                      <Box sx={(theme) => get3DSegmentedActiveStyle(theme)} />
                    </Box>

                    {/* Direct Fallback Option */}
                    <Box
                      onClick={() => handleRuleFallbackChange('direct')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color:
                          policyActiveIndex === 0
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '13px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      直连兜底
                    </Box>

                    {/* Rule Adjustable Option */}
                    <Box
                      onClick={() => handleRuleFallbackChange('adjustable')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color:
                          policyActiveIndex === 1
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '13px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      规则可调
                    </Box>

                    {/* Proxy Fallback Option */}
                    <Box
                      onClick={() => handleRuleFallbackChange('proxy')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color:
                          policyActiveIndex === 2
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '13px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      代理兜底
                    </Box>
                  </Box>
                </Box>

                {/* Section 3: Minimal Settings */}
                <Box
                  sx={{
                    p: 1,
                    flexShrink: 0,
                    ...get3DCardStyle(theme, 'default'),
                    '&:hover': {
                      transform: 'none',
                      boxShadow: get3DCardStyle(theme, 'default').boxShadow,
                    },
                  }}
                >
                  <Typography
                    variant="subtitle2"
                    sx={{ fontWeight: 'bold', mb: 0.5, fontSize: '13px' }}
                  >
                    基础设置
                  </Typography>
                  <List dense sx={{ py: 0 }}>
                    <ListItem
                      sx={{
                        py: 0.1,
                        px: 0.5,
                        display: 'flex',
                        justifyContent: 'space-between',
                      }}
                    >
                      <Typography variant="caption" sx={{ fontSize: '13px' }}>
                        开机自动启动
                      </Typography>
                      <Switch
                        size="small"
                        checked={verge?.enable_auto_launch ?? false}
                        onChange={(_, checked: boolean) =>
                          patchVerge({ enable_auto_launch: checked })
                        }
                        sx={{
                          transform: 'scale(0.9)',
                          transformOrigin: 'right center',
                        }}
                      />
                    </ListItem>
                    <ListItem
                      sx={{
                        py: 0.1,
                        px: 0.5,
                        display: 'flex',
                        justifyContent: 'space-between',
                      }}
                    >
                      <Typography variant="caption" sx={{ fontSize: '13px' }}>
                        启动时最小化
                      </Typography>
                      <Switch
                        size="small"
                        checked={verge?.enable_silent_start ?? false}
                        onChange={(_, checked: boolean) =>
                          patchVerge({ enable_silent_start: checked })
                        }
                        sx={{
                          transform: 'scale(0.9)',
                          transformOrigin: 'right center',
                        }}
                      />
                    </ListItem>
                    <ListItem
                      sx={{
                        py: 0.1,
                        px: 0.5,
                        display: 'flex',
                        justifyContent: 'space-between',
                      }}
                    >
                      <Typography variant="caption" sx={{ fontSize: '13px' }}>
                        通知弹窗显示
                      </Typography>
                      <Switch
                        size="small"
                        checked={notificationsEnabled}
                        onChange={(_, checked: boolean) => {
                          setNotificationsEnabled(checked)
                          localStorage.setItem(
                            'clash-verge-enable-notification',
                            checked ? 'true' : 'false',
                          )
                        }}
                        sx={{
                          transform: 'scale(0.9)',
                          transformOrigin: 'right center',
                        }}
                      />
                    </ListItem>
                    <ListItem
                      sx={{
                        py: 0.25,
                        px: 0.5,
                        display: 'flex',
                        justifyContent: 'space-between',
                        alignItems: 'center',
                      }}
                    >
                      <Typography variant="caption" sx={{ fontSize: '13px' }}>
                        Mixed Port
                      </Typography>
                      <Box
                        sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}
                      >
                        <TextField
                          size="small"
                          type="text"
                          value={mixedPortVal}
                          onChange={(e) =>
                            setMixedPortVal(
                              e.target.value
                                ? parseInt(e.target.value, 10) || 0
                                : 0,
                            )
                          }
                          onBlur={handleSavePort}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                              ;(e.target as HTMLInputElement).blur()
                            }
                          }}
                          slotProps={{
                            htmlInput: {
                              style: {
                                paddingTop: '2px',
                                paddingBottom: '2px',
                                paddingLeft: '4px',
                                paddingRight: '4px',
                                width: '80px',
                                fontSize: '13px',
                                textAlign: 'center',
                              },
                            },
                          }}
                          sx={get3DInputStyle(theme)}
                        />
                      </Box>
                    </ListItem>
                    <ListItem
                      sx={{
                        py: 0.1,
                        px: 0.5,
                        display: 'flex',
                        justifyContent: 'space-between',
                        alignItems: 'center',
                      }}
                    >
                      <Typography
                        variant="caption"
                        sx={{ fontSize: '13px', flexShrink: 0 }}
                      >
                        主题模式
                      </Typography>
                      <Box
                        sx={(theme) => ({
                          position: 'relative',
                          display: 'flex',
                          alignItems: 'center',
                          bgcolor: 'action.hover',
                          borderRadius: '4px',
                          p: '1px',
                          width: '120px',
                          height: 26,
                          userSelect: 'none',
                          ...get3DSegmentedContainerStyle(
                            theme.palette.mode === 'light',
                          ),
                        })}
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
                            transition:
                              'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                            transform: `translate3d(${themeActiveIndex * 100}%, 0, 0)`,
                          }}
                        >
                          <Box
                            sx={(theme) => get3DSegmentedActiveStyle(theme)}
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
                            color:
                              themeActiveIndex === 0
                                ? '#1E1200'
                                : 'text.secondary',
                            fontSize: '11px',
                            fontWeight: 'bold',
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
                            color:
                              themeActiveIndex === 1
                                ? '#1E1200'
                                : 'text.secondary',
                            fontSize: '11px',
                            fontWeight: 'bold',
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
                            color:
                              themeActiveIndex === 2
                                ? '#1E1200'
                                : 'text.secondary',
                            fontSize: '11px',
                            fontWeight: 'bold',
                            cursor: 'pointer',
                            zIndex: 1,
                            transition: 'color 0.2s ease',
                          }}
                        >
                          深色
                        </Box>
                      </Box>
                    </ListItem>
                    <ListItem
                      sx={{
                        py: 0.1,
                        px: 0.5,
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'stretch',
                        mt: 0.5,
                      }}
                    >
                      <Box
                        sx={{
                          display: 'flex',
                          justifyContent: 'space-between',
                          alignItems: 'center',
                          mb: 0.25,
                        }}
                      >
                        <Typography variant="caption" sx={{ fontSize: '13px' }}>
                          立体磨砂 (Depth)
                        </Typography>
                        <Typography
                          variant="caption"
                          sx={{
                            fontSize: '12px',
                            color: 'text.secondary',
                            fontWeight: 'bold',
                          }}
                        >
                          {depthFactor.toFixed(1)}
                        </Typography>
                      </Box>
                      <Slider
                        size="small"
                        value={depthFactor}
                        min={0.0}
                        max={2.0}
                        step={0.1}
                        onChange={(_, val) =>
                          handleDepthFactorChange(val as number)
                        }
                        sx={get3DSliderStyle(theme, mode)}
                      />
                    </ListItem>
                    <ListItem
                      sx={{
                        py: 0.1,
                        px: 0.5,
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'stretch',
                        mt: 0.5,
                      }}
                    >
                      <Box
                        sx={{
                          display: 'flex',
                          justifyContent: 'space-between',
                          alignItems: 'center',
                          mb: 0.25,
                        }}
                      >
                        <Typography variant="caption" sx={{ fontSize: '13px' }}>
                          色彩霓虹 (Vibrancy)
                        </Typography>
                        <Typography
                          variant="caption"
                          sx={{
                            fontSize: '12px',
                            color: 'text.secondary',
                            fontWeight: 'bold',
                          }}
                        >
                          {vibrancyFactor.toFixed(1)}
                        </Typography>
                      </Box>
                      <Slider
                        size="small"
                        value={vibrancyFactor}
                        min={0.0}
                        max={2.0}
                        step={0.1}
                        onChange={(_, val) =>
                          handleVibrancyFactorChange(val as number)
                        }
                        sx={get3DSliderStyle(theme, mode)}
                      />
                    </ListItem>
                  </List>

                  <Box
                    sx={{
                      display: 'flex',
                      justifyContent: 'center',
                      width: '100%',
                      mt: 1,
                    }}
                  >
                    <Button
                      variant="contained"
                      color="primary"
                      size="small"
                      onClick={() => setLogsOpen(true)}
                      sx={{
                        fontSize: 13,
                        height: 24,
                        px: 2,
                        ...get3DButtonStyle(theme, 'contained', 'primary'),
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
                <Box
                  sx={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 1,
                    px: 0.5,
                    width: '100%',
                    justifyContent: 'flex-start',
                  }}
                >
                  <Typography
                    variant="subtitle2"
                    sx={{
                      fontWeight: 'bold',
                      fontSize: '11px',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    路径控制（右键点击链接）
                  </Typography>
                  <Box
                    sx={(theme) => ({
                      position: 'relative',
                      display: 'flex',
                      alignItems: 'center',
                      bgcolor: 'action.hover',
                      borderRadius: '4px',
                      p: '1px',
                      width: '140px',
                      height: 24,
                      userSelect: 'none',
                      ...get3DSegmentedContainerStyle(
                        theme.palette.mode === 'light',
                      ),
                    })}
                  >
                    {/* Sliding Background Indicator */}
                    <Box
                      sx={{
                        position: 'absolute',
                        left: 0,
                        top: 0,
                        width: '50%',
                        height: '100%',
                        zIndex: 0,
                        transition:
                          'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                        transform: `translate3d(${connectionsType === 'active' ? 0 : 100}%, 0, 0)`,
                      }}
                    >
                      <Box sx={(theme) => get3DSegmentedActiveStyle(theme)} />
                    </Box>

                    {/* Active Option */}
                    <Box
                      onClick={() => setConnectionsType('active')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color:
                          connectionsType === 'active'
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      活跃 ({connectionsData?.activeConnections.length || 0})
                    </Box>

                    {/* Closed Option */}
                    <Box
                      onClick={() => setConnectionsType('closed')}
                      sx={{
                        flex: 1,
                        height: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color:
                          connectionsType === 'closed'
                            ? '#1E1200'
                            : 'text.secondary',
                        fontSize: '11px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        zIndex: 1,
                        transition: 'color 0.2s ease',
                      }}
                    >
                      历史 ({connectionsData?.closedConnections.length || 0})
                    </Box>
                  </Box>
                </Box>

                {/* Search and Action Row */}
                <Box
                  sx={{
                    display: 'flex',
                    gap: 1,
                    alignItems: 'center',
                    px: 0.5,
                    mt: 0.5,
                  }}
                >
                  <Box sx={{ flex: 1 }}>
                    <BaseSearchBox onSearch={handleSearch} />
                  </Box>
                  <Button
                    size="small"
                    variant="contained"
                    onClick={() => closeAllConnections()}
                    sx={{
                      fontSize: 11,
                      height: 24,
                      px: 1.5,
                      minWidth: 'auto',
                      ...get3DButtonStyle(theme, 'contained', 'primary'),
                    }}
                  >
                    断开全部
                  </Button>
                  {connectionsType === 'closed' && (
                    <Button
                      size="small"
                      variant="contained"
                      onClick={() => clearClosedConnections()}
                      sx={{
                        fontSize: 11,
                        height: 24,
                        px: 1.5,
                        minWidth: 'auto',
                        ...get3DButtonStyle(theme, 'contained', 'primary'),
                      }}
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
                        detailRef.current?.open(
                          detail,
                          connectionsType === 'closed',
                          el,
                        )
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
              flex: isMinimalWidth ? '1 1 0%' : '0 0 178px',
              height: isMinimalWidth ? 'auto' : '178px',
              borderTop: '1px solid',
              borderTopColor: 'var(--divider-color, rgba(0,0,0,0.12))',
              background: 'inherit',
              padding: isMinimalWidth ? '4px 12px 0px 12px' : '8px 12px 0px 12px',
              display: 'flex',
              gap: '12px',
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

      {/* Logs View Dialog */}
      <Dialog
        open={logsOpen}
        onClose={() => setLogsOpen(false)}
        maxWidth="md"
        fullWidth
        slotProps={{
          paper: {
            sx: {
              background:
                mode === 'light' ? 'var(--background-color)' : '#1e1f27',
              height: '480px',
              position: 'relative',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
              boxSizing: 'border-box',
              '& .base-page > header': {
                pr: 6,
              },
            },
          },
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
