import { RouterOutlined, RefreshRounded, CheckCircleOutlineRounded } from '@mui/icons-material'
import {
  Box,
  Button,
  CircularProgress,
  FormControl,
  MenuItem,
  Select,
  Typography,
  alpha,
  useTheme,
} from '@mui/material'
import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import yaml from 'js-yaml'

import { EnhancedCard } from './enhanced-card'
import {
  getProfiles,
  readProfileFile,
  saveProfileFile,
  enhanceProfiles,
  patchClashMode,
  calcuProxies,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

// 定义规则集模板
const GFWLIST_PROVIDER_YAML = `rule-providers:
  gfwlist:
    type: http
    behavior: domain
    url: "https://raw.githubusercontent.com/Loyalsoldier/clash-rules/release/gfw.txt"
    path: ./ruleset/gfwlist.yaml
    interval: 86400
`

export const SmartRoutingCard = () => {
  const { t } = useTranslation()
  const theme = useTheme()
  const [loading, setLoading] = useState(false)
  const [routingMode, setRoutingMode] = useState<string>(() => {
    return localStorage.getItem('verge_smart_routing_mode') || 'bypass-china'
  })

  // 辅助函数：智能匹配订阅中的主代理组
  const findMainProxyGroup = async (): Promise<string> => {
    try {
      const proxies = await calcuProxies()
      const groups = proxies.groups || []
      if (groups.length === 0) return 'GLOBAL'

      // 寻找 select 类型且名称中含有常见字样的组
      const selectGroups = groups.filter((g) => g.type === 'select')
      const commonPattern = /proxy|选择|手动|select|node|节点/i
      const matched = selectGroups.find((g) => commonPattern.test(g.name))
      if (matched) return matched.name

      // 退而求其次选择第一个 select 组
      if (selectGroups.length > 0) return selectGroups[0].name

      // 再退而求其次选择第一个组
      return groups[0].name
    } catch (e) {
      console.error('Failed to calculate proxies:', e)
      return 'Proxy' // 默认兜底
    }
  }

  // 核心功能：应用路由模式修改
  const applyRoutingMode = async (mode: string) => {
    setLoading(true)
    try {
      // 1. 获取所有 profile，找到当前激活的 profile 及其 rules 项
      const profilesConfig = await getProfiles()
      const activeUid = profilesConfig.current
      if (!activeUid) {
        throw new Error('没有检测到当前激活的订阅配置文件')
      }

      const activeProfile = profilesConfig.items?.find((item) => item.uid === activeUid)
      if (!activeProfile) {
        throw new Error('未找到当前激活订阅的配置项')
      }

      const rulesUid = activeProfile.option?.rules
      if (!rulesUid) {
        throw new Error('当前订阅没有关联的规则文件')
      }

      // 2. 读取并解析当前的 rules 文件
      const rawRulesYaml = await readProfileFile(rulesUid)
      const rulesData = (yaml.load(rawRulesYaml) || {}) as {
        prepend?: string[]
        append?: string[]
        delete?: string[]
      }

      // 提取原有的用户自定义规则（过滤掉 GFWList 和 Bypass China 规则模板）
      const userCustomRules = (rulesData.prepend || []).filter((rule) => {
        const isGfwRule = rule.includes('gfwlist') || rule.includes('MATCH,DIRECT')
        const isChinaRule =
          rule.includes('GEOSITE,cn') ||
          rule.includes('GEOIP,cn') ||
          rule.includes('MATCH,Proxy')
        return !isGfwRule && !isChinaRule
      })

      // 3. 根据所选模式生成新的 rules prepend，并调用内核 mode 修改
      let newPrepend = [...userCustomRules]

      if (mode === 'gfwlist') {
        // GFWList 模式
        await patchClashMode('rule')
        const mainProxyGroup = await findMainProxyGroup()
        newPrepend.push(`RULE-SET,gfwlist,${mainProxyGroup}`)
        newPrepend.push('MATCH,DIRECT')

        // 在全局 Merge 中添加 rule-providers
        try {
          const mergeYaml = await readProfileFile('Merge')
          const mergeObj = (yaml.load(mergeYaml) || {}) as Record<string, any>
          
          if (!mergeObj['rule-providers'] || !mergeObj['rule-providers'].gfwlist) {
            mergeObj['rule-providers'] = mergeObj['rule-providers'] || {}
            mergeObj['rule-providers'].gfwlist = {
              type: 'http',
              behavior: 'domain',
              url: 'https://raw.githubusercontent.com/Loyalsoldier/clash-rules/release/gfw.txt',
              path: './ruleset/gfwlist.yaml',
              interval: 86400,
            }
            await saveProfileFile('Merge', yaml.dump(mergeObj))
          }
        } catch (mergeErr) {
          console.warn('Failed to inject rule-providers into Merge:', mergeErr)
        }
      } else if (mode === 'bypass-china') {
        // 绕过大陆模式
        await patchClashMode('rule')
        newPrepend.push('GEOSITE,cn,DIRECT')
        newPrepend.push('GEOIP,cn,DIRECT,no-resolve')
      } else if (mode === 'global') {
        // 全局代理
        await patchClashMode('global')
      } else if (mode === 'direct') {
        // 完全直连
        await patchClashMode('direct')
      }

      // 4. 保存 rules 文件并增强配置使其生效
      rulesData.prepend = newPrepend
      await saveProfileFile(rulesUid, yaml.dump(rulesData))
      await enhanceProfiles()

      localStorage.setItem('verge_smart_routing_mode', mode)
      setRoutingMode(mode)
      showNotice.success('home.page.feedback.notifications.profileSwitched')
    } catch (e: any) {
      console.error('Failed to apply routing mode:', e)
      showNotice.error(e.message || String(e))
    } finally {
      setLoading(false)
    }
  }

  // 同步初始化模式
  useEffect(() => {
    // 首次载入时，如果与本地存储不一致，则重新应用一次
    const stored = localStorage.getItem('verge_smart_routing_mode')
    if (stored && stored !== routingMode) {
      setRoutingMode(stored)
    }
  }, [])

  return (
    <EnhancedCard
      title="智能分流中心"
      icon={<RouterOutlined />}
      iconColor="success"
      action={
        <Button
          size="small"
          onClick={() => applyRoutingMode(routingMode)}
          disabled={loading}
          sx={{ minWidth: 'auto' }}
          className="aero-crystal-btn"
        >
          {loading ? (
            <CircularProgress size={16} sx={{ color: 'inherit' }} />
          ) : (
            <RefreshRounded fontSize="small" />
          )}
        </Button>
      }
    >
      <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', justifyContent: 'space-between', gap: 1.5 }}>
        <Box>
          <Typography variant="body2" sx={{ color: 'text.secondary', mb: 1.5, fontSize: 13 }}>
            系统级智能网络分流，国内流量自动绕过，海外流量及被墙站点智能分流。
          </Typography>

          <FormControl fullWidth size="small">
            <Select
              value={routingMode}
              onChange={(e) => applyRoutingMode(e.target.value)}
              disabled={loading}
              className="aero-crystal-btn"
              sx={{
                height: 38,
                bgcolor: 'rgba(255, 255, 255, 0.05)',
                '& .MuiSelect-select': {
                  py: 1.2,
                  px: 2,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 1,
                  fontSize: 14,
                  fontWeight: 600,
                },
              }}
            >
              <MenuItem value="bypass-china" sx={{ fontSize: 14, fontWeight: 500 }}>
                💡 绕过中国大陆 (推荐)
              </MenuItem>
              <MenuItem value="gfwlist" sx={{ fontSize: 14, fontWeight: 500 }}>
                🛡️ 仅代理被墙网站 (GFWList)
              </MenuItem>
              <MenuItem value="global" sx={{ fontSize: 14, fontWeight: 500 }}>
                🌐 全局代理模式
              </MenuItem>
              <MenuItem value="direct" sx={{ fontSize: 14, fontWeight: 500 }}>
                🔌 完全直连模式
              </MenuItem>
            </Select>
          </FormControl>
        </Box>

        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            gap: 1,
            py: 0.8,
            px: 1.5,
            borderRadius: 1.5,
            bgcolor: alpha(theme.palette.success.main, 0.08),
            border: '1px solid',
            borderColor: alpha(theme.palette.success.main, 0.15),
          }}
        >
          <CheckCircleOutlineRounded color="success" sx={{ fontSize: 16 }} />
          <Typography variant="caption" sx={{ color: 'success.main', fontWeight: 600, fontSize: 12 }}>
            状态正常：
            {routingMode === 'bypass-china' && '绕过大陆智能直连生效中'}
            {routingMode === 'gfwlist' && 'GFWList 黑名单过滤已生效'}
            {routingMode === 'global' && '全局接管中 (不进行分流)'}
            {routingMode === 'direct' && '直连中 (绕过所有代理)'}
          </Typography>
        </Box>
      </Box>
    </EnhancedCard>
  )
}
