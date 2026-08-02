import { useQuery } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'


import { useVisibility } from '@/hooks/use-visibility'
import {
  getRunningMode,
  isAdmin,
  isServiceAvailable,
  isLongOperationRunning,
} from '@/services/cmds'
import { STARTUP_GRACE_MS } from '@/services/delay'
import { showNotice } from '@/services/notice-service'

import { useVerge } from './use-verge'

export interface SystemState {
  runningMode: 'Sidecar' | 'Service'
  isAdminMode: boolean
  isServiceOk: boolean
}

const defaultSystemState = {
  runningMode: 'Sidecar',
  isAdminMode: false,
  isServiceOk: false,
} as SystemState

/**
 * 自定义 hook 用于获取系统运行状态
 * 包括运行模式、管理员状态、系统服务是否可用
 */
export function useSystemState() {
  const { verge, patchVerge } = useVerge()
  const isVisible = useVisibility()
  const disablingTunRef = useRef(false)
  const [isStartingUp, setIsStartingUp] = useState(true)

  useEffect(() => {
    const timer = setTimeout(() => setIsStartingUp(false), STARTUP_GRACE_MS)
    return () => clearTimeout(timer)
  }, [])

  const {
    data: systemState = defaultSystemState,
    refetch: mutateSystemState,
    isLoading,
  } = useQuery({
    queryKey: ['getSystemState'],
    queryFn: async () => {
      const [runningMode, isAdminMode, isServiceOk] = await Promise.all([
        getRunningMode(),
        isAdmin(),
        isServiceAvailable(),
      ])
      return { runningMode, isAdminMode, isServiceOk } as SystemState
    },
    // 启动期（10s 内）5s 轮询一次；启动完成后 30s 轮询一次。
    // 原值 2s 在 IPC 阻塞期间会堆积 timeout notice，加剧正反馈卡死。
    refetchInterval: isVisible ? (isStartingUp ? 5000 : 30000) : false,
    refetchIntervalInBackground: false,
  })

  const isSidecarMode = systemState.runningMode === 'Sidecar'
  const isServiceMode = systemState.runningMode === 'Service'
  const isTunModeAvailable = systemState.isAdminMode || systemState.isServiceOk

  const enable_tun_mode = verge?.enable_tun_mode
  const cooldownTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isTunModeAvailableRef = useRef(isTunModeAvailable)
  const isLoadingRef = useRef(isLoading)
  const isStartingUpRef = useRef(isStartingUp)
  const patchVergeRef = useRef(patchVerge)

  useEffect(() => {
    isTunModeAvailableRef.current = isTunModeAvailable
  }, [isTunModeAvailable])

  useEffect(() => {
    isLoadingRef.current = isLoading
  }, [isLoading])

  useEffect(() => {
    isStartingUpRef.current = isStartingUp
  }, [isStartingUp])

  useEffect(() => {
    patchVergeRef.current = patchVerge
  }, [patchVerge])

  useEffect(() => {
    if (enable_tun_mode === undefined) return

    // 长操作期间（用户主动切换 TUN/系统代理等）禁止自动关闭 TUN：
    // installServiceAndRestartCore 完成后 isTunModeAvailable 可能短暂为 false
    // （mihomo 刚重启，服务 IPC 未就绪），此时用户已 patchVerge({enable_tun_mode:true})，
    // 若自动关闭 TUN 的 patchVerge 与之并发，会导致配置反复修改、mihomo 反复 reload、
    // IPC 通道阻塞、notice 堆积，最终画面卡死。
    // 长操作结束后 isTunModeAvailable 已正确刷新，useEffect 不会误触发。
    if (
      !disablingTunRef.current &&
      enable_tun_mode &&
      !isTunModeAvailableRef.current &&
      !isLoadingRef.current &&
      !isStartingUpRef.current &&
      !isLongOperationRunning()
    ) {
      disablingTunRef.current = true
      patchVergeRef.current({ enable_tun_mode: false, enable_system_proxy: true })
        .then(() => {
          showNotice.info(
            'settings.sections.system.notifications.tunMode.autoDisabled',
          )
        })
        .catch((err) => {
          console.error('[useVerge] 自动关闭TUN模式失败:', err)
          showNotice.error(
            'settings.sections.system.notifications.tunMode.autoDisableFailed',
          )
        })
        .finally(() => {
          // 避免 verge 数据更新不及时导致重复执行关闭 Tun 模式
          cooldownTimerRef.current = setTimeout(() => {
            disablingTunRef.current = false
            cooldownTimerRef.current = null
          }, 1000)
        })
    }

    return () => {
      if (cooldownTimerRef.current != null) {
        clearTimeout(cooldownTimerRef.current)
        cooldownTimerRef.current = null
        disablingTunRef.current = false
      }
    }
  }, [enable_tun_mode])

  return {
    runningMode: systemState.runningMode,
    isAdminMode: systemState.isAdminMode,
    isServiceOk: systemState.isServiceOk,
    isSidecarMode,
    isServiceMode,
    isTunModeAvailable,
    mutateSystemState,
    isLoading,
  }
}
