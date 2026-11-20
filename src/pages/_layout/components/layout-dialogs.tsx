import type { SxProps, Theme } from '@mui/material'
import type { Update } from '@tauri-apps/plugin-updater'
import type React from 'react'

import type { GithubRelease } from '@/pages/_layout/types'

import { ClientUpdateDialog } from './dialogs/client-update-dialog'
import { CoreUpdateDialog } from './dialogs/core-update-dialog'
import { DeleteProfileDialog } from './dialogs/delete-profile-dialog'
import { EditProfileDialog } from './dialogs/edit-profile-dialog'
import { LogsDialog } from './dialogs/logs-dialog'

interface LayoutDialogsProps {
  // Edit Profile Dialog
  editProfileOpen: boolean
  setEditProfileOpen: (open: boolean) => void
  editProfileName: string
  setEditProfileName: (name: string) => void
  editProfileUrl: string
  setEditProfileUrl: (url: string) => void
  editProfileInterval: number
  setEditProfileInterval: (val: number) => void
  isEditProfileLocal: boolean
  handleSaveProfile: () => void
  defaultBtn3DStyle: SxProps<Theme>
  primaryBtn3DStyle: SxProps<Theme>

  // Client Update Dialog
  clientUpdateOpen: boolean
  setClientUpdateOpen: (open: boolean) => void
  clientStatus: string
  appVersion: string
  clientUpdateObj: Update | null
  clientProgress: number
  clientProgressMessage: string
  handleClientUpgrade: () => void

  // Core Update Dialog
  coreUpdateOpen: boolean
  setCoreUpdateOpen: (open: boolean) => void
  coreUpgradeStatus: string
  coreVersion: string | undefined
  coreUpdateRelease: GithubRelease | null
  coreUpgradeProgress: number
  coreUpgradeMessage: string
  handleCoreUpgrade: () => void

  // Logs View Dialog
  logsOpen: boolean
  setLogsOpen: (open: boolean) => void

  // Delete Profile Confirm Dialog (BUG-390)
  deleteConfirmProfile: { uid: string; name: string } | null
  onCancelDeleteProfile: () => void
  onConfirmDeleteProfile: () => void
}

export const LayoutDialogs: React.FC<LayoutDialogsProps> = ({
  editProfileOpen,
  setEditProfileOpen,
  editProfileName,
  setEditProfileName,
  editProfileUrl,
  setEditProfileUrl,
  editProfileInterval,
  setEditProfileInterval,
  isEditProfileLocal,
  handleSaveProfile,
  deleteConfirmProfile,
  onCancelDeleteProfile,
  onConfirmDeleteProfile,
  defaultBtn3DStyle,
  primaryBtn3DStyle,

  clientUpdateOpen,
  setClientUpdateOpen,
  clientStatus,
  appVersion,
  clientUpdateObj,
  clientProgress,
  clientProgressMessage,
  handleClientUpgrade,

  coreUpdateOpen,
  setCoreUpdateOpen,
  coreUpgradeStatus,
  coreVersion,
  coreUpdateRelease,
  coreUpgradeProgress,
  coreUpgradeMessage,
  handleCoreUpgrade,

  logsOpen,
  setLogsOpen,
}) => (
  <>
    <EditProfileDialog
      open={editProfileOpen}
      onClose={() => setEditProfileOpen(false)}
      editProfileName={editProfileName}
      setEditProfileName={setEditProfileName}
      editProfileUrl={editProfileUrl}
      setEditProfileUrl={setEditProfileUrl}
      editProfileInterval={editProfileInterval}
      setEditProfileInterval={setEditProfileInterval}
      isEditProfileLocal={isEditProfileLocal}
      handleSaveProfile={handleSaveProfile}
      defaultBtn3DStyle={defaultBtn3DStyle}
      primaryBtn3DStyle={primaryBtn3DStyle}
    />
    <ClientUpdateDialog
      open={clientUpdateOpen}
      onClose={() => setClientUpdateOpen(false)}
      clientStatus={clientStatus}
      appVersion={appVersion}
      clientUpdateObj={clientUpdateObj}
      clientProgress={clientProgress}
      clientProgressMessage={clientProgressMessage}
      handleClientUpgrade={handleClientUpgrade}
    />
    <CoreUpdateDialog
      open={coreUpdateOpen}
      onClose={() => setCoreUpdateOpen(false)}
      coreUpgradeStatus={coreUpgradeStatus}
      coreVersion={coreVersion}
      coreUpdateRelease={coreUpdateRelease}
      coreUpgradeProgress={coreUpgradeProgress}
      coreUpgradeMessage={coreUpgradeMessage}
      handleCoreUpgrade={handleCoreUpgrade}
    />
    <LogsDialog open={logsOpen} onClose={() => setLogsOpen(false)} />
    <DeleteProfileDialog
      deleteConfirmProfile={deleteConfirmProfile}
      onCancel={onCancelDeleteProfile}
      onConfirm={onConfirmDeleteProfile}
      defaultBtn3DStyle={defaultBtn3DStyle}
    />
  </>
)
