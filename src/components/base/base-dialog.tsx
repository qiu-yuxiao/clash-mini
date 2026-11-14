import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  useTheme,
  type SxProps,
  type Theme,
} from '@mui/material'
import { ReactNode } from 'react'

import { get3DButtonStyle } from '@/utils/button-styles'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

interface Props {
  title: ReactNode
  open: boolean
  okBtn?: ReactNode
  cancelBtn?: ReactNode
  disableEnforceFocus?: boolean
  disableOk?: boolean
  disableCancel?: boolean
  disableFooter?: boolean
  contentSx?: SxProps<Theme>
  children?: ReactNode
  loading?: boolean
  onOk?: () => void
  onCancel?: () => void
  onClose?: () => void
}

export const BaseDialog: React.FC<Props> = ({
  open,
  title,
  children,
  okBtn,
  cancelBtn,
  disableEnforceFocus,
  contentSx,
  disableCancel,
  disableOk,
  disableFooter,
  loading,
  onOk,
  onCancel,
  onClose,
}) => {
  const theme = useTheme()
  const topOffset = useNarrowOverlayTopOffset()

  return (
    <Dialog
      open={open}
      onClose={onClose}
      disableEnforceFocus={disableEnforceFocus}
      maxWidth={false}
      sx={getNarrowDialogSx(topOffset)}
      slotProps={getNarrowDialogSlotProps(theme, topOffset)}
    >
      <DialogTitle sx={getPopupTitleSx(theme)}>
        <Box sx={{ fontWeight: 'bold', fontSize: '15.5px', lineHeight: 1.35 }}>
          {title}
        </Box>
      </DialogTitle>

      <DialogContent
        sx={[
          getPopupContentSx(),
          ...(Array.isArray(contentSx)
            ? contentSx
            : contentSx
              ? [contentSx]
              : []),
        ]}
      >
        {children}
      </DialogContent>

      {!disableFooter && (
        <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
          {!disableOk && (
            <Button
              fullWidth
              loading={loading}
              sx={{
                ...get3DButtonStyle(theme, 'contained', 'primary'),
                py: 0.85,
                fontSize: '13px',
              }}
              onClick={onOk}
            >
              {okBtn}
            </Button>
          )}
          {!disableCancel && (
            <Button
              fullWidth
              sx={{
                ...get3DButtonStyle(theme, 'outlined', 'default'),
                py: 0.7,
                fontSize: '12.5px',
              }}
              onClick={onCancel}
            >
              {cancelBtn}
            </Button>
          )}
        </DialogActions>
      )}
    </Dialog>
  )
}
