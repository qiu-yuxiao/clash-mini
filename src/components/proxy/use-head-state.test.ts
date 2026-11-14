import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invokeMock, currentProfile } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  currentProfile: { uid: 'profile-1' },
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}))

vi.mock('@/hooks/use-profiles', () => ({
  useProfiles: () => ({
    profiles: { current: currentProfile.uid },
  }),
}))

import {
  DEFAULT_STATE,
  getGroupHeadState,
  resetHeadStateStoreForTest,
  updateHeadState,
} from './use-head-state'

describe('use-head-state 单例全局存储 (BUG-369)', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    invokeMock.mockResolvedValue({})
    resetHeadStateStoreForTest()
  })

  it('不同调用点分别更新 filterText 与 sortType 时互不覆盖且立即可被 getGroupHeadState 读取', () => {
    updateHeadState('profile-1', 'PROXY', { filterText: '-香港' })
    updateHeadState('profile-1', 'PROXY', { sortType: 2 })

    const state = getGroupHeadState('profile-1', 'PROXY')
    expect(state).toEqual({
      ...DEFAULT_STATE,
      filterText: '-香港',
      sortType: 2,
    })
  })

  it('无实质变更的 no-op 更新不会覆写或触发多余保存', async () => {
    vi.useFakeTimers()
    try {
      updateHeadState('profile-1', 'PROXY', {
        filterText: '-香港',
        sortType: 1,
      })
      await vi.advanceTimersByTimeAsync(150)
      expect(invokeMock).toHaveBeenCalledTimes(1)

      // 重复写入相同 sortType=1 不应覆盖 filterText 或再次触发保存
      updateHeadState('profile-1', 'PROXY', { sortType: 1 })
      await vi.advanceTimersByTimeAsync(150)
      expect(invokeMock).toHaveBeenCalledTimes(1)
      expect(getGroupHeadState('profile-1', 'PROXY').filterText).toBe('-香港')
    } finally {
      vi.useRealTimers()
    }
  })
})
