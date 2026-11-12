import {
  alpha,
  useTheme,
  Paper,
  Typography,
  Tooltip,
  Chip,
  CircularProgress,
  keyframes,
} from '@mui/material'
import { useQueryClient } from '@tanstack/react-query'
import { useState, useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { filterSort } from '@/components/proxy/use-filter-sort'
import { useProfiles } from '@/hooks/use-profiles'
import { useProxiesData } from '@/providers/app-data-context'
import {
  frontendLog,
  getProxyAddr,
  notifyNodeSwitched,
  triggerAutoSelect,
} from '@/services/cmds'
import { getDelayManager, NODE_DELAY_MAX_MS } from '@/services/delay'
import { selectNodeForGroupWithTimeout } from '@/services/mihomo-api'
import { showNotice } from '@/services/notice-service'
import type { IProxyItem } from '@/types/clash'
import { get3DCardStyle } from '@/utils/button-styles'

import {
  broadcastNetworkHealth,
  getLatestNetworkHealth,
  type NetworkHealthStatus,
} from '../utils/notification-handlers'
import {
  getFriendlyProtocolName,
  getSignalIcon,
  convertDelayColor,
} from '../utils/style-helpers'

const healthPulse = keyframes`
  0% { opacity: 0.85; }
  50% { opacity: 1; }
  100% { opacity: 0.85; }
`

export const ActiveNodeStatusCard = () => {
  const { proxies } = useProxiesData()
  const { profiles, patchCurrent } = useProfiles()
  const queryClient = useQueryClient()
  const currentProfileUid = profiles?.current || ''
  // 判死/显示/轮换/探针阈值统一为 NODE_DELAY_MAX_MS(5000)，与后端死节点判定、
  // 探针超时（NODE_TEST_TIMEOUT_MS）一致；单点测速同样委托后端测量组，不再有独立的短超时。
  const latencyTimeout = NODE_DELAY_MAX_MS

  const primaryGroup = useMemo(() => {
    return proxies?.groups?.[0] ?? null
  }, [proxies])

  const activeNodeName = primaryGroup?.now || ''

  const activeNodeRecord = useMemo(() => {
    if (!activeNodeName) return null
    return proxies?.records?.[activeNodeName] ?? null
  }, [proxies, activeNodeName])

  const [delay, setDelay] = useState(() => {
    if (!activeNodeName || !primaryGroup?.name || !activeNodeRecord) return -1
    return getDelayManager().getDelayFix(activeNodeRecord, primaryGroup.name)
  })

  const hasRecord = !!activeNodeRecord
  const activeNodeRecordRef = useRef(activeNodeRecord)
  useEffect(() => {
    activeNodeRecordRef.current = activeNodeRecord
  }, [activeNodeRecord])

  // 通过 listener 响应式订阅延迟更新，不再依赖 proxiesData 驱动重算
  useEffect(() => {
    const record = activeNodeRecordRef.current
    if (!activeNodeName || !primaryGroup?.name || !record) return

    setDelay(getDelayManager().getDelayFix(record, primaryGroup.name))

    const handler = (update: { delay: number }) => {
      setDelay(update.delay)
    }
    getDelayManager().setListener(activeNodeName, primaryGroup.name, handler)
    return () => {
      // 带上具体 handler：列表项同样在监听该节点，无参调用会连它一起摘除
      getDelayManager().removeListener(
        activeNodeName,
        primaryGroup.name,
        handler,
      )
    }
  }, [activeNodeName, primaryGroup?.name, hasRecord])

  const [testing, setTesting] = useState(false)
  const [nodeAddr, setNodeAddr] = useState<string>('')
  const [networkHealth, setNetworkHealth] = useState<NetworkHealthStatus>(
    getLatestNetworkHealth,
  )

  useEffect(() => {
    const onHealth = (e: Event) => {
      const detail = (e as CustomEvent<NetworkHealthStatus>).detail
      if (detail) setNetworkHealth(detail)
    }
    window.addEventListener('clash-mini-network-health', onHealth)
    return () => {
      window.removeEventListener('clash-mini-network-health', onHealth)
    }
  }, [])

  const handleSelfHealRetest = async (e: React.MouseEvent) => {
    e.stopPropagation()
    if (!currentProfileUid || testing) return
    setTesting(true)
    try {
      const results = await triggerAutoSelect(
        currentProfileUid,
        undefined,
        0,
        true,
      )
      if (results.some(([, d]) => d > 0 && d < latencyTimeout)) {
        broadcastNetworkHealth('network_health::ok')
      } else if (results.length === 0) {
        showNotice.error('settings.mini.testNotExecuted')
      } else {
        showNotice.error('settings.mini.allNodesTimeout')
      }
      void queryClient.invalidateQueries({ queryKey: ['getProxies'] })
    } catch (err) {
      const msg = String(err instanceof Error ? err.message : err)
      if (msg.includes('AUTO_SELECT_BUSY')) {
        showNotice.info('settings.mini.testingInProgress')
      } else if (msg.includes('AUTO_SELECT_NO_RESULT')) {
        showNotice.error('settings.mini.testNotExecuted')
      } else {
        showNotice.error('settings.mini.testFailed', { message: msg })
      }
    } finally {
      setTesting(false)
    }
  }

  useEffect(() => {
    let cancelled = false
    if (!activeNodeName) {
      Promise.resolve().then(() => setNodeAddr(''))
      return
    }
    getProxyAddr(activeNodeName, activeNodeRecord?.provider)
      .then((res) => {
        if (cancelled) return
        if (res) {
          setNodeAddr(`${res[0]}:${res[1]}`)
        } else {
          setNodeAddr('')
        }
      })
      .catch((err) => {
        if (cancelled) return
        console.error('Failed to get proxy address:', err)
        setNodeAddr('')
      })
    return () => {
      cancelled = true
    }
  }, [activeNodeName, activeNodeRecord?.provider])

  const handleTestDelay = async (e: React.MouseEvent) => {
    e.stopPropagation()
    if (!activeNodeName || !primaryGroup?.name || !currentProfileUid) return
    setTesting(true)
    try {
      // 委托后端统一测量组（PROXY__METRICS）单点拨测：避免 TUN 下
      // /proxies/{name}/delay 对活跃节点自测回环导致的假 Timeout
      const res = await getDelayManager().checkDelay(
        currentProfileUid,
        activeNodeName,
        primaryGroup.name,
      )
      setDelay(res.delay)
      getDelayManager().queueGroupNotification(primaryGroup.name)
    } catch (err) {
      // 单点测速与群发测速/后台自愈共用后端同一把互斥锁（AUTO_SELECT_RUNNING），
      // 因此这些失败属控制面竞争/环境问题，必须给出可读提示，不能静默无响应。
      const msg = String(err instanceof Error ? err.message : err)
      if (msg.includes('AUTO_SELECT_BUSY')) {
        showNotice.info('settings.mini.testingInProgress')
      } else if (msg.includes('AUTO_SELECT_NO_RESULT')) {
        showNotice.error('settings.mini.testNotExecuted')
      } else {
        showNotice.error('settings.mini.testFailed', { message: msg })
      }
      console.error('[ActiveNodeCard] 单点测速失败:', err)
    } finally {
      setTesting(false)
    }
  }

  const handleCycleNode = async (e: React.MouseEvent) => {
    e.stopPropagation()
    if (!primaryGroup?.name || !primaryGroup?.all) return

    let currentCandidateNodes = primaryGroup.all
    try {
      const stateStr = localStorage.getItem('proxy-head-state')
      if (stateStr && currentProfileUid) {
        const stateObj = JSON.parse(stateStr)
        const groupState = stateObj[currentProfileUid]?.[primaryGroup.name]
        if (groupState) {
          const { filterText = '', sortType = 0 } = groupState

          currentCandidateNodes = filterSort(
            primaryGroup.all,
            primaryGroup.name,
            filterText,
            sortType,
            latencyTimeout,
          )
        }
      }
    } catch (err) {
      console.error('Failed to parse proxy-head-state for cycling:', err)
    }

    if (currentCandidateNodes.length === 0) return

    const currentIndex = currentCandidateNodes.findIndex(
      (node: IProxyItem) => node?.name === activeNodeName,
    )

    let nextNodeName = ''
    const len = currentCandidateNodes.length
    let found = false

    for (let i = 1; i <= len; i++) {
      const checkIndex = (currentIndex + i) % len
      const node = currentCandidateNodes[checkIndex]
      const delay = getDelayManager().getDelayFix(node, primaryGroup.name)
      const isTimeout = delay === 0 || delay >= latencyTimeout

      if (!isTimeout) {
        nextNodeName = node?.name
        found = true
        break
      }
    }

    if (!found) {
      const nextIndex = (currentIndex + 1) % len
      nextNodeName = currentCandidateNodes[nextIndex]?.name
    }

    if (nextNodeName) {
      // BUG-336: 顶栏点击轮换节点必须同步乐观更新 ['getProxies'] 缓存、通知后端切点宽限期并持久化写回 profile.selected
      const groupName = primaryGroup.name
      const previousSnapshot = queryClient.getQueryData(['getProxies'])
      queryClient.setQueryData(
        ['getProxies'],
        (old: Record<string, unknown> | undefined) => {
          if (!old || typeof old !== 'object') return old
          const next: Record<string, unknown> = { ...old }
          if (Array.isArray(old.groups)) {
            next.groups = old.groups.map((g: Record<string, unknown>) =>
              g && g.name === groupName ? { ...g, now: nextNodeName } : g,
            )
          }
          return next
        },
      )

      try {
        await selectNodeForGroupWithTimeout(groupName, nextNodeName)
        notifyNodeSwitched()
        if (currentProfileUid) {
          patchCurrent({
            selected: [{ name: 'PROXY', now: nextNodeName }],
          }).catch((saveErr) => {
            const msg = `[ActiveNodeCard] 保存轮换节点选择失败: ${saveErr instanceof Error ? saveErr.message : String(saveErr)}`
            console.error(msg)
            frontendLog('error', msg)
          })
        }
        void queryClient.invalidateQueries({ queryKey: ['getProxies'] })
      } catch (err) {
        if (previousSnapshot !== undefined) {
          queryClient.setQueryData(['getProxies'], previousSnapshot)
        } else {
          void queryClient.invalidateQueries({ queryKey: ['getProxies'] })
        }
        console.error('Failed to select node:', err)
      }
    }
  }

  const { t } = useTranslation()
  const theme = useTheme()
  const isDark = theme.palette.mode === 'dark'
  const signalInfo = getSignalIcon(delay, t, latencyTimeout, isDark)
  const delayColor = convertDelayColor(delay, latencyTimeout, isDark)
  const skinFallback = useMemo(() => {
    return typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d'
  }, [])
  const skin = theme.controlSkin || skinFallback
  const isRetro3DDark = skin === 'retro-3d' && isDark

  const chipTintBase = useMemo(() => {
    switch (signalInfo.color) {
      case 'success.main':
        return theme.palette.success.main
      case 'primary.main':
        return theme.palette.primary.main
      case 'warning.dark':
        return theme.palette.warning.dark
      case 'warning.main':
        return theme.palette.warning.main
      case 'error.main':
        return theme.palette.error.main
      default:
        return theme.palette.text.secondary
    }
  }, [signalInfo.color, theme.palette])

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
        minWidth: 0,
        overflow: 'hidden',
        ...get3DCardStyle(theme, 'default'),
      }}
    >
      <Typography
        variant="caption"
        color={isRetro3DDark ? '#2C1F03' : 'text.secondary'}
        sx={{
          fontSize: '11px',
          '@media (max-width: 580px)': {
            display: 'none',
          },
        }}
      >
        {t('settings.mini.activeNodeLabel', {
          defaultValue: '活跃节点：',
        })}
      </Typography>

      <Typography
        variant="caption"
        sx={{
          fontWeight: 'bold',
          fontSize: '11px',
          color: isRetro3DDark ? '#2C1F03' : 'primary.main',
          border: '1px solid',
          borderColor: isRetro3DDark ? '#2C1F03' : 'primary.main',
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

      <Tooltip title={t('settings.mini.rotateNodeTooltip')}>
        <Typography
          variant="body2"
          onClick={handleCycleNode}
          sx={{
            fontWeight: 'bold',
            fontSize: '12px',
            color: isRetro3DDark ? '#2C1F03' : 'text.primary',
            maxWidth: { xs: '120px', sm: '180px' },
            minWidth: 0,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            cursor: 'pointer',
            transition: 'color 0.2s',
            '&:hover': {
              color: isRetro3DDark ? 'rgba(44, 31, 3, 0.65)' : 'primary.main',
            },
            '&:active': {
              color: isRetro3DDark ? 'rgba(44, 31, 3, 0.45)' : undefined,
            },
          }}
        >
          {(activeNodeName ? activeNodeName.replace(/\s\(\d{6}\)$/, '') : '') ||
            t('home.components.currentProxy.labels.noActiveNode', {
              defaultValue: '未选择节点 (直接连接)',
            })}
        </Typography>
      </Tooltip>

      {activeNodeName && (
        <Chip
          size="small"
          icon={
            testing || delay === -2 ? (
              <CircularProgress
                size={10}
                color="inherit"
                sx={{ width: 10, height: 10 }}
              />
            ) : (
              signalInfo.icon
            )
          }
          label={
            testing || delay === -2
              ? t('settings.mini.statusTesting', { defaultValue: '测试中' }) +
                '...'
              : getDelayManager().formatDelay(delay, latencyTimeout)
          }
          color={delayColor}
          onClick={handleTestDelay}
          sx={{
            fontSize: '11px',
            height: '20px',
            fontWeight: 600,
            cursor: 'pointer',
            bgcolor: testing ? undefined : alpha(chipTintBase, 0.12),
            color:
              signalInfo.color === 'text.secondary'
                ? isRetro3DDark
                  ? '#2C1F03'
                  : 'text.secondary'
                : signalInfo.color,
            '& .MuiChip-label': {
              color: 'inherit',
            },
            '& .MuiChip-icon': {
              color: 'inherit',
              fontSize: '12px',
            },
          }}
        />
      )}

      {networkHealth !== 'network_health::ok' && (
        <Tooltip
          title={
            networkHealth === 'network_health::offline'
              ? t('settings.mini.networkOfflineTooltip')
              : t('settings.mini.allNodesTimeoutTooltip')
          }
        >
          <Chip
            size="small"
            color="error"
            label={
              networkHealth === 'network_health::offline'
                ? t('settings.mini.networkOffline')
                : t('settings.mini.allNodesTimeout')
            }
            onClick={handleSelfHealRetest}
            sx={{
              fontSize: '10px',
              height: '20px',
              fontWeight: 700,
              cursor: 'pointer',
              animation: `${healthPulse} 1.8s infinite ease-in-out`,
            }}
          />
        </Tooltip>
      )}

      {nodeAddr && networkHealth === 'network_health::ok' && (
        <Typography
          variant="caption"
          color={isRetro3DDark ? '#2C1F03' : 'text.secondary'}
          sx={{
            fontWeight: 'bold',
            fontSize: '11px',
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
