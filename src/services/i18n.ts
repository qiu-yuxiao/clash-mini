import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

export const supportedLanguages = [
  'zh',
  'en',
  'ru',
  'fa',
  'tt',
  'id',
  'ar',
  'ko',
  'tr',
  'de',
  'es',
  'jp',
  'zhtw',
]

export const FALLBACK_LANGUAGE = 'zh'
const LANGUAGE_STORAGE_KEY = 'verge-language'

const normalizeLanguage = (language?: string) =>
  language?.toLowerCase().replace(/_/g, '-')

export const resolveLanguage = (language?: string) => {
  const normalized = normalizeLanguage(language)
  if (!normalized) {
    return FALLBACK_LANGUAGE
  }

  if (normalized === 'zh-tw') return 'zhtw'
  if (normalized === 'zh-cn') return 'zh'

  if (supportedLanguages.includes(normalized)) {
    return normalized
  }

  const baseLanguage = normalized.split('-')[0]
  if (supportedLanguages.includes(baseLanguage)) {
    return baseLanguage
  }

  return FALLBACK_LANGUAGE
}

const getLanguageStorage = () => {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage
  } catch {
    return null
  }
}

export const cacheLanguage = (language: string) => {
  const storage = getLanguageStorage()
  if (!storage) return

  try {
    storage.setItem(LANGUAGE_STORAGE_KEY, resolveLanguage(language))
  } catch (error) {
    console.warn('[i18n] Failed to cache language:', error)
  }
}

export const getCachedLanguage = () => {
  const storage = getLanguageStorage()
  if (!storage) return undefined

  try {
    const cached = storage.getItem(LANGUAGE_STORAGE_KEY)
    return cached ? resolveLanguage(cached) : undefined
  } catch (error) {
    console.warn('[i18n] Failed to read cached language:', error)
    return undefined
  }
}

type LocaleModule = {
  default: Record<string, unknown>
}

const localeModules = import.meta.glob<LocaleModule>('@/locales/*/index.ts')

const localeLoaders = Object.entries(localeModules).reduce<
  Record<string, () => Promise<LocaleModule>>
>((acc, [path, loader]) => {
  const match = path.match(/[/\\]locales[/\\]([^/\\]+)[/\\]index\.ts$/)
  if (match) {
    acc[match[1]] = loader
  }
  return acc
}, {})

export const languages: Record<string, Record<string, unknown>> = supportedLanguages.reduce(
  (acc, lang) => {
    acc[lang] = {}
    return acc
  },
  {} as Record<string, Record<string, unknown>>,
)

export const loadLanguage = async (language: string) => {
  try {
    const loader = localeLoaders[language]
    if (!loader) {
      throw new Error(`Locale loader not found for language "${language}"`)
    }
    const module = await loader()
    return module.default
  } catch (error) {
    if (language !== FALLBACK_LANGUAGE) {
      console.warn(
        `Failed to load language ${language}, fallback to ${FALLBACK_LANGUAGE}, ${error}`,
      )
      const fallbackLoader = localeLoaders[FALLBACK_LANGUAGE]
      if (!fallbackLoader) {
        throw new Error(
          `Fallback language "${FALLBACK_LANGUAGE}" resources are missing.`,
          { cause: error },
        )
      }
      const fallback = await fallbackLoader()
      return fallback.default
    }
    throw error
  }
}

// TODO(i18n-fallback): 当前翻译 key 不存在时，i18next 默认返回 key 本身作为降级
// 这是 i18next 的默认行为，优点是开发时能快速发现缺失的 key
// 已知限制与未来改进方向：
// 1. 目前仅配置了 fallbackLng（语言级降级：zh -> en 等），未配置 key 级别的默认值
// 2. 生产环境可考虑开启 saveMissing + 后端上报，自动收集缺失的 key
// 3. 可通过 returnDefaultValue 选项返回空字符串或更友好的占位符
// 4. 可结合 i18next-scanner 等工具在构建时扫描所有 key，避免遗漏
// 5. 对于关键页面的文案，可在代码中提供默认值作为双重保障（t('key', '默认值')）
//
// 当前策略评估：
// - 开发阶段：显示 key 本身便于快速定位缺失翻译，收益大于体验影响
// - 生产阶段：理论上所有 key 都应已翻译，缺失属于 bug，显示 key 便于上报
// - 若未来需要更友好的用户体验，可改为返回空字符串或上一级 key
i18n.use(initReactI18next).init({
  resources: {},
  lng: FALLBACK_LANGUAGE,
  fallbackLng: FALLBACK_LANGUAGE,
  interpolation: {
    escapeValue: false,
  },
  // 参考配置（未来可启用）：
  // saveMissing: true,           // 上报缺失的 key
  // missingKeyHandler: (lng, ns, key) => {
  //   console.warn(`[i18n] Missing key: ${lng}:${ns}:${key}`)
  // },
  // returnDefaultValue: false,   // 是否返回默认值而非 key
})

export const changeLanguage = async (language: string) => {
  const targetLanguage = resolveLanguage(language)

  if (!i18n.hasResourceBundle(targetLanguage, 'translation')) {
    const resources = await loadLanguage(targetLanguage)
    i18n.addResourceBundle(targetLanguage, 'translation', resources)
  }

  await i18n.changeLanguage(targetLanguage)
  cacheLanguage(targetLanguage)
}

export const initializeLanguage = async (
  initialLanguage: string = FALLBACK_LANGUAGE,
) => {
  await changeLanguage(initialLanguage)
}
