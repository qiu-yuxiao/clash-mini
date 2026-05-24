import { CloseRounded } from '@mui/icons-material'
import {
  styled,
  ListItem,
  IconButton,
  ListItemText,
  Box,
  Button,
  alpha,
} from '@mui/material'
import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import { useTranslation } from 'react-i18next'
import { closeConnection } from 'tauri-plugin-mihomo-api'

import parseTraffic from '@/utils/parse-traffic'
import { addQuickRoutingRule } from '@/utils/quick-routing'

const Tag = styled('span')(({ theme }) => ({
  fontSize: '10px',
  padding: '0 4px',
  lineHeight: 1.375,
  border: '1px solid',
  borderRadius: 4,
  borderColor: alpha(theme.palette.text.secondary, 0.35),
  marginTop: '4px',
  marginRight: '4px',
}))

interface Props {
  value: IConnectionsItem
  closed: boolean
  onShowDetail?: (el: HTMLElement) => void
}

export const ConnectionItem = (props: Props) => {
  const { value, closed, onShowDetail } = props

  const { id, metadata, chains, start, curUpload, curDownload } = value
  const { t } = useTranslation()

  const onDelete = useLockFn(async () => closeConnection(id))
  const showTraffic = curUpload! >= 100 || curDownload! >= 100

  // 快捷路由跳转处理：进程优先，域名次之
  const handleQuickRoute = async (target: 'DIRECT' | 'PROXY') => {
    if (metadata.process) {
      await addQuickRoutingRule('process', metadata.process, target)
    } else if (metadata.host) {
      await addQuickRoutingRule('domain', metadata.host, target)
    } else if (metadata.destinationIP) {
      await addQuickRoutingRule('domain', metadata.destinationIP, target)
    }
  }

  return (
    <ListItem
      dense
      sx={{
        borderBottom: '1px solid var(--divider-color)',
        '&:hover .quick-route-actions': { display: 'flex !important' },
      }}
      secondaryAction={
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          {/* 快捷直连与代理悬浮按钮：默认隐藏，hover时显示 */}
          <Box
            className="quick-route-actions"
            sx={{
              display: 'none',
              alignItems: 'center',
              gap: 0.5,
              mr: 1,
            }}
          >
            <Button
              size="small"
              variant="outlined"
              color="success"
              onClick={() => handleQuickRoute('DIRECT')}
              className="aero-crystal-btn-success"
              sx={{
                fontSize: '10px',
                py: 0.1,
                px: 1.2,
                minWidth: 'auto',
                height: 22,
                textTransform: 'none',
                lineHeight: 1.8,
              }}
            >
              直连
            </Button>
            <Button
              size="small"
              variant="outlined"
              color="primary"
              onClick={() => handleQuickRoute('PROXY')}
              className="aero-crystal-btn-primary"
              sx={{
                fontSize: '10px',
                py: 0.1,
                px: 1.2,
                minWidth: 'auto',
                height: 22,
                textTransform: 'none',
                lineHeight: 1.8,
              }}
            >
              代理
            </Button>
          </Box>

          {!closed && (
            <IconButton
              edge="end"
              color="inherit"
              onClick={onDelete}
              title={t('connections.components.actions.closeConnection')}
              aria-label={t('connections.components.actions.closeConnection')}
            >
              <CloseRounded />
            </IconButton>
          )}
        </Box>
      }
    >
      <ListItemText
        sx={{ userSelect: 'text', cursor: 'pointer' }}
        primary={metadata.host || metadata.destinationIP}
        onClick={(e) => onShowDetail?.(e.currentTarget as HTMLElement)}
        secondary={
          <Box sx={{ display: 'flex', flexWrap: 'wrap' }}>
            <Tag sx={{ textTransform: 'uppercase', color: 'success' }}>
              {metadata.network}
            </Tag>

            <Tag>{metadata.type}</Tag>

            {!!metadata.process && <Tag>{metadata.process}</Tag>}

            {chains?.length > 0 && (
              <Tag>{[...chains].reverse().join(' / ')}</Tag>
            )}

            <Tag>{dayjs(start).fromNow()}</Tag>

            {showTraffic && (
              <Tag>
                {parseTraffic(curUpload!)} / {parseTraffic(curDownload!)}
              </Tag>
            )}
          </Box>
        }
      />
    </ListItem>
  )
}
