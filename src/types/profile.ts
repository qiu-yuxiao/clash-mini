export interface IProfileItem {
  uid: string
  type?: 'local' | 'remote' | 'merge' | 'script'
  name?: string
  desc?: string
  file?: string
  url?: string
  updated?: number
  selected?: {
    name?: string
    now?: string
  }[]
  extra?: {
    upload: number
    download: number
    total: number
    expire: number
  }
  option?: IProfileOption
  home?: string
}

export interface IProfileOption {
  user_agent?: string
  with_proxy?: boolean
  self_proxy?: boolean
  update_interval?: number
  timeout_seconds?: number
  danger_accept_invalid_certs?: boolean
  allow_auto_update?: boolean
  merge?: string
  script?: string
  rules?: string
  proxies?: string
  groups?: string
}

export interface IProfilesConfig {
  current?: string
  items?: IProfileItem[]
}

export interface IVergeTestItem {
  uid: string
  name?: string
  icon?: string
  url: string
}
export interface IAddress {
  V4?: {
    ip: string
    broadcast?: string
    netmask?: string
  }
  V6?: {
    ip: string
    broadcast?: string
    netmask?: string
  }
}
export interface INetworkInterface {
  name: string
  addr: IAddress[]
  mac_addr?: string
  index: number
}

export interface ISeqProfileConfig {
  prepend: []
  append: []
  delete: []
}

