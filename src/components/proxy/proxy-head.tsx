import {
  AccessTimeOutlined,
  BoltOutlined,
  SortByAlphaOutlined,
  SortOutlined,
} from '@mui/icons-material'
import type { SxProps, Theme } from '@mui/material'
import { Box, IconButton } from '@mui/material'
import { keyframes } from '@mui/material'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import { debugLog } from '@/utils/debug'

import { ProxySortType } from './use-filter-sort'
import { HeadState } from './use-head-state'

interface Props {
  sx?: SxProps<Theme>
  groupName: string
  headState: HeadState
  isTesting?: boolean
  onHeadState: (state: Partial<HeadState>) => void
  onCheckDelay: () => void
  onLocation?: () => void
}

const pulseGlow = keyframes`
  0% {
    transform: scale(1);
    filter: drop-shadow(0 0 1px rgba(255, 193, 7, 0.4));
    opacity: 0.8;
  }
  50% {
    transform: scale(1.12);
    filter: drop-shadow(0 0 8px rgba(255, 193, 7, 0.95));
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
  groupName,
  headState,
  isTesting = false,
  onHeadState,
  onCheckDelay,
}: Props) => {
  const { sortType, filterText } = headState

  const { t } = useTranslation()
  const [autoFocus, setAutoFocus] = useState(false)
  const filterTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastTriggeredFilterRef = useRef<string>(filterText || '')

  const onCheckDelayRef = useRef(onCheckDelay)
  useEffect(() => {
    onCheckDelayRef.current = onCheckDelay
  }, [onCheckDelay])

  useEffect(() => {
    return () => {
      if (filterTimerRef.current) {
        clearTimeout(filterTimerRef.current)
      }
    }
  }, [])

  const triggerAutoDelayTest = useCallback(() => {
    // 延迟 60ms 确保 React 状态更新并完成过滤计算
    setTimeout(() => {
      onCheckDelayRef.current()
    }, 60)
  }, [])

  const handleFilterChangeText = useCallback(
    (newText: string) => {
      if (filterTimerRef.current) {
        clearTimeout(filterTimerRef.current)
        filterTimerRef.current = null
      }
      const trimmed = newText.trim()
      // 清空过滤时不自动触发全量测速择优
      if (!trimmed) {
        lastTriggeredFilterRef.current = ''
        return
      }
      // 防抖 700ms 自动触发一次多路测速择优
      filterTimerRef.current = setTimeout(() => {
        if (lastTriggeredFilterRef.current !== trimmed) {
          lastTriggeredFilterRef.current = trimmed
          debugLog(`[ProxyHead] 过滤输入完成，自动触发测速择优: "${trimmed}"`)
          triggerAutoDelayTest()
        }
      }, 700)
    },
    [triggerAutoDelayTest],
  )

  const handleFilterEnter = useCallback(
    (currentText: string) => {
      if (filterTimerRef.current) {
        clearTimeout(filterTimerRef.current)
        filterTimerRef.current = null
      }
      const trimmed = currentText.trim()
      if (!trimmed) return
      lastTriggeredFilterRef.current = trimmed
      debugLog(`[ProxyHead] 回车立即触发测速择优: "${trimmed}"`)
      triggerAutoDelayTest()
    },
    [triggerAutoDelayTest],
  )

  useEffect(() => {
    // fix the focus conflict
    const timer = setTimeout(() => setAutoFocus(true), 100)
    return () => clearTimeout(timer)
  }, [])

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
            color: (theme) =>
              isTesting
                ? theme.palette.mode === 'dark'
                  ? 'warning.main'
                  : 'warning.dark'
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
        {sortType === 1 && (
          <AccessTimeOutlined aria-hidden="true" sx={{ fontSize: 17 }} />
        )}
        {sortType === 2 && (
          <SortByAlphaOutlined aria-hidden="true" sx={{ fontSize: 17 }} />
        )}
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
          value={filterText}
          onChangeText={handleFilterChangeText}
          onEnter={handleFilterEnter}
          onSearch={(_, state) =>
            onHeadState({
              filterText: state.text,
            })
          }
        />
      </Box>
    </Box>
  )
}
