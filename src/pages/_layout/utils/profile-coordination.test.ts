import { beforeEach, describe, expect, it, vi } from 'vitest'

import { hasUsablePersistedExit } from '@/pages/_layout/utils/profile-coordination'
import { getProxyByNameWithTimeout } from '@/services/mihomo-api'

vi.mock('@/services/mihomo-api', () => ({
  getProxyByNameWithTimeout: vi.fn(),
}))

describe('最高方针「连续性优先」：hasUsablePersistedExit 决定初始化选点是否允许切点', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('持久出口有效且仍在过滤子集内 → 保持现状（禁止切点）', async () => {
    vi.mocked(getProxyByNameWithTimeout).mockResolvedValue({
      now: '日本 01',
      all: ['日本 01', '日本 02'],
    } as never)

    expect(await hasUsablePersistedExit(['日本 01', '日本 02'])).toBe(true)
  })

  it('出口为空（无 now）→ 允许重选，保证能建立可用出口', async () => {
    vi.mocked(getProxyByNameWithTimeout).mockResolvedValue({
      now: '',
      all: ['日本 01'],
    } as never)

    expect(await hasUsablePersistedExit(['日本 01'])).toBe(false)
  })

  it('持久出口已不在新配置 / 当前过滤子集内 → 允许重选', async () => {
    vi.mocked(getProxyByNameWithTimeout).mockResolvedValue({
      now: '新加坡 01',
      all: ['日本 01', '日本 02'],
    } as never)

    expect(await hasUsablePersistedExit(['日本 01', '日本 02'])).toBe(false)
  })

  it('持久出口是广告伪节点 → 允许重选', async () => {
    vi.mocked(getProxyByNameWithTimeout).mockResolvedValue({
      now: 'https://promo.example.com',
      all: ['https://promo.example.com', '日本 01'],
    } as never)

    expect(
      await hasUsablePersistedExit(['https://promo.example.com', '日本 01']),
    ).toBe(false)
  })

  it('读取组信息异常 → 保守允许重选（fail-open，避免退化成无出口）', async () => {
    vi.mocked(getProxyByNameWithTimeout).mockRejectedValue(
      new Error('ipc timeout'),
    )

    expect(await hasUsablePersistedExit(['日本 01'])).toBe(false)
  })
})
