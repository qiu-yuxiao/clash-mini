import { useQuery } from '@tanstack/react-query'
import { useLockFn } from 'ahooks'

import { getClashInfo, patchClashConfig } from '@/services/cmds'
import { getVersionWithTimeout } from '@/services/mihomo-api'
import { queryClient } from '@/services/query-client'
import type { IConfigData } from '@/types/clash'

const PORT_KEYS = [
  'port',
  'socks-port',
  'mixed-port',
  'redir-port',
  'tproxy-port',
] as const

type ClashInfoPatch = Partial<
  Pick<
    IConfigData,
    | 'port'
    | 'socks-port'
    | 'mixed-port'
    | 'redir-port'
    | 'tproxy-port'
    | 'external-controller'
    | 'secret'
  >
>

const hasClashInfoPayload = (patch: ClashInfoPatch) =>
  PORT_KEYS.some((key) => patch[key] != null) ||
  patch['external-controller'] != null ||
  patch.secret != null

const validatePortRange = (port: number) => {
  if (port < 1000) {
    throw new Error('The port should not < 1000')
  }
  if (port > 65535) {
    throw new Error('The port should not > 65536')
  }
}

const validatePorts = (patch: ClashInfoPatch) => {
  PORT_KEYS.forEach((key) => {
    const port = patch[key]
    if (!port) return
    validatePortRange(port)
  })
}

export const useClash = () => {
  const { data: versionData, refetch: mutateVersion } = useQuery({
    queryKey: ['getVersion'],
    queryFn: getVersionWithTimeout,
  })

  const version = versionData?.meta
    ? `${versionData.version} Mihomo`
    : versionData?.version || '-'

  return {
    version,
    mutateVersion,
  }
}

export const useClashInfo = () => {
  const { data: clashInfo, refetch: mutateInfo } = useQuery({
    queryKey: ['getClashInfo'],
    queryFn: getClashInfo,
  })

  const patchInfo = useLockFn(async (patch: ClashInfoPatch) => {
    if (!hasClashInfoPayload(patch)) return

    validatePorts(patch)

    await patchClashConfig(patch)
    mutateInfo()
    queryClient.invalidateQueries({ queryKey: ['getClashConfig'] })
  })

  return {
    clashInfo,
    mutateInfo,
    patchInfo,
  }
}
