import { styled, alpha } from '@mui/material/styles'
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
            ? `linear-gradient(to bottom, ${theme.palette.primary.dark} 0%, ${theme.palette.primary.main} 100%)`
            : `linear-gradient(to bottom, ${alpha(theme.palette.primary.dark, 0.8)} 0%, ${alpha(theme.palette.primary.main, 0.9)} 100%)`,
          opacity: 0.85,
          boxShadow: isLight
            ? `inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0,0,0,0.4),
               0 1px 0 rgba(255,255,255,0.8),
               0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.3 * var(--vibrancy-factor, 1.0)))`
            : `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.8),
               0 1px 0 rgba(255, 255, 255, 0.08),
               inset 0 0 calc(8px * var(--vibrancy-factor, 1.0)) ${alpha(theme.palette.primary.main, 0.4)},
               0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.4 * var(--vibrancy-factor, 1.0)))`,
        },
        '& .MuiSwitch-thumb': {
          background: isLight
            ? `radial-gradient(circle at 35% 35%, #ffffff 0%, ${theme.palette.primary.light} 55%, ${theme.palette.primary.main} 100%)`
            : `radial-gradient(circle at 35% 35%, #ffffff 0%, ${theme.palette.primary.main} 55%, ${theme.palette.primary.dark} 100%)`,
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
      border: `1px solid ${isLight ? alpha(theme.palette.primary.main, 0.35) : alpha(theme.palette.primary.main, 0.4)}`,
      background: isLight
        ? `radial-gradient(circle at 35% 35%, #ffffff 0%, ${alpha(theme.palette.primary.light, 0.3)} 60%, ${alpha(theme.palette.primary.main, 0.5)} 100%)`
        : `radial-gradient(circle at 35% 35%, #ffffff 0%, ${alpha(theme.palette.primary.main, 0.4)} 60%, ${alpha(theme.palette.primary.dark, 0.65)} 100%)`,
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
      backgroundColor: isLight ? alpha(theme.palette.primary.main, 0.15) : alpha(theme.palette.primary.main, 0.08),
      opacity: 0.85,
      border: `1px solid ${isLight ? alpha(theme.palette.primary.main, 0.22) : alpha(theme.palette.primary.main, 0.12)}`,
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
           0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
        : `0 calc(5px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.65),
           inset 0 calc(1.2px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.5),
           inset 0 calc(-1.5px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.45),
           0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`,
    },
  }
})
