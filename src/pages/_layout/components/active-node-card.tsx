import {
  alpha,
  useTheme,
  Paper,
  Typography,
  Tooltip,
  Chip,
  CircularProgress,
} from '@mui/material'
import { useState, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { filterSort } from '@/components/proxy/use-filter-sort'
import { useProfiles } from '@/hooks/use-profiles'
import { useVerge } from '@/hooks/use-verge'
import { useProxiesData } from '@/providers/app-data-context'
import { getProxyAddr } from '@/services/cmds'
import delayManager from '@/services/delay'
import { get3DCardStyle } from '@/utils/button-styles'
import {
  healthcheckProxyProvider,
  selectNodeForGroup,
} from 'tauri-plugin-mihomo-api'

import {
  getFriendlyProtocolName,
  getSignalIcon,
  convertDelayColor,
} from '../utils/style-helpers'

export const ActiveNodeStatusCard = () => {
  const { proxies } = useProxiesData()
  const { profiles } = useProfiles()
  const currentProfileUid = profiles?.current || ''
  const { verge } = useVerge()
  const latencyTimeout = verge?.default_latency_timeout || 10000

  const primaryGroup = useMemo(() => {
    const groups = proxies?.groups || []
    const primaryKeywords = ['auto', 'select', 'proxy', '节点选择', '自动选择']
    return (
      groups.find((group: any) =>
        primaryKeywords.some((keyword) =>
          (group?.name ?? '').toLowerCase().includes(keyword.toLowerCase()),
        ),
      ) ||
      groups.filter((g: any) => g?.name !== 'GLOBAL')[0] ||
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
      if (activeNodeRecord?.provider) {
        await healthcheckProxyProvider(activeNodeRecord.provider)
      } else {
        await delayManager.checkDelay(activeNodeName, primaryGroup.name, 10000)
      }
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
          const {
            filterText = '',
            sortType = 0,
            filterMatchCase = false,
            filterMatchWholeWord = false,
            filterUseRegularExpression = false,
          } = groupState

          const searchState = {
            matchCase: filterMatchCase,
            matchWholeWord: filterMatchWholeWord,
            useRegularExpression: filterUseRegularExpression,
          }

          currentCandidateNodes = filterSort(
            primaryGroup.all,
            primaryGroup.name,
            filterText,
            sortType,
            latencyTimeout,
            searchState,
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
        await selectNodeForGroup(primaryGroup.name, nextNodeName)
      } catch (err) {
        console.error('Failed to select node:', err)
      }
    }
  }

  const { t } = useTranslation()
  const signalInfo = getSignalIcon(delay, t)
  const delayColor = convertDelayColor(delay)
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
          defaultValue: '当前活跃出口节点：',
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
              : delayManager.formatDelay(delay)
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
