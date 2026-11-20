import { useState } from 'react'

import { useServiceInstaller } from '@/hooks/use-service-installer'
import { useSystemState } from '@/hooks/use-system-state'
import { useVerge } from '@/hooks/use-verge'
import {
  beginLongOperation,
  endLongOperation,
  patchClashMode,
} from '@/services/cmds'
import { closeAllConnectionsWithTimeout } from '@/services/mihomo-api'
import {
  markLoadingNotice,
  settleLoadingNotice,
  showNotice,
} from '@/services/notice-service'

interface UseProxyModeActionsParams {
  activateSelected: () => Promise<void>
}

export function useProxyModeActions({
  activateSelected,
}: UseProxyModeActionsParams) {
  const { verge, patchVerge } = useVerge()
  const { isTunModeAvailable, mutateSystemState } = useSystemState()
  const { installServiceAndSync } = useServiceInstaller()
  const [addUrlOpen, setAddUrlOpen] = useState(false)

  const { enable_tun_mode, enable_system_proxy } = verge ?? {}
  const currentMode = enable_tun_mode
    ? 'tun'
    : enable_system_proxy
      ? 'system'
      : 'manual'
  const activeIndex =
    currentMode === 'tun' ? 2 : currentMode === 'system' ? 1 : 0

  const policyActiveIndex =
    verge?.rule_fallback === 'proxy'
      ? 0
      : verge?.rule_fallback === 'addurl'
        ? 2
        : 1

  const handleRuleFallbackChange = async (
    fallback: 'direct' | 'proxy' | 'addurl',
  ): Promise<boolean> => {
    if (verge?.rule_fallback === fallback) return true

    const waitId = showNotice.info('layout.notices.adjustingProxyMode', 0)
    markLoadingNotice(waitId)

    beginLongOperation()
    try {
      const applied = await patchVerge(
        { rule_fallback: fallback },
        { silentError: true },
      )
      if (!applied) return false

      await patchClashMode('rule')
      await activateSelected()
      if (fallback === 'proxy') {
        showNotice.success('layout.notices.switchedToGlobalProxy')
      } else if (fallback === 'direct') {
        showNotice.success('layout.notices.switchedToGfwlist')
      } else {
        showNotice.success('layout.notices.switchedToAddUrl')
      }
      return true
    } catch (err: unknown) {
      showNotice.error(err)
      return false
    } finally {
      settleLoadingNotice(waitId)
      endLongOperation()
    }
  }

  const handleAddUrlClick = async () => {
    if (verge?.rule_fallback === 'addurl') {
      setAddUrlOpen(true)
      return
    }
    const ok = await handleRuleFallbackChange('addurl')
    if (ok) {
      setAddUrlOpen(true)
    }
  }

  const handleTakeoverModeChange = async (
    targetMode: 'manual' | 'system' | 'tun',
  ) => {
    if (targetMode === currentMode) return

    const waitId = showNotice.info('layout.notices.adjusting', 0)
    markLoadingNotice(waitId)
    beginLongOperation()
    try {
      if (targetMode === 'tun' && !isTunModeAvailable) {
        try {
          await installServiceAndSync({ silentError: true })
        } catch {
          showNotice.error('layout.notices.tunServiceConfigFailed')
          return
        }
      }

      const modeConfig = {
        manual: {
          patch: { enable_system_proxy: false, enable_tun_mode: false },
          notice: 'layout.notices.switchedToManualMode' as const,
        },
        system: {
          patch: { enable_system_proxy: true, enable_tun_mode: false },
          notice: 'layout.notices.systemProxyEnabled' as const,
        },
        tun: {
          patch: { enable_system_proxy: false, enable_tun_mode: true },
          notice: 'layout.notices.tunModeEnabled' as const,
        },
      }[targetMode]

      const applied = await patchVerge(modeConfig.patch, { silentError: true })
      if (!applied) return
      await mutateSystemState()
      if (targetMode === 'manual' && verge?.auto_close_connection) {
        await closeAllConnectionsWithTimeout().catch(() =>
          console.warn('[layout] closeAllConnectionsWithTimeout failed'),
        )
      }
      showNotice.success(modeConfig.notice)
    } catch (err) {
      showNotice.error(err)
    } finally {
      settleLoadingNotice(waitId)
      endLongOperation()
    }
  }

  return {
    verge,
    patchVerge,
    activeIndex,
    policyActiveIndex,
    addUrlOpen,
    setAddUrlOpen,
    handleRuleFallbackChange,
    handleAddUrlClick,
    handleTakeoverModeChange,
  }
}
