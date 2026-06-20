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
  handleRuleFallbackChange: (fallback: 'direct' | 'adjustable' | 'proxy') => void
  disableCardBorder?: boolean
}

export const RoutingPreferenceCard: React.FC<RoutingPreferenceCardProps> = ({
  policyActiveIndex,
  language,
  handleRuleFallbackChange,
  disableCardBorder,
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
      sx={
        disableCardBorder
          ? undefined
          : {
              p: 1,
              flexShrink: 0,
              ...get3DCardStyle(theme, 'default'),
              '&:hover': {
                transform: 'none',
                boxShadow: get3DCardStyle(theme, 'default').boxShadow,
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
        {t('settings.mini.routingPreference', {
          defaultValue: '分流策略倾向',
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
          height: 22,
          userSelect: 'none',
          ...get3DSegmentedContainerStyle(
            theme,
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
            transform: `translate3d(${policyActiveIndex * 100}%, 0, 0)`,
          }}
        >
          <Box sx={(theme) => get3DSegmentedActiveStyle(theme)} />
        </Box>

        {/* Direct Fallback Option */}
        <Tooltip
          title={t('settings.mini.routingTooltipDirect', {
            defaultValue: '未匹配规则时默认直连',
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
                policyActiveIndex === 0
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
            {t('settings.mini.direct', {
              defaultValue: '直连兜底',
            })}
          </Box>
        </Tooltip>

        {/* Rule Adjustable Option */}
        <Tooltip
          title={t('settings.mini.routingTooltipRules', {
            defaultValue: '在预设规则的基础上任意调整路径控制',
          })}
          placement="top"
          arrow
        >
          <Box
            onClick={() => handleRuleFallbackChange('adjustable')}
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
                language === 'zh' || language === 'zhtw'
                  ? '13px'
                  : '11px',
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
            {t('settings.mini.rules', { defaultValue: '规则可调' })}
          </Box>
        </Tooltip>

        {/* Proxy Fallback Option */}
        <Tooltip
          title={t('settings.mini.routingTooltipProxy', {
            defaultValue: '未匹配规则时默认走代理',
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
                policyActiveIndex === 2
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
            {t('settings.mini.proxy', { defaultValue: '代理兜底' })}
          </Box>
        </Tooltip>
      </Box>
    </Box>
  )
}
