import { Dialog, useTheme } from '@mui/material'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import type React from 'react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useVerge } from '@/hooks/use-verge'
import {
  useAppRefreshers,
  useProxiesData,
  useSystemData,
} from '@/providers/app-data-context'
import { getProfiles, triggerAutoSelect } from '@/services/cmds'
import { getDelayManager, NODE_DELAY_MAX_MS } from '@/services/delay'
import { isSupportedSubscriptionOrNodeInput } from '@/utils/node'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupSkinTokens,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'

import { InputContextMenu } from '../input-context-menu'

import { FirstRunInputStep } from './input-step'
import { FirstRunReadyStep } from './ready-step'

export const CLASH_MINI_GITHUB_URL = 'https://github.com/qiu-yuxiao/clash-mini'
export const CLASH_MINI_WIKI_URL =
  'https://github.com/qiu-yuxiao/clash-mini/wiki'
export const FIRST_RUN_GUIDE_STORAGE_KEY = 'clash-mini-first-run-completed'

/** 就绪判定超时：超时仍未测出有效延迟，就不再宣称「已就绪」，改为如实提示并提供一键重测优选 */
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
  const topOffset = useNarrowOverlayTopOffset()
  const tokens = getPopupSkinTokens(theme)

  const [step, setStep] = useState<'input' | 'ready'>('input')
  const [url, setUrl] = useState('')
  const [detectedClipboardUrl, setDetectedClipboardUrl] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [errorMessage, setErrorMessage] = useState('')
  const [gaveUp, setGaveUp] = useState(false)
  const [retesting, setRetesting] = useState(false)
  const [delayVersion, setDelayVersion] = useState(0)

  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
  } | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const submittingRef = useRef(false)
  const autoRetestedRef = useRef(false)

  // ── 就绪判定：复用细粒度代理与系统上下文，避免订阅全量 AppData 触发无关重绘 ──
  const { proxies } = useProxiesData()
  const { sysproxy } = useSystemData()
  const { verge, patchVerge } = useVerge()
  const { refreshProxy } = useAppRefreshers()
  const refreshProxyRef = useRef(refreshProxy)
  useEffect(() => {
    refreshProxyRef.current = refreshProxy
  }, [refreshProxy])

  // 订阅 DelayManager 的 PROXY 组通知：后端 triggerAutoSelect 注入批量测速结果时立即重算延迟
  useEffect(() => {
    if (!open || step !== 'ready') return
    const onGroupUpdate = () => {
      setDelayVersion((v) => v + 1)
    }
    getDelayManager().setGroupListener('PROXY', onGroupUpdate)
    return () => {
      getDelayManager().removeGroupListener('PROXY', onGroupUpdate)
    }
  }, [open, step])

  const readClipboardSubscriptionUrl =
    useCallback(async (): Promise<string> => {
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
      const trimmed = clip.trim()
      if (trimmed && isSupportedSubscriptionOrNodeInput(trimmed)) {
        return trimmed
      }
      return ''
    }, [])

  /** 当前选中的活跃出口节点名称 */
  const activeNodeName = useMemo(() => {
    const group = proxies?.groups.find((g) => g.name === 'PROXY')
    return group?.now || null
  }, [proxies])

  /** 活跃出口节点最近一次延迟（ms）；优先读取 DelayManager 本次测速缓存，其次回落内核 history */
  const activeDelay = useMemo(() => {
    void delayVersion
    if (!activeNodeName) return null
    const cached = getDelayManager().getDelayUpdate(
      activeNodeName,
      'PROXY',
    )?.delay
    if (
      typeof cached === 'number' &&
      cached > 0 &&
      cached < NODE_DELAY_MAX_MS
    ) {
      return cached
    }
    const node =
      proxies?.records[activeNodeName] ??
      proxies?.proxies.find((p) => p.name === activeNodeName)
    const history = node?.history
    if (history && history.length > 0) {
      const delay = history[history.length - 1]?.delay
      if (typeof delay === 'number') return delay
    }
    return typeof cached === 'number' && cached >= 0 ? cached : null
  }, [proxies, activeNodeName, delayVersion])

  /** 有效延迟（排除 0/超时与假节点阈值）才算真的连通 */
  const proxyReady =
    activeDelay !== null && activeDelay > 0 && activeDelay < NODE_DELAY_MAX_MS

  /** 是否处于需要向用户展示「未确认提示 + 重新测速并优选按钮」的状态 */
  const isUnconfirmed =
    !proxyReady && (gaveUp || (!retesting && activeDelay === 0))

  /**
   * 代理流量接管是否确实处于关闭状态：
   * 1. sysproxy 在设置抽屉未打开时不查询（恒为 undefined），且 TUN 模式下 sysproxy.enable 本就为 false；
   * 2. 因此必须以 verge.enable_system_proxy 或 verge.enable_tun_mode 为准，仅当 verge 已加载且所有接管方式均未开启时才提示。
   */
  const isTakeoverOff = Boolean(
    verge &&
      !verge.enable_system_proxy &&
      !verge.enable_tun_mode &&
      !sysproxy?.enable,
  )

  /** 主动调用后端全节点并发测速并自动切换至最快节点 */
  const handleRetestNodes = useCallback(async () => {
    if (retesting) return
    setRetesting(true)
    setGaveUp(false)
    try {
      const profiles = await getProfiles()
      const currentUid = profiles?.current || ''
      if (currentUid) {
        await triggerAutoSelect(currentUid, undefined, 0, true)
      }
      await refreshProxyRef.current?.({ forceFull: true })
    } catch (err) {
      console.warn('[FirstRunGuide] 重新测速并优选节点失败:', err)
    } finally {
      setRetesting(false)
    }
  }, [retesting])

  // 进入 ready 步骤时立即在后台触发全节点并发测速优选，从订阅全部节点中选出最快节点
  useEffect(() => {
    if (!open || step !== 'ready' || proxyReady || autoRetestedRef.current) {
      return
    }
    autoRetestedRef.current = true
    void (async () => {
      try {
        const profiles = await getProfiles()
        const currentUid = profiles?.current || ''
        if (currentUid) {
          await triggerAutoSelect(currentUid, undefined, 0, true)
        }
        await refreshProxyRef.current?.({ forceFull: true })
      } catch {
        // ignore background auto-select errors
      }
    })()
  }, [open, step, proxyReady])

  // 进入 ready 步骤（或用户点击重测结束）开始计时，超时后停止等待，如实告知「未确认」并展示重测按钮
  useEffect(() => {
    if (!open || step !== 'ready' || proxyReady || retesting) return
    const timer = setTimeout(() => setGaveUp(true), READY_TIMEOUT_MS)
    return () => clearTimeout(timer)
  }, [open, step, proxyReady, retesting])

  // 进入 ready 步骤时立即主动刷新一次，随后在未就绪且未超时期间按 1.5s 周期轮询
  useEffect(() => {
    if (!open || step !== 'ready' || proxyReady || (gaveUp && !retesting)) {
      return
    }
    void refreshProxyRef.current?.()
    const timer = setInterval(() => {
      void refreshProxyRef.current?.()
    }, READY_POLL_INTERVAL_MS)
    return () => clearInterval(timer)
  }, [open, step, proxyReady, gaveUp, retesting])

  // 弹窗打开或窗口切回前台时自动聚焦输入框，并静默探测剪贴板中是否已有待导入的 http(s) 订阅链接
  useEffect(() => {
    if (!open || step !== 'input') return
    const timer1 = setTimeout(() => {
      inputRef.current?.focus()
    }, 100)
    const timer2 = setTimeout(() => {
      if (!document.activeElement || document.activeElement === document.body) {
        inputRef.current?.focus()
      }
    }, 320)

    let cancelled = false
    const detectClipboard = async () => {
      const clipUrl = await readClipboardSubscriptionUrl()
      if (cancelled) return
      setDetectedClipboardUrl(clipUrl)
    }
    void detectClipboard()

    const handleWindowFocusOrVisible = () => {
      if (document.visibilityState === 'hidden') return
      void detectClipboard()
    }
    window.addEventListener('focus', handleWindowFocusOrVisible)
    document.addEventListener('visibilitychange', handleWindowFocusOrVisible)

    return () => {
      cancelled = true
      clearTimeout(timer1)
      clearTimeout(timer2)
      window.removeEventListener('focus', handleWindowFocusOrVisible)
      document.removeEventListener(
        'visibilitychange',
        handleWindowFocusOrVisible,
      )
    }
  }, [open, step, readClipboardSubscriptionUrl])

  const resetForm = () => {
    setStep('input')
    setUrl('')
    setDetectedClipboardUrl('')
    submittingRef.current = false
    autoRetestedRef.current = false
    setSubmitting(false)
    setErrorMessage('')
    setGaveUp(false)
    setRetesting(false)
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
    if (submitting || submittingRef.current) return
    let trimmed = url.trim() || detectedClipboardUrl.trim()
    if (!trimmed) {
      const clipUrl = await readClipboardSubscriptionUrl()
      if (clipUrl) {
        trimmed = clipUrl
        setDetectedClipboardUrl(clipUrl)
      }
    }
    if (!trimmed) {
      setErrorMessage(t('layout.components.firstRun.errEmpty'))
      inputRef.current?.focus()
      return
    }
    if (url.trim() !== trimmed) {
      setUrl(trimmed)
    }

    if (!isSupportedSubscriptionOrNodeInput(trimmed)) {
      setErrorMessage(t('layout.components.firstRun.errScheme'))
      return
    }

    submittingRef.current = true
    setSubmitting(true)
    setErrorMessage('')
    setGaveUp(false)

    try {
      const success = await onImport(trimmed)
      if (success) {
        // 若为首次新手引导且当前既未开启系统代理也未开启 TUN 模式，导入成功后自动开启系统代理；
        // 从帮助菜单再次打开引导时（已存在 FIRST_RUN_GUIDE_STORAGE_KEY === 'true'）尊重用户已有系统代理偏好
        const isReopenedAfterCompletion =
          typeof window !== 'undefined' &&
          localStorage.getItem(FIRST_RUN_GUIDE_STORAGE_KEY) === 'true'
        if (
          !isReopenedAfterCompletion &&
          verge &&
          !verge.enable_system_proxy &&
          !verge.enable_tun_mode
        ) {
          void patchVerge({ enable_system_proxy: true }).catch(() => {})
        }
        setStep('ready')
      } else {
        setErrorMessage(t('layout.components.firstRun.errImport'))
      }
    } catch (err: unknown) {
      console.error('[FirstRunGuide] 导入异常:', err)
      setErrorMessage(t('layout.components.firstRun.errImport'))
    } finally {
      submittingRef.current = false
      setSubmitting(false)
    }
  }

  const handleSubmitImportRef = useRef(handleSubmitImport)
  handleSubmitImportRef.current = handleSubmitImport

  // 【Enter 全窗口监听】除了挂在 Dialog Paper 上，还在 window 级别兜底监听：
  // 首次启动延迟弹窗或异步剪贴板探测后，activeElement 可能停留在 document.body（Paper 之外），
  // 此时仅靠 Paper.onKeyDown 无法收到键盘冒泡事件。双保险 + submittingRef 防重入确保任意焦点位置按 Enter 均生效。
  useEffect(() => {
    if (!open || step !== 'input' || contextMenu !== null) return
    const onWindowKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Enter' || e.isComposing || e.defaultPrevented) return
      if (submittingRef.current) return
      const target = e.target as HTMLElement | null
      if (target?.closest?.('button, [role="menuitem"]')) return
      e.preventDefault()
      void handleSubmitImportRef.current()
    }
    window.addEventListener('keydown', onWindowKeyDown)
    return () => {
      window.removeEventListener('keydown', onWindowKeyDown)
    }
  }, [open, step, contextMenu])

  // 【Enter 弹窗内监听】监听挂在 Dialog Paper 上：弹窗内任意位置按 Enter 都能导入。
  const handleDialogKeyDown = (e: React.KeyboardEvent) => {
    if (e.key !== 'Enter' || e.nativeEvent.isComposing) return
    if (submitting || submittingRef.current || step !== 'input') return
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
        maxWidth={false}
        sx={getNarrowDialogSx(topOffset)}
        slotProps={getNarrowDialogSlotProps(theme, topOffset, {
          onKeyDown: handleDialogKeyDown,
        })}
      >
        {step === 'input' ? (
          <FirstRunInputStep
            url={url}
            detectedClipboardUrl={detectedClipboardUrl}
            submitting={submitting}
            errorMessage={errorMessage}
            inputRef={inputRef}
            isRetro3D={tokens.isRetro3D}
            secondaryText={tokens.secondaryText}
            disabledText={tokens.disabledText}
            readyColor={tokens.readyColor}
            onUrlChange={(value) => {
              setUrl(value)
              if (errorMessage) setErrorMessage('')
            }}
            onFillClipboardUrl={() => {
              if (submitting) return
              setUrl(detectedClipboardUrl)
              setErrorMessage('')
              inputRef.current?.focus()
            }}
            onContextMenu={handleContextMenu}
            onPaste={handlePaste}
            onSubmitImport={() => {
              void handleSubmitImport()
            }}
            onDismissLater={handleDismissLater}
          />
        ) : (
          <FirstRunReadyStep
            proxyReady={proxyReady}
            isUnconfirmed={isUnconfirmed}
            retesting={retesting}
            isTakeoverOff={isTakeoverOff}
            activeNodeName={activeNodeName}
            activeDelay={activeDelay}
            isRetro3D={tokens.isRetro3D}
            primaryText={tokens.primaryText}
            secondaryText={tokens.secondaryText}
            readyColor={tokens.readyColor}
            warningColor={tokens.warningColor}
            onRetestNodes={() => {
              void handleRetestNodes()
            }}
            onOpenWikiOnly={handleOpenWikiOnly}
            onOpenGithubStarAndFinish={handleOpenGithubStarAndFinish}
            onFinish={handleFinish}
          />
        )}
      </Dialog>

      {/* Input Context Menu */}
      <InputContextMenu
        anchorPosition={
          contextMenu !== null
            ? { top: contextMenu.mouseY, left: contextMenu.mouseX }
            : null
        }
        onClose={() => setContextMenu(null)}
        canCut={Boolean(url)}
        canCopy={Boolean(url)}
        canSelectAll={Boolean(url)}
        canClear={Boolean(url)}
        onCut={handleCut}
        onCopy={handleCopy}
        onPaste={handlePaste}
        onSelectAll={handleSelectAll}
        onClear={handleClear}
        minWidth="150px"
      />
    </>
  )
}
