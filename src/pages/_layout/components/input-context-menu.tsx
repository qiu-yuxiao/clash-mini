import { Divider, Menu, MenuItem, useTheme } from '@mui/material'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import {
  getPopupMenuItemStyle,
  getPopupMenuPaperProps,
} from '@/utils/popup-styles'

export interface InputContextMenuProps {
  anchorPosition: { top: number; left: number } | null
  onClose: () => void
  canCut: boolean
  canCopy: boolean
  canSelectAll: boolean
  canClear: boolean
  onCut: () => void
  onCopy: () => void
  onPaste: () => void
  onSelectAll: () => void
  onClear: () => void
  minWidth?: string | number
}

export const InputContextMenu: React.FC<InputContextMenuProps> = ({
  anchorPosition,
  onClose,
  canCut,
  canCopy,
  canSelectAll,
  canClear,
  onCut,
  onCopy,
  onPaste,
  onSelectAll,
  onClear,
  minWidth = '160px',
}) => {
  const { t } = useTranslation()
  const theme = useTheme()

  return (
    <Menu
      anchorReference="anchorPosition"
      anchorPosition={anchorPosition ?? undefined}
      open={anchorPosition !== null}
      onClose={onClose}
      slotProps={{
        paper: getPopupMenuPaperProps(theme, { minWidth }),
      }}
    >
      <MenuItem onClick={onCut} disabled={!canCut}>
        ✂️ {t('shared.actions.cut')}
      </MenuItem>
      <MenuItem onClick={onCopy} disabled={!canCopy}>
        📋 {t('shared.actions.copy')}
      </MenuItem>
      <MenuItem onClick={onPaste}>📥 {t('shared.actions.paste')}</MenuItem>
      <MenuItem onClick={onSelectAll} disabled={!canSelectAll}>
        🔍 {t('shared.actions.selectAll')}
      </MenuItem>
      <Divider />
      <MenuItem
        onClick={onClear}
        disabled={!canClear}
        sx={getPopupMenuItemStyle(theme, { danger: true })}
      >
        🧹 {t('shared.actions.clearInput')}
      </MenuItem>
    </Menu>
  )
}
