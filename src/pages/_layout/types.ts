/** Tauri 更新对象 */
export interface GithubAsset {
  name: string
  browser_download_url: string
}

/** GitHub Release 信息 */
export interface GithubRelease {
  tag_name: string
  assets: GithubAsset[]
}

/** 内核升级进度事件 payload */
export interface CoreUpgradeProgressPayload {
  status: string
  progress?: number
  message?: string
}
