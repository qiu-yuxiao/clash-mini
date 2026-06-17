import {
  RefreshRounded,
  DeleteRounded,
} from '@mui/icons-material'
import {
  Box,
  Typography,
  CircularProgress,
  TextField,
  Button,
  IconButton,
  useTheme,
} from '@mui/material'
import dayjs from 'dayjs'
import React from 'react'
import { useTranslation } from 'react-i18next'

import {
  get3DCardStyle,
  get3DInputStyle,
  get3DButtonStyle,
} from '@/utils/button-styles'
import parseTraffic from '@/utils/parse-traffic'

interface ProfileImportCardProps {
  url: string
  setUrl: (url: string) => void
  profileLoading: boolean
  profileItems: any[]
  currentProfileUid?: string
  importInputRef: React.RefObject<any>
  importInputContextMenu: { mouseX: number; mouseY: number } | null
  setImportInputContextMenu: (val: { mouseX: number; mouseY: number } | null) => void
  handleImportProfile: () => void
  handleSelectProfile: (uid: string) => void
  handleUpdateProfile: (uid: string, e: React.MouseEvent) => void
  handleDeleteProfile: (uid: string, e: React.MouseEvent) => void
  setProfileMenuAnchorPosition: (val: { left: number; top: number } | null) => void
  setContextMenuProfileUid: (uid: string | null) => void
}

export const ProfileImportCard: React.FC<ProfileImportCardProps> = ({
  url,
  setUrl,
  profileLoading,
  profileItems,
  currentProfileUid,
  importInputRef,
  importInputContextMenu,
  setImportInputContextMenu,
  handleImportProfile,
  handleSelectProfile,
  handleUpdateProfile,
  handleDeleteProfile,
  setProfileMenuAnchorPosition,
  setContextMenuProfileUid,
}) => {
  const { t } = useTranslation() as any
  const theme = useTheme()
  const skin =
    (theme as any).controlSkin ||
    (typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d')
  const isRetro3DDark = skin === 'retro-3d' && theme.palette.mode === 'dark'

  const formatTraffic = (num?: number) => {
    if (typeof num !== 'number') return '-'
    const [val, unit] = parseTraffic(num)
    return `${val}${unit}`
  }

  const formatExpire = (expire?: number) => {
    if (!expire) return '-'
    return dayjs(expire * 1000).format('YYYY-MM-DD')
  }

  return (
    <Box
      sx={{
        p: 1,
        flexShrink: 0,
        ...get3DCardStyle(theme, 'default'),
        '&:hover': {
          transform: 'none',
          boxShadow: get3DCardStyle(theme, 'default').boxShadow,
        },
      }}
    >
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          mb: 0.75,
        }}
      >
        <Typography
          variant="subtitle2"
          sx={{
            fontWeight: 'bold',
            fontSize: '13px',
            display: 'flex',
            alignItems: 'center',
            gap: 1,
            color: isRetro3DDark ? '#2C1F03' : 'inherit',
          }}
        >
          {t('settings.mini.profilesTitle', {
            defaultValue: '订阅与机场配置',
          })}
          {profileLoading && <CircularProgress size={10} />}
        </Typography>
      </Box>
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          gap: 0.5,
          mb: 1,
        }}
      >
        <TextField
          inputRef={importInputRef}
          onContextMenu={(e) => {
            e.preventDefault()
            setImportInputContextMenu({
              mouseX: e.clientX,
              mouseY: e.clientY,
            })
          }}
          placeholder={t('settings.mini.importPlaceholder', {
            defaultValue: '填入订阅链接/节点配置...',
          })}
          size="small"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          multiline
          minRows={1}
          maxRows={4}
          slotProps={{
            htmlInput: {
              style: {
                paddingTop: '4px',
                paddingBottom: '4px',
                fontSize: '13px',
                boxSizing: 'border-box',
              },
            },
          }}
          sx={{ width: '100%', ...get3DInputStyle(theme) }}
        />
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'center',
            width: '100%',
          }}
        >
          <Button
            variant="contained"
            color="primary"
            onClick={handleImportProfile}
            sx={{
              fontSize: 12,
              height: 24,
              px: 2,
              ...get3DButtonStyle(theme, 'contained', 'primary'),
            }}
            disabled={profileLoading}
          >
            {t('settings.mini.importConfig', {
              defaultValue: '导入节点信息',
            })}
          </Button>
        </Box>
      </Box>
      {/* Profiles List */}
      <Box
        sx={{
          maxHeight: 110,
          overflowY: 'auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 0.5,
        }}
      >
        {profileItems.map((item) => {
          const isActive = item.uid === currentProfileUid
          const isHighlighted =
            isActive || item.uid === 'L_Direct_Imports'
          const extra = item.extra
          const hasExtra = !!extra
          const {
            upload = 0,
            download = 0,
            total = 0,
          } = extra ?? {}
          const progress =
            total > 0
              ? Math.min(
                  Math.round(
                    ((download + upload) * 100) / (total + 0.01),
                  ),
                  100,
                )
              : 0

          return (
            <Box
              key={item.uid}
              onClick={() => handleSelectProfile(item.uid)}
              onContextMenu={(e) => {
                e.preventDefault()
                e.stopPropagation()
                setProfileMenuAnchorPosition({
                  left: e.clientX,
                  top: e.clientY,
                })
                setContextMenuProfileUid(item.uid)
              }}
              sx={{
                display: 'flex',
                flexDirection: 'column',
                p: '6px 8px',
                mb: 0.5,
                borderRadius: '6px',
                cursor: 'pointer',
                bgcolor: (theme) =>
                  theme.palette.mode === 'light'
                    ? '#ffffff'
                    : '#282A36',
                borderLeft: (theme) =>
                  `3px solid ${isHighlighted ? theme.palette.primary.main : 'transparent'}`,
                boxShadow: '0 1px 3px rgba(0,0,0,0.05)',
                transition: 'all 0.2s',
                '&:hover': {
                  bgcolor: 'action.hover',
                },
              }}
            >
              {/* Line 1: Title & Actions */}
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  width: '100%',
                }}
              >
                <Typography
                  variant="body2"
                  sx={{
                    fontSize: '13px',
                    fontWeight: isHighlighted ? 600 : 400,
                    color: isHighlighted
                      ? 'primary.main'
                      : 'text.primary',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    maxWidth: '70%',
                  }}
                  title={item.name}
                >
                  {item.name ||
                    t('settings.mini.unnamedConfig', {
                      defaultValue: '未命名配置',
                    })}
                </Typography>
                <Box
                  sx={{
                    display: 'flex',
                    gap: 0.25,
                    alignItems: 'center',
                  }}
                >
                  {item.type === 'remote' && (
                    <IconButton
                      size="small"
                      onClick={(e) =>
                        handleUpdateProfile(item.uid, e)
                      }
                      sx={{
                        p: 0.1,
                        color: isActive
                          ? 'primary.main'
                          : 'text.secondary',
                      }}
                    >
                      <RefreshRounded sx={{ fontSize: 12 }} />
                    </IconButton>
                  )}
                  <IconButton
                    size="small"
                    onClick={(e) =>
                      handleDeleteProfile(item.uid, e)
                    }
                    sx={{ p: 0.1, color: 'error.main' }}
                  >
                    <DeleteRounded sx={{ fontSize: 12 }} />
                  </IconButton>
                </Box>
              </Box>

              {/* Line 2: Traffic & Expiration (Only for remote with extra data) */}
              {item.type === 'remote' && hasExtra && (
                <Box
                  sx={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    mt: 0.25,
                    fontSize: '11px',
                    color: 'text.secondary',
                  }}
                >
                  <span>
                    {formatTraffic(upload + download)} /{' '}
                    {formatTraffic(total)}
                  </span>
                  <span>
                    {extra?.expire
                      ? formatExpire(extra.expire)
                      : '-'}
                  </span>
                </Box>
              )}

              {/* Line 2 for local profiles (Node count & updated time) */}
              {item.type === 'local' && (
                <Box
                  sx={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    mt: 0.25,
                    fontSize: '11px',
                    color: 'text.secondary',
                  }}
                >
                  <span>
                    {item.uid === 'L_Direct_Imports'
                      ? `${t('settings.mini.nodeCount', { defaultValue: '节点数: ' })}${!item.desc || item.desc === '本地手动导入的代理节点' ? '0' : item.desc}`
                      : t('settings.mini.localFile', {
                          defaultValue: '本地文件',
                        })}
                  </span>
                  <span>
                    {item.updated
                      ? dayjs(item.updated * 1000).format(
                          'YYYY-MM-DD',
                        )
                      : '-'}
                  </span>
                </Box>
              )}

              {/* Line 3: Traffic progress bar (Only if total traffic > 0) */}
              {item.type === 'remote' && total > 0 && (
                <Box
                  sx={{
                    width: '100%',
                    height: 2,
                    bgcolor: 'action.hover',
                    borderRadius: 1,
                    mt: 0.5,
                    overflow: 'hidden',
                  }}
                >
                  <Box
                    sx={{
                      width: `${progress}%`,
                      height: '100%',
                      bgcolor: 'primary.main',
                    }}
                  />
                </Box>
              )}
            </Box>
          )
        })}
      </Box>
    </Box>
  )
}
