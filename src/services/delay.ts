import type { IProxyItem } from '@/types/clash'
import { debugLog } from '@/utils/debug'
import { delayProxyByName, ProxyDelay } from 'tauri-plugin-mihomo-api'

const hashKey = (name: string, group: string) => `${group ?? ''}::${name}`

export interface DelayUpdate {
  delay: number
  elapsed?: number
  updatedAt: number
}

const CACHE_TTL = 30 * 60 * 1000

// 节点延迟判定阈值：与后端 src-tauri/src/module/monitor.rs 的
// NODE_DELAY_MIN_MS / NODE_DELAY_MAX_MS 保持一致，作为单一真源，严禁各自漂移。
// - 下限 30ms：过滤机场伪造的超低延迟广告/假节点
// - 上限 2000ms：超过即视为死节点，不参与自动选点
//   探针超时现已与死节点阈值统一为 2000ms（checkListDelay 的 timeout 与后端 NODE_TEST_TIMEOUT_MS
//   均取 NODE_DELAY_MAX_MS），超过即按不可用处理，UI 显示 Error/Timeout，不再展示真实延迟。
//   此处阈值同时决定「探针超时」与「候选/选点」资格，前后端语义完全一致。
export const NODE_DELAY_MIN_MS = 30
export const NODE_DELAY_MAX_MS = 2000
export const INTERNAL_CONTROL_TIMEOUT_MS = 3000

class DelayManager {
  private cache = new Map<string, DelayUpdate>()
  private urlMap = new Map<string, string>()

  // 每个节点的监听
  private listenerMap = new Map<string, (update: DelayUpdate) => void>()

  // 每个分组的监听
  private groupListenerMap = new Map<string, () => void>()

  private pendingItemUpdates = new Map<string, DelayUpdate[]>()
  private pendingGroupUpdates = new Set<string>()
  private itemFlushScheduled = false
  private groupFlushScheduled = false

  /** 批量测速进行中标志，现已废弃，始终返回 false */
  get isBatchTesting(): boolean {
    return false
  }

  constructor() {
    if (typeof window !== 'undefined') {
      setInterval(
        () => {
          const now = Date.now()
          const expiredKeys: string[] = []
          this.cache.forEach((entry, key) => {
            if (now - entry.updatedAt > CACHE_TTL) {
              expiredKeys.push(key)
            }
          })
          expiredKeys.forEach((key) => this.cache.delete(key))
        },
        2 * 60 * 60 * 1000,
      ) // Clean up expired cache every 2 hours
    }
  }

  private scheduleOnNextFrame(run: () => void): void {
    if (typeof window !== 'undefined') {
      if (typeof window.requestAnimationFrame === 'function') {
        window.requestAnimationFrame(() => run())
        return
      }
    }

    Promise.resolve().then(run)
  }

  private scheduleItemFlush() {
    if (this.itemFlushScheduled) return
    this.itemFlushScheduled = true

    this.scheduleOnNextFrame(() => {
      this.itemFlushScheduled = false
      const updates = this.pendingItemUpdates
      this.pendingItemUpdates = new Map()

      updates.forEach((queue, key) => {
        const listener = this.listenerMap.get(key)
        if (!listener) return

        queue.forEach((update) => {
          try {
            listener(update)
          } catch (error) {
            console.error(
              `[DelayManager] 通知节点延迟监听器失败: ${key}`,
              error,
            )
          }
        })
      })

      // React rendering during this flush may drop new updates
      // into the swapped-in Map. Re-schedule if any were missed.
      if (this.pendingItemUpdates.size > 0) {
        this.scheduleItemFlush()
      }
    })
  }

  private scheduleGroupFlush() {
    if (this.groupFlushScheduled) return
    this.groupFlushScheduled = true

    this.scheduleOnNextFrame(() => {
      this.groupFlushScheduled = false
      const groups = this.pendingGroupUpdates
      this.pendingGroupUpdates = new Set()

      groups.forEach((group) => {
        const listener = this.groupListenerMap.get(group)
        if (!listener) return
        try {
          listener()
        } catch (error) {
          console.error(
            `[DelayManager] 通知分组延迟监听器失败: ${group}`,
            error,
          )
        }
      })

      if (this.pendingGroupUpdates.size > 0) {
        this.scheduleGroupFlush()
      }
    })
  }

  /** 触发组级通知，驱动 useRenderList 等组件重排 */
  queueGroupNotification(group: string) {
    this.pendingGroupUpdates.add(group)
    this.scheduleGroupFlush()
  }

  setUrl(group: string, url: string) {
    debugLog(`[DelayManager] 设置测试URL，组: ${group}, URL: ${url}`)
    this.urlMap.set(group, url)
  }

  getUrl(group: string) {
    const url = this.urlMap.get(group)
    debugLog(
      `[DelayManager] 获取测试URL，组: ${group}, URL: ${url || '未设置'}`,
    )
    // 如果未设置URL，返回默认URL
    return url || 'http://cp.cloudflare.com/generate_204'
  }

  setListener(
    name: string,
    group: string,
    listener: (update: DelayUpdate) => void,
  ) {
    const key = hashKey(name, group)
    this.listenerMap.set(key, listener)
  }

  removeListener(name: string, group: string) {
    const key = hashKey(name, group)
    this.listenerMap.delete(key)
  }

  setGroupListener(group: string, listener: () => void) {
    this.groupListenerMap.set(group, listener)
  }

  removeGroupListener(group: string) {
    this.groupListenerMap.delete(group)
  }

  setDelay(
    name: string,
    group: string,
    delay: number,
    meta?: { elapsed?: number },
  ): DelayUpdate {
    const key = hashKey(name, group)
    debugLog(
      `[DelayManager] 设置延迟，代理: ${name}, 组: ${group}, 延迟: ${delay}`,
    )
    const update: DelayUpdate = {
      delay,
      elapsed: meta?.elapsed,
      updatedAt: Date.now(),
    }

    this.cache.set(key, update)

    const queue = this.pendingItemUpdates.get(key)
    if (queue) {
      queue.push(update)
    } else {
      this.pendingItemUpdates.set(key, [update])
    }
    this.scheduleItemFlush()

    return update
  }

  getDelayUpdate(name: string, group: string) {
    const key = hashKey(name, group)
    const entry = this.cache.get(key)
    if (!entry) return undefined

    if (Date.now() - entry.updatedAt > CACHE_TTL) {
      this.cache.delete(key)
      return undefined
    }

    return { ...entry }
  }

  getDelay(name: string, group: string) {
    const update = this.getDelayUpdate(name, group)
    return update ? update.delay : -1
  }

  getDelayFix(proxy: IProxyItem, group: string) {
    if (!proxy) return -1
    const update = this.getDelayUpdate(proxy.name, group)
    if (update && (update.delay >= 0 || update.delay === -2)) {
      return update.delay
    }

    // 添加 history 属性的安全检查
    if (proxy.history && proxy.history.length > 0) {
      return proxy.history[proxy.history.length - 1].delay ?? -1
    }
    return -1
  }

  async checkDelay(
    name: string,
    group: string,
    timeout: number,
  ): Promise<DelayUpdate> {
    debugLog(
      `[DelayManager] 开始测试延迟，代理: ${name}, 组: ${group}, 超时: ${timeout}ms`,
    )

    // 先将状态设置为测试中
    this.setDelay(name, group, -2)

    const startTime = Date.now()

    try {
      const url = this.getUrl(group)
      debugLog(`[DelayManager] 调用API测试延迟，代理: ${name}, URL: ${url}`)

      let raceFinished = false
      let timerId: ReturnType<typeof setTimeout> | null = null

      // 设置超时处理, delay = 0 为超时
      const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
        timerId = setTimeout(() => {
          if (!raceFinished) {
            resolve({ delay: 0 })
          }
        }, timeout)
      })

      // 使用Promise.race来实现超时控制
      const result = await Promise.race([
        delayProxyByName(name, url, timeout)
          .then((res) => {
            raceFinished = true
            if (timerId) {
              clearTimeout(timerId)
              timerId = null
            }
            return res
          })
          .catch((err) => {
            if (timerId) {
              clearTimeout(timerId)
              timerId = null
            }
            raceFinished = true
            console.error(
              `[DelayManager] delayProxyByName error for ${name}:`,
              err,
            )
            return { delay: 1e6 }
          }),
        timeoutPromise.then((res) => {
          raceFinished = true
          return res
        }),
      ])

      // 确保至少显示500ms的加载动画
      const elapsedTime = Date.now() - startTime
      if (elapsedTime < 500) {
        await new Promise((resolve) => setTimeout(resolve, 500 - elapsedTime))
      }

      const delay = result.delay
      const elapsed = elapsedTime
      debugLog(`[DelayManager] 延迟测试完成，代理: ${name}, 结果: ${delay}ms`)

      return this.setDelay(name, group, delay, { elapsed })
    } catch (error) {
      // 确保至少显示500ms的加载动画
      await new Promise((resolve) => setTimeout(resolve, 500))
      console.error(`[DelayManager] 延迟测试出错，代理: ${name}`, error)
      const delay = 1e6 // error
      const elapsed = Date.now() - startTime

      return this.setDelay(name, group, delay, { elapsed })
    }
  }


  /**
   * 注入后台批量测速结果到缓存，并通知 UI 刷新
   * 由 Tauri 事件 verge://backend-delay-results 驱动
   */
  injectBatchResults(group: string, results: Array<[string, number]>) {
    debugLog(
      `[DelayManager] 注入后台测速结果，组: ${group}, 数量: ${results.length}`,
    )
    const now = Date.now()
    for (const [name, delay] of results) {
      this.setDelay(name, group, delay, { elapsed: Date.now() - now })
    }
    this.queueGroupNotification(group)
  }

  formatDelay(delay: number, timeout = NODE_DELAY_MAX_MS) {
    if (delay === -1) return '-'
    if (delay === -2) return 'testing'
    if (delay === 0 || (delay >= timeout && delay <= 1e5)) return 'Timeout'
    if (delay > 1e5) return 'Error'
    return `${delay}`
  }

  formatDelayColor(delay: number, timeout = NODE_DELAY_MAX_MS, isDarkMode = false) {
    if (delay < 0) return ''
    if (delay === 0 || delay >= timeout) return 'error.main'
    if (delay >= 400) return isDarkMode ? 'warning.main' : 'warning.dark'
    if (delay >= 250) return 'primary.main'
    return 'success.main'
  }
}

export default new DelayManager()
