import { Box, Typography, Button, useTheme } from '@mui/material'
import React from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox, BaseEmpty } from '@/components/base'
import { ConnectionTable } from '@/components/connection/connection-table'
import {
  get3DButtonStyle,
  get3DSegmentedContainerStyle,
  get3DSegmentedActiveStyle,
  get3DSegmentedActiveTextColor,
} from '@/utils/button-styles'
import { closeAllConnections } from 'tauri-plugin-mihomo-api'

interface ConnectionsPanelProps {
  connectionsType: 'active' | 'closed'
  setConnectionsType: (type: 'active' | 'closed') => void
  connectionsData: any
  handleSearch: (match: (content: string) => boolean, state: any) => void
  filterConn: any[]
  detailRef: React.RefObject<any>
  isColumnManagerOpen: boolean
  setIsColumnManagerOpen: (open: boolean) => void
  clearClosedConnections: () => void
}

export const ConnectionsPanel: React.FC<ConnectionsPanelProps> = ({
  connectionsType,
  setConnectionsType,
  connectionsData,
  handleSearch,
  filterConn,
  detailRef,
  isColumnManagerOpen,
  setIsColumnManagerOpen,
  clearClosedConnections,
}) => {
  const { t } = useTranslation() as any
  const theme = useTheme()

  return (
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
          {t('settings.mini.pathControl', {
            defaultValue: '路径控制（右键点击链接）',
          })}
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
            height: 18,
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
              transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
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
                  ? get3DSegmentedActiveTextColor(theme)
                  : 'text.secondary',
              fontSize: '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
            }}
          >
            {t('settings.mini.connectionsActive', {
              defaultValue: '活跃',
            })}{' '}
            ({connectionsData?.activeConnections?.length || 0})
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
                  ? get3DSegmentedActiveTextColor(theme)
                  : 'text.secondary',
              fontSize: '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
            }}
          >
            {t('settings.mini.connectionsHistory', {
              defaultValue: '历史',
            })}{' '}
            ({connectionsData?.closedConnections?.length || 0})
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
            height: 20,
            px: 1.5,
            minWidth: 'auto',
            ...get3DButtonStyle(theme, 'contained', 'primary'),
          }}
        >
          {t('settings.mini.connectionsDisconnectAll', {
            defaultValue: '断开全部',
          })}
        </Button>
        {connectionsType === 'closed' && (
          <Button
            size="small"
            variant="contained"
            onClick={clearClosedConnections}
            sx={{
              fontSize: 11,
              height: 20,
              px: 1.5,
              minWidth: 'auto',
              ...get3DButtonStyle(theme, 'contained', 'primary'),
            }}
          >
            {t('settings.mini.connectionsClearHistory', {
              defaultValue: '清空历史',
            })}
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
  )
}
