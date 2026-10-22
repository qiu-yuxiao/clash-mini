import {
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
  currentMonitor,
} from '@tauri-apps/api/window'
import type React from 'react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MINI_HEIGHT_THRESHOLD,
  MINI_WIDTH_THRESHOLD,
} from '@/constants'
import { withIpcTimeout } from '@/services/cmds'
import debounce from '@/utils/debounce'
import { isMiniByPhysical, isMinimalWidthByPhysical } from '@/utils/mini-window'
import { isWindowResizing } from '@/utils/window-resizing'

import { WindowContext, type WindowContextType } from './window-context'

/** FEAT-003: Idle duration (ms) before chrome auto-hides */
const IDLE_HIDE_DELAY_MS = 10_000

/** 大尺寸模式窗口尺寸（logical px），取自 @/constants（与后端 MAX_WIDTH/MAX_HEIGHT 同源） */
const LARGE_MODE_WIDTH = MAX_WINDOW_WIDTH
const LARGE_MODE_HEIGHT = MAX_WINDOW_HEIGHT

/** 进入大尺寸模式前的窗口尺寸持久化 key（跨会话恢复用） */
const LAST_NON_LARGE_SIZE_KEY = 'last-non-large-size'

/**
 * 确保窗口在当前屏幕内可见。
 * 大尺寸模式切换后调用，若窗口右下角超出屏幕边界则调整位置。
 */
async function ensureWindowInScreen(
  win: NonNullable<WindowContextType['currentWindow']>,
  targetWidth: number = LARGE_MODE_WIDTH,
  targetHeight: number = LARGE_MODE_HEIGHT,
): Promise<void> {
  try {
    const [pos, monitor] = await Promise.all([
      withIpcTimeout(win.outerPosition(), 5000, 'outerPosition'),
      withIpcTimeout(currentMonitor(), 5000, 'currentMonitor'),
    ])
    if (!monitor) return

    const factor = monitor.scaleFactor
    const physW = Math.round(targetWidth * factor)
    const physH = Math.round(targetHeight * factor)
    // 使用 workArea（排除任务栏）而非整个 monitor size
    const monW = monitor.workArea.size.width
    const monH = monitor.workArea.size.height
    const monX = monitor.workArea.position.x
    const monY = monitor.workArea.position.y

    let newX = pos.x
    let newY = pos.y
    if (newX + physW > monX + monW) newX = monX + monW - physW
    if (newY + physH > monY + monH) newY = monY + monH - physH
    if (newX < monX) newX = monX
    if (newY < monY) newY = monY

    // 仅在位置需要调整时调用 setPosition，减少 IPC 调用
    if (newX !== pos.x || newY !== pos.y) {
      await withIpcTimeout(
        win.setPosition(new PhysicalPosition(newX, newY)),
        5000,
        'setPosition',
      )
    }
  } catch (err) {
    console.warn('[WindowProvider] ensureWindowInScreen failed:', err)
  }
}

/**
 * 获取窗口所在显示器的缩放因子（physical ↔ logical 转换用）。
 * 统一使用 monitor.scaleFactor，与 ensureWindowInScreen 同源，
 * 避免混合 DPI 多屏场景下 window.devicePixelRatio 可能与实际显示器不一致。
 * 取值失败时回退到 window.devicePixelRatio。
 */
async function getScaleFactor(): Promise<number> {
  try {
    const monitor = await withIpcTimeout(
      currentMonitor(),
      5000,
      'currentMonitor',
    )
    if (monitor) return monitor.scaleFactor
  } catch {
    // 忽略，回退到 window.devicePixelRatio
  }
  return typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1
}

export const WindowProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const isTauri = typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__
  const currentWindow = useMemo(
    () => (isTauri ? getCurrentWindow() : null),
    [isTauri],
  )

  const [isLargeMode, setIsLargeMode] = useState(false)
  /** FEAT-003: true when custom titlebar is hidden by idle auto-hide timer (stealth mode) */
  const [isDecorationsHidden, setIsDecorationsHidden] = useState(false)
  /**
   * 内容宽度 ≤ 285px（窄窗口）：控制路由表格、流量面板布局。
   * 【仅供首帧布局】用的是页面视口（整数 CSS 像素）的粗估；
   * 权威判定见 applyGeometry（窗口内尺寸物理像素 + 1 物理像素容差），挂载后立即校正。
   */
  const [isMinimalWidth, setIsMinimalWidth] = useState(
    () =>
      typeof window !== 'undefined' &&
      window.innerWidth <= MINI_WIDTH_THRESHOLD,
  )
  /** 内容宽 ≤ 285 且 高 ≤ 126（最小窗口）：触发数据精简 + DOM卸载（同上，仅首帧粗估） */
  const [isMiniStatus, setIsMiniStatus] = useState(
    () =>
      typeof window !== 'undefined' &&
      window.innerWidth <= MINI_WIDTH_THRESHOLD &&
      window.innerHeight <= MINI_HEIGHT_THRESHOLD,
  )

  const idleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isDecorationsHiddenRef = useRef(false)
  /** 极小窗状态镜像。权威来源是窗口内尺寸（物理像素），由 applyGeometry 唯一写入 */
  const isMiniStatusRef = useRef(false)
  // ── Drag-vs-click detection (for stealth mode) ────────────────────────────
  const mouseDownPosRef = useRef<{ x: number; y: number } | null>(null)
  const dragStartedRef = useRef(false)

  // ── 大尺寸模式（替代原生 maximize）─────────────────────────────────────────
  /** isLargeMode 的 ref 镜像，供 onResized 闭包读取最新值，避免闭包陷阱 */
  const isLargeModeRef = useRef(false)
  /**
   * 最近一次非大尺寸模式的窗口尺寸（logical px）。
   * 进入大尺寸模式时保存当前尺寸，退出时恢复。
   * 初始值 {width:271, height:680} 与后端 DEFAULT_WIDTH/DEFAULT_HEIGHT 对齐，
   * 挂载后通过 innerSize() IPC 校正为实际值（兼容窗口状态恢复插件）。
   */
  const lastNonLargeSizeRef = useRef({ width: 271, height: 680 })

  const close = useCallback(async () => {
    if (!currentWindow) return
    await new Promise((resolve) => setTimeout(resolve, 20))
    await currentWindow.close()
  }, [currentWindow])

  const minimize = useCallback(async () => {
    if (!currentWindow) return
    await new Promise((resolve) => setTimeout(resolve, 10))
    await currentWindow.minimize()
  }, [currentWindow])

  // ── Idle timer（FEAT-003）：必须自续期，禁止自杀式退出 ──────────────────────
  const resetIdleTimer = useCallback(() => {
    if (idleTimerRef.current !== null) clearTimeout(idleTimerRef.current)

    // 用局部递归函数排程，保证每次触发后**无条件**重新武装。
    // 旧实现在条件不满足时直接 return，导致「启动 10 秒后窗口还不是极小窗」就把
    // 计时器永久注销，整场会话再也不会有下一次判定（v3.0.0~v3.0.3 失效主因之一）。
    const arm = () => {
      idleTimerRef.current = setTimeout(() => {
        idleTimerRef.current = null
        if (!isDecorationsHiddenRef.current && isMiniStatusRef.current) {
          isDecorationsHiddenRef.current = true
          setIsDecorationsHidden(true)
        }
        arm()
      }, IDLE_HIDE_DELAY_MS)
    }

    arm()
  }, [])

  // ── Restore chrome (show custom titlebar) ─────────────────────────────────
  const restoreChrome = useCallback(() => {
    // 旧实现在未隐藏时提前 return 且不续期，调用点（窗口变大）等于白调。
    // 这里无条件续期：恢复本身是一次交互，且让调用方不必依赖「已隐藏」这个前置条件。
    if (isDecorationsHiddenRef.current) {
      isDecorationsHiddenRef.current = false
      setIsDecorationsHidden(false)
    }
    resetIdleTimer()
  }, [resetIdleTimer])

  /**
   * 同步窄窗 / 极小窗状态（FEAT-003 唯一判定入口）。
   *
   * 入参必须是**窗口内尺寸的物理像素**（onResized payload / innerSize()），
   * 禁止传页面视口数值——视口是整数 CSS 像素（如 125% 档 158 物理 / 1.25 = 126.4，
   * 上报可能进位为 127），粗估不可靠，150% 档虽恰为整数但其余档位仍有取整方向问题。
   */
  const applyGeometry = useCallback(
    (physW: number, physH: number, sf: number) => {
      const nextIsMiniStatus = isMiniByPhysical(physW, physH, sf)
      const nextIsMinimalWidth = isMinimalWidthByPhysical(physW, sf)

      isMiniStatusRef.current = nextIsMiniStatus
      setIsMinimalWidth((prev) =>
        prev !== nextIsMinimalWidth ? nextIsMinimalWidth : prev,
      )
      setIsMiniStatus((prev) =>
        prev !== nextIsMiniStatus ? nextIsMiniStatus : prev,
      )

      // 离开极小窗尺寸（变宽或拉高）：立即退出隐匿、恢复标题栏
      if (!nextIsMiniStatus && isDecorationsHiddenRef.current) {
        restoreChrome()
      }
    },
    [restoreChrome],
  )

  // ── Resize listener: track large-mode + minimal width ────────────────────────
  useEffect(() => {
    if (!currentWindow) return
    let isUnmounted = false
    let lastWidth = -1
    let lastHeight = -1

    const checkMaximized = debounce(
      async (event: { payload: { width: number; height: number } }) => {
        if (isUnmounted) return
        // 拖拽调整窗口大小期间（resize-handles 设置 __isResizing）跳过处理，
        // 避免每像素 churn，也让位给拖拽自身的 setSize/setPosition。
        // 收尾的 onResized 在 __isResizing 清除后触发，会用最终尺寸正常刷新。
        if (isWindowResizing()) return
        const { width, height } = event.payload
        if (width === lastWidth && height === lastHeight) return
        lastWidth = width
        lastHeight = height

        // event.payload 是窗口内尺寸的物理像素——这是权威一手数据。
        // 窄窗/极小窗判定统一走 applyGeometry（物理像素域 + 1 物理像素容差），
        // 不再读 window.innerWidth/innerHeight（整数 CSS 像素会吃掉那 1 个物理像素）。
        const factor = window.devicePixelRatio || 1
        applyGeometry(width, height, factor)

        // 窗口几何变化也是一次交互：从这里起重新计时 10 秒。
        // 关键补强——拖窗口边缘走的是 Tauri 缩放边框子窗口，页面收不到 mousemove；
        // 拖页面内手柄又被 pointerdown 的 preventDefault 抑制了兼容鼠标事件。
        // 没有这一句，上述两条路径都无法给空闲计时器续命。
        // （续期在监听器入口每帧做，见下方 onResized 包装；这里不重复续期。）

        // 大尺寸模式逻辑：转换为 logical 比较
        const logicalW = width / factor
        const logicalH = height / factor
        const isLargeSize =
          Math.abs(logicalW - LARGE_MODE_WIDTH) < 1 &&
          Math.abs(logicalH - LARGE_MODE_HEIGHT) < 1

        if (isLargeModeRef.current && !isLargeSize) {
          // 从大尺寸模式拖动变小 → 自动退出大尺寸模式
          isLargeModeRef.current = false
          setIsLargeMode(false)
          // 更新 lastNonLargeSizeRef 为当前拖动后的尺寸
          lastNonLargeSizeRef.current = { width: logicalW, height: logicalH }
        } else if (!isLargeModeRef.current && !isLargeSize) {
          // 非大尺寸模式下拖动 → 更新 lastNonLargeSizeRef
          lastNonLargeSizeRef.current = { width: logicalW, height: logicalH }
        }
        // isLargeSize=true 且 isLargeModeRef.current=true：toggleMaximize 触发的 setSize，无需处理
      },
      300,
    )

    // 续期放在防抖之外、监听器入口处：onResized 每帧都续。
    // 若只在 300ms 防抖回调里续期，持续超过 10 秒的长拖期间防抖被反复推迟、
    // 回调一次都不执行，空闲计时器会在拖拽中途触发（用拖拽前的旧状态判定），
    // 造成标题栏瞬隐瞬显。入口处每帧续期后，拖拽全程计时器不会走到期。
    const unlistenPromise = currentWindow.onResized((event) => {
      resetIdleTimer()
      checkMaximized(event)
    })

    return () => {
      isUnmounted = true
      unlistenPromise
        .then((unlisten) => unlisten())
        .catch((err) => console.warn('[WindowProvider] 清理监听器失败:', err))
    }
  }, [currentWindow, applyGeometry, resetIdleTimer])

  // ── Activity listeners: reset timer on any user input ──────────────────────
  useEffect(() => {
    resetIdleTimer()

    const handleMouseDown = (e: MouseEvent) => {
      mouseDownPosRef.current = { x: e.clientX, y: e.clientY }
      dragStartedRef.current = false
      resetIdleTimer()
    }

    // L-16: mousemove 事件触发非常频繁，用 throttle 限制 resetIdleTimer 调用频率
    // 100ms 节流对用户体验无感知，但能显著减少定时器重置次数
    let lastMouseMoveTime = 0
    const THROTTLE_MS = 100
    const throttledResetIdle = () => {
      const now = Date.now()
      if (now - lastMouseMoveTime >= THROTTLE_MS) {
        lastMouseMoveTime = now
        resetIdleTimer()
      }
    }

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDecorationsHiddenRef.current) {
        throttledResetIdle()
        return
      }
      if (
        mouseDownPosRef.current &&
        !dragStartedRef.current &&
        (e.buttons & 1) !== 0
      ) {
        const dx = e.clientX - mouseDownPosRef.current.x
        const dy = e.clientY - mouseDownPosRef.current.y
        if (Math.sqrt(dx * dx + dy * dy) > 5) {
          const target = e.target as HTMLElement
          if (
            !target.closest(
              'button, a, input, select, textarea, [data-no-drag]',
            )
          ) {
            dragStartedRef.current = true
            currentWindow?.startDragging().catch(() => {
              console.warn('[window] startDragging failed')
              dragStartedRef.current = false
            })
          } else {
            mouseDownPosRef.current = null
          }
        }
      }
    }

    const handleMouseUp = (e: MouseEvent) => {
      if (
        isDecorationsHiddenRef.current &&
        mouseDownPosRef.current &&
        !dragStartedRef.current
      ) {
        const dx = e.clientX - mouseDownPosRef.current.x
        const dy = e.clientY - mouseDownPosRef.current.y
        if (Math.sqrt(dx * dx + dy * dy) < 5) {
          const target = e.target as HTMLElement
          if (!target.closest('[data-no-drag]')) {
            restoreChrome()
          }
        }
      }
      mouseDownPosRef.current = null
      dragStartedRef.current = false
    }

    const handleKeyActivity = () => {
      if (!isDecorationsHiddenRef.current) resetIdleTimer()
    }

    document.addEventListener('mousedown', handleMouseDown, { passive: true })
    document.addEventListener('mousemove', handleMouseMove, { passive: true })
    document.addEventListener('mouseup', handleMouseUp, { passive: true })
    document.addEventListener('keydown', handleKeyActivity, { passive: true })

    return () => {
      if (idleTimerRef.current !== null) clearTimeout(idleTimerRef.current)
      document.removeEventListener('mousedown', handleMouseDown)
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.removeEventListener('keydown', handleKeyActivity)
    }
  }, [resetIdleTimer, restoreChrome, currentWindow])

  // L-17: TOCTOU（检查-使用时间差）问题——isLargeMode 状态与操作之间
  // 可能有窗口状态变化。影响很小，try-catch 已能安全处理边界情况。
  const toggleMaximize = useCallback(async () => {
    if (!currentWindow) return
    try {
      if (!isLargeModeRef.current) {
        // 进入大尺寸模式：保存当前 innerSize，然后设为 640×860
        const innerSize = await withIpcTimeout(
          currentWindow.innerSize(),
          5000,
          'innerSize',
        )
        const factor = await getScaleFactor()
        lastNonLargeSizeRef.current = {
          width: innerSize.width / factor,
          height: innerSize.height / factor,
        }
        // 持久化「进入大尺寸前的尺寸」，供下次启动（窗口状态恢复插件恢复了 640×860 时）恢复
        if (typeof window !== 'undefined') {
          localStorage.setItem(
            LAST_NON_LARGE_SIZE_KEY,
            JSON.stringify(lastNonLargeSizeRef.current),
          )
        }

        await withIpcTimeout(
          currentWindow.setSize(
            new LogicalSize(LARGE_MODE_WIDTH, LARGE_MODE_HEIGHT),
          ),
          5000,
          'setSize-large',
        )

        await ensureWindowInScreen(currentWindow)

        isLargeModeRef.current = true
        setIsLargeMode(true)
      } else {
        // 退出大尺寸模式：恢复保存的尺寸
        const saved = lastNonLargeSizeRef.current
        await withIpcTimeout(
          currentWindow.setSize(new LogicalSize(saved.width, saved.height)),
          5000,
          'setSize-restore',
        )

        await ensureWindowInScreen(currentWindow, saved.width, saved.height)

        isLargeModeRef.current = false
        setIsLargeMode(false)
      }
    } catch (err) {
      console.warn('[WindowProvider] toggleMaximize failed:', err)
    }
  }, [currentWindow])

  useEffect(() => {
    if (!currentWindow) return
    currentWindow.setMinimizable?.(true)
    // Windows 禁用 OS 级 maximize（Win+上箭头、双击标题栏、任务栏右键），
    // 改由 toggleMaximize 自定义 640×860 大尺寸模式。
    currentWindow.setMaximizable?.(false)
    // 初始化 lastNonLargeSizeRef 为当前实际窗口尺寸（兼容窗口状态恢复插件）
    currentWindow
      .innerSize()
      .then(async (size) => {
        const factor = await getScaleFactor()
        const logicalW = size.width / factor
        const logicalH = size.height / factor

        // 首帧校正：以窗口内尺寸（物理像素）作为窄窗/极小窗判定的权威初值，
        // 覆盖 useState 里基于页面视口的粗估（视口是整数 CSS 像素，边界上差 1 像素）。
        applyGeometry(size.width, size.height, factor)

        // 若窗口状态恢复插件恢复了 640×860（大尺寸模式下关闭程序），
        // 标记为大尺寸模式，lastNonLargeSizeRef 恢复为持久化的「进入大尺寸前的尺寸」
        const isLargeSize =
          Math.abs(logicalW - LARGE_MODE_WIDTH) < 1 &&
          Math.abs(logicalH - LARGE_MODE_HEIGHT) < 1
        if (isLargeSize) {
          isLargeModeRef.current = true
          setIsLargeMode(true)
          // 读取持久化的「进入大尺寸前的尺寸」；无记录或数据损坏则回退默认
          // （默认值 = 后端 DEFAULT_WIDTH / DEFAULT_HEIGHT，见 resolve/window.rs）
          let restored = { width: 271, height: 680 }
          try {
            const raw = localStorage.getItem(LAST_NON_LARGE_SIZE_KEY)
            if (raw) {
              const parsed = JSON.parse(raw)
              if (
                parsed &&
                typeof parsed.width === 'number' &&
                typeof parsed.height === 'number'
              ) {
                restored = { width: parsed.width, height: parsed.height }
              }
            }
          } catch {
            // 忽略损坏数据，回退默认
          }
          lastNonLargeSizeRef.current = restored
        } else {
          lastNonLargeSizeRef.current = {
            width: logicalW,
            height: logicalH,
          }
        }
      })
      .catch((err) => {
        console.warn('[WindowProvider] 初始化 innerSize 失败:', err)
      })
  }, [currentWindow, applyGeometry])

  const contextValue = useMemo(
    () => ({
      // Whether the window has native decorations (title bar + borders).
      // Windows：始终 frameless（自定义标题栏），故恒为 false（平台不变量，无需进依赖）。
      decorated: false,
      isLargeMode,
      isDecorationsHidden,
      isMinimalWidth,
      isMiniStatus,
      minimize,
      close,
      toggleMaximize,
      currentWindow,
    }),
    [
      isLargeMode,
      isDecorationsHidden,
      isMinimalWidth,
      isMiniStatus,
      minimize,
      close,
      toggleMaximize,
      currentWindow,
    ],
  )

  return <WindowContext value={contextValue}>{children}</WindowContext>
}
