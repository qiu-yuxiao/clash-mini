export interface ErrorContext {
  component?: string
  action?: string
  extra?: Record<string, unknown>
}

export function reportError(
  error: unknown,
  context: ErrorContext = {},
): void {
  const { component, action, extra } = context

  if (typeof window === 'undefined') {
    return
  }

  const errorMessage =
    error instanceof Error ? error.message : String(error)
  const errorStack = error instanceof Error ? error.stack : undefined

  console.error(
    `[ErrorReporter] ${component ? `[${component}]` : ''}${action ? `[${action}]` : ''} ${errorMessage}`,
    ...(errorStack ? [errorStack] : []),
    ...(extra ? [extra] : []),
  )

  // TODO: 集成后端错误上报服务
  // 未来可以将错误信息发送到后端日志收集服务，例如：
  // - Sentry
  // - 自建错误收集 API
  // - 日志服务（如 ELK、Loki 等）
  //
  // 上报时应注意：
  // 1. 去除敏感信息（密码、token、个人隐私数据等）
  // 2. 限制上报频率，避免性能影响
  // 3. 采样策略，生产环境可考虑降低采样率
  // 4. 增加用户标识（匿名）便于问题定位
  //
  // 示例伪代码：
  // if (process.env.NODE_ENV === 'production') {
  //   fetch('/api/errors/report', {
  //     method: 'POST',
  //     body: JSON.stringify({
  //       message: errorMessage,
  //       stack: errorStack,
  //       component,
  //       action,
  //       extra,
  //       url: window.location.href,
  //       userAgent: navigator.userAgent,
  //       timestamp: Date.now(),
  //     }),
  //   }).catch(() => {
  //     // 上报失败不能影响用户体验
  //   })
  // }
}

export function reportErrorAsync(
  promise: Promise<unknown>,
  context: ErrorContext = {},
): void {
  promise.catch((error) => {
    reportError(error, context)
  })
}
