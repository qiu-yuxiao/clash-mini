export type ParsedFilterClauses = {
  requiredGroups: string[][]
  excludedTerms: string[]
}

export type CompileStringMatcherResult = {
  matcher: (content: string) => boolean
  isValid: boolean
}

function stripExclusionPrefix(token: string): string {
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
 * 解析搜索/过滤表达式（空格 AND、`|`/`｜` OR、`-`/`!`/`！` 排除）。
 *
 * 【必须与后端保持一致】后端实现在 `src-tauri/src/utils/node/filter.rs::match_node_filter`。
 */
export const parseFilterClauses = (query: string): ParsedFilterClauses => {
  const trimmed = (query || '').trim()
  if (!trimmed) {
    return { requiredGroups: [], excludedTerms: [] }
  }

  const collapsed = trimmed
    .toLowerCase()
    .replace(/｜/g, '|')
    .split('|')
    .map((part) => part.trim())
    .join('|')

  const requiredGroups: string[][] = []
  const excludedTerms: string[] = []

  for (const token of collapsed.split(/\s+/).filter(Boolean)) {
    if (
      token.startsWith('-') ||
      token.startsWith('!') ||
      token.startsWith('！')
    ) {
      for (const part of token.split('|')) {
        const term = stripExclusionPrefix(part)
        if (term) {
          excludedTerms.push(term)
        }
      }
    } else {
      const branches = token
        .split('|')
        .map((part) => part.trim())
        .filter(Boolean)
      if (branches.length > 0) {
        requiredGroups.push(branches)
      }
    }
  }

  return { requiredGroups, excludedTerms }
}

/**
 * 提取正向检索词（排除 `-`/`!`/`！` 否定项），去重后按长度降序排列以供多关键词高亮。
 */
export const extractPositiveSearchTerms = (query: string): string[] => {
  const { requiredGroups } = parseFilterClauses(query)
  const unique = new Set<string>()
  for (const branches of requiredGroups) {
    for (const branch of branches) {
      unique.add(branch)
    }
  }
  return Array.from(unique).sort((a, b) => b.length - a.length)
}

export const compileStringMatcher = (
  query: string,
): CompileStringMatcherResult => {
  const { requiredGroups, excludedTerms } = parseFilterClauses(query)
  if (requiredGroups.length === 0 && excludedTerms.length === 0) {
    return { matcher: () => true, isValid: true }
  }

  return {
    matcher: (content: string) => {
      const lower = (content || '').toLowerCase()
      for (const excluded of excludedTerms) {
        if (lower.includes(excluded)) {
          return false
        }
      }
      for (const branches of requiredGroups) {
        if (!branches.some((branch) => lower.includes(branch))) {
          return false
        }
      }
      return true
    },
    isValid: true,
  }
}
