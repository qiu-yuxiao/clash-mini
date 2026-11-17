import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
  useTheme,
} from '@mui/material'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import type React from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useVerge } from '@/hooks/use-verge'
import { showNotice } from '@/services/notice-service'
import { get3DButtonStyle } from '@/utils/button-styles'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupSkinTokens,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

const WHITEINTERNAL_INI_PATH =
  'C:\\Program Files (x86)\\360\\360Safe\\netmon\\whiteinternal.ini'
const WHITEINTERNAL_INI_SNIPPET =
  'path1=Clash Mini\\clash-mini.exe\r\npath2=Clash Mini\\mini-mihomo.exe'

/**
 * 轻量模式防误杀指南弹窗（BUG-395）：
 * 1. 说明 360 在高负载下误杀「曾经露窗又关闭」后台进程的真实原因；
 * 2. 提供两条清晰方案：
 *    - 方案一（省心静默）：一键开启「启动时最小化」，平时不露窗口从根源避开记录，误杀后重开直接静默进托盘；
 *    - 方案二（根治白名单）：Windows 安全模式下将程序路径写入 360 引擎底层硬白名单 `whiteinternal.ini`。
 */
interface AntivirusWhitelistDialogProps {
  open: boolean
  onClose: () => void
}

export const AntivirusWhitelistDialog: React.FC<
  AntivirusWhitelistDialogProps
> = ({ open, onClose }) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)
  const { verge, patchVerge } = useVerge()
  const [enablingSilent, setEnablingSilent] = useState(false)

  const isRetro3D = theme.controlSkin === 'retro-3d'
  const silentEnabled = Boolean(verge?.enable_silent_start)

  const handleEnableSilentStart = async () => {
    if (silentEnabled || enablingSilent) return
    setEnablingSilent(true)
    try {
      const applied = await patchVerge({ enable_silent_start: true })
      if (applied) {
        showNotice.success('settings.mini.settingEnabled')
      }
    } catch (err) {
      showNotice.error(err)
    } finally {
      setEnablingSilent(false)
    }
  }

  const handleCopyText = async (text: string) => {
    try {
      await writeText(text)
    } catch {
      await navigator.clipboard.writeText(text)
    }
    showNotice.success('layout.components.antivirus.copied')
  }

  const cardSx = {
    p: 1.35,
    borderRadius: '8px',
    mb: 1.25,
    bgcolor: isRetro3D
      ? 'rgba(255, 255, 255, 0.22)'
      : theme.palette.mode === 'dark'
        ? 'rgba(128, 128, 128, 0.09)'
        : 'rgba(0, 0, 0, 0.04)',
    border: '1px solid',
    borderColor: isRetro3D ? 'rgba(122, 62, 0, 0.35)' : theme.palette.divider,
  }

  const codeBoxSx = {
    p: 0.9,
    borderRadius: '6px',
    bgcolor:
      theme.palette.mode === 'dark'
        ? 'rgba(0, 0, 0, 0.34)'
        : 'rgba(0, 0, 0, 0.06)',
    border: '1px solid',
    borderColor: theme.palette.divider,
    fontFamily: 'Consolas, "Courier New", monospace',
    fontSize: '11px',
    lineHeight: 1.45,
    color: tokens.primaryText,
    whiteSpace: 'pre-wrap' as const,
    wordBreak: 'break-all' as const,
    overflowWrap: 'anywhere' as const,
    userSelect: 'text' as const,
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth={false}
      sx={getNarrowDialogSx(topOffset)}
      slotProps={getNarrowDialogSlotProps(theme, topOffset)}
    >
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
          <span>🛡️</span>
          <span>{t('layout.components.antivirus.title')}</span>
        </Box>
      </DialogTitle>

      <DialogContent sx={getPopupContentSx({ py: 1.5 })}>
        {/* 简明原因说明 */}
        <Box sx={{ mb: 1.35 }}>
          <Typography
            variant="body2"
            sx={{
              fontWeight: 700,
              color: tokens.primaryText,
              mb: 0.45,
              fontSize: '13px',
              lineHeight: 1.45,
            }}
          >
            {t('layout.components.antivirus.whyTitle')}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              color: tokens.secondaryText,
              lineHeight: 1.55,
              fontSize: '12.5px',
              whiteSpace: 'pre-line',
              overflowWrap: 'anywhere',
            }}
          >
            {t('layout.components.antivirus.why')}
          </Typography>
        </Box>

        {/* 方案一：开启启动时最小化，全程不露窗静默防杀 */}
        <Box sx={cardSx}>
          <Typography
            variant="body2"
            sx={{
              fontWeight: 700,
              color: tokens.primaryText,
              mb: 0.5,
              fontSize: '13px',
              lineHeight: 1.45,
            }}
          >
            {t('layout.components.antivirus.easyTitle')}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              color: tokens.secondaryText,
              lineHeight: 1.55,
              fontSize: '12.5px',
              whiteSpace: 'pre-line',
              overflowWrap: 'anywhere',
            }}
          >
            {t('layout.components.antivirus.easyBody')}
          </Typography>

          <Box
            sx={{
              mt: 0.9,
              p: 0.9,
              borderRadius: '6px',
              bgcolor: isRetro3D
                ? 'rgba(180, 95, 0, 0.14)'
                : theme.palette.mode === 'dark'
                  ? 'rgba(255, 193, 7, 0.12)'
                  : 'rgba(255, 193, 7, 0.14)',
              border: '1px solid',
              borderColor: isRetro3D
                ? 'rgba(122, 62, 0, 0.42)'
                : 'rgba(255, 193, 7, 0.45)',
            }}
          >
            <Typography
              variant="body2"
              sx={{
                color: tokens.primaryText,
                fontWeight: 600,
                lineHeight: 1.5,
                fontSize: '12px',
                whiteSpace: 'pre-line',
                overflowWrap: 'anywhere',
              }}
            >
              {t('layout.components.antivirus.easyWarning')}
            </Typography>
          </Box>

          <Button
            fullWidth
            size="small"
            disabled={silentEnabled || enablingSilent}
            loading={enablingSilent}
            onClick={() => void handleEnableSilentStart()}
            sx={{
              ...get3DButtonStyle(
                theme,
                silentEnabled ? 'outlined' : 'contained',
                silentEnabled ? 'default' : 'primary',
              ),
              mt: 1,
              py: 0.65,
              fontSize: '12.5px',
              fontWeight: 700,
            }}
          >
            {silentEnabled
              ? `✅ ${t('layout.components.antivirus.silentEnabledBtn')}`
              : `⚡ ${t('layout.components.antivirus.enableSilentBtn')}`}
          </Button>
        </Box>

        {/* 方案二：安全模式下修改 360 whiteinternal.ini 硬白名单 */}
        <Box sx={{ ...cardSx, mb: 0 }}>
          <Typography
            variant="body2"
            sx={{
              fontWeight: 700,
              color: tokens.primaryText,
              mb: 0.5,
              fontSize: '13px',
              lineHeight: 1.45,
            }}
          >
            {t('layout.components.antivirus.rootTitle')}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              color: tokens.secondaryText,
              lineHeight: 1.55,
              fontSize: '12.5px',
              whiteSpace: 'pre-line',
              overflowWrap: 'anywhere',
              mb: 0.65,
            }}
          >
            {t('layout.components.antivirus.rootIntro')}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              color: tokens.secondaryText,
              lineHeight: 1.55,
              fontSize: '12px',
              whiteSpace: 'pre-line',
              overflowWrap: 'anywhere',
              mb: 0.9,
            }}
          >
            {t('layout.components.antivirus.rootSteps')}
          </Typography>

          <Box sx={{ ...codeBoxSx, mb: 0.7 }}>{WHITEINTERNAL_INI_PATH}</Box>
          <Box sx={{ ...codeBoxSx, mb: 0.9 }}>{WHITEINTERNAL_INI_SNIPPET}</Box>

          <Box sx={{ display: 'flex', gap: 0.75 }}>
            <Button
              fullWidth
              size="small"
              onClick={() => void handleCopyText(WHITEINTERNAL_INI_PATH)}
              sx={{
                ...get3DButtonStyle(theme, 'outlined', 'default'),
                py: 0.55,
                fontSize: '11.5px',
              }}
            >
              📋 {t('layout.components.antivirus.copyPath')}
            </Button>
            <Button
              fullWidth
              size="small"
              onClick={() => void handleCopyText(WHITEINTERNAL_INI_SNIPPET)}
              sx={{
                ...get3DButtonStyle(theme, 'outlined', 'default'),
                py: 0.55,
                fontSize: '11.5px',
              }}
            >
              📋 {t('layout.components.antivirus.copySnippet')}
            </Button>
          </Box>
        </Box>
      </DialogContent>

      <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
        <Button
          fullWidth
          sx={{
            ...get3DButtonStyle(theme, 'outlined', 'default'),
            py: 0.7,
            fontSize: '12.5px',
          }}
          onClick={onClose}
        >
          {t('layout.components.antivirus.close')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
