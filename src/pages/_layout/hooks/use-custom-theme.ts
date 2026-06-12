import { alpha, createTheme, Theme as MuiTheme, Shadows } from '@mui/material'
import {
  getCurrentWebviewWindow,
  WebviewWindow,
} from '@tauri-apps/api/webviewWindow'
import { Theme as TauriOsTheme } from '@tauri-apps/api/window'
import { useEffect, useMemo, useState } from 'react'

import { useVerge } from '@/hooks/use-verge'
import { defaultDarkTheme, defaultTheme } from '@/pages/_theme'
import { useSetThemeMode, useThemeMode } from '@/services/states'
import getSystem from '@/utils/get-system'

const getSystemAccentColor = (): string | null => {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return null
  }
  try {
    const dummy = document.createElement('div')
    dummy.style.color = 'AccentColor'
    dummy.style.display = 'none'
    document.body.appendChild(dummy)
    const color = window.getComputedStyle(dummy).color
    document.body.removeChild(dummy)

    if (color && color.startsWith('rgb')) {
      const match = color.match(/\d+/g)
      if (match && match.length >= 3) {
        const r = parseInt(match[0], 10)
        const g = parseInt(match[1], 10)
        const b = parseInt(match[2], 10)
        return '#' + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1).toUpperCase()
      }
    }
  } catch (e) {
    // ignore
  }
  return null
}

const CSS_INJECTION_SCOPE_ROOT = '[data-css-injection-root]'
const CSS_INJECTION_SCOPE_LIMIT =
  ':is(.monaco-editor .view-lines, .monaco-editor .view-line, .monaco-editor .margin, .monaco-editor .margin-view-overlays, .monaco-editor .view-overlays, .monaco-editor [class^="mtk"], .monaco-editor [class*=" mtk"])'
const TOP_LEVEL_AT_RULES = [
  '@charset',
  '@import',
  '@namespace',
  '@font-face',
  '@keyframes',
  '@counter-style',
  '@page',
  '@property',
  '@font-feature-values',
  '@color-profile',
]
let cssScopeSupport: boolean | null = null

const canUseCssScope = () => {
  if (cssScopeSupport !== null) {
    return cssScopeSupport
  }
  try {
    const testStyle = document.createElement('style')
    testStyle.textContent = '@scope (:root) { }'
    document.head.appendChild(testStyle)
    cssScopeSupport = !!testStyle.sheet?.cssRules?.length
    document.head.removeChild(testStyle)
  } catch {
    cssScopeSupport = false
  }
  return cssScopeSupport
}

const wrapCssInjectionWithScope = (css?: string) => {
  if (!css?.trim()) {
    return ''
  }
  const lowerCss = css.toLowerCase()
  const hasTopLevelOnlyRule = TOP_LEVEL_AT_RULES.some((rule) =>
    lowerCss.includes(rule),
  )
  if (hasTopLevelOnlyRule) {
    return null
  }
  const scopeRoot = CSS_INJECTION_SCOPE_ROOT
  const scopeLimit = CSS_INJECTION_SCOPE_LIMIT
  const scopedBlock = `@scope (${scopeRoot}) to (${scopeLimit}) {
${css}
}`
  return scopedBlock
}

/**
 * custom theme
 */
export const useCustomTheme = () => {
  const appWindow: WebviewWindow = useMemo(() => getCurrentWebviewWindow(), [])
  const { verge } = useVerge()
  const { theme_mode, theme_setting } = verge ?? {}
  const mode = useThemeMode()
  const setMode = useSetThemeMode()
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

  useEffect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.setAttribute('data-control-skin', controlSkin)
    }
  }, [controlSkin])
  const userBackgroundImage = theme_setting?.background_image || ''
  const hasUserBackground = !!userBackgroundImage

  useEffect(() => {
    if (theme_mode === 'light' || theme_mode === 'dark') {
      setMode(theme_mode)
    }
  }, [theme_mode, setMode])

  useEffect(() => {
    if (theme_mode !== 'system') {
      return
    }

    let isMounted = true

    const timerId = setTimeout(() => {
      if (!isMounted) return
      appWindow
        .theme()
        .then((systemTheme) => {
          if (isMounted && systemTheme) {
            setMode(systemTheme)
          }
        })
        .catch((err) => {
          console.error('Failed to get initial system theme:', err)
        })
    }, 0)

    const unlistenPromise = appWindow.onThemeChanged(({ payload }) => {
      if (isMounted) {
        setMode(payload)
      }
    })

    return () => {
      isMounted = false
      clearTimeout(timerId)
      unlistenPromise
        .then((unlistenFn) => {
          if (typeof unlistenFn === 'function') {
            unlistenFn()
          }
        })
        .catch((err) => {
          console.error('Failed to unlisten from theme changes:', err)
        })
    }
  }, [theme_mode, appWindow, setMode])

  useEffect(() => {
    if (theme_mode === undefined) {
      return
    }

    if (theme_mode === 'system') {
      appWindow.setTheme(null).catch((err) => {
        console.error(
          'Failed to set window theme to follow system (setTheme(null)):',
          err,
        )
      })
    } else if (mode) {
      appWindow.setTheme(mode as TauriOsTheme).catch((err) => {
        console.error(`Failed to set window theme to ${mode}:`, err)
      })
    }
  }, [mode, appWindow, theme_mode])

  const theme = useMemo(() => {
    const setting = theme_setting || {}
    const dt = mode === 'light' ? defaultTheme : defaultDarkTheme
    let muiTheme: MuiTheme

    let resolvedPrimary = setting.primary_color
    if (!resolvedPrimary) {
      resolvedPrimary = getSystemAccentColor() || dt.primary_color
    }
    if (controlSkin === 'original') {
      resolvedPrimary = '#5b5c9d'
    }

    try {
      muiTheme = createTheme({
        breakpoints: {
          values: { xs: 0, sm: 650, md: 900, lg: 1200, xl: 1536 },
        },
        palette: {
          mode,
          primary: { main: resolvedPrimary },
          secondary: { main: setting.secondary_color || dt.secondary_color },
          info: { main: setting.info_color || dt.info_color },
          error: { main: setting.error_color || dt.error_color },
          warning: { main: setting.warning_color || dt.warning_color },
          success: { main: setting.success_color || dt.success_color },
          text: {
            primary: setting.primary_text || dt.primary_text,
            secondary: setting.secondary_text || dt.secondary_text,
          },
          background: {
            paper: dt.background_color,
            default: dt.background_color,
          },
        },
        shadows: Array(25).fill('none') as Shadows,
        typography: {
          fontFamily: setting.font_family
            ? `${setting.font_family}, ${dt.font_family}`
            : dt.font_family,
          fontSize: 12,
          htmlFontSize: 14,
          button: {
            fontSize: '0.8rem',
            textTransform: 'none',
          },
          body1: {
            fontSize: '0.85rem',
            lineHeight: 1.4,
          },
          body2: {
            fontSize: '0.75rem',
            lineHeight: 1.4,
          },
          subtitle1: {
            fontSize: '0.9rem',
          },
          subtitle2: {
            fontSize: '0.8rem',
          },
          h6: {
            fontSize: '0.95rem',
            fontWeight: 600,
          },
          h5: {
            fontSize: '1.1rem',
            fontWeight: 600,
          },
        },
        components: {
          MuiOutlinedInput: {
            styleOverrides: {
              root: ({ theme }) => {
                const isLight = theme.palette.mode === 'light'
                return {
                  backgroundColor: 'var(--theme-input-bg)',
                  borderRadius: 4,
                  transition: 'all 0.2s ease',
                  '& .MuiOutlinedInput-notchedOutline': {
                    borderWidth: '1px',
                    borderColor: 'var(--theme-input-border)',
                  },
                  boxShadow: isLight
                    ? `inset 0 calc(1px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.08),
                       inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.06),
                       0 calc(0.5px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255, 255, 255, 0.5)`
                    : `inset 0 calc(1.5px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.45),
                       inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.35),
                       0 calc(0.5px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255, 255, 255, 0.05)`,
                  '&.Mui-focused': {
                    boxShadow: isLight
                      ? `inset 0 calc(1.5px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.12),
                         0 0 calc(4px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.2 * var(--vibrancy-factor, 1.0)))`
                      : `inset 0 calc(2px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.6),
                         0 0 calc(5px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.25 * var(--vibrancy-factor, 1.0)))`,
                    '& .MuiOutlinedInput-notchedOutline': {
                      borderWidth: '1.5px',
                      borderColor: theme.palette.primary.main,
                    },
                  },
                  '&:hover:not(.Mui-focused) .MuiOutlinedInput-notchedOutline':
                    {
                      borderColor: isLight
                        ? 'rgba(0, 0, 0, 0.23)'
                        : 'rgba(255, 255, 255, 0.23)',
                    },
                }
              },
              input: {
                padding: '8px 12px',
                fontSize: '13px',
              },
            },
          },
        },
      })
    } catch (e) {
      console.error('Error creating MUI theme, falling back to defaults:', e)
      muiTheme = createTheme({
        breakpoints: {
          values: { xs: 0, sm: 650, md: 900, lg: 1200, xl: 1536 },
        },
        palette: {
          mode,
          primary: { main: resolvedPrimary },
          secondary: { main: dt.secondary_color },
          info: { main: dt.info_color },
          error: { main: dt.error_color },
          warning: { main: dt.warning_color },
          success: { main: dt.success_color },
          text: { primary: dt.primary_text, secondary: dt.secondary_text },
          background: {
            paper: dt.background_color,
            default: dt.background_color,
          },
        },
        typography: {
          fontFamily: dt.font_family,
          fontSize: 12,
          htmlFontSize: 14,
        },
        components: {
          MuiOutlinedInput: {
            styleOverrides: {
              root: ({ theme }) => {
                const isLight = theme.palette.mode === 'light'
                return {
                  backgroundColor: 'var(--theme-input-bg)',
                  borderRadius: 4,
                  transition: 'all 0.2s ease',
                  '& .MuiOutlinedInput-notchedOutline': {
                    borderWidth: '1px',
                    borderColor: 'var(--theme-input-border)',
                  },
                  boxShadow: isLight
                    ? `inset 0 calc(1px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.08),
                       inset calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.06),
                       0 calc(0.5px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255, 255, 255, 0.5)`
                    : `inset 0 calc(1.5px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.45),
                       inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(2.5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.35),
                       0 calc(0.5px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255, 255, 255, 0.05)`,
                  '&.Mui-focused': {
                    boxShadow: isLight
                      ? `inset 0 calc(1.5px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.12),
                         0 0 calc(4px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.2 * var(--vibrancy-factor, 1.0)))`
                      : `inset 0 calc(2px * var(--depth-factor, 1.0)) calc(5px * var(--depth-factor, 1.0)) rgba(0, 0, 0, 0.6),
                         0 0 calc(5px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.25 * var(--vibrancy-factor, 1.0)))`,
                    '& .MuiOutlinedInput-notchedOutline': {
                      borderWidth: '1.5px',
                      borderColor: theme.palette.primary.main,
                    },
                  },
                  '&:hover:not(.Mui-focused) .MuiOutlinedInput-notchedOutline':
                    {
                      borderColor: isLight
                        ? 'rgba(0, 0, 0, 0.23)'
                        : 'rgba(255, 255, 255, 0.23)',
                    },
                }
              },
              input: {
                padding: '8px 12px',
                fontSize: '13px',
              },
            },
          },
        },
      })
    }

    const rootEle = document.documentElement
    if (rootEle) {
      const backgroundColor = mode === 'light' ? '#ECECEC' : dt.background_color
      const selectColor = mode === 'light' ? '#f5f5f5' : '#3E3E3E'
      const scrollColor = mode === 'light' ? '#90939980' : '#555555'
      const dividerColor =
        mode === 'light' ? 'rgba(0, 0, 0, 0.06)' : 'rgba(255, 255, 255, 0.06)'
      rootEle.style.setProperty('--divider-color', dividerColor)
      rootEle.style.setProperty('--background-color', backgroundColor)
      rootEle.style.setProperty('--selection-color', selectColor)
      rootEle.style.setProperty('--scroller-color', scrollColor)
      rootEle.style.setProperty('--primary-main', muiTheme.palette.primary.main)
      const primaryHex = muiTheme.palette.primary.main
      let primaryRgb = '91, 92, 157' // default fallback
      if (primaryHex && primaryHex.startsWith('#')) {
        const cleanHex = primaryHex.replace('#', '')
        if (cleanHex.length === 6) {
          const r = parseInt(cleanHex.substring(0, 2), 16)
          const g = parseInt(cleanHex.substring(2, 4), 16)
          const b = parseInt(cleanHex.substring(4, 6), 16)
          primaryRgb = `${r}, ${g}, ${b}`
        }
      }
      rootEle.style.setProperty('--primary-color-rgb', primaryRgb)

      rootEle.style.setProperty(
        '--background-color-alpha',
        alpha(muiTheme.palette.primary.main, 0.1),
      )
      rootEle.style.setProperty(
        '--window-border-color',
        mode === 'light' ? '#cccccc' : '#1E1E1E',
      )
      rootEle.style.setProperty(
        '--scrollbar-bg',
        mode === 'light' ? '#f1f1f1' : '#2E303D',
      )
      rootEle.style.setProperty(
        '--scrollbar-thumb',
        mode === 'light' ? '#c1c1c1' : '#555555',
      )
      rootEle.style.setProperty(
        '--user-background-image',
        hasUserBackground ? `url('${userBackgroundImage}')` : 'none',
      )
      rootEle.style.setProperty(
        '--background-blend-mode',
        setting.background_blend_mode || 'normal',
      )
      rootEle.style.setProperty(
        '--background-opacity',
        setting.background_opacity !== undefined
          ? String(setting.background_opacity)
          : '1',
      )

      // Theme Glass/Flat Variables (Flattened: Opaque, solid, no gloss/glass/skeuomorphism effects)
      if (controlSkin === 'original') {
        rootEle.style.setProperty(
          '--theme-bg',
          mode === 'light' ? '#f5f5f5' : '#2e303d',
        )
        rootEle.style.setProperty(
          '--theme-panel-bg',
          mode === 'light' ? '#ffffff' : '#1e1f29',
        )
        rootEle.style.setProperty(
          '--theme-bg-base-rgb',
          mode === 'light' ? '245, 245, 245' : '46, 48, 61',
        )
        rootEle.style.setProperty(
          '--theme-panel-base-rgb',
          mode === 'light' ? '255, 255, 255' : '30, 31, 41',
        )
        rootEle.style.setProperty(
          '--theme-popover-bg',
          mode === 'light' ? '#ffffff' : '#1b1c23',
        )
        rootEle.style.setProperty(
          '--theme-border',
          mode === 'light' ? '#e2e8f0' : '#30363d',
        )
        rootEle.style.setProperty(
          '--theme-input-bg',
          mode === 'light' ? '#ffffff' : '#1a1b26',
        )
        rootEle.style.setProperty(
          '--theme-input-border',
          mode === 'light' ? '#cbd5e1' : '#475569',
        )
        rootEle.style.setProperty(
          '--theme-border-outer',
          mode === 'light' ? 'rgba(0, 0, 0, 0.05)' : 'rgba(0, 0, 0, 0.25)',
        )
      } else {
        rootEle.style.setProperty(
          '--theme-bg',
          mode === 'light' ? '#f0f5ff' : '#0f1423',
        )
        rootEle.style.setProperty(
          '--theme-panel-bg',
          mode === 'light' ? '#ffffff' : '#1e2438',
        )
        rootEle.style.setProperty(
          '--theme-bg-base-rgb',
          mode === 'light' ? '240, 245, 255' : '15, 20, 35',
        )
        rootEle.style.setProperty(
          '--theme-panel-base-rgb',
          mode === 'light' ? '255, 255, 255' : '30, 36, 56',
        )
        rootEle.style.setProperty(
          '--theme-popover-bg',
          mode === 'light' ? '#ffffff' : '#1e1e23',
        )
        rootEle.style.setProperty(
          '--theme-border',
          mode === 'light' ? '#d0d7de' : '#30363d',
        )
        rootEle.style.setProperty(
          '--theme-input-bg',
          mode === 'light' ? '#ffffff' : '#121824',
        )
        rootEle.style.setProperty(
          '--theme-input-border',
          mode === 'light' ? '#d0d7de' : '#30363d',
        )
        rootEle.style.setProperty(
          '--theme-border-outer',
          mode === 'light' ? 'rgba(0, 0, 0, 0.08)' : 'rgba(0, 0, 0, 0.35)',
        )
      }
      rootEle.style.setProperty(
        '--theme-glass-border-rgb',
        mode === 'light' ? '0, 0, 0' : '255, 255, 255',
      )
      rootEle.style.setProperty(
        '--theme-box-shadow',
        mode === 'light'
          ? '0 8px 32px 0 rgba(0, 0, 0, 0.08)'
          : '0 8px 32px 0 rgba(0, 0, 0, 0.35)',
      )
      rootEle.style.setProperty(
        '--theme-box-shadow-subtle',
        mode === 'light'
          ? '0 4px 16px 0 rgba(0, 0, 0, 0.05)'
          : '0 4px 16px 0 rgba(0, 0, 0, 0.22)',
      )
      rootEle.style.setProperty(
        '--theme-btn-bg',
        mode === 'light' ? '#f5f5f5' : '#2d3345',
      )
      rootEle.style.setProperty(
        '--theme-btn-hover-bg',
        mode === 'light' ? '#e8e8e8' : '#383e52',
      )
      rootEle.style.setProperty('--theme-btn-shadow', 'none')
      rootEle.style.setProperty('--theme-btn-hover-shadow', 'none')
      rootEle.style.setProperty('--theme-btn-active-shadow', 'none')
      rootEle.setAttribute('data-css-injection-root', 'true')
    }

    let styleElement = document.querySelector('style#verge-theme')
    if (!styleElement) {
      styleElement = document.createElement('style')
      styleElement.id = 'verge-theme'
      document.head.appendChild(styleElement!)
    }

    if (styleElement) {
      let scopedCss: string | null = null
      if (canUseCssScope() && setting.css_injection) {
        scopedCss = wrapCssInjectionWithScope(setting.css_injection)
      }
      const effectiveInjectedCss = scopedCss ?? setting.css_injection ?? ''

      const globalStyles = `
        /* 恢复窄 3D 滚动条样式 */
        * {
          scrollbar-width: thin !important;
        }
        ::-webkit-scrollbar {
          width: 6px !important;
          height: 6px !important;
        }
        ::-webkit-scrollbar-track {
          background: transparent !important;
        }
        ::-webkit-scrollbar-thumb {
          background: var(--primary-main, #1976d2) !important;
          border-radius: 3px !important;
          box-shadow: inset 1px 1px 1px rgba(255, 255, 255, 0.3), inset -1px -1px 1px rgba(0, 0, 0, 0.2) !important;
        }
        ::-webkit-scrollbar-thumb:hover {
          background: var(--primary-light, #42a5f5) !important;
        }

        /* 背景图处理 */
        body {
          background-color: ${getSystem() === 'windows' ? 'transparent' : 'var(--background-color)'} !important;
          ${
            hasUserBackground
              ? `
            background-image: var(--user-background-image);
            background-size: cover;
            background-position: center;
            background-attachment: fixed;
            background-blend-mode: var(--background-blend-mode);
            opacity: var(--background-opacity);
          `
              : ''
          }
        }

        /* 修复可能的白色边框 */
        .MuiPaper-root {
          border-color: var(--window-border-color) !important;
        }

        /* 确保模态框和对话框也使用暗色主题 */
        .MuiDialog-paper {
          background-color: ${mode === 'light' ? '#ffffff' : '#2E303D'} !important;
        }

        /* 移除可能的白色点或线条 */
        * {
          outline: none !important;
        }
      `

      styleElement.innerHTML = effectiveInjectedCss + globalStyles
    }

    ;(muiTheme as any).controlSkin = controlSkin
    return muiTheme
  }, [mode, theme_setting, userBackgroundImage, hasUserBackground, controlSkin])

  useEffect(() => {
    const id = setTimeout(() => {
      const dom = document.querySelector('#Gradient2')
      if (dom) {
        dom.innerHTML = `
        <stop offset="0%" stop-color="${theme.palette.primary.main}" />
        <stop offset="80%" stop-color="${theme.palette.primary.dark}" />
        <stop offset="100%" stop-color="${theme.palette.primary.dark}" />
        `
      }
    }, 0)
    return () => clearTimeout(id)
  }, [theme.palette.primary.main, theme.palette.primary.dark])

  return { theme }
}
