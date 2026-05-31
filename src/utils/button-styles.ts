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
  let bevelShadowDark: string

  if (colorType === 'primary') {
    textColor = '#ffffff'
    borderColor = isLight
      ? alpha(theme.palette.primary.dark, 0.25)
      : 'rgba(255, 255, 255, 0.15)'
    backgroundStyle = isLight
      ? `linear-gradient(to bottom, ${theme.palette.primary.light} 0%, ${theme.palette.primary.main} 100%)`
      : `linear-gradient(to bottom, ${theme.palette.primary.light} 0%, ${theme.palette.primary.dark} 100%)`
    bevelShadowDark = isLight
      ? alpha(theme.palette.primary.dark, 0.7)
      : alpha(theme.palette.primary.dark, 0.9)
  } else if (colorType === 'error') {
    textColor = '#ffffff'
    borderColor = 'rgba(0, 0, 0, 0.2)'
    backgroundStyle = isLight
      ? 'linear-gradient(to bottom, #ff7875 0%, #ff4d4f 100%)'
      : `linear-gradient(to bottom, #ff7875 0%, ${theme.palette.error.dark} 100%)`
    bevelShadowDark = isLight ? '#d9363e' : '#5c000e'
  } else {
    textColor = isLight ? '#333333' : theme.palette.text.primary
    borderColor = isLight ? 'rgba(0, 0, 0, 0.15)' : 'rgba(255, 255, 255, 0.15)'
    backgroundStyle = isLight
      ? 'linear-gradient(to bottom, #ffffff 0%, #dbdbdb 100%)'
      : 'linear-gradient(to bottom, #3b4561 0%, #171c2b 100%)'
    bevelShadowDark = isLight ? '#bdbdbd' : '#0e1118'
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
      ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.25),
         inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.75),
         inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.2)${
           colorType === 'primary' || colorType === 'error'
             ? `, 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.25 * var(--vibrancy-factor, 1.0)))`
             : ''
         }`
      : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(5px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45),
         inset calc(1.2px * var(--depth-factor, 1.0)) calc(1.2px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.35),
         inset calc(-1.2px * var(--depth-factor, 1.0)) calc(-1.2px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45)${
           colorType === 'primary' || colorType === 'error'
             ? `, 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.25 * var(--vibrancy-factor, 1.0)))`
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
           0 calc(7px * var(--depth-factor, 1.0)) calc(14px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.2),
           inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.85),
           inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.15)${
             colorType === 'primary' ||
             colorType === 'error' ||
             colorType === 'default'
               ? `, 0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`
        : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(5px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(6px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(7px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(7px * var(--depth-factor, 1.0)) calc(14px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.4),
           inset calc(1.2px * var(--depth-factor, 1.0)) calc(1.2px * var(--depth-factor, 1.0)) 0 0 rgba(255, 255, 255, 0.45),
           inset calc(-1.2px * var(--depth-factor, 1.0)) calc(-1.2px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.35)${
             colorType === 'primary' ||
             colorType === 'error' ||
             colorType === 'default'
               ? `, 0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`,
    },
    '&:active': {
      transform: 'translateY(4px)',
      boxShadow: isLight
        ? `0 0 0 0 transparent,
           inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.4)`
        : `0 0 0 0 transparent,
           inset 0 calc(4px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.65)`,
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

