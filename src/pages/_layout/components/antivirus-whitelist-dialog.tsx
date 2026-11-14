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
import type React from 'react'
import { useTranslation } from 'react-i18next'

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

/**
 * 安全软件白名单说明弹窗：
 * 向用户如实解释「为什么安全软件会清理 Clash Mini / 加白名单有什么好处 / 可以不加 / 不加会怎样 / 怎么加」。
 * 定位是环境说明而非功能开关——不修改任何安全软件配置，只提供透明的指引。
 */
interface AntivirusWhitelistDialogProps {
  open: boolean
  onClose: () => void
}

interface WhitelistSection {
  title: string
  body: string
}

export const AntivirusWhitelistDialog: React.FC<
  AntivirusWhitelistDialogProps
> = ({ open, onClose }) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  const sections: WhitelistSection[] = [
    {
      title: t('layout.components.antivirus.whyTitle'),
      body: t('layout.components.antivirus.why'),
    },
    {
      title: t('layout.components.antivirus.benefitTitle'),
      body: t('layout.components.antivirus.benefit'),
    },
    {
      title: t('layout.components.antivirus.optionalTitle'),
      body: t('layout.components.antivirus.optional'),
    },
    {
      title: t('layout.components.antivirus.consequenceTitle'),
      body: t('layout.components.antivirus.consequence'),
    },
    {
      title: t('layout.components.antivirus.howTitle'),
      body: t('layout.components.antivirus.how'),
    },
  ]

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

      <DialogContent sx={getPopupContentSx({ py: 1.75 })}>
        {sections.map((section) => (
          <Box
            key={section.title}
            sx={{ mb: 1.5, '&:last-of-type': { mb: 0 } }}
          >
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
              {section.title}
            </Typography>
            <Typography
              variant="body2"
              sx={{
                color: tokens.secondaryText,
                lineHeight: 1.55,
                fontSize: '12.5px',
                whiteSpace: 'pre-line',
              }}
            >
              {section.body}
            </Typography>
          </Box>
        ))}
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
