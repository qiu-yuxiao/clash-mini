export default function debounce<T extends (...args: unknown[]) => void>(
  func: T,
  wait: number,
): T {
  let timeout: ReturnType<typeof setTimeout> | null = null
  return function (this: unknown, ...args: Parameters<T>) {
    if (timeout !== null) {
      clearTimeout(timeout)
    }
    timeout = setTimeout(() => func.apply(this, args), wait)
  } as T
}
