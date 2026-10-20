import { CloseRounded } from '@mui/icons-material'
import {
  Box,
  Typography,
  TextField,
  Button,
  Dialog,
  IconButton,
  useTheme,
  type SxProps,
  type Theme,
} from '@mui/material'
import { type Update } from '@tauri-apps/plugin-updater'
import React from 'react'
import { useTranslation } from 'react-i18next'

import {
  get3DInputStyle,
  get3DCardStyle,
  get3DButtonStyle,
} from '@/utils/button-styles'

import LogsPage from '../../logs'
import { isSameVersion, formatCoreVersion } from '../utils/style-helpers'

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

  return (
    <>
      {/* Edit Profile Dialog */}
      <Dialog
        open={editProfileOpen}
        onClose={() => setEditProfileOpen(false)}
        fullWidth
        maxWidth={false}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              p: 3,
              width: 'calc(100% - 32px)',
              maxWidth: '450px',
              borderRadius: '8px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              fontFamily: 'var(--control-font-family)',
              color: theme.palette.text.primary,
            },
          },
        }}
      >
        <Typography
          variant="h6"
          sx={{
            fontWeight: 'bold',
            mb: 2,
            fontFamily: 'var(--control-font-family)',
          }}
        >
          ⚙️ {t('layout.components.dialogs.editProfile.title')}
        </Typography>

        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mb: 3 }}>
          <Box>
            <Typography
              id="profile-name-label"
              variant="body2"
              sx={{
                mb: 0.5,
                fontWeight: 'bold',
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

        <Box sx={{ display: 'flex', justifyContent: 'flex-end', gap: 1.5 }}>
          <Button
            onClick={() => setEditProfileOpen(false)}
            sx={{
              fontSize: '12px',
              height: '28px',
              px: '16px',
              ...defaultBtn3DStyle,
            }}
          >
            {t('shared.actions.cancel')}
          </Button>
          <Button
            onClick={handleSaveProfile}
            sx={{
              fontSize: '12px',
              height: '28px',
              px: '16px',
              ...primaryBtn3DStyle,
            }}
          >
            {t('shared.actions.save')}
          </Button>
        </Box>
      </Dialog>

      {/* Client Update Dialog */}
      <Dialog
        open={clientUpdateOpen}
        onClose={() => {
          if (clientStatus !== 'downloading') {
            setClientUpdateOpen(false)
          }
        }}
        fullWidth
        maxWidth={false}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              p: 3,
              width: 'calc(100% - 32px)',
              maxWidth: '500px',
              borderRadius: '8px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              fontFamily: 'var(--control-font-family)',
              color: theme.palette.text.primary,
            },
          },
        }}
      >
        <Typography
          variant="h6"
          sx={{
            fontWeight: 'bold',
            mb: 2,
            fontFamily: 'var(--control-font-family)',
          }}
        >
          🚀 {t('layout.components.dialogs.client.title')}
        </Typography>

        <Box sx={{ mb: 2 }}>
          <Typography
            variant="body2"
            sx={{ mb: 1, fontFamily: 'var(--control-font-family)' }}
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
              mb: 1,
              fontFamily: 'var(--control-font-family)',
              fontWeight: 'bold',
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
            mb: 1,
            fontFamily: 'var(--control-font-family)',
          }}
        >
          {t('layout.components.dialogs.client.changelog')}
        </Typography>
        <Box
          sx={{
            maxHeight: '200px',
            overflowY: 'auto',
            p: 2,
            mt: 1,
            mb: 2,
            borderRadius: '4px',
            fontSize: '13px',
            whiteSpace: 'pre-wrap',
            fontFamily: 'var(--control-font-family)',
            ...get3DCardStyle(theme, 'default'),
          }}
        >
          {clientUpdateObj?.body ||
            t('layout.components.dialogs.client.changelogEmpty')}
        </Box>

        {clientStatus !== 'idle' && (
          <Box sx={{ width: '100%', mt: 2, mb: 2 }}>
            <Box
              sx={{
                width: '100%',
                height: '8px',
                backgroundColor:
                  theme.palette.mode === 'light'
                    ? 'rgba(0,0,0,0.1)'
                    : 'rgba(255,255,255,0.1)',
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
                mt: 1,
                fontSize: '12px',
                opacity: 0.8,
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {clientProgressMessage}
            </Typography>
          </Box>
        )}

        <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 3 }}>
          <Button
            onClick={() => setClientUpdateOpen(false)}
            disabled={clientStatus === 'downloading'}
            sx={{
              ...get3DButtonStyle(theme, 'outlined', 'default'),
              mr: 1,
            }}
          >
            {t('shared.actions.close')}
          </Button>
          <Button
            onClick={handleClientUpgrade}
            disabled={
              clientStatus === 'downloading' ||
              clientStatus === 'done' ||
              !!(
                clientUpdateObj &&
                isSameVersion(appVersion, clientUpdateObj.version)
              )
            }
            sx={{
              ...get3DButtonStyle(theme, 'contained', 'primary'),
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
        </Box>
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
        fullWidth
        maxWidth={false}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              p: 3,
              width: 'calc(100% - 32px)',
              maxWidth: '450px',
              borderRadius: '8px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              fontFamily: 'var(--control-font-family)',
              color: theme.palette.text.primary,
            },
          },
        }}
      >
        <Typography
          variant="h6"
          sx={{
            fontWeight: 'bold',
            mb: 2,
            fontFamily: 'var(--control-font-family)',
          }}
        >
          ⚙️ {t('layout.components.dialogs.core.title')}
        </Typography>

        <Box sx={{ mb: 2 }}>
          <Typography
            variant="body2"
            sx={{ mb: 1, fontFamily: 'var(--control-font-family)' }}
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
              mb: 1,
              fontFamily: 'var(--control-font-family)',
              fontWeight: 'bold',
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
          <Box sx={{ width: '100%', mt: 2, mb: 2 }}>
            <Box
              sx={{
                width: '100%',
                height: '8px',
                backgroundColor:
                  theme.palette.mode === 'light'
                    ? 'rgba(0,0,0,0.1)'
                    : 'rgba(255,255,255,0.1)',
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
                mt: 1,
                fontSize: '12px',
                opacity: 0.8,
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {coreUpgradeMessage}
            </Typography>
          </Box>
        )}

        <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 3 }}>
          <Button
            onClick={() => setCoreUpdateOpen(false)}
            disabled={
              coreUpgradeStatus === 'checking' ||
              coreUpgradeStatus === 'downloading' ||
              coreUpgradeStatus === 'extracting'
            }
            sx={{
              ...get3DButtonStyle(theme, 'outlined', 'default'),
              mr: 1,
            }}
          >
            {t('shared.actions.close')}
          </Button>
          <Button
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
        </Box>
      </Dialog>

      {/* Logs View Dialog */}
      <Dialog
        open={logsOpen}
        onClose={() => setLogsOpen(false)}
        fullWidth
        maxWidth={false}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              background: 'var(--theme-bg, var(--background-color))',
              height: '480px',
              width: 'calc(100% - 32px)',
              maxWidth: '800px',
              position: 'relative',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
              boxSizing: 'border-box',
              '& .base-page > header': {
                pr: 6,
              },
            },
          },
        }}
      >
        <IconButton
          size="small"
          aria-label={t('layout.a11y.closeLogsDialog')}
          onClick={() => setLogsOpen(false)}
          sx={{
            position: 'absolute',
            top: 14,
            right: 14,
            zIndex: 10,
            color: 'text.secondary',
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
