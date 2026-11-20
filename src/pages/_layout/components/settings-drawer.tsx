import {
  Box,
  MenuItem,
  Select,
  useTheme,
  type SxProps,
  type Theme,
} from '@mui/material'
import type React from 'react'
import { ErrorBoundary } from 'react-error-boundary'
import { useTranslation } from 'react-i18next'

import { AreaErrorFallback } from '@/components/base/base-error-boundary'
import type { PatchVergeOptions } from '@/hooks/use-verge'
import type { IClashInfo } from '@/types/clash'
import type { IProfileItem } from '@/types/profile'
import type { IVergeConfig } from '@/types/verge'
import { get3DButtonStyle, get3DCardStyle } from '@/utils/button-styles'
import { getPopupMenuPaperProps } from '@/utils/popup-styles'
import type { BaseConfig } from 'tauri-plugin-mihomo-api'

import { BasicSettingsCard } from './basic-settings-card'
import { ConnectionsPanel } from './connections-panel'
import { HelpMenuButton } from './help-menu-button'
import { ProfileImportCard } from './profile-import-card'
import { RoutingPreferenceCard } from './routing-preference-card'
import { TakeoverModeCard } from './takeover-mode-card'
import { ThemeSettingsCard } from './theme-settings-card'

const SKIN_OPTIONS = [
  {
    key: 'retro-3d',
    label: 'Trump-3D',
    font: 'Trebuchet MS, SimHei, sans-serif',
  },
  {
    key: 'original',
    label: 'Original',
    font: 'Segoe UI, Microsoft YaHei, sans-serif',
  },
  {
    key: 'modern-flat',
    label: 'Modern',
    font: 'Outfit, DengXian, sans-serif',
  },
  {
    key: 'frosted-glass',
    label: 'Frosted',
    font: 'Segoe UI Light, Microsoft YaHei Light, sans-serif',
  },
  {
    key: 'cyberpunk',
    label: 'Cyberpunk',
    font: 'Consolas, NSimSun, monospace',
  },
  {
    key: 'monochrome',
    label: 'Monochrome',
    font: 'Georgia, KaiTi, serif',
  },
] as const

interface SettingsDrawerProps {
  isMinimalWidth: boolean
  url: string
  setUrl: (url: string) => void
  profileLoading: boolean
  profileItems: IProfileItem[]
  currentProfileUid?: string
  importInputRef: React.RefObject<HTMLInputElement | null>
  importInputContextMenu: { mouseX: number; mouseY: number } | null
  setImportInputContextMenu: (
    val: { mouseX: number; mouseY: number } | null,
  ) => void
  handleImportProfile: () => void
  handleSelectProfile: (uid: string) => void
  handleUpdateProfile: (uid: string, e: React.MouseEvent) => void
  handleDeleteProfile: (uid: string, e: React.MouseEvent) => void
  setProfileMenuAnchorPosition: (
    val: { left: number; top: number } | null,
  ) => void
  setContextMenuProfileUid: (uid: string | null) => void

  activeIndex: number
  language?: string
  handleTakeoverModeChange: (mode: 'manual' | 'system' | 'tun') => void
  policyActiveIndex: number
  handleRuleFallbackChange: (
    fallback: 'direct' | 'proxy' | 'addurl',
  ) => Promise<boolean>
  handleAddUrlClick: () => void

  verge: IVergeConfig | null
  clashConfig: BaseConfig | null
  clashInfo: IClashInfo | null
  patchVerge: (
    val: Partial<IVergeConfig>,
    options?: PatchVergeOptions,
  ) => Promise<boolean | undefined>
  handleAllowLanChange: (checked: boolean) => void
  handleIpv6Change: (checked: boolean) => void
  mixedPortVal: number
  handleSavePort: (port: number) => void

  themeActiveIndex: number
  depthFactor: number
  handleDepthFactorChange: (val: number) => void
  commitDepthFactorChange: (val: number) => void
  vibrancyFactor: number
  handleVibrancyFactorChange: (val: number) => void
  commitVibrancyFactorChange: (val: number) => void
  controlSkin: string
  setControlSkin: (skin: string) => void
  setLogsOpen: (open: boolean) => void
  mode: 'light' | 'dark'

  helpAnchorEl: HTMLElement | null
  handleHelpClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  handleHelpClose: () => void
  handleClientCheck: () => void
  handleCoreCheck: () => void
  onReopenFirstRunGuide: () => void
  clientCheckLoading: boolean
  coreCheckLoading: boolean
  appVersion: string
  coreVersion: string | undefined
  primaryBtn3DStyle: SxProps<Theme>

  currentLanguage: string
  switchLanguage: (lang: string) => void
}

export const SettingsDrawer: React.FC<SettingsDrawerProps> = ({
  isMinimalWidth,
  url,
  setUrl,
  profileLoading,
  profileItems,
  currentProfileUid,
  importInputRef,
  importInputContextMenu,
  setImportInputContextMenu,
  handleImportProfile,
  handleSelectProfile,
  handleUpdateProfile,
  handleDeleteProfile,
  setProfileMenuAnchorPosition,
  setContextMenuProfileUid,

  activeIndex,
  language,
  handleTakeoverModeChange,
  policyActiveIndex,
  handleRuleFallbackChange,
  handleAddUrlClick,

  verge,
  clashConfig,
  clashInfo,
  patchVerge,
  handleAllowLanChange,
  handleIpv6Change,
  mixedPortVal,
  handleSavePort,

  themeActiveIndex,
  depthFactor,
  handleDepthFactorChange,
  commitDepthFactorChange,
  vibrancyFactor,
  handleVibrancyFactorChange,
  commitVibrancyFactorChange,
  controlSkin,
  setControlSkin,
  setLogsOpen,
  mode,

  helpAnchorEl,
  handleHelpClick,
  handleHelpClose,
  handleClientCheck,
  handleCoreCheck,
  onReopenFirstRunGuide,
  clientCheckLoading,
  coreCheckLoading,
  appVersion,
  coreVersion,
  primaryBtn3DStyle,

  currentLanguage,
  switchLanguage,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()

  return (
    <div
      className="theme-panel"
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        zIndex: 100,
        // 仅宽窗 (>285px) 会同时挂载右侧 ConnectionsPanel，须保持不换行横排
        display: 'flex',
        boxSizing: 'border-box',
        padding: '12px',
        gap: '12px',
        overflow: 'hidden',
      }}
    >
      {/* Left Settings Column (240px width) */}
      <Box
        sx={{
          flex: '0 0 240px',
          width: '240px',
          display: 'flex',
          flexDirection: 'column',
          gap: '5px',
          overflow: 'hidden',
          pr: 1,
          borderRight: (t2) => `1px solid ${t2.palette.divider}`,
          pb: { xs: 0, '@media (min-height: 831px)': '30px' },
        }}
      >
        {/* Section 1: Subscriptions Import */}
        <ProfileImportCard
          url={url}
          setUrl={setUrl}
          profileLoading={profileLoading}
          profileItems={profileItems}
          currentProfileUid={currentProfileUid}
          importInputRef={importInputRef}
          importInputContextMenu={importInputContextMenu}
          setImportInputContextMenu={setImportInputContextMenu}
          handleImportProfile={handleImportProfile}
          handleSelectProfile={handleSelectProfile}
          handleUpdateProfile={handleUpdateProfile}
          handleDeleteProfile={handleDeleteProfile}
          setProfileMenuAnchorPosition={setProfileMenuAnchorPosition}
          setContextMenuProfileUid={setContextMenuProfileUid}
        />

        {/* Section 2: Takeover Mode + Routing Preference (合并为同一卡片) */}
        <Box
          sx={{
            p: 1,
            flexShrink: 0,
            ...get3DCardStyle(theme, 'default'),
            '&:hover': {
              transform: 'none',
              boxShadow: get3DCardStyle(theme, 'default').boxShadow as string,
            },
          }}
        >
          <TakeoverModeCard
            activeIndex={activeIndex}
            language={language}
            handleTakeoverModeChange={handleTakeoverModeChange}
            disableCardBorder
          />

          <RoutingPreferenceCard
            policyActiveIndex={policyActiveIndex}
            language={language}
            handleRuleFallbackChange={handleRuleFallbackChange}
            onAddUrlClick={handleAddUrlClick}
            disableCardBorder
          />
        </Box>

        {/* Section 3: Minimal Settings */}
        <BasicSettingsCard
          verge={verge}
          clashConfig={clashConfig}
          clashInfo={clashInfo}
          patchVerge={patchVerge}
          handleAllowLanChange={handleAllowLanChange}
          handleIpv6Change={handleIpv6Change}
          mixedPortVal={mixedPortVal}
          handleSavePort={handleSavePort}
        />

        {/* Section 4: Theme Settings */}
        <ThemeSettingsCard
          verge={verge}
          patchVerge={async (val) => {
            await patchVerge(val)
          }}
          themeActiveIndex={themeActiveIndex}
          depthFactor={depthFactor}
          handleDepthFactorChange={handleDepthFactorChange}
          commitDepthFactorChange={commitDepthFactorChange}
          vibrancyFactor={vibrancyFactor}
          handleVibrancyFactorChange={handleVibrancyFactorChange}
          commitVibrancyFactorChange={commitVibrancyFactorChange}
          controlSkin={controlSkin}
          setLogsOpen={setLogsOpen}
          mode={mode}
        />
      </Box>

      {/* Right Connections column (自适应 flex: 1) — 仅大窗口(宽>285)随设置抽屉挂载 */}
      {!isMinimalWidth && (
        <ErrorBoundary FallbackComponent={AreaErrorFallback}>
          <ConnectionsPanel />
        </ErrorBoundary>
      )}

      {/* Help Button */}
      <HelpMenuButton
        helpAnchorEl={helpAnchorEl}
        handleHelpClick={handleHelpClick}
        handleHelpClose={handleHelpClose}
        handleClientCheck={handleClientCheck}
        handleCoreCheck={handleCoreCheck}
        onReopenFirstRunGuide={onReopenFirstRunGuide}
        clientCheckLoading={clientCheckLoading}
        coreCheckLoading={coreCheckLoading}
        appVersion={appVersion}
        coreVersion={coreVersion}
        controlSkin={controlSkin}
        primaryBtn3DStyle={primaryBtn3DStyle}
      />

      {/* Language Selector */}
      <Select
        value={currentLanguage || 'zh'}
        onChange={(e) => switchLanguage(e.target.value as string)}
        displayEmpty
        inputProps={{
          'aria-label': t('layout.a11y.selectLanguage'),
        }}
        renderValue={() => 'Language'}
        size="small"
        variant="outlined"
        MenuProps={{
          anchorOrigin: {
            vertical: 'top',
            horizontal: 'left',
          },
          transformOrigin: {
            vertical: 'bottom',
            horizontal: 'left',
          },
          slotProps: {
            paper: getPopupMenuPaperProps(theme, { maxHeight: 640 }),
          },
        }}
        sx={{
          position: 'absolute',
          bottom: '0',
          left: '80px',
          width: '97.5px',
          height: '24px',
          zIndex: 200,
          boxSizing: 'border-box',
          fontSize: '11px',
          fontWeight: 'bold',
          fontFamily: 'var(--control-font-family)',
          '@media (max-height: 830px)': {
            display: 'none',
          },
          ...primaryBtn3DStyle,
          '& .MuiSelect-select': {
            paddingTop: 0,
            paddingBottom: 0,
            paddingLeft: '12px',
            paddingRight: '24px',
            height: '100%',
            display: 'flex',
            alignItems: 'center',
            color: 'inherit',
            boxSizing: 'border-box',
          },
          '& .MuiOutlinedInput-notchedOutline': {
            border: 'none !important',
          },
          '&.Mui-focused .MuiOutlinedInput-notchedOutline': {
            border: 'none !important',
          },
          '& .MuiSelect-icon': {
            color: 'inherit',
            right: '4px',
          },
          '&:before, &:after': {
            display: 'none !important',
          },
        }}
      >
        <MenuItem value="zh">简体中文</MenuItem>
        <MenuItem value="en">English</MenuItem>
        <MenuItem value="ru">Русский</MenuItem>
        <MenuItem value="fa">فارسی</MenuItem>
        <MenuItem value="tt">Татарча</MenuItem>
        <MenuItem value="id">Bahasa Indonesia</MenuItem>
        <MenuItem value="ar">العربية</MenuItem>
        <MenuItem value="ko">한국어</MenuItem>
        <MenuItem value="tr">Türkçe</MenuItem>
        <MenuItem value="de">Deutsch</MenuItem>
        <MenuItem value="es">Español</MenuItem>
        <MenuItem value="jp">日本語</MenuItem>
        <MenuItem value="zhtw">繁體中文</MenuItem>
      </Select>

      {/* Excel Selector Row */}
      <Box
        sx={{
          position: 'absolute',
          bottom: '0',
          left: '177.5px',
          width: '462.5px',
          height: '24px',
          display: 'flex',
          alignItems: 'stretch',
          zIndex: 200,
          boxSizing: 'border-box',
          '@media (max-height: 830px)': {
            display: 'none',
          },
        }}
      >
        {SKIN_OPTIONS.map((item, index) => {
          const isSelected = controlSkin === item.key
          const handleSelect = () => {
            localStorage.setItem('clash-mini-control-skin', item.key)
            setControlSkin(item.key)
            window.dispatchEvent(new Event('clash-mini-skin-changed'))
          }

          return (
            <Box
              key={item.key}
              onClick={handleSelect}
              sx={(t2) => {
                const isLight = t2.palette.mode === 'light'
                const cellWidth = index === 0 ? '92.5px' : '74px'
                const unselectedStyle = {
                  width: cellWidth,
                  height: '100%',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  cursor: 'pointer',
                  fontSize: '11px',
                  fontWeight: 'bold',
                  color: isLight ? '#555555' : '#aaaaaa',
                  backgroundColor: isLight ? '#f3f3f3' : '#1e1e1e',
                  border: `1px solid ${isLight ? '#d0d0d0' : '#404040'}`,
                  borderLeft:
                    index === 0
                      ? `1px solid ${isLight ? '#d0d0d0' : '#404040'}`
                      : 'none',
                  boxSizing: 'border-box',
                  fontFamily: item.font,
                  transition: 'background-color 0.1s ease',
                  '&:hover': {
                    backgroundColor: isLight ? '#e5e5e5' : '#2d2d2d',
                  },
                }

                if (isSelected) {
                  const btnStyle = get3DButtonStyle(t2, 'contained', 'primary')
                  return {
                    width: cellWidth,
                    height: '100%',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    cursor: 'pointer',
                    fontSize: '11px',
                    fontWeight: 'bold',
                    boxSizing: 'border-box',
                    fontFamily: item.font,
                    ...btnStyle,
                    borderRadius: btnStyle.borderRadius || '0px',
                    margin: 0,
                  }
                }
                return unselectedStyle
              }}
            >
              {item.label}
            </Box>
          )
        })}
      </Box>
    </div>
  )
}
