import {
  PlayCircleOutlineRounded,
  PauseCircleOutlineRounded,
  SwapVertRounded,
} from '@mui/icons-material'
import { Box, Button, IconButton, MenuItem } from '@mui/material'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  BaseEmpty,
  BasePage,
  BaseSearchBox,
  BaseStyledSelect,
  type SearchState,
  VirtualList,
  type VirtualListHandle,
} from '@/components/base'
import LogItem from '@/components/log/log-item'
import { useClashLog } from '@/hooks/use-clash-log'
import { useLogData } from '@/hooks/use-log-data'
import type { LogFilter } from '@/types/clash'
import { get3DButtonStyle } from '@/utils/button-styles'

const LogPage = () => {
  const { t } = useTranslation()
  const [clashLog, setClashLog] = useClashLog()
  const enableLog = clashLog?.enable ?? true
  const logState = clashLog?.logFilter ?? 'all'
  const logOrder = clashLog?.logOrder ?? 'asc'
  const isDescending = logOrder === 'desc'

  const [match, setMatch] = useState(() => (_: string) => true)
  const [searchState, setSearchState] = useState<SearchState>()
  const {
    response: { data: logData },
    refreshGetClashLog,
  } = useLogData()

  const filterLogs = useMemo(() => {
    if (!logData || logData.length === 0) {
      return []
    }

    // Server-side filtering handles level filtering via query parameters
    // We only need to apply search filtering here
    return logData.filter((data) => {
      // 构建完整的搜索文本，包含时间、类型和内容
      const searchText =
        `${data.time || ''} ${data.type} ${data.payload}`.toLowerCase()

      const matchesSearch = match(searchText)

      return (
        (logState == 'all' ? true : (data.type || '').includes(logState)) &&
        matchesSearch
      )
    })
  }, [logData, logState, match])

  const filteredLogs = useMemo(
    () => (isDescending ? [...filterLogs].reverse() : filterLogs),
    [filterLogs, isDescending],
  )

  const virtuosoRef = useRef<VirtualListHandle>(null)
  // BUG-338: 仅当滚动条停靠底部时才跟随新日志自动滚动（且使用 auto 即时定位替代 smooth 动画，消除 50ms 刷新抖动与回弹）
  const isPinnedToBottomRef = useRef(true)

  const handleListScroll = useCallback((e: Event) => {
    const el = e.target as HTMLElement | null
    if (!el) return
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight
    isPinnedToBottomRef.current = distanceFromBottom <= 48
  }, [])

  useEffect(() => {
    if (
      !isDescending &&
      filteredLogs.length > 0 &&
      isPinnedToBottomRef.current
    ) {
      virtuosoRef.current?.scrollToIndex(filteredLogs.length - 1, {
        align: 'end',
        behavior: 'auto',
      })
    }
  }, [filteredLogs.length, isDescending])

  const handleLogLevelChange = (newLevel: LogFilter) => {
    setClashLog((pre) => ({
      enable: pre?.enable ?? false,
      logLevel: pre?.logLevel ?? 'info',
      logFilter: newLevel,
      logOrder: pre?.logOrder ?? 'asc',
    }))
  }

  const handleToggleLog = async () => {
    setClashLog((pre) => ({
      enable: !enableLog,
      logLevel: pre?.logLevel ?? 'info',
      logFilter: pre?.logFilter ?? 'all',
      logOrder: pre?.logOrder ?? 'asc',
    }))
  }

  const handleToggleOrder = () => {
    setClashLog((pre) => ({
      enable: pre?.enable ?? false,
      logLevel: pre?.logLevel ?? 'info',
      logFilter: pre?.logFilter ?? 'all',
      logOrder: pre?.logOrder === 'desc' ? 'asc' : 'desc',
    }))
  }

  return (
    <BasePage
      full
      title={t('logs.page.title')}
      contentStyle={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'auto',
      }}
      header={
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
          <IconButton
            title={t(
              enableLog ? 'shared.actions.pause' : 'shared.actions.resume',
            )}
            aria-label={t(
              enableLog ? 'shared.actions.pause' : 'shared.actions.resume',
            )}
            size="small"
            color="inherit"
            onClick={handleToggleLog}
            sx={{ p: 0.5 }}
          >
            {enableLog ? (
              <PauseCircleOutlineRounded sx={{ fontSize: 18 }} />
            ) : (
              <PlayCircleOutlineRounded sx={{ fontSize: 18 }} />
            )}
          </IconButton>
          <IconButton
            title={t(
              isDescending
                ? 'logs.actions.showAscending'
                : 'logs.actions.showDescending',
            )}
            aria-label={t(
              isDescending
                ? 'logs.actions.showAscending'
                : 'logs.actions.showDescending',
            )}
            size="small"
            color="inherit"
            onClick={handleToggleOrder}
            sx={{ p: 0.5 }}
          >
            <SwapVertRounded
              sx={{
                fontSize: 18,
                transform: isDescending ? 'scaleY(-1)' : 'none',
                transition: 'transform 0.2s ease',
              }}
            />
          </IconButton>

          <Button
            size="small"
            onClick={() => {
              refreshGetClashLog(true)
            }}
            sx={(theme) => ({
              ...get3DButtonStyle(theme, 'contained', 'primary'),
              minWidth: 'auto',
              px: 1,
              py: 0.2,
              fontSize: '11px',
              height: '24px',
            })}
          >
            {t('shared.actions.clear')}
          </Button>
        </Box>
      }
    >
      <Box
        sx={{
          pt: 1,
          mb: 0.5,
          mx: '10px',
          height: '39px',
          display: 'flex',
          alignItems: 'center',
        }}
      >
        <BaseStyledSelect
          aria-label={t('shared.filters.logLevels.label')}
          value={logState}
          sx={{ width: 95, minWidth: 90 }}
          onChange={(e) => handleLogLevelChange(e.target.value as LogFilter)}
        >
          <MenuItem value="all">{t('shared.filters.logLevels.all')}</MenuItem>
          <MenuItem value="debug">
            {t('shared.filters.logLevels.debug')}
          </MenuItem>
          <MenuItem value="info">{t('shared.filters.logLevels.info')}</MenuItem>
          <MenuItem value="warn">{t('shared.filters.logLevels.warn')}</MenuItem>
          <MenuItem value="err">{t('shared.filters.logLevels.error')}</MenuItem>
        </BaseStyledSelect>
        <BaseSearchBox
          onSearch={(matcher, state) => {
            setMatch(() => matcher)
            setSearchState(state)
          }}
        />
      </Box>

      {filteredLogs.length > 0 ? (
        <VirtualList
          ref={virtuosoRef}
          count={filteredLogs.length}
          estimateSize={50}
          onScroll={handleListScroll}
          renderItem={(i) => (
            <LogItem value={filteredLogs[i]} searchState={searchState} />
          )}
          style={{ flex: 1 }}
        />
      ) : (
        <BaseEmpty />
      )}
    </BasePage>
  )
}

export default LogPage
