export interface IConnectionsItem {
  id: string
  metadata: {
    network: string
    type: string
    host: string
    sourceIP: string
    sourcePort: string
    destinationPort: string
    destinationIP?: string
    remoteDestination?: string
    process?: string
    processPath?: string
  }
  upload: number
  download: number
  start: string
  chains: string[]
  rule: string
  rulePayload: string
  curUpload?: number // upload speed, calculate at runtime
  curDownload?: number // download speed, calculate at runtime
}

export interface IConnections {
  downloadTotal: number
  uploadTotal: number
  connections: IConnectionsItem[]
}

export interface IConnectionSetting {
  layout: 'table' | 'list'
}

