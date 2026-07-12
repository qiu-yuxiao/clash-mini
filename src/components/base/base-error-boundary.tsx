import { type ReactNode } from 'react'
import { ErrorBoundary, type FallbackProps } from 'react-error-boundary'

// TODO(error-boundary-granularity): 当前错误边界粒度较粗，主要是全局级别的
// 未来改进方向：按模块拆分错误边界，实现局部错误隔离
//
// 现状与问题：
// - BaseErrorBoundary 主要用于应用根级别包裹，任何子组件出错都会导致整个页面降级
// - 虽然 AreaErrorFallback 已提供区域级别的降级 UI，但实际使用粒度还不够细
// - 局部错误（如某个卡片、某个表格列）可能导致较大范围的白屏/降级
//
// 改进方案：
// 1. 页面级错误边界：每个路由页面独立包裹，单个页面崩溃不影响其他页面
// 2. 模块级错误边界：代理列表、连接表、设置页面等大模块独立包裹
// 3. 组件级错误边界：
//    - 代理卡片组件（单个代理节点渲染失败不影响整个列表）
//    - 图表组件（流量图、延迟图等）
//    - 配置编辑器（Monaco 编辑器加载失败）
//    - 订阅预览卡片
// 4. 提供 useErrorBoundary hook 便于业务组件主动触发错误边界
// 5. 结合错误上报（reportError），在错误边界中自动上报错误
//
// 拆分策略：
// - 数据展示类组件：尽量细粒度，单个条目失败不影响整体
// - 用户操作类组件：稍粗粒度，确保操作完整性
// - 核心功能（代理切换、配置保存）：应有独立的错误处理和用户提示
//
// 风险与权衡：
// - 错误边界过多会增加复杂度和性能开销
// - 应优先在高频出错、第三方依赖多的组件处添加
// - 错误边界只能捕获渲染阶段错误，无法捕获事件处理、异步代码中的错误
function ErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
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

      <button
        onClick={resetErrorBoundary}
        style={{
          marginTop: 12,
          padding: '8px 16px',
          cursor: 'pointer',
        }}
      >
        Try again
      </button>
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
