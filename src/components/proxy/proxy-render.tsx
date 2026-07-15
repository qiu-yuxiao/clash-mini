import { InboxRounded } from '@mui/icons-material'
import { Box, Typography } from '@mui/material'
import { memo, useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { useIconCache } from '@/hooks/use-icon-cache'
import { useVerge } from '@/hooks/use-verge'
import { useThemeMode } from '@/services/states'

import { ProxyHead } from './proxy-head'
import { ProxyItem } from './proxy-item'
import { DEFAULT_STATE } from './use-head-state'
import type { HeadState } from './use-head-state'
import type { IRenderItem } from './use-render-list'

interface RenderProps {
  item: IRenderItem
  indent: boolean
  isTesting?: boolean
  onLocation: (group: IRenderItem['group']) => void
  onCheckAll: (groupName: string) => void
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
  onChangeProxy: (
    group: IRenderItem['group'],
    proxy: IRenderItem['proxy'] & { name: string },
  ) => void
}

const ProxyRenderComponent = (props: RenderProps) => {
  const { t: _t } = useTranslation()
  const {
    indent,
    item,
    onLocation,
    onCheckAll,
    onHeadState,
    onChangeProxy,
    isTesting,
  } = props
  const { type, group, headState, proxy, proxyCol, col } = item
  const { verge } = useVerge()
  const enable_group_icon = verge?.enable_group_icon ?? true
  const mode = useThemeMode()
  const isDark = mode === 'light' ? false : true
  const _itembackgroundcolor = isDark ? '#282A36' : '#ffffff'
  const _iconCachePath = useIconCache({
    icon: group?.icon,
    cacheKey: (group?.name ?? '').replaceAll(' ', ''),
    enabled: enable_group_icon,
  })

  const showType = headState?.showType
  const proxyColItemsMemo = useMemo(() => {
    if (type !== 4 || !proxyCol) {
      return null
    }

    return proxyCol.map((proxyItem, idx) => (
      <ProxyItem
        key={`${item.key}-${proxyItem?.name ?? 'unknown'}`}
        group={group}
        proxy={proxyItem}
        selected={group?.now === proxyItem?.name}
        showType={showType}
        indexInGroup={item.indexInGroup}
        sx={{
          py: 0,
          pl: 0,
          ...(idx < (col || 3) - 1
            ? {
                borderRight: '5px double var(--theme-border)',
              }
            : {}),
        }}
        onClick={() => onChangeProxy(group, proxyItem)}
      />
    ))
  }, [
    type,
    proxyCol,
    item.key,
    group,
    showType,
    onChangeProxy,
    item.indexInGroup,
    col,
  ])

  if (type === 1) {
    return (
      <ProxyHead
        sx={{ pl: 2, pr: 3, mt: indent ? 1 : 0.5, mb: 1 }}
        url={group?.testUrl}
        groupName={group?.name ?? ''}
        headState={headState ?? DEFAULT_STATE}
        isTesting={isTesting}
        onLocation={() => onLocation(group)}
        onCheckDelay={() => onCheckAll(group?.name ?? '')}
        onHeadState={(p) => onHeadState(group?.name ?? '', p)}
      />
    )
  }

  if (type === 2) {
    if (!proxy) return null
    return (
      <ProxyItem
        group={group}
        proxy={proxy}
        selected={group?.now === proxy?.name}
        showType={headState?.showType}
        indexInGroup={item.indexInGroup}
        sx={{ py: 0, pl: 2 }}
        onClick={() => onChangeProxy(group, proxy)}
      />
    )
  }

  if (type === 3) {
    return (
      <Box
        sx={{
          py: 2,
          pl: 0,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <InboxRounded sx={{ fontSize: '2.5em', color: 'inherit' }} />
        <Typography sx={{ color: 'inherit' }}>No Proxies</Typography>
      </Box>
    )
  }

  if (type === 4) {
    return (
      <Box
        sx={{
          minHeight: '24px',
          height: 'auto',
          display: 'grid',
          gridTemplateColumns: `repeat(${col || 3}, 1fr)`,
          pl: 0,
        }}
      >
        {proxyColItemsMemo}
      </Box>
    )
  }

  return null
}

export const ProxyRender = memo(ProxyRenderComponent)
