import { createContextState } from 'foxact/create-context-state'

const [ThemeModeProvider, useThemeMode, useSetThemeMode] = createContextState<
  'light' | 'dark'
>()

export {
  ThemeModeProvider,
  useThemeMode,
  useSetThemeMode,
}
