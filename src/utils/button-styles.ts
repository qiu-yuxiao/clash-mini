import { Theme } from '@mui/material'

// Centralized Skin Getter Helper
const getActiveSkin = (theme: any): string => {
  if (typeof window !== 'undefined') {
    return localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
  }
  if (theme && theme.controlSkin) {
    return theme.controlSkin
  }
  return 'retro-3d'
}

// 3D Bevel Button Styling Helper
export const get3DButtonStyle = (
  theme: Theme,
  variant: 'contained' | 'outlined',
  colorType: 'primary' | 'error' | 'default' = 'default',
): any => {
  const skin = getActiveSkin(theme)
  const isLight = theme.palette.mode === 'light'

  // ==========================================
  // RETRO 3D (Default Style)
  // ==========================================
  if (skin === 'retro-3d') {
    let backgroundStyle: string
    let textColor: string
    let borderColor: string
    let bevelShadowDark: string

    if (colorType === 'primary') {
      textColor = '#1E1200'
      borderColor = isLight ? '#9E670B' : '#6E4302'
      backgroundStyle =
        'radial-gradient(circle at center, #FFD54F 0%, #FFA000 55%, #F57C00 80%, #E65100 100%)'
      bevelShadowDark = isLight ? '#9E670B' : '#6E4302'
    } else if (colorType === 'error') {
      textColor = '#ffffff'
      borderColor = isLight ? '#B71C1C' : '#6B0505'
      backgroundStyle =
        'radial-gradient(circle at center, #FF5252 0%, #E53935 45%, #C62828 80%, #8E24AA 100%)'
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
      fontFamily: 'Trebuchet MS, SimHei, sans-serif',
      textTransform: 'none' as const,
      fontWeight: 'bold',
      borderRadius: '6px',
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
        transform: 'translateY(2px)',
        boxShadow: isLight
          ? `0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.5),
             inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.45)`
          : `0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.7),
             inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) 0 rgba(0, 0, 0, 0.65)`,
      },
    }
  }

  // ==========================================
  // ORIGINAL
  // ==========================================
  if (skin === 'original') {
    const bg =
      colorType === 'primary'
        ? 'var(--primary-main)'
        : colorType === 'error'
          ? theme.palette.error.main
          : isLight
            ? '#ffffff'
            : '#2e303d'

    const hoverBg =
      colorType === 'primary'
        ? 'var(--primary-main)'
        : colorType === 'error'
          ? theme.palette.error.dark
          : isLight
            ? '#f5f5f5'
            : '#242530'

    const textClr =
      colorType === 'primary' || colorType === 'error'
        ? '#ffffff'
        : theme.palette.text.primary

    const borderClr =
      colorType === 'primary'
        ? 'var(--primary-main)'
        : colorType === 'error'
          ? theme.palette.error.main
          : 'var(--theme-border)'

    return {
      fontFamily: 'Segoe UI, Microsoft YaHei, sans-serif',
      textTransform: 'none' as const,
      fontWeight: 'bold',
      borderRadius: 'calc(4px * var(--control-skin-val1, 1.0))',
      transition: 'all 0.15s ease',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      boxShadow: 'none',
      '&:hover': {
        background: hoverBg,
        borderColor: borderClr,
        filter: colorType === 'primary' ? 'brightness(0.95)' : 'none',
      },
      '&:active': {
        transform: 'translateY(1px)',
        boxShadow: 'none',
      },
    }
  }

  // ==========================================
  // MODERN FLAT
  // ==========================================
  if (skin === 'modern-flat') {
    const bg =
      colorType === 'primary'
        ? theme.palette.primary.main
        : colorType === 'error'
          ? theme.palette.error.main
          : isLight
            ? '#f3f4f6'
            : '#1f2937'

    const hoverBg =
      colorType === 'primary'
        ? theme.palette.primary.dark
        : colorType === 'error'
          ? theme.palette.error.dark
          : isLight
            ? '#e5e7eb'
            : '#374151'

    const borderClr =
      colorType === 'primary'
        ? theme.palette.primary.main
        : colorType === 'error'
          ? theme.palette.error.main
          : isLight
            ? '#d1d5db'
            : '#4b5563'

    const textClr =
      colorType === 'primary' || colorType === 'error'
        ? '#ffffff'
        : theme.palette.text.primary

    return {
      fontFamily: 'Outfit, DengXian, sans-serif',
      textTransform: 'none' as const,
      fontWeight: 'bold',
      borderRadius: '4px',
      transition: 'all 0.15s ease',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      boxShadow: '0 1px 2px rgba(0,0,0,0.05)',
      '&:hover': {
        background: hoverBg,
        borderColor: borderClr,
      },
      '&:active': {
        transform: 'none',
        boxShadow: 'none',
      },
    }
  }

  // ==========================================
  // FROSTED GLASS
  // ==========================================
  if (skin === 'frosted-glass') {
    const bg =
      colorType === 'primary'
        ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.25)'
        : colorType === 'error'
          ? 'rgba(239, 68, 68, 0.25)'
          : 'rgba(255, 255, 255, 0.08)'

    const hoverBg =
      colorType === 'primary'
        ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.4)'
        : colorType === 'error'
          ? 'rgba(239, 68, 68, 0.4)'
          : 'rgba(255, 255, 255, 0.15)'

    return {
      fontFamily: 'Segoe UI Light, Microsoft YaHei Light, sans-serif',
      textTransform: 'none' as const,
      fontWeight: 'normal',
      borderRadius: '8px',
      transition: 'all 0.15s ease',
      border: '1px solid rgba(255, 255, 255, 0.12)',
      background: bg,
      color: theme.palette.text.primary,
      backdropFilter: 'blur(calc(8px * var(--vibrancy-factor, 1.0)))',
      boxShadow: '0 4px 12px 0 rgba(0, 0, 0, 0.08)',
      '&:hover': {
        background: hoverBg,
        transform: 'translateY(-1px)',
        boxShadow: '0 6px 16px 0 rgba(0, 0, 0, 0.12)',
      },
      '&:active': {
        transform: 'translateY(1px)',
        boxShadow: '0 2px 4px 0 rgba(0, 0, 0, 0.04)',
      },
    }
  }

  // ==========================================
  // CYBERPUNK
  // ==========================================
  if (skin === 'cyberpunk') {
    const neonColor =
      colorType === 'primary'
        ? '#00f0ff'
        : colorType === 'error'
          ? '#ff0055'
          : '#39ff14'

    const bg =
      colorType === 'primary'
        ? '#0a0e17'
        : colorType === 'error'
          ? '#1c0a0a'
          : '#0d1117'
    const hoverBg =
      colorType === 'primary'
        ? '#101726'
        : colorType === 'error'
          ? '#2c1010'
          : '#161b22'

    return {
      fontFamily: 'Consolas, NSimSun, monospace',
      textTransform: 'none' as const,
      fontWeight: 'bold',
      borderRadius: '0px',
      transition: 'all 0.15s ease',
      border: '1px solid',
      borderColor: neonColor,
      background: bg,
      color: neonColor,
      boxShadow: `0 0 calc(5px * var(--depth-factor, 1.0)) ${neonColor}`,
      '&:hover': {
        background: hoverBg,
        boxShadow: `0 0 calc(10px * var(--depth-factor, 1.0)) ${neonColor}`,
        transform: 'translateY(-1px)',
      },
      '&:active': {
        transform: 'translateY(1px)',
        boxShadow: `0 0 calc(2px * var(--depth-factor, 1.0)) ${neonColor}`,
      },
    }
  }

  // ==========================================
  // MONOCHROME
  // ==========================================
  if (skin === 'monochrome') {
    const bg =
      colorType === 'primary'
        ? isLight
          ? '#000000'
          : '#ffffff'
        : colorType === 'error'
          ? isLight
            ? '#333333'
            : '#cccccc'
          : isLight
            ? '#ffffff'
            : '#000000'

    const hoverBg =
      colorType === 'primary'
        ? isLight
          ? '#333333'
          : '#cccccc'
        : isLight
          ? '#cccccc'
          : '#333333'

    const textClr =
      colorType === 'primary'
        ? isLight
          ? '#ffffff'
          : '#000000'
        : isLight
          ? '#000000'
          : '#ffffff'

    return {
      fontFamily: 'Georgia, KaiTi, serif',
      textTransform: 'none' as const,
      fontWeight: 'bold',
      borderRadius: '0px',
      transition: 'all 0.15s ease',
      border:
        'calc(1px * var(--vibrancy-factor, 1.0)) solid ' +
        (isLight ? '#000000' : '#ffffff'),
      background: bg,
      color: textClr,
      boxShadow: 'none',
      '&:hover': {
        background: hoverBg,
        color: textClr,
      },
      '&:active': {
        transform: 'translateY(1px)',
      },
    }
  }

  return {}
}

// 3D Bevel Card Styling Helper
export const get3DCardStyle = (
  theme: any,
  cardType: 'primary' | 'default' | 'upload' | 'download',
  isLightMode?: boolean,
): any => {
  const skin = getActiveSkin(theme)
  const isLight =
    isLightMode !== undefined
      ? isLightMode
      : theme && theme.palette
        ? theme.palette.mode === 'light'
        : true

  // ==========================================
  // RETRO 3D (Default Style)
  // ==========================================
  if (skin === 'retro-3d') {
    let backgroundStyle: string
    let textColor: string
    let borderColor: string
    let bevelShadowDark: string

    if (cardType === 'primary') {
      textColor = '#1E1200'
      borderColor = isLight ? '#9E670B' : '#6E4302'
      backgroundStyle =
        'radial-gradient(circle at center, #FFD54F 0%, #FFA000 55%, #F57C00 80%, #E65100 100%)'
      bevelShadowDark = isLight ? '#9E670B' : '#6E4302'
    } else if (cardType === 'upload') {
      textColor = isLight ? '#7F6000' : '#FFEB3B'
      borderColor = isLight ? '#D4AF37' : '#9C7A14'
      backgroundStyle = isLight
        ? 'radial-gradient(circle at center, #FFFDE7 0%, #FFF59D 45%, #FFD54F 75%, #FFB300 100%)'
        : 'radial-gradient(circle at center, #FFA000 0%, #F57C00 50%, #E65100 80%, #803000 100%)'
      bevelShadowDark = isLight ? '#B38F1E' : '#705407'
    } else if (cardType === 'download') {
      textColor = isLight ? '#004D40' : '#80D8FF'
      borderColor = isLight ? '#0084FF' : '#0D47A1'
      backgroundStyle = isLight
        ? 'radial-gradient(circle at center, #E3F2FD 0%, #90CAF9 45%, #42A5F5 75%, #1E88E5 100%)'
        : 'radial-gradient(circle at center, #00B0FF 0%, #0091EA 50%, #0D47A1 80%, #0A3570 100%)'
      bevelShadowDark = isLight ? '#0066CC' : '#08306B'
    } else {
      textColor = isLight
        ? '#2D3748'
        : theme && theme.palette
          ? theme.palette.text.primary
          : '#ffffff'
      borderColor = isLight ? '#94A3B8' : '#0F172A'
      backgroundStyle = isLight
        ? 'radial-gradient(circle at center, #F8FAFC 0%, #F1F5F9 55%, #CBD5E1 100%)'
        : 'radial-gradient(circle at center, #334155 0%, #1E293B 70%, #0F172A 100%)'
      bevelShadowDark = isLight ? '#94A3B8' : '#0F172A'
    }

    return {
      borderRadius: '6px',
      border: 'none',
      borderColor,
      background: backgroundStyle,
      color: textColor,
      transition: 'all 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
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
             cardType === 'primary' ||
             cardType === 'upload' ||
             cardType === 'download'
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
             cardType === 'primary' ||
             cardType === 'upload' ||
             cardType === 'download'
               ? `, 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb, 91, 92, 157), calc(0.15 * var(--vibrancy-factor, 1.0)))`
               : ''
           }`,
    }
  }

  // ==========================================
  // ORIGINAL
  // ==========================================
  if (skin === 'original') {
    const bg =
      cardType === 'primary'
        ? 'var(--primary-main)'
        : cardType === 'upload'
          ? isLight
            ? 'var(--upload-bg)'
            : 'var(--upload-bg-dark)'
          : cardType === 'download'
            ? isLight
              ? 'var(--download-bg)'
              : 'var(--download-bg-dark)'
            : isLight
              ? '#ffffff'
              : '#1e1f29'

    const borderClr = 'var(--theme-border)'
    const textClr =
      cardType === 'primary'
        ? '#ffffff'
        : theme && theme.palette
          ? theme.palette.text.primary
          : '#ffffff'

    return {
      fontFamily: 'Segoe UI, Microsoft YaHei, sans-serif',
      borderRadius: 'calc(8px * var(--control-skin-val1, 1.0))',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      transition: 'none',
      boxShadow: '0 1px 3px rgba(0,0,0,0.05)',
    }
  }

  // ==========================================
  // MODERN FLAT
  // ==========================================
  if (skin === 'modern-flat') {
    const bg =
      cardType === 'primary'
        ? theme && theme.palette
          ? theme.palette.primary.main
          : '#1976d2'
        : cardType === 'upload'
          ? isLight
            ? '#fffde7'
            : '#ffa000'
          : cardType === 'download'
            ? isLight
              ? '#e3f2fd'
              : '#00b0ff'
            : isLight
              ? '#ffffff'
              : '#1f2937'

    const borderClr = isLight ? '#e5e7eb' : '#374151'
    const textClr =
      cardType === 'primary'
        ? '#ffffff'
        : theme && theme.palette
          ? theme.palette.text.primary
          : '#ffffff'

    return {
      fontFamily: 'Outfit, DengXian, sans-serif',
      borderRadius: '4px',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      transition: 'none',
      boxShadow: '0 1px 3px rgba(0,0,0,0.05)',
    }
  }

  // ==========================================
  // FROSTED GLASS
  // ==========================================
  if (skin === 'frosted-glass') {
    const bg =
      cardType === 'primary'
        ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.15)'
        : 'rgba(255, 255, 255, 0.06)'

    return {
      fontFamily: 'Segoe UI Light, Microsoft YaHei Light, sans-serif',
      borderRadius: '8px',
      border: '1px solid rgba(255, 255, 255, 0.1)',
      background: bg,
      color: theme && theme.palette ? theme.palette.text.primary : '#ffffff',
      backdropFilter: 'blur(calc(12px * var(--vibrancy-factor, 1.0)))',
      transition: 'none',
      boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.06)',
    }
  }

  // ==========================================
  // CYBERPUNK
  // ==========================================
  if (skin === 'cyberpunk') {
    const neonColor = '#00f0ff'
    const bg = cardType === 'primary' ? '#0a0e17' : '#05070c'

    return {
      fontFamily: 'Consolas, NSimSun, monospace',
      borderRadius: '0px',
      border: '1px solid ' + neonColor,
      background: bg,
      color: neonColor,
      transition: 'none',
      boxShadow: `0 0 calc(10px * var(--depth-factor, 1.0)) rgba(0, 240, 255, 0.2)`,
    }
  }

  // ==========================================
  // MONOCHROME
  // ==========================================
  if (skin === 'monochrome') {
    return {
      fontFamily: 'Georgia, KaiTi, serif',
      borderRadius: '0px',
      border:
        'calc(1px * var(--vibrancy-factor, 1.0)) solid ' +
        (isLight ? '#000000' : '#ffffff'),
      background: isLight ? '#ffffff' : '#000000',
      color: isLight ? '#000000' : '#ffffff',
      transition: 'none',
      boxShadow: 'none',
    }
  }

  return {}
}

// 3D Recessed Input Styling Helper
export const get3DInputStyle = (theme: Theme): any => {
  const skin = getActiveSkin(theme)
  const isLight = theme.palette.mode === 'light'
  const primaryMain = theme.palette.primary.main

  if (skin === 'retro-3d') {
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

  if (skin === 'original') {
    const borderClr = isLight ? '#cbd5e1' : '#475569'
    return {
      '& .MuiOutlinedInput-root': {
        fontFamily: 'Segoe UI, Microsoft YaHei, sans-serif',
        backgroundColor: isLight ? '#ffffff' : '#1e1f29',
        color: isLight ? '#1e293b' : '#f1f5f9',
        borderRadius: 'calc(4px * var(--control-skin-val1, 1.0))',
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: '1px',
          borderColor: borderClr,
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderColor: isLight ? '#94a3b8' : '#64748b',
        },
        '&.Mui-focused': {
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: 'var(--primary-main)',
            borderWidth: '1.5px',
          },
        },
      },
    }
  }

  if (skin === 'modern-flat') {
    const borderClr = isLight ? '#d1d5db' : '#4b5563'
    return {
      '& .MuiOutlinedInput-root': {
        fontFamily: 'Outfit, DengXian, sans-serif',
        backgroundColor: isLight ? '#ffffff' : '#111827',
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: '1px',
          borderColor: borderClr,
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderColor: primaryMain,
        },
        '&.Mui-focused': {
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: primaryMain,
            borderWidth: '2px',
          },
        },
      },
    }
  }

  if (skin === 'frosted-glass') {
    return {
      '& .MuiOutlinedInput-root': {
        fontFamily: 'Segoe UI Light, Microsoft YaHei Light, sans-serif',
        backgroundColor: 'rgba(255, 255, 255, 0.05)',
        backdropFilter: 'blur(calc(6px * var(--vibrancy-factor, 1.0)))',
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: '1px',
          borderColor: 'rgba(255, 255, 255, 0.1)',
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderColor: 'rgba(255, 255, 255, 0.25)',
        },
        '&.Mui-focused': {
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: primaryMain,
            borderWidth: '1px',
          },
        },
      },
    }
  }

  if (skin === 'cyberpunk') {
    const neonColor = '#ff0055'
    return {
      '& .MuiOutlinedInput-root': {
        fontFamily: 'Consolas, NSimSun, monospace',
        backgroundColor: '#030508',
        color: '#ff0055',
        boxShadow: `0 0 calc(4px * var(--depth-factor, 1.0)) rgba(255, 0, 85, 0.15)`,
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: '1px',
          borderColor: neonColor,
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderColor: '#00f0ff',
        },
        '&.Mui-focused': {
          boxShadow: `0 0 calc(8px * var(--depth-factor, 1.0)) rgba(255, 0, 85, 0.35)`,
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: '#ff0055',
            borderWidth: '1px',
          },
        },
      },
    }
  }

  if (skin === 'monochrome') {
    return {
      '& .MuiOutlinedInput-root': {
        fontFamily: 'Georgia, KaiTi, serif',
        backgroundColor: isLight ? '#ffffff' : '#000000',
        color: isLight ? '#000000' : '#ffffff',
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: 'calc(1px * var(--vibrancy-factor, 1.0))',
          borderColor: isLight ? '#000000' : '#ffffff',
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderColor: isLight ? '#000000' : '#ffffff',
        },
        '&.Mui-focused': {
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: isLight ? '#000000' : '#ffffff',
            borderWidth: 'calc(2px * var(--vibrancy-factor, 1.0))',
          },
        },
      },
    }
  }

  return {}
}

// 3D Segmented Control Container Styling Helper
export const get3DSegmentedContainerStyle = (themeOrIsLight: any): any => {
  const isLight =
    themeOrIsLight && typeof themeOrIsLight === 'object'
      ? themeOrIsLight.palette.mode === 'light'
      : !!themeOrIsLight

  let skin = 'retro-3d'
  if (themeOrIsLight && typeof themeOrIsLight === 'object') {
    skin = getActiveSkin(themeOrIsLight)
  } else if (typeof window !== 'undefined') {
    skin = localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
  }

  if (skin === 'retro-3d') {
    return {
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
    }
  }

  if (skin === 'original') {
    return {
      border: '1px solid ' + (isLight ? '#cbd5e1' : '#475569'),
      background: isLight ? '#f1f5f9' : '#1e1f29',
      boxShadow: 'none',
      borderRadius: 'calc(4px * var(--control-skin-val1, 1.0))',
    }
  }

  if (skin === 'modern-flat') {
    return {
      border: '1px solid ' + (isLight ? '#d1d5db' : '#4b5563'),
      background: isLight ? '#f3f4f6' : '#111827',
      boxShadow: 'none',
    }
  }

  if (skin === 'frosted-glass') {
    return {
      border: '1px solid rgba(255, 255, 255, 0.1)',
      background: 'rgba(255, 255, 255, 0.04)',
      backdropFilter: 'blur(calc(8px * var(--vibrancy-factor, 1.0)))',
    }
  }

  if (skin === 'cyberpunk') {
    return {
      border: '1px solid #39ff14',
      background: '#05070c',
    }
  }

  if (skin === 'monochrome') {
    return {
      border: '1px solid ' + (isLight ? '#000000' : '#ffffff'),
      background: isLight ? '#ffffff' : '#000000',
    }
  }

  return {}
}

// 3D Segmented Active Block Styling Helper
export const get3DSegmentedActiveStyle = (theme: any): any => {
  const skin = getActiveSkin(theme)
  const isLight = theme.palette.mode === 'light'

  if (skin === 'retro-3d') {
    return {
      position: 'absolute' as const,
      left: '1px',
      right: '1px',
      top: '1px',
      bottom: '1px',
      borderRadius: '3px',
      background:
        'radial-gradient(circle at center, #FFD54F 0%, #FFA000 55%, #F57C00 80%, #E65100 100%)',
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

  if (skin === 'original') {
    return {
      position: 'absolute' as const,
      left: '1px',
      right: '1px',
      top: '1px',
      bottom: '1px',
      borderRadius: 'calc(3px * var(--control-skin-val1, 1.0))',
      background: 'var(--primary-main)',
      border: 'none',
      boxShadow: 'none',
      color: '#ffffff',
    }
  }

  if (skin === 'modern-flat') {
    return {
      position: 'absolute' as const,
      left: '1px',
      right: '1px',
      top: '1px',
      bottom: '1px',
      borderRadius: '2px',
      background: theme.palette.primary.main,
      border: 'none',
      boxShadow: 'none',
      color: '#ffffff',
    }
  }

  if (skin === 'frosted-glass') {
    return {
      position: 'absolute' as const,
      left: '1px',
      right: '1px',
      top: '1px',
      bottom: '1px',
      borderRadius: '4px',
      background: 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.3)',
      border: '1px solid rgba(255, 255, 255, 0.2)',
      boxShadow: 'none',
    }
  }

  if (skin === 'cyberpunk') {
    return {
      position: 'absolute' as const,
      left: '1px',
      right: '1px',
      top: '1px',
      bottom: '1px',
      borderRadius: '0px',
      background: '#39ff14',
      border: 'none',
      boxShadow: `0 0 calc(8px * var(--vibrancy-factor, 1.0)) #39ff14`,
      color: '#000000',
    }
  }

  if (skin === 'monochrome') {
    return {
      position: 'absolute' as const,
      left: '1px',
      right: '1px',
      top: '1px',
      bottom: '1px',
      borderRadius: '0px',
      background: isLight ? '#000000' : '#ffffff',
      border: 'none',
      boxShadow: 'none',
      color: isLight ? '#ffffff' : '#000000',
    }
  }

  return {}
}
