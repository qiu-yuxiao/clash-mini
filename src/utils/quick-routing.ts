import yaml from 'js-yaml'

import {
  readProfileFile,
  saveProfileFile,
  enhanceProfiles,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

let mergeFileLock = Promise.resolve()

async function withMergeFileLock<T>(fn: () => Promise<T>): Promise<T> {
  const result = mergeFileLock.then(() => fn())
  mergeFileLock = result.then(
    () => {},
    () => {},
  )
  return result
}

// 核心功能：添加快捷分流规则到全局 Merge 的 prepend-rules 中 (置顶生效)
export const addQuickRoutingRule = async (
  type: 'process' | 'domain',
  value: string,
  target: 'DIRECT' | 'PROXY' | 'REJECT',
) => {
  if (!value) return

  try {
    // 1. 获取主代理组名称 (如果是代理，则需要目标代理组名；如果是封锁，使用 REJECT；如果是直连，使用 DIRECT)
    let proxyGroup = 'DIRECT'
    if (target === 'PROXY') {
      proxyGroup = 'PROXY'
    } else if (target === 'REJECT') {
      proxyGroup = 'REJECT'
    }

    // 2. 构造 Clash 规则字符串
    let newRule = ''
    let domainSuffix = value

    if (type === 'process') {
      newRule = `PROCESS-NAME,${value},${proxyGroup}`
    } else {
      // 提取域名后缀的简化方法：如果是 IP，直接用 IP 规则；如果是域名，使用 DOMAIN-SUFFIX
      const isIp = /^[0-9.:]+$/.test(value)
      if (isIp) {
        newRule = `IP-CIDR,${value}/32,${proxyGroup}`
      } else {
        // 自动提取二级或三级域名后缀
        const parts = value.split('.')
        if (parts.length >= 2) {
          const lastPart = parts[parts.length - 1]
          const secondLastPart = parts[parts.length - 2]
          const isDoubleTld =
            ['com', 'org', 'net', 'gov', 'edu', 'co'].includes(
              secondLastPart,
            ) && lastPart.length <= 3

          if (parts.length >= 3 && isDoubleTld) {
            domainSuffix = parts.slice(-3).join('.')
          } else {
            domainSuffix = parts.slice(-2).join('.')
          }
        }
        newRule = `DOMAIN-SUFFIX,${domainSuffix},${proxyGroup}`
      }
    }

    await withMergeFileLock(async () => {
      // 3. 读取全局 Merge 配置文件
      let mergeYaml: string
      try {
        mergeYaml = await readProfileFile('Merge')
      } catch (readErr) {
        console.warn(
          'Global Merge file not found or failed to read, initializing empty merge:',
          readErr,
        )
        mergeYaml = '{}'
      }
      const mergeObj = (yaml.load(mergeYaml) || {}) as Record<string, any>

      // 4. 确保 prepend-rules 数组存在
      mergeObj['prepend-rules'] = mergeObj['prepend-rules'] || []

      // 5. 过滤掉已有的相同属性规则（去重并置顶）
      const matchPrefix =
        type === 'process'
          ? `PROCESS-NAME,${value},`
          : `DOMAIN-SUFFIX,${domainSuffix},`

      const filteredRules = (mergeObj['prepend-rules'] as string[]).filter(
        (rule) => !rule.startsWith(matchPrefix) && rule !== newRule,
      )

      // 置顶写入规则
      mergeObj['prepend-rules'] = [newRule, ...filteredRules]

      // 6. 保存
      await saveProfileFile('Merge', yaml.dump(mergeObj))
    })
    await enhanceProfiles()

    showNotice.success(
      'profiles.page.feedback.notifications.profileSwitched',
      `手动路径控制规则已置顶生效: ${newRule}`,
      2500,
    )
  } catch (e: any) {
    console.error('Failed to add quick routing rule:', e)
    showNotice.error(e.message || String(e))
  }
}

// 把用户输入的一串文本（多个网址/域名，支持换行、逗号、分号分隔）解析成
// 一条条 Clash 规则（DOMAIN-SUFFIX 或 IP-CIDR），全部以 PROXY 置顶写入 Merge 的 prepend-rules。
// 返回实际新增的规则条数。
export const addQuickRoutingRules = async (
  rawText: string,
): Promise<number> => {
  const lines = (rawText || '')
    .split(/[\n,;]+/)
    .map((l) => l.trim())
    .filter(Boolean)

  if (lines.length === 0) return 0

  try {
    const rules = lines
      .map((line) => buildProxyRule(line))
      .filter((rule): rule is string => !!rule)

    if (rules.length === 0) return 0

    let addedCount = 0
    await withMergeFileLock(async () => {
      let mergeYaml: string
      try {
        mergeYaml = await readProfileFile('Merge')
      } catch (readErr) {
        console.warn(
          'Global Merge file not found or failed to read, initializing empty merge:',
          readErr,
        )
        mergeYaml = '{}'
      }
      const mergeObj = (yaml.load(mergeYaml) || {}) as Record<string, any>

      const existing: string[] = Array.isArray(mergeObj['prepend-rules'])
        ? (mergeObj['prepend-rules'] as string[])
        : []
      const added: string[] = []
      for (const rule of rules) {
        if (!existing.includes(rule) && !added.includes(rule)) {
          added.push(rule)
        }
      }

      if (added.length === 0) {
        return
      }

      addedCount = added.length

      // 手动添加的网址置顶，压过 GFWList 与 MATCH。
      mergeObj['prepend-rules'] = [...added, ...existing]

      await saveProfileFile('Merge', yaml.dump(mergeObj))
    })

    if (addedCount === 0) return 0

    await enhanceProfiles()

    return addedCount
  } catch (e: any) {
    console.error('Failed to add bulk routing rules:', e)
    showNotice.error(e.message || String(e))
    return 0
  }
}

// 从一段输入（可能带协议头/路径/端口）中提取出用于 PROXY 的 Clash 规则字符串。
const buildProxyRule = (input: string): string | null => {
  if (!input) return null

  let value = input.trim()
  // 去掉协议头 http:// https:// etc.
  value = value.replace(/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//, '')
  // 去掉路径、查询、锚点、端口
  value = value.split('/')[0].split('?')[0].split('#')[0].split(':')[0]
  value = value.replace(/^www\./, '')
  value = value.trim()
  if (!value) return null

  // IP 直接走 IP-CIDR
  if (/^[0-9.]+$/.test(value)) {
    return `IP-CIDR,${value}/32,PROXY`
  }
  if (/^[0-9a-fA-F:]+$/.test(value)) {
    return `IP-CIDR,${value}/128,PROXY`
  }

  // 域名取后缀（二级或三级）
  const parts = value.split('.')
  if (parts.length < 2) return null
  const lastPart = parts[parts.length - 1]
  const secondLastPart = parts[parts.length - 2]
  const isDoubleTld =
    ['com', 'org', 'net', 'gov', 'edu', 'co'].includes(secondLastPart) &&
    lastPart.length <= 3
  const domainSuffix =
    parts.length >= 3 && isDoubleTld
      ? parts.slice(-3).join('.')
      : parts.slice(-2).join('.')

  return `DOMAIN-SUFFIX,${domainSuffix},PROXY`
}
