import { ClearRounded } from '@mui/icons-material'
import { Box, SvgIcon, TextField, styled, IconButton } from '@mui/material'
import Tooltip from '@mui/material/Tooltip'
import {
  ChangeEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'

import matchCaseIcon from '@/assets/image/component/match_case.svg?react'
import matchWholeWordIcon from '@/assets/image/component/match_whole_word.svg?react'
import UseRegularExpressionIcon from '@/assets/image/component/use_regular_expression.svg?react'
import { get3DInputStyle } from '@/utils/button-styles'
import { buildRegex, compileStringMatcher } from '@/utils/search-matcher'

export type SearchState = {
  text: string
  matchCase: boolean
  matchWholeWord: boolean
  useRegularExpression: boolean
}

type SearchOptionState = Omit<SearchState, 'text'>

type SearchProps = {
  value?: string
  defaultValue?: string
  autoFocus?: boolean
  placeholder?: string
  matchCase?: boolean
  matchWholeWord?: boolean
  useRegularExpression?: boolean
  searchState?: Partial<SearchOptionState>
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
  searchState,
  matchCase: defaultMatchCase = false,
  matchWholeWord: defaultMatchWholeWord = false,
  useRegularExpression: defaultUseRegularExpression = false,
  onSearch,
}: SearchProps) => {
  const { t } = useTranslation()
  const onSearchRef = useRef(onSearch)
  const lastSearchStateRef = useRef<SearchState | null>(null)

  const [text, setText] = useControllableState<string>({
    controlled: value,
    defaultValue: defaultValue ?? '',
  })

  const [matchCase, setMatchCase] = useControllableState<boolean>({
    controlled: searchState?.matchCase,
    defaultValue: defaultMatchCase,
  })

  const [matchWholeWord, setMatchWholeWord] = useControllableState<boolean>({
    controlled: searchState?.matchWholeWord,
    defaultValue: defaultMatchWholeWord,
  })

  const [useRegularExpression, setUseRegularExpression] =
    useControllableState<boolean>({
      controlled: searchState?.useRegularExpression,
      defaultValue: defaultUseRegularExpression,
    })

  const iconStyle = {
    fontSize: 18,
    sx: { cursor: 'pointer' },
  }

  useEffect(() => {
    onSearchRef.current = onSearch
  }, [onSearch])

  const emitSearch = useCallback((nextState: SearchState) => {
    const prevState = lastSearchStateRef.current
    const isSameState =
      !!prevState &&
      prevState.text === nextState.text &&
      prevState.matchCase === nextState.matchCase &&
      prevState.matchWholeWord === nextState.matchWholeWord &&
      prevState.useRegularExpression === nextState.useRegularExpression
    if (isSameState) return

    const compiled = compileStringMatcher(nextState.text, nextState)
    onSearchRef.current(compiled.matcher, nextState)

    lastSearchStateRef.current = nextState
  }, [])

  useEffect(() => {
    emitSearch({ text, matchCase, matchWholeWord, useRegularExpression })
  }, [emitSearch, matchCase, matchWholeWord, text, useRegularExpression])

  const effectiveErrorMessage = useMemo(() => {
    if (!useRegularExpression || !text) return ''
    const flags = matchCase ? '' : 'i'
    return buildRegex(text, flags) ? '' : t('shared.validation.invalidRegex')
  }, [matchCase, t, text, useRegularExpression])

  const handleChangeText = (
    e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    const nextText = e.target?.value ?? ''
    setText(nextText)
    emitSearch({
      text: nextText,
      matchCase,
      matchWholeWord,
      useRegularExpression,
    })
  }

  const handleToggleUseRegularExpression = () => {
    const next = !useRegularExpression
    setUseRegularExpression(next)
    emitSearch({
      text,
      matchCase,
      matchWholeWord,
      useRegularExpression: next,
    })
  }

  const handleClearInput = () => {
    setText('')
    emitSearch({ text: '', matchCase, matchWholeWord, useRegularExpression })
  }

  const handleToggleMatchCase = () => {
    const next = !matchCase
    setMatchCase(next)
    emitSearch({ text, matchCase: next, matchWholeWord, useRegularExpression })
  }

  const handleToggleMatchWholeWord = () => {
    const next = !matchWholeWord
    setMatchWholeWord(next)
    emitSearch({ text, matchCase, matchWholeWord: next, useRegularExpression })
  }

  return (
    <Tooltip title={effectiveErrorMessage || ''} placement="bottom-start">
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
        error={!!effectiveErrorMessage}
        slotProps={{
          input: {
            sx: { pr: 1 },
            endAdornment: (
              <Box sx={{ display: 'flex' }}>
                {!!text && (
                  <Tooltip title={t('shared.placeholders.resetInput')}>
                    <IconButton
                      size="small"
                      {...iconStyle}
                      onClick={handleClearInput}
                    >
                      <ClearRounded fontSize="inherit" />
                    </IconButton>
                  </Tooltip>
                )}
                <Tooltip title={t('shared.placeholders.matchCase')}>
                  <IconButton
                    size="small"
                    color={matchCase ? 'primary' : 'default'}
                    {...iconStyle}
                    onClick={handleToggleMatchCase}
                  >
                    <SvgIcon
                      component={matchCaseIcon}
                      fontSize="inherit"
                      aria-label={matchCase ? 'active' : 'inactive'}
                    />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t('shared.placeholders.matchWholeWord')}>
                  <IconButton
                    size="small"
                    color={matchWholeWord ? 'primary' : 'default'}
                    {...iconStyle}
                    onClick={handleToggleMatchWholeWord}
                  >
                    <SvgIcon
                      component={matchWholeWordIcon}
                      fontSize="inherit"
                      aria-label={matchWholeWord ? 'active' : 'inactive'}
                    />
                  </IconButton>
                </Tooltip>
                <Tooltip title={t('shared.placeholders.useRegex')}>
                  <IconButton
                    size="small"
                    color={useRegularExpression ? 'primary' : 'default'}
                    {...iconStyle}
                    onClick={handleToggleUseRegularExpression}
                  >
                    <SvgIcon
                      component={UseRegularExpressionIcon}
                      fontSize="inherit"
                      aria-label={useRegularExpression ? 'active' : 'inactive'}
                    />
                  </IconButton>
                </Tooltip>
              </Box>
            ),
          },
        }}
      />
    </Tooltip>
  )
}
