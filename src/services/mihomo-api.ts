import { shouldCloseStaleProxyConnection } from '@/utils/path-control'
import {
  getProxyByName,
  selectNodeForGroup,
  getConnections,
  closeConnection,
  closeAllConnections,
  getVersion,
  getBaseConfig,
} from 'tauri-plugin-mihomo-api'

import { withIpcTimeout } from './cmds'

const MIHO_API_TIMEOUT = 15000

export const getProxyByNameWithTimeout = (name: string) =>
  withIpcTimeout(
    getProxyByName(name),
    MIHO_API_TIMEOUT,
    'mihomo:getProxyByName',
  )

export const selectNodeForGroupWithTimeout = (group: string, node: string) =>
  withIpcTimeout(
    selectNodeForGroup(group, node),
    MIHO_API_TIMEOUT,
    'mihomo:selectNodeForGroup',
  )

export const getConnectionsWithTimeout = () =>
  withIpcTimeout(getConnections(), MIHO_API_TIMEOUT, 'mihomo:getConnections')

export const closeConnectionWithTimeout = (id: string) =>
  withIpcTimeout(
    closeConnection(id),
    MIHO_API_TIMEOUT,
    'mihomo:closeConnection',
  )

export const closeAllConnectionsWithTimeout = () =>
  withIpcTimeout(
    closeAllConnections(),
    MIHO_API_TIMEOUT,
    'mihomo:closeAllConnections',
  )

/**
 * 前台用户显式切换节点至 `targetNode` 后，精准关闭仍挂在非 `targetNode` 旧出口上的 `PROXY` 存量长连接（BUG-396）。
 * 直连（`DIRECT`）与已位于 `targetNode` 上的连接完全不受影响。
 */
export async function closeStaleProxyConnections(
  targetNode: string,
  groupName: string = 'PROXY',
): Promise<number> {
  try {
    const { connections } = await getConnectionsWithTimeout()
    const staleConns = (connections ?? []).filter((conn) =>
      shouldCloseStaleProxyConnection(conn.chains ?? [], targetNode, groupName),
    )
    if (staleConns.length === 0) return 0
    await Promise.allSettled(
      staleConns.map((conn) => closeConnectionWithTimeout(conn.id)),
    )
    return staleConns.length
  } catch (err) {
    console.warn('[MihomoApi] closeStaleProxyConnections failed:', err)
    return 0
  }
}

export const getVersionWithTimeout = () =>
  withIpcTimeout(getVersion(), MIHO_API_TIMEOUT, 'mihomo:getVersion')

export const getBaseConfigWithTimeout = () =>
  withIpcTimeout(getBaseConfig(), MIHO_API_TIMEOUT, 'mihomo:getBaseConfig')
