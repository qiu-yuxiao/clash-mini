import { Close, CropSquare, FilterNone, Minimize } from '@mui/icons-material'
import { Box, IconButton } from '@mui/material'

import { useWindowControls } from '@/hooks/use-window'
import getSystem from '@/utils/get-system'

export function WindowControls() {
  const OS = getSystem()
  const { isLargeMode, minimize, close, toggleMaximize } = useWindowControls()

  // 通过前端对 tauri 窗口进行翻转全屏时会短暂地与系统图标重叠渲染。
  // 这可能是上游缺陷，保险起见跨平台以窗口的最大化翻转为准。

  const btnSx = {
    width: '28px',
    height: '28px',
    p: 0,
    borderRadius: '4px',
    cursor: 'default',
  }

  return (
    <Box
      sx={{
        display: 'flex',
        gap: '2px',
        alignItems: 'center',
        flexShrink: 0,
        height: '30px',
      }}
    >
      {OS === 'macos' && (
        <>
          {/* macOS 风格：关闭 → 最小化 → 全屏 */}
          <IconButton sx={btnSx} onClick={close}>
            <Close sx={{ width: 14, height: 14 }} />
          </IconButton>
          <IconButton sx={btnSx} onClick={minimize}>
            <Minimize sx={{ width: 14, height: 14 }} />
          </IconButton>
          <IconButton sx={btnSx} onClick={toggleMaximize}>
            {isLargeMode ? (
              <FilterNone sx={{ width: 14, height: 14 }} />
            ) : (
              <CropSquare sx={{ width: 14, height: 14 }} />
            )}
          </IconButton>
        </>
      )}

      {OS === 'windows' && (
        <>
          {/* Windows 风格：最小化 → 最大化 → 关闭 */}
          <IconButton sx={btnSx} onClick={minimize}>
            <Minimize sx={{ width: 16, height: 16 }} />
          </IconButton>
          <IconButton sx={btnSx} onClick={toggleMaximize}>
            {isLargeMode ? (
              <FilterNone sx={{ width: 16, height: 16 }} />
            ) : (
              <CropSquare sx={{ width: 16, height: 16 }} />
            )}
          </IconButton>
          <IconButton
            sx={{
              ...btnSx,
              ':hover': { bgcolor: 'red', color: 'white' },
            }}
            onClick={close}
          >
            <Close sx={{ width: 16, height: 16 }} />
          </IconButton>
        </>
      )}

      {OS === 'linux' && (
        <>
          {/* Linux 桌面常见布局（GNOME/KDE 多为：最小化 → 最大化 → 关闭） */}
          <IconButton sx={btnSx} onClick={minimize}>
            <Minimize sx={{ width: 16, height: 16 }} />
          </IconButton>
          <IconButton sx={btnSx} onClick={toggleMaximize}>
            {isLargeMode ? (
              <FilterNone sx={{ width: 16, height: 16 }} />
            ) : (
              <CropSquare sx={{ width: 16, height: 16 }} />
            )}
          </IconButton>
          <IconButton
            sx={{
              ...btnSx,
              ':hover': { bgcolor: 'red', color: 'white' },
            }}
            onClick={close}
          >
            <Close sx={{ width: 16, height: 16 }} />
          </IconButton>
        </>
      )}
    </Box>
  )
}
