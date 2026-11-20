import type { Theme } from '@mui/material'

import { getActiveSkin } from './skin'

// 3D Recessed Input Styling Helper
export const get3DInputStyle = (theme: Theme): Record<string, unknown> => {
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
