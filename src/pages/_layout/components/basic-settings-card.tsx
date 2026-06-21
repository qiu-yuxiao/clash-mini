import {
  Box,
  Typography,
  List,
  ListItem,
  TextField,
  useTheme,
} from '@mui/material'
import React from 'react'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import { get3DCardStyle, get3DInputStyle } from '@/utils/button-styles'

interface BasicSettingsCardProps {
  verge: any
  clashConfig: any
  patchVerge: (val: any) => Promise<void>
  handleAllowLanChange: (checked: boolean) => void
  mixedPortVal: number
  setMixedPortVal: (val: number) => void
  handleSavePort: () => void
}

export const BasicSettingsCard: React.FC<BasicSettingsCardProps> = ({
  verge,
  clashConfig,
  patchVerge,
  handleAllowLanChange,
  mixedPortVal,
  setMixedPortVal,
  handleSavePort,
}) => {
  const { t } = useTranslation() as any
  const theme = useTheme()
  const skin =
    (theme as any).controlSkin ||
    (typeof window !== 'undefined'
      ? localStorage.getItem('clash-mini-control-skin') || 'retro-3d'
      : 'retro-3d')
  const isRetro3DDark = skin === 'retro-3d' && theme.palette.mode === 'dark'

  return (
    <Box
      sx={{
        p: 1,
        flexShrink: 0,
        ...get3DCardStyle(theme, 'default'),
        '&:hover': {
          transform: 'none',
          boxShadow: get3DCardStyle(theme, 'default').boxShadow,
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
      <List dense sx={{ py: 0 }}>
        <ListItem
          sx={{
            py: 0.1,
            px: 0.5,
            display: 'flex',
            justifyContent: 'space-between',
          }}
        >
          <Typography
            variant="caption"
            sx={{
              fontSize: '13px',
              color: isRetro3DDark ? '#2C1F03' : 'inherit',
            }}
          >
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
              transform: 'scale(0.9)',
              transformOrigin: 'right center',
            }}
          />
        </ListItem>
        <ListItem
          sx={{
            py: 0.1,
            px: 0.5,
            display: 'flex',
            justifyContent: 'space-between',
          }}
        >
          <Typography
            variant="caption"
            sx={{
              fontSize: '13px',
              color: isRetro3DDark ? '#2C1F03' : 'inherit',
            }}
          >
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
              transform: 'scale(0.9)',
              transformOrigin: 'right center',
            }}
          />
        </ListItem>
        <ListItem
          sx={{
            py: 0.1,
            px: 0.5,
            display: 'flex',
            justifyContent: 'space-between',
          }}
        >
          <Typography
            variant="caption"
            sx={{
              fontSize: '13px',
              color: isRetro3DDark ? '#2C1F03' : 'inherit',
            }}
          >
            Allow LAN
          </Typography>
          <Switch
            size="small"
            checked={clashConfig?.['allow-lan'] ?? false}
            onChange={(_, checked: boolean) => {
              handleAllowLanChange(checked)
            }}
            sx={{
              transform: 'scale(0.9)',
              transformOrigin: 'right center',
            }}
          />
        </ListItem>
        <ListItem
          sx={{
            py: 0.25,
            px: 0.5,
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <Typography
            variant="caption"
            sx={{
              fontSize: '13px',
              color: isRetro3DDark ? '#2C1F03' : 'inherit',
            }}
          >
            Mixed Port
          </Typography>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
            <TextField
              size="small"
              type="text"
              value={mixedPortVal}
              onChange={(e) =>
                setMixedPortVal(
                  e.target.value
                    ? parseInt(e.target.value, 10) || 0
                    : 0,
                )
              }
              onBlur={handleSavePort}
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
        </ListItem>
      </List>
    </Box>
  )
}
