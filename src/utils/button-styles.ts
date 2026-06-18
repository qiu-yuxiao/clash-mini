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
      textColor = '#2C1F03'
      borderColor = isLight ? '#8A6D00' : '#634E00'
      bevelShadowDark = isLight ? '#8A6D00' : '#5D4037'
      backgroundStyle =
        'linear-gradient(135deg, #FFE082 0%, #FFD54F 20%, #FFC107 45%, #FFB300 70%, #D4AF37 85%, #B8860B 100%)'
    } else if (colorType === 'error') {
      textColor = '#ffffff'
      borderColor = isLight ? '#B71C1C' : '#6B0505'
      bevelShadowDark = isLight ? '#8B0000' : '#4A0404'
      backgroundStyle =
        'linear-gradient(135deg, #FF8A80 0%, #FF5252 30%, #D50000 75%, #880E4F 100%)'
    } else {
      textColor = isLight
        ? '#3C2F0F'
        : variant === 'outlined'
          ? '#FFE082'
          : '#2C1F03'
      borderColor = isLight ? '#D4AF37' : '#8A6D00'
      bevelShadowDark = isLight ? '#B8860B' : '#755500'
      backgroundStyle = isLight
        ? 'linear-gradient(135deg, #FFFDE7 0%, #FFF9C4 35%, #FFF59D 70%, #FFE082 100%)'
        : 'linear-gradient(135deg, #FFF59D 0%, #FBC02D 35%, #F57F17 70%, #E65100 100%)'
    }

    if (variant === 'outlined' && colorType === 'default') {
      backgroundStyle = isLight
        ? 'linear-gradient(to bottom, rgba(255, 253, 231, 0.4) 0%, rgba(255, 245, 157, 0.2) 100%)'
        : 'linear-gradient(to bottom, rgba(42, 36, 19, 0.4) 0%, rgba(23, 20, 11, 0.2) 100%)'
    }

    const glowColor =
      colorType === 'error'
        ? 'rgba(213, 0, 0, calc(0.15 * var(--vibrancy-factor, 1.0)))'
        : 'rgba(255, 193, 7, calc(0.2 * var(--vibrancy-factor, 1.0)))'

    const glowColorHover =
      colorType === 'error'
        ? 'rgba(213, 0, 0, calc(0.35 * var(--vibrancy-factor, 1.0)))'
        : 'rgba(255, 193, 7, calc(0.4 * var(--vibrancy-factor, 1.0)))'

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
      textShadow:
        colorType === 'error'
          ? '0 -1px 0 rgba(0, 0, 0, 0.4)'
          : '0 1px 0 rgba(255, 255, 255, 0.45)',
      boxShadow: isLight
        ? `0 1px 0 0 ${bevelShadowDark},
           0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.35),
           0 1px 0 rgba(255, 255, 255, 0.8),
           0 0 calc(8px * var(--vibrancy-factor, 1.0)) ${glowColor}`
        : `0 1px 0 0 ${bevelShadowDark},
           0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.6),
           0 1px 0 rgba(255, 255, 255, 0.08),
           0 0 calc(8px * var(--vibrancy-factor, 1.0)) ${glowColor}`,
      '&:hover': {
        transform: 'translateY(-2px)',
        background:
          colorType === 'primary'
            ? 'linear-gradient(135deg, #FFF59D 0%, #FFE082 20%, #FFD54F 45%, #FFC107 70%, #FFB300 85%, #D4AF37 100%)'
            : colorType === 'error'
              ? 'linear-gradient(135deg, #FFAB91 0%, #FF8A80 30%, #FF5252 75%, #C62828 100%)'
              : isLight
                ? 'linear-gradient(135deg, #FFFDE7 0%, #FFF9C4 20%, #FFF59D 50%, #FFD54F 100%)'
                : 'linear-gradient(135deg, #FFF9C4 0%, #FFF59D 20%, #FBC02D 50%, #F57F17 100%)',
        boxShadow: isLight
          ? `0 1px 0 0 ${bevelShadowDark},
             0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
             0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
             0 calc(3px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.4),
             0 calc(4px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.2),
             0 1px 0 rgba(255, 255, 255, 0.8),
             0 0 calc(12px * var(--vibrancy-factor, 1.0)) ${glowColorHover}`
          : `0 1px 0 0 ${bevelShadowDark},
             0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
             0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
             0 calc(3px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.7),
             0 calc(4px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.3),
             0 1px 0 rgba(255, 255, 255, 0.08),
             0 0 calc(12px * var(--vibrancy-factor, 1.0)) ${glowColorHover}`,
      },
      '&:active': {
        transform: 'translateY(3px)',
        boxShadow: isLight
          ? `inset 0 1px 0 0 ${bevelShadowDark},
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
             0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.5)`
          : `inset 0 1px 0 0 ${bevelShadowDark},
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
             0 calc(1px * var(--depth-factor, 1.0)) 2px 0 rgba(0, 0, 0, 0.7)`,
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
      borderRadius: 'calc(6px * var(--control-skin-val1, 1.0))',
      transition: 'all 0.15s ease',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      boxShadow:
        '0 calc(2px * var(--control-skin-val2, 1.0)) calc(4px * var(--control-skin-val2, 1.0)) rgba(0, 0, 0, calc(var(--theme-shadow-base-opacity-card, 0.06) * var(--control-skin-val2, 1.0)))',
      '&:hover': {
        background: hoverBg,
        borderColor: hoverBg,
        boxShadow:
          '0 calc(3px * var(--control-skin-val2, 1.0)) calc(6px * var(--control-skin-val2, 1.0)) rgba(0, 0, 0, calc(var(--theme-shadow-base-opacity-card, 0.06) * 2 * var(--control-skin-val2, 1.0)))',
      },
      '&:active': {
        transform: 'translateY(1px)',
        boxShadow:
          '0 calc(1px * var(--control-skin-val2, 1.0)) calc(2px * var(--control-skin-val2, 1.0)) rgba(0, 0, 0, calc(var(--theme-shadow-base-opacity-card, 0.06) * var(--control-skin-val2, 1.0)))',
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

    const bg = isLight
      ? '#ffffff'
      : colorType === 'primary'
        ? '#0a0e17'
        : colorType === 'error'
          ? '#1c0a0a'
          : '#0d1117'

    const hoverBg = isLight
      ? colorType === 'primary'
        ? '#e0f7fa'
        : colorType === 'error'
          ? '#ffebee'
          : '#e8f5e9'
      : colorType === 'primary'
        ? '#101726'
        : colorType === 'error'
          ? '#2c1010'
          : '#161b22'

    const textClr = isLight
      ? colorType === 'primary'
        ? '#006064'
        : colorType === 'error'
          ? '#c2185b'
          : '#1b5e20'
      : neonColor

    const borderClr = isLight
      ? colorType === 'primary'
        ? '#00bcd4'
        : colorType === 'error'
          ? '#ff0055'
          : '#4caf50'
      : neonColor

    return {
      fontFamily: 'Consolas, NSimSun, monospace',
      textTransform: 'none' as const,
      fontWeight: 'bold',
      borderRadius: '0px',
      transition: 'all 0.15s ease',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      boxShadow: `0 0 calc(5px * var(--depth-factor, 1.0)) ${borderClr}`,
      '&:hover': {
        background: hoverBg,
        boxShadow: `0 0 calc(10px * var(--depth-factor, 1.0)) ${borderClr}`,
        transform: 'translateY(-1px)',
      },
      '&:active': {
        transform: 'translateY(1px)',
        boxShadow: `0 0 calc(2px * var(--depth-factor, 1.0)) ${borderClr}`,
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
      textColor = '#2C1F03'
      borderColor = isLight ? '#8A6D00' : '#634E00'
      backgroundStyle =
        'linear-gradient(135deg, #FFE082 0%, #FFD54F 20%, #FFC107 45%, #FFB300 70%, #D4AF37 85%, #B8860B 100%)'
      bevelShadowDark = isLight ? '#8A6D00' : '#5D4037'
    } else if (cardType === 'upload') {
      textColor = isLight ? '#5D4037' : '#FFD54F'
      borderColor = isLight ? '#B38F1E' : '#705407'
      backgroundStyle = isLight
        ? 'linear-gradient(135deg, #FFE0B2 0%, #FFCC80 35%, #FFA726 70%, #FB8C00 100%)'
        : 'linear-gradient(135deg, #E65100 0%, #D84315 40%, #BF360C 75%, #800000 100%)'
      bevelShadowDark = isLight ? '#B38F1E' : '#705407'
    } else if (cardType === 'download') {
      textColor = isLight ? '#37474F' : '#90A4AE'
      borderColor = isLight ? '#78909C' : '#37474F'
      backgroundStyle = isLight
        ? 'linear-gradient(135deg, #ECEFF1 0%, #CFD8DC 35%, #90A4AE 70%, #78909C 100%)'
        : 'linear-gradient(135deg, #263238 0%, #37474F 40%, #455A64 75%, #1C252C 100%)'
      bevelShadowDark = isLight ? '#78909C' : '#263238'
    } else {
      textColor = isLight ? '#3C2F0F' : '#FFE082'
      borderColor = isLight ? '#D4AF37' : '#8A6D00'
      backgroundStyle = isLight
        ? 'linear-gradient(135deg, #FFFDE7 0%, #FFF9C4 35%, #FFF59D 70%, #FFE082 100%)'
        : 'linear-gradient(135deg, #FFF59D 0%, #FBC02D 35%, #F57F17 70%, #E65100 100%)'
      bevelShadowDark = isLight ? '#B8860B' : '#755500'
    }

    const glowColor =
      cardType === 'primary' || cardType === 'upload' || cardType === 'download'
        ? `, 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.2 * var(--vibrancy-factor, 1.0)))`
        : ''

    return {
      borderRadius: '6px',
      border: 'none',
      borderColor,
      background: backgroundStyle,
      color: textColor,
      transition: 'all 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
      boxShadow: isLight
        ? `0 1px 0 0 ${bevelShadowDark},
           0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.35),
           0 1px 0 rgba(255, 255, 255, 0.8)${glowColor}`
        : `0 1px 0 0 ${bevelShadowDark},
           0 calc(1px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 ${bevelShadowDark},
           0 calc(3px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.6),
           0 1px 0 rgba(255, 255, 255, 0.08)${glowColor}`,
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
      borderRadius: 'calc(8px * var(--control-skin-val1, 1.0))',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      transition: 'none',
      boxShadow:
        '0 calc(2px * var(--control-skin-val2, 1.0)) calc(8px * var(--control-skin-val2, 1.0)) rgba(0, 0, 0, calc(var(--theme-shadow-base-opacity-card, 0.06) * var(--control-skin-val2, 1.0)))',
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
    const bg = isLight
      ? cardType === 'primary'
        ? '#f8fafc'
        : '#ffffff'
      : cardType === 'primary'
        ? '#0a0e17'
        : '#05070c'

    const textClr = isLight
      ? cardType === 'upload'
        ? '#b78103'
        : cardType === 'download'
          ? '#0066cc'
          : '#0f172a'
      : neonColor

    const borderClr = isLight
      ? cardType === 'upload'
        ? '#f59e0b'
        : cardType === 'download'
          ? '#3b82f6'
          : '#00b0ff'
      : neonColor

    return {
      fontFamily: 'Consolas, NSimSun, monospace',
      borderRadius: '0px',
      border: '1px solid ' + borderClr,
      background: bg,
      color: textClr,
      transition: 'none',
      boxShadow: isLight
        ? `0 0 calc(10px * var(--depth-factor, 1.0)) rgba(0, 176, 255, 0.15)`
        : `0 0 calc(10px * var(--depth-factor, 1.0)) rgba(0, 240, 255, 0.2)`,
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
        backgroundColor: isLight
          ? 'rgba(255, 253, 231, 0.8)'
          : 'rgba(26, 22, 14, 0.4)',
        boxShadow: isLight
          ? `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.2),
             0 1px 0 rgba(255, 255, 255, 0.8)`
          : `inset 0 calc(4px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.6),
             0 1px 0 rgba(255, 255, 255, 0.08)`,
        transition: 'box-shadow 0.2s ease, border-color 0.2s ease',
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: 'calc(2px * var(--depth-factor, 1.0))',
          borderColor: isLight
            ? 'rgba(212, 175, 55, 0.35)'
            : 'rgba(212, 175, 55, 0.25)',
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderWidth: 'calc(2px * var(--depth-factor, 1.0))',
          borderColor: isLight
            ? 'rgba(212, 175, 55, 0.6)'
            : 'rgba(212, 175, 55, 0.5)',
        },
        '&.Mui-focused': {
          boxShadow: isLight
            ? `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.25),
                0 1px 0 rgba(255, 255, 255, 0.8),
                0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.35 * var(--vibrancy-factor, 1.0)))`
            : `inset 0 calc(4px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.65),
                0 1px 0 rgba(255, 255, 255, 0.08),
                0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.35 * var(--vibrancy-factor, 1.0)))`,
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: isLight ? '#B8860B' : '#FFA000',
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
        borderRadius: 'calc(6px * var(--control-skin-val1, 1.0))',
        boxShadow:
          'inset 0 calc(1px * var(--control-skin-val2, 1.0)) calc(3px * var(--control-skin-val2, 1.0)) rgba(0, 0, 0, calc(var(--theme-shadow-base-opacity-card, 0.06) * var(--control-skin-val2, 1.0)))',
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: '1px',
          borderColor: borderClr,
          borderRadius: 'calc(6px * var(--control-skin-val1, 1.0))',
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderColor: primaryMain,
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
    const bg = isLight ? '#ffffff' : '#030508'
    const textClr = isLight ? '#0f172a' : '#ff0055'
    const borderClr = isLight ? '#cbd5e1' : neonColor
    const hoverBorderClr = isLight ? '#ff0055' : '#00f0ff'

    return {
      '& .MuiOutlinedInput-root': {
        fontFamily: 'Consolas, NSimSun, monospace',
        backgroundColor: bg,
        color: textClr,
        boxShadow: isLight
          ? `inset 0 1px 3px rgba(0, 0, 0, 0.05)`
          : `0 0 calc(4px * var(--depth-factor, 1.0)) rgba(255, 0, 85, 0.15)`,
        '& .MuiOutlinedInput-notchedOutline': {
          borderWidth: '1px',
          borderColor: borderClr,
        },
        '&:hover .MuiOutlinedInput-notchedOutline': {
          borderColor: hoverBorderClr,
        },
        '&.Mui-focused': {
          boxShadow: isLight
            ? `0 0 0 3px rgba(255, 0, 85, 0.15)`
            : `0 0 calc(8px * var(--depth-factor, 1.0)) rgba(255, 0, 85, 0.35)`,
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
        ? `inset 0 calc(2.5px * var(--depth-factor, 1.0)) calc(4.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.2),
           0 1px 0 rgba(255, 255, 255, 0.8)`
        : `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.7),
           0 1px 0 rgba(255, 255, 255, 0.08)`,
      border: '1px solid',
      borderColor: isLight
        ? 'rgba(212, 175, 55, 0.35)'
        : 'rgba(212, 175, 55, 0.22)',
      backgroundColor: isLight
        ? 'rgba(212, 175, 55, 0.06)'
        : 'rgba(212, 175, 55, 0.08)',
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
      border: '1px solid ' + (isLight ? '#00b0ff' : '#39ff14'),
      background: isLight ? '#ffffff' : '#05070c',
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
        'linear-gradient(135deg, #FFE082 0%, #FFD54F 20%, #FFC107 45%, #FFB300 70%, #D4AF37 85%, #B8860B 100%)',
      boxShadow: isLight
        ? `0 calc(1px * var(--depth-factor, 1.0)) 0 0 #8A6D00,
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 #8A6D00,
           0 calc(2px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.35),
           inset 0 calc(1.5px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.7),
           inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.3),
           0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.2 * var(--vibrancy-factor, 1.0)))`
        : `0 calc(1px * var(--depth-factor, 1.0)) 0 0 #5D4037,
           0 calc(2px * var(--depth-factor, 1.0)) 0 0 #5D4037,
           0 calc(2px * var(--depth-factor, 1.0)) 1px 0 rgba(0, 0, 0, 0.55),
           inset 0 calc(1.5px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.5),
           inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) 0 rgba(255, 255, 255, 0.2),
           0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.25 * var(--vibrancy-factor, 1.0)))`,
      border: '1px solid',
      borderColor: isLight ? '#8A6D00' : '#634E00',
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
      borderRadius: 'calc(4px * var(--control-skin-val1, 1.0))',
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
      background: isLight ? '#00b0ff' : '#39ff14',
      border: 'none',
      boxShadow: isLight
        ? `0 0 calc(8px * var(--depth-factor, 1.0)) #00b0ff`
        : `0 0 calc(8px * var(--depth-factor, 1.0)) #39ff14`,
      color: isLight ? '#ffffff' : '#000000',
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

export const get3DSegmentedActiveTextColor = (theme: any): string => {
  const skin = getActiveSkin(theme)
  const isLight = theme.palette.mode === 'light'
  switch (skin) {
    case 'retro-3d':
      return '#2C1F03'
    case 'cyberpunk':
      return isLight ? '#ffffff' : '#000000'
    case 'monochrome':
      return isLight ? '#ffffff' : '#000000'
    case 'frosted-glass':
      return isLight ? 'rgba(0, 0, 0, 0.85)' : '#ffffff'
    case 'original':
    case 'modern-flat':
    default:
      return '#ffffff'
  }
}
