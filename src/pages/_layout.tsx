// Copyright (c) 2026 秋雨潇潇 <qiuyuxiao@gmail.com> (Portions relating to modifications)
// SPDX-License-Identifier: GPL-3.0-only

import {
  CloseRounded,
  PushPinRounded,
  SettingsRounded as SettingsRoundedIcon,
} from '@mui/icons-material'
import {
  IconButton,
  Paper,
  ThemeProvider,
  type SxProps,
  type Theme,
} from '@mui/material'
import { getVersion as getAppVersion } from '@tauri-apps/api/app'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ErrorBoundary } from 'react-error-boundary'
import { useTranslation } from 'react-i18next'

import AppIcon from '@/assets/image/app-icon.png'
import { AreaErrorFallback } from '@/components/base/base-error-boundary'
import { NoticeManager } from '@/components/layout/notice-manager'
import { ResizeHandles } from '@/components/layout/resize-handles'
import { WindowControls } from '@/components/layout/window-controller'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import {
  CUSTOM_TITLEBAR_HEIGHT,
  SETTINGS_CLOSE_BUTTON_Z_INDEX,
} from '@/constants'
import { useClash, useClashInfo } from '@/hooks/use-clash'
import { useI18n } from '@/hooks/use-i18n'
import { useWindowDecorations } from '@/hooks/use-window'
import {
  useAppRefreshers,
  useClashConfigData,
  useSystemData,
} from '@/providers/app-data-context'
import { openWebUrl } from '@/services/cmds'
import { useThemeMode } from '@/services/states'
import { get3DButtonStyle } from '@/utils/button-styles'
import { bindMainAppShortcuts } from '@/utils/disable-webview-shortcuts'

import { ActiveNodeStatusCard } from './_layout/components/active-node-card'
import { AddUrlDialog } from './_layout/components/add-url-dialog'
import {
  CLASH_MINI_GITHUB_URL,
  CLASH_MINI_WIKI_URL,
  FirstRunGuideDialog,
} from './_layout/components/first-run-guide-dialog'
import { InputContextMenu } from './_layout/components/input-context-menu'
import { LayoutDialogs } from './_layout/components/layout-dialogs'
import { MiniTrafficPanel } from './_layout/components/mini-traffic-panel'
import { ProfileContextMenu } from './_layout/components/profile-context-menu'
import { SettingsDrawer } from './_layout/components/settings-drawer'
import {
  useCustomTheme,
  useLayoutEvents,
  useLoadingOverlay,
} from './_layout/hooks'
import { useClientUpdate } from './_layout/hooks/use-client-update'
import { useCoreUpdate } from './_layout/hooks/use-core-update'
import { useImportContextMenu } from './_layout/hooks/use-import-context-menu'
import { useMinimalSettings } from './_layout/hooks/use-minimal-settings'
import { useProfileActions } from './_layout/hooks/use-profile-actions'
import { useProxyModeActions } from './_layout/hooks/use-proxy-mode-actions'
import { useSkinControls } from './_layout/hooks/use-skin-controls'
import { handleNoticeMessage } from './_layout/utils'
import { OS, buildImportant3DButtonStyle } from './_layout/utils/style-helpers'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)

// 底部常驻流量条(MiniTrafficPanel)的固定高度(px)。
// 这是面板自身的显示尺寸，与窗口极小高度开关阈值(MINI_HEIGHT_THRESHOLD=126)无关，
// 不应耦合（历史上曾与旧阈值 135 同值纯属巧合，窗口侧先后修正为 121、126 后两值已分离）。
const TRAFFIC_PANE_HEIGHT_WIDE = 135
const TRAFFIC_PANE_HEIGHT_MINIMAL = 100

const Layout = () => {
  // Active Skin State — managed by useSkinControls
  const {
    controlSkin,
    setControlSkin,
    depthFactor,
    vibrancyFactor,
    handleDepthFactorChange,
    commitDepthFactorChange,
    handleVibrancyFactorChange,
    commitVibrancyFactorChange,
  } = useSkinControls()

  // Update States
  const [helpAnchorEl, setHelpAnchorEl] = useState<null | HTMLElement>(null)
  const [appVersion, setAppVersion] = useState<string>('')

  // 必须在 useCoreUpdate 之前调用，以提供 coreVersion/mutateVersion
  const { version: coreVersion, mutateVersion } = useClash()

  const {
    clientUpdateOpen,
    setClientUpdateOpen,
    clientUpdateObj,
    clientStatus,
    clientProgress,
    clientProgressMessage,
    clientCheckLoading,
    handleClientCheck,
    handleClientUpgrade,
  } = useClientUpdate({ appVersion, setHelpAnchorEl })

  const {
    coreUpdateOpen,
    setCoreUpdateOpen,
    coreUpdateRelease,
    coreUpgradeStatus,
    coreUpgradeProgress,
    coreUpgradeMessage,
    coreCheckLoading,
    handleCoreCheck,
    handleCoreUpgrade,
  } = useCoreUpdate({ coreVersion, mutateVersion, setHelpAnchorEl })

  const mode = useThemeMode()
  const { t } = useTranslation()
  const { theme } = useCustomTheme(controlSkin)
  const { switchLanguage, currentLanguage } = useI18n()
  const { decorated, isDecorationsHidden, isMinimalWidth, isMiniStatus } =
    useWindowDecorations()

  // Language Sync Ref to prevent deadlock/rollback loops
  const lastLanguageRef = useRef<string | undefined>(undefined)
  const switchLanguageRef = useRef(switchLanguage)
  useEffect(() => {
    switchLanguageRef.current = switchLanguage
  }, [switchLanguage])

  // Drawer Toggle State
  const { isSettingsOpen: drawerOpen, setIsSettingsOpen: setDrawerOpen } =
    useSystemData()

  // 设置抽屉打开状态下缩到最小窗口时自动关闭（避免遮挡主内容）
  useEffect(() => {
    if (drawerOpen && isMiniStatus) {
      setDrawerOpen(false)
    }
  }, [drawerOpen, isMiniStatus, setDrawerOpen])

  // Profiles & Context Menu Actions
  const {
    url,
    setUrl,
    profileLoading,
    profileItems,
    currentProfileUid,
    activateSelected,
    profileMenuAnchorPosition,
    setProfileMenuAnchorPosition,
    contextMenuTargetItem,
    isContextMenuLocal,
    setContextMenuProfileUid,
    deleteConfirmProfile,
    setDeleteConfirmProfile,
    editProfileOpen,
    setEditProfileOpen,
    editProfileName,
    setEditProfileName,
    editProfileUrl,
    setEditProfileUrl,
    editProfileInterval,
    setEditProfileInterval,
    isEditProfileLocal,
    firstRunGuideOpen,
    setFirstRunGuideOpen,
    handleImportProfile,
    handleSelectProfile,
    handleUpdateProfile,
    handleDeleteProfile,
    handleConfirmDeleteProfile,
    handleEditProfileClick,
    handleEditProfileFileClick,
    handleCopyProfileLinkClick,
    handleUpdateProfileClick,
    handleDeleteProfileClick,
    handleSaveProfile,
  } = useProfileActions()

  const {
    importInputContextMenu,
    setImportInputContextMenu,
    importInputRef,
    handlePaste: handleImportInputPaste,
    handleCopy: handleImportInputCopy,
    handleCut: handleImportInputCut,
    handleSelectAll: handleImportInputSelectAll,
    handleClear: handleImportInputClear,
  } = useImportContextMenu({ url, setUrl })

  const {
    verge,
    patchVerge,
    activeIndex,
    policyActiveIndex,
    addUrlOpen,
    setAddUrlOpen,
    handleRuleFallbackChange,
    handleAddUrlClick,
    handleTakeoverModeChange,
  } = useProxyModeActions({ activateSelected })

  const { language } = verge ?? {}

  // BUG-385: 通过派生可见态与显式开关处理器统一收口抽屉内浮层菜单锚点，
  // 避免在 useEffect 中同步 setState，且在极小窗自动收起或手动关闭设置时同步清理。
  const isDrawerVisible = drawerOpen && !isMiniStatus
  const activeHelpAnchorEl = isDrawerVisible ? helpAnchorEl : null
  const activeProfileMenuAnchorPosition = isDrawerVisible
    ? profileMenuAnchorPosition
    : null
  const activeImportInputContextMenu = isDrawerVisible
    ? importInputContextMenu
    : null

  const resetDrawerMenuAnchors = useCallback(() => {
    setHelpAnchorEl(null)
    setProfileMenuAnchorPosition(null)
    setImportInputContextMenu(null)
  }, [setImportInputContextMenu, setProfileMenuAnchorPosition])

  const openSettingsDrawer = useCallback(() => {
    resetDrawerMenuAnchors()
    setDrawerOpen(true)
  }, [resetDrawerMenuAnchors, setDrawerOpen])

  const closeSettingsDrawer = useCallback(() => {
    resetDrawerMenuAnchors()
    setDeleteConfirmProfile(null)
    setDrawerOpen(false)
  }, [resetDrawerMenuAnchors, setDeleteConfirmProfile, setDrawerOpen])

  const primaryBtn3DStyle = useMemo<SxProps<Theme>>(
    () => buildImportant3DButtonStyle(theme, 'primary'),
    [theme],
  )

  const defaultBtn3DStyle = useMemo<SxProps<Theme>>(
    () => buildImportant3DButtonStyle(theme, 'default'),
    [theme],
  )

  const { clashInfo } = useClashInfo()
  const { clashConfig } = useClashConfigData()
  const { refreshAll } = useAppRefreshers()

  useEffect(() => {
    getAppVersion()
      .then(setAppVersion)
      .catch((err) => console.error('Failed to get app version:', err))
  }, [])

  const handleHelpClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setHelpAnchorEl(event.currentTarget)
  }

  const handleHelpClose = () => {
    setHelpAnchorEl(null)
  }

  const themeModeVal = verge?.theme_mode || 'system'
  const themeActiveIndex =
    themeModeVal === 'dark' ? 2 : themeModeVal === 'light' ? 1 : 0

  // 端口编辑态 + allow-lan/ipv6 切换已抽到 use-minimal-settings（详见该 hook 注释）
  const { mixedPortVal, handleSavePort, handleClashBoolChange } =
    useMinimalSettings()

  useEffect(() => {
    if (language && language !== lastLanguageRef.current) {
      lastLanguageRef.current = language
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguageRef.current(language)
    }
  }, [language])

  const lastVisibilityRefreshAtRef = useRef(0)
  const refreshAllRef = useRef(refreshAll)
  useEffect(() => {
    refreshAllRef.current = refreshAll
  }, [refreshAll])

  // 窗口恢复可见时仅同步前端状态展示（节点探测与自愈由后端 monitor 统一驱动）
  useEffect(() => {
    if (typeof window === 'undefined') return
    const handleVisibilityChange = () => {
      if (document.visibilityState !== 'visible') return
      const now = Date.now()
      if (now - lastVisibilityRefreshAtRef.current < 3000) return
      lastVisibilityRefreshAtRef.current = now
      void refreshAllRef.current()
    }
    document.addEventListener('visibilitychange', handleVisibilityChange)
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [])

  const themeReady = useMemo(() => Boolean(theme), [theme])
  useLoadingOverlay(themeReady)

  const handleNotice = useCallback(
    (payload: [string, string]) => {
      const [status, msg] = payload
      try {
        handleNoticeMessage(status, msg, t, () => {})
      } catch (error) {
        console.error('[通知处理] 失败:', error)
      }
    },
    [t],
  )
  useLayoutEvents(handleNotice)

  // Logs dialog state
  const [logsOpen, setLogsOpen] = useState(false)

  // BUG-355: 统一经 disable-webview-shortcuts 单一监听器绑定主界面快捷键（覆盖全部弹窗/菜单排除、编辑态保护与防误伤 Alt+F4）
  const isAnyOverlayOpen =
    drawerOpen ||
    logsOpen ||
    editProfileOpen ||
    Boolean(deleteConfirmProfile) ||
    addUrlOpen ||
    firstRunGuideOpen ||
    clientUpdateOpen ||
    coreUpdateOpen ||
    Boolean(activeHelpAnchorEl)
  const isAnyOverlayOpenRef = useRef(isAnyOverlayOpen)
  useEffect(() => {
    isAnyOverlayOpenRef.current = isAnyOverlayOpen
  }, [isAnyOverlayOpen])
  useEffect(() => bindMainAppShortcuts(() => isAnyOverlayOpenRef.current), [])

  // Custom Titlebar Render
  const customTitlebar = useMemo(
    () =>
      !decorated && !isDecorationsHidden ? (
        <div
          className="the_titlebar"
          data-tauri-drag-region="true"
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            padding: '0 8px 0 10px',
            boxSizing: 'border-box',
            height: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            minHeight: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            maxHeight: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            borderBottom: '1px solid var(--divider-color)',
            background: 'var(--background-color)',
            userSelect: 'none',
            flexShrink: 0,
            gap: '8px',
            overflow: 'hidden',
          }}
        >
          <img
            src={AppIcon}
            alt=""
            draggable={false}
            data-tauri-drag-region="true"
            style={{
              width: '16px',
              height: '16px',
              flexShrink: 0,
              pointerEvents: 'none',
            }}
          />
          <span
            data-tauri-drag-region="true"
            style={{
              fontSize: '12px',
              fontWeight: 500,
              color: 'var(--text-primary-color)',
              flexShrink: 0,
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              pointerEvents: 'none',
            }}
          >
            Clash Mini Ver.{appVersion}
          </span>

          <div
            data-tauri-no-drag="true"
            style={{
              marginLeft: 'auto',
              flexShrink: 0,
              display: 'flex',
              alignItems: 'center',
              height: `${CUSTOM_TITLEBAR_HEIGHT}px`,
            }}
          >
            <WindowControls />
          </div>
        </div>
      ) : null,
    [decorated, isDecorationsHidden, appVersion],
  )

  if (!themeReady) {
    return (
      <div
        style={{
          width: '100vw',
          height: '100vh',
          background: mode === 'light' ? '#fff' : '#181a1b',
          transition: 'background 0.2s',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: mode === 'light' ? '#333' : '#fff',
        }}
      ></div>
    )
  }

  return (
    <ThemeProvider theme={theme}>
      <NoticeManager position={verge?.notice_position} />

      <Paper
        square
        elevation={0}
        className={`${OS} layout`}
        {...(isDecorationsHidden ? { 'data-tauri-drag-region': 'true' } : {})}
        style={{
          width: '100vw',
          height: '100vh',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          background: 'transparent',
          position: 'relative',
        }}
        onContextMenu={(e) => {
          if (
            !['input', 'textarea'].includes(
              e.currentTarget.tagName.toLowerCase(),
            ) &&
            !e.currentTarget.isContentEditable
          ) {
            e.preventDefault()
          }
        }}
      >
        <ResizeHandles />
        {customTitlebar}

        <div
          className="layout-content"
          style={{
            display: 'flex',
            flexDirection: 'column',
            height:
              decorated || isDecorationsHidden
                ? '100vh'
                : `calc(100vh - ${CUSTOM_TITLEBAR_HEIGHT}px)`,
            width: '100vw',
            overflow: 'hidden',
            position: 'relative',
            boxSizing: 'border-box',
          }}
        >
          {/* Upper Pane: Node Selection */}
          <div
            style={{
              flex: '1 1 0%',
              minHeight: 0,
              position: 'relative',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
            }}
          >
            {/* 右上角独立控制按钮（仅在设置面板打开时渲染，以允许关闭设置） */}
            {drawerOpen && (
              <div
                data-no-drag="true"
                style={{
                  position: 'absolute',
                  top: '3px',
                  right: '8px',
                  zIndex: SETTINGS_CLOSE_BUTTON_Z_INDEX,
                }}
              >
                <IconButton
                  size="small"
                  data-close-settings="true"
                  aria-label={t('layout.a11y.closeSettings')}
                  onClick={closeSettingsDrawer}
                  sx={(theme) => ({
                    ...get3DButtonStyle(theme, 'contained', 'primary'),
                    width: '28px',
                    height: '28px',
                    p: 0,
                  })}
                >
                  <CloseRounded
                    aria-hidden="true"
                    sx={{ fontSize: '20px', width: '20px', height: '20px' }}
                  />
                </IconButton>
              </div>
            )}

            {/*置顶当前节点与快捷控制栏（仅在未打开设置时渲染）*/}
            {!drawerOpen && (
              <div
                data-no-drag="true"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  width: '100%',
                  boxSizing: 'border-box',
                  padding: '0px 8px 2px 8px',
                  height: '30px',
                  position: 'relative',
                  zIndex: 9998,
                  gap: 0,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <ErrorBoundary FallbackComponent={AreaErrorFallback}>
                    <ActiveNodeStatusCard />
                  </ErrorBoundary>
                </div>

                <IconButton
                  size="small"
                  aria-label={t('layout.a11y.pinWindow')}
                  onClick={() =>
                    patchVerge({
                      enable_always_on_top: !verge?.enable_always_on_top,
                    }).catch((err) =>
                      console.error('[Layout] toggle pinWindow failed:', err),
                    )
                  }
                  sx={(theme) => ({
                    ...get3DButtonStyle(
                      theme,
                      'contained',
                      verge?.enable_always_on_top ? 'primary' : 'default',
                    ),
                    flexShrink: 0,
                    width: '28px',
                    height: '28px',
                    p: 0,
                  })}
                >
                  <PushPinRounded
                    aria-hidden="true"
                    sx={{
                      fontSize: '20px',
                      width: '20px',
                      height: '20px',
                      color: verge?.enable_always_on_top
                        ? '#FF3B30'
                        : '#888888',
                      filter: verge?.enable_always_on_top
                        ? 'drop-shadow(0 0 3px rgba(255, 59, 48, 0.85)) drop-shadow(0 1px 1px rgba(255, 255, 255, 0.45))'
                        : 'none',
                      transform: verge?.enable_always_on_top
                        ? 'rotate(45deg)'
                        : 'none',
                      transition:
                        'transform 0.2s ease, color 0.2s ease, filter 0.2s ease',
                    }}
                  />
                </IconButton>

                <IconButton
                  size="small"
                  aria-label={t('layout.a11y.openSettings')}
                  onClick={openSettingsDrawer}
                  sx={(theme) => ({
                    ...get3DButtonStyle(theme, 'contained', 'default'),
                    flexShrink: 0,
                    width: '28px',
                    height: '28px',
                    p: 0,
                  })}
                >
                  <SettingsRoundedIcon
                    aria-hidden="true"
                    sx={{ fontSize: '20px', width: '20px', height: '20px' }}
                  />
                </IconButton>
              </div>
            )}

            {/*节点组选择列表（设置抽屉打开被全屏遮挡或极小窗时彻底卸载，释放节点列表内存与 Timer）*/}
            {!drawerOpen && !isMiniStatus && (
              <div style={{ flex: 1, overflow: 'hidden' }}>
                <ErrorBoundary FallbackComponent={AreaErrorFallback}>
                  <ProxyGroups
                    mode={clashConfig?.mode?.toLowerCase() || 'rule'}
                  />
                </ErrorBoundary>
              </div>
            )}

            {/* Settings Drawer (Conditionally mounted; unmounting avoids idle WebSocket/resource drain) */}
            {isDrawerVisible && (
              <SettingsDrawer
                isMinimalWidth={isMinimalWidth}
                url={url}
                setUrl={setUrl}
                profileLoading={profileLoading}
                profileItems={profileItems}
                currentProfileUid={currentProfileUid}
                importInputRef={importInputRef}
                importInputContextMenu={activeImportInputContextMenu}
                setImportInputContextMenu={setImportInputContextMenu}
                handleImportProfile={handleImportProfile}
                handleSelectProfile={handleSelectProfile}
                handleUpdateProfile={handleUpdateProfile}
                handleDeleteProfile={handleDeleteProfile}
                setProfileMenuAnchorPosition={setProfileMenuAnchorPosition}
                setContextMenuProfileUid={setContextMenuProfileUid}
                activeIndex={activeIndex}
                language={language}
                handleTakeoverModeChange={handleTakeoverModeChange}
                policyActiveIndex={policyActiveIndex}
                handleRuleFallbackChange={handleRuleFallbackChange}
                handleAddUrlClick={handleAddUrlClick}
                verge={verge ?? null}
                clashConfig={clashConfig ?? null}
                clashInfo={clashInfo ?? null}
                patchVerge={patchVerge}
                handleAllowLanChange={handleClashBoolChange('allow-lan')}
                handleIpv6Change={handleClashBoolChange('ipv6')}
                mixedPortVal={mixedPortVal}
                handleSavePort={handleSavePort}
                themeActiveIndex={themeActiveIndex}
                depthFactor={depthFactor}
                handleDepthFactorChange={handleDepthFactorChange}
                commitDepthFactorChange={commitDepthFactorChange}
                vibrancyFactor={vibrancyFactor}
                handleVibrancyFactorChange={handleVibrancyFactorChange}
                commitVibrancyFactorChange={commitVibrancyFactorChange}
                controlSkin={controlSkin}
                setControlSkin={setControlSkin}
                setLogsOpen={setLogsOpen}
                mode={mode}
                helpAnchorEl={activeHelpAnchorEl}
                handleHelpClick={handleHelpClick}
                handleHelpClose={handleHelpClose}
                handleClientCheck={handleClientCheck}
                handleCoreCheck={handleCoreCheck}
                onReopenFirstRunGuide={() => {
                  setHelpAnchorEl(null)
                  setFirstRunGuideOpen(true)
                }}
                clientCheckLoading={clientCheckLoading}
                coreCheckLoading={coreCheckLoading}
                appVersion={appVersion}
                coreVersion={coreVersion}
                primaryBtn3DStyle={primaryBtn3DStyle}
                currentLanguage={currentLanguage}
                switchLanguage={switchLanguage}
              />
            )}
          </div>

          {/* Lower Pane: Constant Traffic Dashboard */}
          <div
            style={{
              flex: isMinimalWidth
                ? `0 0 ${TRAFFIC_PANE_HEIGHT_MINIMAL}px`
                : `0 1 ${TRAFFIC_PANE_HEIGHT_WIDE}px`,
              maxHeight: '100%',
              height: isMinimalWidth
                ? `${TRAFFIC_PANE_HEIGHT_MINIMAL}px`
                : `${TRAFFIC_PANE_HEIGHT_WIDE}px`,
              background: 'inherit',
              padding: isMinimalWidth ? '3px 6px 2px 6px' : '8px 12px 2px 12px',
              display: 'flex',
              gap: isMinimalWidth ? '6px' : 0,
              overflow: 'hidden',
              boxSizing: 'border-box',
            }}
          >
            <ErrorBoundary FallbackComponent={AreaErrorFallback}>
              <MiniTrafficPanel isMinimalWidth={isMinimalWidth} />
            </ErrorBoundary>
          </div>
        </div>
      </Paper>

      {/* Popups & dialogs：单个弹窗渲染异常只做局部降级，不拖垮整页 */}
      {/* Profile Card Context Menu (BUG-072) */}
      <ProfileContextMenu
        anchorPosition={activeProfileMenuAnchorPosition}
        isLocal={isContextMenuLocal}
        hasUrl={Boolean(contextMenuTargetItem?.url)}
        onClose={() => setProfileMenuAnchorPosition(null)}
        onEdit={handleEditProfileClick}
        onEditFile={handleEditProfileFileClick}
        onCopyLink={handleCopyProfileLinkClick}
        onUpdate={handleUpdateProfileClick}
        onDelete={handleDeleteProfileClick}
      />

      {/* Import Input Context Menu (BUG-091) */}
      <InputContextMenu
        anchorPosition={
          activeImportInputContextMenu !== null
            ? {
                top: activeImportInputContextMenu.mouseY,
                left: activeImportInputContextMenu.mouseX,
              }
            : null
        }
        onClose={() => setImportInputContextMenu(null)}
        canCut={Boolean(
          importInputRef.current &&
            importInputRef.current.selectionStart !==
              importInputRef.current.selectionEnd,
        )}
        canCopy={Boolean(
          importInputRef.current &&
            importInputRef.current.selectionStart !==
              importInputRef.current.selectionEnd,
        )}
        canSelectAll={Boolean(url)}
        canClear={Boolean(url)}
        onCut={handleImportInputCut}
        onCopy={handleImportInputCopy}
        onPaste={handleImportInputPaste}
        onSelectAll={handleImportInputSelectAll}
        onClear={handleImportInputClear}
      />

      <ErrorBoundary FallbackComponent={AreaErrorFallback}>
        <LayoutDialogs
          editProfileOpen={editProfileOpen}
          setEditProfileOpen={setEditProfileOpen}
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
          clientUpdateOpen={clientUpdateOpen}
          setClientUpdateOpen={setClientUpdateOpen}
          clientStatus={clientStatus}
          appVersion={appVersion}
          clientUpdateObj={clientUpdateObj}
          clientProgress={clientProgress}
          clientProgressMessage={clientProgressMessage}
          handleClientUpgrade={handleClientUpgrade}
          coreUpdateOpen={coreUpdateOpen}
          setCoreUpdateOpen={setCoreUpdateOpen}
          coreUpgradeStatus={coreUpgradeStatus}
          coreVersion={coreVersion}
          coreUpdateRelease={coreUpdateRelease}
          coreUpgradeProgress={coreUpgradeProgress}
          coreUpgradeMessage={coreUpgradeMessage}
          handleCoreUpgrade={handleCoreUpgrade}
          logsOpen={logsOpen}
          setLogsOpen={setLogsOpen}
          deleteConfirmProfile={deleteConfirmProfile}
          onCancelDeleteProfile={() => setDeleteConfirmProfile(null)}
          onConfirmDeleteProfile={handleConfirmDeleteProfile}
        />
        <FirstRunGuideDialog
          open={firstRunGuideOpen}
          onClose={() => setFirstRunGuideOpen(false)}
          onImport={handleImportProfile}
          onOpenGithub={() => {
            openWebUrl(CLASH_MINI_GITHUB_URL).catch((e) => {
              console.error('[FirstRunGuide] 打开 GitHub 失败:', e)
            })
          }}
          onOpenWiki={() => {
            openWebUrl(CLASH_MINI_WIKI_URL).catch((e) => {
              console.error('[FirstRunGuide] 打开 Wiki 失败:', e)
            })
          }}
          onLater={() => {
            setFirstRunGuideOpen(false)
            openSettingsDrawer()
          }}
        />
        <AddUrlDialog open={addUrlOpen} onClose={() => setAddUrlOpen(false)} />
      </ErrorBoundary>
    </ThemeProvider>
  )
}

export default Layout
