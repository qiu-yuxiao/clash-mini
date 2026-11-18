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
  if (
    AD_PHRASES.some(
      (phrase) => normalized === phrase || normalized.startsWith(phrase),
    )
  ) {
    return true
  }
  return isStructuralBannerName(name, lower)
}

const REGION_SUBSTRINGS_CJK = [
  '香港',
  '台湾',
  '台灣',
  '澳门',
  '澳門',
  '日本',
  '新加坡',
  '狮城',
  '美国',
  '美國',
  '韩国',
  '韓國',
  '首尔',
  '东京',
  '大阪',
  '硅谷',
  '洛杉矶',
  '西雅图',
  '圣何塞',
  '纽约',
  '英国',
  '英國',
  '伦敦',
  '德国',
  '德國',
  '法兰克福',
  '法国',
  '法國',
  '巴黎',
  '荷兰',
  '荷蘭',
  '阿姆斯特丹',
  '俄罗斯',
  '俄羅斯',
  '莫斯科',
  '加拿大',
  '多伦多',
  '温哥华',
  '澳洲',
  '澳大利亚',
  '悉尼',
  '墨尔本',
  '新西兰',
  '印度',
  '孟买',
  '印尼',
  '雅加达',
  '马来西亚',
  '吉隆坡',
  '泰国',
  '曼谷',
  '越南',
  '菲律宾',
  '土耳其',
  '伊斯坦布尔',
  '阿根廷',
  '巴西',
  '圣保罗',
  '智利',
  '墨西哥',
  '南非',
  '埃及',
  '阿联酋',
  '迪拜',
  '以色列',
  '乌克兰',
  '波兰',
  '瑞典',
  '瑞士',
  '苏黎世',
  '意大利',
  '西班牙',
  '爱尔兰',
  '芬兰',
  '挪威',
  '丹麦',
  '奥地利',
  '比利时',
  '捷克',
  '匈牙利',
  '罗马尼亚',
  '卢森堡',
  '冰岛',
  '葡萄牙',
  '希腊',
  '中国',
  '上海',
  '北京',
  '广州',
  '深圳',
  '欧洲',
  '北美',
  '南美',
  '亚太',
  'hong kong',
] as const

const REGION_ASCII_TOKENS = new Set([
  'hk',
  'hkg',
  'hongkong',
  'tw',
  'tpe',
  'taiwan',
  'mo',
  'mac',
  'jp',
  'jpn',
  'japan',
  'tyo',
  'osa',
  'nrt',
  'kix',
  'tokyo',
  'osaka',
  'sg',
  'sgp',
  'sin',
  'singapore',
  'usa',
  'america',
  'lax',
  'sjc',
  'sea',
  'ord',
  'ewr',
  'nyc',
  'iad',
  'dfw',
  'kr',
  'kor',
  'korea',
  'icn',
  'sel',
  'seoul',
  'uk',
  'gbr',
  'britain',
  'england',
  'lon',
  'london',
  'de',
  'deu',
  'germany',
  'fra',
  'frankfurt',
  'fr',
  'france',
  'par',
  'paris',
  'nl',
  'nld',
  'netherlands',
  'ams',
  'amsterdam',
  'ru',
  'rus',
  'russia',
  'mow',
  'moscow',
  'ca',
  'can',
  'canada',
  'yyz',
  'yvr',
  'toronto',
  'au',
  'aus',
  'australia',
  'syd',
  'sydney',
  'mel',
  'ind',
  'india',
  'bom',
  'idn',
  'indonesia',
  'cgk',
  'mys',
  'malaysia',
  'kul',
  'th',
  'tha',
  'thailand',
  'bkk',
  'vn',
  'vnm',
  'vietnam',
  'ph',
  'phl',
  'philippines',
  'mnl',
  'tr',
  'tur',
  'turkey',
  'ist',
  'arg',
  'argentina',
  'bra',
  'brazil',
  'gru',
  'uae',
  'dxb',
  'dubai',
  'bgp',
  'iplc',
  'iepl',
])

const BANNER_DOMAIN_TLDS = new Set([
  'com',
  'net',
  'org',
  'xyz',
  'top',
  'vip',
  'cc',
  'io',
  'me',
  'club',
  'link',
  'site',
  'shop',
  'info',
  'icu',
  'cyou',
  'online',
  'store',
  'tech',
  'app',
  'dev',
  'cloud',
  'pw',
  'co',
  'tv',
  'im',
  'la',
  'pro',
  'ltd',
  'work',
  'fun',
  'live',
  'ink',
  'buzz',
  'click',
  'cn',
  'hk',
  'tw',
  'jp',
  'sg',
  'uk',
  'us',
  'ru',
  'kr',
  'de',
])

const isCodeContextChar = (ch: string | undefined): boolean =>
  ch !== undefined && /[-_[\]0-9]/.test(ch)

/** 判断节点名是否带有地区或专线暗示（与后端 `node.rs::has_region_hint` 逐条对齐） */
export const hasRegionHint = (name: string): boolean => {
  if (!name || !name.trim()) return false

  for (const ch of name) {
    const cp = ch.codePointAt(0)
    if (cp !== undefined && cp >= 0x1f1e6 && cp <= 0x1f1ff) {
      return true
    }
  }

  const lower = name.toLowerCase()
  if (REGION_SUBSTRINGS_CJK.some((kw) => lower.includes(kw))) {
    return true
  }

  const re = /[A-Za-z]+/g
  for (;;) {
    const match = re.exec(name)
    if (!match) break
    const raw = match[0]
    const start = match.index
    const end = start + raw.length
    const left = start > 0 ? name[start - 1] : undefined
    const right = end < name.length ? name[end] : undefined
    if (left === '.' || right === '.') continue

    const token = raw.toLowerCase()
    if (REGION_ASCII_TOKENS.has(token)) return true
    if (token === 'cn' && (raw === 'CN' || right === '2')) return true
    if (
      token === 'us' &&
      (raw === 'US' || isCodeContextChar(left) || isCodeContextChar(right))
    ) {
      return true
    }
  }

  return false
}

const isLowerAlnum = (ch: string | undefined): boolean =>
  ch !== undefined && ch >= 'a' && ch <= 'z'
    ? true
    : ch !== undefined && ch >= '0' && ch <= '9'

const hasEmbeddedBannerDomain = (name: string, lower: string): boolean => {
  const hasNonAscii = [...name].some((ch) => (ch.codePointAt(0) ?? 0) > 0x7f)
  if (!(name.includes(':') || name.includes('：') || hasNonAscii)) {
    return false
  }

  for (let i = 2; i < lower.length - 1; i++) {
    if (lower[i] !== '.') continue
    if (!isLowerAlnum(lower[i - 1])) continue

    let l = i
    while (l > 0 && (isLowerAlnum(lower[l - 1]) || lower[l - 1] === '-')) {
      l--
    }
    if (i - l < 2) continue

    let r = i + 1
    while (r < lower.length && lower[r] >= 'a' && lower[r] <= 'z') {
      r++
    }
    if (r === i + 1) continue

    const next = lower[r]
    if (isLowerAlnum(next) || next === '.') continue

    if (BANNER_DOMAIN_TLDS.has(lower.slice(i + 1, r))) {
      return true
    }
  }

  return false
}

const hasColonQuotaOrDate = (lower: string): boolean => {
  const colonIdx = lower.search(/[:：]/)
  if (colonIdx === -1) return false
  const rhs = lower.slice(colonIdx + 1)
  return (
    /(?:^|[^0-9.])20\d{2}[-/.年]\d/.test(rhs) ||
    /(?:^|[^0-9.])\d+(?:\.\d+)?\s*(?:gib|tib|gb|tb|mb|pb)(?![a-z])/.test(rhs) ||
    /(?:^|[^0-9.])\d+(?:\.\d+)?\s*(?:天|小时|days?(?![a-z]))/.test(rhs)
  )
}

const isStructuralBannerName = (name: string, lower: string): boolean => {
  if (hasRegionHint(name)) return false
  return hasEmbeddedBannerDomain(name, lower) || hasColonQuotaOrDate(lower)
}

const stripExclusionPrefix = (token: string): string => {
  if (
    token.startsWith('-') ||
    token.startsWith('!') ||
    token.startsWith('！')
  ) {
    return token.slice(1).trim()
  }
  return token
}

/**
 * 节点名多关键词与排除过滤匹配。
 *
 * 【必须与后端保持一致】后端实现在 `src-tauri/src/utils/node.rs::match_node_filter`。
 * - 大小写不敏感；
 * - 空串 / 纯空白 / 孤立符号（`-`、`!`、`|`）返回 `true`；
 * - 空格分隔多个 `AND` 子句；
 * - 子句内支持 `|` 或全角 `｜` 分隔多关键词 `OR` 匹配（允许两侧空格，如 `日本 | 新加坡`）；
 * - 以 `-`、`!` 或 `！` 开头的子句表示排除（如 `-香港`、`日本|新加坡 -倍率`）。
 */
export const matchNodeFilter = (name: string, filterText: string): boolean => {
  const trimmed = (filterText || '').trim()
  if (!trimmed) return true

  const nameLower = (name || '').toLowerCase()
  const collapsed = trimmed
    .toLowerCase()
    .replace(/｜/g, '|')
    .split('|')
    .map((part) => part.trim())
    .join('|')

  for (const token of collapsed.split(/\s+/).filter(Boolean)) {
    if (
      token.startsWith('-') ||
      token.startsWith('!') ||
      token.startsWith('！')
    ) {
      for (const part of token.split('|')) {
        const term = stripExclusionPrefix(part)
        if (term && nameLower.includes(term)) {
          return false
        }
      }
    } else {
      const branches = token
        .split('|')
        .map((part) => part.trim())
        .filter(Boolean)
      if (
        branches.length > 0 &&
        !branches.some((branch) => nameLower.includes(branch))
      ) {
        return false
      }
    }
  }

  return true
}

const normalizeFilterForTokens = (filterText: string): string =>
  (filterText || '').trim().replace(/｜/g, '|')

/**
 * 判断过滤文本是否包含至少一个非空的实际匹配词（排除 "", "   ", "-", "!", "！", "|", "｜", "- | !" 等纯符号）。
 */
export const hasEffectiveFilterTerm = (filterText: string): boolean => {
  const normalized = normalizeFilterForTokens(filterText)
  if (!normalized) return false

  for (const token of normalized.split(/\s+/).filter(Boolean)) {
    for (const part of token.split('|')) {
      const term = stripExclusionPrefix(part.trim())
      if (term.length > 0) {
        return true
      }
    }
  }

  return false
}

/**
 * 判断过滤文本是否处于不完整的操作符半输入态（BUG-392）：
 * - 不含任何有效关键词（如 "-"、"!"）；
 * - 以 "|" / "｜" 结尾等待下一分支（如 "日本|"）；
 * - 最末子句 / 分支以孤立排除前缀 "-" / "!" / "！" 结尾等待排除词（如 "日本 -"、"-香港|-"）。
 * 注意："hk-" 属于前缀包含词本身，不视为未完成操作符。
 */
export const isIncompleteFilterInput = (filterText: string): boolean => {
  const trimmed = (filterText || '').trim()
  if (!hasEffectiveFilterTerm(trimmed)) return true
  if (trimmed.endsWith('|') || trimmed.endsWith('｜')) return true

  const normalized = normalizeFilterForTokens(trimmed)
  const tokens = normalized.split(/\s+/).filter(Boolean)
  const lastToken = tokens[tokens.length - 1] ?? ''
  const segments = lastToken.split('|')
  const lastPart = (segments[segments.length - 1] ?? '').trim()

  return lastPart === '-' || lastPart === '!' || lastPart === '！'
}

/** 与后端 universal_parser.rs 对齐的单节点 URI 协议头（多行导入也受支持） */
export const SUPPORTED_NODE_URI_SCHEMES = [
  'vmess://',
  'ss://',
  'trojan://',
  'vless://',
  'hysteria2://',
  'hysteria://',
  'anytls://',
  'tuic://',
  'wireguard://',
  'socks5://',
  'socks://',
] as const

/** 判断首次引导输入框 / 剪贴板文本是否为合法的单条订阅 URL 或单/多行节点 URI（BUG-389） */
export const isSupportedSubscriptionOrNodeInput = (raw: string): boolean => {
  const lines = raw
    .trim()
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
  if (lines.length === 0) return false

  const hasHttpUrl = lines.some((line) => /^https?:\/\//i.test(line))
  if (hasHttpUrl) {
    return lines.length === 1 && !/\s/.test(lines[0])
  }

  return lines.every((line) => {
    const lower = line.toLowerCase()
    return SUPPORTED_NODE_URI_SCHEMES.some((scheme) => lower.startsWith(scheme))
  })
}

export type BatchCheckTrigger = 'bolt' | 'f4' | 'filter' | 'f1'

/**
 * 裁决前端批量测速入口的「是否切换节点 / 是否清理非新目标旧连接」意图（BUG-397）：
 * - `bolt`（闪电图标）与 `f4`：仅刷新当前可见列表延迟，绝不换节点、绝不断存量连接；
 * - `filter`（过滤框输入防抖 / 回车）与 `f1`（全量优选）：测速后择优切换并精准清理旧代理连接。
 */
export const resolveBatchCheckIntent = (
  trigger: BatchCheckTrigger,
): { select: boolean; cleanStale: boolean } => {
  const shouldSwitch = trigger === 'filter' || trigger === 'f1'
  return {
    select: shouldSwitch,
    cleanStale: shouldSwitch,
  }
}
