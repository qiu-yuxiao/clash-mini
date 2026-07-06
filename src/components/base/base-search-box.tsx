import { ClearRounded } from '@mui/icons-material'
import { Box, TextField, styled, IconButton } from '@mui/material'
import Tooltip from '@mui/material/Tooltip'
import { ChangeEvent, useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { get3DInputStyle } from '@/utils/button-styles'
import { compileStringMatcher } from '@/utils/search-matcher'

export type SearchState = {
  text: string
}

type SearchProps = {
  value?: string
  defaultValue?: string
  autoFocus?: boolean
  placeholder?: string
  onSearch: (match: (content: string) => boolean, state: SearchState) => void
}

const StyledTextField = styled(TextField)(({ theme }) => ({
  ...get3DInputStyle(theme),
  '& .MuiInputBase-root': {
    background: theme.palette.mode === 'light' ? '#fff' : undefined,
    paddingRight: '4px',
  },
  "& .MuiInputBase-root svg[aria-label='active'] path": {
    fill: theme.palette.primary.light,
  },
  "& .MuiInputBase-root svg[aria-label='inactive'] path": {
    fill: '#A7A7A7',
  },
}))

const useControllableState = <T,>(options: {
  controlled: T | undefined
  defaultValue: T
}) => {
  const { controlled, defaultValue } = options
  const [uncontrolled, setUncontrolled] = useState(defaultValue)
  const isControlled = controlled !== undefined

  const value = isControlled ? controlled : uncontrolled

  const setValue = useCallback(
    (next: T) => {
      if (!isControlled) setUncontrolled(next)
    },
    [isControlled],
  )

  return [value, setValue] as const
}

export const BaseSearchBox = ({
  value,
  defaultValue,
  autoFocus,
  placeholder,
  onSearch,
}: SearchProps) => {
  const { t } = useTranslation()
  const onSearchRef = useRef(onSearch)
  const lastSearchStateRef = useRef<SearchState | null>(null)

  const [text, setText] = useControllableState<string>({
    controlled: value,
    defaultValue: defaultValue ?? '',
  })

  const iconStyle = {
    fontSize: 18,
    sx: { cursor: 'pointer' },
  }

  useEffect(() => {
    onSearchRef.current = onSearch
  }, [onSearch])

  const emitSearch = useCallback((nextText: string) => {
    const prevState = lastSearchStateRef.current
    if (prevState && prevState.text === nextText) return

    const compiled = compileStringMatcher(nextText)
    onSearchRef.current(compiled.matcher, { text: nextText })

    lastSearchStateRef.current = { text: nextText }
  }, [])

  useEffect(() => {
    emitSearch(text)
  }, [emitSearch, text])

  const handleChangeText = (
    e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    const nextText = e.target?.value ?? ''
    setText(nextText)
    emitSearch(nextText)
  }

  const handleClearInput = () => {
    setText('')
    emitSearch('')
  }

  return (
    <StyledTextField
      autoComplete="new-password"
      hiddenLabel
      fullWidth
      size="small"
      variant="outlined"
      autoFocus={autoFocus}
      spellCheck="false"
      placeholder={placeholder ?? t('shared.placeholders.filter')}
      sx={{ input: { py: 0.65, px: 1.25 } }}
      value={text}
      onChange={handleChangeText}
      slotProps={{
        input: {
          sx: { pr: 4 },
          endAdornment: (
            <Box sx={{ display: 'flex', alignItems: 'center' }}>
              <Tooltip title={t('shared.placeholders.resetInput')}>
                <span>
                  <IconButton
                    size="small"
                    {...iconStyle}
                    onClick={handleClearInput}
                    disabled={!text}
                    sx={{ ...iconStyle.sx, opacity: text ? 1 : 0.5 }}
                  >
                    <ClearRounded fontSize="inherit" />
                  </IconButton>
                </span>
              </Tooltip>
            </Box>
          ),
        },
      }}
    />
  )
}
