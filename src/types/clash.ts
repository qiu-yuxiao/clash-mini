export interface IConfigData {
  port: number
  mode: string
  ipv6: boolean
  'socket-port': number
  allowLan: boolean
  'log-level': string
  'mixed-port': number
  'redir-port': number
  'socks-port': number
  'tproxy-port': number
  'external-controller': string
  'external-controller-cors': {
    'allow-private-network': boolean
    'allow-origins': string[]
  }
  secret: string
  'unified-delay': boolean
  tun: {
    stack: string
    device: string
    'auto-route': boolean
    'auto-redirect'?: boolean
    'auto-detect-interface': boolean
    'dns-hijack': string[]
    'route-exclude-address'?: string[]
    'strict-route': boolean
    mtu: number
  }
  dns?: {
    enable?: boolean
    listen?: string
    'enhanced-mode'?: 'fake-ip' | 'redir-host'
    'fake-ip-range'?: string
    'fake-ip-filter'?: string[]
    'fake-ip-filter-mode'?: 'blacklist' | 'whitelist'
    'prefer-h3'?: boolean
    'respect-rules'?: boolean
    nameserver?: string[]
    fallback?: string[]
    'default-nameserver'?: string[]
    'proxy-server-nameserver'?: string[]
    'direct-nameserver'?: string[]
    'direct-nameserver-follow-policy'?: boolean
    'nameserver-policy'?: Record<string, unknown>
    'use-hosts'?: boolean
    'use-system-hosts'?: boolean
    'fallback-filter'?: {
      geoip?: boolean
      'geoip-code'?: string
      ipcidr?: string[]
      domain?: string[]
    }
  }
  tunnels?: {
    network: string[]
    address: string
    target: string
    proxy?: string
  }[]
  'proxy-groups'?: IProxyGroupItem[]
}

export interface IProxyItem {
  name: string
  type: string
  udp: boolean
  xudp: boolean
  tfo: boolean
  mptcp: boolean
  smux: boolean
  history: {
    time: string
    delay: number
  }[]
  all?: string[]
  now?: string
  /**
   * 内核最近一次拨测判定的存活状态（mihomo `/proxies` 原生字段）。
   * 拨测失败只置 false、不追加 history，故必须用它来兜住 history 里的陈旧成功值。
   */
  alive?: boolean
  hidden?: boolean
  icon?: string
  provider?: string // 记录是否来自provider
  fixed?: string // 记录固定(优先)的节点
}

export type IProxyGroupItem = Omit<IProxyItem, 'all'> & {
  all: IProxyItem[]
}

export interface IProxyProviderItem {
  name: string
  type: string
  proxies: IProxyItem[]
  updatedAt: string
  vehicleType: string
  subscriptionInfo?: {
    Upload: number
    Download: number
    Total: number
    Expire: number
  }
}

export interface IRuleProviderItem {
  name: string
  behavior: string
  format: string
  ruleCount: number
  type: string
  updatedAt: string
  vehicleType: string
}

export interface ILogItem {
  type: string
  time?: string
  payload: string
}

export type LogLevel = import('tauri-plugin-mihomo-api').LogLevel
export type LogFilter = 'all' | 'debug' | 'info' | 'warn' | 'err'
export type LogOrder = 'asc' | 'desc'

export interface IClashLog {
  enable: boolean
  logLevel: LogLevel
  logFilter: LogFilter
  logOrder: LogOrder
}

/**
 * Some interface for command
 */

export interface IClashInfo {
  // status: string;
  mixed_port?: number // clash mixed port
  socks_port?: number // clash socks port
  redir_port?: number // clash redir port
  tproxy_port?: number // clash tproxy port
  port?: number // clash http port
  server?: string // external-controller
  secret?: string
  ipv6?: boolean
  allow_lan?: boolean
}
