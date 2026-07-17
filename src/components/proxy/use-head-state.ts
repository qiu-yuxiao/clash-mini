import { invoke } from '@tauri-apps/api/core'
import { useCallback, useEffect, useReducer } from 'react'

import { useProfiles } from '@/hooks/use-profiles'

import { ProxySortType } from './use-filter-sort'

export interface HeadState {
  open?: boolean
  showType: boolean
  sortType: ProxySortType
  filterText: string
  textState: 'url' | 'filter' | null
  testUrl: string
}

type HeadStateStorage = Record<string, Record<string, HeadState>>

const HEAD_STATE_KEY = 'proxy-head-state'
export const DEFAULT_STATE: HeadState = {
  open: false,
  showType: true,
  sortType: 1,
  filterText: '',
  textState: null,
  testUrl: '',
}

type HeadStateAction =
  | { type: 'reset' }
  | { type: 'replace'; payload: HeadStateStorage }
  | {
      type: 'update'
      profileUid: string
      groupName: string
      patch: Partial<HeadState>
    }

// use-head-state-reducer
function headStateReducer(
  state: HeadStateStorage,
  action: HeadStateAction,
): HeadStateStorage {
  switch (action.type) {
    case 'reset':
      return {}
    case 'replace':
      return action.payload
    case 'update': {
      const { profileUid, groupName, patch } = action
      const profileState = state[profileUid] || {}
      const prev = profileState[groupName] || DEFAULT_STATE
      return {
        ...state,
        [profileUid]: {
          ...profileState,
          [groupName]: { ...prev, ...patch },
        },
      }
    }
    default:
      return state
  }
}

export function useHeadStateNew() {
  const { profiles } = useProfiles()
  const current = profiles?.current || ''

  const [state, dispatch] = useReducer(headStateReducer, {})

  // 1. Load entire storage once on mount
  useEffect(() => {
    invoke<HeadStateStorage>('get_proxy_head_state')
      .then((data) => {
        if (data && typeof data === 'object') {
          dispatch({ type: 'replace', payload: data })
          localStorage.setItem(HEAD_STATE_KEY, JSON.stringify(data))
        }
      })
      .catch((err) => {
        console.warn('[useHeadState] 从后端加载状态失败，尝试 localStorage 兜底:', err)
        try {
          const data = JSON.parse(
            localStorage.getItem(HEAD_STATE_KEY) ?? 'null',
          ) as HeadStateStorage
          if (data && typeof data === 'object') {
            dispatch({ type: 'replace', payload: data })
          }
        } catch (e) {
          console.warn('[useHeadState] localStorage 兜底也失败，状态初始化为空:', e)
        }
      })
  }, [])

  // 2. Save entire storage only when state updates
  useEffect(() => {
    if (Object.keys(state).length === 0) return
    const timer = setTimeout(async () => {
      try {
        localStorage.setItem(HEAD_STATE_KEY, JSON.stringify(state))
        await invoke('save_proxy_head_state', { state })
      } catch (err) {
        console.warn('[useHeadState] 保存 head_state 到后端失败:', err)
      }
    }, 100)
    return () => clearTimeout(timer)
  }, [state])

  const setHeadState = useCallback(
    (groupName: string, obj: Partial<HeadState>) => {
      if (!current) return
      dispatch({ type: 'update', profileUid: current, groupName, patch: obj })
    },
    [current],
  )

  const currentProfileState = state[current] || {}
  return [currentProfileState, setHeadState] as const
}
