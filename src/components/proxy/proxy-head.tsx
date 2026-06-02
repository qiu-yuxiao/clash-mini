import {
  MyLocationOutlined,
  BoltOutlined,
  SortOutlined,
  AccessTimeOutlined,
  SortByAlphaOutlined,
  LinkOutlined,
  VisibilityOutlined,
  VisibilityOffOutlined,
  FilterListOutlined,
} from '@mui/icons-material'
import {
  Box,
  IconButton,
  TextField,
  SxProps,
  Theme,
  keyframes,
  useTheme,
} from '@mui/material'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import { useVerge } from '@/hooks/use-verge'
import delayManager from '@/services/delay'
import { get3DInputStyle } from '@/utils/button-styles'
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
  onLocation,
  onCheckDelay,
}: Props) => {
  const theme = useTheme()
  const { width } = useWindowWidth()
  const isMinimal = width <= 285

  const {
    showType,
    sortType,
    filterText,
    textState,
    testUrl,
    filterMatchCase,
    filterMatchWholeWord,
    filterUseRegularExpression,
  } = headState

  const effectiveTextState = isMinimal ? 'filter' : textState

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
      {!isMinimal && (
        <IconButton
          size="small"
          color="inherit"
          title={t('proxies.page.tooltips.locate')}
          onClick={onLocation}
          sx={{ width: 26, height: 26, p: 0 }}
        >
          <MyLocationOutlined sx={{ fontSize: 17 }} />
        </IconButton>
      )}

      <IconButton
        size="small"
        color="inherit"
        title={t('proxies.page.tooltips.delayCheck')}
        onClick={() => {
          debugLog(`[ProxyHead] 点击延迟测试按钮，组: ${groupName}`)
          if (testUrl?.trim() && effectiveTextState !== 'filter') {
            debugLog(`[ProxyHead] 使用自定义测试URL: ${testUrl}`)
            onHeadState({ textState: 'url' })
          }
          onCheckDelay()
        }}
        sx={{ width: 26, height: 26, p: 0 }}
      >
        <BoltOutlined
          sx={{
            fontSize: 17,
            animation: isTesting
              ? `${pulseGlow} 1.2s infinite ease-in-out`
              : 'none',
            color: isTesting ? '#ffc107' : 'inherit',
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
        onClick={() =>
          onHeadState({ sortType: ((sortType + 1) % 3) as ProxySortType })
        }
        sx={{ width: 26, height: 26, p: 0 }}
      >
        {sortType !== 1 && sortType !== 2 && (
          <SortOutlined sx={{ fontSize: 17 }} />
        )}
        {sortType === 1 && <AccessTimeOutlined sx={{ fontSize: 17 }} />}
        {sortType === 2 && <SortByAlphaOutlined sx={{ fontSize: 17 }} />}
      </IconButton>

      {!isMinimal && (
        <IconButton
          size="small"
          color="inherit"
          title={t('proxies.page.tooltips.delayCheckUrl')}
          onClick={() =>
            onHeadState({ textState: textState === 'url' ? null : 'url' })
          }
          sx={{ width: 26, height: 26, p: 0 }}
        >
          <LinkOutlined
            sx={{ fontSize: 17, opacity: textState === 'url' ? 1 : 0.6 }}
          />
        </IconButton>
      )}

      {!isMinimal && (
        <IconButton
          size="small"
          color="inherit"
          title={
            showType
              ? t('proxies.page.tooltips.showBasic')
              : t('proxies.page.tooltips.showDetail')
          }
          onClick={() => onHeadState({ showType: !showType })}
          sx={{ width: 26, height: 26, p: 0 }}
        >
          {showType ? (
            <VisibilityOutlined sx={{ fontSize: 17 }} />
          ) : (
            <VisibilityOffOutlined sx={{ fontSize: 17 }} />
          )}
        </IconButton>
      )}

      {!isMinimal && (
        <IconButton
          size="small"
          color="inherit"
          title={t('proxies.page.tooltips.filter')}
          onClick={() =>
            onHeadState({ textState: textState === 'filter' ? null : 'filter' })
          }
          sx={{ width: 26, height: 26, p: 0 }}
        >
          <FilterListOutlined
            sx={{ fontSize: 17, opacity: textState === 'filter' ? 1 : 0.6 }}
          />
        </IconButton>
      )}

      {effectiveTextState === 'filter' && (
        <Box
          sx={{
            ml: 0.5,
            flex: '1 1 auto',
            height: 24,
            '& input': { py: 0.3, px: 0.5, fontSize: 11 },
          }}
        >
          <BaseSearchBox
            autoFocus={autoFocus}
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
      )}

      {effectiveTextState === 'url' && (
        <TextField
          autoComplete="off"
          autoFocus={autoFocus}
          hiddenLabel
          value={testUrl}
          size="small"
          variant="outlined"
          placeholder={t('proxies.page.placeholders.delayCheckUrl')}
          onChange={(e) => onHeadState({ testUrl: e.target.value })}
          sx={{
            ml: 0.5,
            flex: '1 1 auto',
            '& input': { py: 0.3, px: 0.5, fontSize: 11, height: 20 },
            ...get3DInputStyle(theme),
          }}
        />
      )}
    </Box>
  )
}
