import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
  useTheme,
  type SxProps,
  type Theme,
} from '@mui/material'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import { get3DButtonStyle } from '@/utils/button-styles'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupSkinTokens,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

interface DeleteProfileDialogProps {
  deleteConfirmProfile: { uid: string; name: string } | null
  onCancel: () => void
  onConfirm: () => void
  defaultBtn3DStyle: SxProps<Theme>
}

export const DeleteProfileDialog: React.FC<DeleteProfileDialogProps> = ({
  deleteConfirmProfile,
  onCancel,
  onConfirm,
  defaultBtn3DStyle,
}) => {
  const theme = useTheme()
  const { t } = useTranslation()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  return (
    <Dialog
      open={Boolean(deleteConfirmProfile)}
      onClose={onCancel}
      fullWidth={false}
      maxWidth={false}
      sx={getNarrowDialogSx(topOffset)}
      slotProps={getNarrowDialogSlotProps(theme, topOffset)}
    >
      <DialogTitle sx={getPopupTitleSx(theme)}>
        {t('profiles.modals.confirmDelete.title')}
      </DialogTitle>
      <DialogContent sx={getPopupContentSx()}>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          {deleteConfirmProfile?.name && (
            <Typography
              variant="body2"
              sx={{
                fontWeight: 700,
                color: tokens.primaryText,
                wordBreak: 'break-all',
              }}
            >
              {deleteConfirmProfile.name}
            </Typography>
          )}
          <Typography variant="body2" sx={{ color: tokens.secondaryText }}>
            {t('profiles.modals.confirmDelete.message')}
          </Typography>
        </Box>
      </DialogContent>
      <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
        <Button
          fullWidth
          onClick={onCancel}
          sx={{
            ...(defaultBtn3DStyle as Record<string, unknown>),
            py: 0.7,
            fontSize: '12.5px',
          }}
        >
          {t('shared.actions.cancel')}
        </Button>
        <Button
          fullWidth
          onClick={onConfirm}
          sx={{
            ...get3DButtonStyle(theme, 'contained', 'error'),
            py: 0.7,
            fontSize: '12.5px',
          }}
        >
          {t('shared.actions.delete')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
