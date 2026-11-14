import { CloseRounded } from '@mui/icons-material'
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  TextField,
  Typography,
  useTheme,
  type SxProps,
  type Theme,
} from '@mui/material'
import { type Update } from '@tauri-apps/plugin-updater'
import React from 'react'
import { useTranslation } from 'react-i18next'

import { get3DButtonStyle, get3DInputStyle } from '@/utils/button-styles'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupSkinTokens,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

import LogsPage from '../../logs'
import { formatCoreVersion, isSameVersion } from '../utils/style-helpers'

interface GithubAsset {
  name: string
  browser_download_url: string
}

interface GithubRelease {
  tag_name: string
  assets: GithubAsset[]
}

interface LayoutDialogsProps {
  // Edit Profile Dialog
  editProfileOpen: boolean
  setEditProfileOpen: (open: boolean) => void
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

  // Client Update Dialog
  clientUpdateOpen: boolean
  setClientUpdateOpen: (open: boolean) => void
  clientStatus: string
  appVersion: string
  clientUpdateObj: Update | null
  clientProgress: number
  clientProgressMessage: string
  handleClientUpgrade: () => void

  // Core Update Dialog
  coreUpdateOpen: boolean
  setCoreUpdateOpen: (open: boolean) => void
  coreUpgradeStatus: string
  coreVersion: string | undefined
  coreUpdateRelease: GithubRelease | null
  coreUpgradeProgress: number
  coreUpgradeMessage: string
  handleCoreUpgrade: () => void

  // Logs View Dialog
  logsOpen: boolean
  setLogsOpen: (open: boolean) => void
  mode: 'light' | 'dark'
}

export const LayoutDialogs: React.FC<LayoutDialogsProps> = ({
  editProfileOpen,
  setEditProfileOpen,
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

  clientUpdateOpen,
  setClientUpdateOpen,
  clientStatus,
  appVersion,
  clientUpdateObj,
  clientProgress,
  clientProgressMessage,
  handleClientUpgrade,

  coreUpdateOpen,
  setCoreUpdateOpen,
  coreUpgradeStatus,
  coreVersion,
  coreUpdateRelease,
  coreUpgradeProgress,
  coreUpgradeMessage,
  handleCoreUpgrade,

  logsOpen,
  setLogsOpen,
  mode: _mode,
}) => {
  const theme = useTheme()
  const { t } = useTranslation()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  return (
    <>
      {/* Edit Profile Dialog */}
      <Dialog
        open={editProfileOpen}
        onClose={() => setEditProfileOpen(false)}
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
            onClick={() => setEditProfileOpen(false)}
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

      {/* Client Update Dialog */}
      <Dialog
        open={clientUpdateOpen}
        onClose={() => {
          if (clientStatus !== 'downloading') {
            setClientUpdateOpen(false)
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
            onClick={() => setClientUpdateOpen(false)}
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

      {/* Core Update Dialog */}
      <Dialog
        open={coreUpdateOpen}
        onClose={() => {
          if (
            coreUpgradeStatus !== 'checking' &&
            coreUpgradeStatus !== 'downloading' &&
            coreUpgradeStatus !== 'extracting'
          ) {
            setCoreUpdateOpen(false)
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
            onClick={() => setCoreUpdateOpen(false)}
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

      {/* Logs View Dialog */}
      <Dialog
        open={logsOpen}
        onClose={() => setLogsOpen(false)}
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
          onClick={() => setLogsOpen(false)}
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
          {logsOpen && <LogsPage />}
        </Box>
      </Dialog>
    </>
  )
}
