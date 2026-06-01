import { Theme } from '@mui/material'

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
  let bevelShadowDark: string

  if (colorType === 'primary') {
    textColor = '#ffffff'
    borderColor = isLight ? 'rgba(0, 0, 0, 0.25)' : 'rgba(255, 255, 255, 0.15)'
    backgroundStyle =
      'linear-gradient(135deg, #FFE875 0%, #FFA000 45%, #F57C00 75%, #D84315 100%)'
    bevelShadowDark = isLight ? '#9E670B' : '#6E4302'
  } else if (colorType === 'error') {
    textColor = '#ffffff'
    borderColor = 'rgba(0, 0, 0, 0.2)'
    backgroundStyle =
      'linear-gradient(135deg, #FF8A80 0%, #FF1744 45%, #D50000 80%, #880E4F 100%)'
    bevelShadowDark = isLight ? '#B71C1C' : '#6B0505'
  } else {
    textColor = isLight ? '#2D3748' : theme.palette.text.primary
    borderColor = isLight ? 'rgba(0, 0, 0, 0.15)' : 'rgba(255, 255, 255, 0.15)'
    backgroundStyle = isLight
      ? 'linear-gradient(135deg, #FFFFFF 0%, #F1F5F9 50%, #CBD5E1 100%)'
      : 'linear-gradient(135deg, #475569 0%, #334155 50%, #1E293B 100%)'
    bevelShadowDark = isLight ? '#94A3B8' : '#0F172A'
  }

  if (variant === 'outlined' && colorType === 'default') {
    backgroundStyle = isLight
      ? 'linear-gradient(to bottom, #ffffff 0%, #fafafa 100%)'
      : 'linear-gradient(to bottom, #222a3d 0%, #161a29 100%)'
  }

  return {
    textTransform: 'none' as const,
    fontWeight: 'bold',
    borderRadius: '2px', // Force sharp cubical corners
    transition: 'all 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
    border: '1px solid',
    borderColor,
    background: backgroundStyle,
    color: textColor,
    boxShadow: isLight
      ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.4),
         0 calc(7px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.3),
         inset 0 1px 0 rgba(255, 255, 255, 0.4),
         inset 1px 1px 0 rgba(255, 255, 255, 0.2),
         inset -1px -1px 0 rgba(0, 0, 0, 0.15)${
           colorType === 'primary' || colorType === 'error'
             ? `, 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.15 * var(--vibrancy-factor, 1.0)))`
             : ''
         }`
      : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.55),
         0 calc(7px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45),
         inset 0 1px 0 rgba(255, 255, 255, 0.3),
         inset 1px 1px 0 rgba(255, 255, 255, 0.15),
         inset -1px -1px 0 rgba(0, 0, 0, 0.25)${
           colorType === 'primary' || colorType === 'error'
             ? `, 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.15 * var(--vibrancy-factor, 1.0)))`
             : ''
         }`,
    '&:hover': {
      transform: 'translateY(-2px)',
      boxShadow: isLight
        ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(5px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(6px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(7px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(7px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.35),
           0 calc(10px * var(--depth-factor, 1.0)) calc(15px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.25),
           inset 0 1px 0 rgba(255, 255, 255, 0.5),
           inset 1px 1px 0 rgba(255, 255, 255, 0.3),
           inset -1px -1px 0 rgba(0, 0, 0, 0.1)${
             colorType === 'primary' ||
             colorType === 'error' ||
             colorType === 'default'
               ? `, 0 0 calc(12px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`
        : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(5px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(6px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(7px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(7px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.45),
           0 calc(10px * var(--depth-factor, 1.0)) calc(15px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.4),
           inset 0 1px 0 rgba(255, 255, 255, 0.4),
           inset 1px 1px 0 rgba(255, 255, 255, 0.2),
           inset -1px -1px 0 rgba(0, 0, 0, 0.2)${
             colorType === 'primary' ||
             colorType === 'error' ||
             colorType === 'default'
               ? `, 0 0 calc(12px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`,
    },
    '&:active': {
      transform: 'translateY(4px)',
      boxShadow: isLight
        ? `0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.5),
           inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45)`
        : `0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.7),
           inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.65)`,
    },
  }
}

// 3D Recessed Input Styling Helper
export const get3DInputStyle = (theme: Theme) => {
  const isLight = theme.palette.mode === 'light'
  return {
    '& .MuiOutlinedInput-root': {
      backgroundColor: isLight ? '#ffffff' : 'transparent',
      boxShadow: isLight
        ? `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.25),
           0 1px 0 rgba(255, 255, 255, 0.8)`
        : `inset 0 calc(4px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.65),
           0 1px 0 rgba(255, 255, 255, 0.08)`,
      transition: 'box-shadow 0.2s ease, border-color 0.2s ease',
      '& .MuiOutlinedInput-notchedOutline': {
        borderColor: isLight
          ? 'rgba(0, 0, 0, 0.25)'
          : 'rgba(255, 255, 255, 0.25)',
      },
      '&:hover .MuiOutlinedInput-notchedOutline': {
        borderColor: isLight
          ? 'rgba(0, 0, 0, 0.4)'
          : 'rgba(255, 255, 255, 0.4)',
      },
      '&.Mui-focused': {
        boxShadow: isLight
          ? `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.3),
              0 1px 0 rgba(255, 255, 255, 0.8),
              0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.3 * var(--vibrancy-factor, 1.0)))`
          : `inset 0 calc(4px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.7),
              0 1px 0 rgba(255, 255, 255, 0.08),
              0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.3 * var(--vibrancy-factor, 1.0)))`,
        '& .MuiOutlinedInput-notchedOutline': {
          borderColor: theme.palette.primary.main,
          borderWidth: '1px',
        },
      },
    },
  }
}

// 3D Segmented Control Container Styling Helper
export const get3DSegmentedContainerStyle = (isLight: boolean) => ({
  boxShadow: isLight
    ? `inset 0 calc(2.5px * var(--depth-factor, 1.0)) calc(4.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.25),
       0 1px 0 rgba(255, 255, 255, 0.8)`
    : `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.75),
       0 1px 0 rgba(255, 255, 255, 0.08)`,
  border: '1px solid',
  borderColor: isLight ? 'rgba(0, 0, 0, 0.18)' : 'rgba(255, 255, 255, 0.12)',
  backgroundColor: isLight ? 'rgba(0, 0, 0, 0.03)' : 'rgba(0, 0, 0, 0.22)',
})

// 3D Segmented Active Block Styling Helper
export const get3DSegmentedActiveStyle = (theme: any) => {
  const isLight = theme.palette.mode === 'light'
  return {
    position: 'absolute' as const,
    left: '1px',
    right: '1px',
    top: '1px',
    bottom: '1px',
    borderRadius: '3px',
    background: isLight
      ? `linear-gradient(to bottom, #ffffff 0%, #dbdbdb 100%)`
      : `linear-gradient(to bottom, ${theme.palette.primary.light} 0%, ${theme.palette.primary.main} 100%)`,
    boxShadow: isLight
      ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(0,0,0,0.22),
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 rgba(0,0,0,0.2),
         0 calc(2.5px * var(--depth-factor, 1.0)) calc(4.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.25),
         inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.85)`
      : `0 calc(1.5px * var(--depth-factor, 1.0)) 0 0 rgba(0,0,0,0.4),
         0 calc(2.5px * var(--depth-factor, 1.0)) 0 0 rgba(0,0,0,0.45),
         0 calc(3px * var(--depth-factor, 1.0)) calc(5.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.5),
         inset 0 calc(1.2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.35)`,
    border: '1px solid',
    borderColor: isLight ? 'rgba(0, 0, 0, 0.15)' : 'rgba(255, 255, 255, 0.15)',
  }
}
