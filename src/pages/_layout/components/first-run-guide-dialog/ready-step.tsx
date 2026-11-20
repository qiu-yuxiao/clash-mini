import {
  Box,
  Button,
  CircularProgress,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
  useTheme,
} from '@mui/material'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import { get3DButtonStyle } from '@/utils/button-styles'
import {
  getPopupActionsSx,
  getPopupContentSx,
  getPopupTitleSx,
} from '@/utils/popup-styles'

import { FirstRunStepBadge } from './step-badge'

interface FirstRunReadyStepProps {
  proxyReady: boolean
  isUnconfirmed: boolean
  retesting: boolean
  isTakeoverOff: boolean
  activeNodeName: string | null
  activeDelay: number | null
  isRetro3D: boolean
  primaryText: string
  secondaryText: string
  readyColor: string
  warningColor: string
  onRetestNodes: () => void
  onOpenWikiOnly: () => void
  onOpenGithubStarAndFinish: () => void
  onFinish: () => void
}

export const FirstRunReadyStep: React.FC<FirstRunReadyStepProps> = ({
  proxyReady,
  isUnconfirmed,
  retesting,
  isTakeoverOff,
  activeNodeName,
  activeDelay,
  isRetro3D,
  primaryText,
  secondaryText,
  readyColor,
  warningColor,
  onRetestNodes,
  onOpenWikiOnly,
  onOpenGithubStarAndFinish,
  onFinish,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()

  const starAccentColor = isRetro3D
    ? '#7A3E00'
    : theme.palette.mode === 'dark'
      ? '#FFD54F'
      : '#B26A00'

  const statusColor = proxyReady
    ? readyColor
    : isUnconfirmed
      ? warningColor
      : secondaryText

  return (
    <>
      <DialogTitle sx={getPopupTitleSx(theme, { color: readyColor })}>
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
          <span>{t('layout.components.firstRun.readyTitle')}</span>
        </Box>
        <FirstRunStepBadge
          currentStep={2}
          isRetro3D={isRetro3D}
          secondaryText={secondaryText}
          readyColor={readyColor}
        />
      </DialogTitle>

      <DialogContent sx={getPopupContentSx({ py: 1.75 })}>
        {/* 卡片 1：实时节点连通仪表盘 */}
        <Box
          sx={{
            p: 1.5,
            borderRadius: '8px',
            bgcolor: (t2) =>
              t2.palette.mode === 'dark'
                ? 'rgba(76, 175, 80, 0.12)'
                : 'rgba(76, 175, 80, 0.08)',
            border: '1px solid',
            borderColor: (t2) =>
              t2.palette.mode === 'dark'
                ? 'rgba(76, 175, 80, 0.32)'
                : 'rgba(76, 175, 80, 0.28)',
            mb: 1.5,
          }}
        >
          <Typography
            variant="body2"
            sx={{
              fontWeight: 700,
              color: readyColor,
              mb: 0.5,
              fontSize: '13.5px',
            }}
          >
            ✅ {t('layout.components.firstRun.imported')}
          </Typography>

          {/* 就绪状态：测出有效延迟后才宣称连通，否则如实显示「检测中 / 未测出」 */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              gap: 0.75,
              mt: 0.5,
            }}
          >
            {!proxyReady && !isUnconfirmed && (
              <CircularProgress size={12} sx={{ color: secondaryText }} />
            )}
            <Typography
              variant="body2"
              sx={{
                color: statusColor,
                fontWeight: 600,
                lineHeight: 1.5,
                fontSize: '12.5px',
              }}
            >
              {proxyReady
                ? t('layout.components.firstRun.readyOk')
                : isUnconfirmed
                  ? t('layout.components.firstRun.readyUnconfirmed')
                  : t('layout.components.firstRun.checking')}
            </Typography>
          </Box>

          {/* 当前最优节点：仅在测速优选完成（proxyReady）或超时确认（isUnconfirmed）后才展示，避免一开始把未测速的默认首节点当作「最优节点」误导用户 */}
          {activeNodeName && (proxyReady || isUnconfirmed) && (
            <Box
              sx={{
                mt: 1,
                pt: 0.85,
                borderTop: '1px dashed rgba(128, 128, 128, 0.25)',
                display: 'flex',
                flexDirection: 'column',
                gap: 0.4,
              }}
            >
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 1,
                }}
              >
                <Typography
                  variant="caption"
                  sx={{
                    color: secondaryText,
                    fontWeight: 600,
                    fontSize: '11.5px',
                  }}
                >
                  🌐 {t('layout.components.firstRun.activeNodeLabel')}
                </Typography>
                {proxyReady && activeDelay !== null && (
                  <Box
                    sx={{
                      px: 0.85,
                      py: 0.15,
                      borderRadius: '999px',
                      fontSize: '11px',
                      fontWeight: 700,
                      color: readyColor,
                      bgcolor: 'rgba(76, 175, 80, 0.18)',
                      flexShrink: 0,
                    }}
                  >
                    ⚡ {activeDelay} ms
                  </Box>
                )}
              </Box>
              <Typography
                variant="body2"
                sx={{
                  color: primaryText,
                  fontWeight: 700,
                  fontSize: '13px',
                  lineHeight: 1.4,
                  wordBreak: 'break-word',
                }}
              >
                {activeNodeName}
              </Typography>
            </Box>
          )}

          {!proxyReady && (isUnconfirmed || retesting) && (
            <Button
              fullWidth
              size="small"
              sx={{
                ...get3DButtonStyle(theme, 'outlined', 'primary'),
                mt: 1.1,
                py: 0.65,
                fontSize: '12.5px',
                fontWeight: 700,
              }}
              onClick={onRetestNodes}
              loading={retesting}
              disabled={retesting}
            >
              🔄 {t('layout.components.firstRun.retestNodes')}
            </Button>
          )}

          {isTakeoverOff && (
            <Typography
              variant="body2"
              sx={{
                color: warningColor,
                fontWeight: 600,
                mt: 0.75,
                fontSize: '12px',
                lineHeight: 1.5,
              }}
            >
              ⚠️ {t('layout.components.firstRun.sysProxyOff')}
            </Typography>
          )}
        </Box>

        {/* 卡片 2：金色星标 GitHub Star 开源应援卡片 */}
        <Box
          sx={{
            p: 1.5,
            borderRadius: '8px',
            bgcolor: isRetro3D
              ? 'rgba(255, 255, 255, 0.22)'
              : (t2) =>
                  t2.palette.mode === 'dark'
                    ? 'rgba(255, 193, 7, 0.09)'
                    : 'rgba(255, 193, 7, 0.1)',
            border: '1px solid',
            borderColor: isRetro3D
              ? 'rgba(122, 62, 0, 0.35)'
              : 'rgba(255, 193, 7, 0.45)',
          }}
        >
          <Typography
            variant="body2"
            sx={{
              fontWeight: 700,
              color: starAccentColor,
              mb: 0.5,
              fontSize: '13.5px',
              lineHeight: 1.45,
            }}
          >
            {t('layout.components.firstRun.starTitle')}
          </Typography>

          <Typography
            variant="body2"
            sx={{
              color: secondaryText,
              lineHeight: 1.55,
              fontSize: '12.5px',
            }}
          >
            {t('layout.components.firstRun.wikiHint')}
          </Typography>

          <Box
            sx={{
              mt: 0.75,
              display: 'flex',
              justifyContent: 'flex-end',
            }}
          >
            <Button
              size="small"
              variant="text"
              onClick={onOpenWikiOnly}
              sx={{
                fontSize: '11.5px',
                fontWeight: 600,
                textTransform: 'none',
                color: 'primary.main',
                py: 0.2,
                px: 0.75,
                minHeight: 'auto',
              }}
            >
              📖 {t('layout.components.firstRun.openWiki')} ↗
            </Button>
          </Box>
        </Box>
      </DialogContent>

      <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
        <Button
          fullWidth
          sx={{
            ...get3DButtonStyle(theme, 'contained', 'primary'),
            py: 0.85,
            fontSize: '13px',
          }}
          onClick={onOpenGithubStarAndFinish}
        >
          {t('layout.components.firstRun.openGithubStar')}
        </Button>
        <Button
          fullWidth
          sx={{
            ...get3DButtonStyle(theme, 'outlined', 'default'),
            py: 0.7,
            fontSize: '12.5px',
          }}
          onClick={onFinish}
        >
          {t('layout.components.firstRun.finish')}
        </Button>
      </DialogActions>
    </>
  )
}
