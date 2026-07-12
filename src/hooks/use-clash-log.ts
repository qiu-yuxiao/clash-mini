import { useLocalStorage } from 'foxact/use-local-storage'

import type { IClashLog } from '@/types/clash'

const defaultClashLog: IClashLog = {
  enable: true,
  logLevel: 'info',
  logFilter: 'all',
  logOrder: 'asc',
}

// L-34: 安全的 JSON 解析，防止 localStorage 中损坏的数据导致错误
const safeJsonParse = (str: string): IClashLog => {
  try {
    const parsed = JSON.parse(str)
    // 简单的类型校验，确保返回值符合预期结构
    if (parsed && typeof parsed === 'object') {
      return {
        ...defaultClashLog,
        ...parsed,
      }
    }
    return defaultClashLog
  } catch {
    console.warn('[useClashLog] 无法解析 localStorage 中的数据，使用默认值')
    return defaultClashLog
  }
}

export const useClashLog = () =>
  useLocalStorage<IClashLog>('clash-log', defaultClashLog, {
    serializer: JSON.stringify,
    deserializer: safeJsonParse,
  })
