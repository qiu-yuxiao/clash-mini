import { Box, Button, Popover, useTheme } from '@mui/material'
import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import { useImperativeHandle, useState, type Ref } from 'react'
import { useTranslation } from 'react-i18next'

import { closeConnectionWithTimeout } from '@/services/mihomo-api'
import type { IConnectionsItem } from '@/types/connection'
import { get3DButtonStyle } from '@/utils/button-styles'
import parseTraffic from '@/utils/parse-traffic'
import { getPopupCardStyle, getPopupSkinTokens } from '@/utils/popup-styles'

export interface ConnectionDetailRef {
  open: (detail: IConnectionsItem, closed: boolean, el?: HTMLElement) => void
}

export function ConnectionDetail({ ref }: { ref?: Ref<ConnectionDetailRef> }) {
  const theme = useTheme()
  const [open, setOpen] = useState(false)
  const [detail, setDetail] = useState<IConnectionsItem>(null!)
  const [closed, setClosed] = useState(false)
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)

  useImperativeHandle(ref, () => ({
    open: (detail: IConnectionsItem, closed: boolean, el?: HTMLElement) => {
      setDetail(detail)
      setClosed(closed)
      setAnchorEl(el || null)
      setOpen(true)
    },
  }))

  const onClose = () => {
    setOpen(false)
    setAnchorEl(null)
  }

  return (
    <Popover
      open={open}
      anchorEl={anchorEl}
      onClose={onClose}
      anchorOrigin={{
        vertical: 'bottom',
        horizontal: 'left',
      }}
      transformOrigin={{
        vertical: 'top',
        horizontal: 'left',
      }}
      slotProps={{
        paper: {
          className: 'popup-card-paper',
          sx: {
            ...getPopupCardStyle(theme),
            maxWidth: '520px',
            maxHeight: '480px',
            overflowY: 'auto',
            p: 2,
          },
        },
      }}
    >
      {detail ? (
        <InnerConnectionDetail
          data={detail}
          closed={closed}
          onClose={onClose}
        />
      ) : null}
    </Popover>
  )
}

interface InnerProps {
  data: IConnectionsItem
  closed: boolean
  onClose?: () => void
}

const InnerConnectionDetail = ({ data, closed, onClose }: InnerProps) => {
  const { t } = useTranslation()
  const { metadata, rulePayload } = data
  const theme = useTheme()
  const tokens = getPopupSkinTokens(theme)
  const chains = [...(data?.chains ?? [])].reverse().join(' / ')
  const rule = rulePayload ? `${data?.rule}(${rulePayload})` : data?.rule
  const rawHost =
    metadata?.host ||
    metadata?.remoteDestination ||
    metadata?.destinationIP ||
    metadata?.process ||
    '--'
  const host = metadata?.destinationPort
    ? `${rawHost}:${metadata.destinationPort}`
    : rawHost
  const Destination = metadata?.destinationIP
    ? metadata.destinationIP
    : metadata?.remoteDestination || '--'

  const information = [
    { label: t('connections.components.fields.host'), value: host },
    {
      label: t('shared.labels.downloaded'),
      value: parseTraffic(data.download).join(' '),
    },
    {
      label: t('shared.labels.uploaded'),
      value: parseTraffic(data.upload).join(' '),
    },
    {
      label: t('connections.components.fields.dlSpeed'),
      value: parseTraffic(data.curDownload ?? -1).join(' ') + '/s',
    },
    {
      label: t('connections.components.fields.ulSpeed'),
      value: parseTraffic(data.curUpload ?? -1).join(' ') + '/s',
    },
    {
      label: t('connections.components.fields.chains'),
      value: chains,
    },
    { label: t('connections.components.fields.rule'), value: rule },
    {
      label: t('connections.components.fields.process'),
      value: `${metadata?.process ?? ''}${metadata?.processPath ? `(${metadata.processPath})` : ''}`,
    },
    {
      label: t('connections.components.fields.time'),
      value: data?.start ? dayjs(data.start).fromNow() : '--',
    },
    {
      label: t('connections.components.fields.source'),
      value: metadata?.sourceIP
        ? `${metadata.sourceIP}:${metadata?.sourcePort ?? ''}`
        : '--',
    },
    {
      label: t('connections.components.fields.destination'),
      value: Destination,
    },
    {
      label: t('connections.components.fields.destinationPort'),
      value: `${metadata?.destinationPort ?? ''}`,
    },
    {
      label: t('connections.components.fields.type'),
      value: `${metadata?.type ?? ''}(${metadata?.network ?? ''})`,
    },
  ]

  const onDelete = useLockFn(async () => closeConnectionWithTimeout(data.id))

  return (
    <Box
      sx={{
        userSelect: 'text',
        color: tokens.secondaryText,
        fontSize: '12px',
        display: 'flex',
        flexDirection: 'column',
        gap: 0.75,
        fontFamily: 'var(--control-font-family)',
      }}
    >
      {information.map((each) => (
        <Box key={each.label} sx={{ display: 'flex', gap: 1 }}>
          <b style={{ minWidth: 100, display: 'inline-block' }}>
            {each.label}:
          </b>
          <span
            style={{
              wordBreak: 'break-all',
              color: tokens.primaryText,
              flex: 1,
            }}
          >
            {each.value}
          </span>
        </Box>
      ))}

      {!closed && (
        <Box
          sx={{
            textAlign: 'right',
            mt: 1,
            pt: 1.25,
            borderTop: `1px solid ${tokens.dividerColor}`,
          }}
        >
          <Button
            size="small"
            title={t('connections.components.actions.closeConnection')}
            onClick={() => {
              onDelete()
              onClose?.()
            }}
            sx={(currentTheme) => ({
              ...get3DButtonStyle(currentTheme, 'contained', 'error'),
            })}
          >
            {t('connections.components.actions.closeConnection')}
          </Button>
        </Box>
      )}
    </Box>
  )
}
