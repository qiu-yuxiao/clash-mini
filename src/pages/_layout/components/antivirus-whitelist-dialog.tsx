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

import {
  get3DCardStyle,
  get3DButtonStyle,
  getActiveSkin,
} from '@/utils/button-styles'

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

  // 弹窗卡片底色来自 get3DCardStyle：retro-3d 皮肤主题灰白次级文字反差几乎为零，
  // 与 first-run-guide-dialog 一致换用配套深棕系；其他皮肤沿用主题色
  const isRetro3D = getActiveSkin(theme) === 'retro-3d'
  const secondaryText = isRetro3D ? 'rgba(44, 31, 3, 0.78)' : 'text.secondary'

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
      fullWidth
      maxWidth={false}
      slotProps={{
        paper: {
          sx: {
            ...get3DCardStyle(theme, 'default'),
            m: 0,
            p: 0,
            overflow: 'hidden',
            width: '100%',
            maxWidth: '520px',
            borderRadius: '8px',
          },
        },
      }}
    >
      <DialogTitle
        sx={{
          color: 'inherit',
          px: 1.75,
          py: 1.5,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          textAlign: 'center',
          gap: 0.6,
          borderBottom: (t2) => `1px solid ${t2.palette.divider}`,
        }}
      >
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

      <DialogContent sx={{ px: 1.75, py: 1.75 }}>
        {sections.map((section) => (
          <Box
            key={section.title}
            sx={{ mb: 1.5, '&:last-of-type': { mb: 0 } }}
          >
            <Typography
              variant="body2"
              sx={{
                fontWeight: 700,
                color: 'text.primary',
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
                color: secondaryText,
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

      <DialogActions
        disableSpacing
        sx={{
          px: 1.75,
          py: 1.5,
          borderTop: (t2) => `1px solid ${t2.palette.divider}`,
        }}
      >
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
