import { Context, createContext, use } from 'react'

import type { IProxyItem, IProxyGroupItem } from '@/types/clash'
import type { IConnectionsItem } from '@/types/connection'
import type { BaseConfig, ProxyProvider } from 'tauri-plugin-mihomo-api'

export interface SystemProxyInfo {
  enable: boolean
  server: string
  bypass: string
}

export interface ConnectionWithSpeed extends IConnectionsItem {
  curUpload: number
  curDownload: number
}

export interface ConnectionSpeedData {
  id: string
  upload: number
  download: number
  timestamp: number
}

export interface ProxiesContextType {
  proxies:
    | {
        groups: IProxyGroupItem[]
        records: Record<string, IProxyItem>
        proxies: IProxyItem[]
      }
    | undefined
  proxyProviders: Record<string, ProxyProvider | undefined>
  isProxiesPending: boolean
}

export interface ClashConfigContextType {
  clashConfig: BaseConfig | undefined
  isClashConfigPending: boolean
}

export interface SystemContextType {
  sysproxy: SystemProxyInfo | undefined
  runningMode?: string
  systemProxyAddress: string
  isSettingsOpen: boolean
  setIsSettingsOpen: (open: boolean) => void
}

export interface RefreshersContextType {
  refreshProxy: (options?: { forceFull?: boolean }) => Promise<unknown>
  refreshClashConfig: () => Promise<unknown>
  refreshSysproxy: () => Promise<unknown>
  refreshProxyProviders: () => Promise<unknown>
  refreshAll: () => Promise<unknown>
}

export const ProxiesContext = createContext<ProxiesContextType | null>(null)
export const ClashConfigContext = createContext<ClashConfigContextType | null>(
  null,
)
export const SystemContext = createContext<SystemContextType | null>(null)
export const RefreshersContext = createContext<RefreshersContextType | null>(
  null,
)

const useCtx = <T>(ctx: Context<T | null>, hookName: string): T => {
  const v = use(ctx)
  if (!v) throw new Error(`${hookName} must be used within AppDataProvider`)
  return v
}

export const useProxiesData = () => {
  const { proxies, proxyProviders, isProxiesPending } = useCtx(
    ProxiesContext,
    'useProxiesData',
  )

  return {
    proxies,
    proxyProviders: proxyProviders as Record<string, ProxyProvider>,
    isProxiesPending,
  }
}

export const useClashConfigData = (): ClashConfigContextType =>
  useCtx(ClashConfigContext, 'useClashConfigData')

export const useSystemData = (): SystemContextType =>
  useCtx(SystemContext, 'useSystemData')

export const useAppRefreshers = (): RefreshersContextType =>
  useCtx(RefreshersContext, 'useAppRefreshers')
