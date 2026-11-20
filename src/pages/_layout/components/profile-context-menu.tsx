import { Divider, Menu, MenuItem, useTheme } from '@mui/material'
import type React from 'react'
import { useTranslation } from 'react-i18next'

import {
  getPopupMenuItemStyle,
  getPopupMenuPaperProps,
} from '@/utils/popup-styles'

interface ProfileContextMenuProps {
  anchorPosition: { top: number; left: number } | null
  isLocal: boolean
  hasUrl: boolean
  onClose: () => void
  onEdit: () => void
  onEditFile: () => void
  onCopyLink: () => void
  onUpdate: () => void
  onDelete: () => void
}

export const ProfileContextMenu: React.FC<ProfileContextMenuProps> = ({
  anchorPosition,
  isLocal,
  hasUrl,
  onClose,
  onEdit,
  onEditFile,
  onCopyLink,
  onUpdate,
  onDelete,
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
        paper: getPopupMenuPaperProps(theme, { minWidth: '160px' }),
      }}
    >
      <MenuItem onClick={onEdit}>📝 {t('shared.actions.edit')}</MenuItem>
      <MenuItem onClick={onEditFile}>
        📄 {t('profiles.components.menu.editFile')}
      </MenuItem>
      <Divider />
      <MenuItem onClick={onCopyLink} disabled={isLocal || !hasUrl}>
        🔗 {t('profiles.components.menu.copyLink')}
      </MenuItem>
      <MenuItem onClick={onUpdate} disabled={isLocal}>
        🔄 {t('profiles.components.menu.update')}
      </MenuItem>
      <Divider />
      <MenuItem
        onClick={onDelete}
        sx={getPopupMenuItemStyle(theme, { danger: true })}
      >
        ❌ {t('shared.actions.delete')}
      </MenuItem>
    </Menu>
  )
}
