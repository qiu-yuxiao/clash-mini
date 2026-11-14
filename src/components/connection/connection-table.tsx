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
import { memo, useCallback, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { closeConnectionWithTimeout } from '@/services/mihomo-api'
import { showNotice } from '@/services/notice-service'
import {
  extractConnectionTarget,
  formatPathControlHostLabel,
  resolveConnectionControlStatus,
  type PathControlConnectionItem,
  type PathControlStatus,
  type PathControlTarget,
} from '@/utils/path-control'
import {
  getPopupMenuItemStyle,
  getPopupMenuPaperProps,
  getPopupSkinTokens,
} from '@/utils/popup-styles'
import {
  addQuickRoutingRule,
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
  height: '20px',
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
  } | null>(null)

  const handleContextMenu = useCallback(
    (event: React.MouseEvent, row: PathControlConnectionItem) => {
      event.preventDefault()
      setContextMenu({
        mouseX: event.clientX + 2,
        mouseY: event.clientY - 6,
        row,
        anchorEl: event.currentTarget as HTMLElement,
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
      const { row } = contextMenu
      setContextMenu(null)

      const target =
        row._target ?? extractConnectionTarget(row, row._managedRule)
      if (!target) return

      const replaceRawRules =
        row._isSynthetic && row._managedRule
          ? [row._managedRule.raw]
          : undefined

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
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 4,
  })

  const virtualRows = rowVirtualizer.getVirtualItems()
  const totalSize = rowVirtualizer.getTotalSize()

  return (
    <>
      <Box sx={SX_OUTER}>
        <Box ref={tableContainerRef} sx={SX_SCROLL_CONTAINER}>
          <Box
            sx={{
              minWidth: '100%',
              width: '100%',
            }}
          >
            <Box sx={SX_HEADER_STICKY}>
              {table.getHeaderGroups().map((headerGroup) => (
                <Box key={headerGroup.id} sx={SX_HEADER_ROW}>
                  {headerGroup.headers.map((header) => {
                    const isChains = header.column.id === 'chains'
                    return (
                      <Box
                        key={header.id}
                        sx={[
                          SX_HEADER_CELL_BASE,
                          {
                            flex: isChains ? '0 0 74px' : '1 1 0%',
                            minWidth: 0,
                            width: isChains ? '74px' : 'auto',
                            px: isChains ? 0.5 : 1.5,
                            borderRight: (theme) =>
                              !isChains
                                ? `2px solid ${theme.palette.divider}`
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
                          {flexRender(
                            header.column.columnDef.header,
                            header.getContext(),
                          )}
                          {{
                            asc: ' ▲',
                            desc: ' ▼',
                          }[header.column.getIsSorted() as string] ?? null}
                        </Box>
                      </Box>
                    )
                  })}
                </Box>
              ))}
            </Box>
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
          paper: getPopupMenuPaperProps(theme, { minWidth: 200 }),
        }}
      >
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
