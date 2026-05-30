import { styled } from '@mui/material/styles'
import { default as MuiSwitch, SwitchProps } from '@mui/material/Switch'

export const Switch = styled((props: SwitchProps) => (
  <MuiSwitch
    focusVisibleClassName=".Mui-focusVisible"
    disableRipple
    {...props}
  />
))(({ theme }) => ({
  width: 32,
  height: 18,
  padding: 0,
  marginRight: 1,
  '& .MuiSwitch-switchBase': {
    padding: 0,
    margin: 2,
    transitionDuration: '300ms',
    '&.Mui-checked': {
      transform: 'translateX(14px)',
      color: '#fff',
      '& + .MuiSwitch-track': {
        backgroundColor: theme.palette.primary.main,
        opacity: 1,
        border: 0,
        boxShadow: `inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
                    inset 0 0 calc(6px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.5 * var(--vibrancy-factor, 1.0))),
                    0 0 calc(4px * var(--vibrancy-factor, 1.0)) rgba(var(--primary-color-rgb), calc(0.15 * var(--vibrancy-factor, 1.0)))`,
      },
      '&.Mui-disabled + .MuiSwitch-track': {
        opacity: 0.5,
      },
    },
    '&.Mui-focusVisible .MuiSwitch-thumb': {
      border: '6px solid #fff',
    },
    '&.Mui-disabled .MuiSwitch-thumb': {
      color:
        theme.palette.mode === 'light'
          ? theme.palette.grey[100]
          : theme.palette.grey[600],
    },
    '&.Mui-disabled + .MuiSwitch-track': {
      opacity: theme.palette.mode === 'light' ? 0.7 : 0.3,
    },
  },
  '& .MuiSwitch-thumb': {
    boxSizing: 'border-box',
    width: 14,
    height: 14,
    boxShadow: `calc(1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) rgba(0,0,0,0.35),
                inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(255,255,255,0.6)`,
  },
  '& .MuiSwitch-track': {
    borderRadius: 18 / 2,
    backgroundColor: theme.palette.mode === 'light' ? '#BBBBBB' : '#39393D',
    opacity: 1,
    boxShadow: `inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(3px * var(--depth-factor, 1.0)) rgba(0,0,0,0.25),
                inset calc(-1px * var(--depth-factor, 1.0)) calc(-1px * var(--depth-factor, 1.0)) calc(1px * var(--depth-factor, 1.0)) rgba(255,255,255,0.1)`,
    transition: theme.transitions.create(['background-color'], {
      duration: 500,
    }),
  },
  '&:hover .MuiSwitch-thumb': {
    filter: 'brightness(1.15)',
    boxShadow: `calc(1px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(4px * var(--depth-factor, 1.0)) rgba(0,0,0,0.4),
                inset calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) calc(1.5px * var(--depth-factor, 1.0)) rgba(255,255,255,0.7)`,
  },
}))
