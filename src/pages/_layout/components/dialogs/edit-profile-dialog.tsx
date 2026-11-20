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
  type SxProps,
  type Theme,
} from '@mui/material'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import { get3DInputStyle } from '@/utils/button-styles'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupSkinTokens,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

interface EditProfileDialogProps {
  open: boolean
  onClose: () => void
  editProfileName: string
  setEditProfileName: (name: string) => void
  editProfileUrl: string
  setEditProfileUrl: (url: string) => void
  editProfileInterval: number
  setEditProfileInterval: (val: number) => void
  isEditProfileLocal: boolean
  handleSaveProfile: () => void
  defaultBtn3DStyle: SxProps<Theme>
  primaryBtn3DStyle: SxProps<Theme>
}

export const EditProfileDialog: React.FC<EditProfileDialogProps> = ({
  open,
  onClose,
  editProfileName,
  setEditProfileName,
  editProfileUrl,
  setEditProfileUrl,
  editProfileInterval,
  setEditProfileInterval,
  isEditProfileLocal,
  handleSaveProfile,
  defaultBtn3DStyle,
  primaryBtn3DStyle,
}) => {
  const theme = useTheme()
  const { t } = useTranslation()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  return (
    <Dialog
      open={open}
      onClose={onClose}
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
          <span>⚙️</span>
          <span>{t('layout.components.dialogs.editProfile.title')}</span>
        </Box>
      </DialogTitle>

      <DialogContent sx={getPopupContentSx()}>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          <Box>
            <Typography
              id="profile-name-label"
              variant="body2"
              sx={{
                mb: 0.5,
                fontWeight: 'bold',
                fontSize: '12.5px',
                color: tokens.primaryText,
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {t('layout.components.dialogs.editProfile.nameLabel')}
            </Typography>
            <TextField
              aria-labelledby="profile-name-label"
              fullWidth
              size="small"
              value={editProfileName}
              onChange={(e) => setEditProfileName(e.target.value)}
              sx={get3DInputStyle(theme)}
            />
          </Box>

          <Box>
            <Typography
              id="profile-url-label"
              variant="body2"
              sx={{
                mb: 0.5,
                fontWeight: 'bold',
                fontSize: '12.5px',
                color: tokens.primaryText,
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {t('layout.components.dialogs.editProfile.urlLabel')}
            </Typography>
            <TextField
              aria-labelledby="profile-url-label"
              fullWidth
              size="small"
              value={editProfileUrl}
              disabled={isEditProfileLocal}
              onChange={(e) => setEditProfileUrl(e.target.value)}
              sx={get3DInputStyle(theme)}
            />
          </Box>

          <Box>
            <Typography
              id="profile-interval-label"
              variant="body2"
              sx={{
                mb: 0.5,
                fontWeight: 'bold',
                fontSize: '12.5px',
                color: tokens.primaryText,
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {t('layout.components.dialogs.editProfile.intervalLabel')}
            </Typography>
            <TextField
              aria-labelledby="profile-interval-label"
              fullWidth
              size="small"
              type="number"
              value={editProfileInterval}
              disabled={isEditProfileLocal}
              onChange={(e) => setEditProfileInterval(Number(e.target.value))}
              sx={get3DInputStyle(theme)}
            />
          </Box>
        </Box>
      </DialogContent>

      <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
        <Button
          fullWidth
          onClick={handleSaveProfile}
          sx={{
            ...primaryBtn3DStyle,
            py: 0.85,
            fontSize: '13px',
          }}
        >
          {t('shared.actions.save')}
        </Button>
        <Button
          fullWidth
          onClick={onClose}
          sx={{
            ...defaultBtn3DStyle,
            py: 0.7,
            fontSize: '12.5px',
          }}
        >
          {t('shared.actions.cancel')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
