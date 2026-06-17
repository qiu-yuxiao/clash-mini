export interface ITrafficItem {
  up: number
  down: number
  up_rate?: number
  down_rate?: number
  last_updated?: number
  upTotal?: number
  downTotal?: number
}

export interface IFormattedTrafficData {
  up_rate_formatted: string
  down_rate_formatted: string
  total_up_formatted: string
  total_down_formatted: string
  is_fresh: boolean
}

export interface IFormattedMemoryData {
  inuse_formatted: string
  oslimit_formatted: string
  usage_percent: number
  is_fresh: boolean
}

// 增强的类型安全接口定义，确保所有字段必需
export interface ISystemMonitorOverview {
  traffic: {
    raw: {
      up: number
      down: number
      up_rate: number
      down_rate: number
    }
    formatted: {
      up_rate: string
      down_rate: string
      total_up: string
      total_down: string
    }
    is_fresh: boolean
  }
  memory: {
    raw: {
      inuse: number
      oslimit: number
      usage_percent: number
    }
    formatted: {
      inuse: string
      oslimit: string
      usage_percent: number
    }
    is_fresh: boolean
  }
  overall_status: 'active' | 'inactive' | 'error' | 'unknown' | 'healthy'
}

// 类型安全的数据验证器
export interface ISystemMonitorOverviewValidator {
  validate(data: any): data is ISystemMonitorOverview
  sanitize(data: any): ISystemMonitorOverview
}


export interface ITrafficDataPoint {
  up: number
  down: number
  timestamp: number
  name: string
}

export interface ISamplingConfig {
  rawDataMinutes: number
  compressedDataMinutes: number
  compressionRatio: number
}

export interface ISamplerStats {
  rawBufferSize: number
  compressedBufferSize: number
  compressionQueueSize: number
  totalMemoryPoints: number
}

export interface ITrafficWorkerInitMessage {
  type: 'init'
  config: ISamplingConfig & {
    snapshotIntervalMs: number
    defaultRangeMinutes: number
  }
}

export interface ITrafficWorkerAppendMessage {
  type: 'append'
  payload: {
    up: number
    down: number
    timestamp?: number
  }
}

export interface ITrafficWorkerClearMessage {
  type: 'clear'
}

export interface ITrafficWorkerSetRangeMessage {
  type: 'setRange'
  minutes: number
}

export interface ITrafficWorkerRequestSnapshotMessage {
  type: 'requestSnapshot'
}

export type TrafficWorkerRequestMessage =
  | ITrafficWorkerInitMessage
  | ITrafficWorkerAppendMessage
  | ITrafficWorkerClearMessage
  | ITrafficWorkerSetRangeMessage
  | ITrafficWorkerRequestSnapshotMessage

export interface ITrafficWorkerSnapshotMessage {
  type: 'snapshot'
  dataPoints: ITrafficDataPoint[]
  availableDataPoints: ITrafficDataPoint[]
  samplerStats: ISamplerStats
  rangeMinutes: number
  lastTimestamp?: number
  reason:
    | 'init'
    | 'interval'
    | 'range-change'
    | 'request'
    | 'append-throttle'
    | 'clear'
}

export interface ITrafficWorkerLogMessage {
  type: 'log'
  message: string
}

export type TrafficWorkerResponseMessage =
  | ITrafficWorkerSnapshotMessage
  | ITrafficWorkerLogMessage
