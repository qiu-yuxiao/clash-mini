import { styled } from '@mui/material/styles'
import { default as MuiSwitch, SwitchProps } from '@mui/material/Switch'

export const Switch = styled((props: SwitchProps) => (
  <MuiSwitch
    focusVisibleClassName=".Mui-focusVisible"
    disableRipple
    {...props}
  />
))(({ theme }) => {
  const isLight = theme.palette.mode === 'light'
  return {
    width: 28,
    height: 14,
    padding: 0,
    marginRight: 1,
    '& .MuiSwitch-switchBase': {
      padding: 0,
      margin: 0,
      transitionDuration: '300ms',
      '&.Mui-checked': {
        transform: 'translateX(14px)',
        color: '#fff',
        '& + .MuiSwitch-track': {
          background: isLight
            ? `linear-gradient(to bottom, #FFA000 0%, #E65100 100%)`
            : `linear-gradient(to bottom, rgba(255, 160, 0, 0.8) 0%, rgba(230, 81, 0, 0.9) 100%)`,
          opacity: 0.85,
          boxShadow: isLight
            ? `inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0,0,0,0.4),
               0 1px 0 rgba(255,255,255,0.8),
               0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.3 * var(--vibrancy-factor, 1.0)))`
            : `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.8),
               0 1px 0 rgba(255, 255, 255, 0.08),
               inset 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, 0.4),
               0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.4 * var(--vibrancy-factor, 1.0)))`,
        },
        '& .MuiSwitch-thumb': {
          background: isLight
            ? `radial-gradient(circle at 35% 35%, #ffffff 0%, #FFD54F 55%, #FFA000 100%)`
            : `radial-gradient(circle at 35% 35%, #ffffff 0%, #FFA000 55%, #E65100 100%)`,
          border: `1px solid ${isLight ? '#9E670B' : '#6E4302'}`,
        },
        '&.Mui-disabled + .MuiSwitch-track': {
          opacity: 0.5,
        },
      },
      '&.Mui-focusVisible .MuiSwitch-thumb': {
        border: '6px solid #fff',
      },
      '&.Mui-disabled .MuiSwitch-thumb': {
        color: isLight ? theme.palette.grey[100] : theme.palette.grey[600],
      },
      '&.Mui-disabled + .MuiSwitch-track': {
        opacity: isLight ? 0.7 : 0.3,
      },
    },
    '& .MuiSwitch-thumb': {
      boxSizing: 'border-box',
      width: 14,
      height: 14,
      border: `1px solid ${isLight ? 'rgba(255, 160, 0, 0.35)' : 'rgba(255, 160, 0, 0.4)'}`,
      background: isLight
        ? `radial-gradient(circle at 35% 35%, #ffffff 0%, rgba(255, 213, 79, 0.25) 60%, rgba(255, 160, 0, 0.45) 100%)`
        : `radial-gradient(circle at 35% 35%, #ffffff 0%, rgba(255, 160, 0, 0.3) 60%, rgba(230, 81, 0, 0.55) 100%)`,
      boxShadow: isLight
        ? `0 calc(2.5px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.25),
           inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.8),
           inset 0 calc(-1px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.15)`
        : `0 calc(3px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0,0,0,0.5),
           inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.4),
           inset 0 calc(-1.5px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.55)`,
      transition:
        'transform 0.1s ease-out, box-shadow 0.1s ease-out, filter 0.1s ease-out, background 0.1s ease-out',
    },
    '& .MuiSwitch-track': {
      borderRadius: 7,
      backgroundColor: isLight ? 'rgba(212, 175, 55, 0.15)' : 'rgba(212, 175, 55, 0.08)',
      opacity: 0.85,
      border: `1px solid ${isLight ? 'rgba(212, 175, 55, 0.25)' : 'rgba(212, 175, 55, 0.12)'}`,
      boxShadow: isLight
        ? 'inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.3), 0 1px 0 rgba(255, 255, 255, 0.8)'
        : 'inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.75), 0 1px 0 rgba(255, 255, 255, 0.08)',
      transition: theme.transitions.create(['background-color'], {
        duration: 500,
      }),
    },
    '&:hover .MuiSwitch-thumb': {
      transform: 'scale(1.2)',
      filter: 'brightness(1.15)',
      boxShadow: isLight
        ? `0 calc(4px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
           inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.9),
           inset 0 calc(-1px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.1),
           0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.35 * var(--vibrancy-factor, 1.0)))`
        : `0 calc(5px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.65),
           inset 0 calc(1.2px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.5),
           inset 0 calc(-1.5px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.45),
           0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.35 * var(--vibrancy-factor, 1.0)))`,
    },
  }
})
