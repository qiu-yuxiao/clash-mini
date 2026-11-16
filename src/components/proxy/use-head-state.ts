import { invoke } from '@tauri-apps/api/core'
import { useCallback, useEffect, useSyncExternalStore } from 'react'

import { useProfiles } from '@/hooks/use-profiles'

import { ProxySortType } from './use-filter-sort'

export interface HeadState {
  open?: boolean
  showType: boolean
  sortType: ProxySortType
  filterText: string
}

type HeadStateStorage = Record<string, Record<string, HeadState>>

const HEAD_STATE_KEY = 'proxy-head-state'
const EMPTY_PROFILE_STATE: Record<string, HeadState> = {}

export const DEFAULT_STATE: HeadState = {
  open: false,
  showType: true,
  sortType: 1,
  filterText: '',
}

let headStateStore: HeadStateStorage = {}
let initializedSync = false
let backendLoadStarted = false
let persistTimer: ReturnType<typeof setTimeout> | null = null
const dirtyKeys = new Set<string>()
const listeners = new Set<() => void>()

function readLocalStorageStore(): HeadStateStorage | null {
  try {
    if (typeof localStorage === 'undefined') return null
    const raw = localStorage.getItem(HEAD_STATE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as unknown
    return parsed && typeof parsed === 'object'
      ? (parsed as HeadStateStorage)
      : null
  } catch {
    return null
  }
}

function writeLocalStorageStore(data: HeadStateStorage): void {
  try {
    if (typeof localStorage === 'undefined') return
    localStorage.setItem(HEAD_STATE_KEY, JSON.stringify(data))
  } catch {}
}

function ensureInitializedSync(): void {
  if (initializedSync) return
  initializedSync = true
  const local = readLocalStorageStore()
  if (local) {
    headStateStore = local
  }
}

function notifyListeners(): void {
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void): () => void {
  ensureInitializedSync()
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function getSnapshot(): HeadStateStorage {
  ensureInitializedSync()
  return headStateStore
}

async function persistHeadStateNow(): Promise<void> {
  try {
    await invoke('save_proxy_head_state', { state: headStateStore })
  } catch (err) {
    console.warn('[useHeadState] 保存 head_state 到后端失败:', err)
  }
}

function scheduleBackendPersist(): void {
  if (persistTimer !== null) {
    clearTimeout(persistTimer)
  }
  persistTimer = setTimeout(() => {
    persistTimer = null
    void persistHeadStateNow()
  }, 100)
}

/**
 * 若存在待落盘的 head_state 写入（100ms 防抖窗口内），立即冲刷至后端，
 * 确保紧随其后的自动选点（Enter / F1 / F4）读取到最新的 filterText。
 */
export async function flushHeadStatePersist(): Promise<void> {
  if (persistTimer === null) return
  clearTimeout(persistTimer)
  persistTimer = null
  await persistHeadStateNow()
}

function mergeBackendData(remote: HeadStateStorage): boolean {
  let changed = false
  const nextStore: HeadStateStorage = { ...headStateStore }
  for (const [profileUid, groups] of Object.entries(remote)) {
    if (!groups || typeof groups !== 'object') continue
    const existingProfile = nextStore[profileUid] || {}
    let mergedProfile: Record<string, HeadState> | null = null
    for (const [groupName, groupState] of Object.entries(groups)) {
      if (!groupState || typeof groupState !== 'object') continue
      if (dirtyKeys.has(`${profileUid}::${groupName}`)) continue
      if (!mergedProfile) {
        mergedProfile = { ...existingProfile }
      }
      mergedProfile[groupName] = { ...DEFAULT_STATE, ...groupState }
      changed = true
    }
    if (mergedProfile) {
      nextStore[profileUid] = mergedProfile
    }
  }
  if (changed) {
    headStateStore = nextStore
    writeLocalStorageStore(headStateStore)
  }
  return changed
}

function ensureBackendLoaded(): void {
  ensureInitializedSync()
  if (backendLoadStarted) return
  backendLoadStarted = true
  invoke<HeadStateStorage>('get_proxy_head_state')
    .then((data) => {
      if (data && typeof data === 'object' && mergeBackendData(data)) {
        notifyListeners()
      }
    })
    .catch((err) => {
      console.warn(
        '[useHeadState] 从后端加载状态失败，继续使用 localStorage 缓存:',
        err,
      )
    })
}

export function updateHeadState(
  profileUid: string,
  groupName: string,
  patch: Partial<HeadState>,
): void {
  if (!profileUid) return
  ensureInitializedSync()
  const profileState = headStateStore[profileUid] || {}
  const prev = profileState[groupName] || DEFAULT_STATE
  const next: HeadState = { ...prev, ...patch }

  if (
    prev.open === next.open &&
    prev.showType === next.showType &&
    prev.sortType === next.sortType &&
    prev.filterText === next.filterText
  ) {
    return
  }

  dirtyKeys.add(`${profileUid}::${groupName}`)
  headStateStore = {
    ...headStateStore,
    [profileUid]: {
      ...profileState,
      [groupName]: next,
    },
  }
  writeLocalStorageStore(headStateStore)
  scheduleBackendPersist()
  notifyListeners()
}

export function getGroupHeadState(
  profileUid: string,
  groupName = 'PROXY',
): HeadState {
  ensureInitializedSync()
  if (!profileUid) return DEFAULT_STATE
  return headStateStore[profileUid]?.[groupName] ?? DEFAULT_STATE
}

/** 测试辅助：重置全局单例存储状态（仅用于 Vitest 单元测试） */
export function resetHeadStateStoreForTest(initial?: HeadStateStorage): void {
  if (persistTimer !== null) {
    clearTimeout(persistTimer)
    persistTimer = null
  }
  headStateStore = initial ?? {}
  initializedSync = true
  backendLoadStarted = false
  dirtyKeys.clear()
  notifyListeners()
}

export function useHeadStateNew() {
  const { profiles } = useProfiles()
  const current = profiles?.current || ''
  const state = useSyncExternalStore(subscribe, getSnapshot, getSnapshot)

  useEffect(() => {
    ensureBackendLoaded()
  }, [])

  const setHeadState = useCallback(
    (groupName: string, obj: Partial<HeadState>) => {
      if (!current) return
      updateHeadState(current, groupName, obj)
    },
    [current],
  )

  const currentProfileState = state[current] || EMPTY_PROFILE_STATE
  return [currentProfileState, setHeadState] as const
}
