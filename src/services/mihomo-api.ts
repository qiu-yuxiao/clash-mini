import {
  getProxies,
  getProxyProviders,
  getProxyByName,
  selectNodeForGroup,
  getConnections,
  closeConnection,
  closeAllConnections,
  getVersion,
  getRules,
  getRuleProviders,
  getBaseConfig,
} from 'tauri-plugin-mihomo-api'

import { withIpcTimeout } from './cmds'

const MIHO_API_TIMEOUT = 15000

export const getProxiesWithTimeout = () =>
  withIpcTimeout(getProxies(), MIHO_API_TIMEOUT, 'mihomo:getProxies')

export const getProxyProvidersWithTimeout = () =>
  withIpcTimeout(getProxyProviders(), MIHO_API_TIMEOUT, 'mihomo:getProxyProviders')

export const getProxyByNameWithTimeout = (name: string) =>
  withIpcTimeout(getProxyByName(name), MIHO_API_TIMEOUT, 'mihomo:getProxyByName')

export const selectNodeForGroupWithTimeout = (group: string, node: string) =>
  withIpcTimeout(selectNodeForGroup(group, node), MIHO_API_TIMEOUT, 'mihomo:selectNodeForGroup')

export const getConnectionsWithTimeout = () =>
  withIpcTimeout(getConnections(), MIHO_API_TIMEOUT, 'mihomo:getConnections')

export const closeConnectionWithTimeout = (id: string) =>
  withIpcTimeout(closeConnection(id), MIHO_API_TIMEOUT, 'mihomo:closeConnection')

export const closeAllConnectionsWithTimeout = () =>
  withIpcTimeout(closeAllConnections(), MIHO_API_TIMEOUT, 'mihomo:closeAllConnections')

export const getVersionWithTimeout = () =>
  withIpcTimeout(getVersion(), MIHO_API_TIMEOUT, 'mihomo:getVersion')

export const getRulesWithTimeout = () =>
  withIpcTimeout(getRules(), MIHO_API_TIMEOUT, 'mihomo:getRules')

export const getRuleProvidersWithTimeout = () =>
  withIpcTimeout(getRuleProviders(), MIHO_API_TIMEOUT, 'mihomo:getRuleProviders')

export const getBaseConfigWithTimeout = () =>
  withIpcTimeout(getBaseConfig(), MIHO_API_TIMEOUT, 'mihomo:getBaseConfig')