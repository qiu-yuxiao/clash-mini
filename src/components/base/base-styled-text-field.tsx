import { TextField, type TextFieldProps, styled } from '@mui/material'
import { useTranslation } from 'react-i18next'

export const BaseStyledTextField = styled((props: TextFieldProps) => {
  const { t } = useTranslation()
  const { label, placeholder, slotProps: externalSlotProps, ...rest } = props

  return (
    <TextField
      autoComplete="new-password"
      label={label || placeholder || t('shared.placeholders.filter')}
      fullWidth
      size="small"
      variant="outlined"
      spellCheck="false"
      placeholder={placeholder ?? t('shared.placeholders.filter')}
      slotProps={{
        ...externalSlotProps,
        inputLabel: {
          shrink: true,
          ...(externalSlotProps?.inputLabel as Record<string, unknown> || {}),
        },
      }}
      sx={{ input: { py: 0.65, px: 1.25 } }}
      {...rest}
    />
  )
})(({ theme }) => ({
  '& .MuiInputBase-root': {
    background: theme.palette.mode === 'light' ? '#fff' : undefined,
  },
}))
