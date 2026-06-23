import {
  FlashOnRounded,
  PublicRounded,
  LinkOffRounded,
  ContentCopyRounded,
  InfoOutlined,
  BlockRounded,
} from '@mui/icons-material'
import { Box, Menu, MenuItem, alpha } from '@mui/material'
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

import { showNotice } from '@/services/notice-service'
import type { IConnectionsItem } from '@/types/connection'
import { addQuickRoutingRule } from '@/utils/quick-routing'
import { closeConnection } from 'tauri-plugin-mihomo-api'

const ROW_HEIGHT = 20

const SX_OUTER: React.ComponentProps<typeof Box>['sx'] = {
  display: 'flex',
  flexDirection: 'column',
  flex: 1,
  minHeight: 0,
  position: 'relative',
  fontFamily: (theme) => theme.typography.fontFamily,
}

const SX_SCROLL_CONTAINER: React.ComponentProps<typeof Box>['sx'] = {
  flex: 1,
  minHeight: 0,
  overflowY: 'auto',
  overflowX: 'hidden',
  WebkitOverflowScrolling: 'touch',
  overscrollBehavior: 'contain',
  scrollbarWidth: 'none',
  msOverflowStyle: 'none !important',
  '&::-webkit-scrollbar': {
    display: 'none !important',
    width: '0 !important',
    height: '0 !important',
  },
  border: '5px double var(--theme-border)',
  borderRadius: '4px',
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
  row: Row<IConnectionsItem>
  virtualStart: number
  virtualSize: number
  onShowDetail: (data: IConnectionsItem, el?: HTMLElement) => void
  onContextMenu: (event: React.MouseEvent, row: IConnectionsItem) => void
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
                  flex: isChains ? '0 0 50px' : '1 1 0%',
                  minWidth: 0,
                  width: isChains ? '50px' : 'auto',
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
  connections: IConnectionsItem[]
  onShowDetail: (data: IConnectionsItem, el?: HTMLElement) => void
  columnManagerOpen?: boolean
  onCloseColumnManager?: () => void
}

export const ConnectionTable = (props: Props) => {
  const { connections, onShowDetail: rawOnShowDetail } = props
  const onShowDetailRef = useRef(rawOnShowDetail)
  onShowDetailRef.current = rawOnShowDetail
  const onShowDetail = useCallback(
    (data: IConnectionsItem, el?: HTMLElement) =>
      onShowDetailRef.current(data, el),
    [],
  )

  const [contextMenu, setContextMenu] = useState<{
    mouseX: number
    mouseY: number
    row: IConnectionsItem
    anchorEl: HTMLElement
  } | null>(null)

  const handleContextMenu = useCallback(
    (event: React.MouseEvent, row: IConnectionsItem) => {
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

  const handleDirect = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    const { metadata } = row
    setContextMenu(null)
    if (metadata.host) {
      await addQuickRoutingRule('domain', metadata.host, 'DIRECT')
    } else if (metadata.destinationIP) {
      await addQuickRoutingRule('domain', metadata.destinationIP, 'DIRECT')
    } else if (metadata.process) {
      await addQuickRoutingRule('process', metadata.process, 'DIRECT')
    }
  }, [contextMenu])

  const handleProxy = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    const { metadata } = row
    setContextMenu(null)
    if (metadata.host) {
      await addQuickRoutingRule('domain', metadata.host, 'PROXY')
    } else if (metadata.destinationIP) {
      await addQuickRoutingRule('domain', metadata.destinationIP, 'PROXY')
    } else if (metadata.process) {
      await addQuickRoutingRule('process', metadata.process, 'PROXY')
    }
  }, [contextMenu])

  const handleReject = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    const { metadata } = row
    setContextMenu(null)
    if (metadata.host) {
      await addQuickRoutingRule('domain', metadata.host, 'REJECT')
    } else if (metadata.destinationIP) {
      await addQuickRoutingRule('domain', metadata.destinationIP, 'REJECT')
    } else if (metadata.process) {
      await addQuickRoutingRule('process', metadata.process, 'REJECT')
    }
  }, [contextMenu])

  const handleDisconnect = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    setContextMenu(null)
    try {
      await closeConnection(row.id)
    } catch (err) {
      console.error(err)
    }
  }, [contextMenu])

  const handleCopy = useCallback(async () => {
    if (!contextMenu) return
    const { row } = contextMenu
    setContextMenu(null)
    const metadata = row.metadata ?? {}
    const host = metadata.host || metadata.remoteDestination
    const port = metadata.destinationPort
    const address = (port ? `${host}:${port}` : host) || ''
    try {
      await navigator.clipboard.writeText(address)
      showNotice.success('connections.copied', `已复制: ${address}`, 2000)
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

  const columnDefs = useMemo<ColumnDef<IConnectionsItem>[]>(() => {
    return [
      {
        id: 'host',
        accessorFn: (row) => {
          const m = row.metadata ?? {}
          return m.host
            ? `${m.host}:${m.destinationPort}`
            : `${m.remoteDestination}:${m.destinationPort}`
        },
        header: '链接目标',
        cell: (ctx) => {
          const row = ctx.row.original
          const m = row.metadata ?? {}
          return m.host
            ? `${m.host}:${m.destinationPort}`
            : `${m.remoteDestination}:${m.destinationPort}`
        },
      },
      {
        id: 'chains',
        header: '路由',
        cell: (ctx) => {
          const row = ctx.row.original
          const chains = row.chains || []
          if (chains.length === 0) return '直连'
          const upperChains = chains.map((c) => c.toUpperCase())
          if (upperChains.includes('REJECT')) return '封锁'
          if (upperChains.includes('DIRECT')) return '直连'
          return '代理'
        },
      },
    ]
  }, [])

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
                            flex: isChains ? '0 0 50px' : '1 1 0%',
                            minWidth: 0,
                            width: isChains ? '50px' : 'auto',
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
          paper: {
            sx: {
              background: (theme: any) => theme.palette.background.paper,
              border: '1px solid',
              borderColor: (theme: any) => theme.palette.divider,
              borderRadius: 1.5,
              boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.25)',
              minWidth: 200,
              py: 0.5,
            },
          },
        }}
      >
        <MenuItem
          onClick={handleDirect}
          sx={{ gap: 1.5, py: 1, px: 2, fontSize: '12.5px' }}
        >
          <FlashOnRounded sx={{ fontSize: 18, color: 'success.main' }} />
          设为全局直连
        </MenuItem>
        <MenuItem
          onClick={handleProxy}
          sx={{ gap: 1.5, py: 1, px: 2, fontSize: '12.5px' }}
        >
          <PublicRounded sx={{ fontSize: 18, color: 'primary.main' }} />
          设为代理分流
        </MenuItem>
        <MenuItem
          onClick={handleReject}
          sx={{ gap: 1.5, py: 1, px: 2, fontSize: '12.5px' }}
        >
          <BlockRounded sx={{ fontSize: 18, color: 'error.main' }} />
          封锁它 (REJECT)
        </MenuItem>
        <MenuItem
          onClick={handleDisconnect}
          sx={{ gap: 1.5, py: 1, px: 2, fontSize: '12.5px' }}
        >
          <LinkOffRounded sx={{ fontSize: 18, color: 'error.main' }} />
          断开此连接
        </MenuItem>
        <MenuItem
          onClick={handleCopy}
          sx={{ gap: 1.5, py: 1, px: 2, fontSize: '12.5px' }}
        >
          <ContentCopyRounded sx={{ fontSize: 18, color: 'text.secondary' }} />
          复制连接地址
        </MenuItem>
        <MenuItem
          onClick={handleDetail}
          sx={{ gap: 1.5, py: 1, px: 2, fontSize: '12.5px' }}
        >
          <InfoOutlined sx={{ fontSize: 18, color: 'text.secondary' }} />
          查看详细信息
        </MenuItem>
      </Menu>
    </>
  )
}
