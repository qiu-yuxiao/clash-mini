import { Box, Typography, Tooltip, useTheme } from '@mui/material'
import React from 'react'
import { useTranslation } from 'react-i18next'

import {
  get3DCardStyle,
  get3DSegmentedContainerStyle,
  get3DSegmentedActiveStyle,
  get3DSegmentedActiveTextColor,
} from '@/utils/button-styles'

interface RoutingPreferenceCardProps {
  policyActiveIndex: number
  language?: string
  handleRuleFallbackChange: (fallback: 'direct' | 'proxy' | 'addurl') => void
  onAddUrlClick: () => void
  disableCardBorder?: boolean
}

export const RoutingPreferenceCard: React.FC<RoutingPreferenceCardProps> = ({
  policyActiveIndex,
  language,
  handleRuleFallbackChange,
  onAddUrlClick,
  disableCardBorder,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const skinFallback = React.useMemo(() => {
    return typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d'
  }, [])
  const skin = theme.controlSkin || skinFallback
  const isRetro3DDark = skin === 'retro-3d' && theme.palette.mode === 'dark'

  return (
    <Box
      sx={
        disableCardBorder
          ? undefined
          : {
              p: 1,
              flexShrink: 0,
              ...get3DCardStyle(theme, 'default'),
              '&:hover': {
                transform: 'none',
                boxShadow: get3DCardStyle(theme, 'default').boxShadow as string,
              },
            }
      }
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
        {t('settings.mini.proxyMethod', { defaultValue: '代理方式' })}
      </Typography>
      <Box
        sx={(theme) => ({
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          bgcolor: 'action.hover',
          borderRadius: '4px',
          p: '1px',
          height: 22,
          userSelect: 'none',
          ...get3DSegmentedContainerStyle(theme),
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
            transform: `translate3d(${policyActiveIndex * 100}%, 0, 0)`,
          }}
        >
          <Box sx={(theme) => get3DSegmentedActiveStyle(theme)} />
        </Box>

        {/* Global Proxy Option */}
        <Tooltip
          title={t('settings.mini.globalProxyTooltip', {
            defaultValue: '所有流量都走代理（简单但费流量）',
          })}
          placement="top"
          arrow
        >
          <Box
            onClick={() => handleRuleFallbackChange('proxy')}
            sx={{
              flex: 1,
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color:
                policyActiveIndex === 0
                  ? get3DSegmentedActiveTextColor(theme)
                  : isRetro3DDark
                    ? 'rgba(44, 31, 3, 0.75)'
                    : 'text.secondary',
              fontSize:
                language === 'zh' || language === 'zhtw' ? '13px' : '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
              '&:hover': {
                color:
                  policyActiveIndex === 0
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? '#2C1F03'
                      : undefined,
              },
              '&:active': {
                color:
                  policyActiveIndex === 0
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.5)'
                      : undefined,
              },
            }}
          >
            {t('settings.mini.globalProxy', { defaultValue: '全局代理' })}
          </Box>
        </Tooltip>

        {/* GFWList Option */}
        <Tooltip
          title={t('settings.mini.gfwlistTooltip', {
            defaultValue: '仅 GFWList 中的网站走代理，其余直连',
          })}
          placement="top"
          arrow
        >
          <Box
            onClick={() => handleRuleFallbackChange('direct')}
            sx={{
              flex: 1,
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color:
                policyActiveIndex === 1
                  ? get3DSegmentedActiveTextColor(theme)
                  : isRetro3DDark
                    ? 'rgba(44, 31, 3, 0.75)'
                    : 'text.secondary',
              fontSize:
                language === 'zh' || language === 'zhtw' ? '13px' : '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
              '&:hover': {
                color:
                  policyActiveIndex === 1
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? '#2C1F03'
                      : undefined,
              },
              '&:active': {
                color:
                  policyActiveIndex === 1
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.5)'
                      : undefined,
              },
            }}
          >
            {t('settings.mini.gfwlist', { defaultValue: 'GFWList' })}
          </Box>
        </Tooltip>

        {/* Add URL Option */}
        <Tooltip
          title={t('settings.mini.addUrlTooltip', {
            defaultValue: '在 GFWList 基础上，手动添加需要代理的网址',
          })}
          placement="top"
          arrow
        >
          <Box
            onClick={onAddUrlClick}
            sx={{
              flex: 1,
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color:
                policyActiveIndex === 2
                  ? get3DSegmentedActiveTextColor(theme)
                  : isRetro3DDark
                    ? 'rgba(44, 31, 3, 0.75)'
                    : 'text.secondary',
              fontSize:
                language === 'zh' || language === 'zhtw' ? '13px' : '11px',
              fontWeight: 'bold',
              cursor: 'pointer',
              zIndex: 1,
              transition: 'color 0.2s ease',
              '&:hover': {
                color:
                  policyActiveIndex === 2
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? '#2C1F03'
                      : undefined,
              },
              '&:active': {
                color:
                  policyActiveIndex === 2
                    ? get3DSegmentedActiveTextColor(theme)
                    : isRetro3DDark
                      ? 'rgba(44, 31, 3, 0.5)'
                      : undefined,
              },
            }}
          >
            {t('settings.mini.addUrl', { defaultValue: '添加网址' })}
          </Box>
        </Tooltip>
      </Box>
    </Box>
  )
}
