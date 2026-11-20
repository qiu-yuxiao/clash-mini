import type { Theme } from '@mui/material'

// Centralized Skin Getter Helper
export const getActiveSkin = (theme: Theme): string => {
  // 优先使用 React theme 中的 controlSkin（保证与 React 状态同步）
  if (theme && theme.controlSkin) {
    return theme.controlSkin
  }
  // fallback: 从 localStorage 读取
  if (typeof window !== 'undefined') {
    return localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
  }
  return 'retro-3d'
}
