import {
  AccessTimeOutlined,
  BoltOutlined,
  SortByAlphaOutlined,
  SortOutlined,
} from '@mui/icons-material'
import type { SxProps, Theme } from '@mui/material'
import { Box, CircularProgress, IconButton, Tooltip } from '@mui/material'
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
  const [pendingAutoTest, setPendingAutoTest] = useState(false)
  const filterInputRef = useRef<HTMLInputElement | null>(null)
  const filterTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastTriggeredFilterRef = useRef<string>(filterText || '')

  const onCheckDelayRef = useRef(onCheckDelay)
  useEffect(() => {
    onCheckDelayRef.current = onCheckDelay
  }, [onCheckDelay])

  // BUG-355: 响应 Ctrl+F 全局快捷键聚焦并全选过滤框
  useEffect(() => {
    const onShortcut = (e: Event) => {
      const action = (e as CustomEvent<string>).detail
      if (action === 'focus-filter' && filterInputRef.current) {
        filterInputRef.current.focus()
        filterInputRef.current.select()
      }
    }
    window.addEventListener('clash-mini-shortcut', onShortcut)
    return () => {
      window.removeEventListener('clash-mini-shortcut', onShortcut)
    }
  }, [])

  useEffect(() => {
    return () => {
      if (filterTimerRef.current) {
        clearTimeout(filterTimerRef.current)
      }
    }
  }, [])

  const triggerAutoDelayTest = useCallback(() => {
    setPendingAutoTest(false)
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
        setPendingAutoTest(false)
        return
      }
      // BUG-360: 700ms 防抖等待期间展示显式脉冲倒计时提示，支持按 Esc 取消本次自动测速
      setPendingAutoTest(lastTriggeredFilterRef.current !== trimmed)
      filterTimerRef.current = setTimeout(() => {
        filterTimerRef.current = null
        setPendingAutoTest(false)
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
      setPendingAutoTest(false)
      const trimmed = currentText.trim()
      if (!trimmed) return
      lastTriggeredFilterRef.current = trimmed
      debugLog(`[ProxyHead] 回车立即触发测速择优: "${trimmed}"`)
      triggerAutoDelayTest()
    },
    [triggerAutoDelayTest],
  )

  const handleFilterKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape' && filterTimerRef.current) {
      e.stopPropagation()
      clearTimeout(filterTimerRef.current)
      filterTimerRef.current = null
      setPendingAutoTest(false)
    }
  }, [])

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
          inputRef={filterInputRef}
          autoFocus={autoFocus}
          value={filterText}
          onChangeText={handleFilterChangeText}
          onEnter={handleFilterEnter}
          onKeyDownExtra={handleFilterKeyDown}
          endAdornmentExtra={
            pendingAutoTest || isTesting ? (
              <Tooltip title={t('settings.mini.filterAutoTestingTooltip')}>
                <Box
                  sx={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    color: 'warning.main',
                    mr: 0.25,
                  }}
                >
                  <CircularProgress size={11} color="inherit" />
                </Box>
              </Tooltip>
            ) : null
          }
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
