import type {
  CheckOptions,
  DownloadEvent,
  DownloadOptions,
} from '@tauri-apps/plugin-updater'

const DEFAULT_MIXED_PORT = 10801
const CHECK_PROXY_TIMEOUT_MS = 15_000
const CHECK_DIRECT_TIMEOUT_MS = 20_000
const CHECK_RETRY_DELAY_MS = 1_000
const DOWNLOAD_ATTEMPT_TIMEOUT_MS = 180_000
const DOWNLOAD_RETRY_DELAYS_MS = [1_500, 2_500] as const

export interface ClientUpdateHandle {
  version: string
  downloadAndInstall: (
    onEvent?: (progress: DownloadEvent) => void,
    options?: DownloadOptions,
  ) => Promise<void>
}

export function resolveLocalMixedProxyUrl(
  vergeMixedPort?: number | null,
  clashMixedPort?: number | null,
): string {
  const port = vergeMixedPort ?? clashMixedPort ?? DEFAULT_MIXED_PORT
  const validPort =
    Number.isInteger(port) && port >= 1 && port <= 65535
      ? port
      : DEFAULT_MIXED_PORT
  return `http://127.0.0.1:${validPort}`
}

function formatErr(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

const defaultSleep = (ms: number) =>
  new Promise<void>((resolve) => {
    setTimeout(resolve, ms)
  })

export async function checkClientUpdateWithFallback<
  T extends ClientUpdateHandle,
>(params: {
  localProxyUrl?: string
  checkFn: (options?: CheckOptions) => Promise<T | null>
  logError: (msg: string) => void
  sleepFn?: (ms: number) => Promise<void>
}): Promise<T | null> {
  const { localProxyUrl, checkFn, logError, sleepFn = defaultSleep } = params

  if (localProxyUrl) {
    try {
      return await checkFn({
        proxy: localProxyUrl,
        timeout: CHECK_PROXY_TIMEOUT_MS,
      })
    } catch (proxyErr) {
      logError(
        `[ClientUpdate] 经本地代理 (${localProxyUrl}) 检查更新失败，回退系统/直连通道重试: ${formatErr(proxyErr)}`,
      )
      await sleepFn(CHECK_RETRY_DELAY_MS)
    }
  }

  try {
    return await checkFn({ timeout: CHECK_DIRECT_TIMEOUT_MS })
  } catch (directErr) {
    logError(`[ClientUpdate] 检查客户端更新最终失败: ${formatErr(directErr)}`)
    throw directErr
  }
}

export async function downloadAndInstallClientUpdateWithRetry<
  T extends ClientUpdateHandle,
>(params: {
  initialUpdate: T
  onAttemptReset?: (attempt: number, maxAttempts: number) => void
  onEvent: (event: DownloadEvent) => void
  recheckDirectFn?: () => Promise<T | null>
  logError: (msg: string) => void
  sleepFn?: (ms: number) => Promise<void>
}): Promise<void> {
  const {
    initialUpdate,
    onAttemptReset,
    onEvent,
    recheckDirectFn,
    logError,
    sleepFn = defaultSleep,
  } = params

  const maxAttempts = DOWNLOAD_RETRY_DELAYS_MS.length + 1
  let currentUpdate: T = initialUpdate
  let lastError: unknown

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    let downloadFinished = false
    if (attempt > 1) {
      onAttemptReset?.(attempt, maxAttempts)
    }

    if (attempt === maxAttempts && recheckDirectFn) {
      try {
        const fallbackUpdate = await recheckDirectFn()
        if (
          fallbackUpdate &&
          fallbackUpdate.version === initialUpdate.version
        ) {
          currentUpdate = fallbackUpdate
        }
      } catch (recheckErr) {
        logError(
          `[ClientUpdate] 第 ${attempt}/${maxAttempts} 次重试前刷新直连更新句柄失败（继续使用原句柄）: ${formatErr(recheckErr)}`,
        )
      }
    }

    try {
      await currentUpdate.downloadAndInstall(
        (ev) => {
          if (ev.event === 'Finished') {
            downloadFinished = true
          }
          onEvent(ev)
        },
        { timeout: DOWNLOAD_ATTEMPT_TIMEOUT_MS },
      )
      return
    } catch (err) {
      lastError = err
      if (downloadFinished) {
        logError(
          `[ClientUpdate] 安装包已下载完成但校验或启动安装器失败（不自动重试以防重复弹窗）: ${formatErr(err)}`,
        )
        throw err
      }

      if (attempt < maxAttempts) {
        const waitMs = DOWNLOAD_RETRY_DELAYS_MS[attempt - 1]
        logError(
          `[ClientUpdate] 下载安装包中断 (第 ${attempt}/${maxAttempts} 次): ${formatErr(err)}，${waitMs}ms 后自动重试`,
        )
        await sleepFn(waitMs)
      } else {
        logError(
          `[ClientUpdate] 客户端升级下载连续 ${maxAttempts} 次失败: ${formatErr(err)}`,
        )
      }
    }
  }

  throw lastError
}
