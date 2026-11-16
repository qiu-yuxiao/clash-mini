import { createTheme, type Theme } from '@mui/material'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { getSwitchSkinStyle } from '@/components/base/base-switch'
import { CUSTOM_TITLEBAR_HEIGHT, MIN_WINDOW_WIDTH } from '@/constants'
import {
  MIN_LOADING_NOTICE_DWELL_MS,
  getSnapshotNotices,
  hideNotice,
  markLoadingNotice,
  settleLoadingNotice,
  showNotice,
} from '@/services/notice-service'

import {
  getNarrowDialogSlotProps,
  getNarrowDialogSx,
  getPopupCardStyle,
  getPopupMenuPaperProps,
  getPopupSkinTokens,
  resolveNarrowOverlayTopOffset,
} from './popup-styles'

function makeTestTheme(mode: 'light' | 'dark', controlSkin: string): Theme {
  const theme = createTheme({ palette: { mode } })
  theme.controlSkin = controlSkin
  return theme
}

describe('popup-styles 窄窗弹窗几何锁定与皮肤统一样式', () => {
  afterEach(() => {
    for (const item of getSnapshotNotices()) {
      hideNotice(item.id)
    }
  })

  it('resolveNarrowOverlayTopOffset 正确避让自定义标题栏且在隐藏标题栏时贴顶', () => {
    expect(resolveNarrowOverlayTopOffset(null)).toBe(CUSTOM_TITLEBAR_HEIGHT)
    expect(resolveNarrowOverlayTopOffset(undefined)).toBe(
      CUSTOM_TITLEBAR_HEIGHT,
    )
    expect(
      resolveNarrowOverlayTopOffset({
        decorated: false,
        isDecorationsHidden: false,
      }),
    ).toBe(30)
    expect(
      resolveNarrowOverlayTopOffset({
        decorated: true,
        isDecorationsHidden: false,
      }),
    ).toBe(0)
    expect(
      resolveNarrowOverlayTopOffset({
        decorated: false,
        isDecorationsHidden: true,
      }),
    ).toBe(0)
  })

  it('getNarrowDialogSx 与 getNarrowDialogSlotProps 将弹窗、遮罩与卡片宽度严格锁定在左侧 271px', () => {
    const theme = makeTestTheme('dark', 'retro-3d')
    const rootSx = getNarrowDialogSx(CUSTOM_TITLEBAR_HEIGHT) as Record<
      string,
      unknown
    >
    expect(rootSx.top).toBe('30px')
    expect(rootSx.left).toBe(0)
    expect(rootSx.right).toBe('auto')
    expect(rootSx.width).toBe(`${MIN_WINDOW_WIDTH}px`)
    expect(rootSx.height).toBe('calc(100vh - 30px)')

    const slotProps = getNarrowDialogSlotProps(theme, CUSTOM_TITLEBAR_HEIGHT)
    const backdropSx = (slotProps?.backdrop as { sx?: Record<string, unknown> })
      ?.sx
    const containerSx = (
      slotProps?.container as { sx?: Record<string, unknown> }
    )?.sx
    const paper = slotProps?.paper as {
      className?: string
      sx?: Record<string, unknown>
    }

    expect(backdropSx?.left).toBe(0)
    expect(backdropSx?.top).toBe('30px')
    expect(backdropSx?.width).toBe('271px')
    expect(containerSx?.width).toBe('271px')
    expect(containerSx?.justifyContent).toBe('flex-start')
    expect(paper?.className).toBe('popup-card-paper')
    expect(paper?.sx?.m).toBe(0)
    expect(paper?.sx?.width).toBe('271px')
    expect(paper?.sx?.minWidth).toBe('271px')
    expect(paper?.sx?.maxWidth).toBe('271px')
    expect(paper?.sx?.maxHeight).toBe('calc(100vh - 30px - 8px)')
  })

  it('retro-3d 深色模式下弹窗使用深棕对比色而非白字', () => {
    const darkRetro = makeTestTheme('dark', 'retro-3d')
    const tokens = getPopupSkinTokens(darkRetro)
    expect(tokens.isRetro3D).toBe(true)
    expect(tokens.primaryText).toBe('#2C1F03')
    expect(tokens.secondaryText).toBe('rgba(44, 31, 3, 0.78)')

    const cardStyle = getPopupCardStyle(darkRetro)
    expect(cardStyle.color).toBe('#2C1F03')
  })

  it('frosted-glass 浮层与右键菜单强制高不透明实底，防止文字穿透重叠', () => {
    const lightFrosted = makeTestTheme('light', 'frosted-glass')
    const darkFrosted = makeTestTheme('dark', 'frosted-glass')

    const lightCard = getPopupCardStyle(lightFrosted)
    const darkCard = getPopupCardStyle(darkFrosted)
    expect(lightCard.background).toBe('rgba(248, 250, 252, 0.97)')
    expect(darkCard.background).toBe('rgba(26, 28, 38, 0.97)')
    expect(lightCard.backdropFilter).toBe('none')

    const menuPaper = getPopupMenuPaperProps(darkFrosted, { minWidth: 200 })
    expect(menuPaper.className).toBe('popup-card-paper')
    expect((menuPaper.sx as Record<string, unknown>).background).toBe(
      'rgba(26, 28, 38, 0.97)',
    )
    expect((menuPaper.sx as Record<string, unknown>).minWidth).toBe(200)
  })

  it('showNotice.warning 能正确入队并支持 hideNotice 关闭', () => {
    const id = showNotice.warning('test warning notice', 0)
    expect(id).toBeGreaterThanOrEqual(0)
    const active = getSnapshotNotices().find((item) => item.id === id)
    expect(active?.type).toBe('warning')
    expect(active?.duration).toBe(0)

    hideNotice(id)
    expect(getSnapshotNotices().some((item) => item.id === id)).toBe(false)
  })

  it('settleLoadingNotice 在极快操作完成后保留加载提示满最短停留时间再退场', () => {
    vi.useFakeTimers()
    try {
      const waitId = showNotice.info('loading-dwell-fast', 0)
      markLoadingNotice(waitId)

      vi.advanceTimersByTime(50)
      settleLoadingNotice(waitId)
      expect(getSnapshotNotices().some((item) => item.id === waitId)).toBe(true)

      vi.advanceTimersByTime(MIN_LOADING_NOTICE_DWELL_MS - 50 - 1)
      expect(getSnapshotNotices().some((item) => item.id === waitId)).toBe(true)

      vi.advanceTimersByTime(1)
      expect(getSnapshotNotices().some((item) => item.id === waitId)).toBe(
        false,
      )
    } finally {
      vi.useRealTimers()
    }
  })

  it('settleLoadingNotice 在耗时操作超过最短停留时间后立即关闭', () => {
    vi.useFakeTimers()
    try {
      const waitId = showNotice.info('loading-dwell-slow', 0)
      markLoadingNotice(waitId)

      vi.advanceTimersByTime(MIN_LOADING_NOTICE_DWELL_MS + 10)
      settleLoadingNotice(waitId)
      expect(getSnapshotNotices().some((item) => item.id === waitId)).toBe(
        false,
      )
    } finally {
      vi.useRealTimers()
    }
  })

  it('hideNotice 手动关闭加载提示时应清理挂起的 settle 定时器', () => {
    vi.useFakeTimers()
    try {
      const waitId = showNotice.info('loading-dwell-cancel', 0)
      markLoadingNotice(waitId)

      vi.advanceTimersByTime(40)
      settleLoadingNotice(waitId)
      hideNotice(waitId)
      expect(getSnapshotNotices().some((item) => item.id === waitId)).toBe(
        false,
      )

      const nextId = showNotice.success('loading-dwell-next', 0)
      vi.advanceTimersByTime(MIN_LOADING_NOTICE_DWELL_MS)
      expect(getSnapshotNotices().some((item) => item.id === nextId)).toBe(true)
      hideNotice(nextId)
    } finally {
      vi.useRealTimers()
    }
  })

  it('BUG-384: Monochrome 与 Cyberpunk 开关共享 0px 直角方形小尺寸几何且黑白开闭态对比鲜明', () => {
    for (const mode of ['light', 'dark'] as const) {
      const monoStyle = getSwitchSkinStyle(makeTestTheme(mode, 'monochrome'))
      const cyberStyle = getSwitchSkinStyle(makeTestTheme(mode, 'cyberpunk'))

      expect(monoStyle.width).toBe(28)
      expect(monoStyle.height).toBe(14)
      expect(monoStyle.width).toBe(cyberStyle.width)
      expect(monoStyle.height).toBe(cyberStyle.height)

      const monoSmall = monoStyle['&.MuiSwitch-sizeSmall'] as CSSObjectLike
      const cyberSmall = cyberStyle['&.MuiSwitch-sizeSmall'] as CSSObjectLike
      expect(monoSmall.width).toBe('18px !important')
      expect(monoSmall.height).toBe('10px !important')
      expect(monoSmall.width).toBe(cyberSmall.width)
      expect(monoSmall.height).toBe(cyberSmall.height)

      const monoThumb = monoStyle['& .MuiSwitch-thumb'] as CSSObjectLike
      const monoTrack = monoStyle['& .MuiSwitch-track'] as CSSObjectLike
      expect(monoThumb.borderRadius).toBe('0px')
      expect(monoTrack.borderRadius).toBe(0)

      const monoBase = monoStyle['& .MuiSwitch-switchBase'] as CSSObjectLike
      const monoChecked = monoBase['&.Mui-checked'] as CSSObjectLike
      const monoCheckedTrack = monoChecked[
        '& + .MuiSwitch-track'
      ] as CSSObjectLike
      const monoCheckedThumb = monoChecked[
        '& .MuiSwitch-thumb'
      ] as CSSObjectLike

      expect(monoCheckedTrack.backgroundColor).not.toBe(
        monoTrack.backgroundColor,
      )
      expect(monoCheckedThumb.backgroundColor).not.toBe(monoThumb.background)
    }
  })
})

type CSSObjectLike = Record<string, unknown>
