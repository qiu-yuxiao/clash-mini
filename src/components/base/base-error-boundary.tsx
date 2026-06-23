import { type ReactNode } from 'react'
import { ErrorBoundary, type FallbackProps } from 'react-error-boundary'

function ErrorFallback({ error }: FallbackProps) {
  const errorMessage = error instanceof Error ? error.message : String(error)
  const errorStack = error instanceof Error ? error.stack : undefined

  return (
    <div role="alert" style={{ padding: 16 }}>
      <h4>Something went wrong:(</h4>

      <pre>{errorMessage}</pre>

      <details title="Error Stack">
        <summary>Error Stack</summary>
        <pre>{errorStack}</pre>
      </details>
    </div>
  )
}

export const AreaErrorFallback = ({ error }: FallbackProps) => {
  const errorMessage = error instanceof Error ? error.message : String(error)
  return (
    <div
      role="alert"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
        color: 'var(--theme-text-secondary, #999)',
        fontSize: 14,
        gap: 12,
      }}
    >
      <span>⚠️ 该区域发生错误</span>
      <span style={{ fontSize: 12, opacity: 0.7 }}>{errorMessage}</span>
    </div>
  )
}

interface Props {
  children?: ReactNode
}

export const BaseErrorBoundary = ({ children }: Props) => {
  return (
    <ErrorBoundary FallbackComponent={ErrorFallback}>{children}</ErrorBoundary>
  )
}
