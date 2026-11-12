import {
  alpha,
  Box,
  ListItemButton,
  styled,
  SxProps,
  Theme,
} from '@mui/material'
import React, { memo, useCallback } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseLoading } from '@/components/base'
import { useProxyDelayState } from '@/hooks/use-proxy-delay-state'
import { useWindowDecorations } from '@/hooks/use-window'
import { getProfiles } from '@/services/cmds'
import { getDelayManager } from '@/services/delay'
import { showNotice } from '@/services/notice-service'
import type { IProxyItem, IProxyGroupItem } from '@/types/clash'

interface Props {
  group: IProxyGroupItem
  proxy: IProxyItem
  selected: boolean
  showType?: boolean
  indexInGroup?: number
  sx?: SxProps<Theme>
  onClick?: (name: string) => void
}

const Widget = styled(Box)(() => ({
  padding: '1px 3px',
  fontSize: 10,
  borderRadius: '3px',
  height: '22px',
  display: 'flex',
  alignItems: 'center',
}))

export const ProxyItem = memo(
  (props: Props) => {
    const {
      group,
      proxy,
      selected,
      showType = true,
      indexInGroup = 0,
      sx,
      onClick,
    } = props

    const { t } = useTranslation()
    const { isMinimalWidth } = useWindowDecorations()
    const isMinimal = isMinimalWidth

    const displayName = (proxy?.name ?? '').replace(/\s\(\d{6}\)$/, '')
    const displayNow = proxy?.now ? proxy.now.replace(/\s\(\d{6}\)$/, '') : ''

    const { delayValue, isPreset, timeout } = useProxyDelayState(
      proxy,
      group?.name ?? '',
    )

    // BUG-357: 右侧延迟徽章设为独立点击热区（阻止冒泡），仅对该单节点实时探活而不切换当前连接
    const handleSingleDelayCheck = useCallback(
      async (e: React.MouseEvent) => {
        e.stopPropagation()
        if (isPreset || !proxy?.name || delayValue === -2) return
        const targetGroup = group?.name || 'PROXY'
        try {
          const currentUid = (await getProfiles())?.current || ''
          if (!currentUid) return
          await getDelayManager().checkDelay(
            currentUid,
            proxy.name,
            targetGroup,
          )
          getDelayManager().queueGroupNotification(targetGroup)
        } catch (err) {
          const msg = String(err instanceof Error ? err.message : err)
          if (msg.includes('AUTO_SELECT_BUSY')) {
            showNotice.info('settings.mini.testingInProgress')
          } else if (msg.includes('AUTO_SELECT_NO_RESULT')) {
            showNotice.error('settings.mini.testNotExecuted')
          } else {
            showNotice.error('settings.mini.testFailed', { message: msg })
          }
        }
      },
      [delayValue, group?.name, isPreset, proxy?.name],
    )

    return (
      <ListItemButton
        dense
        selected={selected}
        onClick={() => onClick?.(proxy?.name)}
        sx={[
          {
            borderRadius: 0,
            minHeight: '24px',
            height: 'auto',
            py: 0,
            px: 0,
            display: 'flex',
            alignItems: 'center',
            fontSize: '12px',
            overflow: 'hidden',
            borderBottom: (theme) => `2px solid ${theme.palette.divider}`,
          },
          ({ palette: { mode, primary } }) => {
            const isOdd = indexInGroup % 2 !== 0
            const bgcolor = isOdd
              ? mode === 'light'
                ? 'rgba(0, 120, 215, 0.04)'
                : 'rgba(50, 100, 180, 0.08)'
              : 'transparent'
            const selectColor = mode === 'light' ? primary.main : primary.light

            return {
              '&:hover .the-icon': { display: 'none' },
              '&.Mui-selected': {
                borderLeft: `3px solid ${selectColor}`,
                bgcolor:
                  mode === 'light'
                    ? alpha(primary.main, 0.15)
                    : alpha(primary.main, 0.35),
              },
              backgroundColor: bgcolor,
              transition: 'background-color 0.2s',
            }
          },
          ...(Array.isArray(sx) ? sx : sx ? [sx] : []),
        ]}
      >
        {isMinimal ? (
          <>
            <Box
              sx={{
                width: 55,
                height: '100%',
                flexShrink: 0,
                display: 'flex',
                justifyContent: 'flex-end',
                pr: 0.5,
                alignItems: 'center',
                boxSizing: 'border-box',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                fontSize: '11px',
                color: 'text.secondary',
                fontWeight: 'normal',
                textTransform: 'uppercase',
                borderRight: (theme) => `2px solid ${theme.palette.divider}`,
              }}
            >
              {proxy?.type ?? ''}
            </Box>

            <Box
              title={displayName}
              sx={{
                flex: 1,
                height: '100%',
                minWidth: 0,
                display: 'flex',
                justifyContent: 'flex-start',
                alignItems: 'center',
                px: 1,
                boxSizing: 'border-box',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                fontSize: '12px',
                color: 'text.primary',
                fontWeight: selected ? 'bold' : 'normal',
                borderRight: (theme) => `2px solid ${theme.palette.divider}`,
              }}
            >
              <Box
                sx={{
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {displayName}
              </Box>
            </Box>
          </>
        ) : (
          <Box
            title={`${displayName}${displayNow ? ` - ${displayNow}` : ''}`}
            sx={{
              flex: 1,
              height: '100%',
              minWidth: 0,
              display: 'flex',
              alignItems: 'center',
              px: 1,
              boxSizing: 'border-box',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              fontSize: '12px',
              color: 'text.primary',
              fontWeight: selected ? 'bold' : 'normal',
              borderRight: (theme) => `2px solid ${theme.palette.divider}`,
            }}
          >
            <Box
              sx={{
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {displayName}
              {showType && displayNow && (
                <Box
                  component="span"
                  sx={{ color: 'text.secondary', ml: 0.5, fontSize: '11px' }}
                >
                  - {displayNow}
                </Box>
              )}
            </Box>
          </Box>
        )}

        <Box
          title={t('settings.mini.testSingleNodeTooltip')}
          onClick={handleSingleDelayCheck}
          sx={{
            width: 65,
            height: '100%',
            flexShrink: 0,
            display: isPreset ? 'none' : 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            boxSizing: 'border-box',
            cursor: delayValue === -2 ? 'wait' : 'pointer',
            '&:hover .the-delay': {
              textDecoration: 'underline',
            },
          }}
        >
          {delayValue === -2 && (
            <Widget>
              <BaseLoading />
            </Widget>
          )}

          {delayValue === -1 && (
            <Widget
              className="the-delay"
              sx={{
                color: 'text.disabled',
                cursor: 'pointer',
                fontSize: '11px',
              }}
            >
              -
            </Widget>
          )}

          {delayValue >= 0 && (
            <Widget
              key={proxy?.name ?? `node-delay-${indexInGroup}`}
              className="the-delay"
              sx={({ palette }) => ({
                color: getDelayManager().formatDelayColor(
                  delayValue,
                  timeout,
                  palette.mode === 'dark',
                ),
                cursor: 'pointer',
                fontSize: '11px',
                fontWeight: 600,
              })}
            >
              {getDelayManager().formatDelay(delayValue, timeout)}
            </Widget>
          )}
        </Box>
      </ListItemButton>
    )
  },
  (prevProps, nextProps) => {
    const prevHistory = prevProps.proxy?.history
    const nextHistory = nextProps.proxy?.history
    const prevLastHistory =
      prevHistory && prevHistory.length > 0
        ? prevHistory[prevHistory.length - 1]
        : undefined
    const nextLastHistory =
      nextHistory && nextHistory.length > 0
        ? nextHistory[nextHistory.length - 1]
        : undefined

    return (
      prevProps.selected === nextProps.selected &&
      prevProps.showType === nextProps.showType &&
      prevProps.indexInGroup === nextProps.indexInGroup &&
      prevProps.group?.name === nextProps.group?.name &&
      prevProps.proxy?.name === nextProps.proxy?.name &&
      prevProps.proxy?.type === nextProps.proxy?.type &&
      prevProps.proxy?.now === nextProps.proxy?.now &&
      prevProps.proxy?.history?.length === nextProps.proxy?.history?.length &&
      prevLastHistory?.time === nextLastHistory?.time &&
      prevLastHistory?.delay === nextLastHistory?.delay
    )
  },
)
