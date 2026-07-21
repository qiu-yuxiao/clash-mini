import { useQuery } from '@tanstack/react-query'
import { useLockFn } from 'ahooks'
import { useCallback } from 'react'

import {
  calcuProxies,
  frontendLog,
  getProfiles,
  patchProfile,
  patchProfilesConfig,
} from '@/services/cmds'
import { selectNodeForGroupWithTimeout } from '@/services/mihomo-api'
import { queryClient } from '@/services/query-client'
import type { IProfileItem, IProfilesConfig } from '@/types/profile'
import { debugLog } from '@/utils/debug'

// 从 localStorage 的 proxy-head-state 读取当前 profile 的过滤词（与后端 match_filter 语义一致：
// 大小写不敏感子串匹配）。读取失败或为空时返回 ''，等价于"无过滤"，即全部节点都在子集内。
// 该 key 由 use-head-state.ts 在加载/变更时写入，activateSelected 运行时通常已就绪。
function readActiveFilterText(uid?: string): string {
  if (!uid) return ''
  try {
    const raw = localStorage.getItem('proxy-head-state')
    if (!raw) return ''
    const storage = JSON.parse(raw) as Record<
      string,
      Record<string, { filterText?: string }>
    >
    return storage?.[uid]?.['PROXY']?.filterText ?? ''
  } catch {
    return ''
  }
}

export const useProfiles = () => {
  const {
    data: profiles,
    refetch,
    error,
    isFetching: isValidating,
  } = useQuery({
    queryKey: ['getProfiles'],
    queryFn: async () => {
      const data = await getProfiles()
      debugLog(
        '[useProfiles] 配置数据更新成功，配置数量:',
        data?.items?.length || 0,
      )
      return data
    },
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    staleTime: 500,
    retry: 3,
    retryDelay: 1000,
    refetchInterval: false,
  })

  const mutateProfiles = useCallback(async () => {
    await refetch()
  }, [refetch])

  const updateProfilesCacheSelected = useCallback(
    (uid: string, selected: { name: string; now: string }[]) => {
      queryClient.setQueryData<IProfilesConfig>(['getProfiles'], (old) => {
        if (!old) return old
        return {
          ...old,
          items: old.items?.map((item) => {
            if (item && item.uid === uid) {
              return {
                ...item,
                selected,
              }
            }
            return item
          }),
        }
      })
    },
    [],
  )

  const patchProfiles = useLockFn(
    async (
      value: Partial<IProfilesConfig>,
      signal?: AbortSignal,
      options?: { deferRefreshOnSuccess?: boolean },
    ) => {
      try {
        if (signal?.aborted) {
          throw new DOMException('Operation was aborted', 'AbortError')
        }
        const success = await patchProfilesConfig(value)

        if (signal?.aborted) {
          throw new DOMException('Operation was aborted', 'AbortError')
        }

        if (!options?.deferRefreshOnSuccess || !success) {
          await mutateProfiles()
        }

        return success
      } catch (error) {
        if (error instanceof DOMException && error.name === 'AbortError') {
          throw error
        }

        await mutateProfiles()
        throw error
      }
    },
  )

  const patchCurrent = useLockFn(async (value: Partial<IProfileItem>) => {
    if (profiles?.current) {
      await patchProfile(profiles.current, value)
      // 同步更新 React Query 内存缓存
      queryClient.setQueryData<IProfilesConfig>(['getProfiles'], (old) => {
        if (!old) return old
        return {
          ...old,
          items: old.items?.map((item) => {
            if (item && item.uid === profiles.current) {
              return {
                ...item,
                ...value,
              }
            }
            return item
          }),
        }
      })
      if (!value.selected) {
        await mutateProfiles()
      }
    }
  })

  // 根据selected的节点选择恢复 PROXY 组
  // 【核心架构约定】Mini 只有 PROXY 一个有效代理组，此函数只处理 PROXY 组，
  // 不遍历多组（上游 Clash Verge Rev 的多组遍历逻辑已移除）
  const activateSelected = useCallback(
    async (profileOverride?: IProfilesConfig) => {
      try {
        debugLog('[ActivateSelected] 开始处理代理选择')

        const proxiesData = await calcuProxies()
        const profileData = profileOverride ?? profiles

        if (!profileData || !proxiesData || !profileData.items) {
          debugLog('[ActivateSelected] 代理或配置数据不可用，跳过处理')
          return
        }

        const current = profileData.items?.find(
          (e) => e && e.uid === profileData.current,
        )

        if (!current) {
          debugLog('[ActivateSelected] 未找到当前profile配置')
          return
        }

        // 检查是否有saved的代理选择
        const { selected = [] } = current
        if (selected.length === 0) {
          debugLog('[ActivateSelected] 当前profile无保存的代理选择，跳过')
          return
        }

        // Mini 单组架构：groups 只有 PROXY 一个元素
        const proxyGroup = proxiesData.groups?.[0]
        if (!proxyGroup) {
          debugLog('[ActivateSelected] 未找到 PROXY 组，跳过')
          return
        }

        // Mini 单组架构：selected 数组仅一个元素 {name:'PROXY', now}
        const savedEntry = selected[0]
        const savedProxyName: string | undefined =
          savedEntry?.name === 'PROXY' ? savedEntry.now : undefined
        if (!savedProxyName) {
          debugLog('[ActivateSelected] selected 中无 PROXY 组的有效记录，跳过')
          return
        }

        const availableProxies = Array.isArray(proxyGroup.all)
          ? proxyGroup.all
          : []
        const currentNow = proxyGroup.now || ''

        const stripSuffix = (n: string) => {
          return n.replace(/\s\(\d{6}\)$/, '').trim()
        }

        // 将 selected 校准为内核当前实际运行的节点，防止前后端状态脱节
        const calibrateSelected = async (uid: string, proxyName: string) => {
          const newSelected = [{ name: 'PROXY', now: proxyName }]
          try {
            await patchProfile(uid, { selected: newSelected })
            updateProfilesCacheSelected(uid, newSelected)
            await queryClient.invalidateQueries({ queryKey: ['getProxies'] })
          } catch (err) {
            console.error('[ActivateSelected] 修正 Profile.selected 失败:', err)
          }
        }

        const matchedProxy = availableProxies.find((proxy) => {
          const pName = typeof proxy === 'string' ? proxy : proxy?.name
          if (!pName) return false
          return stripSuffix(pName) === stripSuffix(savedProxyName)
        })

        if (!matchedProxy) {
          const msg = `[ActivateSelected] 保存的代理 ${savedProxyName} 不存在于 PROXY 组（订阅可能已更新），PROXY.now 维持 mihomo 内核当前值 ${currentNow}`
          console.warn(msg)
          // 写入后端 latest.log，便于排查"前端显示与后端实际不一致"问题
          frontendLog('error', msg)

          // 🛡️【强咬合防线】将本地配置 selected 强制修正校准为内核当前实际的运行节点，防止状态脱节
          if (currentNow) {
            await calibrateSelected(current.uid, currentNow)
          }
          return
        }

        const matchedProxyName =
          typeof matchedProxy === 'string' ? matchedProxy : matchedProxy.name

        if (matchedProxyName === currentNow) {
          debugLog('[ActivateSelected] PROXY 组选择已是目标状态，无需更新')
          return
        }

        // 【治本修复】savedProxyName 可能已被污染（如导入盲选写回的越界节点）。
        // 仅当它在用户当前 filterText 子集内时才信任并切过去；越界则不再强加给内核，
        // 而是把本地 selected 校正为内核实际节点 currentNow，避免"前端显示 ≠ 内核实际选路"复现。
        const filterText = readActiveFilterText(current?.uid)
        const savedInSubset =
          filterText.trim() === '' ||
          savedProxyName.toLowerCase().includes(filterText.trim().toLowerCase())

        if (!savedInSubset) {
          const msg = `[ActivateSelected] 保存的代理 ${savedProxyName} 不在当前过滤范围「${filterText}」内（疑似被污染的持久选择），不切内核，改为校正 selected 为内核实际节点 ${currentNow}`
          console.warn(msg)
          frontendLog('error', msg)
          // 🛡️【强咬合防线】越界时不切内核，仅把 selected 校准为内核实际节点，保持前后端一致
          if (currentNow) {
            await calibrateSelected(current.uid, currentNow)
          }
          return
        }

        debugLog(
          `[ActivateSelected] 需要切换 PROXY 组: ${currentNow} -> ${matchedProxyName}`,
        )
        try {
          await selectNodeForGroupWithTimeout('PROXY', matchedProxyName)
        } catch (error: unknown) {
          const msg = `[ActivateSelected] 切换 PROXY 组失败 (target=${matchedProxyName}, current=${currentNow}): ${error instanceof Error ? error.message : String(error)}`
          console.warn(msg)
          // 写入后端 latest.log：reload_config 后 mihomo 未就绪或节点已失效时切换会失败，
          // 此时 mihomo 实际选路与前端 selected 不一致，是断流 of 常见根因
          frontendLog('error', msg)

          // 🛡️【强咬合防线】切换异常时，将本地配置 selected 强制重置校准为内核当前的真实运行节点
          if (currentNow) {
            await calibrateSelected(current.uid, currentNow)
          }
          return
        }

        // 写回 selected 数组（仅 PROXY 一项）
        const newSelected = [{ name: 'PROXY', now: matchedProxyName }]
        try {
          await patchProfile(current.uid, { selected: newSelected })
          updateProfilesCacheSelected(current.uid, newSelected)
          debugLog('[ActivateSelected] 代理选择配置保存成功')

          queryClient.invalidateQueries({ queryKey: ['getProxies'] })
        } catch (error: unknown) {
          console.error(
            '[ActivateSelected] 保存代理选择配置失败:',
            error instanceof Error ? error.message : String(error),
          )
        }
      } catch (error: unknown) {
        console.error(
          '[ActivateSelected] 处理代理选择失败:',
          error instanceof Error ? error.message : String(error),
        )
      }
    },
    [profiles, updateProfilesCacheSelected],
  )

  return {
    profiles,
    current: profiles?.items?.find((p) => p && p.uid === profiles.current),
    activateSelected,
    patchProfiles,
    patchCurrent,
    mutateProfiles,
    // 新增故障检测状态
    isLoading: isValidating,
    error,
    isStale: !profiles && !error && !isValidating, // 检测是否处于异常状态
  }
}
