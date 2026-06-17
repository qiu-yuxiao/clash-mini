import {
  SignalWifi3Bar as SignalGood,
  SignalWifi2Bar as SignalMedium,
  SignalWifi0Bar as SignalNone,
  SignalWifi4Bar as SignalStrong,
  SignalWifi1Bar as SignalWeak,
  WifiOff as SignalError,
} from '@mui/icons-material'
import { alpha, type Theme } from '@mui/material'

import delayManager from '@/services/delay'
import getSystem from '@/utils/get-system'

const OS = getSystem()

export const portableFlag = false

export function getSignalIcon(delay: number, t: any) {
  if (delay === -2)
    return {
      icon: <SignalNone />,
      text: t('settings.mini.statusTesting', { defaultValue: '测试中' }),
      color: 'text.secondary',
    }
  if (delay === -1)
    return {
      icon: <SignalNone />,
      text: t('settings.mini.statusUntested', { defaultValue: '未测试' }),
      color: 'text.secondary',
    }
  if (delay > 1e5)
    return {
      icon: <SignalError />,
      text: t('settings.mini.statusError', { defaultValue: '错误' }),
      color: 'error.main',
    }
  if (delay === 0 || delay >= 10000)
    return {
      icon: <SignalError />,
      text: t('settings.mini.statusTimeout', { defaultValue: '超时' }),
      color: 'error.main',
    }
  if (delay >= 500)
    return {
      icon: <SignalWeak />,
      text: t('settings.mini.statusDelayHigh', { defaultValue: '延迟较高' }),
      color: 'error.main',
    }
  if (delay >= 300)
    return {
      icon: <SignalMedium />,
      text: t('settings.mini.statusDelayMedium', { defaultValue: '延迟中等' }),
      color: 'warning.main',
    }
  if (delay >= 200)
    return {
      icon: <SignalGood />,
      text: t('settings.mini.statusDelayGood', { defaultValue: '延迟良好' }),
      color: 'info.main',
    }
  return {
    icon: <SignalStrong />,
    text: t('settings.mini.statusDelayExcellent', { defaultValue: '延迟极佳' }),
    color: 'success.main',
  }
}

export function convertDelayColor(
  delayValue: number,
): 'success' | 'warning' | 'error' | 'primary' | 'default' {
  const colorStr = delayManager.formatDelayColor(delayValue)
  if (!colorStr) return 'default'
  const mainColor = colorStr.split('.')[0]
  switch (mainColor) {
    case 'success':
      return 'success'
    case 'warning':
      return 'warning'
    case 'error':
      return 'error'
    case 'primary':
      return 'primary'
    default:
      return 'default'
  }
}

export const getMenuItemHoverStyle = (theme: Theme, skin: string) => {
  const isLight = theme.palette.mode === 'light'

  let fontFamily = 'Trebuchet MS, SimHei, sans-serif'
  if (skin === 'original') fontFamily = 'Segoe UI, Microsoft YaHei, sans-serif'
  else if (skin === 'modern-flat') fontFamily = 'Outfit, DengXian, sans-serif'
  else if (skin === 'frosted-glass')
    fontFamily = 'Segoe UI Light, Microsoft YaHei Light, sans-serif'
  else if (skin === 'cyberpunk') fontFamily = 'Consolas, NSimSun, monospace'
  else if (skin === 'monochrome') fontFamily = 'Georgia, KaiTi, serif'

  const baseStyle = {
    fontFamily,
    fontSize: '13px',
    padding: '8px 16px',
    transition: 'all 0.15s ease',
    color: theme.palette.text.primary,
  }

  if (skin === 'retro-3d') {
    return {
      ...baseStyle,
      fontWeight: 'bold',
      '&:hover': {
        background:
          'linear-gradient(135deg, #FFE082 0%, #FFD54F 35%, #FFB300 70%, #B8860B 100%) !important',
        color: '#2C1F03 !important',
        textShadow: '0 1px 0 rgba(255, 255, 255, 0.45)',
      },
    }
  }
  if (skin === 'original') {
    return {
      ...baseStyle,
      fontWeight: 'bold',
      '&:hover': {
        background: 'rgba(91, 92, 157, 0.08) !important',
        color: 'var(--primary-main) !important',
      },
    }
  }
  if (skin === 'modern-flat') {
    return {
      ...baseStyle,
      fontWeight: 'bold',
      '&:hover': {
        background: `${alpha(theme.palette.primary.main, 0.1)} !important`,
        color: `${theme.palette.primary.main} !important`,
      },
    }
  }
  if (skin === 'frosted-glass') {
    return {
      ...baseStyle,
      '&:hover': {
        background: 'rgba(255, 255, 255, 0.12) !important',
        color: `${theme.palette.primary.main} !important`,
      },
    }
  }
  if (skin === 'cyberpunk') {
    return {
      ...baseStyle,
      fontWeight: 'bold',
      borderRadius: '0px',
      border: '1px solid transparent',
      '&:hover': {
        background: '#39ff14 !important',
        color: '#000000 !important',
        border: '1px solid #39ff14 !important',
        boxShadow: '0 0 8px #39ff14',
      },
    }
  }
  if (skin === 'monochrome') {
    return {
      ...baseStyle,
      borderRadius: '0px',
      '&:hover': {
        background: isLight ? '#000000 !important' : '#ffffff !important',
        color: isLight ? '#ffffff !important' : '#000000 !important',
      },
    }
  }
  return baseStyle
}

export const getFriendlyProtocolName = (type?: string) => {
  if (!type) return ''
  const map: Record<string, string> = {
    ss: 'Shadowsocks',
    ssr: 'ShadowsocksR',
    vmess: 'VMess',
    vless: 'VLESS',
    trojan: 'Trojan',
    hysteria: 'Hysteria',
    hysteria2: 'Hysteria 2',
    tuic: 'TUIC',
    wireguard: 'WireGuard',
    shadowsocks: 'Shadowsocks',
    shadowsocksr: 'ShadowsocksR',
  }
  const low = type.toLowerCase()
  return map[low] || type.toUpperCase()
}

export const formatCoreVersion = (version?: string) => {
  if (!version) return ''
  const clean = version.trim().replace(/^v+/i, '')
  return `Ver.${clean}`
}

export const isSameVersion = (ver1?: string, ver2?: string) => {
  if (!ver1 || !ver2) return false
  const clean = (v: string) => {
    return v
      .toLowerCase()
      .replace(/mihomo/g, '')
      .replace(/meta/g, '')
      .replace(/^v+/g, '')
      .replace(/[^a-z0-9.]/g, '')
      .trim()
  }
  return clean(ver1) === clean(ver2)
}

export const get3DSliderStyle = (theme: any, mode: 'light' | 'dark') => {
  const isLight = mode === 'light'
  const skin =
    theme.controlSkin ||
    (typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d')

  if (skin === 'retro-3d') {
    return {
      py: 0.5,
      '& .MuiSlider-rail': {
        height: 5,
        opacity: 0.85,
        bgcolor: isLight
          ? 'rgba(212, 175, 55, 0.25)'
          : 'rgba(212, 175, 55, 0.15)',
        boxShadow: isLight
          ? 'inset 0 3px 5px rgba(0,0,0,0.3), 0 1px 0 rgba(255,255,255,0.8)'
          : 'inset 0 3px 5px rgba(0,0,0,0.75), 0 1px 0 rgba(255,255,255,0.08)',
        border: `1px solid ${isLight ? '#B8860B' : '#8A6D00'}`,
        borderRadius: 2.5,
      },
      '& .MuiSlider-track': {
        height: 5,
        border: 'none',
        borderRadius: 2.5,
        background:
          'linear-gradient(to bottom, #FFE082 0%, #FFC107 50%, #B8860B 100%)',
        boxShadow: isLight
          ? 'inset 0 1px 0 rgba(255,255,255,0.4)'
          : 'inset 0 1px 0 rgba(255,255,255,0.08)',
      },
      '& .MuiSlider-thumb': {
        width: 14,
        height: 14,
        top: '50%',
        transform: 'translate(-50%, -50%)',
        border: `1px solid ${isLight ? '#8A6D00' : '#634E00'}`,
        background: isLight
          ? `radial-gradient(circle at 35% 35%, #FFFDE7 0%, #FFD54F 45%, #FFA000 80%, #B8860B 100%)`
          : `radial-gradient(circle at 35% 35%, #FFF9C4 0%, #FFC107 45%, #FFA000 80%, #8A6D00 100%)`,
        boxShadow: isLight
          ? `0 calc(3px * var(--depth-factor, 1.0)) calc(6px * var(--depth-factor, 1.0)) rgba(0,0,0,0.25),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.8),
             0 0 calc(8px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.25 * var(--vibrancy-factor, 1.0)))`
          : `0 calc(4px * var(--depth-factor, 1.0)) calc(8px * var(--depth-factor, 1.0)) rgba(0,0,0,0.6),
             inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.4),
             0 0 calc(10px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.3 * var(--vibrancy-factor, 1.0)))`,
        transition:
          'transform 0.1s ease-out, box-shadow 0.1s ease-out, filter 0.1s ease-out',
        '&:hover, &.Mui-focusVisible': {
          transform: 'translate(-50%, -50%) scale(1.2)',
          filter: 'brightness(1.15)',
          boxShadow: isLight
            ? `0 calc(5px * var(--depth-factor, 1.0)) calc(10px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
               inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.9),
               0 0 calc(12px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.35 * var(--vibrancy-factor, 1.0)))`
            : `0 calc(6px * var(--depth-factor, 1.0)) calc(12px * var(--depth-factor, 1.0)) rgba(0,0,0,0.7),
               inset 0 calc(1px * var(--depth-factor, 1.0)) 0 rgba(255,255,255,0.5),
               0 0 calc(14px * var(--vibrancy-factor, 1.0)) rgba(255, 193, 7, calc(0.4 * var(--vibrancy-factor, 1.0)))`,
        },
        '&.Mui-active': {
          transform: 'translate(-50%, -50%) scale(0.92)',
          boxShadow: isLight
            ? `inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(2px * var(--depth-factor, 1.0)) rgba(0,0,0,0.15)`
            : `inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0,0,0,0.5)`,
        },
      },
    }
  }

  if (skin === 'original') {
    return {
      py: 0.5,
      '& .MuiSlider-rail': {
        height: 4,
        bgcolor: isLight ? '#cbd5e1' : '#30363d',
        border: 'none',
        borderRadius: 2,
      },
      '& .MuiSlider-track': {
        height: 4,
        border: 'none',
        borderRadius: 2,
        bgcolor: 'var(--primary-main)',
      },
      '& .MuiSlider-thumb': {
        width: 12,
        height: 12,
        bgcolor: '#ffffff',
        border: `2px solid var(--primary-main)`,
        boxShadow: '0 1px 2px rgba(0,0,0,0.15)',
        '&:hover, &.Mui-focusVisible': {
          transform: 'translate(-50%, -50%) scale(1.15)',
        },
      },
    }
  }

  if (skin === 'modern-flat') {
    return {
      py: 0.5,
      '& .MuiSlider-rail': {
        height: 4,
        bgcolor: isLight ? '#e5e7eb' : '#374151',
        border: 'none',
        borderRadius: 'calc(2px * var(--control-skin-val1, 1.0))',
      },
      '& .MuiSlider-track': {
        height: 4,
        border: 'none',
        borderRadius: 'calc(2px * var(--control-skin-val1, 1.0))',
        bgcolor: theme.palette.primary.main,
      },
      '& .MuiSlider-thumb': {
        width: 12,
        height: 12,
        bgcolor: '#ffffff',
        borderRadius: 'calc(6px * var(--control-skin-val1, 1.0))',
        border: `2px solid ${theme.palette.primary.main}`,
        boxShadow: '0 1px 2px rgba(0,0,0,0.15)',
        '&:hover, &.Mui-focusVisible': {
          transform: 'translate(-50%, -50%) scale(1.15)',
        },
      },
    }
  }

  if (skin === 'frosted-glass') {
    return {
      py: 0.5,
      '& .MuiSlider-rail': {
        height: 4,
        bgcolor: isLight ? 'rgba(0, 0, 0, 0.06)' : 'rgba(255, 255, 255, 0.08)',
        border: `1px solid ${isLight ? 'rgba(0, 0, 0, 0.08)' : 'rgba(255, 255, 255, 0.1)'}`,
        borderRadius: 2,
      },
      '& .MuiSlider-track': {
        height: 4,
        border: 'none',
        borderRadius: 2,
        bgcolor: isLight
          ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.6)'
          : 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.5)',
      },
      '& .MuiSlider-thumb': {
        width: 12,
        height: 12,
        bgcolor: isLight
          ? 'rgba(255, 255, 255, 0.9)'
          : 'rgba(255, 255, 255, 0.8)',
        border: `1px solid ${isLight ? 'rgba(var(--primary-color-rgb, 91, 92, 157), 0.3)' : 'rgba(255, 255, 255, 0.3)'}`,
        boxShadow: isLight ? '0 1px 3px rgba(0, 0, 0, 0.12)' : 'none',
        '&:hover, &.Mui-focusVisible': {
          transform: 'translate(-50%, -50%) scale(1.15)',
          bgcolor: 'rgba(255, 255, 255, 1.0)',
          boxShadow: isLight
            ? '0 2px 5px rgba(0, 0, 0, 0.18)'
            : '0 0 4px rgba(255, 255, 255, 0.4)',
        },
      },
    }
  }

  if (skin === 'cyberpunk') {
    return {
      py: 0.5,
      '& .MuiSlider-rail': {
        height: 2,
        bgcolor: isLight ? '#ffffff' : '#05070c',
        border: '1px solid #ff0055',
        borderRadius: 0,
      },
      '& .MuiSlider-track': {
        height: 2,
        border: 'none',
        borderRadius: 0,
        bgcolor: isLight ? '#4caf50' : '#39ff14',
      },
      '& .MuiSlider-thumb': {
        width: 8,
        height: 12,
        borderRadius: 0,
        bgcolor: isLight ? '#4caf50' : '#39ff14',
        boxShadow: `0 0 5px ${isLight ? '#4caf50' : '#39ff14'}`,
        '&:hover, &.Mui-focusVisible': {
          transform: 'translate(-50%, -50%) scale(1.15)',
          boxShadow: `0 0 8px ${isLight ? '#4caf50' : '#39ff14'}`,
        },
      },
    }
  }

  if (skin === 'monochrome') {
    return {
      py: 0.5,
      '& .MuiSlider-rail': {
        height: 2,
        bgcolor: isLight ? '#ffffff' : '#000000',
        border: '1px solid ' + (isLight ? '#000000' : '#ffffff'),
        borderRadius: 0,
      },
      '& .MuiSlider-track': {
        height: 2,
        border: 'none',
        borderRadius: 0,
        bgcolor: isLight ? '#000000' : '#ffffff',
      },
      '& .MuiSlider-thumb': {
        width: 10,
        height: 10,
        borderRadius: 0,
        bgcolor: isLight ? '#000000' : '#ffffff',
        border: 'none',
        boxShadow: 'none',
      },
    }
  }

  return {}
}

export { OS }
