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
import type React from 'react'
import { useTranslation } from 'react-i18next'

import type { GithubRelease } from '@/pages/_layout/types'
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

import { formatCoreVersion, isSameVersion } from '../../utils/style-helpers'

interface CoreUpdateDialogProps {
  open: boolean
  onClose: () => void
  coreUpgradeStatus: string
  coreVersion: string | undefined
  coreUpdateRelease: GithubRelease | null
  coreUpgradeProgress: number
  coreUpgradeMessage: string
  handleCoreUpgrade: () => void
}

export const CoreUpdateDialog: React.FC<CoreUpdateDialogProps> = ({
  open,
  onClose,
  coreUpgradeStatus,
  coreVersion,
  coreUpdateRelease,
  coreUpgradeProgress,
  coreUpgradeMessage,
  handleCoreUpgrade,
}) => {
  const theme = useTheme()
  const { t } = useTranslation()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  return (
    <Dialog
      open={open}
      onClose={() => {
        if (
          coreUpgradeStatus !== 'checking' &&
          coreUpgradeStatus !== 'downloading' &&
          coreUpgradeStatus !== 'extracting'
        ) {
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
          <span>⚙️</span>
          <span>{t('layout.components.dialogs.core.title')}</span>
        </Box>
      </DialogTitle>

      <DialogContent sx={getPopupContentSx()}>
        <Box
          sx={{
            p: 1.25,
            borderRadius: '6px',
            bgcolor: tokens.subtleSurfaceBg,
            border: `1px solid ${tokens.subtleSurfaceBorder}`,
          }}
        >
          <Typography
            variant="body2"
            sx={{
              mb: 0.6,
              fontSize: '12.5px',
              color: tokens.secondaryText,
              fontFamily: 'var(--control-font-family)',
            }}
          >
            {t('layout.components.dialogs.update.currentVersion', {
              version: coreVersion
                ? formatCoreVersion(coreVersion)
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
              version: coreUpdateRelease?.tag_name
                ? formatCoreVersion(coreUpdateRelease.tag_name)
                : t('layout.components.dialogs.core.fetching'),
            })}
          </Typography>
        </Box>

        {coreUpgradeStatus !== 'idle' && (
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
                  width: `${coreUpgradeProgress}%`,
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
              {coreUpgradeMessage}
            </Typography>
          </Box>
        )}
      </DialogContent>

      <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
        <Button
          fullWidth
          onClick={handleCoreUpgrade}
          disabled={
            coreUpgradeStatus === 'checking' ||
            coreUpgradeStatus === 'downloading' ||
            coreUpgradeStatus === 'extracting' ||
            coreUpgradeStatus === 'done' ||
            isSameVersion(coreVersion, coreUpdateRelease?.tag_name)
          }
          sx={{
            ...get3DButtonStyle(theme, 'contained', 'primary'),
            py: 0.85,
            fontSize: '13px',
          }}
        >
          {coreUpgradeStatus === 'done'
            ? t('layout.components.dialogs.core.updateDone')
            : coreUpgradeStatus !== 'idle'
              ? t('layout.components.dialogs.update.updating')
              : isSameVersion(coreVersion, coreUpdateRelease?.tag_name)
                ? t('layout.components.dialogs.update.upToDate')
                : t('layout.components.dialogs.update.updateNow')}
        </Button>
        <Button
          fullWidth
          onClick={onClose}
          disabled={
            coreUpgradeStatus === 'checking' ||
            coreUpgradeStatus === 'downloading' ||
            coreUpgradeStatus === 'extracting'
          }
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
