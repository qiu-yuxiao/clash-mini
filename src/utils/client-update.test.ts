import type { DownloadEvent } from '@tauri-apps/plugin-updater'
import { describe, expect, it, vi } from 'vitest'

import {
  checkClientUpdateWithFallback,
  type ClientUpdateHandle,
  downloadAndInstallClientUpdateWithRetry,
  resolveLocalMixedProxyUrl,
} from './client-update'

describe('BUG-417: 客户端在线升级多路检查、断流重试与错误落盘', () => {
  it('resolveLocalMixedProxyUrl 按 verge_mixed_port > clashInfo.mixed_port > 10801 优先级解析且兜底非法端口', () => {
    expect(resolveLocalMixedProxyUrl(17890, 10801)).toBe(
      'http://127.0.0.1:17890',
    )
    expect(resolveLocalMixedProxyUrl(undefined, 10802)).toBe(
      'http://127.0.0.1:10802',
    )
    expect(resolveLocalMixedProxyUrl(null, null)).toBe('http://127.0.0.1:10801')
    expect(resolveLocalMixedProxyUrl(0, 99999)).toBe('http://127.0.0.1:10801')
  })

  it('checkClientUpdateWithFallback 本地代理失败后自动记录日志并回退直连/系统代理通道', async () => {
    const fakeUpdate: ClientUpdateHandle = {
      version: '3.6.2',
      downloadAndInstall: vi.fn(async () => {}),
    }
    const calls: Array<Record<string, unknown> | undefined> = []
    const logs: string[] = []
    const sleepFn = vi.fn(async () => {})

    const res = await checkClientUpdateWithFallback({
      localProxyUrl: 'http://127.0.0.1:10801',
      checkFn: async (opts) => {
        calls.push(opts as Record<string, unknown> | undefined)
        if (calls.length === 1) {
          throw new Error('proxy timeout')
        }
        return fakeUpdate
      },
      logError: (msg) => logs.push(msg),
      sleepFn,
    })

    expect(res).toBe(fakeUpdate)
    expect(calls).toHaveLength(2)
    expect(calls[0]).toMatchObject({ proxy: 'http://127.0.0.1:10801' })
    expect(calls[1]).not.toHaveProperty('proxy')
    expect(sleepFn).toHaveBeenCalledTimes(1)
    expect(logs).toHaveLength(1)
    expect(logs[0]).toContain('http://127.0.0.1:10801')
    expect(logs[0]).toContain('proxy timeout')
  })

  it('downloadAndInstallClientUpdateWithRetry 在 Finished 前断流时自动重试并在第 3 次切换备用句柄成功', async () => {
    const logs: string[] = []
    const events: DownloadEvent[] = []
    const resets: Array<[number, number]> = []
    const sleepFn = vi.fn(async () => {})

    let primaryCalls = 0
    const initialUpdate: ClientUpdateHandle = {
      version: '3.6.2',
      downloadAndInstall: vi.fn(async (onEvent) => {
        primaryCalls += 1
        onEvent?.({ event: 'Started', data: { contentLength: 30_000_000 } })
        onEvent?.({ event: 'Progress', data: { chunkLength: 2_503_448 } })
        throw new Error(`stream aborted attempt ${primaryCalls}`)
      }),
    }

    const directUpdate: ClientUpdateHandle = {
      version: '3.6.2',
      downloadAndInstall: vi.fn(async (onEvent) => {
        onEvent?.({ event: 'Started', data: { contentLength: 30_000_000 } })
        onEvent?.({ event: 'Progress', data: { chunkLength: 30_000_000 } })
        onEvent?.({ event: 'Finished' })
      }),
    }

    await downloadAndInstallClientUpdateWithRetry({
      initialUpdate,
      onAttemptReset: (attempt, max) => resets.push([attempt, max]),
      onEvent: (ev) => events.push(ev),
      recheckDirectFn: async () => directUpdate,
      logError: (msg) => logs.push(msg),
      sleepFn,
    })

    expect(initialUpdate.downloadAndInstall).toHaveBeenCalledTimes(2)
    expect(directUpdate.downloadAndInstall).toHaveBeenCalledTimes(1)
    expect(resets).toEqual([
      [2, 3],
      [3, 3],
    ])
    expect(sleepFn).toHaveBeenCalledTimes(2)
    expect(logs).toHaveLength(2)
    expect(events.some((e) => e.event === 'Finished')).toBe(true)
  })

  it('Finished 触发后的校验或启动安装阶段报错绝不重复触发第 2 次安装', async () => {
    const logs: string[] = []
    const sleepFn = vi.fn(async () => {})
    const recheckDirectFn = vi.fn(async () => null)

    const initialUpdate: ClientUpdateHandle = {
      version: '3.6.2',
      downloadAndInstall: vi.fn(async (onEvent) => {
        onEvent?.({ event: 'Started', data: { contentLength: 100 } })
        onEvent?.({ event: 'Finished' })
        throw new Error('signature verification failed')
      }),
    }

    await expect(
      downloadAndInstallClientUpdateWithRetry({
        initialUpdate,
        onEvent: () => {},
        recheckDirectFn,
        logError: (msg) => logs.push(msg),
        sleepFn,
      }),
    ).rejects.toThrow('signature verification failed')

    expect(initialUpdate.downloadAndInstall).toHaveBeenCalledTimes(1)
    expect(sleepFn).not.toHaveBeenCalled()
    expect(recheckDirectFn).not.toHaveBeenCalled()
    expect(logs).toHaveLength(1)
    expect(logs[0]).toContain('不自动重试以防重复弹窗')
  })
})
