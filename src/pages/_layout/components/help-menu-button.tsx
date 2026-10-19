import { HelpOutlineRounded } from '@mui/icons-material'
import {
  Button,
  Menu,
  MenuItem,
  Divider,
  useTheme,
  type SxProps,
  type Theme,
} from '@mui/material'
import { open } from '@tauri-apps/plugin-shell'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import { exportDiagnosticBundle } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import {
  getMenuItemHoverStyle,
  formatCoreVersion,
} from '../utils/style-helpers'

interface HelpMenuButtonProps {
  helpAnchorEl: HTMLElement | null
  handleHelpClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  handleHelpClose: () => void
  handleClientCheck: () => void
  handleCoreCheck: () => void
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
  clientCheckLoading,
  coreCheckLoading,
  appVersion,
  coreVersion,
  controlSkin,
  primaryBtn3DStyle,
}) => {
  const { t: _t } = useTranslation()
  const theme = useTheme()

  return (
    <>
      <Button
        variant="contained"
        onClick={handleHelpClick}
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
          '@media (max-height: 830px)': {
            display: 'none',
          },
          ...primaryBtn3DStyle,
        }}
      >
        <HelpOutlineRounded sx={{ fontSize: '16px' }} />
      </Button>

      <Menu
        anchorEl={helpAnchorEl}
        open={Boolean(helpAnchorEl)}
        onClose={handleHelpClose}
        anchorOrigin={{
          vertical: 'top',
          horizontal: 'left',
        }}
        transformOrigin={{
          vertical: 'bottom',
          horizontal: 'left',
        }}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              minWidth: '220px',
              mb: '8px',
              borderRadius: '6px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              '& .MuiList-root': {
                padding: '4px 0',
              },
            },
          },
        }}
      >
        <MenuItem
          onClick={async () => {
            handleHelpClose()
            try {
              await open('https://github.com/qiu-yuxiao/clash-mini')
            } catch (err) {
              console.error('Failed to open help link:', err)
            }
          }}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🐱 GitHub 主页
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
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          💡 帮助指南 (Wiki)
        </MenuItem>
        <Divider sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }} />
        <MenuItem
          onClick={async () => {
            handleHelpClose()
            const zipPath = await exportDiagnosticBundle()
            if (!zipPath) return
            showNotice.success(`诊断包已导出：${zipPath}`)
            // 打开所在目录，方便用户直接取件发出（配置已脱敏，可直接发送）
            const dir = zipPath.replace(/[\\/][^\\/]+$/, '')
            if (dir && dir !== zipPath) {
              await open(dir).catch((err) => {
                console.error('Failed to open diagnostics dir:', err)
              })
            }
          }}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📦 导出诊断包
        </MenuItem>
        <Divider sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }} />
        <MenuItem
          onClick={handleClientCheck}
          disabled={clientCheckLoading}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🚀 检查软件更新 {appVersion ? `(v${appVersion})` : ''}
        </MenuItem>
        <MenuItem
          onClick={handleCoreCheck}
          disabled={coreCheckLoading}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          ⚙️ 检查内核更新{' '}
          {coreVersion ? `(${formatCoreVersion(coreVersion)})` : ''}
        </MenuItem>
      </Menu>
    </>
  )
}
