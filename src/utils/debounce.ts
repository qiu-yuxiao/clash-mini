export default function debounce<Args extends unknown[], R>(
  func: (...args: Args) => R,
  wait: number,
): (...args: Args) => void {
  let timeout: ReturnType<typeof setTimeout> | null = null
  return function (this: unknown, ...args: Args) {
    if (timeout !== null) {
      clearTimeout(timeout)
    }
    timeout = setTimeout(() => {
      func.apply(this, args)
    }, wait)
  }
}
