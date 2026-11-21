import {
  FlashOnRounded,
  PublicRounded,
  LinkOffRounded,
  ContentCopyRounded,
  InfoOutlined,
  BlockRounded,
  RestoreRounded,
} from '@mui/icons-material'
import { Box, Menu, MenuItem, alpha, useTheme } from '@mui/material'
import {
  ColumnDef,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  Row,
  SortingState,
  useReactTable,
} from '@tanstack/react-table'
import { useVirtualizer } from '@tanstack/react-virtual'
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseEmpty } from '@/components/base'
import { closeConnectionWithTimeout } from '@/services/mihomo-api'
import { showNotice } from '@/services/notice-service'
import {
  get3DSegmentedActiveStyle,
  get3DSegmentedActiveTextColor,
  get3DSegmentedContainerStyle,
} from '@/utils/button-styles'
import {
  extractConnectionTarget,
  formatPathControlHostLabel,
  resolveConnectionControlStatus,
  type PathControlConnectionItem,
  type PathControlStatus,
  type PathControlTarget,
  type PathControlViewMode,
} from '@/utils/path-control'
import {
  getPopupMenuItemStyle,
  getPopupMenuPaperProps,
  getPopupSkinTokens,
} from '@/utils/popup-styles'
import {
  addQuickRoutingRule,
  extractRegistrableDomain,
  isIPv4Address,
  isIPv6Address,
  removeQuickRoutingRule,
  type QuickRoutingTarget,
} from '@/utils/quick-routing'

const ROW_HEIGHT = 20

const STATUS_COLOR_MAP: Record<PathControlStatus, string> = {
  direct: 'success.main',
  proxy: 'primary.main',
  reject: 'error.main',
  disconnected: 'warning.main',
}

const SX_OUTER: React.ComponentProps<typeof Box>['sx'] = {
  display: 'flex',
  flexDirection: 'column',
  flex: 1,
  height: '100%',
  minHeight: 0,
  position: 'relative',
  fontFamily: (theme) => theme.typography.fontFamily,
}

const SX_SCROLL_CONTAINER: React.ComponentProps<typeof Box>['sx'] = {
  flex: 1,
  height: '100%',
  minHeight: 0,
  overflowY: 'auto',
  overflowX: 'hidden',
  WebkitOverflowScrolling: 'touch',
  overscrollBehavior: 'contain',
  border: '5px double var(--theme-border)',
  borderRadius: '4px',
  scrollbarWidth: 'none',
  '&::-webkit-scrollbar': {
    display: 'none !important',
  },
}

const SX_HEADER_STICKY: React.ComponentProps<typeof Box>['sx'] = {
  position: 'sticky',
  top: 0,
  zIndex: 2,
}

const SX_CELL_CONTENT: React.ComponentProps<typeof Box>['sx'] = {
  flex: 1,
  display: 'flex',
  alignItems: 'center',
  gap: 0.5,
  px: 0,
  py: 0,
}

const SX_HEADER_ROW: React.ComponentProps<typeof Box>['sx'] = {
  display: 'flex',
  borderBottom: '5px double var(--theme-border)',
  backgroundColor: (theme) => theme.palette.background.paper,
  height: '24px',
}

const SX_HEADER_CELL_BASE: React.ComponentProps<typeof Box>['sx'] = {
  display: 'flex',
  alignItems: 'center',
  position: 'relative',
  boxSizing: 'border-box',
  fontSize: '12px',
  fontWeight: 600,
  color: 'text.secondary',
  userSelect: 'none',
  px: 1.5,
  py: '0.5px',
}

const SX_DATA_CELL_BASE: React.ComponentProps<typeof Box>['sx'] = {
  boxSizing: 'border-box',
  px: 1.5,
  py: '0.5px',
  fontSize: '12px',
  display: 'flex',
  alignItems: 'center',
  whiteSpace: 'nowrap',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
}

const SX_ROW_BASE: React.ComponentProps<typeof Box>['sx'] = {
  display: 'flex',
  position: 'absolute',
  left: 0,
  right: 0,
  cursor: 'pointer',
  borderBottom: (theme) => `2px solid ${theme.palette.divider}`,
}

interface RowComponentProps {
  row: Row<PathControlConnectionItem>
  virtualStart: number
  virtualSize: number
  onShowDetail: (data: PathControlConnectionItem, el?: HTMLElement) => void
  onContextMenu: (
    event: React.MouseEvent,
    row: PathControlConnectionItem,
  ) => void
}

const RowComponent = memo(
  function RowComponent({
    row,
    virtualStart,
    virtualSize,
    onShowDetail,
    onContextMenu,
  }: RowComponentProps) {
    const handleClick = useCallback(
      (e: React.MouseEvent<HTMLDivElement>) =>
        onShowDetail(row.original, e.currentTarget),
      [onShowDetail, row.original],
    )

    const handleContextMenu = useCallback(
      (e: React.MouseEvent) => {
        onContextMenu(e, row.original)
      },
      [onContextMenu, row.original],
    )

    return (
      <Box
        sx={[
          SX_ROW_BASE,
          {
            height: virtualSize,
            transform: `translateY(${virtualStart}px)`,
            backgroundColor:
              row.index % 2 === 0
                ? (theme) => theme.palette.background.paper
                : (theme) =>
                    theme.palette.mode === 'light' ? '#eef4ff' : '#232b3f',
            '&:hover': {
              backgroundColor: (theme) =>
                alpha(theme.palette.primary.main, 0.08),
            },
          },
        ]}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
      >
        {row.getVisibleCells().map((cell) => {
          const isChains = cell.column.id === 'chains'
          return (
            <Box
              key={cell.id}
              sx={[
                SX_DATA_CELL_BASE,
                {
                  flex: isChains ? '0 0 74px' : '1 1 0%',
                  minWidth: 0,
                  width: isChains ? '74px' : 'auto',
                  px: isChains ? 0.5 : 1.5,
                  justifyContent: isChains ? 'center' : 'flex-start',
                  borderRight: (theme) =>
                    !isChains ? `2px solid ${theme.palette.divider}` : 'none',
                },
              ]}
            >
              {flexRender(cell.column.columnDef.cell, cell.getContext())}
            </Box>
          )
        })}
      </Box>
    )
  },
  (prev, next) =>
    prev.row.original === next.row.original &&
    prev.row.index === next.row.index &&
    prev.virtualStart === next.virtualStart &&
    prev.virtualSize === next.virtualSize &&
    prev.onShowDetail === next.onShowDetail &&
    prev.onContextMenu === next.onContextMenu,
)

interface Props {
  connections: PathControlConnectionItem[]
  viewMode?: PathControlViewMode
  onViewModeChange?: (mode: PathControlViewMode) => void
  onShowDetail: (data: PathControlConnectionItem, el?: HTMLElement) => void
  onMarkDisconnected?: (row: PathControlConnectionItem) => void
  onClearDisconnectedTarget?: (targetKey?: string, connId?: string) => void
  onCloseMatchingTarget?: (target: PathControlTarget) => Promise<void>
  columnManagerOpen?: boolean
  onCloseColumnManager?: () => void
}

export const ConnectionTable = (props: Props) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const tokens = getPopupSkinTokens(theme)
  const menuItemSx = getPopupMenuItemStyle(theme, {
    extraSx: { gap: 1.5, py: 1, px: 2, fontSize: '12.5px' },
  })
  const {
    connections,
    viewMode,
    onViewModeChange,
    onShowDetail: rawOnShowDetail,
    onMarkDisconnected,
    onClearDisconnectedTarget,
    onCloseMatchingTarget,
  } = props
  const onShowDetailRef = useRef(rawOnShowDetail)
  onShowDetailRef.current = rawOnShowDetail
  const onShowDetail = useCallback(
    (data: PathControlConnectionItem, el?: HTMLElement) =>
      onShowDetailRef.current(data, el),
    [],
  )

  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
    row: PathControlConnectionItem
    anchorEl: HTMLElement
    scope: 'exact' | 'suffix'
  } | null>(null)

  const handleContextMenu = useCallback(
    (event: React.MouseEvent, row: PathControlConnectionItem) => {
      event.preventDefault()
      const isAlreadySuffixRule = Boolean(
        row._isSynthetic && row._managedRule?.ruleType === 'DOMAIN-SUFFIX',
      )
      setContextMenu({
        mouseX: event.clientX + 2,
        mouseY: event.clientY - 6,
        row,
        anchorEl: event.currentTarget as HTMLElement,
        scope: isAlreadySuffixRule ? 'suffix' : 'exact',
      })
    },
    [],
  )

  const handleCloseContextMenu = useCallback(() => {
    setContextMenu(null)
  }, [])

  const applyQuickRoute = useCallback(
    async (targetGroup: QuickRoutingTarget) => {
      if (!contextMenu) return
      const { row, scope } = contextMenu
      setContextMenu(null)

      const target =
        row._target ?? extractConnectionTarget(row, row._managedRule)
      if (!target) return

      const replaceRawRules =
        row._isSynthetic && row._managedRule
          ? [row._managedRule.raw]
          : undefined

      const isAlreadySuffixRule = Boolean(
        row._isSynthetic && row._managedRule?.ruleType === 'DOMAIN-SUFFIX',
      )

      if (scope === 'suffix') {
        const suffixDomain =
          isAlreadySuffixRule && row._managedRule
            ? row._managedRule.matchKey
            : extractRegistrableDomain(target.value)
        if (!suffixDomain) return

        const appliedRule = await addQuickRoutingRule(
          'domain-suffix',
          suffixDomain,
          targetGroup,
          replaceRawRules ? { replaceRawRules } : undefined,
        )
        if (appliedRule) {
          const suffixTarget: PathControlTarget = {
            type: 'domain',
            value: suffixDomain,
            key: `domain-suffix:${suffixDomain}`,
          }
          onClearDisconnectedTarget?.(suffixTarget.key, row.id)
          await onCloseMatchingTarget?.(suffixTarget)
        }
        return
      }

      const appliedRule = await addQuickRoutingRule(
        target.type,
        target.value,
        targetGroup,
        replaceRawRules ? { replaceRawRules } : undefined,
      )
      if (appliedRule) {
        onClearDisconnectedTarget?.(target.key, row.id)
        await onCloseMatchingTarget?.(target)
      }
    },
    [contextMenu, onClearDisconnectedTarget, onCloseMatchingTarget],
  )

  const handleDirect = useCallback(
    () => applyQuickRoute('DIRECT'),
    [applyQuickRoute],
  )

  const handleProxy = useCallback(
    () => applyQuickRoute('PROXY'),
    [applyQuickRoute],
  )

  const handleReject = useCallback(
    () => applyQuickRoute('REJECT'),
    [applyQuickRoute],
  )

  const handleDisconnect = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    setContextMenu(null)
    if (row._isSynthetic || row._isClosed) return

    onMarkDisconnected?.(row)
    try {
      await closeConnectionWithTimeout(row.id)
    } catch (err) {
      console.error(err)
    }
  }, [contextMenu, onMarkDisconnected])

  const handleRestoreDefault = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    setContextMenu(null)

    const target = row._target ?? extractConnectionTarget(row, row._managedRule)
    if (row._managedRule) {
      await removeQuickRoutingRule({ rawRule: row._managedRule.raw })
    }
    if (target || row.id) {
      onClearDisconnectedTarget?.(target?.key, row.id)
    }
  }, [contextMenu, onClearDisconnectedTarget])

  const handleCopy = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    setContextMenu(null)
    const metadata = row.metadata ?? {}
    const host =
      metadata.host ||
      metadata.remoteDestination ||
      metadata.destinationIP ||
      metadata.process ||
      ''
    const port = metadata.destinationPort
    const address = (port && host ? `${host}:${port}` : host) || ''
    if (!address) return
    try {
      await navigator.clipboard.writeText(address)
      showNotice.success('connections.notices.copied', { address }, 2000)
    } catch (err) {
      console.error('Failed to copy connection address:', err)
    }
  }, [contextMenu])

  const handleDetail = useCallback(() => {
    if (!contextMenu) return
    const { row, anchorEl } = contextMenu
    setContextMenu(null)
    onShowDetail(row, anchorEl)
  }, [contextMenu, onShowDetail])

  const getStatusLabel = useCallback(
    (status: PathControlStatus): string => {
      switch (status) {
        case 'direct':
          return t('connections.components.miniTable.chainsDirect')
        case 'reject':
          return t('connections.components.miniTable.chainsReject')
        case 'disconnected':
          return t('connections.components.miniTable.chainsDisconnected', {
            defaultValue: '已断开',
          })
        case 'proxy':
        default:
          return t('connections.components.miniTable.chainsProxy')
      }
    },
    [t],
  )

  const columnDefs = useMemo<ColumnDef<PathControlConnectionItem>[]>(() => {
    return [
      {
        id: 'host',
        accessorFn: (row) => {
          const base = formatPathControlHostLabel(row)
          const port = row.metadata?.destinationPort
          return port ? `${base}:${port}` : base
        },
        header: () => t('connections.components.miniTable.hostColumn'),
        cell: (ctx) => ctx.getValue<string>(),
      },
      {
        id: 'chains',
        accessorFn: (row) => {
          const status = resolveConnectionControlStatus(row)
          return getStatusLabel(status)
        },
        header: () => t('connections.components.miniTable.chainsColumn'),
        cell: (ctx) => {
          const status = resolveConnectionControlStatus(ctx.row.original)
          const label = getStatusLabel(status)
          return (
            <Box
              component="span"
              title={ctx.row.original._managedRule?.raw}
              sx={{ color: STATUS_COLOR_MAP[status], fontWeight: 600 }}
            >
              {label}
            </Box>
          )
        },
      },
    ]
  }, [t, getStatusLabel])

  const [sorting, setSorting] = useState<SortingState>([])

  const table = useReactTable({
    data: connections,
    state: {
      sorting,
    },
    onSortingChange: setSorting,
    columns: columnDefs,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: sorting.length ? getSortedRowModel() : undefined,
  })

  const rows = table.getRowModel().rows
  const tableContainerRef = useRef<HTMLDivElement | null>(null)
  useEffect(() => {
    if (viewMode === undefined) return
    tableContainerRef.current?.scrollTo({ top: 0 })
  }, [viewMode])
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 4,
  })

  const virtualRows = rowVirtualizer.getVirtualItems()
  const totalSize = rowVirtualizer.getTotalSize()

  const menuRow = contextMenu?.row
  const isAlreadySuffixRule = Boolean(
    menuRow?._isSynthetic &&
      menuRow?._managedRule?.ruleType === 'DOMAIN-SUFFIX',
  )
  const menuExactTarget = menuRow
    ? (menuRow._target ??
      extractConnectionTarget(menuRow, menuRow._managedRule))
    : null
  const menuRootDomain =
    !isAlreadySuffixRule &&
    menuExactTarget?.type === 'domain' &&
    !isIPv4Address(menuExactTarget.value) &&
    !isIPv6Address(menuExactTarget.value)
      ? extractRegistrableDomain(menuExactTarget.value)
      : null

  return (
    <>
      <Box sx={SX_OUTER}>
        <Box ref={tableContainerRef} sx={SX_SCROLL_CONTAINER}>
          <Box
            sx={{
              minWidth: '100%',
              width: '100%',
              ...(rows.length === 0
                ? {
                    display: 'flex',
                    flexDirection: 'column',
                    height: '100%',
                  }
                : null),
            }}
          >
            <Box sx={SX_HEADER_STICKY}>
              {table.getHeaderGroups().map((headerGroup) => (
                <Box key={headerGroup.id} sx={SX_HEADER_ROW}>
                  {headerGroup.headers.map((header) => {
                    const isChains = header.column.id === 'chains'
                    const showViewSwitch =
                      !isChains &&
                      viewMode !== undefined &&
                      onViewModeChange !== undefined
                    return (
                      <Box
                        key={header.id}
                        sx={[
                          SX_HEADER_CELL_BASE,
                          {
                            flex: isChains ? '0 0 74px' : '1 1 0%',
                            minWidth: 0,
                            width: isChains ? '74px' : 'auto',
                            px: isChains ? 0.5 : undefined,
                            pl: !isChains ? 1.5 : undefined,
                            pr: !isChains ? 0.75 : undefined,
                            justifyContent: !isChains
                              ? 'space-between'
                              : undefined,
                            gap: !isChains ? 0.75 : undefined,
                            borderRight: (tTheme) =>
                              !isChains
                                ? `2px solid ${tTheme.palette.divider}`
                                : 'none',
                          },
                        ]}
                      >
                        <Box
                          component="span"
                          onClick={
                            header.column.getCanSort()
                              ? header.column.getToggleSortingHandler()
                              : undefined
                          }
                          sx={[
                            SX_CELL_CONTENT,
                            {
                              minWidth: 0,
                              justifyContent: isChains
                                ? 'center'
                                : 'flex-start',
                              cursor: header.column.getCanSort()
                                ? 'pointer'
                                : 'default',
                              whiteSpace: 'nowrap',
                            },
                          ]}
                        >
                          <Box
                            component="span"
                            sx={{
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                            }}
                          >
                            {flexRender(
                              header.column.columnDef.header,
                              header.getContext(),
                            )}
                          </Box>
                          {{
                            asc: ' ▲',
                            desc: ' ▼',
                          }[header.column.getIsSorted() as string] ?? null}
                        </Box>
                        {showViewSwitch && (
                          <Box
                            role="tablist"
                            aria-label={t(
                              'connections.components.miniTable.hostColumn',
                            )}
                            onClick={(e) => e.stopPropagation()}
                            sx={(tTheme) => ({
                              position: 'relative',
                              display: 'inline-grid',
                              gridTemplateColumns: '1fr 1fr',
                              alignItems: 'center',
                              boxSizing: 'border-box',
                              height: 18,
                              p: '1px',
                              borderRadius: '4px',
                              bgcolor: 'action.hover',
                              flexShrink: 0,
                              userSelect: 'none',
                              ...get3DSegmentedContainerStyle(tTheme),
                            })}
                          >
                            <Box
                              sx={{
                                position: 'absolute',
                                left: 0,
                                top: 0,
                                width: '50%',
                                height: '100%',
                                zIndex: 0,
                                pointerEvents: 'none',
                                transition:
                                  'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                                transform: `translate3d(${viewMode === 'active' ? 0 : 100}%, 0, 0)`,
                              }}
                            >
                              <Box
                                sx={(tTheme) =>
                                  get3DSegmentedActiveStyle(tTheme)
                                }
                              />
                            </Box>
                            {(['active', 'closed'] as const).map((mode) => {
                              const isSelected = viewMode === mode
                              const label =
                                mode === 'active'
                                  ? t('connections.components.actions.active')
                                  : t('connections.components.actions.closed')
                              return (
                                <Box
                                  key={mode}
                                  role="tab"
                                  aria-selected={isSelected}
                                  onClick={(e) => {
                                    e.stopPropagation()
                                    if (viewMode !== mode) {
                                      onViewModeChange(mode)
                                    }
                                  }}
                                  sx={{
                                    height: '100%',
                                    display: 'flex',
                                    alignItems: 'center',
                                    justifyContent: 'center',
                                    px: 0.75,
                                    fontSize: '11px',
                                    fontWeight: 700,
                                    lineHeight: 1,
                                    whiteSpace: 'nowrap',
                                    cursor: 'pointer',
                                    zIndex: 1,
                                    transition: 'color 0.2s ease',
                                    color: isSelected
                                      ? get3DSegmentedActiveTextColor(theme)
                                      : 'text.secondary',
                                    '&:hover': {
                                      color: isSelected
                                        ? get3DSegmentedActiveTextColor(theme)
                                        : 'text.primary',
                                    },
                                  }}
                                >
                                  {label}
                                </Box>
                              )
                            })}
                          </Box>
                        )}
                      </Box>
                    )
                  })}
                </Box>
              ))}
            </Box>
            {rows.length === 0 ? (
              <Box sx={{ flex: 1, minHeight: 120, display: 'flex' }}>
                <BaseEmpty />
              </Box>
            ) : (
              <Box
                sx={{
                  position: 'relative',
                  height: totalSize,
                }}
              >
                {virtualRows.map((virtualRow) => {
                  const row = rows[virtualRow.index]
                  if (!row) return null

                  return (
                    <RowComponent
                      key={row.id}
                      row={row}
                      virtualStart={virtualRow.start}
                      virtualSize={virtualRow.size}
                      onShowDetail={onShowDetail}
                      onContextMenu={handleContextMenu}
                    />
                  )
                })}
              </Box>
            )}
          </Box>
        </Box>
      </Box>

      <Menu
        open={contextMenu !== null}
        onClose={handleCloseContextMenu}
        anchorReference="anchorPosition"
        anchorPosition={
          contextMenu !== null
            ? { top: contextMenu.mouseY, left: contextMenu.mouseX }
            : undefined
        }
        slotProps={{
          paper: getPopupMenuPaperProps(theme, {
            minWidth: 216,
            maxWidth: 300,
          }),
        }}
      >
        {contextMenu && menuRootDomain && (
          <Box
            onKeyDown={(e) => e.stopPropagation()}
            onClick={(e) => e.stopPropagation()}
            sx={{
              px: 1.25,
              pt: 0.5,
              pb: 0.75,
              mb: 0.25,
              borderBottom: `1px solid ${tokens.dividerColor}`,
            }}
          >
            <Box
              role="tablist"
              sx={(tTheme) => ({
                position: 'relative',
                display: 'grid',
                gridTemplateColumns: '1fr 1fr',
                alignItems: 'center',
                boxSizing: 'border-box',
                height: 22,
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
                  width: '50%',
                  height: '100%',
                  zIndex: 0,
                  pointerEvents: 'none',
                  transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                  transform: `translate3d(${contextMenu.scope === 'exact' ? 0 : 100}%, 0, 0)`,
                }}
              >
                <Box sx={(tTheme) => get3DSegmentedActiveStyle(tTheme)} />
              </Box>
              {(
                [
                  {
                    value: 'exact' as const,
                    label: t('connections.components.contextMenu.scopeExact'),
                    title: undefined,
                  },
                  {
                    value: 'suffix' as const,
                    label: `*.${menuRootDomain}`,
                    title: `*.${menuRootDomain}`,
                  },
                ] as const
              ).map((opt) => {
                const isSelected = contextMenu.scope === opt.value
                return (
                  <Box
                    key={opt.value}
                    role="tab"
                    aria-selected={isSelected}
                    title={opt.title}
                    onClick={() =>
                      setContextMenu((prev) =>
                        prev ? { ...prev, scope: opt.value } : null,
                      )
                    }
                    sx={{
                      height: '100%',
                      minWidth: 0,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      px: 0.75,
                      fontSize: '11px',
                      fontWeight: 700,
                      lineHeight: 1,
                      cursor: 'pointer',
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
                    <Box
                      component="span"
                      sx={{
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {opt.label}
                    </Box>
                  </Box>
                )
              })}
            </Box>
          </Box>
        )}
        <MenuItem onClick={handleDirect} sx={menuItemSx}>
          <FlashOnRounded sx={{ fontSize: 18, color: tokens.readyColor }} />
          {t('connections.components.contextMenu.setDirect')}
        </MenuItem>
        <MenuItem onClick={handleProxy} sx={menuItemSx}>
          <PublicRounded
            sx={{ fontSize: 18, color: tokens.accentPrimaryColor }}
          />
          {t('connections.components.contextMenu.setProxy')}
        </MenuItem>
        <MenuItem onClick={handleReject} sx={menuItemSx}>
          <BlockRounded sx={{ fontSize: 18, color: tokens.errorColor }} />
          {t('connections.components.contextMenu.setReject')}
        </MenuItem>
        <MenuItem
          onClick={handleDisconnect}
          disabled={Boolean(
            contextMenu?.row._isSynthetic || contextMenu?.row._isClosed,
          )}
          sx={menuItemSx}
        >
          <LinkOffRounded sx={{ fontSize: 18, color: tokens.errorColor }} />
          {t('connections.components.contextMenu.disconnect')}
        </MenuItem>
        {Boolean(
          contextMenu?.row._managedRule ||
            contextMenu?.row._controlStatus === 'disconnected',
        ) && (
          <MenuItem onClick={handleRestoreDefault} sx={menuItemSx}>
            <RestoreRounded sx={{ fontSize: 18, color: tokens.infoColor }} />
            {t('connections.components.contextMenu.restoreDefault', {
              defaultValue: '恢复默认路由',
            })}
          </MenuItem>
        )}
        <MenuItem onClick={handleCopy} sx={menuItemSx}>
          <ContentCopyRounded
            sx={{ fontSize: 18, color: tokens.secondaryText }}
          />
          {t('connections.components.contextMenu.copyAddress')}
        </MenuItem>
        <MenuItem onClick={handleDetail} sx={menuItemSx}>
          <InfoOutlined sx={{ fontSize: 18, color: tokens.secondaryText }} />
          {t('connections.components.contextMenu.viewDetail')}
        </MenuItem>
      </Menu>
    </>
  )
}
