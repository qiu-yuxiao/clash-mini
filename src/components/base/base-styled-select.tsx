import { Select, SelectProps, styled, useTheme } from '@mui/material'

import { getPopupMenuPaperProps } from '@/utils/popup-styles'

export const BaseStyledSelect = styled((props: SelectProps<string>) => {
  const theme = useTheme()
  const { MenuProps, ...rest } = props

  return (
    <Select
      size="small"
      autoComplete="new-password"
      sx={{
        width: 120,
        height: 33.375,
        mr: 1,
        '[role="button"]': { py: 0.65 },
      }}
      {...rest}
      MenuProps={{
        ...MenuProps,
        slotProps: {
          ...MenuProps?.slotProps,
          paper: MenuProps?.slotProps?.paper ?? getPopupMenuPaperProps(theme),
        },
      }}
    />
  )
})(({ theme }) => ({
  background: theme.palette.mode === 'light' ? '#fff' : undefined,
}))
