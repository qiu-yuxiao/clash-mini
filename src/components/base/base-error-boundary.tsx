import type { ReactNode } from 'react'
import { ErrorBoundary, type FallbackProps } from 'react-error-boundary'
import { useTranslation } from 'react-i18next'

// 错误边界分级策略（已按此落地，新增代码请遵循）：
//
// - 全局：`main.tsx` 的 BaseErrorBoundary —— 最后兜底，正常不应被触发
// - 页面：`BasePage`、`/unlock` 的 Outlet 各包一层 —— 单页崩溃不影响其他页
// - 区域：活跃节点栏 / 节点列表 / 连接表 / 流量图 / 弹窗组，统一用 AreaErrorFallback
//   局部降级（替换为一行提示而不是整页白屏）
//
// 原则：
// - 数据展示类（列表、图表）尽量细粒度，单条目失败不影响整体
// - 用户操作类稍粗粒度，保证操作的完整性
// - 错误边界**只能捕获渲染期异常**，捕获不到事件处理与异步代码里的错误
//   （异步错误请走 error-reporter / showNotice）
// - 边界不宜过密：优先给高频出错、依赖第三方渲染的组件加
function ErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
  const { t } = useTranslation()
  const errorMessage = error instanceof Error ? error.message : String(error)
  const errorStack = error instanceof Error ? error.stack : undefined

  return (
    <div role="alert" style={{ padding: 16 }}>
      <h4>⚠️ {t('shared.feedback.errorBoundary.pageTitle')}</h4>

      <pre>{errorMessage}</pre>

      <details title={t('shared.feedback.errorBoundary.details')}>
        <summary>{t('shared.feedback.errorBoundary.details')}</summary>
        <pre>{errorStack}</pre>
      </details>

      <button
        type="button"
        onClick={resetErrorBoundary}
        style={{
          marginTop: 12,
          padding: '8px 16px',
          cursor: 'pointer',
        }}
      >
        {t('shared.actions.retry')}
      </button>
    </div>
  )
}

export const AreaErrorFallback = ({ error }: FallbackProps) => {
  const { t } = useTranslation()
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
      <span>⚠️ {t('shared.feedback.errorBoundary.areaTitle')}</span>
      <span style={{ fontSize: 12, opacity: 0.7 }}>{errorMessage}</span>
    </div>
  )
}

interface Props {
  children?: ReactNode
  resetKey?: unknown
}

// L-21: 增强 BaseErrorBoundary，支持 resetKey 触发重置，避免白屏
export const BaseErrorBoundary = ({ children, resetKey }: Props) => {
  return (
    <ErrorBoundary
      FallbackComponent={ErrorFallback}
      resetKeys={resetKey !== undefined ? [resetKey] : undefined}
    >
      {children}
    </ErrorBoundary>
  )
}
