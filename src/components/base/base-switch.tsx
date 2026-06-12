import { styled } from '@mui/material/styles'
import { default as MuiSwitch, SwitchProps } from '@mui/material/Switch'

export const Switch = styled((props: SwitchProps) => (
  <MuiSwitch
    focusVisibleClassName=".Mui-focusVisible"
    disableRipple
    {...props}
  />
))(({ theme }) => {
  const isLight = theme.palette.mode === 'light'
  const skin =
    (theme as any).controlSkin ||
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
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '300ms',
        '&.Mui-checked': {
          transform: 'translateX(14px)',
          color: '#fff',
          '& + .MuiSwitch-track': {
            background: isLight
              ? `linear-gradient(to bottom, #FFA000 0%, #E65100 100%)`
              : `linear-gradient(to bottom, rgba(255, 160, 0, 0.8) 0%, rgba(230, 81, 0, 0.9) 100%)`,
            opacity: 0.85,
            boxShadow: isLight
              ? `inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0,0,0,0.4),
                 0 1px 0 rgba(255,255,255,0.8),
                 0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.3 * var(--vibrancy-factor, 1.0)))`
              : `inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.8),
                 0 1px 0 rgba(255, 255, 255, 0.08),
                 inset 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, 0.4),
                 0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.4 * var(--vibrancy-factor, 1.0)))`,
          },
          '& .MuiSwitch-thumb': {
            background: isLight
              ? `radial-gradient(circle at 35% 35%, #ffffff 0%, #FFD54F 55%, #FFA000 100%)`
              : `radial-gradient(circle at 35% 35%, #ffffff 0%, #FFA000 55%, #E65100 100%)`,
            border: `1px solid ${isLight ? '#9E670B' : '#6E4302'}`,
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 14,
        height: 14,
        border: `1px solid ${isLight ? 'rgba(255, 160, 0, 0.35)' : 'rgba(255, 160, 0, 0.4)'}`,
        background: isLight
          ? `radial-gradient(circle at 35% 35%, #ffffff 0%, rgba(255, 213, 79, 0.25) 60%, rgba(255, 160, 0, 0.45) 100%)`
          : `radial-gradient(circle at 35% 35%, #ffffff 0%, rgba(255, 160, 0, 0.3) 60%, rgba(230, 81, 0, 0.55) 100%)`,
        boxShadow: isLight
          ? `0 calc(2.5px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.25),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.8),
             inset 0 calc(-1px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.15)`
          : `0 calc(3px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0,0,0,0.5),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.4),
             inset 0 calc(-1.5px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.55)`,
        transition:
          'transform 0.1s ease-out, box-shadow 0.1s ease-out, filter 0.1s ease-out, background 0.1s ease-out',
      },
      '& .MuiSwitch-track': {
        borderRadius: 7,
        backgroundColor: isLight
          ? 'rgba(212, 175, 55, 0.15)'
          : 'rgba(212, 175, 55, 0.08)',
        opacity: 0.85,
        border: `1px solid ${isLight ? 'rgba(212, 175, 55, 0.25)' : 'rgba(212, 175, 55, 0.12)'}`,
        boxShadow: isLight
          ? 'inset 0 calc(2px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.3), 0 1px 0 rgba(255, 255, 255, 0.8)'
          : 'inset 0 calc(3px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.75), 0 1px 0 rgba(255, 255, 255, 0.08)',
      },
      '&:hover .MuiSwitch-thumb': {
        transform: 'scale(1.2)',
        filter: 'brightness(1.15)',
        boxShadow: isLight
          ? `0 calc(4px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.9),
             inset 0 calc(-1px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.1),
             0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.35 * var(--vibrancy-factor, 1.0)))`
          : `0 calc(5px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.65),
             inset 0 calc(1.2px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.5),
             inset 0 calc(-1.5px * var(--depth-factor, 1.0)) 0 rgba(0,0,0,0.45),
             0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 160, 0, calc(0.35 * var(--vibrancy-factor, 1.0)))`,
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
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '200ms',
        '&.Mui-checked': {
          transform: 'translateX(14px)',
          color: '#fff',
          '& + .MuiSwitch-track': {
            backgroundColor: 'var(--primary-main)',
            opacity: 1,
            boxShadow: 'none',
            border: 'none',
          },
          '& .MuiSwitch-thumb': {
            background: '#ffffff',
            border: 'none',
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
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '200ms',
        '&.Mui-checked': {
          transform: 'translateX(14px)',
          color: '#fff',
          '& + .MuiSwitch-track': {
            backgroundColor: theme.palette.primary.main,
            opacity: 1,
            boxShadow: 'none',
            border: 'none',
          },
          '& .MuiSwitch-thumb': {
            background: '#ffffff',
            border: 'none',
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
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '250ms',
        '&.Mui-checked': {
          transform: 'translateX(14px)',
          color: '#fff',
          '& + .MuiSwitch-track': {
            background: 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.35)',
            border: '1px solid rgba(255, 255, 255, 0.15)',
            opacity: 1,
          },
          '& .MuiSwitch-thumb': {
            background: 'rgba(255, 255, 255, 0.9)',
            border: '1px solid rgba(255, 255, 255, 0.3)',
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 12,
        height: 12,
        margin: '1px',
        border: '1px solid rgba(255, 255, 255, 0.2)',
        background: 'rgba(255, 255, 255, 0.3)',
        backdropFilter: 'blur(4px)',
        boxShadow: 'none',
      },
      '& .MuiSwitch-track': {
        borderRadius: 7,
        backgroundColor: 'rgba(255, 255, 255, 0.08)',
        border: '1px solid rgba(255, 255, 255, 0.1)',
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
    return {
      width: 28,
      height: 14,
      padding: 0,
      marginRight: 1,
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '150ms',
        '&.Mui-checked': {
          transform: 'translateX(14px)',
          '& + .MuiSwitch-track': {
            background: '#0a0e17',
            border: '1px solid #00f0ff',
            opacity: 1,
          },
          '& .MuiSwitch-thumb': {
            background: '#00f0ff',
            color: '#00f0ff',
            borderRadius: '0px',
            boxShadow: '0 0 6px #00f0ff',
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 12,
        height: 12,
        margin: '1px',
        borderRadius: '0px',
        background: '#ff0055',
        color: '#ff0055',
        boxShadow: '0 0 4px #ff0055',
      },
      '& .MuiSwitch-track': {
        borderRadius: 0,
        backgroundColor: '#05070c',
        border: '1px solid #ff0055',
        opacity: 1,
      },
      '&:hover .MuiSwitch-thumb': {
        boxShadow: '0 0 10px currentColor',
      },
    }
  }

  // ==========================================
  // MONOCHROME
  // ==========================================
  if (skin === 'monochrome') {
    return {
      width: 28,
      height: 14,
      padding: 0,
      marginRight: 1,
      '& .MuiSwitch-switchBase': {
        padding: 0,
        margin: 0,
        transitionDuration: '100ms',
        '&.Mui-checked': {
          transform: 'translateX(14px) !important',
          '& + .MuiSwitch-track': {
            backgroundColor: (isLight ? '#000000' : '#ffffff') + ' !important',
            border: '1px solid ' + (isLight ? '#000000' : '#ffffff') + ' !important',
            opacity: '1 !important',
          },
          '& .MuiSwitch-thumb': {
            backgroundColor: (isLight ? '#ffffff' : '#000000') + ' !important',
            borderRadius: '0px !important',
            border: 'none !important',
          },
        },
      },
      '& .MuiSwitch-thumb': {
        boxSizing: 'border-box',
        width: 12,
        height: 12,
        margin: '1px',
        borderRadius: '0px',
        backgroundColor: isLight ? '#000000' : '#ffffff',
        border: 'none',
        boxShadow: 'none',
      },
      '& .MuiSwitch-track': {
        borderRadius: 0,
        backgroundColor: isLight ? '#ffffff' : '#000000',
        border: '1px solid ' + (isLight ? '#000000' : '#ffffff'),
        opacity: 1,
      },
    }
  }

  return {}
})
