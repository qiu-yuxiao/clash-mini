import { alpha, Theme } from '@mui/material'

// 3D Bevel Button Styling Helper
export const get3DButtonStyle = (
  theme: Theme,
  variant: 'contained' | 'outlined',
  colorType: 'primary' | 'error' | 'default' = 'default',
) => {
  const isLight = theme.palette.mode === 'light'

  let backgroundStyle: string
  let textColor: string
  let borderColor: string

  if (colorType === 'primary') {
    textColor = isLight ? '#333333' : theme.palette.primary.contrastText
    borderColor = isLight ? 'rgba(0, 0, 0, 0.15)' : 'rgba(255, 255, 255, 0.15)'
    backgroundStyle = isLight
      ? `linear-gradient(to bottom, #ffffff 0%, ${alpha(theme.palette.primary.main, 0.15)} 100%)`
      : `linear-gradient(to bottom, ${theme.palette.primary.main} 0%, ${alpha(theme.palette.primary.main, 0.75)} 100%)`
  } else if (colorType === 'error') {
    textColor = '#ffffff'
    borderColor = 'rgba(0, 0, 0, 0.2)'
    backgroundStyle = isLight
      ? 'linear-gradient(to bottom, #ff7875 0%, #ff4d4f 100%)'
      : `linear-gradient(to bottom, ${theme.palette.error.main} 0%, ${alpha(theme.palette.error.main, 0.75)} 100%)`
  } else {
    textColor = isLight ? '#333333' : theme.palette.text.primary
    borderColor = isLight ? 'rgba(0, 0, 0, 0.15)' : 'rgba(255, 255, 255, 0.15)'
    backgroundStyle = isLight
      ? 'linear-gradient(to bottom, #ffffff 0%, #f0f0f0 100%)'
      : 'linear-gradient(to bottom, #303850 0%, #1e2438 100%)'
  }

  if (variant === 'outlined' && colorType === 'default') {
    backgroundStyle = isLight
      ? 'linear-gradient(to bottom, #ffffff 0%, #fafafa 100%)'
      : 'linear-gradient(to bottom, #2b3145 0%, #1a1e2d 100%)'
  }

  return {
    textTransform: 'none' as const,
    fontWeight: 'bold',
    transition: 'all 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
    border: '1px solid',
    borderColor,
    background: backgroundStyle,
    color: textColor,
    boxShadow: isLight
      ? `0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0,0,0,0.12),
         inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255,255,255,0.8),
         inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.08)${
           colorType === 'primary'
             ? ', 0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.15 * var(--vibrancy-factor, 1.0)))'
             : ''
         }`
      : `0 calc(2px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.3),
         inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(255,255,255,0.25),
         inset calc(-1.5px * var(--depth-factor, 1.0)) calc(-1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35)${
           colorType === 'primary'
             ? ', 0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.15 * var(--vibrancy-factor, 1.0)))'
             : ''
         }`,
    '&:hover': {
      transform: 'translateY(-1.5px)',
      boxShadow: isLight
        ? `0 calc(4px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) rgba(0,0,0,0.16),
           inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255,255,255,0.9),
           inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.1)${
             colorType === 'primary' || colorType === 'default'
               ? ', 0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.15 * var(--vibrancy-factor, 1.0)))'
               : ''
           }`
        : `0 calc(4px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.45),
           inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(255,255,255,0.35),
           inset calc(-1.5px * var(--depth-factor, 1.0)) calc(-1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.45)${
             colorType === 'primary' || colorType === 'default'
               ? ', 0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.15 * var(--vibrancy-factor, 1.0)))'
               : ''
           }`,
    },
    '&:active': {
      transform: 'translateY(1px)',
      boxShadow: isLight
        ? `inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0,0,0,0.15)`
        : `inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.5)`,
    },
  }
}
