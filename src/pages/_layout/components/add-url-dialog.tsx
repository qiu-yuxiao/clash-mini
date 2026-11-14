import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  Menu,
  MenuItem,
  TextField,
  useTheme,
} from '@mui/material'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import React, { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { showNotice } from '@/services/notice-service'
import { get3DButtonStyle, get3DInputStyle } from '@/utils/button-styles'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupMenuItemStyle,
  getPopupMenuPaperProps,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'
import { addQuickRoutingRules } from '@/utils/quick-routing'

interface AddUrlDialogProps {
  open: boolean
  onClose: () => void
}

export const AddUrlDialog: React.FC<AddUrlDialogProps> = ({
  open,
  onClose,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const topOffset = useNarrowOverlayTopOffset()
  const [text, setText] = useState('')
  const [submitting, setSubmitting] = useState(false)

  // Context Menu State for Input Box (Mouse Paste Bug Fix)
  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
    hasSelection: boolean
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
    } catch (e: unknown) {
      showNotice.error(e instanceof Error ? e.message : String(e))
    } finally {
      setSubmitting(false)
    }
  }

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault()
    const input = inputRef.current
    const selectionStart = input?.selectionStart ?? 0
    const selectionEnd = input?.selectionEnd ?? 0
    setContextMenu({
      mouseX: event.clientX,
      mouseY: event.clientY,
      hasSelection: selectionStart !== selectionEnd,
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
      const newValue =
        text.substring(0, start) + textToPaste + text.substring(end)
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

  const hasSelection = Boolean(contextMenu?.hasSelection)

  return (
    <>
      <Dialog
        open={open}
        onClose={handleClose}
        maxWidth={false}
        sx={getNarrowDialogSx(topOffset)}
        slotProps={getNarrowDialogSlotProps(theme, topOffset)}
      >
        <DialogTitle sx={getPopupTitleSx(theme)}>
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 0.75,
              fontWeight: 'bold',
              fontSize: '15.5px',
              lineHeight: 1.35,
            }}
          >
            <span>🌐</span>
            <span>
              {t('settings.mini.addUrl', { defaultValue: '添加网址' })}
            </span>
          </Box>
        </DialogTitle>

        <DialogContent sx={getPopupContentSx()}>
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

        <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
          <Button
            fullWidth
            sx={{
              ...get3DButtonStyle(theme, 'contained', 'primary'),
              py: 0.85,
              fontSize: '13px',
            }}
            onClick={handleAdd}
            disabled={submitting}
          >
            {t('settings.mini.addUrlButton', { defaultValue: '添加' })}
          </Button>
          <Button
            fullWidth
            sx={{
              ...get3DButtonStyle(theme, 'outlined', 'default'),
              py: 0.7,
              fontSize: '12.5px',
            }}
            onClick={handleClose}
            disabled={submitting}
          >
            {t('shared.actions.cancel', { defaultValue: '取消' })}
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
          paper: getPopupMenuPaperProps(theme, { minWidth: '160px' }),
        }}
      >
        <MenuItem onClick={handleCut} disabled={!hasSelection}>
          ✂️ {t('shared.actions.cut')}
        </MenuItem>
        <MenuItem onClick={handleCopy} disabled={!hasSelection}>
          📋 {t('shared.actions.copy')}
        </MenuItem>
        <MenuItem onClick={handlePaste}>
          📥 {t('shared.actions.paste')}
        </MenuItem>
        <MenuItem onClick={handleSelectAll} disabled={!text}>
          🔍 {t('shared.actions.selectAll')}
        </MenuItem>
        <Divider />
        <MenuItem
          onClick={handleClear}
          disabled={!text}
          sx={getPopupMenuItemStyle(theme, { danger: true })}
        >
          🧹 {t('shared.actions.clearInput')}
        </MenuItem>
      </Menu>
    </>
  )
}
