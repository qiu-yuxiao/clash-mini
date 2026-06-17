import { Box, Typography, Tooltip, useTheme } from '@mui/material'
import React from 'react'
import { useTranslation } from 'react-i18next'

import {
  get3DCardStyle,
  get3DSegmentedContainerStyle,
  get3DSegmentedActiveStyle,
  get3DSegmentedActiveTextColor,
} from '@/utils/button-styles'

interface TakeoverModeCardProps {
  activeIndex: number
  language?: string
  handleTakeoverModeChange: (mode: 'manual' | 'system' | 'tun') => void
}

export const TakeoverModeCard: React.FC<TakeoverModeCardProps> = ({
  activeIndex,
  language,
  handleTakeoverModeChange,
}) => {
  const { t } = useTranslation() as any
  const theme = useTheme()
  const skin =
    (theme as any).controlSkin ||
    (typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d')
  const isRetro3DDark = skin === 'retro-3d' && theme.palette.mode === 'dark'

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
        {t('settings.mini.takeoverMode', {
          defaultValue: '流量接管模式',
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
          mb: 1,
          height: 22,
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
            transform: `translate3d(${activeIndex * 100}%, 0, 0)`,
          }}
        >
          <Box sx={(theme) => get3DSegmentedActiveStyle(theme)} />
        </Box>

        {/* Manual Mode Option */}
        <Tooltip
          title={t('settings.mini.takeoverTooltipManual', {
            defaultValue: '完全手动配置代理',
          })}
          placement="top"
          arrow
        >
          <Box
            onClick={() => handleTakeoverModeChange('manual')}
            sx={{
              flex: 1,
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color:
                activeIndex === 0
                  ? get3DSegmentedActiveTextColor(theme)
                  : isRetro3DDark
                    ? 'rgba(44, 31, 3, 0.75)'
                    : 'text.secondary',
              fontSize:
                language === 'zh' || language === 'zhtw'
                  ? '13px'
                  : '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
              '&:hover': {
                color:
                  activeIndex === 0
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? '#2C1F03'
                      : undefined,
              },
              '&:active': {
                color:
                  activeIndex === 0
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.5)'
                      : undefined,
              },
            }}
          >
            {t('settings.mini.manual', {
              defaultValue: '手动模式',
            })}
          </Box>
        </Tooltip>

        {/* System Proxy Option */}
        <Tooltip
          title={t('settings.mini.takeoverTooltipSystem', {
            defaultValue: '自动启用系统全局代理',
          })}
          placement="top"
          arrow
        >
          <Box
            onClick={() => handleTakeoverModeChange('system')}
            sx={{
              flex: 1,
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color:
                activeIndex === 1
                  ? get3DSegmentedActiveTextColor(theme)
                  : isRetro3DDark
                    ? 'rgba(44, 31, 3, 0.75)'
                    : 'text.secondary',
              fontSize:
                language === 'zh' || language === 'zhtw'
                  ? '13px'
                  : '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
              '&:hover': {
                color:
                  activeIndex === 1
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? '#2C1F03'
                      : undefined,
              },
              '&:active': {
                color:
                  activeIndex === 1
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.5)'
                      : undefined,
              },
            }}
          >
            {t('settings.mini.system', {
              defaultValue: '系统代理',
            })}
          </Box>
        </Tooltip>

        {/* TUN Mode Option */}
        <Tooltip
          title={t('settings.mini.takeoverTooltipTun', {
            defaultValue: '开启虚拟网卡接管全机流量',
          })}
          placement="top"
          arrow
        >
          <Box
            onClick={() => handleTakeoverModeChange('tun')}
            sx={{
              flex: 1,
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color:
                activeIndex === 2
                  ? get3DSegmentedActiveTextColor(theme)
                  : isRetro3DDark
                    ? 'rgba(44, 31, 3, 0.75)'
                    : 'text.secondary',
              fontSize:
                language === 'zh' || language === 'zhtw'
                  ? '13px'
                  : '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
              '&:hover': {
                color:
                  activeIndex === 2
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? '#2C1F03'
                      : undefined,
              },
              '&:active': {
                color:
                  activeIndex === 2
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.5)'
                      : undefined,
              },
            }}
          >
            {t('settings.mini.tun', { defaultValue: 'TUN 模式' })}
          </Box>
        </Tooltip>
      </Box>
    </Box>
  )
}
