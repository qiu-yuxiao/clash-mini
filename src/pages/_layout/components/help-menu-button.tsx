import { HelpOutlineRounded } from '@mui/icons-material'
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  MenuItem,
  MenuList,
  useTheme,
  type SxProps,
  type Theme,
} from '@mui/material'
import { open } from '@tauri-apps/plugin-shell'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import { exportDiagnosticBundle, showInFolder } from '@/services/cmds'
import { hideNotice, showNotice } from '@/services/notice-service'
import { get3DButtonStyle } from '@/utils/button-styles'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupMenuItemStyle,
  getPopupSkinTokens,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

import { formatCoreVersion } from '../utils/style-helpers'

interface HelpMenuButtonProps {
  helpAnchorEl: HTMLElement | null
  handleHelpClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  handleHelpClose: () => void
  handleClientCheck: () => void
  handleCoreCheck: () => void
  onReopenFirstRunGuide: () => void
  onOpenAntivirusWhitelist: () => void
  clientCheckLoading: boolean
  coreCheckLoading: boolean
  appVersion: string
  coreVersion: string | undefined
  controlSkin: string
  primaryBtn3DStyle: SxProps<Theme>
}

export const HelpMenuButton: React.FC<HelpMenuButtonProps> = ({
  helpAnchorEl,
  handleHelpClick,
  handleHelpClose,
  handleClientCheck,
  handleCoreCheck,
  onReopenFirstRunGuide,
  onOpenAntivirusWhitelist,
  clientCheckLoading,
  coreCheckLoading,
  appVersion,
  coreVersion,
  controlSkin: _controlSkin,
  primaryBtn3DStyle,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  const itemBaseSx = {
    borderRadius: '6px',
    px: 1.25,
    py: 0.8,
    whiteSpace: 'normal',
    wordBreak: 'break-word',
  }
  const menuItemSx = getPopupMenuItemStyle(theme, { extraSx: itemBaseSx })
  const primaryMenuItemSx = getPopupMenuItemStyle(theme, {
    extraSx: { ...itemBaseSx, fontWeight: 700 },
  })

  return (
    <>
      <Button
        variant="contained"
        onClick={handleHelpClick}
        aria-label={t('layout.components.helpMenu.buttonAriaLabel')}
        aria-haspopup="dialog"
        aria-expanded={Boolean(helpAnchorEl)}
        sx={{
          position: 'absolute',
          bottom: '0',
          left: '0',
          width: '80px',
          height: '24px',
          p: 0,
          minWidth: 'auto',
          zIndex: 200,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          outline: 'none',
          boxSizing: 'border-box',
          // BUG-354: 默认 680px 窗口高度下不得隐藏帮助与诊断包入口；低于 830px 时自动停靠至设置面板右上角关闭按钮旁，零遮挡
          '@media (max-height: 830px)': {
            bottom: 'auto',
            left: 'auto',
            top: '3px',
            right: '40px',
            width: '28px',
            height: '28px',
            zIndex: 9999,
          },
          ...primaryBtn3DStyle,
        }}
      >
        <HelpOutlineRounded sx={{ fontSize: '16px' }} />
      </Button>

      <Dialog
        open={Boolean(helpAnchorEl)}
        onClose={handleHelpClose}
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
            <span>❓</span>
            <span>{t('layout.components.helpMenu.buttonAriaLabel')}</span>
          </Box>
        </DialogTitle>

        <DialogContent sx={getPopupContentSx({ px: 1.25, py: 1 })}>
          <MenuList disablePadding>
            <MenuItem
              onClick={async () => {
                handleHelpClose()
                try {
                  await open('https://github.com/qiu-yuxiao/clash-mini')
                } catch (err) {
                  console.error('Failed to open help link:', err)
                }
              }}
              sx={primaryMenuItemSx}
            >
              ⭐ {t('layout.components.helpMenu.github')}
            </MenuItem>
            <MenuItem
              onClick={async () => {
                handleHelpClose()
                try {
                  await open('https://github.com/qiu-yuxiao/clash-mini/wiki')
                } catch (err) {
                  console.error('Failed to open wiki link:', err)
                }
              }}
              sx={menuItemSx}
            >
              💡 {t('layout.components.helpMenu.wiki')}
            </MenuItem>
            <MenuItem
              onClick={() => {
                handleHelpClose()
                onReopenFirstRunGuide()
              }}
              sx={menuItemSx}
            >
              🧭 {t('layout.components.helpMenu.firstRunGuide')}
            </MenuItem>
            <MenuItem
              onClick={() => {
                handleHelpClose()
                onOpenAntivirusWhitelist()
              }}
              sx={menuItemSx}
            >
              🛡️ {t('layout.components.antivirus.menuLabel')}
            </MenuItem>
            <Divider sx={{ my: 0.5, borderColor: tokens.dividerColor }} />
            <MenuItem
              onClick={async () => {
                handleHelpClose()
                const waitId = showNotice.info(
                  t('layout.components.helpMenu.generatingDiagnostics'),
                  0,
                )
                try {
                  const zipPath = await exportDiagnosticBundle()
                  if (!zipPath) return
                  showNotice.success(
                    t('layout.components.helpMenu.diagnosticsExported', {
                      path: zipPath,
                    }),
                  )
                  // 优先使用文件管理器高亮选中该 zip 文件，失败则兜底打开所在目录
                  await showInFolder(zipPath).catch(async (err) => {
                    console.warn(
                      'showInFolder failed, falling back to open dir:',
                      err,
                    )
                    const dir = zipPath.replace(/[\\/][^\\/]+$/, '')
                    if (dir && dir !== zipPath) {
                      await open(dir).catch((openErr) => {
                        console.error(
                          'Failed to open diagnostics dir:',
                          openErr,
                        )
                      })
                    }
                  })
                } finally {
                  hideNotice(waitId)
                }
              }}
              sx={menuItemSx}
            >
              📦 {t('layout.components.helpMenu.exportDiagnostics')}
            </MenuItem>
            <Divider sx={{ my: 0.5, borderColor: tokens.dividerColor }} />
            <MenuItem
              onClick={handleClientCheck}
              disabled={clientCheckLoading}
              sx={menuItemSx}
            >
              🚀 {t('layout.components.helpMenu.checkAppUpdate')}{' '}
              {appVersion ? `(v${appVersion})` : ''}
            </MenuItem>
            <MenuItem
              onClick={handleCoreCheck}
              disabled={coreCheckLoading}
              sx={menuItemSx}
            >
              ⚙️ {t('layout.components.helpMenu.checkCoreUpdate')}{' '}
              {coreVersion ? `(${formatCoreVersion(coreVersion)})` : ''}
            </MenuItem>
          </MenuList>
        </DialogContent>

        <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
          <Button
            fullWidth
            sx={{
              ...get3DButtonStyle(theme, 'outlined', 'default'),
              py: 0.65,
              fontSize: '12.5px',
            }}
            onClick={handleHelpClose}
          >
            {t('shared.actions.close')}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  )
}
