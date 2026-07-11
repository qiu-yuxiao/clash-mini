import { Button, Dialog, DialogActions, DialogContent, DialogTitle, TextField, useTheme } from '@mui/material'
import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  get3DCardStyle,
  get3DButtonStyle,
  get3DInputStyle,
} from '@/utils/button-styles'
import { addQuickRoutingRules } from '@/utils/quick-routing'
import { showNotice } from '@/services/notice-service'

interface AddUrlDialogProps {
  open: boolean
  onClose: () => void
}

export const AddUrlDialog: React.FC<AddUrlDialogProps> = ({ open, onClose }) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const [text, setText] = useState('')
  const [submitting, setSubmitting] = useState(false)

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

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      slotProps={{
        paper: {
          sx: {
            ...get3DCardStyle(theme, 'default'),
            p: 0,
            overflow: 'hidden',
            minWidth: 360,
            maxWidth: '92vw',
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
  )
}
