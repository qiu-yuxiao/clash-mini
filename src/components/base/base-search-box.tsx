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
  inputRef?: React.Ref<HTMLInputElement>
  endAdornmentExtra?: React.ReactNode
  onKeyDownExtra?: (e: React.KeyboardEvent<HTMLDivElement>) => void
  onChangeText?: (text: string) => void
  onEnter?: (text: string) => void
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
  inputRef,
  endAdornmentExtra,
  onKeyDownExtra,
  onChangeText,
  onEnter,
  onSearch,
}: SearchProps) => {
  const { t } = useTranslation()
  const onSearchRef = useRef(onSearch)

  const [text, setText] = useControllableState<string>({
    controlled: value,
    defaultValue: defaultValue ?? '',
  })
  // BUG-392: 中文 IME 组词期间保持受控输入框平滑显示，但不向外派发拼音中间态（如 "-x"）以免误触发全量/错误子集测速
  const isComposingRef = useRef(false)
  const [composingText, setComposingText] = useState<string | null>(null)
  const displayValue = composingText !== null ? composingText : text
  // BUG-343: 初始化记录当前文本，仅在外部受控值真正变化或用户输入变化时触发单次 emitSearch
  const lastSearchStateRef = useRef<SearchState | null>(
    text ? null : { text: '' },
  )

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

    lastSearchStateRef.current = { text: nextText }
    const compiled = compileStringMatcher(nextText)
    onSearchRef.current(compiled.matcher, { text: nextText })
  }, [])

  useEffect(() => {
    emitSearch(text)
  }, [emitSearch, text])

  const handleChangeText = (
    e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    const nextText = e.target?.value ?? ''
    if (isComposingRef.current) {
      setComposingText(nextText)
      return
    }
    setText(nextText)
    emitSearch(nextText)
    onChangeText?.(nextText)
  }

  const handleClearInput = () => {
    isComposingRef.current = false
    setComposingText(null)
    setText('')
    emitSearch('')
    onChangeText?.('')
  }

  return (
    <StyledTextField
      inputRef={inputRef}
      autoComplete="new-password"
      hiddenLabel
      fullWidth
      size="small"
      variant="outlined"
      autoFocus={autoFocus}
      spellCheck="false"
      placeholder={placeholder ?? t('shared.placeholders.filter')}
      sx={{ input: { py: 0.65, px: 1.25 } }}
      value={displayValue}
      onChange={handleChangeText}
      onCompositionStart={() => {
        isComposingRef.current = true
        setComposingText(displayValue)
      }}
      onCompositionEnd={(e) => {
        isComposingRef.current = false
        const committed =
          (e.target as HTMLInputElement | HTMLTextAreaElement)?.value ?? ''
        setComposingText(null)
        setText(committed)
        emitSearch(committed)
        onChangeText?.(committed)
      }}
      onBlur={() => {
        if (isComposingRef.current) {
          isComposingRef.current = false
          setComposingText(null)
        }
      }}
      onKeyDown={(e) => {
        onKeyDownExtra?.(e)
        if (
          e.key === 'Enter' &&
          !e.nativeEvent.isComposing &&
          e.keyCode !== 229
        ) {
          onEnter?.(text)
        }
      }}
      slotProps={{
        input: {
          sx: { pr: 4 },
          endAdornment: (
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
              {endAdornmentExtra}
              <Tooltip title={t('shared.placeholders.resetInput')}>
                <span>
                  <IconButton
                    size="small"
                    {...iconStyle}
                    onClick={handleClearInput}
                    disabled={!displayValue}
                    sx={{ ...iconStyle.sx, opacity: displayValue ? 1 : 0.5 }}
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
