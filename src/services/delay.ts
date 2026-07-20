import { delayProxyByNameWithTimeout } from '@/services/mihomo-api'
import type { IProxyItem } from '@/types/clash'
import { debugLog } from '@/utils/debug'
import { ProxyDelay } from 'tauri-plugin-mihomo-api'

const hashKey = (name: string, group?: string) => `${group || 'PROXY'}::${name}`

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

// 解锁流媒体 region 探测超时：HTTP 经代理访问外网，可能较慢，需较长超时（还原 15s）
export const UNLOCK_TIMEOUT_MS = 15000

// 启动后系统态探活宽限：服务初始化可能耗时，宽限期内保持快轮询（还原 10s）
export const STARTUP_GRACE_MS = 10000

class DelayManager {
  private cache = new Map<string, DelayUpdate>()
  // L-22: urlMap 用于存储组测试 URL，组数量通常有限（<100），内存泄漏影响极小
  // 提供 clearUrlMap 方法供 profile 切换时手动调用清理
  private urlMap = new Map<string, string>()

  // 每个节点的监听
  private listenerMap = new Map<string, (update: DelayUpdate) => void>()

  // 每个分组的监听
  private groupListenerMap = new Map<string, Array<() => void>>()

  private pendingItemUpdates = new Map<string, DelayUpdate[]>()
  private pendingGroupUpdates = new Set<string>()
  private itemFlushScheduled = false
  private groupFlushScheduled = false

  // M2-09: 保存 interval ID，HMR 场景下可清理避免累积
  private cleanupIntervalId: ReturnType<typeof setInterval> | null = null

  constructor() {
    if (typeof window !== 'undefined') {
      this.cleanupIntervalId = setInterval(
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
        5 * 60 * 1000,
      ) // Clean up expired cache every 5 minutes
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

  // L-23: 如果在 flush 前 listener 被移除，对应的 pending updates 会被丢弃。
  // 影响很小：丢弃的是延迟更新通知，组件重新挂载后会重新获取最新延迟，不会导致数据不一致。
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
        const listeners = this.groupListenerMap.get(group)
        if (!listeners || listeners.length === 0) return
        listeners.forEach((listener) => {
          try {
            listener()
          } catch (error) {
            console.error(
              `[DelayManager] 通知分组延迟监听器失败: ${group}`,
              error,
            )
          }
        })
      })

      if (this.pendingGroupUpdates.size > 0) {
        this.scheduleGroupFlush()
      }
    })
  }

  /** 触发组级通知，驱动 useRenderList 等组件重排 */
  queueGroupNotification(group = 'PROXY') {
    this.pendingGroupUpdates.add(group)
    this.scheduleGroupFlush()
  }

  setUrl(group = 'PROXY', url: string) {
    debugLog(`[DelayManager] 设置测试URL，组: ${group}, URL: ${url}`)
    this.urlMap.set(group, url)
  }

  private getUrl(group = 'PROXY') {
    const url = this.urlMap.get(group)
    debugLog(
      `[DelayManager] 获取测试URL，组: ${group}, URL: ${url || '未设置'}`,
    )
    // 如果未设置URL，返回默认URL
    return url || 'http://cp.cloudflare.com/generate_204'
  }

  /** 清空所有组的测试 URL 缓存，用于 profile 切换时清理 */
  clearUrlMap() {
    this.urlMap.clear()
    debugLog('[DelayManager] 已清空 urlMap 缓存')
  }

  /** 清空节点延迟值缓存，用于 Profile 切换时避免同名节点复用旧 Profile 的延迟数据 */
  clearCache() {
    this.cache.clear()
    debugLog('[DelayManager] 已清空延迟缓存')
  }

  setListener(
    name: string,
    group = 'PROXY',
    listener: (update: DelayUpdate) => void,
  ) {
    const key = hashKey(name, group)
    this.listenerMap.set(key, listener)
  }

  removeListener(name: string, group = 'PROXY') {
    const key = hashKey(name, group)
    this.listenerMap.delete(key)
  }

  setGroupListener(group = 'PROXY', listener: () => void) {
    const listeners = this.groupListenerMap.get(group) || []
    if (!listeners.includes(listener)) {
      listeners.push(listener)
      this.groupListenerMap.set(group, listeners)
    }
  }

  removeGroupListener(group = 'PROXY', listener?: () => void) {
    if (!listener) {
      this.groupListenerMap.delete(group)
      return
    }
    const listeners = this.groupListenerMap.get(group)
    if (listeners) {
      const filtered = listeners.filter((l) => l !== listener)
      if (filtered.length === 0) {
        this.groupListenerMap.delete(group)
      } else {
        this.groupListenerMap.set(group, filtered)
      }
    }
  }

  setDelay(
    name: string,
    group = 'PROXY',
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

  getDelayUpdate(name: string, group = 'PROXY') {
    const key = hashKey(name, group)
    const entry = this.cache.get(key)
    if (!entry) return undefined

    if (Date.now() - entry.updatedAt > CACHE_TTL) {
      return undefined
    }

    return { ...entry }
  }

  getDelayFix(proxy: IProxyItem, group = 'PROXY') {
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
    group = 'PROXY',
    timeout: number,
    signal?: AbortSignal,
  ): Promise<DelayUpdate> {
    debugLog(
      `[DelayManager] 开始测试延迟，代理: ${name}, 组: ${group}, 超时: ${timeout}ms`,
    )

    if (signal?.aborted) {
      throw new DOMException('Aborted', 'AbortError')
    }

    // 先将状态设置为测试中
    this.setDelay(name, group, -2)

    const startTime = Date.now()
    let timerId: ReturnType<typeof setTimeout> | null = null
    let abortListener: (() => void) | null = null
    let raceFinished = false

    try {
      const url = this.getUrl(group)
      debugLog(`[DelayManager] 调用API测试延迟，代理: ${name}, URL: ${url}`)

      // 设置超时处理, delay = 0 为超时
      const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
        timerId = setTimeout(() => {
          if (!raceFinished) {
            resolve({ delay: 0 })
          }
        }, timeout)
      })

      // 监听取消信号
      // L-24: 只有当 signal 存在时才创建 abortPromise 并加入 race
      // 避免无 signal 时产生永不 settle 的 promise（虽不影响功能，但不优雅）
      const abortPromise = signal
        ? new Promise<ProxyDelay>((_, reject) => {
            abortListener = () => {
              reject(new DOMException('Aborted', 'AbortError'))
            }
            signal.addEventListener('abort', abortListener)
          })
        : null

      // 使用Promise.race来实现超时与取消控制
      const racePromises: Promise<ProxyDelay>[] = [
        delayProxyByNameWithTimeout(name, url)
          .then((res) => {
            raceFinished = true
            return res
          })
          .catch((err) => {
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
      ]
      if (abortPromise) {
        racePromises.push(abortPromise)
      }
      const result = await Promise.race(racePromises)

      // 确保至少显示500ms的加载动画，除非被取消
      const elapsedTime = Date.now() - startTime
      if (elapsedTime < 500 && !signal?.aborted) {
        await new Promise((resolve) => setTimeout(resolve, 500 - elapsedTime))
      }

      const delay = result.delay
      const elapsed = elapsedTime
      debugLog(`[DelayManager] 延迟测试完成，代理: ${name}, 结果: ${delay}ms`)

      return this.setDelay(name, group, delay, { elapsed })
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        debugLog(`[DelayManager] 延迟测试已取消，代理: ${name}`)
        throw error
      }
      // 确保至少显示500ms的加载动画
      await new Promise((resolve) => setTimeout(resolve, 500))
      console.error(`[DelayManager] 延迟测试出错，代理: ${name}`, error)
      const delay = 1e6 // error
      const elapsed = Date.now() - startTime

      return this.setDelay(name, group, delay, { elapsed })
    } finally {
      raceFinished = true
      if (timerId) {
        clearTimeout(timerId)
      }
      if (signal && abortListener) {
        signal.removeEventListener('abort', abortListener)
      }
    }
  }

  /**
   * 注入后台批量测速结果到缓存，并通知 UI 刷新
   * 由 Tauri 事件 verge://backend-delay-results 驱动
   */
  injectBatchResults(group = 'PROXY', results: Array<[string, number]>) {
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
    return formatDelay(delay, timeout)
  }

  formatDelayColor(
    delay: number,
    timeout = NODE_DELAY_MAX_MS,
    isDarkMode = false,
  ) {
    return formatDelayColor(delay, timeout, isDarkMode)
  }

  /** 销毁定时器等资源，用于 HMR / 测试清理 */
  destroy() {
    if (this.cleanupIntervalId) {
      clearInterval(this.cleanupIntervalId)
      this.cleanupIntervalId = null
    }
  }
}

/** 将延迟数值转换为 UI 展示字符串 */
export function formatDelay(delay: number, timeout = NODE_DELAY_MAX_MS) {
  if (delay === -1) return '-'
  if (delay === -2) return 'testing'
  if (delay === 0 || (delay >= timeout && delay <= 1e5)) return 'Timeout'
  if (delay > 1e5) return 'Error'
  return `${delay}`
}

/** 将延迟数值映射为 MUI 主题色名 */
export function formatDelayColor(
  delay: number,
  timeout = NODE_DELAY_MAX_MS,
  isDarkMode = false,
) {
  if (delay < 0) return ''
  if (delay === 0 || delay >= timeout) return 'error.main'
  if (delay >= 400) return isDarkMode ? 'warning.main' : 'warning.dark'
  if (delay >= 250) return 'primary.main'
  return 'success.main'
}

let _instance: DelayManager | null = null

/** 获取 DelayManager 惰性单例（避免模块加载时即创建实例等副作用） */
export function getDelayManager(): DelayManager {
  if (!_instance) {
    _instance = new DelayManager()
  }
  return _instance
}
