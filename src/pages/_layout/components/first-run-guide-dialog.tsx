import {
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
  Typography,
  useTheme,
  Menu,
  MenuItem,
  Divider,
} from '@mui/material'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import type React from 'react'
import { useState, useRef, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { useAppData } from '@/providers/app-data-context'
import { NODE_DELAY_MAX_MS } from '@/services/delay'
import {
  get3DCardStyle,
  get3DButtonStyle,
  get3DInputStyle,
  getActiveSkin,
} from '@/utils/button-styles'

import { getMenuItemHoverStyle } from '../utils/style-helpers'

export const CLASH_MINI_WIKI_URL =
  'https://github.com/qiu-yuxiao/clash-mini/wiki'
export const FIRST_RUN_GUIDE_STORAGE_KEY = 'clash-mini-first-run-completed'

/** 就绪判定超时：超时仍未测出有效延迟，就不再宣称「已就绪」，改为如实提示 */
const READY_TIMEOUT_MS = 20_000
/** 就绪判定轮询间隔：借用既有代理上下文刷新，不另造测量链路 */
const READY_POLL_INTERVAL_MS = 1_500

interface FirstRunGuideDialogProps {
  open: boolean
  onClose: () => void
  onImport: (url: string) => Promise<boolean>
  onOpenWiki: () => void
  onLater: () => void
}

export const FirstRunGuideDialog: React.FC<FirstRunGuideDialogProps> = ({
  open,
  onClose,
  onImport,
  onOpenWiki,
  onLater,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const controlSkin = theme.controlSkin || 'default'

  const [step, setStep] = useState<'input' | 'ready'>('input')
  const [url, setUrl] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [errorMessage, setErrorMessage] = useState('')
  const [gaveUp, setGaveUp] = useState(false)

  // Context menu for input
  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
  } | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // ── 就绪判定：复用既有代理上下文，不新建测量链路 ──
  const { proxies, sysproxy, refreshProxy } = useAppData()
  // refreshProxy 的引用不保证稳定，用 ref 持有，避免写进轮询副作用依赖后反复重排
  const refreshProxyRef = useRef(refreshProxy)
  useEffect(() => {
    refreshProxyRef.current = refreshProxy
  }, [refreshProxy])

  /** 活跃出口节点最近一次延迟（ms）；无数据返回 null */
  const activeDelay = useMemo(() => {
    const group = proxies.groups.find((g) => g.name === 'PROXY')
    const activeName = group?.now
    if (!activeName) return null
    const node =
      proxies.records[activeName] ??
      proxies.proxies.find((p) => p.name === activeName)
    const history = node?.history
    if (!history || history.length === 0) return null
    const delay = history[history.length - 1]?.delay
    return typeof delay === 'number' ? delay : null
  }, [proxies])

  /** 有效延迟（排除 0/超时与假节点阈值）才算真的连通 */
  const proxyReady =
    activeDelay !== null && activeDelay > 0 && activeDelay < NODE_DELAY_MAX_MS

  // 进入第二步开始计时：超时仍无有效延迟则放弃「已就绪」的表述
  useEffect(() => {
    if (!open || step !== 'ready') return
    const timer = setTimeout(() => setGaveUp(true), READY_TIMEOUT_MS)
    return () => clearTimeout(timer)
  }, [open, step])

  // 未就绪且未超时：周期刷新代理数据，等节点测出延迟
  useEffect(() => {
    if (!open || step !== 'ready' || proxyReady || gaveUp) return
    const timer = setInterval(() => {
      void refreshProxyRef.current?.()
    }, READY_POLL_INTERVAL_MS)
    return () => clearInterval(timer)
  }, [open, step, proxyReady, gaveUp])

  // 弹窗打开时尝试自动聚焦输入框
  useEffect(() => {
    if (!open) return
    const timer = setTimeout(() => {
      inputRef.current?.focus()
    }, 100)
    return () => clearTimeout(timer)
  }, [open])

  const resetForm = () => {
    // 重置回第一步：支持「帮助菜单 → 新手引导」重新打开时从导入步开始
    setStep('input')
    setUrl('')
    setSubmitting(false)
    setErrorMessage('')
    setGaveUp(false)
    setContextMenu(null)
  }

  /** 记录「已引导」标记：三个出口共用，避免重复写散落各处 */
  const markGuided = () => {
    localStorage.setItem(FIRST_RUN_GUIDE_STORAGE_KEY, 'true')
  }

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault()
    setContextMenu({
      mouseX: event.clientX,
      mouseY: event.clientY,
    })
  }

  const handlePaste = async () => {
    setContextMenu(null)
    let textToPaste = ''
    try {
      textToPaste = await readText()
    } catch {
      try {
        textToPaste = await navigator.clipboard.readText()
      } catch (e) {
        console.error('Failed to read clipboard:', e)
      }
    }

    if (!textToPaste) return
    setUrl(textToPaste.trim())
    setErrorMessage('')
  }

  const handleCopy = async () => {
    setContextMenu(null)
    if (!url) return
    try {
      await writeText(url)
    } catch {
      try {
        await navigator.clipboard.writeText(url)
      } catch (e) {
        console.error('Failed to copy:', e)
      }
    }
  }

  const handleCut = async () => {
    setContextMenu(null)
    if (!url) return
    await handleCopy()
    setUrl('')
  }

  const handleSelectAll = () => {
    setContextMenu(null)
    inputRef.current?.select()
  }

  const handleClear = () => {
    setContextMenu(null)
    setUrl('')
    setErrorMessage('')
    inputRef.current?.focus()
  }

  const handleSubmitImport = async () => {
    if (submitting) return
    const trimmed = url.trim()
    if (!trimmed) {
      setErrorMessage(t('layout.components.firstRun.errEmpty'))
      inputRef.current?.focus()
      return
    }

    if (!trimmed.startsWith('http://') && !trimmed.startsWith('https://')) {
      setErrorMessage(t('layout.components.firstRun.errScheme'))
      return
    }

    setSubmitting(true)
    setErrorMessage('')
    setGaveUp(false)

    try {
      const success = await onImport(trimmed)
      if (success) {
        setStep('ready')
      } else {
        setErrorMessage(t('layout.components.firstRun.errImport'))
      }
    } catch (err: unknown) {
      console.error('[FirstRunGuide] 导入异常:', err)
      setErrorMessage(t('layout.components.firstRun.errImport'))
    } finally {
      setSubmitting(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !submitting && step === 'input') {
      e.preventDefault()
      handleSubmitImport()
    }
  }

  const handleFinish = () => {
    markGuided()
    resetForm()
    onClose()
  }

  const handleOpenWikiAndFinish = () => {
    markGuided()
    resetForm()
    onOpenWiki()
    onClose()
  }

  const handleDismissLater = () => {
    markGuided()
    resetForm()
    onLater()
  }

  // 弹窗卡片底色来自 get3DCardStyle：retro-3d 皮肤为金黄渐变，其主题灰白
  // 次级文字（text.secondary/disabled）反差几乎为零，换用与卡片配套的深棕系
  // （#2C1F03 即 get3DCardStyle default 的 textColor）；其他皮肤沿用主题色
  const isRetro3D = getActiveSkin(theme) === 'retro-3d'
  const secondaryText = isRetro3D ? 'rgba(44, 31, 3, 0.78)' : 'text.secondary'
  const disabledText = isRetro3D ? 'rgba(44, 31, 3, 0.55)' : 'text.disabled'
  const readyColor = isRetro3D ? '#1B5E20' : 'success.main'
  const warningColor = isRetro3D ? '#8C4A03' : 'warning.main'

  const statusColor = proxyReady
    ? readyColor
    : gaveUp
      ? warningColor
      : secondaryText

  return (
    <>
      <Dialog
        open={open}
        onClose={() => {
          // 导入进行中：点遮罩、按 Esc（reason 为 escapeKeyDown）都不允许关闭，
          // 避免把正在进行的导入中途打断。
          if (submitting) return
          if (step === 'ready') {
            handleFinish()
          } else {
            handleDismissLater()
          }
        }}
        fullWidth
        maxWidth={false}
        slotProps={{
          paper: {
            sx: {
              ...get3DCardStyle(theme, 'default'),
              p: 0,
              overflow: 'hidden',
              width: 'calc(100% - 32px)',
              maxWidth: '430px',
              borderRadius: '10px',
            },
          },
        }}
      >
        {step === 'input' ? (
          <>
            <DialogTitle
              sx={{
                color: 'inherit',
                fontWeight: 'bold',
                fontSize: '16px',
                px: 2.5,
                py: 2,
                display: 'flex',
                alignItems: 'center',
                gap: 1,
                borderBottom: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <span>🎉</span>
              <span>{t('layout.components.firstRun.title')}</span>
            </DialogTitle>

            <DialogContent sx={{ px: 2.5, py: 2 }}>
              <Typography
                variant="body2"
                sx={{
                  color: secondaryText,
                  lineHeight: 1.6,
                  mb: 2,
                  fontSize: '13.5px',
                }}
              >
                {t('layout.components.firstRun.intro')}
              </Typography>

              <Box sx={{ mb: 1 }}>
                <TextField
                  autoFocus
                  fullWidth
                  size="small"
                  value={url}
                  disabled={submitting}
                  onChange={(e) => {
                    setUrl(e.target.value)
                    if (errorMessage) setErrorMessage('')
                  }}
                  onKeyDown={handleKeyDown}
                  placeholder="https://example.com/api/v1/client/subscribe?token=..."
                  sx={get3DInputStyle(theme)}
                  inputRef={inputRef}
                  onContextMenu={handleContextMenu}
                />
              </Box>

              {errorMessage && (
                <Typography
                  variant="caption"
                  sx={{
                    color: 'error.main',
                    fontWeight: 600,
                    display: 'block',
                    mt: 0.5,
                    fontSize: '12px',
                  }}
                >
                  ⚠️ {errorMessage}
                </Typography>
              )}

              <Box
                sx={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  mt: 1.5,
                }}
              >
                <Button
                  size="small"
                  variant="text"
                  onClick={handlePaste}
                  disabled={submitting}
                  sx={{
                    fontSize: '12px',
                    textTransform: 'none',
                    color: 'primary.main',
                    px: 0.5,
                  }}
                >
                  📋 {t('layout.components.firstRun.paste')}
                </Button>

                <Typography
                  variant="caption"
                  sx={{ color: disabledText, fontSize: '11px' }}
                >
                  {t('layout.components.firstRun.enterHint')}
                </Typography>
              </Box>
            </DialogContent>

            <DialogActions
              sx={{
                px: 2.5,
                py: 1.8,
                gap: 1,
                borderTop: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <Button
                sx={get3DButtonStyle(theme, 'outlined', 'default')}
                onClick={handleDismissLater}
                disabled={submitting}
              >
                {t('layout.components.firstRun.later')}
              </Button>
              <Button
                sx={get3DButtonStyle(theme, 'contained', 'primary')}
                onClick={handleSubmitImport}
                loading={submitting}
                disabled={submitting}
              >
                {t('layout.components.firstRun.importAndStart')}
              </Button>
            </DialogActions>
          </>
        ) : (
          <>
            <DialogTitle
              sx={{
                color: readyColor,
                fontWeight: 'bold',
                fontSize: '16px',
                px: 2.5,
                py: 2,
                display: 'flex',
                alignItems: 'center',
                gap: 1,
                borderBottom: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <span>🚀</span>
              <span>{t('layout.components.firstRun.readyTitle')}</span>
            </DialogTitle>

            <DialogContent sx={{ px: 2.5, py: 2 }}>
              <Box
                sx={{
                  p: 1.5,
                  borderRadius: '6px',
                  bgcolor: (t2) =>
                    t2.palette.mode === 'dark'
                      ? 'rgba(76, 175, 80, 0.12)'
                      : 'rgba(76, 175, 80, 0.08)',
                  border: '1px solid',
                  borderColor: (t2) =>
                    t2.palette.mode === 'dark'
                      ? 'rgba(76, 175, 80, 0.3)'
                      : 'rgba(76, 175, 80, 0.25)',
                  mb: 2,
                }}
              >
                <Typography
                  variant="body2"
                  sx={{
                    fontWeight: 600,
                    color: readyColor,
                    mb: 0.5,
                    fontSize: '14px',
                  }}
                >
                  ✅ {t('layout.components.firstRun.imported')}
                </Typography>

                {/* 就绪状态：测出有效延迟后才宣称连通，否则如实显示「检测中 / 未测出」 */}
                <Box
                  sx={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 0.75,
                    mt: 0.5,
                  }}
                >
                  {!proxyReady && !gaveUp && (
                    <CircularProgress
                      size={12}
                      sx={{ color: secondaryText }}
                    />
                  )}
                  <Typography
                    variant="body2"
                    sx={{
                      color: statusColor,
                      fontWeight: 600,
                      lineHeight: 1.5,
                      fontSize: '13px',
                    }}
                  >
                    {proxyReady
                      ? t('layout.components.firstRun.readyOk')
                      : gaveUp
                        ? t('layout.components.firstRun.readyUnconfirmed')
                        : t('layout.components.firstRun.checking')}
                  </Typography>
                </Box>

                {!sysproxy.enable && (
                  <Typography
                    variant="body2"
                    sx={{
                      color: warningColor,
                      fontWeight: 600,
                      mt: 0.75,
                      fontSize: '12.5px',
                      lineHeight: 1.5,
                    }}
                  >
                    ⚠️ {t('layout.components.firstRun.sysProxyOff')}
                  </Typography>
                )}
              </Box>

              <Typography
                variant="body2"
                sx={{
                  color: secondaryText,
                  lineHeight: 1.6,
                  fontSize: '13px',
                }}
              >
                {t('layout.components.firstRun.wikiHint')}
              </Typography>
            </DialogContent>

            <DialogActions
              sx={{
                px: 2.5,
                py: 1.8,
                gap: 1,
                borderTop: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <Button
                sx={get3DButtonStyle(theme, 'outlined', 'default')}
                onClick={handleFinish}
              >
                {t('layout.components.firstRun.finish')}
              </Button>
              <Button
                sx={get3DButtonStyle(theme, 'contained', 'primary')}
                onClick={handleOpenWikiAndFinish}
              >
                🌐 {t('layout.components.firstRun.openWiki')}
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>

      {/* Input Context Menu */}
      <Menu
        anchorReference="anchorPosition"
        anchorPosition={
          contextMenu !== null
            ? {
                top: contextMenu.mouseY,
                left: contextMenu.mouseX,
              }
            : undefined
        }
        open={contextMenu !== null}
        onClose={() => setContextMenu(null)}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              minWidth: '150px',
              borderRadius: '6px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              '& .MuiList-root': {
                padding: '4px 0',
              },
            },
          },
        }}
      >
        <MenuItem
          onClick={handleCut}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          ✂️ {t('shared.actions.cut')}
        </MenuItem>
        <MenuItem
          onClick={handleCopy}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📋 {t('shared.actions.copy')}
        </MenuItem>
        <MenuItem
          onClick={handlePaste}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📥 {t('shared.actions.paste')}
        </MenuItem>
        <MenuItem
          onClick={handleSelectAll}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🔍 {t('shared.actions.selectAll')}
        </MenuItem>
        <Divider sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }} />
        <MenuItem
          onClick={handleClear}
          disabled={!url}
          sx={{
            ...getMenuItemHoverStyle(theme, controlSkin),
            color: 'error.main',
          }}
        >
          🧹 {t('shared.actions.clearInput')}
        </MenuItem>
      </Menu>
    </>
  )
}
