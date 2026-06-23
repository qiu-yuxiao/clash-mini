import '@mui/material/styles'

declare module '@mui/material/styles' {
  interface Theme {
    controlSkin: string
  }
  interface ThemeOptions {
    controlSkin?: string
  }
}
