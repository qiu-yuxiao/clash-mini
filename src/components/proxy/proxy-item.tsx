import React, { memo, useEffect, useRef } from 'react'
import {
  alpha,
  Box,
  ListItemButton,
  styled,
  SxProps,
  Theme,
  keyframes,
} from '@mui/material'

const shimmer = keyframes`
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(100%);
  }
`

const hoverSweep = keyframes`
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
`

const popIn = keyframes`
  0% {
    transform: scale(0.85);
    opacity: 0.5;
  }
  50% {
    transform: scale(1.08);
    opacity: 1;
  }
  100% {
    transform: scale(1);
    opacity: 1;
  }
`

import { BaseLoading } from '@/components/base'
import { useProxyDelayState } from '@/hooks/use-proxy-delay-state'
import delayManager from '@/services/delay'
import type { IProxyItem, IProxyGroupItem } from '@/types/clash'

import { useWindowWidth } from './use-window-width'

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

export const ProxyItem = memo((props: Props) => {
  const {
    group,
    proxy,
    selected,
    showType = true,
    indexInGroup = 0,
    sx,
    onClick,
  } = props

  const { width } = useWindowWidth()
  const isMinimal = width <= 285

  const displayName = (proxy?.name ?? '').replace(/\s\(\d{6}\)$/, '')
  const displayNow = proxy?.now ? proxy.now.replace(/\s\(\d{6}\)$/, '') : ''

  // -1/<=0 为不显示，-2 为 loading
  const { delayValue, isPreset, timeout } = useProxyDelayState(
    proxy,
    group?.name ?? '',
  )

  const delayRef = useRef<HTMLDivElement>(null)
  const prevDelayRef = useRef(delayValue)

  useEffect(() => {
    if (prevDelayRef.current !== delayValue && delayRef.current) {
      delayRef.current.style.animation = 'none'
      void delayRef.current.offsetWidth
      delayRef.current.style.animation = `${popIn} 0.4s ease-out`
    }
    prevDelayRef.current = delayValue
  }, [delayValue])

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
          position: 'relative',
          contain: 'layout style',
          borderBottom: (theme) => `2px solid ${theme.palette.divider}`,
        },
        ({ palette: { mode, primary } }) => {
          const isOdd = indexInGroup % 2 !== 0
          const isTesting = delayValue === -2
          const bgcolor = isTesting
            ? 'transparent'
            : isOdd
              ? mode === 'light'
                ? 'rgba(0, 120, 215, 0.04)'
                : 'rgba(50, 100, 180, 0.08)'
              : 'transparent'
          const selectColor = mode === 'light' ? primary.main : primary.light

          return {
            '&:hover .the-icon': { display: 'none' },
            pointerEvents: isTesting ? 'none' : 'auto',
            '&:hover': {
              transform: isTesting ? 'none' : 'translateY(-1.5px)',
              boxShadow: isTesting
                ? 'none'
                : mode === 'light'
                  ? '0 3px 8px rgba(0, 0, 0, 0.08)'
                  : '0 3px 8px rgba(0, 0, 0, 0.3)',
              backgroundImage: isTesting
                ? 'none'
                : mode === 'light'
                  ? 'linear-gradient(120deg, rgba(255,255,255,0) 30%, rgba(255,255,255,0.4) 50%, rgba(255,255,255,0) 70%)'
                  : 'linear-gradient(120deg, rgba(255,255,255,0) 30%, rgba(255,255,255,0.08) 50%, rgba(255,255,255,0) 70%)',
              backgroundSize: isTesting ? undefined : '200% 100%',
              animation: isTesting
                ? 'none'
                : `${hoverSweep} 0.6s ease-out`,
            },
            '&.Mui-selected': {
              borderLeft: `3px solid ${selectColor}`,
              bgcolor:
                mode === 'light'
                  ? alpha(primary.main, 0.15)
                  : alpha(primary.main, 0.35),
            },
            backgroundColor: bgcolor,
            ...(isTesting
              ? {
                  '&::after': {
                    content: '""',
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: '100%',
                    backgroundImage:
                      mode === 'light'
                        ? 'linear-gradient(90deg, rgba(255,193,7,0) 25%, rgba(255,193,7,0.15) 50%, rgba(255,193,7,0) 75%)'
                        : 'linear-gradient(90deg, rgba(255,193,7,0) 25%, rgba(255,193,7,0.08) 50%, rgba(255,193,7,0) 75%)',
                    animation: `${shimmer} 1.5s infinite linear`,
                    pointerEvents: 'none',
                  },
                }
              : {}),
            transition: 'transform 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275), box-shadow 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275), background-color 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
          }
        },
        ...(Array.isArray(sx) ? sx : sx ? [sx] : []),
      ]}
    >
      {isMinimal ? (
        <>
          {/* Column 1: Protocol/Type (Width: 55px, centered, border-right divider) */}
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

          {/* Column 2: Name (Flex growth, centered, border-right divider) */}
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
        /* Column 1: Name (Flex growth, overflow ellipsis) */
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

      {/* Column 3: Delay (Width: 65px, centered) */}
      <Box
        sx={{
          width: 65,
          height: '100%',
          flexShrink: 0,
          display: isPreset ? 'none' : 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          boxSizing: 'border-box',
        }}
      >
        {delayValue === -2 && (
          <Widget>
            <BaseLoading />
          </Widget>
        )}

        {delayValue >= 0 && (
          <Widget
            key={proxy?.uid ?? `node-delay-${indexInGroup}`}
            ref={delayRef}
            className="the-delay"
            sx={({ palette }) => ({
              color: delayManager.formatDelayColor(
                delayValue,
                timeout,
                palette.mode === 'dark',
              ),
              cursor: 'default',
              fontSize: '11px',
              fontWeight: 600,
              animation: `${popIn} 0.4s ease-out`,
            })}
          >
            {delayManager.formatDelay(delayValue, timeout)}
          </Widget>
        )}
      </Box>
    </ListItemButton>
  )
}, (prevProps, nextProps) => {
  const prevHistory = prevProps.proxy?.history
  const nextHistory = nextProps.proxy?.history
  const prevLastHistory = prevHistory && prevHistory.length > 0 ? prevHistory[prevHistory.length - 1] : undefined
  const nextLastHistory = nextHistory && nextHistory.length > 0 ? nextHistory[nextHistory.length - 1] : undefined

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
})
