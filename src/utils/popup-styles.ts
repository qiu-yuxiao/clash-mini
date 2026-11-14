import {
  alpha,
  type DialogProps,
  type PaperProps,
  type SxProps,
  type Theme,
} from '@mui/material'
import type React from 'react'
import { use } from 'react'

import { CUSTOM_TITLEBAR_HEIGHT, MIN_WINDOW_WIDTH } from '@/constants'
import { WindowContext, type WindowContextType } from '@/providers/window'
import { get3DCardStyle, getActiveSkin } from '@/utils/button-styles'

export interface PopupSkinTokens {
  skin: string
  isRetro3D: boolean
  primaryText: string
  secondaryText: string
  disabledText: string
  dividerColor: string
  readyColor: string
  warningColor: string
  errorColor: string
  infoColor: string
  accentPrimaryColor: string
  subtleSurfaceBg: string
  subtleSurfaceBorder: string
}

/**
 * 解析窄窗浮层（弹窗 / 遮罩 / 顶部通知）的顶部安全偏移：
 * 自定义标题栏可见时让出 30px 标题栏，隐藏（原生装饰或闲置自动隐藏）时贴顶 0px。
 */
export function resolveNarrowOverlayTopOffset(
  state?: Pick<WindowContextType, 'decorated' | 'isDecorationsHidden'> | null,
): number {
  return state?.decorated || state?.isDecorationsHidden
    ? 0
    : CUSTOM_TITLEBAR_HEIGHT
}

/** React Hook：安全读取当前窗口装饰状态并返回窄窗浮层顶部偏移（px） */
export function useNarrowOverlayTopOffset(): number {
  const ctx = use(WindowContext)
  return resolveNarrowOverlayTopOffset(ctx ?? null)
}

/** 返回与统一弹窗卡片底色匹配的对比度语义色（特别处理 retro-3d 深浅色均为金黄底色的场景） */
export function getPopupSkinTokens(theme: Theme): PopupSkinTokens {
  const skin = getActiveSkin(theme)
  const isLight = theme.palette.mode === 'light'
  const isRetro3D = skin === 'retro-3d'

  return {
    skin,
    isRetro3D,
    primaryText: isRetro3D
      ? isLight
        ? '#3C2F0F'
        : '#2C1F03'
      : theme.palette.text.primary,
    secondaryText: isRetro3D
      ? 'rgba(44, 31, 3, 0.78)'
      : theme.palette.text.secondary,
    disabledText: isRetro3D
      ? 'rgba(44, 31, 3, 0.5)'
      : theme.palette.text.disabled,
    dividerColor: isRetro3D ? 'rgba(44, 31, 3, 0.2)' : theme.palette.divider,
    readyColor: isRetro3D ? '#1B5E20' : theme.palette.success.main,
    warningColor: isRetro3D ? '#8C4A03' : theme.palette.warning.main,
    errorColor: isRetro3D ? '#991B1B' : theme.palette.error.main,
    infoColor: isRetro3D ? '#01579B' : theme.palette.info.main,
    accentPrimaryColor: isRetro3D ? '#0D47A1' : theme.palette.primary.main,
    subtleSurfaceBg: isRetro3D
      ? 'rgba(255, 255, 255, 0.22)'
      : isLight
        ? 'rgba(0, 0, 0, 0.04)'
        : 'rgba(255, 255, 255, 0.06)',
    subtleSurfaceBorder: isRetro3D
      ? 'rgba(122, 62, 0, 0.32)'
      : theme.palette.divider,
  }
}

/**
 * 统一弹窗与浮层菜单卡片表面样式：
 * 沿用新手引导弹窗 get3DCardStyle(theme, 'default')，并确保 frosted-glass 在浮层下使用高不透明实底、无文字穿透。
 */
export function getPopupCardStyle(theme: Theme): Record<string, unknown> {
  const base = get3DCardStyle(theme, 'default')
  const skin = getActiveSkin(theme)
  const isLight = theme.palette.mode === 'light'

  if (skin === 'frosted-glass') {
    return {
      ...base,
      boxSizing: 'border-box',
      backgroundImage: 'none',
      background: isLight
        ? 'rgba(248, 250, 252, 0.97)'
        : 'rgba(26, 28, 38, 0.97)',
      border: isLight
        ? '1px solid rgba(15, 23, 42, 0.12)'
        : '1px solid rgba(255, 255, 255, 0.14)',
      boxShadow: isLight
        ? '0 8px 28px rgba(15, 23, 42, 0.16)'
        : '0 8px 28px rgba(0, 0, 0, 0.45)',
      backdropFilter: 'none',
    }
  }

  return {
    ...base,
    boxSizing: 'border-box',
  }
}

/** 窄窗弹窗 Root + Backdrop 几何锁定：永远锚定左侧 0~271px、且不遮挡顶部自定义标题栏 */
export function getNarrowDialogSx(topOffset: number): SxProps<Theme> {
  const areaSx = {
    top: `${topOffset}px`,
    left: 0,
    right: 'auto',
    bottom: 0,
    width: `${MIN_WINDOW_WIDTH}px`,
    height: `calc(100vh - ${topOffset}px)`,
  }

  return {
    ...areaSx,
    '& .MuiBackdrop-root': areaSx,
  }
}

/** 窄窗弹窗 slotProps：锁定 Backdrop / Container / Paper 到左侧 271px 窄窗坐标，Paper 统一样式与 271px 精确宽度 */
export function getNarrowDialogSlotProps(
  theme: Theme,
  topOffset: number,
  options?: {
    paperSx?: Record<string, unknown>
    onKeyDown?: React.KeyboardEventHandler<HTMLDivElement>
  },
): DialogProps['slotProps'] {
  const areaSx = {
    top: `${topOffset}px`,
    left: 0,
    right: 'auto',
    bottom: 0,
    width: `${MIN_WINDOW_WIDTH}px`,
    height: `calc(100vh - ${topOffset}px)`,
  }

  return {
    backdrop: {
      sx: areaSx,
    },
    container: {
      sx: {
        width: `${MIN_WINDOW_WIDTH}px`,
        height: '100%',
        m: 0,
        p: 0,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-start',
      },
    },
    paper: {
      className: 'popup-card-paper',
      ...(options?.onKeyDown ? { onKeyDown: options.onKeyDown } : {}),
      sx: {
        ...getPopupCardStyle(theme),
        m: 0,
        p: 0,
        width: `${MIN_WINDOW_WIDTH}px`,
        minWidth: `${MIN_WINDOW_WIDTH}px`,
        maxWidth: `${MIN_WINDOW_WIDTH}px`,
        maxHeight: `calc(100vh - ${topOffset}px - 8px)`,
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        boxSizing: 'border-box',
        ...(options?.paperSx ?? {}),
      },
    },
  }
}

/** 新手引导风格 DialogTitle 样式 */
export function getPopupTitleSx(
  theme: Theme,
  extraSx?: Record<string, unknown>,
): Record<string, unknown> {
  const tokens = getPopupSkinTokens(theme)
  return {
    color: tokens.primaryText,
    px: 1.75,
    py: 1.35,
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    textAlign: 'center',
    gap: 0.5,
    fontSize: '15px',
    fontWeight: 700,
    lineHeight: 1.35,
    flexShrink: 0,
    borderBottom: `1px solid ${tokens.dividerColor}`,
    ...(extraSx ?? {}),
  }
}

/** 新手引导风格 DialogContent 样式 */
export function getPopupContentSx(
  extraSx?: Record<string, unknown>,
): Record<string, unknown> {
  return {
    px: 1.75,
    py: 1.5,
    '&.MuiDialogContent-root': { pt: 1.5 },
    overflowY: 'auto',
    overflowX: 'hidden',
    flex: '1 1 auto',
    minHeight: 0,
    ...(extraSx ?? {}),
  }
}

/** 新手引导风格 DialogActions 样式（全宽纵向堆叠） */
export function getPopupActionsSx(
  theme: Theme,
  extraSx?: Record<string, unknown>,
): Record<string, unknown> {
  const tokens = getPopupSkinTokens(theme)
  return {
    flexDirection: 'column',
    alignItems: 'stretch',
    px: 1.75,
    py: 1.35,
    gap: 1,
    flexShrink: 0,
    borderTop: `1px solid ${tokens.dividerColor}`,
    ...(extraSx ?? {}),
  }
}

function getSkinMenuFontFamily(skin: string): string {
  switch (skin) {
    case 'original':
      return 'Segoe UI, Microsoft YaHei, sans-serif'
    case 'modern-flat':
      return 'Outfit, DengXian, sans-serif'
    case 'frosted-glass':
      return 'Segoe UI Light, Microsoft YaHei Light, sans-serif'
    case 'cyberpunk':
      return 'Consolas, NSimSun, monospace'
    case 'monochrome':
      return 'Georgia, KaiTi, serif'
    default:
      return 'Trebuchet MS, SimHei, sans-serif'
  }
}

/** 统一右键菜单与下拉选项条目样式（确保 retro-3d 金黄卡片下深浅色对比度与 hover 高亮清晰） */
export function getPopupMenuItemStyle(
  theme: Theme,
  options?: {
    danger?: boolean
    extraSx?: Record<string, unknown>
  },
): Record<string, unknown> {
  const tokens = getPopupSkinTokens(theme)
  const isLight = theme.palette.mode === 'light'
  const fontFamily = getSkinMenuFontFamily(tokens.skin)

  const base: Record<string, unknown> = {
    fontFamily,
    fontSize: '12.5px',
    minHeight: '32px',
    py: 0.75,
    px: 1.75,
    gap: 1.25,
    color: options?.danger ? tokens.errorColor : tokens.primaryText,
    transition: 'background-color 0.12s ease, color 0.12s ease',
    '&.Mui-disabled': {
      color: tokens.disabledText,
      opacity: 0.55,
    },
  }

  let interactionSx: Record<string, unknown>
  if (tokens.skin === 'retro-3d') {
    interactionSx = {
      fontWeight: 600,
      '&:hover': {
        background: `${isLight ? 'rgba(184, 134, 11, 0.22)' : 'rgba(44, 31, 3, 0.16)'} !important`,
        color: `${options?.danger ? '#7F1D1D' : '#1F1500'} !important`,
      },
      '&.Mui-selected': {
        background: `${isLight ? 'rgba(184, 134, 11, 0.28)' : 'rgba(44, 31, 3, 0.22)'} !important`,
      },
    }
  } else if (tokens.skin === 'cyberpunk') {
    interactionSx = {
      fontWeight: 700,
      borderRadius: '0px',
      '&:hover': {
        background: '#39ff14 !important',
        color: '#000000 !important',
        boxShadow: '0 0 8px #39ff14',
      },
      '&.Mui-selected': {
        background: 'rgba(0, 240, 255, 0.18) !important',
      },
    }
  } else if (tokens.skin === 'monochrome') {
    interactionSx = {
      borderRadius: '0px',
      '&:hover': {
        background: `${isLight ? '#000000' : '#ffffff'} !important`,
        color: `${isLight ? '#ffffff' : '#000000'} !important`,
      },
      '&.Mui-selected': {
        background: `${isLight ? 'rgba(0, 0, 0, 0.12)' : 'rgba(255, 255, 255, 0.18)'} !important`,
      },
    }
  } else {
    interactionSx = {
      fontWeight: 500,
      '&:hover': {
        background: `${alpha(theme.palette.primary.main, isLight ? 0.1 : 0.18)} !important`,
        color: `${options?.danger ? tokens.errorColor : theme.palette.primary.main} !important`,
      },
      '&.Mui-selected': {
        background: `${alpha(theme.palette.primary.main, isLight ? 0.14 : 0.22)} !important`,
      },
    }
  }

  return {
    ...base,
    ...interactionSx,
    ...(options?.extraSx ?? {}),
  }
}

/** 统一 Menu / Select 下拉框 Paper 属性，保持弹窗卡片外观 */
export function getPopupMenuPaperProps(
  theme: Theme,
  extraSx?: Record<string, unknown>,
): PaperProps {
  const tokens = getPopupSkinTokens(theme)
  return {
    className: 'popup-card-paper',
    sx: {
      ...getPopupCardStyle(theme),
      p: 0,
      overflowX: 'hidden',
      '& .MuiList-root': {
        py: '4px',
      },
      '& .MuiMenuItem-root': getPopupMenuItemStyle(theme),
      '& .MuiDivider-root': {
        my: '4px',
        borderColor: tokens.dividerColor,
      },
      ...(extraSx ?? {}),
    },
  }
}
