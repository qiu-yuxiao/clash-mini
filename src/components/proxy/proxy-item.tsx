import {
  alpha,
  Box,
  ListItemButton,
  styled,
  SxProps,
  Theme,
} from '@mui/material'

import { BaseLoading } from '@/components/base'
import { useProxyDelayState } from '@/hooks/use-proxy-delay-state'
import delayManager from '@/services/delay'

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

const TypeBox = styled('span')(({ theme }) => ({
  display: 'inline-block',
  border: '1px solid #ccc',
  borderColor: alpha(theme.palette.text.secondary, 0.25),
  color: alpha(theme.palette.text.secondary, 0.7),
  borderRadius: 3,
  fontSize: 9,
  marginRight: '2px',
  padding: '0 3px',
  lineHeight: 1.3,
  whiteSpace: 'nowrap',
}))

export const ProxyItem = (props: Props) => {
  const { group, proxy, selected, showType = true, indexInGroup = 0, sx, onClick } = props

  // -1/<=0 为不显示，-2 为 loading
  const { delayValue, isPreset, timeout, onDelay } = useProxyDelayState(
    proxy,
    group.name,
  )

  return (
    <ListItemButton
      dense
      selected={selected}
      onClick={() => onClick?.(proxy.name)}
      sx={[
        {
          borderRadius: 0,
          height: '20px',
          minHeight: '20px',
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
            ? (mode === 'light' ? 'rgba(0, 120, 215, 0.04)' : 'rgba(50, 100, 180, 0.08)')
            : 'transparent'
          const selectColor = mode === 'light' ? primary.main : primary.light
          const showDelay = delayValue > 0

          return {
            '&:hover .the-check': { display: !showDelay ? 'inline-block' : 'none' },
            '&:hover .the-delay': { display: showDelay ? 'inline-block' : 'none' },
            '&:hover .the-icon': { display: 'none' },
            '&.Mui-selected': {
              borderLeft: `3px solid ${selectColor}`,
              bgcolor:
                mode === 'light'
                   ? alpha(primary.main, 0.15)
                   : alpha(primary.main, 0.35),
            },
            backgroundColor: bgcolor,
            transition: 'background-color 0.1s ease',
          }
        },
        ...(Array.isArray(sx) ? sx : sx ? [sx] : []),
      ]}
    >
        {/* Column 1: Name (Flex growth, overflow ellipsis) */}
        <Box
          title={`${proxy.name}${proxy.now ? ` - ${proxy.now}` : ''}`}
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
          <Box sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {proxy.name}
            {showType && proxy.now && (
              <Box component="span" sx={{ color: 'text.secondary', ml: 0.5, fontSize: '11px' }}>
                - {proxy.now}
              </Box>
            )}
          </Box>
        </Box>

        {/* Column 2: Protocol/Type (Width: 60px) */}
        <Box
          sx={{
            width: 60,
            height: '100%',
            flexShrink: 0,
            display: 'flex',
            alignItems: 'center',
            px: 1,
            boxSizing: 'border-box',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            fontSize: '11px',
            color: 'text.secondary',
            opacity: 0.85,
            borderRight: (theme) => `2px solid ${theme.palette.divider}`,
          }}
        >
          {proxy.type}
        </Box>

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

          {!proxy.provider && delayValue !== -2 && (
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
              className="the-delay"
              onClick={(e) => {
                if (proxy.provider) return
                e.preventDefault()
                e.stopPropagation()
                onDelay()
              }}
              sx={({ palette }) => ({
                color: delayManager.formatDelayColor(delayValue, timeout),
                cursor: proxy.provider ? 'default' : 'pointer',
                fontSize: '11px',
                fontWeight: 600,
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
