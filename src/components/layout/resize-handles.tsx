import {
  currentMonitor,
  PhysicalPosition,
  PhysicalSize,
} from '@tauri-apps/api/window'
import React, { useCallback, useEffect, useRef } from 'react'

import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
} from '@/constants'
import { useWindow } from '@/hooks/use-window'
import { frontendLog } from '@/services/cmds'
import { computeResizeGeometry } from '@/utils/resize-geometry'
import { setWindowResizing } from '@/utils/window-resizing'

const HANDLE_SIZE = 10

type Direction = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

const handles: { direction: Direction; style: React.CSSProperties }[] = [
  {
    direction: 'n',
    style: {
      top: 0,
      left: HANDLE_SIZE,
      right: HANDLE_SIZE,
      height: HANDLE_SIZE,
      cursor: 'ns-resize',
    },
  },
  {
    direction: 's',
    style: {
      bottom: 0,
      left: HANDLE_SIZE,
      right: HANDLE_SIZE,
      height: HANDLE_SIZE,
      cursor: 'ns-resize',
    },
  },
  {
    direction: 'e',
    style: {
      top: HANDLE_SIZE,
      right: 0,
      bottom: HANDLE_SIZE,
      width: HANDLE_SIZE,
      cursor: 'ew-resize',
    },
  },
  {
    direction: 'w',
    style: {
      top: HANDLE_SIZE,
      left: 0,
      bottom: HANDLE_SIZE,
      width: HANDLE_SIZE,
      cursor: 'ew-resize',
    },
  },
  {
    direction: 'ne',
    style: {
      top: 0,
      right: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nesw-resize',
    },
  },
  {
    direction: 'nw',
    style: {
      top: 0,
      left: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nwse-resize',
    },
  },
  {
    direction: 'se',
    style: {
      bottom: 0,
      right: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nwse-resize',
    },
  },
  {
    direction: 'sw',
    style: {
      bottom: 0,
      left: 0,
      width: HANDLE_SIZE * 2,
      height: HANDLE_SIZE * 2,
      cursor: 'nesw-resize',
    },
  },
]

interface ResizeSession {
  direction: Direction
  // 拖拽起始时的「外位置」（physical px，与 pointermove 换算出的屏幕位移同坐标系）
  startPhysX: number
  startPhysY: number
  // 拖拽起始时的「内尺寸」（客户区，physical px）
  // 【口径铁律】必须用 innerSize()，不可用 outerSize()：落地走 setSize()（Tauri 语义 = 内尺寸），
  // 且 MIN/MAX 边界也是内尺寸。混用会让每帧多出一个外框内缩量。
  startInnerW: number
  startInnerH: number
  // 拖拽起始鼠标屏幕坐标（physical px，与 outerPosition 同坐标系）
  startPointerX: number
  startPointerY: number
  // 缩放因子，用于 physical ↔ logical 转换
  scaleFactor: number
  // 拖拽期间使用的 pointerId，用于 setPointerCapture/releasePointerCapture
  pointerId: number
  // 当前帧待应用的 logical 尺寸/位置（rAF 节流）
  pendingW: number | null
  pendingH: number | null
  pendingX: number | null
  pendingY: number | null
  // 待应用的 physical 坐标（避免 physical -> logical -> physical 双重取整抖动）
  pendingPhysX: number | null
  pendingPhysY: number | null
  // 🛡️ [FM-03 防 IPC 洪水] 缓存上一次实际发出的几何值 (logical px 与 physical px)，比对去重
  lastAppliedW?: number
  lastAppliedH?: number
  lastAppliedX?: number
  lastAppliedY?: number
  lastAppliedPhysX?: number
  lastAppliedPhysY?: number
  // rAF 调度句柄；非 null 表示已有帧排队或正在执行
  rafId: number | null
  // applyPending 是否在飞行中（IPC 未完成），防止下一帧 rAF 触发并发 IPC 调用堆积
  inFlight: boolean
}

export const ResizeHandles: React.FC = () => {
  const { isLargeMode, currentWindow } = useWindow()
  // session ref 持有整个拖拽期间的不可变快照，pointermove/up 通过闭包读取
  const sessionRef = useRef<ResizeSession | null>(null)
  // 已在 pointerdown 登记、但「窗口几何量」尚未读回来的手势。
  // 用于：几何量回来之前用户就松手 / 丢失指针捕获时，仍能正确收尾（释放捕获、复位全局标志），
  // 避免指针捕获与 __isResizing 标志永久残留。
  const pendingDownRef = useRef<{
    pointerId: number
    cancelled: boolean
  } | null>(null)
  // applyPending ref：用于打破 scheduleApply ↔ applyPending 循环依赖
  const applyPendingRef = useRef<
    ((session: ResizeSession) => Promise<void>) | null
  >(null)

  const scheduleApply = useCallback((session: ResizeSession) => {
    // 已有帧排队 或 IPC 飞行中：跳过，避免并发 IPC 堆积
    if (session.rafId !== null || session.inFlight) return
    session.rafId = requestAnimationFrame(() => {
      void applyPendingRef.current?.(session)
    })
  }, [])

  const applyPending = useCallback(
    async (session: ResizeSession) => {
      // 取出本帧的待应用值，立即清空，避免下一帧重复应用
      const w = session.pendingW
      const h = session.pendingH
      const x = session.pendingX
      const y = session.pendingY
      const physX = session.pendingPhysX
      const physY = session.pendingPhysY
      session.pendingW = null
      session.pendingH = null
      session.pendingX = null
      session.pendingY = null
      session.pendingPhysX = null
      session.pendingPhysY = null
      session.rafId = null
      session.inFlight = true
      try {
        if (!currentWindow) return

        // 计算目标几何（如某维度无需调整则维持上次应用值或初始值）
        const targetW =
          w ?? session.lastAppliedW ?? session.startInnerW / session.scaleFactor
        const targetH =
          h ?? session.lastAppliedH ?? session.startInnerH / session.scaleFactor
        const targetX =
          x ?? session.lastAppliedX ?? session.startPhysX / session.scaleFactor
        const targetY =
          y ?? session.lastAppliedY ?? session.startPhysY / session.scaleFactor
        const targetPhysX =
          physX ?? session.lastAppliedPhysX ?? session.startPhysX
        const targetPhysY =
          physY ?? session.lastAppliedPhysY ?? session.startPhysY

        // 🛡️ [FM-03 防 IPC 洪水] 尺寸/位置变动与上一次已发出的值对比去重
        // 拖拽推到最小尺寸 271x135 边界时，高频 pointermove 计算出的目标值均为 271x135，
        // 经过此判断判定未变动，从而直接跳过发 IPC，彻底消除最小尺寸边界处的 IPC 堆积现象。
        const needsSize =
          (w !== null && targetW !== session.lastAppliedW) ||
          (h !== null && targetH !== session.lastAppliedH)
        const needsPos =
          (physX !== null && targetPhysX !== session.lastAppliedPhysX) ||
          (physY !== null && targetPhysY !== session.lastAppliedPhysY)

        if (!needsSize && !needsPos) return

        // 更新上次发出的实际几何记录
        session.lastAppliedW = targetW
        session.lastAppliedH = targetH
        session.lastAppliedX = targetX
        session.lastAppliedY = targetY
        session.lastAppliedPhysX = targetPhysX
        session.lastAppliedPhysY = targetPhysY

        // 并行发 IPC：setSize 和 setPosition 之间合并调用。
        // 【物理像素下发】位置直接使用 PhysicalPosition(targetPhysX, targetPhysY)，
        // 彻底消除 targetX * scaleFactor 浮点数取整带来的 1px 漂移/抖动。
        await Promise.all([
          needsSize
            ? currentWindow.setSize(
                new PhysicalSize(
                  Math.round(targetW * session.scaleFactor),
                  Math.round(targetH * session.scaleFactor),
                ),
              )
            : Promise.resolve(),
          needsPos
            ? currentWindow.setPosition(
                new PhysicalPosition(targetPhysX, targetPhysY),
              )
            : Promise.resolve(),
        ])
      } catch (err) {
        frontendLog(
          'error',
          `[ResizeHandle] setSize/setPosition failed: ${err}`,
        )
      } finally {
        session.inFlight = false
        // IPC 期间若又有新的 pending，立即再调度一帧应用，避免遗漏最后一帧
        if (
          session.pendingW !== null ||
          session.pendingH !== null ||
          session.pendingPhysX !== null ||
          session.pendingPhysY !== null
        ) {
          scheduleApply(session)
        }
      }
    },
    [currentWindow, scheduleApply],
  )

  // 每次 render 同步 ref，供 scheduleApply 在 rAF 回调中调用最新版本
  applyPendingRef.current = applyPending

  // 拖拽结束冷却定时器，隔离松手瞬间的并发 IPC 与重绘洪峰
  const cooldownTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // 组件卸载（含拖拽中途因切模式等卸载）时复位 resizing 标志并取消 pending 的 rAF 帧及冷却定时器，
  // 避免标志残留导致后续 onResized/updateWindowState 永远跳过 IPC 以及卸载后的异步调用。
  useEffect(
    () => () => {
      if (cooldownTimerRef.current !== null) {
        clearTimeout(cooldownTimerRef.current)
        cooldownTimerRef.current = null
      }
      setWindowResizing(false)
      pendingDownRef.current = null
      if (
        sessionRef.current?.rafId !== null &&
        sessionRef.current?.rafId !== undefined
      ) {
        cancelAnimationFrame(sessionRef.current.rafId)
      }
    },
    [],
  )

  const handlePointerDown = useCallback(
    (direction: Direction) => async (e: React.PointerEvent) => {
      if (!currentWindow || e.button !== 0) return
      e.preventDefault()
      e.stopPropagation()

      // 重新按下拖拽：若处于 200ms 冷却期间，立即取消冷却定时器
      if (cooldownTimerRef.current !== null) {
        clearTimeout(cooldownTimerRef.current)
        cooldownTimerRef.current = null
      }

      // ── 以下三步必须在 await 之前同步完成 ────────────────────────────────
      // ① 锁指针：React 在事件派发结束后会把 SyntheticEvent.currentTarget 置空，
      //    await 之后再读 e.currentTarget 只能拿到 null，setPointerCapture 抛出的
      //    TypeError 又被 catch 吞掉 —— 指针捕获从未建立。后果是窗口一边随拖拽变形、
      //    手柄一边从鼠标底下溜走，pointermove 立刻收不到，表现为「动一下就拖不动」。
      // ② 置位全局拖拽标志，尽早短路 window-provider / use-visibility 的 IPC。
      // ③ 记录按下时的指针坐标（位移一律按按下点绝对计算，await 期间丢失的 move 不会累积误差）。
      const target = e.currentTarget as HTMLElement
      try {
        target.setPointerCapture(e.pointerId)
      } catch (err) {
        frontendLog('warn', `[ResizeHandle] setPointerCapture failed: ${err}`)
      }
      setWindowResizing(true)
      pendingDownRef.current = { pointerId: e.pointerId, cancelled: false }
      const downClientX = e.clientX
      const downClientY = e.clientY

      // 读取窗口当前几何，所有读取并行发 IPC 减少往返。
      // 【口径】位置取 outerPosition（与 setPosition 对齐），尺寸取 innerSize（与 setSize 对齐）。
      const [pos, size, monitor] = await Promise.all([
        currentWindow.outerPosition(),
        currentWindow.innerSize(),
        currentMonitor(),
      ]).catch((err) => {
        frontendLog('error', `[ResizeHandle] read window state failed: ${err}`)
        return [null, null, null]
      })

      const pending = pendingDownRef.current
      pendingDownRef.current = null
      const gestureAlive =
        pending !== null &&
        !pending.cancelled &&
        pending.pointerId === e.pointerId

      if (!gestureAlive || !pos || !size) {
        // 几何量回来之前手势已结束，或读取失败：释放捕获并复位标志，不留残余拖拽态
        try {
          target.releasePointerCapture(e.pointerId)
        } catch {
          // 捕获可能已被系统自动释放，忽略
        }
        setWindowResizing(false)
        return
      }

      const scaleFactor = monitor?.scaleFactor || window.devicePixelRatio || 1

      sessionRef.current = {
        direction,
        startPhysX: pos.x,
        startPhysY: pos.y,
        startInnerW: size.width,
        startInnerH: size.height,
        // pointer events 的 clientX/Y 是 CSS 像素（logical），
        // 乘以缩放因子后与 outerPosition/innerSize 同为 physical 坐标系
        startPointerX: downClientX * scaleFactor,
        startPointerY: downClientY * scaleFactor,
        scaleFactor,
        pointerId: e.pointerId,
        pendingW: null,
        pendingH: null,
        pendingX: null,
        pendingY: null,
        pendingPhysX: null,
        pendingPhysY: null,
        rafId: null,
        inFlight: false,
      }
    },
    [currentWindow],
  )

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      // 手势在「窗口几何量还没读回来」阶段就结束（快速点击 / 捕获丢失）：
      // 只做取消登记，捕获与全局标志由 handlePointerDown 的续体统一收尾，避免早退留下残留状态。
      const pending = pendingDownRef.current
      if (pending && pending.pointerId === e.pointerId) {
        pending.cancelled = true
        return
      }

      const session = sessionRef.current
      if (!session || e.pointerId !== session.pointerId) return

      // 释放指针捕获
      const target = e.currentTarget as HTMLElement
      try {
        target.releasePointerCapture(session.pointerId)
      } catch (err) {
        frontendLog(
          'warn',
          `[ResizeHandle] releasePointerCapture failed: ${err}`,
        )
      }

      // 取消尚未触发的 rAF 帧
      if (session.rafId !== null) {
        cancelAnimationFrame(session.rafId)
        session.rafId = null
      }

      // 🛡️ [FM-04 防竞态] 仅在上一帧 IPC 未在途时立即应用收尾帧；
      // 若上一帧 IPC 正在飞行中 (inFlight === true)，applyPending 的 finally 块
      // 会在完成后自动拾取 pending 几何并安全应用最后一帧，避免并发 IPC 冲突。
      if (!session.inFlight) {
        void applyPending(session)
      }

      sessionRef.current = null

      // 🛡️ [FM-01 冷却恢复] 拖拽结束：启动 200ms 拖拽结束冷却定时器 (Cool-down Reset)，
      // 延迟重置全局 resizing 标志。
      // 避免在收尾 setSize/setPosition IPC 尚未完全落地沉淀前就解除拦截，
      // 导致下游 onResized / updateWindowState 立即涌入并发 IPC 查询和重绘造成松手瞬间卡顿。
      if (cooldownTimerRef.current !== null) {
        clearTimeout(cooldownTimerRef.current)
      }
      cooldownTimerRef.current = setTimeout(() => {
        setWindowResizing(false)
        cooldownTimerRef.current = null
      }, 200)
    },
    [applyPending],
  )

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      const session = sessionRef.current
      if (!session || e.pointerId !== session.pointerId) return

      // 🛡️ 终极防线：如果检测到鼠标左键已释放（e.buttons 不含左键 1 掩码），
      // 说明 pointerup 已经丢失。立即在此主动触发收尾清理，斩断假拖动状态。
      if ((e.buttons & 1) === 0) {
        handlePointerUp(e)
        return
      }

      // 计算鼠标在 physical 坐标系下的位移
      const currentPointerX = e.clientX * session.scaleFactor
      const currentPointerY = e.clientY * session.scaleFactor
      let dx = currentPointerX - session.startPointerX
      let dy = currentPointerY - session.startPointerY

      // 🛡️ [FM-05 消除反向拖拽死区] 动态锚点滑动 Clamping：
      // 当鼠标推过最小/最大尺寸边界时，动态滑动 session.startPointerX/Y 锚点，
      // 使得 dx/dy 紧贴临界边界。一旦鼠标反向移动，可实现 0 延迟即刻拉大/缩小，消除死区粘滞感。
      // 注意：这里的 MIN/MAX 与 computeResizeGeometry 同为「内尺寸」口径，基准值也用 startInnerW/H，
      // 且边界值统一引用 @/constants 的导出常量，避免两处魔数漂移（曾经就是这类重复埋的坑）。
      const sf = session.scaleFactor
      const MIN_W_PHYS = MIN_WINDOW_WIDTH * sf
      const MAX_W_PHYS = MAX_WINDOW_WIDTH * sf
      const MIN_H_PHYS = MIN_WINDOW_HEIGHT * sf
      const MAX_H_PHYS = MAX_WINDOW_HEIGHT * sf

      if (session.direction.includes('e')) {
        const minDx = MIN_W_PHYS - session.startInnerW
        const maxDx = MAX_W_PHYS - session.startInnerW
        if (dx < minDx) {
          session.startPointerX = currentPointerX - minDx
          dx = minDx
        } else if (dx > maxDx) {
          session.startPointerX = currentPointerX - maxDx
          dx = maxDx
        }
      } else if (session.direction.includes('w')) {
        const minDx = session.startInnerW - MAX_W_PHYS
        const maxDx = session.startInnerW - MIN_W_PHYS
        if (dx < minDx) {
          session.startPointerX = currentPointerX - minDx
          dx = minDx
        } else if (dx > maxDx) {
          session.startPointerX = currentPointerX - maxDx
          dx = maxDx
        }
      }

      if (session.direction.includes('s')) {
        const minDy = MIN_H_PHYS - session.startInnerH
        const maxDy = MAX_H_PHYS - session.startInnerH
        if (dy < minDy) {
          session.startPointerY = currentPointerY - minDy
          dy = minDy
        } else if (dy > maxDy) {
          session.startPointerY = currentPointerY - maxDy
          dy = maxDy
        }
      } else if (session.direction.includes('n')) {
        const minDy = session.startInnerH - MAX_H_PHYS
        const maxDy = session.startInnerH - MIN_H_PHYS
        if (dy < minDy) {
          session.startPointerY = currentPointerY - minDy
          dy = minDy
        } else if (dy > maxDy) {
          session.startPointerY = currentPointerY - maxDy
          dy = maxDy
        }
      }

      // 计算 clamp 后的新几何（logical），写入 pending（rAF 节流后统一应用）
      const geo = computeResizeGeometry({
        direction: session.direction,
        startInnerW: session.startInnerW,
        startInnerH: session.startInnerH,
        startPhysX: session.startPhysX,
        startPhysY: session.startPhysY,
        dx,
        dy,
        scaleFactor: session.scaleFactor,
      })
      session.pendingW = geo.width
      session.pendingH = geo.height
      session.pendingX = geo.x
      session.pendingY = geo.y
      session.pendingPhysX = geo.physX
      session.pendingPhysY = geo.physY

      scheduleApply(session)
    },
    [scheduleApply, handlePointerUp],
  )

  if (isLargeMode) return null

  return (
    <>
      {handles.map(({ direction, style }) => (
        <div
          key={direction}
          data-no-drag="true"
          onPointerDown={handlePointerDown(direction)}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerUp}
          onLostPointerCapture={handlePointerUp}
          style={{
            position: 'absolute',
            zIndex: 9999,
            userSelect: 'none',
            touchAction: 'none',
            ...style,
          }}
        />
      ))}
    </>
  )
}
