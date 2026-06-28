import {
  BoltOutlined,
  SortOutlined,
  AccessTimeOutlined,
  SortByAlphaOutlined,
} from '@mui/icons-material'
import { Box, IconButton, SxProps, Theme, keyframes } from '@mui/material'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import { useVerge } from '@/hooks/use-verge'
import delayManager from '@/services/delay'
import { debugLog } from '@/utils/debug'

import type { ProxySortType } from './use-filter-sort'
import type { HeadState } from './use-head-state'
import { useWindowWidth } from './use-window-width'

interface Props {
  sx?: SxProps<Theme>
  url?: string
  groupName: string
  headState: HeadState
  isTesting?: boolean
  onLocation: () => void
  onCheckDelay: () => void
  onHeadState: (val: Partial<HeadState>) => void
}

const pulseGlow = keyframes`
  0% {
    transform: scale(1);
    filter: drop-shadow(0 0 1px rgba(255, 193, 7, 0.4));
    opacity: 0.8;
  }
  50% {
    transform: scale(1.15) rotate(15deg);
    filter: drop-shadow(0 0 8px rgba(255, 193, 7, 0.85));
    opacity: 1;
    color: #ffc107;
  }
  100% {
    transform: scale(1);
    filter: drop-shadow(0 0 1px rgba(255, 193, 7, 0.4));
    opacity: 0.8;
  }
`

const defaultSx: SxProps<Theme> = {}

export const ProxyHead = ({
  sx = defaultSx,
  url,
  groupName,
  headState,
  isTesting = false,
  onHeadState,
  onCheckDelay,
  // We keep onLocation in Props to avoid breaking other files, but we don't use it here.
}: Props) => {
  const { width } = useWindowWidth()
  const isMinimal = width <= 285

  const {
    sortType,
    filterText,
    testUrl,
    filterMatchCase,
    filterMatchWholeWord,
    filterUseRegularExpression,
  } = headState

  const { t } = useTranslation()
  const [autoFocus, setAutoFocus] = useState(false)

  useEffect(() => {
    // fix the focus conflict
    const timer = setTimeout(() => setAutoFocus(true), 100)
    return () => clearTimeout(timer)
  }, [])

  const { verge } = useVerge()
  const defaultLatencyUrl =
    verge?.default_latency_test?.trim() ||
    'http://cp.cloudflare.com/generate_204'

  useEffect(() => {
    delayManager.setUrl(groupName, testUrl?.trim() || url || defaultLatencyUrl)
  }, [groupName, testUrl, defaultLatencyUrl, url])

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 1,
        borderBottom: (theme) => `1px solid ${theme.palette.divider}`,
        pb: 0.5,
        mb: 0.5,
        ...sx,
      }}
    >
      <IconButton
        size="small"
        color="inherit"
        title={t('proxies.page.tooltips.delayCheck')}
        aria-label={t('proxies.page.tooltips.delayCheck')}
        onClick={() => {
          debugLog(`[ProxyHead] 点击延迟测试按钮，组: ${groupName}`)
          onCheckDelay()
        }}
        sx={{ width: 26, height: 26, p: 0 }}
      >
        <BoltOutlined
          aria-hidden="true"
          sx={{
            fontSize: 17,
            animation: isTesting
              ? `${pulseGlow} 1.2s infinite ease-in-out`
              : 'none',
            color: (theme) => isTesting
              ? (theme.palette.mode === 'dark' ? 'warning.main' : 'warning.dark')
              : 'inherit',
          }}
        />
      </IconButton>

      <IconButton
        size="small"
        color="inherit"
        title={
          [
            t('proxies.page.tooltips.sortDefault'),
            t('proxies.page.tooltips.sortDelay'),
            t('proxies.page.tooltips.sortName'),
          ][sortType]
        }
        aria-label={
          [
            t('proxies.page.tooltips.sortDefault'),
            t('proxies.page.tooltips.sortDelay'),
            t('proxies.page.tooltips.sortName'),
          ][sortType]
        }
        onClick={() =>
          onHeadState({ sortType: ((sortType + 1) % 3) as ProxySortType })
        }
        sx={{ width: 26, height: 26, p: 0 }}
      >
        {sortType !== 1 && sortType !== 2 && (
          <SortOutlined aria-hidden="true" sx={{ fontSize: 17 }} />
        )}
        {sortType === 1 && <AccessTimeOutlined aria-hidden="true" sx={{ fontSize: 17 }} />}
        {sortType === 2 && <SortByAlphaOutlined aria-hidden="true" sx={{ fontSize: 17 }} />}
      </IconButton>

      <Box
        sx={{
          ml: 0.5,
          flex: '1 1 auto',
          '& input': { py: 0.3, px: 0.5, fontSize: 11 },
        }}
      >
        <BaseSearchBox
          autoFocus={autoFocus}
          minimal={isMinimal}
          value={filterText}
          searchState={{
            matchCase: filterMatchCase,
            matchWholeWord: filterMatchWholeWord,
            useRegularExpression: filterUseRegularExpression,
          }}
          onSearch={(_, state) =>
            onHeadState({
              filterText: state.text,
              filterMatchCase: state.matchCase,
              filterMatchWholeWord: state.matchWholeWord,
              filterUseRegularExpression: state.useRegularExpression,
            })
          }
        />
      </Box>
    </Box>
  )
}
