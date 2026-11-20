import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
  useTheme,
} from '@mui/material'
import type { Update } from '@tauri-apps/plugin-updater'
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

import { isSameVersion } from '../../utils/style-helpers'

interface ClientUpdateDialogProps {
  open: boolean
  onClose: () => void
  clientStatus: string
  appVersion: string
  clientUpdateObj: Update | null
  clientProgress: number
  clientProgressMessage: string
  handleClientUpgrade: () => void
}

export const ClientUpdateDialog: React.FC<ClientUpdateDialogProps> = ({
  open,
  onClose,
  clientStatus,
  appVersion,
  clientUpdateObj,
  clientProgress,
  clientProgressMessage,
  handleClientUpgrade,
}) => {
  const theme = useTheme()
  const { t } = useTranslation()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  return (
    <Dialog
      open={open}
      onClose={() => {
        if (clientStatus !== 'downloading') {
          onClose()
        }
      }}
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
          <span>🚀</span>
          <span>{t('layout.components.dialogs.client.title')}</span>
        </Box>
      </DialogTitle>

      <DialogContent sx={getPopupContentSx()}>
        <Box sx={{ mb: 1.25 }}>
          <Typography
            variant="body2"
            sx={{
              mb: 0.5,
              fontSize: '12.5px',
              color: tokens.secondaryText,
              fontFamily: 'var(--control-font-family)',
            }}
          >
            {t('layout.components.dialogs.update.currentVersion', {
              version: appVersion
                ? `v${appVersion}`
                : t('layout.components.dialogs.update.unknown'),
            })}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              fontSize: '12.5px',
              fontWeight: 'bold',
              color: tokens.primaryText,
              fontFamily: 'var(--control-font-family)',
            }}
          >
            {t('layout.components.dialogs.update.latestVersion', {
              version: clientUpdateObj?.version
                ? `v${clientUpdateObj.version}`
                : t('layout.components.dialogs.update.unknown'),
            })}
          </Typography>
        </Box>

        <Typography
          variant="body2"
          sx={{
            fontWeight: 'bold',
            mb: 0.75,
            fontSize: '12.5px',
            color: tokens.primaryText,
            fontFamily: 'var(--control-font-family)',
          }}
        >
          {t('layout.components.dialogs.client.changelog')}
        </Typography>
        <Box
          sx={{
            maxHeight: '160px',
            overflowY: 'auto',
            p: 1.25,
            borderRadius: '6px',
            fontSize: '12px',
            lineHeight: 1.5,
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            fontFamily: 'var(--control-font-family)',
            bgcolor: tokens.subtleSurfaceBg,
            border: `1px solid ${tokens.subtleSurfaceBorder}`,
            color: tokens.primaryText,
          }}
        >
          {clientUpdateObj?.body ||
            t('layout.components.dialogs.client.changelogEmpty')}
        </Box>

        {clientStatus !== 'idle' && (
          <Box sx={{ width: '100%', mt: 1.5 }}>
            <Box
              sx={{
                width: '100%',
                height: '8px',
                bgcolor: tokens.subtleSurfaceBg,
                border: `1px solid ${tokens.subtleSurfaceBorder}`,
                borderRadius: '4px',
                overflow: 'hidden',
              }}
            >
              <Box
                sx={{
                  width: `${clientProgress}%`,
                  height: '100%',
                  background: 'var(--primary-main)',
                  transition: 'width 0.2s ease',
                }}
              />
            </Box>
            <Typography
              variant="body2"
              sx={{
                mt: 0.75,
                fontSize: '12px',
                color: tokens.secondaryText,
                wordBreak: 'break-word',
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {clientProgressMessage}
            </Typography>
          </Box>
        )}
      </DialogContent>

      <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
        <Button
          fullWidth
          onClick={handleClientUpgrade}
          disabled={
            clientStatus === 'downloading' ||
            clientStatus === 'done' ||
            Boolean(
              clientUpdateObj &&
                isSameVersion(appVersion, clientUpdateObj.version),
            )
          }
          sx={{
            ...get3DButtonStyle(theme, 'contained', 'primary'),
            py: 0.85,
            fontSize: '13px',
          }}
        >
          {clientStatus === 'done'
            ? t('layout.components.dialogs.client.prepareRestart')
            : clientStatus === 'downloading'
              ? t('layout.components.dialogs.update.updating')
              : clientUpdateObj &&
                  isSameVersion(appVersion, clientUpdateObj.version)
                ? t('layout.components.dialogs.update.upToDate')
                : t('layout.components.dialogs.update.updateNow')}
        </Button>
        <Button
          fullWidth
          onClick={onClose}
          disabled={clientStatus === 'downloading'}
          sx={{
            ...get3DButtonStyle(theme, 'outlined', 'default'),
            py: 0.7,
            fontSize: '12.5px',
          }}
        >
          {t('shared.actions.close')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
