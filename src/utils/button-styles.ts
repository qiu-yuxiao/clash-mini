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
    textColor = '#1E1200'
    borderColor = isLight ? '#9E670B' : '#6E4302'
    backgroundStyle = 'radial-gradient(circle at center, #FFD54F 0%, #FFA000 55%, #F57C00 80%, #E65100 100%)'
    bevelShadowDark = isLight ? '#9E670B' : '#6E4302'
  } else if (colorType === 'error') {
    textColor = '#ffffff'
    borderColor = isLight ? '#B71C1C' : '#6B0505'
    backgroundStyle = 'radial-gradient(circle at center, #FF5252 0%, #E53935 45%, #C62828 80%, #8E24AA 100%)'
    bevelShadowDark = isLight ? '#B71C1C' : '#6B0505'
  } else {
    textColor = isLight ? '#2D3748' : theme.palette.text.primary
    borderColor = isLight ? '#94A3B8' : '#0F172A'
    backgroundStyle = isLight
      ? 'radial-gradient(circle at center, #F8FAFC 0%, #F1F5F9 55%, #CBD5E1 100%)'
      : 'radial-gradient(circle at center, #334155 0%, #1E293B 70%, #0F172A 100%)'
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
    borderRadius: '6px', // Rounded keycaps
    transition: 'all 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
    border: 'calc(2px * var(--depth-factor, 1.0)) solid',
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
         inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.4),
         inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.2),
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
         inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.3),
         inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.15),
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
           inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.5),
           inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.3),
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
           inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.4),
           inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.2),
           inset -1px -1px 0 rgba(0, 0, 0, 0.2)${
             colorType === 'primary' ||
             colorType === 'error' ||
             colorType === 'default'
               ? `, 0 0 calc(12px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`,
    },
    '&:active': {
      transform: 'translateY(2px)', // Displacement halved (4px -> 2px)
      boxShadow: isLight
        ? `0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.5),
           inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45)`
        : `0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.7),
           inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.65)`,
    },
  }
}

// 3D Bevel Card Styling Helper
export const get3DCardStyle = (
  theme: any,
  cardType: 'primary' | 'default' | 'upload' | 'download',
  isLightMode?: boolean,
) => {
  const isLight = isLightMode !== undefined ? isLightMode : theme.palette.mode === 'light'

  let backgroundStyle: string
  let textColor: string
  let borderColor: string
  let bevelShadowDark: string
  let borderWidth = 'calc(2px * var(--depth-factor, 1.0))'

  if (cardType === 'primary') {
    textColor = '#1E1200'
    borderColor = isLight ? '#9E670B' : '#6E4302'
    backgroundStyle = 'radial-gradient(circle at center, #FFD54F 0%, #FFA000 55%, #F57C00 80%, #E65100 100%)'
    bevelShadowDark = isLight ? '#9E670B' : '#6E4302'
  } else if (cardType === 'upload') {
    textColor = isLight ? '#7F6000' : '#FFEB3B'
    borderColor = isLight ? '#D4AF37' : '#9C7A14'
    backgroundStyle = isLight
      ? 'radial-gradient(circle at center, #FFFDE7 0%, #FFF59D 50%, #FBC02D 80%, #E65100 100%)'
      : 'radial-gradient(circle at center, #F57C00 0%, #E65100 65%, #BF360C 90%, #3E1B00 100%)'
    bevelShadowDark = isLight ? '#B38F1E' : '#705407'
    borderWidth = 'calc(1px * var(--depth-factor, 1.0))'
  } else if (cardType === 'download') {
    textColor = isLight ? '#004D40' : '#80D8FF'
    borderColor = isLight ? '#0084FF' : '#0D47A1'
    backgroundStyle = isLight
      ? 'radial-gradient(circle at center, #E0F7FA 0%, #80DEEA 50%, #00ACC1 80%, #006064 100%)'
      : 'radial-gradient(circle at center, #0091EA 0%, #0D47A1 65%, #01579B 90%, #071F4D 100%)'
    bevelShadowDark = isLight ? '#0066CC' : '#08306B'
    borderWidth = 'calc(1px * var(--depth-factor, 1.0))'
  } else {
    textColor = isLight ? '#2D3748' : theme.palette.text.primary
    borderColor = isLight ? '#94A3B8' : '#0F172A'
    backgroundStyle = isLight
      ? 'radial-gradient(circle at center, #F8FAFC 0%, #F1F5F9 55%, #CBD5E1 100%)'
      : 'radial-gradient(circle at center, #334155 0%, #1E293B 70%, #0F172A 100%)'
    bevelShadowDark = isLight ? '#94A3B8' : '#0F172A'
  }

  return {
    borderRadius: '6px',
    border: `${borderWidth} solid`,
    borderColor,
    background: backgroundStyle,
    color: textColor,
    transition: 'all 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
    boxShadow: isLight
      ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.4),
         0 calc(5px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.3),
         inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.4),
         inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.2),
         inset -1px -1px 0 rgba(0, 0, 0, 0.15)`
      : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
         0 calc(3px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.55),
         0 calc(5px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45),
         inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.3),
         inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.15),
         inset -1px -1px 0 rgba(0, 0, 0, 0.25)`,
    '&:hover': {
      transform: 'translateY(-1.5px)',
      boxShadow: isLight
        ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(4px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.35),
           0 calc(7px * var(--depth-factor, 1.0)) calc(12px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.25),
           inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.5),
           inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.3),
           inset -1px -1px 0 rgba(0, 0, 0, 0.1)${
             cardType === 'primary' || cardType === 'upload' || cardType === 'download'
               ? `, 0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.25 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`
        : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(4px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(4px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.45),
           0 calc(7px * var(--depth-factor, 1.0)) calc(12px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.4),
           inset 0 calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.4),
           inset calc(2px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.2),
           inset -1px -1px 0 rgba(0, 0, 0, 0.2)${
             cardType === 'primary' || cardType === 'upload' || cardType === 'download'
               ? `, 0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.25 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`,
    },
  }
}

// 3D Recessed Input Styling Helper
export const get3DInputStyle = (theme: Theme) => {
  const isLight = theme.palette.mode === 'light'
  const primaryMain = theme.palette.primary.main
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
        borderWidth: 'calc(2px * var(--depth-factor, 1.0))',
        borderColor: isLight
          ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.22)'
          : 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.2)',
      },
      '&:hover .MuiOutlinedInput-notchedOutline': {
        borderWidth: 'calc(2px * var(--depth-factor, 1.0))',
        borderColor: isLight
          ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.45)'
          : 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.4)',
      },
      '&.Mui-focused': {
        boxShadow: isLight
          ? `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.3),
              0 1px 0 rgba(255, 255, 255, 0.8),
              0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`
          : `inset 0 calc(4px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.7),
              0 1px 0 rgba(255, 255, 255, 0.08),
              0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.35 * var(--vibrancy-factor, 1.0)))`,
        '& .MuiOutlinedInput-notchedOutline': {
          borderColor: primaryMain,
          borderWidth: 'calc(2px * var(--depth-factor, 1.0))',
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
  borderColor: isLight
    ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.25)'
    : 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.2)',
  backgroundColor: isLight
    ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.05)'
    : 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.08)',
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
    background: 'radial-gradient(circle at center, #FFD54F 0%, #FFA000 55%, #F57C00 80%, #E65100 100%)',
    boxShadow: isLight
      ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 #9E670B,
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 #9E670B,
         0 calc(2px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.35),
         inset 0 calc(1.5px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.45),
         inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.2),
         0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.2 * var(--vibrancy-factor, 1.0)))`
      : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 #6E4302,
         0 calc(2px * var(--depth-factor, 1.0)) 0 0 #6E4302,
         0 calc(2px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.55),
         inset 0 calc(1.5px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.3),
         inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.15),
         0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.25 * var(--vibrancy-factor, 1.0)))`,
    border: '1px solid',
    borderColor: isLight ? '#9E670B' : '#6E4302',
  }
}
