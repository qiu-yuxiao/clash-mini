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
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
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
    filter: brightness(1.4);
    opacity: 0.5;
  }
  50% {
    transform: scale(1.08);
    filter: brightness(1.2);
    opacity: 1;
  }
  100% {
    transform: scale(1);
    filter: brightness(1);
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
}))

export const ProxyItem = (props: Props) => {
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
  const { delayValue, isPreset, timeout, onDelay } = useProxyDelayState(
    proxy,
    group?.name ?? '',
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
          const isTesting = delayValue === -2
          const bgcolor = isTesting
            ? 'transparent'
            : isOdd
              ? mode === 'light'
                ? 'rgba(0, 120, 215, 0.04)'
                : 'rgba(50, 100, 180, 0.08)'
              : 'transparent'
          const selectColor = mode === 'light' ? primary.main : primary.light
          const showDelay = delayValue > 0

          return {
            '&:hover .the-check': {
              display: !showDelay ? 'inline-block' : 'none',
            },
            '&:hover .the-delay': {
              display: showDelay ? 'inline-block' : 'none',
            },
            '&:hover .the-icon': { display: 'none' },
            '&:hover': {
              transform: 'translateY(-1.5px)',
              boxShadow:
                mode === 'light'
                  ? '0 3px 8px rgba(0, 0, 0, 0.08)'
                  : '0 3px 8px rgba(0, 0, 0, 0.3)',
              backgroundImage: isTesting
                ? undefined
                : mode === 'light'
                  ? 'linear-gradient(120deg, rgba(255,255,255,0) 30%, rgba(255,255,255,0.4) 50%, rgba(255,255,255,0) 70%)'
                  : 'linear-gradient(120deg, rgba(255,255,255,0) 30%, rgba(255,255,255,0.08) 50%, rgba(255,255,255,0) 70%)',
              backgroundSize: isTesting ? '400% 100%' : '200% 100%',
              animation: isTesting
                ? `${shimmer} 1.5s infinite linear`
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
                  backgroundImage:
                    mode === 'light'
                      ? 'linear-gradient(90deg, rgba(255,193,7,0.02) 25%, rgba(255,193,7,0.1) 37%, rgba(255,193,7,0.02) 63%)'
                      : 'linear-gradient(90deg, rgba(255,193,7,0.01) 25%, rgba(255,193,7,0.06) 37%, rgba(255,193,7,0.01) 63%)',
                  backgroundSize: '400% 100%',
                  animation: `${shimmer} 1.5s infinite linear`,
                }
              : {}),
            transition: 'all 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
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

        {!proxy?.provider && delayValue !== -2 && (
          <Widget
            className="the-check"
            onClick={(e) => {
              e.preventDefault()
              e.stopPropagation()
              onDelay()
            }}
            sx={({ palette }) => ({
              display: 'none',
              cursor: 'pointer',
              fontSize: '11px',
              border: `1px solid ${alpha(palette.primary.main, 0.3)}`,
              color: 'primary.main',
              ':hover': { bgcolor: alpha(palette.primary.main, 0.15) },
            })}
          >
            Check
          </Widget>
        )}

        {delayValue >= 0 && (
          <Widget
            key={delayValue}
            className="the-delay"
            onClick={(e) => {
              if (proxy?.provider) return
              e.preventDefault()
              e.stopPropagation()
              onDelay()
            }}
            sx={({ palette }) => ({
              color: delayManager.formatDelayColor(delayValue, timeout, palette.mode === 'dark'),
              cursor: proxy?.provider ? 'default' : 'pointer',
              fontSize: '11px',
              fontWeight: 600,
              animation: `${popIn} 0.4s ease-out`,
              ...(!proxy.provider
                ? { ':hover': { bgcolor: alpha(palette.primary.main, 0.15) } }
                : {}),
            })}
          >
            {delayManager.formatDelay(delayValue, timeout)}
          </Widget>
        )}
      </Box>
    </ListItemButton>
  )
}
