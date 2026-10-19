/**
 * 判断一个节点是否是广告、说明或假节点。
 *
 * 【必须与后端保持一致】后端实现在 `src-tauri/src/utils/node.rs::is_dummy_node`，
 * 那是配置生成、自愈判定、节点恢复三个环节的权威来源。前端这里若用更宽松或更
 * 严格的规则，就会出现「后端认为可用、前端却隐藏」的节点，进而导致**后端选中一个
 * 用户在界面上看不到的节点**（历史反复出过这类前后端不一致问题）。
 *
 * 因此这里逐条对齐后端语义：
 * - 英文广告词 / URL scheme：允许子串匹配（这些词不会出现在正常节点名里）
 * - 中文广告短语：**归一化后要求「完全匹配或以该短语开头」**，
 *   不能用 includes —— 否则 "日本群组-A"、"美国-网址专线"、"上海-群节点-01"
 *   这类真实节点会被误杀。
 *
 * 取舍说明：前缀匹配会漏判「最新官网发布页」这类短语不在开头的伪节点。这是
 * 有意为之——漏判伪节点代价很小（伪节点测不出延迟，不会被选点逻辑选中，后端
 * 还会在检测/恢复环节再兜一层），而误杀真实节点会直接让用户少一个可用节点。
 */

/** 中文广告短语（与后端 ad_phrases 逐条一致） */
const AD_PHRASES = [
  '流量',
  '过期时间',
  '网址',
  '官网',
  '剩余',
  '套餐到期',
  '续费',
  '公告',
  '购买',
  '群',
] as const

/** 英文广告词（与后端高置信度分支逐条一致，允许子串） */
const AD_WORDS_EN = ['expire', 'traffic', 'website', 'subscribe'] as const

/** 可多层剥掉的包裹符号（与后端 wrappers 一致） */
const WRAPPERS: ReadonlyArray<readonly [string, string]> = [
  ['【', '】'],
  ['(', ')'],
  ['（', '）'],
  ['[', ']'],
  ['「', '」'],
]

/** 通用伪节点前缀（与后端一致） */
const DUMMY_PREFIXES = ['节点-', '节点:', '节点：', '节点 '] as const

/**
 * 节点名归一化：剥掉两端包裹符号（可多层）与「节点-」这类通用前缀，
 * 让不在开头的广告短语回到开头再判定。**不剥区域/协议前缀**（如 "CN2-"、"HK-"）。
 */
const normalizeDummyName = (name: string): string => {
  let s = name.trim()

  // 多层剥壳：每轮只剥最外层一对，剥不动就停
  for (;;) {
    const before = s.length
    for (const [open, close] of WRAPPERS) {
      if (
        s.startsWith(open) &&
        s.endsWith(close) &&
        s.length >= open.length + close.length
      ) {
        s = s.slice(open.length, s.length - close.length).trim()
        break
      }
    }
    if (s.length === before) break
  }

  for (const prefix of DUMMY_PREFIXES) {
    if (s.startsWith(prefix)) {
      s = s.slice(prefix.length).trim()
      break
    }
  }

  return s
}

export const isDummyNode = (name: string): boolean => {
  if (!name) return true

  // DIRECT 是策略而非真实节点：前端列表不应把它当成可选项展示
  if (name === 'DIRECT') return true

  const lower = name.toLowerCase()

  // 高置信度：URL scheme 或明确的英文广告词
  if (
    lower.includes('http://') ||
    lower.includes('https://') ||
    AD_WORDS_EN.some((word) => lower.includes(word))
  ) {
    return true
  }

  const normalized = normalizeDummyName(lower)
  return AD_PHRASES.some(
    (phrase) => normalized === phrase || normalized.startsWith(phrase),
  )
}
