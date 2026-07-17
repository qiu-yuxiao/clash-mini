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
  testUrl?: string
  all?: string[]
  now?: string
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
}

export interface IProxyGroupConfig {
  name: string
  type: 'select' | 'url-test' | 'fallback' | 'load-balance' | 'relay'
  proxies?: string[]
  use?: string[]
  url?: string
  interval?: number
  lazy?: boolean
  timeout?: number
  'max-failed-times'?: number
  'disable-udp'?: boolean
  'interface-name': string
  'routing-mark'?: number
  'include-all'?: boolean
  'include-all-proxies'?: boolean
  'include-all-providers'?: boolean
  filter?: string
  'exclude-filter'?: string
  'exclude-type'?: string
  'expected-status'?: string
  hidden?: boolean
  icon?: string
}

export interface WsOptions {
  path?: string
  headers?: {
    [key: string]: string
  }
  'max-early-data'?: number
  'early-data-header-name'?: string
  'v2ray-http-upgrade'?: boolean
  'v2ray-http-upgrade-fast-open'?: boolean
}

export interface HttpOptions {
  method?: string
  path?: string[]
  headers?: {
    [key: string]: string[]
  }
}

export interface H2Options {
  path?: string
  host?: string
}

export interface GrpcOptions {
  'grpc-service-name'?: string
}

export interface RealityOptions {
  'public-key'?: string
  'short-id'?: string
}
export type ClientFingerprint =
  | 'chrome'
  | 'firefox'
  | 'safari'
  | 'iOS'
  | 'android'
  | 'edge'
  | '360'
  | 'qq'
  | 'random'
export type NetworkType = 'ws' | 'http' | 'h2' | 'grpc' | 'tcp'
export type CipherType =
  | 'none'
  | 'auto'
  | 'dummy'
  | 'aes-128-gcm'
  | 'aes-192-gcm'
  | 'aes-256-gcm'
  | 'lea-128-gcm'
  | 'lea-192-gcm'
  | 'lea-256-gcm'
  | 'aes-128-gcm-siv'
  | 'aes-256-gcm-siv'
  | '2022-blake3-aes-128-gcm'
  | '2022-blake3-aes-256-gcm'
  | 'aes-128-cfb'
  | 'aes-192-cfb'
  | 'aes-256-cfb'
  | 'aes-128-ctr'
  | 'aes-192-ctr'
  | 'aes-256-ctr'
  | 'chacha20'
  | 'chacha20-ietf'
  | 'chacha20-ietf-poly1305'
  | '2022-blake3-chacha20-poly1305'
  | 'rabbit128-poly1305'
  | 'xchacha20-ietf-poly1305'
  | 'xchacha20'
  | 'aegis-128l'
  | 'aegis-256'
  | 'aez-384'
  | 'deoxys-ii-256-128'
  | 'rc4-md5'
export type MieruTransport = 'TCP' | 'UDP'
export type MieruMultiplexing =
  | 'MULTIPLEXING_OFF'
  | 'MULTIPLEXING_LOW'
  | 'MULTIPLEXING_MIDDLE'
  | 'MULTIPLEXING_HIGH'
export type SudokuAeadMethod = 'chacha20-poly1305' | 'aes-128-gcm' | 'none'
export type SudokuTableType = 'prefer_ascii' | 'prefer_entropy'
export type SudokuHttpMaskMode = 'legacy' | 'stream' | 'poll' | 'auto'
export type SudokuHttpMaskStrategy = 'random' | 'post' | 'websocket'
// base
export interface IProxyBaseConfig {
  tfo?: boolean
  mptcp?: boolean
  'interface-name'?: string
  'routing-mark'?: number
  'ip-version'?: 'dual' | 'ipv4' | 'ipv6' | 'ipv4-prefer' | 'ipv6-prefer'
  'dialer-proxy'?: string
}
// direct
export interface IProxyDirectConfig extends IProxyBaseConfig {
  name: string
  type: 'direct'
}
// dns
export interface IProxyDnsConfig extends IProxyBaseConfig {
  name: string
  type: 'dns'
}
// http
export interface IProxyHttpConfig extends IProxyBaseConfig {
  name: string
  type: 'http'
  server?: string
  port?: number
  username?: string
  password?: string
  tls?: boolean
  sni?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  headers?: {
    [key: string]: string
  }
}
// socks5
export interface IProxySocks5Config extends IProxyBaseConfig {
  name: string
  type: 'socks5'
  server?: string
  port?: number
  username?: string
  password?: string
  tls?: boolean
  udp?: boolean
  'skip-cert-verify'?: boolean
  fingerprint?: string
}
// ssh
export interface IProxySshConfig extends IProxyBaseConfig {
  name: string
  type: 'ssh'
  server?: string
  port?: number
  username?: string
  password?: string
  'private-key'?: string
  'private-key-passphrase'?: string
  'host-key'?: string
  'host-key-algorithms'?: string
}
// trojan
export interface IProxyTrojanConfig extends IProxyBaseConfig {
  name: string
  type: 'trojan'
  server?: string
  port?: number
  password?: string
  alpn?: string[]
  sni?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  udp?: boolean
  network?: NetworkType
  'reality-opts'?: RealityOptions
  'grpc-opts'?: GrpcOptions
  'ws-opts'?: WsOptions
  'ss-opts'?: {
    enabled?: boolean
    method?: string
    password?: string
  }
  'client-fingerprint'?: ClientFingerprint
}
// anytls
export interface IProxyAnyTLSConfig extends IProxyBaseConfig {
  name: string
  type: 'anytls'
  server?: string
  port?: number
  password?: string
  alpn?: string[]
  sni?: string
  'client-fingerprint'?: ClientFingerprint
  'skip-cert-verify'?: boolean
  fingerprint?: string
  certificate?: string
  'private-key'?: string
  'ech-opts'?: {
    enable?: boolean
    config?: string
  }
  udp?: boolean
  'idle-session-check-interval'?: number
  'idle-session-timeout'?: number
  'min-idle-session'?: number
}
// tuic
export interface IProxyTuicConfig extends IProxyBaseConfig {
  name: string
  type: 'tuic'
  server?: string
  port?: number
  token?: string
  uuid?: string
  password?: string
  ip?: string
  'heartbeat-interval'?: number
  alpn?: string[]
  'reduce-rtt'?: boolean
  'request-timeout'?: number
  'udp-relay-mode'?: string
  'congestion-controller'?: string
  'disable-sni'?: boolean
  'max-udp-relay-packet-size'?: number
  'fast-open'?: boolean
  'max-open-streams'?: number
  cwnd?: number
  'skip-cert-verify'?: boolean
  fingerprint?: string
  ca?: string
  'ca-str'?: string
  'recv-window-conn'?: number
  'recv-window'?: number
  'disable-mtu-discovery'?: boolean
  'max-datagram-frame-size'?: number
  sni?: string
  'udp-over-stream'?: boolean
  'udp-over-stream-version'?: number
}
// mieru
export interface IProxyMieruConfig extends IProxyBaseConfig {
  name: string
  type: 'mieru'
  server?: string
  port?: number
  'port-range'?: string
  transport?: MieruTransport
  udp?: boolean
  username?: string
  password?: string
  multiplexing?: MieruMultiplexing
  'handshake-mode'?: string
}
// masque
export interface IProxyMasqueConfig extends IProxyBaseConfig {
  name: string
  type: 'masque'
  server?: string
  port?: number
  'private-key'?: string
  'public-key'?: string
  ip?: string
  ipv6?: string
  mtu?: number
  udp?: boolean
  'remote-dns-resolve'?: boolean
  dns?: string[]
}
// vless
export interface IProxyVlessConfig extends IProxyBaseConfig {
  name: string
  type: 'vless'
  server?: string
  port?: number
  uuid?: string
  flow?: string
  tls?: boolean
  alpn?: string[]
  udp?: boolean
  'packet-addr'?: boolean
  xudp?: boolean
  'packet-encoding'?: string
  network?: NetworkType
  'reality-opts'?: RealityOptions
  'http-opts'?: HttpOptions
  'h2-opts'?: H2Options
  'grpc-opts'?: GrpcOptions
  'ws-opts'?: WsOptions
  'ws-path'?: string
  'ws-headers'?: {
    [key: string]: string
  }
  'skip-cert-verify'?: boolean
  fingerprint?: string
  servername?: string
  'client-fingerprint'?: ClientFingerprint
  smux?: boolean
}
// vmess
export interface IProxyVmessConfig extends IProxyBaseConfig {
  name: string
  type: 'vmess'
  server?: string
  port?: number
  uuid?: string
  alterId?: number
  cipher?: CipherType
  udp?: boolean
  network?: NetworkType
  tls?: boolean
  alpn?: string[]
  'skip-cert-verify'?: boolean
  fingerprint?: string
  servername?: string
  'reality-opts'?: RealityOptions
  'http-opts'?: HttpOptions
  'h2-opts'?: H2Options
  'grpc-opts'?: GrpcOptions
  'ws-opts'?: WsOptions
  'packet-addr'?: boolean
  xudp?: boolean
  'packet-encoding'?: string
  'global-padding'?: boolean
  'authenticated-length'?: boolean
  'client-fingerprint'?: ClientFingerprint
  smux?: boolean
}
export interface WireGuardPeerOptions {
  server?: string
  port?: number
  'public-key'?: string
  'pre-shared-key'?: string
  reserved?: number[]
  'allowed-ips'?: string[]
}
// wireguard
export interface IProxyWireguardConfig
  extends IProxyBaseConfig,
    WireGuardPeerOptions {
  name: string
  type: 'wireguard'
  ip?: string
  ipv6?: string
  'private-key'?: string
  workers?: number
  mtu?: number
  udp?: boolean
  'persistent-keepalive'?: number
  peers?: WireGuardPeerOptions[]
  'remote-dns-resolve'?: boolean
  dns?: string[]
  'refresh-server-ip-interval'?: number
}
// hysteria
export interface IProxyHysteriaConfig extends IProxyBaseConfig {
  name: string
  type: 'hysteria'
  server?: string
  port?: number
  ports?: string
  protocol?: string
  'obfs-protocol'?: string
  up?: string
  'up-speed'?: number
  down?: string
  'down-speed'?: number
  auth?: string
  'auth-str'?: string
  obfs?: string
  sni?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  alpn?: string[]
  ca?: string
  'ca-str'?: string
  'recv-window-conn'?: number
  'recv-window'?: number
  'disable-mtu-discovery'?: boolean
  'fast-open'?: boolean
  'hop-interval'?: number
}
// hysteria2
export interface IProxyHysteria2Config extends IProxyBaseConfig {
  name: string
  type: 'hysteria2'
  server?: string
  port?: number
  ports?: string
  'hop-interval'?: number
  protocol?: string
  'obfs-protocol'?: string
  up?: string
  down?: string
  password?: string
  obfs?: string
  'obfs-password'?: string
  sni?: string
  'skip-cert-verify'?: boolean
  fingerprint?: string
  alpn?: string[]
  ca?: string
  'ca-str'?: string
  cwnd?: number
  'udp-mtu'?: number
}
// shadowsocks
export interface IProxyShadowsocksConfig extends IProxyBaseConfig {
  name: string
  type: 'ss'
  server?: string
  port?: number
  password?: string
  cipher?: CipherType
  udp?: boolean
  plugin?: 'obfs' | 'v2ray-plugin' | 'shadow-tls' | 'restls'
  'plugin-opts'?: {
    mode?: string
    host?: string
    password?: string
    path?: string
    tls?: string
    fingerprint?: string
    headers?: {
      [key: string]: string
    }
    'skip-cert-verify'?: boolean
    version?: number
    mux?: boolean
    'v2ray-http-upgrade'?: boolean
    'v2ray-http-upgrade-fast-open'?: boolean
    'version-hint'?: string
    'restls-script'?: string
  }
  'udp-over-tcp'?: boolean
  'udp-over-tcp-version'?: number
  'client-fingerprint'?: ClientFingerprint
  smux?: boolean
}
// sudoku
export interface IProxySudokuConfig extends IProxyBaseConfig {
  name: string
  type: 'sudoku'
  server?: string
  port?: number
  key?: string
  'aead-method'?: SudokuAeadMethod
  'padding-min'?: number
  'padding-max'?: number
  'table-type'?: SudokuTableType
  'enable-pure-downlink'?: boolean
  'http-mask'?: boolean
  'http-mask-mode'?: SudokuHttpMaskMode
  'http-mask-tls'?: boolean
  'http-mask-host'?: string
  'http-mask-strategy'?: SudokuHttpMaskStrategy
  'custom-table'?: string
  'custom-tables'?: string[]
}
// shadowsocksR
export interface IProxyshadowsocksRConfig extends IProxyBaseConfig {
  name: string
  type: 'ssr'
  server?: string
  port?: number
  password?: string
  cipher?: CipherType
  obfs?: string
  'obfs-param'?: string
  protocol?: string
  'protocol-param'?: string
  udp?: boolean
}
// sing-mux
export interface IProxySmuxConfig {
  smux?: {
    enabled?: boolean
    protocol?: 'smux' | 'yamux' | 'h2mux'
    'max-connections'?: number
    'min-streams'?: number
    'max-streams'?: number
    padding?: boolean
    statistic?: boolean
    'only-tcp'?: boolean
    'brutal-opts'?: {
      enabled?: boolean
      up?: string
      down?: string
    }
  }
}
// snell
export interface IProxySnellConfig extends IProxyBaseConfig {
  name: string
  type: 'snell'
  server?: string
  port?: number
  psk?: string
  udp?: boolean
  version?: number
}
export type IProxyConfig = (
  | IProxyDirectConfig
  | IProxyDnsConfig
  | IProxyHttpConfig
  | IProxySocks5Config
  | IProxySshConfig
  | IProxyTrojanConfig
  | IProxyAnyTLSConfig
  | IProxyTuicConfig
  | IProxyMieruConfig
  | IProxyMasqueConfig
  | IProxyVlessConfig
  | IProxyVmessConfig
  | IProxyWireguardConfig
  | IProxyHysteriaConfig
  | IProxyHysteria2Config
  | IProxyShadowsocksConfig
  | IProxySudokuConfig
  | IProxyshadowsocksRConfig
  | IProxySnellConfig
) & {
  smux?: boolean | IProxySmuxConfig['smux']
}
