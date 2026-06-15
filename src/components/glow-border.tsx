import { useEffect, useState } from 'react'
import { useTheme } from '@mui/material'

/**
 * GlowBorder
 *
 * A static, theme-adaptive 4px window border.
 * - No animation loops or requestAnimationFrame, reducing CPU/GPU overhead to 0%.
 * - Supports 6 distinct skins matching Clash Mini's aesthetics.
 */
export const GlowBorder = () => {
  const theme = useTheme()
  const isLight = theme.palette.mode === 'light'
  
  const [controlSkin, setControlSkin] = useState(() => {
    return typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d'
  })

  useEffect(() => {
    const handleSkinChanged = () => {
      setControlSkin(
        localStorage.getItem('clash-mini-control-skin') || 'retro-3d',
      )
    }
    window.addEventListener('clash-mini-skin-changed', handleSkinChanged)
    return () => {
      window.removeEventListener('clash-mini-skin-changed', handleSkinChanged)
    }
  }, [])

  let borderStyle: React.CSSProperties = {}

  switch (controlSkin) {
    case 'retro-3d':
      borderStyle = {
        border: '4px double transparent',
        borderImage: 'linear-gradient(135deg, #FFC400, #0084FF) 4',
        boxShadow: 'inset 0 0 8px rgba(212, 175, 55, 0.4)',
      }
      break
    case 'original':
      borderStyle = {
        border: `4px solid ${isLight ? '#E0E0E0' : '#2D2D2D'}`,
      }
      break
    case 'modern-flat':
      borderStyle = {
        border: `4px solid var(--primary-main, ${theme.palette.primary.main})`,
        boxShadow: '0 0 10px rgba(0, 0, 0, 0.15)',
      }
      break
    case 'frosted-glass':
      borderStyle = {
        border: '4px double rgba(255, 255, 255, 0.25)',
        boxShadow: '0 0 20px rgba(0, 0, 0, 0.3)',
      }
      break
    case 'cyberpunk':
      borderStyle = {
        border: '4px double transparent',
        borderImage: 'linear-gradient(135deg, #00F5FF, #FF007F) 4',
        filter: 'drop-shadow(0 0 4px rgba(0, 245, 255, 0.5))',
      }
      break
    case 'monochrome':
      borderStyle = {
        border: `4px solid ${isLight ? '#808080' : '#404040'}`,
      }
      break
    default:
      borderStyle = {
        border: `4px solid ${isLight ? '#E0E0E0' : '#2D2D2D'}`,
      }
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        pointerEvents: 'none',
        zIndex: 9999,
        boxSizing: 'border-box',
        ...borderStyle,
      }}
    />
  )
}
