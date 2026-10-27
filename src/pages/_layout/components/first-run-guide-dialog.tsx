import { KeyboardReturn as KeyboardReturnIcon } from '@mui/icons-material'
import {
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  InputAdornment,
  TextField,
  Typography,
  useTheme,
  Menu,
  MenuItem,
  Divider,
} from '@mui/material'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import type React from 'react'
import { useState, useRef, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import {
  useAppRefreshers,
  useProxiesData,
  useSystemData,
} from '@/providers/app-data-context'
import { NODE_DELAY_MAX_MS } from '@/services/delay'
import {
  get3DCardStyle,
  get3DButtonStyle,
  get3DInputStyle,
  getActiveSkin,
} from '@/utils/button-styles'

import { getMenuItemHoverStyle } from '../utils/style-helpers'

export const CLASH_MINI_GITHUB_URL = 'https://github.com/qiu-yuxiao/clash-mini'
export const CLASH_MINI_WIKI_URL =
  'https://github.com/qiu-yuxiao/clash-mini/wiki'
export const FIRST_RUN_GUIDE_STORAGE_KEY = 'clash-mini-first-run-completed'

/** 就绪判定超时：超时仍未测出有效延迟，就不再宣称「已就绪」，改为如实提示 */
const READY_TIMEOUT_MS = 20_000
/** 就绪判定轮询间隔：借用既有代理上下文刷新，不另造测量链路 */
const READY_POLL_INTERVAL_MS = 1_500

interface FirstRunGuideDialogProps {
  open: boolean
  onClose: () => void
  onImport: (url: string) => Promise<boolean>
  onOpenWiki: () => void
  onOpenGithub?: () => void
  onLater: () => void
}

export const FirstRunGuideDialog: React.FC<FirstRunGuideDialogProps> = ({
  open,
  onClose,
  onImport,
  onOpenWiki,
  onOpenGithub,
  onLater,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const controlSkin = theme.controlSkin || 'default'

  const [step, setStep] = useState<'input' | 'ready'>('input')
  const [url, setUrl] = useState('')
  const [detectedClipboardUrl, setDetectedClipboardUrl] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [errorMessage, setErrorMessage] = useState('')
  const [gaveUp, setGaveUp] = useState(false)

  // Context menu for input
  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
  } | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // ── 就绪判定：复用细粒度代理与系统上下文，避免订阅全量 AppData 触发无关重绘 ──
  const { proxies } = useProxiesData()
  const { sysproxy } = useSystemData()
  const { refreshProxy } = useAppRefreshers()
  // refreshProxy 的引用不保证稳定，用 ref 持有，避免写进轮询副作用依赖后反复重排
  const refreshProxyRef = useRef(refreshProxy)
  useEffect(() => {
    refreshProxyRef.current = refreshProxy
  }, [refreshProxy])

  /** 当前选中的活跃出口节点名称 */
  const activeNodeName = useMemo(() => {
    const group = proxies?.groups.find((g) => g.name === 'PROXY')
    return group?.now || null
  }, [proxies])

  /** 活跃出口节点最近一次延迟（ms）；无数据返回 null */
  const activeDelay = useMemo(() => {
    if (!activeNodeName) return null
    const node =
      proxies?.records[activeNodeName] ??
      proxies?.proxies.find((p) => p.name === activeNodeName)
    const history = node?.history
    if (!history || history.length === 0) return null
    const delay = history[history.length - 1]?.delay
    return typeof delay === 'number' ? delay : null
  }, [proxies, activeNodeName])

  /** 有效延迟（排除 0/超时与假节点阈值）才算真的连通 */
  const proxyReady =
    activeDelay !== null && activeDelay > 0 && activeDelay < NODE_DELAY_MAX_MS

  // 进入第二步开始计时：超时仍无有效延迟则放弃「已就绪」的表述
  useEffect(() => {
    if (!open || step !== 'ready') return
    const timer = setTimeout(() => setGaveUp(true), READY_TIMEOUT_MS)
    return () => clearTimeout(timer)
  }, [open, step])

  // 未就绪且未超时：周期刷新代理数据，等节点测出延迟
  useEffect(() => {
    if (!open || step !== 'ready' || proxyReady || gaveUp) return
    const timer = setInterval(() => {
      void refreshProxyRef.current?.()
    }, READY_POLL_INTERVAL_MS)
    return () => clearInterval(timer)
  }, [open, step, proxyReady, gaveUp])

  // 弹窗打开时尝试自动聚焦输入框，并静默探测剪贴板中是否已有待导入的 http(s) 订阅链接
  useEffect(() => {
    if (!open) return
    const timer = setTimeout(() => {
      inputRef.current?.focus()
    }, 100)

    let cancelled = false
    const detectClipboard = async () => {
      let clip = ''
      try {
        clip = await readText()
      } catch {
        try {
          clip = await navigator.clipboard.readText()
        } catch {
          // ignore permission errors
        }
      }
      if (cancelled || !clip) return
      const trimmed = clip.trim()
      if (
        !trimmed.includes('\n') &&
        !trimmed.includes(' ') &&
        (trimmed.startsWith('http://') || trimmed.startsWith('https://'))
      ) {
        setDetectedClipboardUrl(trimmed)
      } else {
        setDetectedClipboardUrl('')
      }
    }
    void detectClipboard()

    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [open])

  const resetForm = () => {
    // 重置回第一步：支持「帮助菜单 → 新手引导」重新打开时从导入步开始
    setStep('input')
    setUrl('')
    setDetectedClipboardUrl('')
    setSubmitting(false)
    setErrorMessage('')
    setGaveUp(false)
    setContextMenu(null)
  }

  /** 记录「已引导」标记：三个出口共用，避免重复写散落各处 */
  const markGuided = () => {
    localStorage.setItem(FIRST_RUN_GUIDE_STORAGE_KEY, 'true')
  }

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault()
    setContextMenu({
      mouseX: event.clientX,
      mouseY: event.clientY,
    })
  }

  const handlePaste = async () => {
    setContextMenu(null)
    let textToPaste = ''
    try {
      textToPaste = await readText()
    } catch {
      try {
        textToPaste = await navigator.clipboard.readText()
      } catch (e) {
        console.error('Failed to read clipboard:', e)
      }
    }

    if (!textToPaste) return
    setUrl(textToPaste.trim())
    setErrorMessage('')
  }

  const handleCopy = async () => {
    setContextMenu(null)
    if (!url) return
    try {
      await writeText(url)
    } catch {
      try {
        await navigator.clipboard.writeText(url)
      } catch (e) {
        console.error('Failed to copy:', e)
      }
    }
  }

  const handleCut = async () => {
    setContextMenu(null)
    if (!url) return
    await handleCopy()
    setUrl('')
  }

  const handleSelectAll = () => {
    setContextMenu(null)
    inputRef.current?.select()
  }

  const handleClear = () => {
    setContextMenu(null)
    setUrl('')
    setErrorMessage('')
    inputRef.current?.focus()
  }

  const handleSubmitImport = async () => {
    if (submitting) return
    const trimmed = url.trim()
    if (!trimmed) {
      setErrorMessage(t('layout.components.firstRun.errEmpty'))
      inputRef.current?.focus()
      return
    }

    if (!trimmed.startsWith('http://') && !trimmed.startsWith('https://')) {
      setErrorMessage(t('layout.components.firstRun.errScheme'))
      return
    }

    setSubmitting(true)
    setErrorMessage('')
    setGaveUp(false)

    try {
      const success = await onImport(trimmed)
      if (success) {
        setStep('ready')
      } else {
        setErrorMessage(t('layout.components.firstRun.errImport'))
      }
    } catch (err: unknown) {
      console.error('[FirstRunGuide] 导入异常:', err)
      setErrorMessage(t('layout.components.firstRun.errImport'))
    } finally {
      setSubmitting(false)
    }
  }

  // 【Enter 全局化】监听挂在 Dialog Paper 上：弹窗内任意位置按 Enter 都能导入。
  // 旧实现只挂在输入框上——焦点一旦离开输入框（例如点了下方的提示文字），键盘事件
  // 根本到不了监听器，表现为「按了回车毫无反应」（BUG-299）。
  // 排除规则：
  //  · isComposing——中文输入法组词确认的 Enter 不算提交指令；
  //  · 焦点落在按钮上——放行按钮自身行为（「稍后」「粘贴」「导入」各自的 Enter 语义）。
  const handleDialogKeyDown = (e: React.KeyboardEvent) => {
    if (e.key !== 'Enter' || e.nativeEvent.isComposing) return
    if (submitting || step !== 'input') return
    if ((e.target as HTMLElement).closest('button')) return
    e.preventDefault()
    void handleSubmitImport()
  }

  const handleFinish = () => {
    markGuided()
    resetForm()
    onClose()
  }

  const handleOpenGithubStarAndFinish = () => {
    markGuided()
    resetForm()
    if (onOpenGithub) {
      onOpenGithub()
    } else {
      onOpenWiki()
    }
    onClose()
  }

  const handleOpenWikiOnly = () => {
    onOpenWiki()
  }

  const handleDismissLater = () => {
    markGuided()
    resetForm()
    onLater()
  }

  // 弹窗卡片底色来自 get3DCardStyle：retro-3d 皮肤为金黄渐变，其主题灰白
  // 次级文字（text.secondary/disabled）反差几乎为零，换用与卡片配套的深棕系
  // （#2C1F03 即 get3DCardStyle default 的 textColor）；其他皮肤沿用主题色
  const isRetro3D = getActiveSkin(theme) === 'retro-3d'
  const secondaryText = isRetro3D ? 'rgba(44, 31, 3, 0.78)' : 'text.secondary'
  const disabledText = isRetro3D ? 'rgba(44, 31, 3, 0.55)' : 'text.disabled'
  const readyColor = isRetro3D ? '#1B5E20' : 'success.main'
  const warningColor = isRetro3D ? '#8C4A03' : 'warning.main'
  const starAccentColor = isRetro3D
    ? '#7A3E00'
    : theme.palette.mode === 'dark'
      ? '#FFD54F'
      : '#B26A00'

  const statusColor = proxyReady
    ? readyColor
    : gaveUp
      ? warningColor
      : secondaryText

  const stepBadge = (currentStep: 1 | 2) => (
    <Box
      sx={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 0.75,
        px: 1,
        py: 0.2,
        borderRadius: '999px',
        fontSize: '10.5px',
        fontWeight: 700,
        letterSpacing: '0.6px',
        bgcolor: isRetro3D
          ? 'rgba(44, 31, 3, 0.12)'
          : 'rgba(128, 128, 128, 0.14)',
        color: secondaryText,
        border: '1px solid',
        borderColor: isRetro3D
          ? 'rgba(44, 31, 3, 0.22)'
          : 'rgba(128, 128, 128, 0.24)',
      }}
    >
      <Box
        sx={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          bgcolor: currentStep === 1 ? 'primary.main' : readyColor,
        }}
      />
      <span>STEP {currentStep} / 2</span>
    </Box>
  )

  return (
    <>
      <Dialog
        open={open}
        onClose={() => {
          // 导入进行中：点遮罩、按 Esc（reason 为 escapeKeyDown）都不允许关闭，
          // 避免把正在进行的导入中途打断。
          if (submitting) return
          if (step === 'ready') {
            handleFinish()
          } else {
            handleDismissLater()
          }
        }}
        fullWidth
        maxWidth={false}
        slotProps={{
          paper: {
            onKeyDown: handleDialogKeyDown,
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
        {step === 'input' ? (
          <>
            <DialogTitle
              sx={{
                color: 'inherit',
                px: 1.75,
                py: 1.5,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'flex-start',
                gap: 0.6,
                borderBottom: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 0.75,
                  fontWeight: 'bold',
                  fontSize: '15.5px',
                  lineHeight: 1.35,
                }}
              >
                <span>🎉</span>
                <span>{t('layout.components.firstRun.title')}</span>
              </Box>
              {stepBadge(1)}
            </DialogTitle>

            <DialogContent sx={{ px: 1.75, py: 1.75 }}>
              <Typography
                variant="body2"
                sx={{
                  color: secondaryText,
                  lineHeight: 1.6,
                  mb: 1.5,
                  fontSize: '13px',
                }}
              >
                {t('layout.components.firstRun.intro')}
              </Typography>

              {/* 三大核心价值亮点胶囊：自适应均分撑满行宽，避免窄窗折行后右侧留白失衡 */}
              <Box
                sx={{
                  display: 'flex',
                  flexWrap: 'wrap',
                  gap: 0.75,
                  mb: 1.75,
                }}
              >
                {(
                  [
                    'layout.components.firstRun.featureAutoSelect',
                    'layout.components.firstRun.featureDnsGuard',
                    'layout.components.firstRun.featureStealth',
                  ] as const
                ).map((featureKey) => (
                  <Box
                    key={featureKey}
                    sx={{
                      flex: '1 1 auto',
                      textAlign: 'center',
                      px: 1,
                      py: 0.4,
                      borderRadius: '6px',
                      fontSize: '11.5px',
                      fontWeight: 600,
                      color: 'inherit',
                      bgcolor: isRetro3D
                        ? 'rgba(44, 31, 3, 0.1)'
                        : (t2) =>
                            t2.palette.mode === 'dark'
                              ? 'rgba(255, 255, 255, 0.06)'
                              : 'rgba(0, 0, 0, 0.04)',
                      border: '1px solid',
                      borderColor: isRetro3D
                        ? 'rgba(44, 31, 3, 0.2)'
                        : (t2) => t2.palette.divider,
                    }}
                  >
                    {t(featureKey)}
                  </Box>
                ))}
              </Box>

              {/* 剪贴板智能探测横幅：若剪贴板已是 http(s) 订阅链接且输入框为空，提供一键点击填入 */}
              {detectedClipboardUrl && !url && (
                <Box
                  onClick={() => {
                    if (submitting) return
                    setUrl(detectedClipboardUrl)
                    setErrorMessage('')
                    inputRef.current?.focus()
                  }}
                  sx={{
                    mb: 1.25,
                    px: 1.25,
                    py: 0.75,
                    borderRadius: '6px',
                    cursor: submitting ? 'default' : 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 1,
                    bgcolor: isRetro3D
                      ? 'rgba(27, 94, 32, 0.12)'
                      : (t2) =>
                          t2.palette.mode === 'dark'
                            ? 'rgba(46, 125, 50, 0.18)'
                            : 'rgba(46, 125, 50, 0.1)',
                    border: '1px dashed',
                    borderColor: isRetro3D
                      ? 'rgba(27, 94, 32, 0.45)'
                      : 'success.main',
                    transition: 'transform 0.12s ease',
                    '&:hover': {
                      transform: 'translateY(-1px)',
                    },
                  }}
                >
                  <Typography
                    variant="caption"
                    sx={{
                      color: readyColor,
                      fontWeight: 700,
                      fontSize: '12px',
                      lineHeight: 1.45,
                    }}
                  >
                    ✨ {t('layout.components.firstRun.clipboardDetected')}
                  </Typography>
                </Box>
              )}

              <Box sx={{ mb: 1 }}>
                <TextField
                  autoFocus
                  fullWidth
                  size="small"
                  value={url}
                  disabled={submitting}
                  onChange={(e) => {
                    setUrl(e.target.value)
                    if (errorMessage) setErrorMessage('')
                  }}
                  placeholder="https://example.com/api/v1/client/subscribe?token=..."
                  sx={get3DInputStyle(theme)}
                  inputRef={inputRef}
                  onContextMenu={handleContextMenu}
                  // 装饰性回车图标：呼应下方的 Enter 提示文字，同时弱化「提示文字像按钮」的观感
                  slotProps={{
                    input: {
                      endAdornment: (
                        <InputAdornment position="end">
                          <KeyboardReturnIcon
                            sx={{ fontSize: 16, color: 'text.disabled' }}
                          />
                        </InputAdornment>
                      ),
                    },
                  }}
                />
              </Box>

              {errorMessage && (
                <Typography
                  variant="caption"
                  sx={{
                    color: 'error.main',
                    fontWeight: 600,
                    display: 'block',
                    mt: 0.5,
                    fontSize: '12px',
                  }}
                >
                  ⚠️ {errorMessage}
                </Typography>
              )}

              {/* 粘贴操作与回车提示：主从上下两行居中排布，避免窄窗内横向拥挤 */}
              <Box
                sx={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  gap: 0.5,
                  mt: 1,
                }}
              >
                <Button
                  size="small"
                  variant="text"
                  onClick={handlePaste}
                  disabled={submitting}
                  sx={{
                    fontSize: '12px',
                    fontWeight: 600,
                    textTransform: 'none',
                    color: 'primary.main',
                    px: 1,
                    py: 0.25,
                    minHeight: 'auto',
                  }}
                >
                  📋 {t('layout.components.firstRun.paste')}
                </Button>

                <Typography
                  variant="caption"
                  sx={{
                    color: disabledText,
                    fontSize: '11.5px',
                    lineHeight: 1.4,
                    textAlign: 'center',
                  }}
                >
                  {t('layout.components.firstRun.enterHint')}
                </Typography>
              </Box>
            </DialogContent>

            <DialogActions
              disableSpacing
              sx={{
                flexDirection: 'column',
                alignItems: 'center',
                px: 1.75,
                py: 1.5,
                gap: 1,
                borderTop: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <Button
                fullWidth
                sx={{
                  ...get3DButtonStyle(theme, 'contained', 'primary'),
                  py: 0.85,
                  fontSize: '13px',
                }}
                onClick={handleSubmitImport}
                loading={submitting}
                disabled={submitting}
              >
                {t('layout.components.firstRun.importAndStart')}
              </Button>
              <Button
                fullWidth
                sx={{
                  ...get3DButtonStyle(theme, 'outlined', 'default'),
                  py: 0.7,
                  fontSize: '12.5px',
                }}
                onClick={handleDismissLater}
                disabled={submitting}
              >
                {t('layout.components.firstRun.later')}
              </Button>
            </DialogActions>
          </>
        ) : (
          <>
            <DialogTitle
              sx={{
                color: readyColor,
                px: 1.75,
                py: 1.5,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'flex-start',
                gap: 0.6,
                borderBottom: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 0.75,
                  fontWeight: 'bold',
                  fontSize: '15.5px',
                  lineHeight: 1.35,
                }}
              >
                <span>🚀</span>
                <span>{t('layout.components.firstRun.readyTitle')}</span>
              </Box>
              {stepBadge(2)}
            </DialogTitle>

            <DialogContent sx={{ px: 1.75, py: 1.75 }}>
              {/* 卡片 1：实时节点连通仪表盘 */}
              <Box
                sx={{
                  p: 1.5,
                  borderRadius: '8px',
                  bgcolor: (t2) =>
                    t2.palette.mode === 'dark'
                      ? 'rgba(76, 175, 80, 0.12)'
                      : 'rgba(76, 175, 80, 0.08)',
                  border: '1px solid',
                  borderColor: (t2) =>
                    t2.palette.mode === 'dark'
                      ? 'rgba(76, 175, 80, 0.32)'
                      : 'rgba(76, 175, 80, 0.28)',
                  mb: 1.5,
                }}
              >
                <Typography
                  variant="body2"
                  sx={{
                    fontWeight: 700,
                    color: readyColor,
                    mb: 0.5,
                    fontSize: '13.5px',
                  }}
                >
                  ✅ {t('layout.components.firstRun.imported')}
                </Typography>

                {/* 就绪状态：测出有效延迟后才宣称连通，否则如实显示「检测中 / 未测出」 */}
                <Box
                  sx={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 0.75,
                    mt: 0.5,
                  }}
                >
                  {!proxyReady && !gaveUp && (
                    <CircularProgress size={12} sx={{ color: secondaryText }} />
                  )}
                  <Typography
                    variant="body2"
                    sx={{
                      color: statusColor,
                      fontWeight: 600,
                      lineHeight: 1.5,
                      fontSize: '12.5px',
                    }}
                  >
                    {proxyReady
                      ? t('layout.components.firstRun.readyOk')
                      : gaveUp
                        ? t('layout.components.firstRun.readyUnconfirmed')
                        : t('layout.components.firstRun.checking')}
                  </Typography>
                </Box>

                {/* 当前活跃出口节点与实测毫秒级延迟：标签与节点全称上下分行，避免长节点名被截断 */}
                {activeNodeName && (
                  <Box
                    sx={{
                      mt: 1,
                      pt: 0.85,
                      borderTop: '1px dashed rgba(128, 128, 128, 0.25)',
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 0.4,
                    }}
                  >
                    <Box
                      sx={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        gap: 1,
                      }}
                    >
                      <Typography
                        variant="caption"
                        sx={{
                          color: secondaryText,
                          fontWeight: 600,
                          fontSize: '11.5px',
                        }}
                      >
                        🌐 {t('layout.components.firstRun.activeNodeLabel')}
                      </Typography>
                      {proxyReady && activeDelay !== null && (
                        <Box
                          sx={{
                            px: 0.85,
                            py: 0.15,
                            borderRadius: '999px',
                            fontSize: '11px',
                            fontWeight: 700,
                            color: readyColor,
                            bgcolor: 'rgba(76, 175, 80, 0.18)',
                            flexShrink: 0,
                          }}
                        >
                          ⚡ {activeDelay} ms
                        </Box>
                      )}
                    </Box>
                    <Typography
                      variant="body2"
                      sx={{
                        color: 'text.primary',
                        fontWeight: 700,
                        fontSize: '13px',
                        lineHeight: 1.4,
                        wordBreak: 'break-word',
                      }}
                    >
                      {activeNodeName}
                    </Typography>
                  </Box>
                )}

                {!sysproxy?.enable && (
                  <Typography
                    variant="body2"
                    sx={{
                      color: warningColor,
                      fontWeight: 600,
                      mt: 0.75,
                      fontSize: '12px',
                      lineHeight: 1.5,
                    }}
                  >
                    ⚠️ {t('layout.components.firstRun.sysProxyOff')}
                  </Typography>
                )}
              </Box>

              {/* 卡片 2：金色星标 GitHub Star 开源应援卡片 */}
              <Box
                sx={{
                  p: 1.5,
                  borderRadius: '8px',
                  bgcolor: isRetro3D
                    ? 'rgba(255, 255, 255, 0.22)'
                    : (t2) =>
                        t2.palette.mode === 'dark'
                          ? 'rgba(255, 193, 7, 0.09)'
                          : 'rgba(255, 193, 7, 0.1)',
                  border: '1px solid',
                  borderColor: isRetro3D
                    ? 'rgba(122, 62, 0, 0.35)'
                    : 'rgba(255, 193, 7, 0.45)',
                }}
              >
                <Typography
                  variant="body2"
                  sx={{
                    fontWeight: 700,
                    color: starAccentColor,
                    mb: 0.5,
                    fontSize: '13.5px',
                    lineHeight: 1.45,
                  }}
                >
                  ⭐ {t('layout.components.firstRun.starTitle')}
                </Typography>

                <Typography
                  variant="body2"
                  sx={{
                    color: secondaryText,
                    lineHeight: 1.55,
                    fontSize: '12.5px',
                  }}
                >
                  {t('layout.components.firstRun.wikiHint')}
                </Typography>

                <Box
                  sx={{
                    mt: 0.75,
                    display: 'flex',
                    justifyContent: 'flex-end',
                  }}
                >
                  <Button
                    size="small"
                    variant="text"
                    onClick={handleOpenWikiOnly}
                    sx={{
                      fontSize: '11.5px',
                      fontWeight: 600,
                      textTransform: 'none',
                      color: 'primary.main',
                      py: 0.2,
                      px: 0.75,
                      minHeight: 'auto',
                    }}
                  >
                    📖 {t('layout.components.firstRun.openWiki')} ↗
                  </Button>
                </Box>
              </Box>
            </DialogContent>

            <DialogActions
              disableSpacing
              sx={{
                flexDirection: 'column',
                alignItems: 'center',
                px: 1.75,
                py: 1.5,
                gap: 1,
                borderTop: (t2) => `1px solid ${t2.palette.divider}`,
              }}
            >
              <Button
                fullWidth
                sx={{
                  ...get3DButtonStyle(theme, 'contained', 'primary'),
                  py: 0.85,
                  fontSize: '13px',
                }}
                onClick={handleOpenGithubStarAndFinish}
              >
                ⭐ {t('layout.components.firstRun.openGithubStar')}
              </Button>
              <Button
                fullWidth
                sx={{
                  ...get3DButtonStyle(theme, 'outlined', 'default'),
                  py: 0.7,
                  fontSize: '12.5px',
                }}
                onClick={handleFinish}
              >
                {t('layout.components.firstRun.finish')}
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>

      {/* Input Context Menu */}
      <Menu
        anchorReference="anchorPosition"
        anchorPosition={
          contextMenu !== null
            ? {
                top: contextMenu.mouseY,
                left: contextMenu.mouseX,
              }
            : undefined
        }
        open={contextMenu !== null}
        onClose={() => setContextMenu(null)}
        slotProps={{
          paper: {
            className: 'theme-panel',
            sx: {
              minWidth: '150px',
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
          onClick={handleCut}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          ✂️ {t('shared.actions.cut')}
        </MenuItem>
        <MenuItem
          onClick={handleCopy}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📋 {t('shared.actions.copy')}
        </MenuItem>
        <MenuItem
          onClick={handlePaste}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          📥 {t('shared.actions.paste')}
        </MenuItem>
        <MenuItem
          onClick={handleSelectAll}
          disabled={!url}
          sx={getMenuItemHoverStyle(theme, controlSkin)}
        >
          🔍 {t('shared.actions.selectAll')}
        </MenuItem>
        <Divider sx={{ my: '4px', borderColor: 'rgba(255, 255, 255, 0.12)' }} />
        <MenuItem
          onClick={handleClear}
          disabled={!url}
          sx={{
            ...getMenuItemHoverStyle(theme, controlSkin),
            color: 'error.main',
          }}
        >
          🧹 {t('shared.actions.clearInput')}
        </MenuItem>
      </Menu>
    </>
  )
}
