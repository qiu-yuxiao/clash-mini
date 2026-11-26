import { Box, Button, Tooltip, Typography, useTheme } from '@mui/material'
import React, { useCallback, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import {
  ConnectionDetail,
  type ConnectionDetailRef,
} from '@/components/connection/connection-detail'
import { ConnectionTable } from '@/components/connection/connection-table'
import { useConnectionData } from '@/hooks/use-connection-data'
import { usePathControl } from '@/hooks/use-path-control'
import { closeAllConnectionsWithTimeout } from '@/services/mihomo-api'
import type { IConnectionsItem } from '@/types/connection'
import { get3DButtonStyle } from '@/utils/button-styles'
import {
  matchesPathControlSearch,
  prioritizePathControlConnections,
  type PathControlConnectionItem,
  type PathControlViewMode,
} from '@/utils/path-control'
import {
  exportCustomRoutingRules,
  importCustomRoutingRules,
} from '@/utils/quick-routing'

interface ConnectionsPanelProps {
  containerRef?: React.RefObject<HTMLDivElement | null>
}

const sortByNewestStart = (list: IConnectionsItem[]) =>
  list.sort(
    (a, b) =>
      new Date(b.start || '0').getTime() - new Date(a.start || '0').getTime(),
  )

export const ConnectionsPanel: React.FC<ConnectionsPanelProps> = ({
  containerRef,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const [routeIoBusy, setRouteIoBusy] = useState(false)
  const [viewMode, setViewMode] = useState<PathControlViewMode>('active')
  const [match, setMatch] = useState<(input: string) => boolean>(
    () => () => true,
  )
  const detailRef = useRef<ConnectionDetailRef | null>(null)

  // BUG-385: 将连接数据订阅与路径控制状态收口至面板生命周期内；
  // 关闭设置或缩回窄窗卸载本面板时，Observer 即刻归零并按 gcTime(30s) 释放缓存，同时避免每秒重绘整个 Layout
  const {
    response: { data: connectionsData },
  } = useConnectionData({ enabled: true })
  const {
    activeControlConnections,
    closedControlConnections,
    actions: pathControlActions,
  } = usePathControl({
    activeConnections: connectionsData?.activeConnections ?? [],
    closedConnections: connectionsData?.closedConnections ?? [],
    enabled: true,
  })

  const baseConnections =
    viewMode === 'active' ? activeControlConnections : closedControlConnections

  const filterConn = useMemo<PathControlConnectionItem[]>(() => {
    const matchConns = baseConnections.filter((conn) =>
      matchesPathControlSearch(conn, match),
    )
    return prioritizePathControlConnections(matchConns, sortByNewestStart)
  }, [baseConnections, match])

  const handleSearch = useCallback(
    (nextMatch: (content: string) => boolean) => {
      setMatch(() => nextMatch)
    },
    [],
  )

  const actionBtnSx = {
    fontSize: 11,
    height: 20,
    px: 1.25,
    minWidth: 'auto',
    lineHeight: 1,
    whiteSpace: 'nowrap',
    flexShrink: 0,
    ...get3DButtonStyle(theme, 'contained', 'primary'),
  } as const

  const handleSaveCustomRoutes = async () => {
    if (routeIoBusy) return
    setRouteIoBusy(true)
    try {
      await exportCustomRoutingRules()
    } finally {
      setRouteIoBusy(false)
    }
  }

  const handleImportCustomRoutes = async () => {
    if (routeIoBusy) return
    setRouteIoBusy(true)
    try {
      await importCustomRoutingRules()
    } finally {
      setRouteIoBusy(false)
    }
  }

  return (
    <>
      <Box
        ref={containerRef}
        sx={{
          flex: 1,
          // WARNING [FOR AI AGENTS / AUDITORS]:
          // This minWidth must remain 0 and MUST NOT be set to a fixed minimum width.
          // In large window the panel mounts and this prevents it from overflowing the flex row.
          // (In narrow window the panel is conditionally unmounted, so it never reaches this squeeze path.)
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: 0.5,
          height: '100%',
          minHeight: 0,
          overflow: 'hidden',
          pb: { xs: 0, '@media (min-height: 831px)': '24px' },
        }}
      >
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            gap: 0.75,
            flexWrap: 'wrap',
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
              flexShrink: 0,
            }}
          >
            {t('settings.mini.pathControl', {
              defaultValue: '路径控制（右键点击链接）',
            })}
          </Typography>
          <Tooltip
            title={t('settings.mini.saveCustomRoutesTooltip', {
              defaultValue:
                '将所有自定义路径控制路由规则保存到指定文件（重装后可导入恢复）',
            })}
            arrow
          >
            <span>
              <Button
                size="small"
                variant="contained"
                disabled={routeIoBusy}
                onClick={handleSaveCustomRoutes}
                sx={actionBtnSx}
              >
                {t('settings.mini.saveCustomRoutes', {
                  defaultValue: '保存路由',
                })}
              </Button>
            </span>
          </Tooltip>
          <Tooltip
            title={t('settings.mini.importCustomRoutesTooltip', {
              defaultValue: '从保存的文件导入自定义路径控制路由规则',
            })}
            arrow
          >
            <span>
              <Button
                size="small"
                variant="contained"
                disabled={routeIoBusy}
                onClick={handleImportCustomRoutes}
                sx={actionBtnSx}
              >
                {t('settings.mini.importCustomRoutes', {
                  defaultValue: '导入路由',
                })}
              </Button>
            </span>
          </Tooltip>
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
            onClick={() => closeAllConnectionsWithTimeout()}
            sx={actionBtnSx}
          >
            {t('settings.mini.connectionsDisconnectAll', {
              defaultValue: '断开全部',
            })}
          </Button>
        </Box>

        {/* Connection Table Container */}
        <Box
          sx={{
            flex: 1,
            height: '100%',
            minHeight: 0,
            mt: 0.5,
            display: 'flex',
            flexDirection: 'column',
            overflow: 'hidden',
          }}
        >
          <ConnectionTable
            connections={filterConn}
            viewMode={viewMode}
            onViewModeChange={setViewMode}
            onShowDetail={(detail, el) =>
              detailRef.current?.open(
                detail,
                Boolean(detail._isClosed || detail._isSynthetic),
                el,
              )
            }
            onMarkDisconnected={pathControlActions?.markConnectionDisconnected}
            onClearDisconnectedTarget={
              pathControlActions?.clearDisconnectedTarget
            }
            onCloseMatchingTarget={
              pathControlActions?.closeMatchingActiveConnections
            }
          />
        </Box>
      </Box>
      <ConnectionDetail ref={detailRef} />
    </>
  )
}
