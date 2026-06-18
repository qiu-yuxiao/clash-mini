import {
  Box,
  Typography,
  List,
  ListItem,
  Slider,
  Button,
  useTheme,
} from '@mui/material'
import React from 'react'
import { useTranslation } from 'react-i18next'

import {
  get3DCardStyle,
  get3DButtonStyle,
  get3DSegmentedContainerStyle,
  get3DSegmentedActiveStyle,
  get3DSegmentedActiveTextColor,
} from '@/utils/button-styles'
import { get3DSliderStyle } from '../utils/style-helpers'

interface ThemeSettingsCardProps {
  verge: any
  patchVerge: (val: any) => Promise<void>
  themeActiveIndex: number
  depthFactor: number
  handleDepthFactorChange: (val: number) => void
  vibrancyFactor: number
  handleVibrancyFactorChange: (val: number) => void
  controlSkin: string
  setLogsOpen: (open: boolean) => void
  mode: 'light' | 'dark'
}

export const ThemeSettingsCard: React.FC<ThemeSettingsCardProps> = ({
  verge,
  patchVerge,
  themeActiveIndex,
  depthFactor,
  handleDepthFactorChange,
  vibrancyFactor,
  handleVibrancyFactorChange,
  controlSkin,
  setLogsOpen,
  mode,
}) => {
  const { t } = useTranslation() as any
  const theme = useTheme()
  const skin =
    (theme as any).controlSkin ||
    (typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d')
  const isRetro3DDark = skin === 'retro-3d' && theme.palette.mode === 'dark'

  const getSlider1Label = () => {
    switch (controlSkin) {
      case 'retro-3d':
        return 'Depth'
      case 'original':
        return 'Radius'
      case 'modern-flat':
        return 'Roundness'
      case 'frosted-glass':
        return 'Opacity'
      case 'cyberpunk':
        return 'Glow'
      case 'monochrome':
        return 'Radius'
      default:
        return 'Depth'
    }
  }

  const getSlider2Label = () => {
    switch (controlSkin) {
      case 'retro-3d':
        return 'Vibrancy'
      case 'original':
        return 'Accent'
      case 'modern-flat':
        return 'Shadow'
      case 'frosted-glass':
        return 'Blur'
      case 'cyberpunk':
        return 'Speed'
      case 'monochrome':
        return 'Border'
      default:
        return 'Vibrancy'
    }
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
      <Typography
        variant="subtitle2"
        sx={{
          fontWeight: 'bold',
          mb: 0.75,
          fontSize: '13px',
          color: isRetro3DDark ? '#2C1F03' : 'inherit',
        }}
      >
        {t('components.verge.themeSettings.title', {
          defaultValue: '主题设置',
        })}
      </Typography>
      <List dense sx={{ py: 0 }}>
        <ListItem
          sx={{
            py: 0.1,
            px: 0.5,
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <Typography
            variant="caption"
            sx={{
              fontSize: '13px',
              color: isRetro3DDark ? '#2C1F03' : 'inherit',
            }}
          >
            {t('components.verge.basic.fields.themeMode', {
              defaultValue: '主题模式',
            })}
          </Typography>
          <Box
            sx={(theme) => ({
              position: 'relative',
              display: 'flex',
              alignItems: 'center',
              bgcolor: 'action.hover',
              borderRadius: '4px',
              p: '1px',
              width: '120px',
              height: 20,
              userSelect: 'none',
              ...get3DSegmentedContainerStyle(
                theme.palette.mode === 'light',
              ),
            })}
          >
            {/* Sliding Background Indicator */}
            <Box
              sx={{
                position: 'absolute',
                left: 0,
                top: 0,
                width: '33.333%',
                height: '100%',
                zIndex: 0,
                transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                transform: `translate3d(${themeActiveIndex * 100}%, 0, 0)`,
              }}
            >
              <Box sx={(theme) => get3DSegmentedActiveStyle(theme)} />
            </Box>

            {/* System Option */}
            <Box
              onClick={() => patchVerge({ theme_mode: 'system' })}
              sx={{
                flex: 1,
                height: '100%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color:
                  themeActiveIndex === 0
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.75)'
                      : 'text.secondary',
                fontSize: '11px',
                fontWeight: 'bold',
                cursor: 'pointer',
                zIndex: 1,
                transition: 'color 0.2s ease',
                '&:hover': {
                  color:
                    themeActiveIndex === 0
                      ? get3DSegmentedActiveTextColor(theme)
                      : isRetro3DDark
                        ? '#2C1F03'
                        : undefined,
                },
                '&:active': {
                  color:
                    themeActiveIndex === 0
                      ? get3DSegmentedActiveTextColor(theme)
                      : isRetro3DDark
                        ? 'rgba(44, 31, 3, 0.5)'
                        : undefined,
                },
              }}
            >
              {t('sections.appearance.system', {
                defaultValue: '系统',
              })}
            </Box>

            {/* Light Option */}
            <Box
              onClick={() => patchVerge({ theme_mode: 'light' })}
              sx={{
                flex: 1,
                height: '100%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color:
                  themeActiveIndex === 1
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.75)'
                      : 'text.secondary',
                fontSize: '11px',
                fontWeight: 'bold',
                cursor: 'pointer',
                zIndex: 1,
                transition: 'color 0.2s ease',
                '&:hover': {
                  color:
                    themeActiveIndex === 1
                      ? get3DSegmentedActiveTextColor(theme)
                      : isRetro3DDark
                        ? '#2C1F03'
                        : undefined,
                },
                '&:active': {
                  color:
                    themeActiveIndex === 1
                      ? get3DSegmentedActiveTextColor(theme)
                      : isRetro3DDark
                        ? 'rgba(44, 31, 3, 0.5)'
                        : undefined,
                },
              }}
            >
              {t('sections.appearance.light', {
                defaultValue: '浅色',
              })}
            </Box>

            {/* Dark Option */}
            <Box
              onClick={() => patchVerge({ theme_mode: 'dark' })}
              sx={{
                flex: 1,
                height: '100%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color:
                  themeActiveIndex === 2
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.75)'
                      : 'text.secondary',
                fontSize: '11px',
                fontWeight: 'bold',
                cursor: 'pointer',
                zIndex: 1,
                transition: 'color 0.2s ease',
                '&:hover': {
                  color:
                    themeActiveIndex === 2
                      ? get3DSegmentedActiveTextColor(theme)
                      : isRetro3DDark
                        ? '#2C1F03'
                        : undefined,
                },
                '&:active': {
                  color:
                    themeActiveIndex === 2
                      ? get3DSegmentedActiveTextColor(theme)
                      : isRetro3DDark
                        ? 'rgba(44, 31, 3, 0.5)'
                        : undefined,
                },
              }}
            >
              {t('sections.appearance.dark', {
                defaultValue: '深色',
              })}
            </Box>
          </Box>
        </ListItem>
        <ListItem
          sx={{
            py: 0.1,
            px: 0.5,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'stretch',
            mt: 0.5,
          }}
        >
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              mb: 0.25,
            }}
          >
            <Typography
              variant="caption"
              sx={{
                fontSize: '13px',
                fontFamily: 'var(--control-font-family)',
                color: isRetro3DDark ? '#2C1F03' : 'inherit',
              }}
            >
              {getSlider1Label()}
            </Typography>
            <Typography
              variant="caption"
              sx={{
                fontSize: '12px',
                color: isRetro3DDark ? '#2C1F03' : 'text.secondary',
                fontWeight: 'bold',
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {depthFactor.toFixed(1)}
            </Typography>
          </Box>
          <Slider
            size="small"
            value={depthFactor}
            min={0.0}
            max={5.0}
            step={0.1}
            onChange={(_, val) => handleDepthFactorChange(val as number)}
            sx={get3DSliderStyle(theme, mode)}
          />
        </ListItem>
        <ListItem
          sx={{
            py: 0.1,
            px: 0.5,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'stretch',
            mt: 0.5,
          }}
        >
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              mb: 0.25,
            }}
          >
            <Typography
              variant="caption"
              sx={{
                fontSize: '13px',
                fontFamily: 'var(--control-font-family)',
                color: isRetro3DDark ? '#2C1F03' : 'inherit',
              }}
            >
              {getSlider2Label()}
            </Typography>
            <Typography
              variant="caption"
              sx={{
                fontSize: '12px',
                color: isRetro3DDark ? '#2C1F03' : 'text.secondary',
                fontWeight: 'bold',
                fontFamily: 'var(--control-font-family)',
              }}
            >
              {vibrancyFactor.toFixed(1)}
            </Typography>
          </Box>
          <Slider
            size="small"
            value={vibrancyFactor}
            min={0.0}
            max={5.0}
            step={0.1}
            onChange={(_, val) => handleVibrancyFactorChange(val as number)}
            sx={get3DSliderStyle(theme, mode)}
          />
        </ListItem>
      </List>

      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          width: '100%',
          mt: 1,
        }}
      >
        <Button
          variant="contained"
          color="primary"
          size="small"
          onClick={() => setLogsOpen(true)}
          sx={{
            fontSize: 13,
            height: 24,
            px: 2,
            ...get3DButtonStyle(theme, 'contained', 'primary'),
          }}
        >
          {t('settings.mini.debugLogs', {
            defaultValue: '系统调试运行日志',
          })}
        </Button>
      </Box>

      {/* Copyright Footer */}
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          mt: 'auto',
          pt: 1,
          borderTop: (theme) => `1px dashed ${theme.palette.divider}`,
          opacity: 0.75,
          '&:hover': {
            opacity: 1,
          },
          transition: 'opacity 0.2s ease',
        }}
      >
        <Typography
          variant="caption"
          sx={{
            fontSize: '11px',
            color: 'text.secondary',
            textAlign: 'center',
            userSelect: 'none',
          }}
        >
          © 2026 秋雨潇潇 (修改部分)
        </Typography>
        <Typography
          variant="caption"
          sx={{
            fontSize: '10px',
            color: 'primary.main',
            textAlign: 'center',
            textDecoration: 'none',
            '&:hover': {
              textDecoration: 'underline',
            },
          }}
          component="a"
          href="mailto:qiuyuxiao@gmail.com"
        >
          qiuyuxiao@gmail.com
        </Typography>
      </Box>
    </Box>
  )
}
