import { invoke } from '@tauri-apps/api/core'
import dayjs from 'dayjs'

import { showNotice } from '@/services/notice-service'
import type {
  IConfigData,
  IProxyItem,
  IProxyGroupItem,
  ILogItem,
  IClashInfo,
} from '@/types/clash'
import type {
  IProfileItem,
  IProfilesConfig,
  IProfileOption,
} from '@/types/profile'
import type { IVergeConfig, ValidationOutcome } from '@/types/verge'
import { debugLog, isDebugLoggingEnabled } from '@/utils/debug'
import { isDummyNode } from '@/utils/node'
import { getProxies, getProxyProviders } from 'tauri-plugin-mihomo-api'

/**
 * 前端日志转发：将诊断日志写入后端 latest.log 文件
 * 用于 UI 线程卡死时（DevTools 无法打开）仍能在后端日志中看到前端 IPC 调用时间线
 */
export function frontendLog(level: 'info' | 'warn' | 'error', message: string) {
  // ERROR 始终转发；INFO/WARN 仅在 debug 开启时转发（见 src/utils/debug 的 isDebugLoggingEnabled）
  if (level !== 'error' && !isDebugLoggingEnabled()) return
  invoke('frontend_log', { level, message }).catch((err) => {
    // 日志通道本身失败时，回退到 console（不递归调 frontendLog 避免死循环）
    console.warn('[frontendLog] 日志转发失败，回退到 console:', err, '原始消息:', message)
  })
}

/**
 * H-17: IPC 超时包装工具函数
 * 对关键 IPC 调用包裹超时保护，防止后端卡住时前端 Promise 永远 pending。
 * 超时后 reject 并附带超时信息，便于调用方统一 catch 处理。
 */
export function withIpcTimeout<T>(
  promise: Promise<T>,
  ms: number,
  label = 'IPC',
): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      const msg = `[${label}] IPC call timed out after ${ms}ms`
      console.error(msg)
      showNotice.error(msg)
      reject(new Error(msg))
    }, ms)

    promise.then(
      (val) => {
        clearTimeout(timer)
        resolve(val)
      },
      (err) => {
        clearTimeout(timer)
        reject(err)
      },
    )
  })
}

export async function getProfiles() {
  return withIpcTimeout(
    invoke<IProfilesConfig>('get_profiles'),
    30_000,
    'getProfiles',
  )
}

export async function triggerAutoSelect(
  profileUid: string,
  nodeNames?: string[],
  sortType?: number,
  select?: boolean,
): Promise<Array<[string, number]>> {
  return withIpcTimeout(
    invoke<Array<[string, number]>>('trigger_auto_select', {
      profileUid,
      nodeNames: nodeNames ?? null,
      sortType: sortType ?? 0,
      select: select ?? true,
    }),
    60_000,
    'triggerAutoSelect',
  )
}

export async function enhanceProfiles() {
  const result = await withIpcTimeout(
    invoke<ValidationOutcome>('enhance_profiles'),
    60_000,
    'enhanceProfiles',
  )
  if (result.status !== 'valid') {
    const msg =
      result.status === 'invalid'
        ? result.message
        : result.status === 'skipped'
          ? result.reason
          : 'enhance_profiles failed'
    throw new Error(msg)
  }
  return true
}

export async function patchProfilesConfig(profiles: IProfilesConfig) {
  return (
    (
      await withIpcTimeout(
        invoke<ValidationOutcome>('patch_profiles_config', { profiles }),
        30_000,
        'patchProfilesConfig',
      )
    ).status === 'valid'
  )
}

export async function viewProfile(index: string) {
  return withIpcTimeout(
    invoke<void>('view_profile', { index }),
    30_000,
    'viewProfile',
  )
}

export async function readProfileFile(index: string) {
  return withIpcTimeout(
    invoke<string>('read_profile_file', { index }),
    10_000,
    'readProfileFile',
  )
}

export async function saveProfileFile(index: string, fileData: string) {
  return (
    (
      await withIpcTimeout(
        invoke<ValidationOutcome>('save_profile_file', {
          index,
          fileData,
        }),
        30_000,
        'saveProfileFile',
      )
    ).status === 'valid'
  )
}

export async function importProfile(url: string, option?: IProfileOption) {
  return withIpcTimeout(
    invoke<void>('import_profile', {
      url,
      option: option || { with_proxy: true },
    }),
    60_000,
    'importProfile',
  )
}

export async function updateProfile(index: string, option?: IProfileOption) {
  return withIpcTimeout(
    invoke<void>('update_profile', { index, option }),
    30_000,
    'updateProfile',
  )
}

export async function deleteProfile(index: string) {
  return withIpcTimeout(
    invoke<void>('delete_profile', { index }),
    30_000,
    'deleteProfile',
  )
}

export async function patchProfile(
  index: string,
  profile: Partial<IProfileItem>,
) {
  return withIpcTimeout(
    invoke<void>('patch_profile', { index, profile }),
    30_000,
    'patchProfile',
  )
}

export async function getClashInfo() {
  return withIpcTimeout(
    invoke<IClashInfo | null>('get_clash_info'),
    10_000,
    'getClashInfo',
  )
}

// Get runtime config which controlled by verge
export async function getRuntimeConfig() {
  return withIpcTimeout(
    invoke<IConfigData | null>('get_runtime_config'),
    10_000,
    'getRuntimeConfig',
  )
}

export async function patchClashConfig(payload: Partial<IConfigData>) {
  return withIpcTimeout(
    invoke<void>('patch_clash_config', { payload }),
    30_000,
    'patchClashConfig',
  )
}

export async function patchClashMode(payload: string) {
  return withIpcTimeout(
    invoke<void>('patch_clash_mode', { payload }),
    30_000,
    'patchClashMode',
  )
}

/**
 * 计算并返回所有代理数据。
 *
 * 【核心架构约定 - 切勿误判为多组架构】
 * Clash Mini 把所有上游代理组的节点合并到唯一一个 PROXY 组中。
 * 返回的 `groups` 数组仅含 PROXY 一个元素（上游 GLOBAL/DIRECT 等组已被过滤）。
 * Mini 的所有节点选择/恢复/切换/测速逻辑只针对 PROXY 组。
 * 调用方不应遍历 groups 做多组处理，直接取 `groups[0]` 即可。
 * 详见 project_memory.md 中「单一 PROXY 组」核心架构约定。
 */
export async function calcuProxies(): Promise<{
  groups: IProxyGroupItem[]
  records: Record<string, IProxyItem>
  proxies: IProxyItem[]
}> {
  // M2-14: 移除 try-catch，让错误传播给调用方决定降级策略
  const [proxyResponse, providerResponse] = await Promise.all([
    getProxies(),
    calcuProxyProviders(),
  ])

  const proxyRecord = proxyResponse?.proxies ?? {}
  const providerRecord = providerResponse ?? {}

  // provider name map
  const providerMap = Object.fromEntries(
    Object.entries(providerRecord).flatMap(([provider, item]) =>
      (item?.proxies ?? []).map((p: IProxyItem) => [
        p.name,
        { ...p, provider },
      ]),
    ),
  )

  // compatible with proxy-providers
  const generateItem = (name: string) => {
    if (proxyRecord[name]) return proxyRecord[name]
    if (providerMap[name]) return providerMap[name]
    return {
      name,
      type: 'unknown',
      udp: false,
      xudp: false,
      tfo: false,
      mptcp: false,
      smux: false,
      history: [],
    }
  }

  // Mini 单组架构：只构造 PROXY 组
  const proxyGroup = proxyRecord['PROXY']
  const groups: IProxyGroupItem[] = proxyGroup
    ? [
        {
          ...proxyGroup,
          all: (proxyGroup.all ?? [])
            .map((item) => generateItem(item))
            .filter((item) => item?.name && !isDummyNode(item.name)),
        },
      ]
    : []

  // 非组节点（DIRECT/REJECT 及无子节点的叶子节点），供渲染使用
  const proxies = Object.values(proxyRecord).filter(
    (p) => !p?.all?.length && p?.name && !isDummyNode(p.name),
  )

  return {
    groups,
    records: proxyRecord as Record<string, IProxyItem>,
    proxies: (proxies as IProxyItem[]) ?? [],
  }
}

export async function calcuProxyProviders() {
  const providers = await getProxyProviders()
  return Object.fromEntries(
    Object.entries(providers?.providers ?? {})
      .sort()
      .filter(
        ([_, item]) =>
          item?.vehicleType === 'HTTP' || item?.vehicleType === 'File',
      )
      .map(([name, item]) => {
        const provider = (item ?? {}) as Record<string, unknown>
        const proxyList = provider.proxies as
          | Array<Record<string, unknown>>
          | undefined
        const proxies = proxyList
          ? proxyList
              .map((p) => ({ ...p, provider: name }) as IProxyItem)
              .filter((p) => p.name && !isDummyNode(p.name))
          : []
        return [
          name,
          {
            ...provider,
            proxies,
          },
        ]
      }),
  )
}

export async function getClashLogs() {
  const regex = /time="(.+?)"\s+level=(.+?)\s+msg="(.+?)"/
  const newRegex = /(.+?)\s+(.+?)\s+(.+)/
  const logs = await withIpcTimeout(
    invoke<string[]>('get_clash_logs'),
    10_000,
    'getClashLogs',
  )

  return (logs ?? []).reduce<ILogItem[]>((acc, log) => {
    const result = log.match(regex)
    if (result) {
      const [_, _time, type, payload] = result
      const time = dayjs(_time).format('MM-DD HH:mm:ss')
      acc.push({ time, type, payload })
      return acc
    }

    const result2 = log.match(newRegex)
    if (result2) {
      const [_, time, type, payload] = result2
      acc.push({ time, type, payload })
    }
    return acc
  }, [])
}

export async function getVergeConfig() {
  return withIpcTimeout(
    invoke<IVergeConfig>('get_verge_config'),
    10_000,
    'getVergeConfig',
  )
}

export async function patchVergeConfig(payload: IVergeConfig) {
  return withIpcTimeout(
    invoke<void>('patch_verge_config', { payload }),
    30_000,
    'patchVergeConfig',
  )
}

export async function getSystemProxy() {
  return withIpcTimeout(
    invoke<{
      enable: boolean
      server: string
      bypass: string
    }>('get_sys_proxy'),
    10_000,
    'getSystemProxy',
  )
}

export async function getAutoProxy() {
  try {
    debugLog('[API] 开始调用 get_auto_proxy')
    const result = await withIpcTimeout(
      invoke<{
        enable: boolean
        url: string
      }>('get_auto_proxy'),
      10_000,
      'getAutoProxy',
    )
    debugLog('[API] get_auto_proxy 调用成功:', result)
    return result
  } catch (error) {
    console.error('[API] get_auto_proxy 调用失败:', error)
    return {
      enable: false,
      url: '',
    }
  }
}

export async function restartCore() {
  return withIpcTimeout(invoke<void>('restart_core'), 60_000, 'restartCore')
}

export async function openCoreDir() {
  return withIpcTimeout(
    invoke<void>('open_core_dir'),
    30_000,
    'openCoreDir',
  ).catch((err) => showNotice.error(err))
}

export async function openLogsDir() {
  return withIpcTimeout(
    invoke<void>('open_logs_dir'),
    30_000,
    'openLogsDir',
  ).catch((err) => showNotice.error(err))
}

export async function openDevTools() {
  return withIpcTimeout(invoke('open_devtools'), 10_000, 'openDevTools')
}

export async function downloadIconCache(url: string, name: string) {
  return withIpcTimeout(
    invoke<string>('download_icon_cache', { url, name }),
    30_000,
    'downloadIconCache',
  )
}

// 获取当前运行模式
export const getRunningMode = async () => {
  return withIpcTimeout(
    invoke<string>('get_running_mode'),
    10_000,
    'getRunningMode',
  )
}

// 获取应用运行时间
export const getAppUptime = async () => {
  return withIpcTimeout(
    invoke<number>('get_app_uptime'),
    10_000,
    'getAppUptime',
  )
}

// 安装系统服务
export const installService = async () => {
  return withIpcTimeout(
    invoke<void>('install_service'),
    60_000,
    'installService',
  )
}

// 卸载系统服务
export const uninstallService = async () => {
  return withIpcTimeout(
    invoke<void>('uninstall_service'),
    60_000,
    'uninstallService',
  )
}

// 重装系统服务

// 修复系统服务

// 系统服务是否可用
export const isServiceAvailable = async () => {
  try {
    return await withIpcTimeout(
      invoke<boolean>('is_service_available'),
      10_000,
      'isServiceAvailable',
    )
  } catch (error) {
    console.error('Service check failed:', error)
    return false
  }
}

export const isAdmin = async () => {
  try {
    return await withIpcTimeout(
      invoke<boolean>('app_is_admin'),
      10_000,
      'isAdmin',
    )
  } catch (error) {
    console.error('检查管理员权限失败:', error)
    return false
  }
}

export const isPortInUse = async (port: number) => {
  try {
    return await withIpcTimeout(
      invoke<boolean>('is_port_in_use', { port }),
      10_000,
      'isPortInUse',
    )
  } catch (error) {
    console.error('检查端口使用状态失败:', error)
    return false
  }
}

export async function getProxyAddr(name: string, provider?: string) {
  return withIpcTimeout(
    invoke<[string, number] | null>('get_proxy_addr', { name, provider }),
    10_000,
    'getProxyAddr',
  )
}
