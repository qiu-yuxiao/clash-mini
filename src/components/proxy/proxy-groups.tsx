import {
  Box,
  Menu,
  MenuItem,
  Typography,
} from '@mui/material'
import { defaultRangeExtractor, useVirtualizer } from '@tanstack/react-virtual'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  type Key,
  type RefObject,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  use,
} from 'react'
import { useTranslation } from 'react-i18next'
import { useLocation } from 'react-router'

import { useProxySelection } from '@/hooks/use-proxy-selection'
import { useVerge } from '@/hooks/use-verge'
import { useProxiesData } from '@/providers/app-data-context'
import { DragRegionContext } from '@/providers/drag-region-context'
import { batchTestLockRef } from '@/services/batch-test-lock'
import {
  getProfiles,
  triggerAutoSelect,
} from '@/services/cmds'
import delayManager from '@/services/delay'
import type { IProxyItem, IProxyGroupItem } from '@/types/clash'
import { debugLog } from '@/utils/debug'
import { isDummyNode } from '@/utils/node'

import { ScrollTopButton } from '../layout/scroll-top-button'

import { ProxyHead } from './proxy-head'
import { ProxyRender } from './proxy-render'
import { DEFAULT_STATE } from './use-head-state'
import type { HeadState } from './use-head-state'
import { type IRenderItem, useRenderList } from './use-render-list'

function useStableCallback<T extends (...args: any[]) => any>(fn: T): T {
  const ref = useRef(fn)
  ref.current = fn
  return useCallback((...args: Parameters<T>) => ref.current(...args), []) as T
}

interface Props {
  mode: string
}

// Performance optimizations for large proxy lists (1000+ nodes):
// 1. Virtual scrolling via @tanstack/react-virtual - only renders visible items
// 2. React.memo on ProxyRender component - prevents unnecessary re-renders
// 3. useMemo for derived data (filtered lists, computed values)
// 4. useCallback for event handlers - stable references for memoized children
// 5. Reference stability optimization in connection data processing
export const ProxyGroups = (props: Props) => {
  const { t } = useTranslation()
  const { pathname } = useLocation()
  const { mode } = props
  // 消费拖拽区域状态，批量测速期间禁用 drag-region
  const { setEnabled: setDragRegionEnabled } = use(DragRegionContext)

  const [testingGroups, setTestingGroups] = useState<Record<string, boolean>>(
    {},
  )
  // H-16: 用 ref 同步镜像 testingGroups，解决闭包竞态——
  // setTestingGroups 是异步批处理的，闭包中的值在整个 async 生命周期内都是调用时的快照
  const testingGroupsRef = useRef<Record<string, boolean>>({})
  // 包装 setTestingGroups 使其同步更新 ref
  const updateTestingGroups = useCallback(
    (updater: (prev: Record<string, boolean>) => Record<string, boolean>) => {
      setTestingGroups((prev) => {
        const next = updater(prev)
        testingGroupsRef.current = next
        return next
      })
    },
    [],
  )


  const [duplicateWarning, setDuplicateWarning] = useState<{
    open: boolean
    message: string
  }>({ open: false, message: '' })

  // F4 手动测速的探针超时固定为死节点阈值 2000ms（与后端 auto-select 一致），
  // 不再读取 verge.default_latency_timeout；useVerge 仅用于订阅配置变更。
  useVerge()
  const { proxies: proxiesData } = useProxiesData()


  const { renderList, onHeadState } = useRenderList(mode)

  const filteredRenderList = useMemo(() => {
    return renderList.filter((item) => item.type !== 1)
  }, [renderList])

  const getGroupHeadState = useCallback(
    (groupName: string) => {
      const headItem = renderList.find(
        (item) => item.type === 1 && item.group?.name === groupName,
      )
      return headItem?.headState
    },
    [renderList],
  )

  // 统代理选择
  const { handleProxyGroupChange } = useProxySelection({
    onError: (error) => {
      console.error('代理切换失败', error)
    },
  })

  const parentRef = useRef<HTMLDivElement>(null)
  const scrollPositionRef = useRef<Record<string, number>>({})
  const scrollTopRef = useRef(0)
  const showScrollTopRef = useRef(false)
  const activeStickyIndexRef = useRef<number | null>(null)
  const restoredScrollKeyRef = useRef<string | null>(null)
  const isMountedRef = useRef(true)
  const [showScrollTop, setShowScrollTop] = useState(false)
  const scrollPositionKey = useMemo(() => `${mode}:normal`, [mode])
  const stickyGroupIndexes = useMemo(
    () =>
      filteredRenderList.flatMap((item, index) =>
        item.type === 0 && !item.group?.hidden ? [index] : [],
      ),
    [filteredRenderList],
  )

  const rangeExtractor = useCallback(
    (range: Parameters<typeof defaultRangeExtractor>[0]) => {
      const activeStickyIndex = [...stickyGroupIndexes]
        .reverse()
        .find((index) => index <= range.startIndex)
      activeStickyIndexRef.current = activeStickyIndex ?? null

      const indexes = defaultRangeExtractor(range)
      return activeStickyIndex == null || indexes.includes(activeStickyIndex)
        ? indexes
        : [activeStickyIndex, ...indexes]
    },
    [stickyGroupIndexes],
  )

  const virtualizer = useVirtualizer({
    count: filteredRenderList.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => {
      const item = filteredRenderList[index]
      if (item?.type === 0) return 56
      if (item?.type === 2) return 24
      if (item?.type === 3) return 80
      if (item?.type === 4) return 24
      return 56
    },
    overscan: 15,
    getItemKey: (index) => filteredRenderList[index]?.key ?? index,
    rangeExtractor,
  })
  const virtualItems = virtualizer.getVirtualItems()
  const activeStickyIndex = activeStickyIndexRef.current

  // 从 localStorage 恢复滚动位置
  useLayoutEffect(() => {
    if (filteredRenderList.length === 0) return
    const node = parentRef.current
    if (!node) return
    if (
      restoredScrollKeyRef.current === scrollPositionKey &&
      node.scrollTop === scrollTopRef.current
    ) {
      return
    }

    try {
      const savedPositions = localStorage.getItem('proxy-scroll-positions')
      if (savedPositions) {
        const positions = JSON.parse(savedPositions)
        scrollPositionRef.current = positions
        const savedPosition = positions[scrollPositionKey]

        if (savedPosition !== undefined) {
          node.scrollTop = savedPosition
          scrollTopRef.current = savedPosition
          const nextShowScrollTop = savedPosition > 100
          showScrollTopRef.current = nextShowScrollTop
          queueMicrotask(() => {
            if (isMountedRef.current) {
              setShowScrollTop(nextShowScrollTop)
            }
          })
        }
      }
    } catch (e) {
      console.error('Error restoring scroll position:', e)
    }
    restoredScrollKeyRef.current = scrollPositionKey
  }, [pathname, filteredRenderList.length, scrollPositionKey])

  // 改为使用节流函数保存滚动位置
  const saveScrollPosition = useCallback(
    (scrollTop: number) => {
      try {
        scrollPositionRef.current[scrollPositionKey] = scrollTop
        localStorage.setItem(
          'proxy-scroll-positions',
          JSON.stringify(scrollPositionRef.current),
        )
      } catch (e) {
        console.error('Error saving scroll position:', e)
      }
    },
    [scrollPositionKey],
  )

  const saveScrollPositionThrottled = useMemo(
    () => throttle(saveScrollPosition, 500),
    [saveScrollPosition],
  )

  const handleScroll = useCallback(
    (event: Event) => {
      const target = event.target as HTMLElement | null
      const nextScrollTop = target?.scrollTop ?? 0
      const nextShowScrollTop = nextScrollTop > 100
      scrollTopRef.current = nextScrollTop

      if (showScrollTopRef.current !== nextShowScrollTop) {
        showScrollTopRef.current = nextShowScrollTop
        setShowScrollTop(nextShowScrollTop)
      }

      saveScrollPositionThrottled(nextScrollTop)
    },
    [saveScrollPositionThrottled],
  )

  // 添加和清理滚动事件监听器
  useEffect(() => {
    isMountedRef.current = true
    const node = parentRef.current
    if (!node) return

    const listener = handleScroll as EventListener
    const options: AddEventListenerOptions = { passive: true }

    node.addEventListener('scroll', listener, options)

    return () => {
      isMountedRef.current = false
      if (restoredScrollKeyRef.current === scrollPositionKey) {
        saveScrollPosition(scrollTopRef.current)
      }
      node.removeEventListener('scroll', listener, options)
      saveScrollPositionThrottled.cancel()
    }
  }, [
    handleScroll,
    saveScrollPosition,
    saveScrollPositionThrottled,
    scrollPositionKey,
  ])

  // 滚动到顶部
  const scrollToTop = useCallback(() => {
    parentRef.current?.scrollTo?.({
      top: 0,
      behavior: 'smooth',
    })
    scrollTopRef.current = 0
    saveScrollPosition(0)
  }, [saveScrollPosition])

  // 关闭重复节点警告
  const handleCloseDuplicateWarning = useCallback(() => {
    setDuplicateWarning({ open: false, message: '' })
  }, [])



  const handleChangeProxy = useCallback(
    (group: IProxyGroupItem, proxy: IProxyItem) => {
      if (!['Selector', 'URLTest', 'Fallback'].includes(group.type)) return

      handleProxyGroupChange(group, proxy)
    },
    [handleProxyGroupChange],
  )

  // 批量测速当前组全部节点并择优切换（委托后端单一引擎）
  const handleCheckAll = useStableCallback(async (groupName: string) => {
    // 防重复触发：测速进行中忽略点击
    if (testingGroupsRef.current[groupName]) return

    debugLog(`[ProxyGroups] 开始批量测速，组: ${groupName}`)
    updateTestingGroups((prev) => ({ ...prev, [groupName]: true }))

    try {
      // 从当前过滤后可见的渲染列表中提取节点名称；该子集同时作为 F4 批量测速+择优的候选池（防越界关键），并非仅用于视觉占位
      const visibleNames = filteredRenderList
        .filter(
          (e) => e.group?.name === groupName && (e.type === 2 || e.type === 4),
        )
        .flatMap((e) =>
          e.type === 4
            ? (e.proxyCol ?? []).map((p) => p?.name)
            : [e.proxy?.name],
        )
        .filter((name): name is string => !!name && !isDummyNode(name))

      debugLog(`[ProxyGroups] 可见节点数量: ${visibleNames.length}`)

      const currentUid = (await getProfiles())?.current || ''
      if (!currentUid) return

      // 视觉占位：立即将待测节点标记为「测速中」以触发流光动画
      for (const name of visibleNames) {
        delayManager.setDelay(name, groupName, -2)
      }
      delayManager.queueGroupNotification(groupName)

      // 委托后端统一执行群发测速 + 择优（select=true）
      // 关键：传入当前可见节点子集 visibleNames，后端只在「该子集」内测速并挑最快，
      // 而非 PROXY 全量——所见即所测所选，与自动选点（全量）行为区分开
      const win = getCurrentWindow()
      try {
        batchTestLockRef.current++ // H-11: 进入测速锁
        await win.setResizable(false)
        setDragRegionEnabled(false)
        await triggerAutoSelect(currentUid, visibleNames, 0, true)
      } catch (err) {
        console.error('[ProxyGroups] 后端批量测速/选点失败:', err)
      } finally {
        // H-11: 引用计数解锁，仅当===0时恢复窗口
        batchTestLockRef.current = Math.max(0, batchTestLockRef.current - 1)
        if (batchTestLockRef.current === 0) {
          await win.setResizable(true)
          setDragRegionEnabled(true)
        }
      }
    } catch (error) {
      console.error(`[ProxyGroups] 批量测速出错，组: ${groupName}`, error)
    } finally {
      if (isMountedRef.current) {
        updateTestingGroups((prev) => ({ ...prev, [groupName]: false }))
        const headState = getGroupHeadState(groupName)
        if (headState?.sortType === 1) {
          onHeadState(groupName, { sortType: headState.sortType })
        }
      }
    }
  })

  // 滚到对应的节点
  const handleLocation = useStableCallback((group: IProxyGroupItem) => {
    if (!group) return
    const { name, now } = group

    const index = filteredRenderList.findIndex(
      (e) =>
        e.group?.name === name &&
        ((e.type === 2 && e.proxy?.name === now) ||
          (e.type === 4 && e.proxyCol?.some((p) => p.name === now))),
    )

    if (index >= 0) {
      virtualizer.scrollToIndex(index, { align: 'center', behavior: 'smooth' })
    }
  })

  const activeGroupHeadItem = useMemo(() => {
    return renderList.find((item) => item.type === 1)
  }, [renderList])

  const renderProxyList = (height: string) => (
    <ProxyVirtualList
      parentRef={parentRef}
      height={height}
      totalSize={virtualizer.getTotalSize()}
      virtualItems={virtualItems}
      renderList={filteredRenderList}
      activeStickyIndex={activeStickyIndex}
      indent={mode === 'rule' || mode === 'script'}
      measureElement={virtualizer.measureElement}
      onLocation={handleLocation}
      onCheckAll={handleCheckAll}
      onHeadState={onHeadState}
      onChangeProxy={handleChangeProxy}
      headItem={activeGroupHeadItem}
      testingGroups={testingGroups}
    />
  )

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        position: 'relative',
      }}
    >
      <Box sx={{ flex: 1, minHeight: 0, position: 'relative', mx: 1, mb: 1 }}>
        {renderProxyList('100%')}
      </Box>
      <ScrollTopButton show={showScrollTop} onClick={scrollToTop} />
    </Box>
  )
}

type VirtualListItem = {
  key: Key
  index: number
  start: number
  end: number
}

interface ProxyVirtualListProps {
  parentRef: RefObject<HTMLDivElement | null>
  height: string
  totalSize: number
  virtualItems: VirtualListItem[]
  renderList: IRenderItem[]
  activeStickyIndex: number | null
  indent: boolean
  measureElement: (node: Element | null) => void
  onLocation: (group: IRenderItem['group']) => void
  onCheckAll: (groupName: string) => void
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
  onChangeProxy: (
    group: IRenderItem['group'],
    proxy: IRenderItem['proxy'] & { name: string },
  ) => void
  headItem?: IRenderItem | null
  testingGroups: Record<string, boolean>
}



function ProxyVirtualList({
  parentRef,
  height,
  totalSize,
  virtualItems,
  renderList,
  activeStickyIndex,
  indent,
  measureElement,
  onLocation,
  onCheckAll,
  onHeadState,
  onChangeProxy,
  headItem,
  testingGroups,
}: ProxyVirtualListProps) {
  const { t } = useTranslation()
  const stickyBackground = 'var(--theme-bg, var(--background-color))'

  return (
    <Box
      className="theme-panel"
      sx={{
        height,
        display: 'flex',
        flexDirection: 'column',
        boxSizing: 'border-box',
        overflow: 'hidden',
      }}
    >
      {headItem && (
        <ProxyHead
          sx={{
            px: 2,
            py: 0.5,
            borderBottom: (theme) => `1px solid ${theme.palette.divider}`,
          }}
          url={headItem.group?.testUrl}
          groupName={headItem.group?.name ?? ''}
          headState={headItem.headState ?? DEFAULT_STATE}
          isTesting={testingGroups[headItem.group?.name ?? '']}
          onLocation={() => onLocation(headItem.group)}
          onCheckDelay={() => onCheckAll(headItem.group?.name ?? '')}
          onHeadState={(p) => onHeadState(headItem.group?.name ?? '', p)}
        />
      )}
      <Box
        ref={parentRef}
        sx={{
          flex: 1,
          minHeight: 0,
          overflow: 'auto',
          scrollbarWidth: 'none !important' as 'none',
          msOverflowStyle: 'none !important',
          '&::-webkit-scrollbar': {
            display: 'none !important',
            width: '0 !important',
            height: '0 !important',
          },
        }}
      >
        <div
          role="list"
          // @ts-expect-error MUI v9 aria-label type issue
          aria-label={t('proxies.a11y.nodelist')}
          style={{ height: totalSize, position: 'relative' }}
        >
          {virtualItems.map((virtualItem) => (
            <div
              key={virtualItem.key}
              role="listitem"
              aria-posinset={virtualItem.index + 1}
              aria-setsize={renderList.length}
              data-index={virtualItem.index}
              ref={measureElement}
              style={{
                position:
                  virtualItem.index === activeStickyIndex
                    ? 'sticky'
                    : 'absolute',
                top: 0,
                left: 0,
                zIndex: virtualItem.index === activeStickyIndex ? 5 : undefined,
                display:
                  virtualItem.index === activeStickyIndex
                    ? 'flow-root'
                    : undefined,
                backgroundColor:
                  virtualItem.index === activeStickyIndex
                    ? stickyBackground
                    : undefined,
                width: '100%',
                transform:
                  virtualItem.index === activeStickyIndex
                    ? undefined
                    : `translateY(${virtualItem.start}px)`,
              }}
            >
              <ProxyRender
                item={renderList[virtualItem.index]}
                indent={indent}
                onLocation={onLocation}
                onCheckAll={onCheckAll}
                onHeadState={onHeadState}
                onChangeProxy={onChangeProxy}
                isTesting={
                  testingGroups[renderList[virtualItem.index]?.group?.name]
                }
              />
            </div>
          ))}
          <div style={{ height: 8 }} />
        </div>
      </Box>
    </Box>
  )
}

// 替换简单防抖函数为更优的节流函数
function throttle<T extends (...args: any[]) => any>(
  func: T,
  wait: number,
): ((...args: Parameters<T>) => void) & { cancel: () => void } {
  let timer: ReturnType<typeof setTimeout> | null = null
  let previous = 0
  let lastArgs: Parameters<T> | null = null

  const run = (args: Parameters<T>) => {
    previous = Date.now()
    timer = null
    lastArgs = null
    func(...args)
  }

  const throttled = function (...args: Parameters<T>) {
    const now = Date.now()
    const remaining = wait - (now - previous)
    lastArgs = args

    if (remaining <= 0) {
      // 修复 BUG-MINOR-001：删除冗余条件 remaining > wait（永远不会成立）
      if (timer) {
        clearTimeout(timer)
      }
      run(args)
    } else if (!timer) {
      timer = setTimeout(() => {
        if (lastArgs) {
          run(lastArgs)
        }
      }, remaining)
    }
  }

  throttled.cancel = () => {
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
    lastArgs = null
  }

  return throttled
}
