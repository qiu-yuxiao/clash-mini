import { ArrowDownwardRounded, ArrowUpwardRounded } from '@mui/icons-material'
import { Box, Typography, useTheme } from '@mui/material'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { EnhancedCanvasTrafficGraph } from '@/components/home/enhanced-canvas-traffic-graph'
import { TrafficGraph } from '@/components/layout/traffic-graph'
import { useTrafficData } from '@/hooks/use-traffic-data'
import { useVisibility } from '@/hooks/use-visibility'
import { useThemeMode } from '@/services/states'
import { get3DCardStyle } from '@/utils/button-styles'
import parseTraffic from '@/utils/parse-traffic'

export const MiniTrafficPanel = ({
  isMinimalWidth,
}: {
  isMinimalWidth: boolean
}) => {
  const mode = useThemeMode()
  const theme = useTheme()
  const { t } = useTranslation()
  const pageVisible = useVisibility()
  const {
    response: { data: traffic },
  } = useTrafficData({ enabled: pageVisible })
  const trafficRef = useRef<any>(null)

  useEffect(() => {
    if (trafficRef.current && traffic) {
      trafficRef.current.appendData({
        up: traffic.up || 0,
        down: traffic.down || 0,
      })
    }
  }, [traffic])

  const [upVal, upUnit] = parseTraffic(traffic?.up || 0)
  const [downVal, downUnit] = parseTraffic(traffic?.down || 0)
  const [upTotalVal, upTotalUnit] = parseTraffic(traffic?.upTotal || 0)
  const [downTotalVal, downTotalUnit] = parseTraffic(traffic?.downTotal || 0)

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        width: '100%',
        height: '100%',
        boxSizing: 'border-box',
        overflow: 'hidden',
      }}
    >
      {/* Traffic Graph (Full Width) */}
      <Box sx={{ flex: 1, width: '100%', minHeight: 0, position: 'relative' }}>
        {isMinimalWidth ? (
          <TrafficGraph ref={trafficRef} />
        ) : (
          <EnhancedCanvasTrafficGraph ref={trafficRef} />
        )}
      </Box>

      {/* Metrics Row (Single Line Below Graph - Raised 3D Button style) */}
      <Box
        sx={{
          display: 'flex',
          width: '100%',
          height: isMinimalWidth ? 'auto' : '22px',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderTop: (theme) =>
            isMinimalWidth ? 'none' : `1px solid ${theme.palette.divider}`,
          mt: isMinimalWidth ? 0.25 : 0.5,
          pt: isMinimalWidth ? 0 : 0.5,
          px: isMinimalWidth ? 0.25 : 1,
          gap: 1,
          boxSizing: 'border-box',
          '@media (max-width: 560px)': {
            height: 'auto',
            flexDirection: 'column',
            gap: 0,
            alignItems: 'stretch',
          },
        }}
      >
        {/* Download Group */}
        <Box sx={{ display: 'flex', flex: 1, minWidth: 0 }}>
          {/* Download Speed */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'download'),
              borderTopRightRadius: 0,
              borderBottomRightRadius: 0,
              borderRight: 'none',
              whiteSpace: 'nowrap',
            }}
          >
            <ArrowDownwardRounded
              sx={{
                color:
                  mode === 'light'
                    ? 'var(--download-text, #0084FF)'
                    : 'var(--download-text-dark, #80D8FF)',
                fontSize: 14,
              }}
            />
            <Typography
              sx={{
                fontSize: '9px',
                color:
                  mode === 'light'
                    ? 'var(--download-text, #006064)'
                    : 'var(--download-text-dark, #80D8FF)',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              {t('home.components.traffic.legends.download', {
                defaultValue: '下载',
              })}
              :
            </Typography>
            <Typography
              sx={{
                fontWeight: 'bold',
                fontSize: '13px',
                color:
                  mode === 'light'
                    ? 'var(--download-text, #00363A)'
                    : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {downVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  fontWeight: 'normal',
                  color:
                    mode === 'light'
                      ? 'var(--download-text, #006064)'
                      : 'var(--download-text-dark, #E0F7FA)',
                }}
              >
                {downUnit}/s
              </span>
            </Typography>
          </Box>

          {/* Download Total */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'download'),
              borderTopLeftRadius: 0,
              borderBottomLeftRadius: 0,
              whiteSpace: 'nowrap',
            }}
          >
            <Typography
              sx={{
                fontSize: '9px',
                color:
                  mode === 'light'
                    ? 'var(--download-text, #006064)'
                    : 'var(--download-text-dark, #80D8FF)',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              {t('settings.mini.total', { defaultValue: '总量' })}:
            </Typography>
            <Typography
              sx={{
                fontWeight: 'bold',
                fontSize: '13px',
                color:
                  mode === 'light'
                    ? 'var(--download-text, #00363A)'
                    : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {downTotalVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  color:
                    mode === 'light'
                      ? 'var(--download-text, #006064)'
                      : 'var(--download-text-dark, #E0F7FA)',
                  fontWeight: 'normal',
                }}
              >
                {downTotalUnit}
              </span>
            </Typography>
          </Box>
        </Box>

        {/* Upload Group */}
        <Box sx={{ display: 'flex', flex: 1, minWidth: 0 }}>
          {/* Upload Speed */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'upload'),
              borderTopRightRadius: 0,
              borderBottomRightRadius: 0,
              borderRight: 'none',
              whiteSpace: 'nowrap',
            }}
          >
            <ArrowUpwardRounded
              sx={{
                color:
                  mode === 'light'
                    ? 'var(--upload-text, #E65100)'
                    : 'var(--upload-text-dark, #FFD54F)',
                fontSize: 14,
              }}
            />
            <Typography
              sx={{
                fontSize: '9px',
                color:
                  mode === 'light'
                    ? 'var(--upload-text, #7B5200)'
                    : 'var(--upload-text-dark, #FFD54F)',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              {t('home.components.traffic.legends.upload', {
                defaultValue: '上传',
              })}
              :
            </Typography>
            <Typography
              sx={{
                fontWeight: 'bold',
                fontSize: '13px',
                color:
                  mode === 'light' ? 'var(--upload-text, #3E2723)' : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {upVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  fontWeight: 'normal',
                  color:
                    mode === 'light'
                      ? 'var(--upload-text, #7B5200)'
                      : 'var(--upload-text-dark, #FFECB3)',
                }}
              >
                {upUnit}/s
              </span>
            </Typography>
          </Box>

          {/* Upload Total */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flex: 1,
              gap: 0.3,
              height: '18px',
              px: 0.5,
              ...get3DCardStyle(theme, 'upload'),
              borderTopLeftRadius: 0,
              borderBottomLeftRadius: 0,
              whiteSpace: 'nowrap',
            }}
          >
            <Typography
              sx={{
                fontSize: '9px',
                color:
                  mode === 'light'
                    ? 'var(--upload-text, #7B5200)'
                    : 'var(--upload-text-dark, #FFD54F)',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
              }}
            >
              {t('settings.mini.total', { defaultValue: '总量' })}:
            </Typography>
            <Typography
              sx={{
                fontSize: '13px',
                fontWeight: 'bold',
                color:
                  mode === 'light' ? 'var(--upload-text, #3E2723)' : '#FFFFFF',
                whiteSpace: 'nowrap',
              }}
            >
              {upTotalVal}{' '}
              <span
                style={{
                  fontSize: '8px',
                  color:
                    mode === 'light'
                      ? 'var(--upload-text, #7B5200)'
                      : 'var(--upload-text-dark, #FFECB3)',
                  fontWeight: 'normal',
                }}
              >
                {upTotalUnit}
              </span>
            </Typography>
          </Box>
        </Box>
      </Box>
    </Box>
  )
}
