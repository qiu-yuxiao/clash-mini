import type { Theme } from '@mui/material'

import { getActiveSkin } from './skin'

// 3D Segmented Control Container Styling Helper
export const get3DSegmentedContainerStyle = (
  themeOrIsLight: Theme | boolean,
): Record<string, unknown> => {
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
export const get3DSegmentedActiveStyle = (
  theme: Theme,
): Record<string, unknown> => {
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

export const get3DSegmentedActiveTextColor = (theme: Theme): string => {
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
