import { CloseRounded } from '@mui/icons-material'
import {
  Box,
  Typography,
  TextField,
  Button,
  Dialog,
  IconButton,
  useTheme,
} from '@mui/material'
import { type Update } from '@tauri-apps/plugin-updater'
import React from 'react'

import {
  get3DInputStyle,
  get3DCardStyle,
  get3DButtonStyle,
} from '@/utils/button-styles'

import LogsPage from '../../logs'
import {
  isSameVersion,
  formatCoreVersion,
} from '../utils/style-helpers'

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
  defaultBtn3DStyle: any
  primaryBtn3DStyle: any

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

  return (
    <>
      {/* Edit Profile Dialog */}
      <Dialog
        open={editProfileOpen}
        onClose={() => setEditProfileOpen(false)}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              p: 3,
              minWidth: '360px',
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
          ⚙️ 编辑配置文件
        </Typography>

        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mb: 3 }}>
          <Box>
            <Typography
              variant="body2"
              sx={{
                mb: 0.5,
                fontWeight: 'bold',
                fontFamily: 'var(--control-font-family)',
              }}
            >
              配置名称
            </Typography>
            <TextField
              fullWidth
              size="small"
              value={editProfileName}
              onChange={(e) => setEditProfileName(e.target.value)}
              sx={get3DInputStyle(theme)}
            />
          </Box>

          <Box>
            <Typography
              variant="body2"
              sx={{
                mb: 0.5,
                fontWeight: 'bold',
                fontFamily: 'var(--control-font-family)',
              }}
            >
              订阅地址
            </Typography>
            <TextField
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
              variant="body2"
              sx={{
                mb: 0.5,
                fontWeight: 'bold',
                fontFamily: 'var(--control-font-family)',
              }}
            >
              更新周期 (单位: 小时, 设为 0 禁用)
            </Typography>
            <TextField
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
            取消
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
            保存
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
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              p: 3,
              minWidth: '400px',
              maxWidth: '600px',
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
          🚀 软件本体更新
        </Typography>

        <Box sx={{ mb: 2 }}>
          <Typography
            variant="body2"
            sx={{ mb: 1, fontFamily: 'var(--control-font-family)' }}
          >
            当前版本: {appVersion ? `v${appVersion}` : '未知'}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              mb: 1,
              fontFamily: 'var(--control-font-family)',
              fontWeight: 'bold',
            }}
          >
            最新版本:{' '}
            {clientUpdateObj?.version ? `v${clientUpdateObj.version}` : '未知'}
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
          更新日志:
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
          {clientUpdateObj?.body || '暂无详细更新日志'}
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
            关闭
          </Button>
          <Button
            onClick={handleClientUpgrade}
            disabled={
              clientStatus === 'downloading' ||
              clientStatus === 'done' ||
              !!(clientUpdateObj &&
                isSameVersion(appVersion, clientUpdateObj.version))
            }
            sx={{
              ...get3DButtonStyle(theme, 'contained', 'primary'),
            }}
          >
            {clientStatus === 'done'
              ? '准备重启'
              : clientStatus === 'downloading'
                ? '更新中...'
                : clientUpdateObj &&
                    isSameVersion(appVersion, clientUpdateObj.version)
                  ? '已是最新'
                  : '立即更新'}
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
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              p: 3,
              minWidth: '400px',
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
          ⚙️ Mihomo 内核更新
        </Typography>

        <Box sx={{ mb: 2 }}>
          <Typography
            variant="body2"
            sx={{ mb: 1, fontFamily: 'var(--control-font-family)' }}
          >
            当前版本: {coreVersion ? formatCoreVersion(coreVersion) : '未知'}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              mb: 1,
              fontFamily: 'var(--control-font-family)',
              fontWeight: 'bold',
            }}
          >
            最新版本:{' '}
            {coreUpdateRelease?.tag_name
              ? formatCoreVersion(coreUpdateRelease.tag_name)
              : '获取中...'}
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
            关闭
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
              ? '更新完成'
              : coreUpgradeStatus !== 'idle'
                ? '更新中...'
                : isSameVersion(coreVersion, coreUpdateRelease?.tag_name)
                  ? '已是最新'
                  : '立即更新'}
          </Button>
        </Box>
      </Dialog>

      {/* Logs View Dialog */}
      <Dialog
        open={logsOpen}
        onClose={() => setLogsOpen(false)}
        maxWidth="md"
        fullWidth
        slotProps={{
          paper: {
            sx: {
              background: 'var(--theme-bg, var(--background-color))',
              height: '480px',
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
