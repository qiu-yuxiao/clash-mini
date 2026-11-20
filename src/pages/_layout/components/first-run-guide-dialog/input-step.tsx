import { KeyboardReturn as KeyboardReturnIcon } from '@mui/icons-material'
import {
  Box,
  Button,
  DialogActions,
  DialogContent,
  DialogTitle,
  InputAdornment,
  TextField,
  Typography,
  useTheme,
} from '@mui/material'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import { get3DButtonStyle, get3DInputStyle } from '@/utils/button-styles'
import {
  getPopupActionsSx,
  getPopupContentSx,
  getPopupTitleSx,
} from '@/utils/popup-styles'

import { FirstRunStepBadge } from './step-badge'

const FEATURE_KEYS = [
  'layout.components.firstRun.featureAutoSelect',
  'layout.components.firstRun.featureDnsGuard',
  'layout.components.firstRun.featureStealth',
] as const

interface FirstRunInputStepProps {
  url: string
  detectedClipboardUrl: string
  submitting: boolean
  errorMessage: string
  inputRef: React.RefObject<HTMLInputElement | null>
  isRetro3D: boolean
  secondaryText: string
  disabledText: string
  readyColor: string
  onUrlChange: (value: string) => void
  onFillClipboardUrl: () => void
  onContextMenu: (event: React.MouseEvent) => void
  onPaste: () => void
  onSubmitImport: () => void
  onDismissLater: () => void
}

export const FirstRunInputStep: React.FC<FirstRunInputStepProps> = ({
  url,
  detectedClipboardUrl,
  submitting,
  errorMessage,
  inputRef,
  isRetro3D,
  secondaryText,
  disabledText,
  readyColor,
  onUrlChange,
  onFillClipboardUrl,
  onContextMenu,
  onPaste,
  onSubmitImport,
  onDismissLater,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()

  return (
    <>
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
          <span>🎉</span>
          <span>{t('layout.components.firstRun.title')}</span>
        </Box>
        <FirstRunStepBadge
          currentStep={1}
          isRetro3D={isRetro3D}
          secondaryText={secondaryText}
          readyColor={readyColor}
        />
      </DialogTitle>

      <DialogContent sx={getPopupContentSx({ py: 1.75 })}>
        <Typography
          variant="body2"
          sx={{
            color: secondaryText,
            lineHeight: 1.6,
            mb: 1.5,
            fontSize: '13px',
          }}
        >
          {t('layout.components.firstRun.intro')}
        </Typography>

        {/* 三大核心价值亮点胶囊：自适应均分撑满行宽，避免窄窗折行后右侧留白失衡 */}
        <Box
          sx={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: 0.75,
            mb: 1.75,
          }}
        >
          {FEATURE_KEYS.map((featureKey) => (
            <Box
              key={featureKey}
              sx={{
                flex: '1 1 auto',
                textAlign: 'center',
                px: 1,
                py: 0.4,
                borderRadius: '6px',
                fontSize: '11.5px',
                fontWeight: 600,
                color: 'inherit',
                bgcolor: isRetro3D
                  ? 'rgba(44, 31, 3, 0.1)'
                  : (t2) =>
                      t2.palette.mode === 'dark'
                        ? 'rgba(255, 255, 255, 0.06)'
                        : 'rgba(0, 0, 0, 0.04)',
                border: '1px solid',
                borderColor: isRetro3D
                  ? 'rgba(44, 31, 3, 0.2)'
                  : (t2) => t2.palette.divider,
              }}
            >
              {t(featureKey)}
            </Box>
          ))}
        </Box>

        {/* 剪贴板智能探测横幅：若剪贴板已是 http(s) 订阅链接且输入框为空，提供一键点击填入 */}
        {detectedClipboardUrl && !url && (
          <Box
            onClick={onFillClipboardUrl}
            sx={{
              mb: 1.25,
              px: 1.25,
              py: 0.75,
              borderRadius: '6px',
              cursor: submitting ? 'default' : 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 1,
              bgcolor: isRetro3D
                ? 'rgba(27, 94, 32, 0.12)'
                : (t2) =>
                    t2.palette.mode === 'dark'
                      ? 'rgba(46, 125, 50, 0.18)'
                      : 'rgba(46, 125, 50, 0.1)',
              border: '1px dashed',
              borderColor: isRetro3D
                ? 'rgba(27, 94, 32, 0.45)'
                : 'success.main',
              transition: 'transform 0.12s ease',
              '&:hover': {
                transform: 'translateY(-1px)',
              },
            }}
          >
            <Typography
              variant="caption"
              sx={{
                color: readyColor,
                fontWeight: 700,
                fontSize: '12px',
                lineHeight: 1.45,
              }}
            >
              ✨ {t('layout.components.firstRun.clipboardDetected')}
            </Typography>
          </Box>
        )}

        <Box sx={{ mb: 1 }}>
          <TextField
            autoFocus
            fullWidth
            size="small"
            value={url}
            disabled={submitting}
            onChange={(e) => onUrlChange(e.target.value)}
            placeholder="https://example.com/api/v1/client/subscribe?token=..."
            sx={get3DInputStyle(theme)}
            inputRef={inputRef}
            onContextMenu={onContextMenu}
            // 回车图标：既呼应下方的 Enter 提示文字，也支持直接点击触发导入
            slotProps={{
              input: {
                endAdornment: (
                  <InputAdornment
                    position="end"
                    onClick={() => {
                      if (!submitting) onSubmitImport()
                    }}
                    sx={{
                      cursor: submitting ? 'default' : 'pointer',
                    }}
                  >
                    <KeyboardReturnIcon
                      sx={{ fontSize: 16, color: 'text.disabled' }}
                    />
                  </InputAdornment>
                ),
              },
            }}
          />
        </Box>

        {errorMessage && (
          <Typography
            variant="caption"
            sx={{
              color: 'error.main',
              fontWeight: 600,
              display: 'block',
              mt: 0.5,
              fontSize: '12px',
            }}
          >
            ⚠️ {errorMessage}
          </Typography>
        )}

        {/* 粘贴操作与回车提示：主从上下两行居中排布，避免窄窗内横向拥挤 */}
        <Box
          sx={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: 0.5,
            mt: 1,
          }}
        >
          <Button
            size="small"
            variant="text"
            onClick={onPaste}
            disabled={submitting}
            sx={{
              fontSize: '12px',
              fontWeight: 600,
              textTransform: 'none',
              color: 'primary.main',
              px: 1,
              py: 0.25,
              minHeight: 'auto',
            }}
          >
            📋 {t('layout.components.firstRun.paste')}
          </Button>

          <Typography
            variant="caption"
            sx={{
              color: disabledText,
              fontSize: '11.5px',
              lineHeight: 1.4,
              textAlign: 'center',
            }}
          >
            {t('layout.components.firstRun.enterHint')}
          </Typography>
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
          onClick={onSubmitImport}
          loading={submitting}
          disabled={submitting}
        >
          {t('layout.components.firstRun.importAndStart')}
        </Button>
        <Button
          fullWidth
          sx={{
            ...get3DButtonStyle(theme, 'outlined', 'default'),
            py: 0.7,
            fontSize: '12.5px',
          }}
          onClick={onDismissLater}
          disabled={submitting}
        >
          {t('layout.components.firstRun.later')}
        </Button>
      </DialogActions>
    </>
  )
}
