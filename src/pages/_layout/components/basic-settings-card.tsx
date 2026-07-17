import { Box, Typography, TextField, useTheme } from '@mui/material'
import React from 'react'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import type { IVergeConfig } from '@/types/verge'
import { get3DCardStyle, get3DInputStyle } from '@/utils/button-styles'
import type { BaseConfig } from 'tauri-plugin-mihomo-api'

interface BasicSettingsCardProps {
  verge: IVergeConfig | null
  clashConfig: BaseConfig | null
  patchVerge: (val: Partial<IVergeConfig>) => Promise<void>
  handleAllowLanChange: (checked: boolean) => void
  handleIpv6Change: (checked: boolean) => void
  mixedPortVal: number
  handleSavePort: (port: number) => void
}

const gridItemSx = () => ({
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  px: 0.5,
  py: 0.15,
})

const labelSx = (isRetro3DDark: boolean) => ({
  fontSize: '13px',
  fontWeight: 'bold',
  color: isRetro3DDark ? '#2C1F03' : 'inherit',
  whiteSpace: 'nowrap' as const,
})

export const BasicSettingsCard: React.FC<BasicSettingsCardProps> = ({
  verge,
  clashConfig,
  patchVerge,
  handleAllowLanChange,
  handleIpv6Change,
  mixedPortVal,
  handleSavePort,
}) => {
  const { t } = useTranslation()
  const theme = useTheme()
  const skinFallback = React.useMemo(() => {
    return typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d'
  }, [])
  const skin = theme.controlSkin || skinFallback
  const isRetro3DDark = skin === 'retro-3d' && theme.palette.mode === 'dark'

  const [localPort, setLocalPort] = React.useState(mixedPortVal)
  const inputRef = React.useRef<HTMLInputElement | null>(null)

  React.useEffect(() => {
    if (document.activeElement !== inputRef.current) {
      setLocalPort(mixedPortVal)
    }
  }, [mixedPortVal])

  const onLocalSave = () => {
    handleSavePort(localPort)
  }

  return (
    <Box
      sx={{
        p: 1,
        flexShrink: 0,
        ...get3DCardStyle(theme, 'default'),
        '&:hover': {
          transform: 'none',
          boxShadow: get3DCardStyle(theme, 'default').boxShadow as string,
        },
      }}
    >
      <Typography
        variant="subtitle2"
        sx={{
          fontWeight: 'bold',
          mb: 0.5,
          fontSize: '13px',
          color: isRetro3DDark ? '#2C1F03' : 'inherit',
        }}
      >
        {t('settings.components.verge.basic.title', {
          defaultValue: '基础设置',
        })}
      </Typography>
      {/* 2×2 Grid: row1 = auto-launch + Allow LAN, row2 = silent-start + Allow IPv6 */}
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
          columnGap: 3,
        }}
      >
        {/* Row 1, Col 1: 开机自动启动 */}
        <Box sx={gridItemSx()}>
          <Typography variant="caption" sx={labelSx(isRetro3DDark)}>
            {t('settings.sections.system.fields.autoLaunch', {
              defaultValue: '开机自动启动',
            })}
          </Typography>
          <Switch
            size="small"
            checked={verge?.enable_auto_launch ?? false}
            onChange={(_, checked: boolean) =>
              patchVerge({ enable_auto_launch: checked })
            }
            sx={{
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Row 1, Col 2: Allow LAN */}
        <Box sx={gridItemSx()}>
          <Typography variant="caption" sx={labelSx(isRetro3DDark)}>
            Allow LAN
          </Typography>
          <Switch
            size="small"
            // WARNING: DO NOT change 'allowLan' to 'allow-lan'!
            // Although Clash core uses 'allow-lan', the Tauri backend serializes the BaseConfig struct
            // to camelCase ('allowLan') when sending it to the frontend.
            // Refer to #[serde(rename_all(serialize = "camelCase"))] on BaseConfig in models.rs.
            checked={clashConfig?.allowLan ?? false}
            onChange={(_, checked: boolean) => {
              handleAllowLanChange(checked)
            }}
            sx={{
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Row 2, Col 1: 启动时最小化 */}
        <Box sx={gridItemSx()}>
          <Typography variant="caption" sx={labelSx(isRetro3DDark)}>
            {t('settings.sections.system.fields.silentStart', {
              defaultValue: '启动时最小化',
            })}
          </Typography>
          <Switch
            size="small"
            checked={verge?.enable_silent_start ?? false}
            onChange={(_, checked: boolean) =>
              patchVerge({ enable_silent_start: checked })
            }
            sx={{
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Row 2, Col 2: Allow IPv6 */}
        <Box sx={gridItemSx()}>
          <Typography variant="caption" sx={labelSx(isRetro3DDark)}>
            Allow IPv6
          </Typography>
          <Switch
            size="small"
            checked={clashConfig?.ipv6 ?? false}
            onChange={(_, checked: boolean) => {
              handleIpv6Change(checked)
            }}
            sx={{
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Mixed Port: full-width row below grid */}
        <Box
          sx={{
            ...gridItemSx(),
            gridColumn: '1 / -1',
            mt: 0.25,
          }}
        >
          <Typography variant="caption" sx={labelSx(isRetro3DDark)}>
            Mixed Port
          </Typography>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
            <TextField
              inputRef={inputRef}
              size="small"
              type="text"
              value={localPort}
              onChange={(e) =>
                setLocalPort(
                  e.target.value ? parseInt(e.target.value, 10) || 0 : 0,
                )
              }
              onBlur={onLocalSave}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  ;(e.target as HTMLInputElement).blur()
                }
              }}
              slotProps={{
                htmlInput: {
                  style: {
                    paddingTop: '2px',
                    paddingBottom: '2px',
                    paddingLeft: '4px',
                    paddingRight: '4px',
                    width: '80px',
                    fontSize: '13px',
                    textAlign: 'center',
                  },
                },
              }}
              sx={get3DInputStyle(theme)}
            />
          </Box>
        </Box>
      </Box>
    </Box>
  )
}
