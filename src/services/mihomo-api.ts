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

export const getVersionWithTimeout = () =>
  withIpcTimeout(getVersion(), MIHO_API_TIMEOUT, 'mihomo:getVersion')

export const getBaseConfigWithTimeout = () =>
  withIpcTimeout(getBaseConfig(), MIHO_API_TIMEOUT, 'mihomo:getBaseConfig')
