import {
  Box,
  Button,
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
import React, { useState, useRef, useEffect } from 'react'

import {
  get3DCardStyle,
  get3DButtonStyle,
  get3DInputStyle,
} from '@/utils/button-styles'

import { getMenuItemHoverStyle } from '../utils/style-helpers'

export const CLASH_MINI_WIKI_URL =
  'https://github.com/qiu-yuxiao/clash-mini/wiki'
export const FIRST_RUN_GUIDE_STORAGE_KEY = 'clash-mini-first-run-completed'

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
  const theme = useTheme()
  const controlSkin = theme.controlSkin || 'default'

  const [step, setStep] = useState<'input' | 'ready'>('input')
  const [url, setUrl] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [errorMessage, setErrorMessage] = useState('')

  // Context menu for input
  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
  } | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // 弹窗打开时尝试自动聚焦输入框
  useEffect(() => {
    if (!open) return
    const timer = setTimeout(() => {
      inputRef.current?.focus()
    }, 100)
    return () => clearTimeout(timer)
  }, [open])

  const resetForm = () => {
    setStep('input')
    setUrl('')
    setSubmitting(false)
    setErrorMessage('')
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
    const trimmed = textToPaste.trim()
    setUrl(trimmed)
    setErrorMessage('')
  }

  const handleCopy = async () => {
    setContextMenu(null)
    if (url) {
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
  }

  const handleCut = async () => {
    setContextMenu(null)
    if (url) {
      await handleCopy()
      setUrl('')
    }
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
    const trimmed = url.trim()
    if (!trimmed) {
      setErrorMessage('请输入或粘贴订阅链接')
      inputRef.current?.focus()
      return
    }

    if (!trimmed.startsWith('http://') && !trimmed.startsWith('https://')) {
      setErrorMessage('订阅链接必须以 http:// 或 https:// 开头')
      return
    }

    setSubmitting(true)
    setErrorMessage('')

    try {
      const success = await onImport(trimmed)
      if (success) {
        // 成功后平滑切到第二步：网络就绪与浏览器测试
        setStep('ready')
      } else {
        setErrorMessage('订阅导入失败，请检查链接是否有效后重试')
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      setErrorMessage(`导入失败: ${msg}`)
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
    localStorage.setItem(FIRST_RUN_GUIDE_STORAGE_KEY, 'true')
    resetForm()
    onClose()
  }

  const handleOpenWikiAndFinish = () => {
    localStorage.setItem(FIRST_RUN_GUIDE_STORAGE_KEY, 'true')
    resetForm()
    onOpenWiki()
    onClose()
  }

  const handleDismissLater = () => {
    localStorage.setItem(FIRST_RUN_GUIDE_STORAGE_KEY, 'true')
    resetForm()
    onLater()
  }

  return (
    <>
      <Dialog
        open={open}
        onClose={(_event, reason) => {
          // 禁止点击遮罩误关，防止正在导入时中断
          if (reason === 'backdropClick' && submitting) return
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
                borderBottom: (t) => `1px solid ${t.palette.divider}`,
              }}
            >
              <span>🎉</span>
              <span>欢迎使用 Clash Mini</span>
            </DialogTitle>

            <DialogContent sx={{ px: 2.5, py: 2 }}>
              <Typography
                variant="body2"
                sx={{
                  color: 'text.secondary',
                  lineHeight: 1.6,
                  mb: 2,
                  fontSize: '13.5px',
                }}
              >
                检测到您是首次使用，尚未添加订阅配置。请在下方填入您的代理订阅链接，我们将自动为您完成节点解析、测速与网络接管：
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
                  📋 一键粘贴剪贴板链接
                </Button>

                <Typography
                  variant="caption"
                  sx={{ color: 'text.disabled', fontSize: '11px' }}
                >
                  按回车键立即导入
                </Typography>
              </Box>
            </DialogContent>

            <DialogActions
              sx={{
                px: 2.5,
                py: 1.8,
                gap: 1,
                borderTop: (t) => `1px solid ${t.palette.divider}`,
              }}
            >
              <Button
                sx={get3DButtonStyle(theme, 'outlined', 'default')}
                onClick={handleDismissLater}
                disabled={submitting}
              >
                稍后手动配置
              </Button>
              <Button
                sx={get3DButtonStyle(theme, 'contained', 'primary')}
                onClick={handleSubmitImport}
                loading={submitting}
                disabled={submitting}
              >
                立即导入并开启
              </Button>
            </DialogActions>
          </>
        ) : (
          <>
            <DialogTitle
              sx={{
                color: 'success.main',
                fontWeight: 'bold',
                fontSize: '16px',
                px: 2.5,
                py: 2,
                display: 'flex',
                alignItems: 'center',
                gap: 1,
                borderBottom: (t) => `1px solid ${t.palette.divider}`,
              }}
            >
              <span>🚀</span>
              <span>代理已就绪，网络畅通！</span>
            </DialogTitle>

            <DialogContent sx={{ px: 2.5, py: 2 }}>
              <Box
                sx={{
                  p: 1.5,
                  borderRadius: '6px',
                  bgcolor: (t) =>
                    t.palette.mode === 'dark'
                      ? 'rgba(76, 175, 80, 0.12)'
                      : 'rgba(76, 175, 80, 0.08)',
                  border: '1px solid',
                  borderColor: (t) =>
                    t.palette.mode === 'dark'
                      ? 'rgba(76, 175, 80, 0.3)'
                      : 'rgba(76, 175, 80, 0.25)',
                  mb: 2,
                }}
              >
                <Typography
                  variant="body2"
                  sx={{
                    fontWeight: 600,
                    color: 'success.main',
                    mb: 0.5,
                    fontSize: '14px',
                  }}
                >
                  ✅ 订阅配置已成功导入并激活！
                </Typography>
                <Typography
                  variant="body2"
                  sx={{
                    color: 'text.primary',
                    lineHeight: 1.6,
                    fontSize: '13px',
                  }}
                >
                  系统代理与 TUN
                  内核已全量就绪。您的浏览器（Edge、Chrome、Firefox
                  等）现在已经可以直接顺畅访问全球网络。
                </Typography>
              </Box>

              <Typography
                variant="body2"
                sx={{
                  color: 'text.secondary',
                  lineHeight: 1.6,
                  fontSize: '13px',
                }}
              >
                您可以点击下方按钮，直接在默认浏览器中打开 Clash Mini 项目 Wiki
                首页，亲测网络连通性并了解更多进阶技巧：
              </Typography>
            </DialogContent>

            <DialogActions
              sx={{
                px: 2.5,
                py: 1.8,
                gap: 1,
                borderTop: (t) => `1px solid ${t.palette.divider}`,
              }}
            >
              <Button
                sx={get3DButtonStyle(theme, 'outlined', 'default')}
                onClick={handleFinish}
              >
                完成并进入主界面
              </Button>
              <Button
                sx={get3DButtonStyle(theme, 'contained', 'primary')}
                onClick={handleOpenWikiAndFinish}
              >
                🌐 打开 Wiki 首页测试
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
          ✂️ 剪切
        </MenuItem>
        <MenuItem
          onClick={handleCopy}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📋 复制
        </MenuItem>
        <MenuItem
          onClick={handlePaste}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📥 粘贴
        </MenuItem>
        <MenuItem
          onClick={handleSelectAll}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🔍 全选
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
          🧹 清空
        </MenuItem>
      </Menu>
    </>
  )
}
