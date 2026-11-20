import { CloseRounded } from '@mui/icons-material'
import { Box, Dialog, IconButton, useTheme } from '@mui/material'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import LogsPage from '@/pages/logs'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupSkinTokens,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

interface LogsDialogProps {
  open: boolean
  onClose: () => void
}

export const LogsDialog: React.FC<LogsDialogProps> = ({ open, onClose }) => {
  const theme = useTheme()
  const { t } = useTranslation()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  return (
    <Dialog
      open={open}
      onClose={onClose}
      fullWidth={false}
      maxWidth={false}
      sx={getNarrowDialogSx(topOffset)}
      slotProps={getNarrowDialogSlotProps(theme, topOffset, {
        paperSx: {
          height: '480px',
          position: 'relative',
          '& .base-page > header': {
            pr: 5,
            pl: 1.5,
            flex: '0 0 42px',
            borderBottom: `1px solid ${tokens.dividerColor}`,
            color: tokens.primaryText,
          },
          '& .base-page > header .MuiTypography-root': {
            fontSize: '14.5px',
            fontWeight: 700,
            color: tokens.primaryText,
          },
          '& .base-container': {
            color: theme.palette.text.primary,
          },
        },
      })}
    >
      <IconButton
        size="small"
        aria-label={t('layout.a11y.closeLogsDialog')}
        onClick={onClose}
        sx={{
          position: 'absolute',
          top: 8,
          right: 8,
          zIndex: 10,
          color: tokens.secondaryText,
        }}
      >
        <CloseRounded aria-hidden="true" fontSize="small" />
      </IconButton>
      <Box sx={{ flex: 1, minHeight: 0, overflow: 'hidden' }}>
        {open && <LogsPage />}
      </Box>
    </Dialog>
  )
}
