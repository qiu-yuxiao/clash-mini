import {
  alpha,
  useTheme,
  Paper,
  Typography,
  Tooltip,
  Chip,
  CircularProgress,
} from '@mui/material'
import { useState, useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { filterSort } from '@/components/proxy/use-filter-sort'
import { useProfiles } from '@/hooks/use-profiles'
import { useProxiesData } from '@/providers/app-data-context'
import { getProxyAddr } from '@/services/cmds'
import delayManager, { NODE_DELAY_MAX_MS } from '@/services/delay'
import { selectNodeForGroupWithTimeout } from '@/services/mihomo-api'
import { get3DCardStyle } from '@/utils/button-styles'

import {
  getFriendlyProtocolName,
  getSignalIcon,
  convertDelayColor,
} from '../utils/style-helpers'

export const ActiveNodeStatusCard = () => {
  const { proxies } = useProxiesData()
  const { profiles } = useProfiles()
  const currentProfileUid = profiles?.current || ''
  // 判死/显示/轮换阈值统一为 NODE_DELAY_MAX_MS(2000)，与后端死节点判定、探针超时一致；
  // 单点测速仍走 singleTestTimeout(1000) 追求快速响应
  const latencyTimeout = NODE_DELAY_MAX_MS
  const singleTestTimeout = 1000

  const primaryGroup = useMemo(() => {
    return proxies?.groups?.find((g: any) => g?.name === 'PROXY') ?? null
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

  const [delay, setDelay] = useState(() => {
    if (!activeNodeName || !primaryGroup?.name || !activeNodeRecord) return -1
    return delayManager.getDelayFix(activeNodeRecord, primaryGroup.name)
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

    setDelay(delayManager.getDelayFix(record, primaryGroup.name))

    const handler = (update: { delay: number }) => {
      setDelay(update.delay)
    }
    delayManager.setListener(activeNodeName, primaryGroup.name, handler)
    return () => {
      delayManager.removeListener(activeNodeName, primaryGroup.name)
    }
  }, [activeNodeName, primaryGroup?.name, hasRecord])

  const [testing, setTesting] = useState(false)
  const [nodeAddr, setNodeAddr] = useState<string>('')

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
    if (!activeNodeName || !primaryGroup?.name) return
    setTesting(true)
    try {
      const res = await delayManager.checkDelay(
        activeNodeName,
        primaryGroup.name,
        singleTestTimeout,
      )
      setDelay(res.delay)
      delayManager.queueGroupNotification(primaryGroup.name)
    } catch (err) {
      console.error(err)
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
      (node: any) => node?.name === activeNodeName,
    )

    let nextNodeName = ''
    const len = currentCandidateNodes.length
    let found = false

    for (let i = 1; i <= len; i++) {
      const checkIndex = (currentIndex + i) % len
      const node = currentCandidateNodes[checkIndex]
      const delay = delayManager.getDelayFix(node, primaryGroup.name)
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
      try {
        await selectNodeForGroupWithTimeout(primaryGroup.name, nextNodeName)
      } catch (err) {
        console.error('Failed to select node:', err)
      }
    }
  }

  const { t } = useTranslation()
  const signalInfo = getSignalIcon(delay, t)
  const delayColor = convertDelayColor(delay, latencyTimeout)
  const theme = useTheme()
  const skinFallback = useMemo(() => {
    return typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d'
  }, [])
  const skin = theme.controlSkin || skinFallback
  const isRetro3DDark = skin === 'retro-3d' && theme.palette.mode === 'dark'

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

      <Tooltip title="点击轮换下一个节点">
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
              : delayManager.formatDelay(delay, latencyTimeout)
          }
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
                ? isRetro3DDark
                  ? '#2C1F03'
                  : 'text.secondary'
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
