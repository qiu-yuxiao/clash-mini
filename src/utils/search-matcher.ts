export type CompileStringMatcherResult = {
  matcher: (content: string) => boolean
  isValid: boolean
}

export const compileStringMatcher = (
  query: string,
): CompileStringMatcherResult => {
  const trimmed = (query || '').trim()
  if (!trimmed) return { matcher: () => true, isValid: true }

  const target = trimmed.toLowerCase()
  return {
    matcher: (content: string) => content.toLowerCase().includes(target),
    isValid: true,
  }
}
