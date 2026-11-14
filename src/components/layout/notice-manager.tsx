import { CloseRounded } from '@mui/icons-material'
import { Alert, Box, IconButton, useTheme } from '@mui/material'
import React, { useCallback, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'

import { MIN_WINDOW_WIDTH } from '@/constants'
import {
  getSnapshotNotices,
  hideNotice,
  showNotice,
  subscribeNotices,
} from '@/services/notice-service'
import type { TranslationKey } from '@/types/generated/i18n-keys'
import type { IVergeConfig } from '@/types/verge'
import { getActiveSkin } from '@/utils/button-styles'
import { useNarrowOverlayTopOffset } from '@/utils/popup-styles'

type NoticePosition = NonNullable<IVergeConfig['notice_position']>
type NoticeItem = ReturnType<typeof getSnapshotNotices>[number]
type TranslationFn = ReturnType<typeof useTranslation>['t']

const resolveNoticeMessage = (
  notice: NoticeItem,
  t: TranslationFn,
): React.ReactNode => {
  const i18n = notice.i18n
  if (!i18n) return notice.message

  const params = (i18n.params ?? {}) as Record<string, unknown>
  const { prefixKey, prefixParams, prefix, message, ...restParams } = params

  const prefixKeyParams =
    prefixParams && typeof prefixParams === 'object'
      ? (prefixParams as Record<string, unknown>)
      : undefined

  const resolvedPrefix =
    typeof prefixKey === 'string'
      ? t(prefixKey as TranslationKey, {
          defaultValue: prefixKey,
          ...(prefixKeyParams ?? {}),
          ...restParams,
        })
      : typeof prefix === 'string'
        ? prefix
        : undefined

  const messageStr = typeof message === 'string' ? message : undefined

  const defaultValue =
    messageStr === undefined
      ? undefined
      : resolvedPrefix
        ? `${resolvedPrefix} ${messageStr}`
        : messageStr

  return t(i18n.key as TranslationKey, {
    defaultValue,
    ...restParams,
    ...(resolvedPrefix !== undefined ? { prefix: resolvedPrefix } : {}),
    ...(messageStr !== undefined ? { message: messageStr } : {}),
  })
}

const extractNoticeCopyText = (input: unknown): string | undefined => {
  if (input === null || input === undefined) return undefined
  if (typeof input === 'string') return input
  if (typeof input === 'number' || typeof input === 'boolean') {
    return String(input)
  }
  if (input instanceof Error) {
    return input.message || input.name
  }
  if (React.isValidElement(input)) return undefined
  if (typeof input === 'object') {
    const maybeMessage = (input as { message?: unknown }).message
    if (typeof maybeMessage === 'string') return maybeMessage
  }
  try {
    return JSON.stringify(input)
  } catch {
    return String(input)
  }
}

const resolveNoticeCopyText = (
  notice: NoticeItem,
  t: TranslationFn,
): string | undefined => {
  if (
    notice.i18n?.key === 'shared.feedback.notices.prefixedRaw' ||
    notice.i18n?.key === 'shared.feedback.notices.raw'
  ) {
    const rawText = extractNoticeCopyText(notice.i18n?.params?.message)
    if (rawText) return rawText
  }

  return (
    extractNoticeCopyText(resolveNoticeMessage(notice, t)) ??
    extractNoticeCopyText(notice.message)
  )
}

interface NoticeManagerProps {
  position?: NoticePosition | null
}

export const NoticeManager: React.FC<NoticeManagerProps> = ({
  position: _position,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const topOffset = useNarrowOverlayTopOffset()
  const skin = getActiveSkin(theme)
  const borderRadius =
    skin === 'cyberpunk' || skin === 'monochrome' ? '0px' : '6px'

  const currentNotices = useSyncExternalStore(
    subscribeNotices,
    getSnapshotNotices,
  )

  const handleClose = (id: number) => {
    hideNotice(id)
  }

  const handleNoticeCopy = useCallback(
    async (notice: NoticeItem) => {
      const text = resolveNoticeCopyText(notice, t)
      if (!text) return
      try {
        await navigator.clipboard.writeText(text)
        showNotice.success(
          'shared.feedback.notifications.common.copySuccess',
          1000,
        )
      } catch (error) {
        console.warn('[NoticeManager] copy to clipboard failed:', error)
      }
    },
    [t],
  )

  return (
    <Box
      sx={{
        position: 'fixed',
        top: `${topOffset}px`,
        left: 0,
        right: 'auto',
        bottom: 'auto',
        width: `${MIN_WINDOW_WIDTH}px`,
        minWidth: `${MIN_WINDOW_WIDTH}px`,
        maxWidth: `${MIN_WINDOW_WIDTH}px`,
        boxSizing: 'border-box',
        zIndex: 1500,
        display: 'flex',
        flexDirection: 'column',
        gap: '4px',
        pointerEvents: 'none',
      }}
    >
      {currentNotices.map((notice) => (
        <Box
          key={notice.id}
          sx={{
            width: '100%',
            minWidth: 0,
            maxWidth: '100%',
            boxSizing: 'border-box',
            pointerEvents: 'auto',
          }}
        >
          <Alert
            severity={notice.type}
            variant="filled"
            sx={{
              width: '100%',
              minWidth: 0,
              maxWidth: '100%',
              boxSizing: 'border-box',
              borderRadius,
              px: 1.25,
              py: 0.5,
              fontSize: '12px',
              lineHeight: 1.45,
              fontFamily: 'var(--control-font-family)',
              boxShadow: '0 4px 12px rgba(0, 0, 0, 0.28)',
              '& .MuiAlert-icon': {
                py: '3px',
                mr: 1,
                fontSize: '18px',
              },
              '& .MuiAlert-message': {
                py: '2px',
                flex: 1,
                minWidth: 0,
                overflowWrap: 'anywhere',
                wordBreak: 'break-word',
              },
              '& .MuiAlert-action': {
                pt: 0,
                pl: 0.5,
                mr: -0.5,
                alignItems: 'center',
              },
            }}
            onContextMenu={(event) => {
              event.preventDefault()
              event.stopPropagation()
              void handleNoticeCopy(notice)
            }}
            action={
              <IconButton
                size="small"
                color="inherit"
                onClick={() => handleClose(notice.id)}
              >
                <CloseRounded fontSize="inherit" />
              </IconButton>
            }
          >
            {resolveNoticeMessage(notice, t)}
          </Alert>
        </Box>
      ))}
    </Box>
  )
}
