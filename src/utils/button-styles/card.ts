import type { Theme } from '@mui/material'

import { getActiveSkin } from './skin'

// 3D Bevel Card Styling Helper
export const get3DCardStyle = (
  theme: Theme,
  cardType: 'primary' | 'default' | 'upload' | 'download',
  isLightMode?: boolean,
): Record<string, unknown> => {
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
      textColor = isLight ? '#3C2F0F' : '#2C1F03'
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
      border: 'none',
      borderColor: borderClr,
      background: bg,
      color: textClr,
      transition: 'none',
      boxShadow: '0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)',
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
