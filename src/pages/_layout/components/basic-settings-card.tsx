import { Box, Tooltip, Typography, TextField, useTheme } from '@mui/material'
import React from 'react'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import { getAutoLaunchStatus } from '@/services/cmds'
import type { IClashInfo } from '@/types/clash'
import type { IVergeConfig } from '@/types/verge'
import {
  get3DCardStyle,
  get3DInputStyle,
  get3DSegmentedActiveStyle,
  get3DSegmentedActiveTextColor,
  get3DSegmentedContainerStyle,
} from '@/utils/button-styles'
import type { BaseConfig } from 'tauri-plugin-mihomo-api'

interface BasicSettingsCardProps {
  verge: IVergeConfig | null
  clashConfig: BaseConfig | null
  clashInfo?: IClashInfo | null
  patchVerge: (val: Partial<IVergeConfig>) => Promise<void>
  handleAllowLanChange: (checked: boolean) => void
  handleIpv6Change: (checked: boolean) => void
  mixedPortVal: number
  handleSavePort: (port: number) => void
}

// BUG-341: 271px 窄窗与多语言（德/俄/英）适配——允许列收缩并省略过长标签，悬停 title 展示完整文案
const gridItemSx = () => ({
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  minWidth: 0,
  gap: 0.5,
  px: 0.5,
  py: 0.15,
})

const labelSx = (isRetro3DDark: boolean) => ({
  fontSize: '13px',
  fontWeight: 'bold',
  color: isRetro3DDark ? '#2C1F03' : 'inherit',
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap' as const,
})

export const BasicSettingsCard: React.FC<BasicSettingsCardProps> = ({
  verge,
  clashConfig,
  clashInfo,
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

  // 自启开关以「系统真实状态」为准，配置值仅作为回退：
  // 用户在任务管理器禁用、或注册表被改写时，配置值不会变，只看配置会显示错误状态。
  // 后端启动时已按配置对账过一次（autostart::reconcile_on_startup），这里负责把
  // 运行期间的外部改动反映回 UI。
  const isTogglingAutoLaunchRef = React.useRef(false)
  const [realAutoLaunch, setRealAutoLaunch] = React.useState<boolean | null>(
    null,
  )
  React.useEffect(() => {
    let alive = true
    getAutoLaunchStatus()
      .then((status) => {
        if (alive && !isTogglingAutoLaunchRef.current) setRealAutoLaunch(status)
      })
      .catch((err) => {
        console.warn('[BasicSettings] getAutoLaunchStatus failed:', err)
      })
    return () => {
      alive = false
    }
  }, [])

  React.useEffect(() => {
    if (document.activeElement !== inputRef.current) {
      setLocalPort(mixedPortVal)
    }
  }, [mixedPortVal])

  const onLocalSave = () => {
    handleSavePort(localPort)
  }

  const isTunModeEnabled = Boolean(verge?.enable_tun_mode)
  const normalizedSniffer =
    verge?.enable_tun_sniffer === true
      ? true
      : verge?.enable_tun_sniffer === false
        ? false
        : null
  const snifferActiveIndex =
    normalizedSniffer === null ? 0 : normalizedSniffer ? 1 : 2
  const snifferOptions = [
    {
      index: 0,
      value: null as boolean | null,
      key: 'settings.mini.tunSnifferAuto',
      defaultLabel: '自动',
    },
    {
      index: 1,
      value: true as boolean | null,
      key: 'settings.mini.tunSnifferOn',
      defaultLabel: '开启',
    },
    {
      index: 2,
      value: false as boolean | null,
      key: 'settings.mini.tunSnifferOff',
      defaultLabel: '关闭',
    },
  ] as const

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
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)',
          columnGap: 1.5,
        }}
      >
        {/* Row 1, Col 1: 开机自动启动 */}
        <Box sx={gridItemSx()}>
          <Typography
            variant="caption"
            title={t('settings.sections.system.fields.autoLaunch', {
              defaultValue: '开机自动启动',
            })}
            sx={labelSx(isRetro3DDark)}
          >
            {t('settings.sections.system.fields.autoLaunch', {
              defaultValue: '开机自动启动',
            })}
          </Typography>
          <Switch
            size="small"
            checked={realAutoLaunch ?? verge?.enable_auto_launch ?? false}
            onChange={async (_, checked: boolean) => {
              isTogglingAutoLaunchRef.current = true
              setRealAutoLaunch(checked)
              try {
                await patchVerge({ enable_auto_launch: checked })
                // 后端写入注册表成功后，再对账确认真实状态
                const status = await getAutoLaunchStatus()
                setRealAutoLaunch(status)
              } catch {
                setRealAutoLaunch(null)
                void getAutoLaunchStatus()
                  .then(setRealAutoLaunch)
                  .catch(() => {})
              } finally {
                isTogglingAutoLaunchRef.current = false
              }
            }}
            sx={{
              flexShrink: 0,
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Row 1, Col 2: Allow LAN */}
        <Box sx={gridItemSx()}>
          <Typography
            variant="caption"
            title="Allow LAN"
            sx={labelSx(isRetro3DDark)}
          >
            Allow LAN
          </Typography>
          <Switch
            size="small"
            // WARNING: DO NOT change 'allowLan' to 'allow-lan'!
            // Although Clash core uses 'allow-lan', the Tauri backend serializes the BaseConfig struct
            // to camelCase ('allowLan') when sending it to the frontend.
            // Refer to #[serde(rename_all(serialize = "camelCase"))] on BaseConfig in models.rs.
            checked={clashInfo?.allow_lan ?? clashConfig?.allowLan ?? false}
            onChange={(_, checked: boolean) => {
              handleAllowLanChange(checked)
            }}
            sx={{
              flexShrink: 0,
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Row 2, Col 1: 启动时最小化 */}
        <Box sx={gridItemSx()}>
          <Typography
            variant="caption"
            title={t('settings.sections.system.fields.silentStart', {
              defaultValue: '启动时最小化',
            })}
            sx={labelSx(isRetro3DDark)}
          >
            {t('settings.sections.system.fields.silentStart', {
              defaultValue: '启动时最小化',
            })}
          </Typography>
          <Switch
            size="small"
            checked={verge?.enable_silent_start ?? false}
            onChange={(_, checked: boolean) => {
              patchVerge({ enable_silent_start: checked }).catch((err) => {
                console.error('[BasicSettings] patchVerge failed:', err)
              })
            }}
            sx={{
              flexShrink: 0,
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Row 2, Col 2: Allow IPv6 */}
        <Box sx={gridItemSx()}>
          <Typography
            variant="caption"
            title="Allow IPv6"
            sx={labelSx(isRetro3DDark)}
          >
            Allow IPv6
          </Typography>
          <Switch
            size="small"
            checked={clashInfo?.ipv6 ?? false}
            onChange={(_, checked: boolean) => {
              handleIpv6Change(checked)
            }}
            sx={{
              flexShrink: 0,
              transform: 'scale(0.85)',
              transformOrigin: 'right center',
            }}
          />
        </Box>
        {/* Row 3 (Full Width): TUN 域名嗅探（三态：自动跟随 / 开启 / 关闭，仅 TUN 模式下生效） */}
        <Tooltip
          title={
            isTunModeEnabled
              ? t('settings.mini.tunSnifferTooltip', {
                  defaultValue:
                    'TUN 模式下识别缓存 IP 的原始域名（自动态将尊重订阅中显式关闭的 sniffer.enable）',
                })
              : t('settings.mini.tunSnifferNeedTunTooltip', {
                  defaultValue:
                    '仅在 TUN 模式下生效（当前未开启 TUN，切换后生效）',
                })
          }
          placement="top"
          arrow
        >
          <Box
            sx={{
              ...gridItemSx(),
              gridColumn: '1 / -1',
              opacity: isTunModeEnabled ? 1 : 0.48,
            }}
          >
            <Typography
              variant="caption"
              title={t('settings.mini.tunSniffer', {
                defaultValue: 'TUN 域名嗅探',
              })}
              sx={labelSx(isRetro3DDark)}
            >
              {t('settings.mini.tunSniffer', {
                defaultValue: 'TUN 域名嗅探',
              })}
            </Typography>
            <Box
              aria-disabled={!isTunModeEnabled}
              sx={(tTheme) => ({
                position: 'relative',
                display: 'flex',
                alignItems: 'center',
                width: 118,
                flexShrink: 0,
                bgcolor: 'action.hover',
                borderRadius: '4px',
                p: '1px',
                height: 20,
                userSelect: 'none',
                cursor: isTunModeEnabled ? 'pointer' : 'not-allowed',
                ...get3DSegmentedContainerStyle(tTheme),
              })}
            >
              <Box
                sx={{
                  position: 'absolute',
                  left: 0,
                  top: 0,
                  width: '33.333%',
                  height: '100%',
                  zIndex: 0,
                  transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                  transform: `translate3d(${snifferActiveIndex * 100}%, 0, 0)`,
                }}
              >
                <Box sx={(tTheme) => get3DSegmentedActiveStyle(tTheme)} />
              </Box>
              {snifferOptions.map((opt) => {
                const isSelected = snifferActiveIndex === opt.index
                return (
                  <Box
                    key={opt.key}
                    onClick={() => {
                      if (!isTunModeEnabled) return
                      if (normalizedSniffer === opt.value) return
                      patchVerge({ enable_tun_sniffer: opt.value }).catch(
                        (err) => {
                          console.error(
                            '[BasicSettings] patchVerge failed:',
                            err,
                          )
                        },
                      )
                    }}
                    sx={{
                      flex: 1,
                      height: '100%',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      fontSize: '11px',
                      fontWeight: 'bold',
                      zIndex: 1,
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      px: 0.25,
                      color: isSelected
                        ? get3DSegmentedActiveTextColor(theme)
                        : isRetro3DDark
                          ? 'rgba(44, 31, 3, 0.75)'
                          : 'text.secondary',
                    }}
                  >
                    {t(opt.key, { defaultValue: opt.defaultLabel })}
                  </Box>
                )
              })}
            </Box>
          </Box>
        </Tooltip>
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
