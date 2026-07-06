export type SearchMatcherOptions = {
  matchCase?: boolean
  matchWholeWord?: boolean
  useRegularExpression?: boolean
}

export type CompileStringMatcherResult = {
  matcher: (content: string) => boolean
  isValid: boolean
}

export const escapeRegex = (value: string) => {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export const buildRegex = (pattern: string, flags = '') => {
  try {
    return new RegExp(pattern, flags)
  } catch {
    return null
  }
}

export const compileStringMatcher = (
  query: string,
  _options: SearchMatcherOptions = {},
): CompileStringMatcherResult => {
  const trimmed = (query || '').trim()
  if (!trimmed) return { matcher: () => true, isValid: true }

  const target = trimmed.toLowerCase()
  return {
    matcher: (content: string) => content.toLowerCase().includes(target),
    isValid: true,
  }
}
