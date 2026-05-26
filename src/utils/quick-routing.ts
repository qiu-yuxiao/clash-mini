import yaml from 'js-yaml'

import {
  readProfileFile,
  saveProfileFile,
  enhanceProfiles,
  calcuProxies,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

// 智能获取主代理组名称
const findMainProxyGroup = async (): Promise<string> => {
  try {
    const proxies = await calcuProxies()
    const groups = proxies.groups || []
    if (groups.length === 0) return 'GLOBAL'

    const selectGroups = groups.filter((g) => g.type === 'select')
    const commonPattern = /proxy|选择|手动|select|node|节点/i
    const matched = selectGroups.find((g) => commonPattern.test(g.name))
    if (matched) return matched.name

    if (selectGroups.length > 0) return selectGroups[0].name
    return groups[0].name
  } catch (e) {
    console.error('Failed to get main proxy group:', e)
    return 'Proxy'
  }
}

// 核心功能：添加快捷分流规则到全局 Merge 的 prepend-rules 中 (置顶生效)
export const addQuickRoutingRule = async (
  type: 'process' | 'domain',
  value: string,
  target: 'DIRECT' | 'PROXY' | 'REJECT'
) => {
  if (!value) return

  try {
    // 1. 获取主代理组名称 (如果是代理，则需要目标代理组名；如果是封锁，使用 REJECT；如果是直连，使用 DIRECT)
    let proxyGroup = 'DIRECT'
    if (target === 'PROXY') {
      proxyGroup = await findMainProxyGroup()
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
          const isDoubleTld = ['com', 'org', 'net', 'gov', 'edu', 'co'].includes(secondLastPart) && lastPart.length <= 3
          
          if (parts.length >= 3 && isDoubleTld) {
            domainSuffix = parts.slice(-3).join('.')
          } else {
            domainSuffix = parts.slice(-2).join('.')
          }
        }
        newRule = `DOMAIN-SUFFIX,${domainSuffix},${proxyGroup}`
      }
    }

    // 3. 读取全局 Merge 配置文件
    let mergeYaml = ''
    try {
      mergeYaml = await readProfileFile('Merge')
    } catch (readErr) {
      console.warn('Global Merge file not found or failed to read, initializing empty merge:', readErr)
      mergeYaml = '{}'
    }
    const mergeObj = (yaml.load(mergeYaml) || {}) as Record<string, any>

    // 4. 确保 prepend-rules 数组存在
    mergeObj['prepend-rules'] = mergeObj['prepend-rules'] || []

    // 5. 过滤掉已有的相同属性规则（去重并置顶）
    const matchPrefix = type === 'process' 
      ? `PROCESS-NAME,${value},` 
      : `DOMAIN-SUFFIX,${domainSuffix},`
      
    const filteredRules = (mergeObj['prepend-rules'] as string[]).filter(
      (rule) => !rule.startsWith(matchPrefix) && rule !== newRule
    )

    // 置顶写入规则
    mergeObj['prepend-rules'] = [newRule, ...filteredRules]

    // 6. 保存并生效
    await saveProfileFile('Merge', yaml.dump(mergeObj))
    await enhanceProfiles()

    showNotice.success(
      'profiles.page.feedback.notifications.profileSwitched',
      `手动路径控制规则已置顶生效: ${newRule}`,
      2500
    )
  } catch (e: any) {
    console.error('Failed to add quick routing rule:', e)
    showNotice.error(e.message || String(e))
  }
}
