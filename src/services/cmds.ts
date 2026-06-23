import { invoke } from '@tauri-apps/api/core'
import dayjs from 'dayjs'
import yaml from 'js-yaml'

import { showNotice } from '@/services/notice-service'
import type { IConfigData, IProxyItem, IProxyGroupItem, ILogItem, IClashInfo } from '@/types/clash'
import type { IProfileItem, IProfilesConfig, IProfileOption } from '@/types/profile'
import type { IVergeConfig, ValidationOutcome } from '@/types/verge'
import { debugLog } from '@/utils/debug'
import { isDummyNode } from '@/utils/node'
import {
  getProxies,
  getProxyProviders,
  delayProxyByName,
} from 'tauri-plugin-mihomo-api'

export async function getProfiles() {
  return invoke<IProfilesConfig>('get_profiles')
}

export async function enhanceProfiles() {
  try {
    const config = await getProfiles()
    const activeUid = config.current
    if (activeUid) {
      const rawYaml = await readProfileFile(activeUid)
      if (rawYaml) {
        const doc = yaml.load(rawYaml) as Record<string, unknown>
        if (doc && typeof doc === 'object' && !Array.isArray(doc)) {
          let modified = false

          // 1. 提取所有原始 proxies 名字
          const proxies = doc.proxies || []
          const proxyNames = Array.isArray(proxies)
            ? proxies.map((p: any) => p && p.name).filter(Boolean)
            : []

          // 2. 提取所有的 proxy-providers 名字
          const providers = doc['proxy-providers'] || {}
          const providerNames =
            providers && typeof providers === 'object'
              ? Object.keys(providers)
              : []

          // 3. 判断是否需要执行过滤 (组数大于 1，或者唯一组的名字不为 PROXY，或者规则非空)
          const groups = doc['proxy-groups'] || []
          const hasMultipleGroups =
            Array.isArray(groups) &&
            (groups.length > 1 ||
              (groups.length === 1 && groups[0].name !== 'PROXY'))
          const hasRules = Array.isArray(doc.rules) && doc.rules.length > 0

          if (hasMultipleGroups || hasRules) {
            // 构造唯一的 PROXY 组
            const newGroup: any = {
              name: 'PROXY',
              type: 'select',
            }
            if (proxyNames.length > 0) {
              newGroup.proxies = proxyNames
            }
            if (providerNames.length > 0) {
              newGroup.use = providerNames
            }
            if (proxyNames.length === 0 && providerNames.length === 0) {
              newGroup.proxies = ['DIRECT']
            }

            // 只保留唯一的 PROXY 组
            doc['proxy-groups'] = [newGroup]

            // 摒弃并清空机场订阅自带的规则列表，完全托管给 Clash Mini 自身的智能路由
            doc.rules = []

            modified = true
          }

          if (modified) {
            await saveProfileFile(activeUid, yaml.dump(doc))
            debugLog(
              `[ProfileTransformer] Successfully cleaned up profile ${activeUid} to single PROXY group and empty rules`,
            )
          }
        }
      }
    }
  } catch (err) {
    console.error('[ProfileTransformer] Failed to clean up profile:', err)
  }

  return (
    (await invoke<ValidationOutcome>('enhance_profiles')).status === 'valid'
  )
}

export async function patchProfilesConfig(profiles: IProfilesConfig) {
  return (
    (await invoke<ValidationOutcome>('patch_profiles_config', { profiles }))
      .status === 'valid'
  )
}

export async function viewProfile(index: string) {
  return invoke<void>('view_profile', { index })
}

export async function readProfileFile(index: string) {
  return invoke<string>('read_profile_file', { index })
}

export async function saveProfileFile(index: string, fileData: string) {
  return (
    (
      await invoke<ValidationOutcome>('save_profile_file', {
        index,
        fileData,
      })
    ).status === 'valid'
  )
}

export async function importProfile(url: string, option?: IProfileOption) {
  return invoke<void>('import_profile', {
    url,
    option: option || { with_proxy: true },
  })
}

export async function updateProfile(index: string, option?: IProfileOption) {
  return invoke<void>('update_profile', { index, option })
}

export async function deleteProfile(index: string) {
  return invoke<void>('delete_profile', { index })
}

export async function patchProfile(
  index: string,
  profile: Partial<IProfileItem>,
) {
  return invoke<void>('patch_profile', { index, profile })
}

export async function getClashInfo() {
  return invoke<IClashInfo | null>('get_clash_info')
}

// Get runtime config which controlled by verge
export async function getRuntimeConfig() {
  return invoke<IConfigData | null>('get_runtime_config')
}

export async function updateProxyChainConfigInRuntime(proxyChainConfig: any) {
  return invoke<void>('update_proxy_chain_config_in_runtime', {
    proxyChainConfig,
  })
}

export async function patchClashConfig(payload: Partial<IConfigData>) {
  return invoke<void>('patch_clash_config', { payload })
}

export async function patchClashMode(payload: string) {
  return invoke<void>('patch_clash_mode', { payload })
}

export async function syncTrayProxySelection() {
  return invoke<void>('sync_tray_proxy_selection')
}

export async function calcuProxies(): Promise<{
  global: IProxyGroupItem
  direct: IProxyItem
  groups: IProxyGroupItem[]
  records: Record<string, IProxyItem>
  proxies: IProxyItem[]
}> {
  const [proxyResponse, providerResponse] = await Promise.all([
    getProxies(),
    calcuProxyProviders(),
  ])

  const proxyRecord = proxyResponse?.proxies ?? {}
  const providerRecord = providerResponse ?? {}

  // provider name map
  const providerMap = Object.fromEntries(
    Object.entries(providerRecord).flatMap(([provider, item]) =>
      (item?.proxies ?? []).map((p: any) => [p.name, { ...p, provider }]),
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

  const { GLOBAL: global, DIRECT: direct, REJECT: reject } = proxyRecord

  let groups: IProxyGroupItem[] = Object.values(proxyRecord).reduce<
    IProxyGroupItem[]
  >((acc, each) => {
    if (each?.name !== 'GLOBAL' && each?.all) {
      acc.push({
        ...each,
        all: (each.all ?? [])
          .map((item) => generateItem(item))
          .filter((item) => item?.name && !isDummyNode(item.name)),
      })
    }

    return acc
  }, [])

  if (global?.all) {
    const globalGroups: IProxyGroupItem[] = global.all.reduce<
      IProxyGroupItem[]
    >((acc, name) => {
      if (proxyRecord[name]?.all) {
        acc.push({
          ...proxyRecord[name],
          all: (proxyRecord[name].all ?? [])
            .map((item) => generateItem(item))
            .filter((item) => item?.name && !isDummyNode(item.name)),
        })
      }
      return acc
    }, [])

    const globalNames = new Set(globalGroups.map((each) => each.name))
    groups = groups
      .filter((group) => {
        return !globalNames.has(group.name)
      })
      .concat(globalGroups)
  }

  const proxies = [direct, reject].filter(Boolean).concat(
    Object.values(proxyRecord).filter(
      (p) =>
        !p?.all?.length &&
        p?.name !== 'DIRECT' &&
        p?.name !== 'REJECT' &&
        p?.name &&
        !isDummyNode(p.name),
    ),
  )

  const _global = {
    ...global,
    all: (global?.all?.map((item) => generateItem(item)) || []).filter(
      (item) => item?.name && !isDummyNode(item.name),
    ),
  }

  return {
    global: _global as IProxyGroupItem,
    direct: direct as IProxyItem,
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
        const proxyList = provider.proxies as Array<Record<string, unknown>> | undefined
        const proxies = proxyList
          ? proxyList
              .map((p) => ({ ...p, provider: name }) as { name: string; provider: string })
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
  const logs = await invoke<string[]>('get_clash_logs')

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
  return invoke<IVergeConfig>('get_verge_config')
}

export async function patchVergeConfig(payload: IVergeConfig) {
  return invoke<void>('patch_verge_config', { payload })
}

export async function getSystemProxy() {
  return invoke<{
    enable: boolean
    server: string
    bypass: string
  }>('get_sys_proxy')
}

export async function getAutoProxy() {
  try {
    debugLog('[API] 开始调用 get_auto_proxy')
    const result = await invoke<{
      enable: boolean
      url: string
    }>('get_auto_proxy')
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
  return invoke<void>('restart_core')
}

export async function openCoreDir() {
  return invoke<void>('open_core_dir').catch((err) => showNotice.error(err))
}

export async function openLogsDir() {
  return invoke<void>('open_logs_dir').catch((err) => showNotice.error(err))
}

export async function cmdGetProxyDelay(
  name: string,
  timeout: number,
  url?: string,
) {
  // 确保URL不为空
  const testUrl = url || 'http://cp.cloudflare.com/generate_204'

  try {
    // 调用 tauri-plugin-mihomo 提供的正常延迟测试函数
    const result = await delayProxyByName(name, testUrl, timeout)

    // 验证返回结果中是否有delay字段，并且值是一个有效的数字
    if (result && typeof result.delay === 'number') {
      return result
    } else {
      // 返回一个有效的结果对象，但标记为超时
      return { delay: 1e6 }
    }
  } catch {
    // 返回一个有效的结果对象，但标记为错误
    return { delay: 1e6 }
  }
}

export async function openDevTools() {
  return invoke('open_devtools')
}

export async function downloadIconCache(url: string, name: string) {
  return invoke<string>('download_icon_cache', { url, name })
}

// 获取当前运行模式
export const getRunningMode = async () => {
  return invoke<string>('get_running_mode')
}

// 获取应用运行时间
export const getAppUptime = async () => {
  return invoke<number>('get_app_uptime')
}

// 安装系统服务
export const installService = async () => {
  return invoke<void>('install_service')
}

// 卸载系统服务
export const uninstallService = async () => {
  return invoke<void>('uninstall_service')
}

// 重装系统服务

// 修复系统服务

// 系统服务是否可用
export const isServiceAvailable = async () => {
  try {
    return await invoke<boolean>('is_service_available')
  } catch (error) {
    console.error('Service check failed:', error)
    return false
  }
}

export const isAdmin = async () => {
  try {
    return await invoke<boolean>('app_is_admin')
  } catch (error) {
    console.error('检查管理员权限失败:', error)
    return false
  }
}

export const isPortInUse = async (port: number) => {
  try {
    return await invoke<boolean>('is_port_in_use', { port })
  } catch (error) {
    console.error('检查端口使用状态失败:', error)
    return false
  }
}

export async function getProxyAddr(name: string, provider?: string) {
  return invoke<[string, number] | null>('get_proxy_addr', { name, provider })
}
