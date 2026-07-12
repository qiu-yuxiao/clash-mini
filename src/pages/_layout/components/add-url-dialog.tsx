import { Button, Dialog, DialogActions, DialogContent, DialogTitle, TextField, useTheme, Menu, MenuItem, Divider } from '@mui/material'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import React, { useState, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { showNotice } from '@/services/notice-service'
import {
  get3DCardStyle,
  get3DButtonStyle,
  get3DInputStyle,
} from '@/utils/button-styles'
import { addQuickRoutingRules } from '@/utils/quick-routing'

import { getMenuItemHoverStyle } from '../utils/style-helpers'

interface AddUrlDialogProps {
  open: boolean
  onClose: () => void
}

export const AddUrlDialog: React.FC<AddUrlDialogProps> = ({ open, onClose }) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const [text, setText] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const controlSkin = (theme as any).controlSkin || 'default'

  // Context Menu State for Input Box (Mouse Paste Bug Fix)
  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
  } | null>(null)
  const inputRef = useRef<HTMLTextAreaElement | HTMLInputElement>(null)

  const handleClose = () => {
    if (submitting) return
    setText('')
    onClose()
  }

  const handleAdd = async () => {
    if (!text.trim()) {
      handleClose()
      return
    }
    setSubmitting(true)
    try {
      const count = await addQuickRoutingRules(text)
      if (count > 0) {
        showNotice.success(t('settings.mini.addUrlSuccess', { count }))
      } else {
        showNotice.info(
          t('settings.mini.addUrlNoChange', {
            defaultValue: '没有新增规则（可能已存在或输入无效）',
          }),
        )
      }
      setText('')
      onClose()
    } catch (e: any) {
      showNotice.error(e?.message || String(e))
    } finally {
      setSubmitting(false)
    }
  }

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault()
    setContextMenu({
      mouseX: event.clientX,
      mouseY: event.clientY,
    })
  }

  const handleCut = async () => {
    setContextMenu(null)
    const input = inputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const selectedText = text.substring(start, end)
      if (selectedText) {
        try {
          await writeText(selectedText)
        } catch {
          try {
            await navigator.clipboard.writeText(selectedText)
          } catch (e) {
            console.error('Failed to copy:', e)
          }
        }
        const newValue = text.substring(0, start) + text.substring(end)
        setText(newValue)
        setTimeout(() => {
          input.focus()
          input.setSelectionRange(start, start)
        }, 0)
      }
    }
  }

  const handleCopy = async () => {
    setContextMenu(null)
    const input = inputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const selectedText = text.substring(start, end)
      if (selectedText) {
        try {
          await writeText(selectedText)
        } catch {
          try {
            await navigator.clipboard.writeText(selectedText)
          } catch (e) {
            console.error('Failed to copy:', e)
          }
        }
      }
    }
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
        console.error('Failed to read from clipboard:', e)
      }
    }

    if (!textToPaste) return

    const input = inputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const newValue = text.substring(0, start) + textToPaste + text.substring(end)
      setText(newValue)
      setTimeout(() => {
        input.focus()
        const newCursorPos = start + textToPaste.length
        input.setSelectionRange(newCursorPos, newCursorPos)
      }, 0)
    } else {
      setText(text + textToPaste)
    }
  }

  const handleSelectAll = () => {
    setContextMenu(null)
    const input = inputRef.current
    if (input) {
      input.focus()
      input.setSelectionRange(0, text.length)
    }
  }

  const handleClear = () => {
    setContextMenu(null)
    setText('')
    const input = inputRef.current
    if (input) {
      input.focus()
    }
  }

  return (
    <>
      <Dialog
        open={open}
        onClose={handleClose}
        slotProps={{
          paper: {
            sx: {
              ...get3DCardStyle(theme, 'default'),
              p: 0,
              overflow: 'hidden',
              width: 'calc(100% - 32px)',
              maxWidth: '380px',
            },
          },
        }}
      >
        <DialogTitle
          sx={{
            color: 'inherit',
            fontWeight: 'bold',
            fontSize: '15px',
            px: 2,
            py: 1.5,
          }}
        >
          {t('settings.mini.addUrl', { defaultValue: '添加网址' })}
        </DialogTitle>
        <DialogContent sx={{ px: 2, pb: 1 }}>
          <TextField
            autoFocus
            multiline
            minRows={4}
            maxRows={10}
            fullWidth
            value={text}
            disabled={submitting}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('settings.mini.addUrlPlaceholder', {
              defaultValue: '请填入需要代理的网址（每行一个）',
            })}
            sx={get3DInputStyle(theme)}
            inputRef={inputRef}
            onContextMenu={handleContextMenu}
          />
        </DialogContent>
        <DialogActions sx={{ px: 2, py: 1.5, gap: 1 }}>
          <Button
            sx={get3DButtonStyle(theme, 'outlined', 'default')}
            onClick={handleClose}
            disabled={submitting}
          >
            {t('shared.actions.cancel', { defaultValue: '取消' })}
          </Button>
          <Button
            sx={get3DButtonStyle(theme, 'contained', 'primary')}
            onClick={handleAdd}
            disabled={submitting}
          >
            {t('settings.mini.addUrlButton', { defaultValue: '添加' })}
          </Button>
        </DialogActions>
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
              minWidth: '160px',
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
          disabled={
            !inputRef.current ||
            inputRef.current.selectionStart ===
              inputRef.current.selectionEnd
          }
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          ✂️ 剪切
        </MenuItem>
        <MenuItem
          onClick={handleCopy}
          disabled={
            !inputRef.current ||
            inputRef.current.selectionStart ===
              inputRef.current.selectionEnd
          }
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
          disabled={!text}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🔍 全选
        </MenuItem>
        <Divider
          sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }}
        />
        <MenuItem
          onClick={handleClear}
          disabled={!text}
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
