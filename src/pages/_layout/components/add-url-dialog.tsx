import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
  useTheme,
} from '@mui/material'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import React, { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  closeConnectionWithTimeout,
  getConnectionsWithTimeout,
} from '@/services/mihomo-api'
import { showNotice } from '@/services/notice-service'
import {
  get3DButtonStyle,
  get3DInputStyle,
  get3DSegmentedActiveStyle,
  get3DSegmentedActiveTextColor,
  get3DSegmentedContainerStyle,
} from '@/utils/button-styles'
import { findMatchingManagedRule } from '@/utils/path-control'
import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupActionsSx,
  getPopupContentSx,
  getPopupSkinTokens,
  getPopupTitleSx,
  useNarrowOverlayTopOffset,
} from '@/utils/popup-styles'
import {
  addQuickRoutingRules,
  buildQuickRuleFromInput,
  parseManagedMergeRuleLine,
  type ManagedMergeRule,
  type QuickRoutingTarget,
} from '@/utils/quick-routing'

import { InputContextMenu } from './input-context-menu'

interface AddUrlDialogProps {
  open: boolean
  onClose: () => void
}

export const AddUrlDialog: React.FC<AddUrlDialogProps> = ({
  open,
  onClose,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const tokens = getPopupSkinTokens(theme)
  const topOffset = useNarrowOverlayTopOffset()
  const [text, setText] = useState('')
  const [routeTarget, setRouteTarget] = useState<QuickRoutingTarget>('PROXY')
  const [submitting, setSubmitting] = useState(false)

  // Context Menu State for Input Box (Mouse Paste Bug Fix)
  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
    hasSelection: boolean
  } | null>(null)
  const inputRef = useRef<HTMLTextAreaElement | HTMLInputElement>(null)

  const handleClose = () => {
    if (submitting) return
    setText('')
    setRouteTarget('PROXY')
    onClose()
  }

  const handleAdd = async () => {
    if (!text.trim()) {
      handleClose()
      return
    }
    setSubmitting(true)
    try {
      const count = await addQuickRoutingRules(text, routeTarget)
      if (count > 0) {
        try {
          const parsedRules = text
            .split(/[\n,;]+/)
            .map((line) => buildQuickRuleFromInput(line, routeTarget))
            .filter((rule): rule is string => Boolean(rule))
            .map((rule) => parseManagedMergeRuleLine(rule))
            .filter((rule): rule is ManagedMergeRule => Boolean(rule))

          if (parsedRules.length > 0) {
            const snapshot = await getConnectionsWithTimeout()
            const activeConns = Array.isArray(snapshot?.connections)
              ? snapshot.connections
              : []
            const matchingIds = activeConns
              .filter((conn) =>
                Boolean(findMatchingManagedRule(conn, parsedRules)),
              )
              .map((conn) => conn.id)
              .filter(Boolean)

            if (matchingIds.length > 0) {
              await Promise.allSettled(
                matchingIds.map((id) => closeConnectionWithTimeout(id)),
              )
            }
          }
        } catch {
          // Best-effort connection refresh; ignore when core is offline.
        }
        showNotice.success(t('settings.mini.addUrlSuccess', { count }))
      } else {
        showNotice.info(
          t('settings.mini.addUrlNoChange', {
            defaultValue: '没有新增规则（可能已存在或输入无效）',
          }),
        )
      }
      setText('')
      setRouteTarget('PROXY')
      onClose()
    } catch (e: unknown) {
      showNotice.error(e instanceof Error ? e.message : String(e))
    } finally {
      setSubmitting(false)
    }
  }

  const handleContextMenu = (event: React.MouseEvent) => {
    event.preventDefault()
    const input = inputRef.current
    const selectionStart = input?.selectionStart ?? 0
    const selectionEnd = input?.selectionEnd ?? 0
    setContextMenu({
      mouseX: event.clientX,
      mouseY: event.clientY,
      hasSelection: selectionStart !== selectionEnd,
    })
  }

  const handleCut = async () => {
    setContextMenu(null)
    const input = inputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const selectedText = text.substring(start, end)
      if (selectedText) {
        try {
          await writeText(selectedText)
        } catch {
          try {
            await navigator.clipboard.writeText(selectedText)
          } catch (e) {
            console.error('Failed to copy:', e)
          }
        }
        const newValue = text.substring(0, start) + text.substring(end)
        setText(newValue)
        setTimeout(() => {
          input.focus()
          input.setSelectionRange(start, start)
        }, 0)
      }
    }
  }

  const handleCopy = async () => {
    setContextMenu(null)
    const input = inputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const selectedText = text.substring(start, end)
      if (selectedText) {
        try {
          await writeText(selectedText)
        } catch {
          try {
            await navigator.clipboard.writeText(selectedText)
          } catch (e) {
            console.error('Failed to copy:', e)
          }
        }
      }
    }
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
        console.error('Failed to read from clipboard:', e)
      }
    }

    if (!textToPaste) return

    const input = inputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const newValue =
        text.substring(0, start) + textToPaste + text.substring(end)
      setText(newValue)
      setTimeout(() => {
        input.focus()
        const newCursorPos = start + textToPaste.length
        input.setSelectionRange(newCursorPos, newCursorPos)
      }, 0)
    } else {
      setText(text + textToPaste)
    }
  }

  const handleSelectAll = () => {
    setContextMenu(null)
    const input = inputRef.current
    if (input) {
      input.focus()
      input.setSelectionRange(0, text.length)
    }
  }

  const handleClear = () => {
    setContextMenu(null)
    setText('')
    const input = inputRef.current
    if (input) {
      input.focus()
    }
  }

  const hasSelection = Boolean(contextMenu?.hasSelection)
  const targetOptions: ReadonlyArray<{
    value: QuickRoutingTarget
    label: string
  }> = [
    {
      value: 'PROXY',
      label: t('connections.components.miniTable.chainsProxy'),
    },
    {
      value: 'DIRECT',
      label: t('connections.components.miniTable.chainsDirect'),
    },
    {
      value: 'REJECT',
      label: t('connections.components.miniTable.chainsReject'),
    },
  ]
  const activeTargetIndex = Math.max(
    0,
    targetOptions.findIndex((item) => item.value === routeTarget),
  )

  return (
    <>
      <Dialog
        open={open}
        onClose={handleClose}
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
            <span>🌐</span>
            <span>
              {t('settings.mini.addUrl', { defaultValue: '添加规则' })}
            </span>
          </Box>
        </DialogTitle>

        <DialogContent sx={getPopupContentSx()}>
          <Box
            role="tablist"
            sx={(tTheme) => ({
              position: 'relative',
              display: 'grid',
              gridTemplateColumns: 'repeat(3, 1fr)',
              alignItems: 'center',
              boxSizing: 'border-box',
              mb: 1,
              height: 24,
              p: '1px',
              borderRadius: '4px',
              bgcolor: 'action.hover',
              userSelect: 'none',
              ...get3DSegmentedContainerStyle(tTheme),
            })}
          >
            <Box
              sx={{
                position: 'absolute',
                left: 0,
                top: 0,
                width: '33.333%',
                height: '100%',
                zIndex: 0,
                pointerEvents: 'none',
                transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                transform: `translate3d(${activeTargetIndex * 100}%, 0, 0)`,
              }}
            >
              <Box sx={(tTheme) => get3DSegmentedActiveStyle(tTheme)} />
            </Box>
            {targetOptions.map((opt) => {
              const isSelected = routeTarget === opt.value
              return (
                <Box
                  key={opt.value}
                  role="tab"
                  aria-selected={isSelected}
                  onClick={() => {
                    if (!submitting) {
                      setRouteTarget(opt.value)
                    }
                  }}
                  sx={{
                    height: '100%',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    px: 0.5,
                    fontSize: '12px',
                    fontWeight: 'bold',
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    cursor: submitting ? 'not-allowed' : 'pointer',
                    zIndex: 1,
                    transition: 'color 0.2s ease',
                    color: isSelected
                      ? get3DSegmentedActiveTextColor(theme)
                      : tokens.secondaryText,
                    '&:hover': {
                      color: isSelected
                        ? get3DSegmentedActiveTextColor(theme)
                        : tokens.primaryText,
                    },
                  }}
                >
                  {opt.label}
                </Box>
              )
            })}
          </Box>
          <TextField
            autoFocus
            multiline
            minRows={4}
            maxRows={10}
            fullWidth
            value={text}
            disabled={submitting}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('settings.mini.addUrlPlaceholder', {
              defaultValue:
                '每行一个网址或 IP\n根域名（如 oracle.com）或 *.前缀匹配整站\n子域名（如 play.googleapis.com）仅精确匹配',
            })}
            sx={get3DInputStyle(theme)}
            inputRef={inputRef}
            onContextMenu={handleContextMenu}
          />
        </DialogContent>

        <DialogActions disableSpacing sx={getPopupActionsSx(theme)}>
          <Button
            fullWidth
            sx={{
              ...get3DButtonStyle(theme, 'contained', 'primary'),
              py: 0.85,
              fontSize: '13px',
            }}
            onClick={handleAdd}
            disabled={submitting}
          >
            {t('settings.mini.addUrlButton', { defaultValue: '添加' })}
          </Button>
          <Button
            fullWidth
            sx={{
              ...get3DButtonStyle(theme, 'outlined', 'default'),
              py: 0.7,
              fontSize: '12.5px',
            }}
            onClick={handleClose}
            disabled={submitting}
          >
            {t('shared.actions.cancel', { defaultValue: '取消' })}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Input Context Menu */}
      <InputContextMenu
        anchorPosition={
          contextMenu !== null
            ? { top: contextMenu.mouseY, left: contextMenu.mouseX }
            : null
        }
        onClose={() => setContextMenu(null)}
        canCut={hasSelection}
        canCopy={hasSelection}
        canSelectAll={Boolean(text)}
        canClear={Boolean(text)}
        onCut={handleCut}
        onCopy={handleCopy}
        onPaste={handlePaste}
        onSelectAll={handleSelectAll}
        onClear={handleClear}
        minWidth="160px"
      />
    </>
  )
}
