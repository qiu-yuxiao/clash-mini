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
          background: isLight ? '#bdbdbd' : '#39393d',
          opacity: 0.85,
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
      border: `1px solid ${isLight ? alpha(theme.palette.primary.dark, 0.25) : 'rgba(255, 255, 255, 0.15)'}`,
      background: isLight
        ? `linear-gradient(to bottom, ${theme.palette.primary.light} 0%, ${theme.palette.primary.main} 100%)`
        : `linear-gradient(to bottom, ${theme.palette.primary.main} 0%, ${alpha(theme.palette.primary.main, 0.75)} 100%)`,
      boxShadow: isLight
        ? `0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0,0,0,0.12),
           inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255,255,255,0.8),
           inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.08),
           0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.25 * var(--vibrancy-factor, 1.0)))`
        : `0 calc(2px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.3),
           inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(255,255,255,0.25),
           inset calc(-1.5px * var(--depth-factor, 1.0)) calc(-1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
           0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.25 * var(--vibrancy-factor, 1.0)))`,
      transition:
        'transform 0.1s ease-out, box-shadow 0.1s ease-out, filter 0.1s ease-out',
    },
    '& .MuiSwitch-track': {
      borderRadius: 7,
      backgroundColor: isLight ? '#e0e0e0' : '#1e222b',
      opacity: 0.85,
      border: `1px solid ${isLight ? 'rgba(0,0,0,0.06)' : 'rgba(255,255,255,0.05)'}`,
      boxShadow: isLight
        ? 'inset 1.5px 1.5px 3px rgba(0,0,0,0.2)'
        : 'inset 2px 2px 4px rgba(0,0,0,0.6)',
      transition: theme.transitions.create(['background-color'], {
        duration: 500,
      }),
    },
    '&:hover .MuiSwitch-thumb': {
      transform: 'scale(1.2)',
      filter: 'brightness(1.15)',
      boxShadow: isLight
        ? `0 calc(4px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) rgba(0,0,0,0.16),
           inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255,255,255,0.9),
           inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.1),
           0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
        : `0 calc(4px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.45),
           inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(255,255,255,0.35),
           inset calc(-1.5px * var(--depth-factor, 1.0)) calc(-1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.45),
           0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`,
    },
  }
})
