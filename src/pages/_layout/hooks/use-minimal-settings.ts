import { useCallback, useEffect, useState } from 'react'

import { useClashInfo } from '@/hooks/use-clash'
import { useVerge } from '@/hooks/use-verge'
import {
  beginLongOperation,
  endLongOperation,
  isPortInUse,
  patchClashConfig,
} from '@/services/cmds'
import { hideNotice, showNotice } from '@/services/notice-service'
import { queryClient } from '@/services/query-client'
import type { IClashInfo } from '@/types/clash'

const DEFAULT_MIXED_PORT = 10801

/**
 * 「最小设置区」的端口与布尔开关动作。
 *
 * 从 `_layout.tsx` 抽出（该文件此前 2000+ 行、职责过载），这里只聚合两件事：
 * 1. `mixed-port` 的本地编辑态与保存（含端口占用校验、失败回滚）
 * 2. `allow-lan` / `ipv6` 切换（含乐观更新、失败回滚、长操作标记）
 *
 * 两者都依赖 `useClashInfo` / `useVerge` 的查询缓存，重复调用不会产生额外请求
 * （react-query 同 key 共享缓存）。
 */
export const useMinimalSettings = () => {
  const { verge, patchVerge } = useVerge()
  const { clashInfo, mutateInfo, patchInfo } = useClashInfo()

  const [mixedPortVal, setMixedPortVal] = useState<number>(
    verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? DEFAULT_MIXED_PORT,
  )

  // 端口来源优先级：用户自定义 verge_mixed_port > 内核实际 mixed_port > 默认值
  useEffect(() => {
    const vPort = verge?.verge_mixed_port
    const cPort = clashInfo?.mixed_port
    if (vPort !== undefined && vPort !== null) {
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setMixedPortVal(vPort)
    } else if (cPort !== undefined && cPort !== null) {
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setMixedPortVal(cPort)
    }
  }, [verge?.verge_mixed_port, clashInfo?.mixed_port])

  const currentPort = useCallback(
    () =>
      verge?.verge_mixed_port ?? clashInfo?.mixed_port ?? DEFAULT_MIXED_PORT,
    [verge?.verge_mixed_port, clashInfo?.mixed_port],
  )

  /**
   * 切换 `allow-lan` / `ipv6`。
   *
   * 这两个字段变更会让后端 **重启内核**（feat/config.rs：allow_lan_changed /
   * ipv6_changed → need_restart → restart_core），期间 IPC 通道最长阻塞 60s，
   * 因此必须 `beginLongOperation` 抑制超时提示风暴。
   * 乐观更新写在长操作之前，失败时用 `mutateInfo()` 回滚到后端真实值。
   */
  const handleClashBoolChange = (field: string) => async (checked: boolean) => {
    if (field === 'ipv6' || field === 'allow-lan') {
      const cacheKey = field === 'allow-lan' ? 'allow_lan' : 'ipv6'
      queryClient.setQueryData<IClashInfo>(['getClashInfo'], (old) =>
        old ? { ...old, [cacheKey]: checked } : old,
      )
    }

    const waitId = showNotice.info('正在调整，请稍候…', 0)
    beginLongOperation()
    try {
      await patchClashConfig({ [field]: checked })
      await mutateInfo()
      showNotice.success('已更新')
    } catch (err: unknown) {
      await mutateInfo()
      showNotice.error(err instanceof Error ? err.message : String(err))
    } finally {
      hideNotice(waitId)
      endLongOperation()
    }
  }

  /** 保存代理端口：占用检查 → 同时写内核与 verge → 失败回滚输入框。 */
  const handleSavePort = async (port: number) => {
    if (port === verge?.verge_mixed_port && port === clashInfo?.mixed_port)
      return
    try {
      const inUse = await isPortInUse(port)
      if (inUse) {
        showNotice.error('settings.modals.clashPort.messages.portInUse', {
          port,
        })
        setMixedPortVal(currentPort())
        return
      }
      await Promise.all([
        patchInfo({ 'mixed-port': port }),
        patchVerge({ verge_mixed_port: port }),
      ])
      showNotice.success('代理端口已保存并重载')
    } catch (err) {
      showNotice.error(err)
      setMixedPortVal(currentPort())
    }
  }

  return { mixedPortVal, handleSavePort, handleClashBoolChange }
}
