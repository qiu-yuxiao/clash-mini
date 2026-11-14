import { Box, Button, Tooltip, Typography, useTheme } from '@mui/material'
import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox, BaseEmpty } from '@/components/base'
import type { ConnectionDetailRef } from '@/components/connection/connection-detail'
import { ConnectionTable } from '@/components/connection/connection-table'
import type { PathControlActions } from '@/hooks/use-path-control'
import { closeAllConnectionsWithTimeout } from '@/services/mihomo-api'
import { get3DButtonStyle } from '@/utils/button-styles'
import type { PathControlConnectionItem } from '@/utils/path-control'
import {
  exportCustomRoutingRules,
  importCustomRoutingRules,
} from '@/utils/quick-routing'

interface ConnectionsPanelProps {
  handleSearch: (match: (content: string) => boolean, state?: unknown) => void
  filterConn: PathControlConnectionItem[]
  pathControlActions?: PathControlActions
  detailRef: React.RefObject<ConnectionDetailRef | null>
  isColumnManagerOpen: boolean
  setIsColumnManagerOpen: (open: boolean) => void
  containerRef?: React.RefObject<HTMLDivElement | null>
}

export const ConnectionsPanel: React.FC<ConnectionsPanelProps> = ({
  handleSearch,
  filterConn,
  pathControlActions,
  detailRef,
  isColumnManagerOpen,
  setIsColumnManagerOpen,
  containerRef,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const [routeIoBusy, setRouteIoBusy] = useState(false)

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
        {filterConn.length === 0 ? (
          <BaseEmpty />
        ) : (
          <ConnectionTable
            connections={filterConn}
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
            columnManagerOpen={isColumnManagerOpen}
            onCloseColumnManager={() => setIsColumnManagerOpen(false)}
          />
        )}
      </Box>
    </Box>
  )
}
