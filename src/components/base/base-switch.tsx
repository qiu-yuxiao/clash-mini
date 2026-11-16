import { type CSSObject, styled, type Theme } from '@mui/material/styles'
import { default as MuiSwitch, type SwitchProps } from '@mui/material/Switch'

interface SquareSwitchPalette {
  trackUncheckedBg: string
  trackUncheckedBorder: string
  trackCheckedBg: string
  trackCheckedBorder: string
  trackCheckedShadow?: string
  trackUncheckedShadow?: string
  thumbUncheckedBg: string
  thumbUncheckedBorder?: string
  thumbUncheckedShadow?: string
  thumbCheckedBg: string
  thumbCheckedBorder?: string
  thumbCheckedShadow?: string
  hoverRule: CSSObject
}

function buildSquareSwitchStyle(p: SquareSwitchPalette): CSSObject {
  return {
    width: 28,
    height: 14,
    padding: 0,
    marginRight: 1,
    '&.MuiSwitch-sizeSmall': {
      width: '18px !important',
      height: '10px !important',
      padding: '0 !important',
      '& .MuiSwitch-switchBase': {
        padding: '0 !important',
        margin: '0 !important',
        width: '10px !important',
        height: '10px !important',
        '&.Mui-checked': {
          transform: 'translateX(8px) !important',
        },
        '& .MuiSwitch-input': {
          width: '18px !important',
          height: '10px !important',
          left: '0 !important',
        },
        '&.Mui-checked .MuiSwitch-input': {
          left: '-8px !important',
        },
      },
      '& .MuiSwitch-thumb': {
        width: '8px !important',
        height: '8px !important',
        margin: '1px !important',
      },
    },
    '& .MuiSwitch-switchBase': {
      padding: 0,
      margin: 0,
      transitionDuration: '150ms',
      '&.Mui-checked': {
        transform: 'translateX(14px) !important',
        '& + .MuiSwitch-track': {
          backgroundColor: `${p.trackCheckedBg} !important`,
          border: `1px solid ${p.trackCheckedBorder} !important`,
          opacity: '1 !important',
          ...(p.trackCheckedShadow ? { boxShadow: p.trackCheckedShadow } : {}),
        },
        '& .MuiSwitch-thumb': {
          backgroundColor: `${p.thumbCheckedBg} !important`,
          color: `${p.thumbCheckedBg} !important`,
          borderRadius: '0px !important',
          ...(p.thumbCheckedBorder ? { border: p.thumbCheckedBorder } : {}),
          ...(p.thumbCheckedShadow ? { boxShadow: p.thumbCheckedShadow } : {}),
        },
      },
    },
    '& .MuiSwitch-thumb': {
      boxSizing: 'border-box',
      width: 12,
      height: 12,
      margin: '1px',
      borderRadius: '0px',
      background: p.thumbUncheckedBg,
      color: p.thumbUncheckedBg,
      ...(p.thumbUncheckedBorder ? { border: p.thumbUncheckedBorder } : {}),
      ...(p.thumbUncheckedShadow ? { boxShadow: p.thumbUncheckedShadow } : {}),
    },
    '& .MuiSwitch-track': {
      borderRadius: 0,
      backgroundColor: p.trackUncheckedBg,
      border: `1px solid ${p.trackUncheckedBorder}`,
      opacity: 1,
      ...(p.trackUncheckedShadow ? { boxShadow: p.trackUncheckedShadow } : {}),
    },
    ...p.hoverRule,
  }
}

export function getSwitchSkinStyle(theme: Theme): CSSObject {
  const isLight = theme.palette.mode === 'light'
  const skin =
    theme.controlSkin ||
    (typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d')

  // ==========================================
  // RETRO 3D (Default Style)
  // ==========================================
  if (skin === 'retro-3d') {
    return {
      width: 28,
      height: 14,
      padding: 0,
      marginRight: 1,
      '&.MuiSwitch-sizeSmall': {
        width: '28px !important',
        height: '14px !important',
        padding: '0 !important',
        '& .MuiSwitch-switchBase': {
          padding: '0 !important',
          margin: '0 !important',
          width: '14px !important',
          height: '14px !important',
          '&.Mui-checked': {
            transform: 'translateX(14px) !important',
          },
          '& .MuiSwitch-input': {
            width: '28px !important',
            height: '14px !important',
            left: '0 !important',
          },
          '&.Mui-checked .MuiSwitch-input': {
            left: '-14px !important',
          },
        },
        '& .MuiSwitch-thumb': {
          width: '14px !important',
          height: '14px !important',
        },
      },
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '300ms',
        '&.Mui-checked': {
          transform: 'translateX(14px) !important',
          color: '#fff !important',
          '& + .MuiSwitch-track': {
            background:
              'linear-gradient(to bottom, #FFE082 0%, #FFC107 50%, #B8860B 100%) !important',
            opacity: '0.85 !important',
            boxShadow:
              (isLight
                ? `inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0,0,0,0.3),
                 0 1px 0 rgba(255,255,255,0.8),
                 0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.3 * var(--vibrancy-factor, 1.0)))`
                : `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.75),
                 0 1px 0 rgba(255, 255, 255, 0.08),
                 inset 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, 0.4),
                 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.4 * var(--vibrancy-factor, 1.0)))`) +
              ' !important',
          },
          '& .MuiSwitch-thumb': {
            background:
              (isLight
                ? `radial-gradient(circle at 35% 35%, #FFFDE7 0%, #FFD54F 45%, #FFA000 80%, #B8860B 100%)`
                : `radial-gradient(circle at 35% 35%, #FFF9C4 0%, #FFC107 45%, #FFA000 80%, #8A6D00 100%)`) +
              ' !important',
            border: `1px solid ${isLight ? '#8A6D00' : '#634E00'} !important`,
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 14,
        height: 14,
        border: `1px solid ${isLight ? 'rgba(138, 109, 0, 0.4)' : 'rgba(99, 78, 0, 0.45)'}`,
        background: isLight
          ? `radial-gradient(circle at 35% 35%, #FFFDE7 0%, rgba(212, 175, 55, 0.3) 60%, rgba(138, 109, 0, 0.45) 100%)`
          : `radial-gradient(circle at 35% 35%, #FFF9C4 0%, rgba(138, 109, 0, 0.3) 60%, rgba(99, 78, 0, 0.55) 100%)`,
        boxShadow: isLight
          ? `0 calc(2.5px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.2),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.7),
             inset 0 calc(-1px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.15)`
          : `0 calc(3px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0,0,0,0.5),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.5),
             inset 0 calc(-1.5px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.45)`,
        transition:
          'transform 0.1s ease-out, box-shadow 0.1s ease-out, filter 0.1s ease-out, background 0.1s ease-out',
      },
      '& .MuiSwitch-track': {
        borderRadius: 7,
        backgroundColor: isLight
          ? 'rgba(212, 175, 55, 0.25)'
          : 'rgba(212, 175, 55, 0.15)',
        opacity: 0.85,
        border: `1px solid ${isLight ? 'rgba(212, 175, 55, 0.3)' : 'rgba(212, 175, 55, 0.18)'}`,
        boxShadow: isLight
          ? 'inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.25), 0 1px 0 rgba(255, 255, 255, 0.8)'
          : 'inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.7), 0 1px 0 rgba(255, 255, 255, 0.08)',
      },
      '&:hover .MuiSwitch-thumb': {
        transform: 'scale(1.2)',
        filter: 'brightness(1.15)',
        boxShadow: isLight
          ? `0 calc(4px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) rgba(0,0,0,0.3),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.8),
             inset 0 calc(-1px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.1),
             0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.35 * var(--vibrancy-factor, 1.0)))`
          : `0 calc(5px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.6),
             inset 0 calc(1.2px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.6),
             inset 0 calc(-1.5px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.4),
             0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.35 * var(--vibrancy-factor, 1.0)))`,
      },
    }
  }

  // ==========================================
  // ORIGINAL
  // ==========================================
  if (skin === 'original') {
    return {
      width: 28,
      height: 14,
      padding: 0,
      marginRight: 1,
      '&.MuiSwitch-sizeSmall': {
        width: '28px !important',
        height: '14px !important',
        padding: '0 !important',
        '& .MuiSwitch-switchBase': {
          padding: '0 !important',
          margin: '0 !important',
          width: '14px !important',
          height: '14px !important',
          '&.Mui-checked': {
            transform: 'translateX(14px) !important',
          },
          '& .MuiSwitch-input': {
            width: '28px !important',
            height: '14px !important',
            left: '0 !important',
          },
          '&.Mui-checked .MuiSwitch-input': {
            left: '-14px !important',
          },
        },
        '& .MuiSwitch-thumb': {
          width: '14px !important',
          height: '14px !important',
        },
      },
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '200ms',
        '&.Mui-checked': {
          transform: 'translateX(14px) !important',
          color: '#fff !important',
          '& + .MuiSwitch-track': {
            backgroundColor: 'var(--primary-main) !important',
            opacity: '1 !important',
            boxShadow: 'none !important',
            border: 'none !important',
          },
          '& .MuiSwitch-thumb': {
            backgroundColor: '#ffffff !important',
            border: 'none !important',
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 14,
        height: 14,
        borderRadius: 'calc(7px * var(--control-skin-val1, 1.0))',
        border: 'none',
        background: isLight ? '#cbd5e1' : '#475569',
        boxShadow: 'none',
      },
      '& .MuiSwitch-track': {
        borderRadius: 'calc(7px * var(--control-skin-val1, 1.0))',
        backgroundColor: isLight ? '#e2e8f0' : '#30363d',
        opacity: 1,
        border: 'none',
      },
    }
  }

  // ==========================================
  // MODERN FLAT
  // ==========================================
  if (skin === 'modern-flat') {
    return {
      width: 28,
      height: 14,
      padding: 0,
      marginRight: 1,
      '&.MuiSwitch-sizeSmall': {
        width: '28px !important',
        height: '14px !important',
        padding: '0 !important',
        '& .MuiSwitch-switchBase': {
          padding: '0 !important',
          margin: '0 !important',
          width: '14px !important',
          height: '14px !important',
          '&.Mui-checked': {
            transform: 'translateX(14px) !important',
          },
          '& .MuiSwitch-input': {
            width: '28px !important',
            height: '14px !important',
            left: '0 !important',
          },
          '&.Mui-checked .MuiSwitch-input': {
            left: '-14px !important',
          },
        },
        '& .MuiSwitch-thumb': {
          width: '14px !important',
          height: '14px !important',
        },
      },
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '200ms',
        '&.Mui-checked': {
          transform: 'translateX(14px) !important',
          color: '#fff !important',
          '& + .MuiSwitch-track': {
            backgroundColor: theme.palette.primary.main + ' !important',
            opacity: '1 !important',
            boxShadow: 'none !important',
            border: 'none !important',
          },
          '& .MuiSwitch-thumb': {
            backgroundColor: '#ffffff !important',
            border: 'none !important',
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 14,
        height: 14,
        border: 'none',
        background: isLight ? '#d1d5db' : '#9ca3af',
        boxShadow: '0 1px 2px rgba(0,0,0,0.15)',
      },
      '& .MuiSwitch-track': {
        borderRadius: 7,
        backgroundColor: isLight ? '#e5e7eb' : '#374151',
        opacity: 1,
        border: 'none',
        boxShadow: 'none',
      },
      '&:hover .MuiSwitch-thumb': {
        transform: 'scale(1.1)',
      },
    }
  }

  // ==========================================
  // FROSTED GLASS
  // ==========================================
  if (skin === 'frosted-glass') {
    return {
      width: 28,
      height: 14,
      padding: 0,
      marginRight: 1,
      '&.MuiSwitch-sizeSmall': {
        width: '28px !important',
        height: '14px !important',
        padding: '0 !important',
        '& .MuiSwitch-switchBase': {
          padding: '0 !important',
          margin: '0 !important',
          width: '14px !important',
          height: '14px !important',
          '&.Mui-checked': {
            transform: 'translateX(14px) !important',
          },
          '& .MuiSwitch-input': {
            width: '28px !important',
            height: '14px !important',
            left: '0 !important',
          },
          '&.Mui-checked .MuiSwitch-input': {
            left: '-14px !important',
          },
        },
        '& .MuiSwitch-thumb': {
          width: '12px !important',
          height: '12px !important',
          margin: '1px !important',
        },
      },
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '250ms',
        '&.Mui-checked': {
          transform: 'translateX(14px) !important',
          color: '#fff !important',
          '& + .MuiSwitch-track': {
            backgroundColor:
              (isLight
                ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.5)'
                : 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.35)') +
              ' !important',
            border: `1px solid ${isLight ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.2)' : 'rgba(255, 255, 255, 0.15)'} !important`,
            opacity: '1 !important',
          },
          '& .MuiSwitch-thumb': {
            backgroundColor:
              (isLight
                ? 'rgba(255, 255, 255, 0.95)'
                : 'rgba(255, 255, 255, 0.9)') + ' !important',
            border: `1px solid ${isLight ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.4)' : 'rgba(255, 255, 255, 0.3)'} !important`,
            boxShadow: isLight
              ? '0 1px 3px rgba(0, 0, 0, 0.15) !important'
              : 'none !important',
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 12,
        height: 12,
        margin: '1px',
        border: `1px solid ${isLight ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.3)' : 'rgba(255, 255, 255, 0.2)'}`,
        background: isLight
          ? 'rgba(255, 255, 255, 0.8)'
          : 'rgba(255, 255, 255, 0.3)',
        backdropFilter: 'blur(4px)',
        boxShadow: isLight ? '0 1px 2px rgba(0, 0, 0, 0.1)' : 'none',
      },
      '& .MuiSwitch-track': {
        borderRadius: 7,
        backgroundColor: isLight
          ? 'rgba(0, 0, 0, 0.06)'
          : 'rgba(255, 255, 255, 0.08)',
        border: `1px solid ${isLight ? 'rgba(0, 0, 0, 0.08)' : 'rgba(255, 255, 255, 0.1)'}`,
        opacity: 1,
      },
      '&:hover .MuiSwitch-thumb': {
        transform: 'scale(1.1)',
      },
    }
  }

  // ==========================================
  // CYBERPUNK
  // ==========================================
  if (skin === 'cyberpunk') {
    const checkedAccent = isLight ? '#00b0ff' : '#00f0ff'
    return buildSquareSwitchStyle({
      trackCheckedBg: isLight ? '#e0f7fa' : '#0a0e17',
      trackCheckedBorder: checkedAccent,
      thumbCheckedBg: checkedAccent,
      thumbCheckedShadow: `0 0 6px ${checkedAccent} !important`,
      thumbUncheckedBg: '#ff0055',
      thumbUncheckedShadow: '0 0 4px #ff0055',
      trackUncheckedBg: isLight ? '#f1f5f9' : '#05070c',
      trackUncheckedBorder: '#ff0055',
      hoverRule: {
        '&:hover .MuiSwitch-thumb': {
          boxShadow: '0 0 10px currentColor',
        },
      },
    })
  }

  // ==========================================
  // MONOCHROME (BUG-384)
  // ==========================================
  if (skin === 'monochrome') {
    const activeBorderAndThumb = isLight ? '#000000' : '#ffffff'
    return buildSquareSwitchStyle({
      trackCheckedBg: isLight ? '#d4d4d4' : '#262626',
      trackCheckedBorder: activeBorderAndThumb,
      trackCheckedShadow: 'none !important',
      thumbCheckedBg: activeBorderAndThumb,
      thumbCheckedShadow: 'none !important',
      thumbUncheckedBg: '#737373',
      thumbUncheckedShadow: 'none',
      trackUncheckedBg: isLight ? '#f4f4f5' : '#0a0a0a',
      trackUncheckedBorder: '#737373',
      hoverRule: {
        '&:hover .MuiSwitch-track': {
          borderColor: activeBorderAndThumb,
        },
      },
    })
  }

  return {}
}

export const Switch = styled((props: SwitchProps) => (
  <MuiSwitch
    focusVisibleClassName=".Mui-focusVisible"
    disableRipple
    {...props}
  />
))(({ theme }) => getSwitchSkinStyle(theme))
