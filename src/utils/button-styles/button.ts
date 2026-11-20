import type { Theme } from '@mui/material'

import { getActiveSkin } from './skin'

// 3D Bevel Button Styling Helper
export const get3DButtonStyle = (
  theme: Theme,
  variant: 'contained' | 'outlined',
  colorType: 'primary' | 'error' | 'default' = 'default',
): Record<string, unknown> => {
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
      '&.Mui-disabled': {
        background: isLight ? '#e0e0e0 !important' : '#32323a !important',
        borderColor: isLight ? '#bdbdbd !important' : '#4a4a52 !important',
        color: isLight ? '#9e9e9e !important' : '#6e6e75 !important',
        boxShadow: 'none !important',
        textShadow: 'none !important',
        transform: 'none !important',
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
      fontWeight: 500,
      borderRadius: 'calc(4px * var(--control-skin-val1, 1.0))',
      transition: 'all 0.15s ease',
      border: '1px solid',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      boxShadow:
        colorType === 'default' ? 'none' : '0 1px 3px rgba(0,0,0,0.15)',
      '&:hover': {
        background: hoverBg,
        borderColor: borderClr,
        filter: colorType === 'primary' ? 'brightness(0.95)' : 'none',
        boxShadow:
          colorType === 'default' ? 'none' : '0 2px 6px rgba(0,0,0,0.2)',
      },
      '&:active': {
        transform: 'translateY(1px)',
        boxShadow:
          colorType === 'default' ? 'none' : '0 0 2px rgba(0,0,0,0.15)',
      },
      '&.Mui-disabled': {
        background: isLight ? '#f5f5f5 !important' : '#1e1e1e !important',
        borderColor: isLight ? '#e0e0e0 !important' : '#333333 !important',
        color: isLight ? '#c0c0c0 !important' : '#555555 !important',
        boxShadow: 'none !important',
        transform: 'none !important',
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
      '&.Mui-disabled': {
        background: isLight ? '#f3f4f6 !important' : '#1f2937 !important',
        borderColor: isLight ? '#e5e7eb !important' : '#374151 !important',
        color: isLight ? '#9ca3af !important' : '#4b5563 !important',
        boxShadow: 'none !important',
        transform: 'none !important',
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
          : isLight
            ? 'rgba(0, 0, 0, 0.06)'
            : 'rgba(255, 255, 255, 0.08)'

    const hoverBg =
      colorType === 'primary'
        ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.4)'
        : colorType === 'error'
          ? 'rgba(239, 68, 68, 0.4)'
          : isLight
            ? 'rgba(0, 0, 0, 0.12)'
            : 'rgba(255, 255, 255, 0.15)'

    return {
      fontFamily: 'Segoe UI Light, Microsoft YaHei Light, sans-serif',
      textTransform: 'none' as const,
      fontWeight: 'normal',
      borderRadius: '8px',
      transition: 'all 0.15s ease',
      border: isLight
        ? '1px solid rgba(0, 0, 0, 0.12)'
        : '1px solid rgba(255, 255, 255, 0.12)',
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
      '&.Mui-disabled': {
        background: isLight
          ? 'rgba(0, 0, 0, 0.03) !important'
          : 'rgba(255, 255, 255, 0.03) !important',
        borderColor: isLight
          ? 'rgba(0, 0, 0, 0.08) !important'
          : 'rgba(255, 255, 255, 0.05) !important',
        color: isLight
          ? 'rgba(0, 0, 0, 0.26) !important'
          : 'rgba(255, 255, 255, 0.25) !important',
        boxShadow: 'none !important',
        backdropFilter: 'none !important',
        transform: 'none !important',
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
      '&.Mui-disabled': {
        background: isLight ? '#f5f5f5 !important' : '#0d1117 !important',
        borderColor: isLight ? '#e0e0e0 !important' : '#21262d !important',
        color: isLight ? '#9e9e9e !important' : '#484f58 !important',
        boxShadow: 'none !important',
        transform: 'none !important',
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
      '&.Mui-disabled': {
        background: isLight ? '#f5f5f5 !important' : '#222222 !important',
        borderColor: isLight ? '#cccccc !important' : '#444444 !important',
        color: isLight ? '#999999 !important' : '#666666 !important',
        boxShadow: 'none !important',
        transform: 'none !important',
      },
    }
  }

  return {}
}
