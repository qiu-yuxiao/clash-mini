import { useCallback, useEffect, useRef, useState } from 'react'

const DEFAULT_SKIN = 'retro-3d'
const PERSIST_DEBOUNCE_MS = 200

function readSkin(): string {
  if (typeof window === 'undefined') return DEFAULT_SKIN
  return localStorage.getItem('clash-mini-control-skin') || DEFAULT_SKIN
}

function readFactor(
  key: string,
  skin: string,
  fallback: number,
  legacyKey: string,
): number {
  const saved = localStorage.getItem(`clash-mini-${skin}-${key}`)
  if (saved !== null) return parseFloat(saved)
  if (skin === DEFAULT_SKIN) {
    const old = localStorage.getItem(legacyKey)
    return old !== null ? parseFloat(old) : fallback
  }
  return 1.0
}

function writeDepthToStorage(skin: string, val: number) {
  localStorage.setItem(`clash-mini-${skin}-val1`, val.toString())
  if (skin === DEFAULT_SKIN) {
    localStorage.setItem('clash-mini-depth-factor', val.toString())
  }
}

function writeVibrancyToStorage(skin: string, val: number) {
  localStorage.setItem(`clash-mini-${skin}-val2`, val.toString())
  if (skin === DEFAULT_SKIN) {
    localStorage.setItem('clash-mini-vibrancy-factor', val.toString())
  }
}

export function useSkinControls() {
  const [controlSkin, setControlSkin] = useState(readSkin)
  const currentSkinRef = useRef(controlSkin)

  const [depthFactor, setDepthFactor] = useState<number>(() =>
    readFactor('val1', readSkin(), 0.3, 'clash-mini-depth-factor'),
  )

  const [vibrancyFactor, setVibrancyFactor] = useState<number>(() =>
    readFactor('val2', readSkin(), 1.0, 'clash-mini-vibrancy-factor'),
  )

  const dirtyDepthRef = useRef<number | null>(null)
  const dirtyVibrancyRef = useRef<number | null>(null)
  const lastPersistedDepthRef = useRef<{ skin: string; val: number } | null>(
    null,
  )
  const lastPersistedVibrancyRef = useRef<{ skin: string; val: number } | null>(
    null,
  )
  const depthTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const vibrancyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const persistDepthDeduplicated = useCallback((skin: string, val: number) => {
    const prev = lastPersistedDepthRef.current
    if (prev && prev.skin === skin && prev.val === val) return
    writeDepthToStorage(skin, val)
    lastPersistedDepthRef.current = { skin, val }
  }, [])

  const persistVibrancyDeduplicated = useCallback(
    (skin: string, val: number) => {
      const prev = lastPersistedVibrancyRef.current
      if (prev && prev.skin === skin && prev.val === val) return
      writeVibrancyToStorage(skin, val)
      lastPersistedVibrancyRef.current = { skin, val }
    },
    [],
  )

  const flushDirtyFactors = useCallback(() => {
    if (depthTimerRef.current !== null) {
      clearTimeout(depthTimerRef.current)
      depthTimerRef.current = null
    }
    if (vibrancyTimerRef.current !== null) {
      clearTimeout(vibrancyTimerRef.current)
      vibrancyTimerRef.current = null
    }
    const skin = currentSkinRef.current
    if (dirtyDepthRef.current !== null) {
      persistDepthDeduplicated(skin, dirtyDepthRef.current)
      dirtyDepthRef.current = null
    }
    if (dirtyVibrancyRef.current !== null) {
      persistVibrancyDeduplicated(skin, dirtyVibrancyRef.current)
      dirtyVibrancyRef.current = null
    }
  }, [persistDepthDeduplicated, persistVibrancyDeduplicated])

  // 监听皮肤切换自定义事件，并在页面卸载/隐藏/失焦时同步冲刷未落盘的脏值（BUG-358）
  useEffect(() => {
    const handleSkinChanged = () => {
      flushDirtyFactors()
      const newSkin = readSkin()
      currentSkinRef.current = newSkin
      setControlSkin(newSkin)
      setDepthFactor(
        readFactor('val1', newSkin, 0.3, 'clash-mini-depth-factor'),
      )
      setVibrancyFactor(
        readFactor('val2', newSkin, 1.0, 'clash-mini-vibrancy-factor'),
      )
    }
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        flushDirtyFactors()
      }
    }
    window.addEventListener('clash-mini-skin-changed', handleSkinChanged)
    window.addEventListener('pagehide', flushDirtyFactors)
    window.addEventListener('beforeunload', flushDirtyFactors)
    window.addEventListener('blur', flushDirtyFactors)
    document.addEventListener('visibilitychange', handleVisibilityChange)
    return () => {
      flushDirtyFactors()
      window.removeEventListener('clash-mini-skin-changed', handleSkinChanged)
      window.removeEventListener('pagehide', flushDirtyFactors)
      window.removeEventListener('beforeunload', flushDirtyFactors)
      window.removeEventListener('blur', flushDirtyFactors)
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [flushDirtyFactors])

  // BUG-358: 拖动过程即时更新内存态与 CSS 变量（60fps 丝滑无阻塞），辅以 200ms 防抖落盘 + 松手去重/失焦/隐藏/卸载即时冲刷
  const handleDepthFactorChange = useCallback(
    (val: number) => {
      setDepthFactor(val)
      document.documentElement.style.setProperty(
        '--depth-factor',
        val.toString(),
      )
      document.documentElement.style.setProperty(
        '--control-skin-val1',
        val.toString(),
      )
      dirtyDepthRef.current = val
      if (depthTimerRef.current !== null) {
        clearTimeout(depthTimerRef.current)
      }
      depthTimerRef.current = setTimeout(() => {
        depthTimerRef.current = null
        if (dirtyDepthRef.current !== null) {
          persistDepthDeduplicated(
            currentSkinRef.current,
            dirtyDepthRef.current,
          )
          dirtyDepthRef.current = null
        }
      }, PERSIST_DEBOUNCE_MS)
    },
    [persistDepthDeduplicated],
  )

  const commitDepthFactorChange = useCallback(
    (val: number) => {
      if (depthTimerRef.current !== null) {
        clearTimeout(depthTimerRef.current)
        depthTimerRef.current = null
      }
      dirtyDepthRef.current = null
      const skin = readSkin()
      currentSkinRef.current = skin
      persistDepthDeduplicated(skin, val)
    },
    [persistDepthDeduplicated],
  )

  const handleVibrancyFactorChange = useCallback(
    (val: number) => {
      setVibrancyFactor(val)
      document.documentElement.style.setProperty(
        '--vibrancy-factor',
        val.toString(),
      )
      document.documentElement.style.setProperty(
        '--control-skin-val2',
        val.toString(),
      )
      dirtyVibrancyRef.current = val
      if (vibrancyTimerRef.current !== null) {
        clearTimeout(vibrancyTimerRef.current)
      }
      vibrancyTimerRef.current = setTimeout(() => {
        vibrancyTimerRef.current = null
        if (dirtyVibrancyRef.current !== null) {
          persistVibrancyDeduplicated(
            currentSkinRef.current,
            dirtyVibrancyRef.current,
          )
          dirtyVibrancyRef.current = null
        }
      }, PERSIST_DEBOUNCE_MS)
    },
    [persistVibrancyDeduplicated],
  )

  const commitVibrancyFactorChange = useCallback(
    (val: number) => {
      if (vibrancyTimerRef.current !== null) {
        clearTimeout(vibrancyTimerRef.current)
        vibrancyTimerRef.current = null
      }
      dirtyVibrancyRef.current = null
      const skin = readSkin()
      currentSkinRef.current = skin
      persistVibrancyDeduplicated(skin, val)
    },
    [persistVibrancyDeduplicated],
  )

  // CSS 变量注入
  useEffect(() => {
    document.documentElement.style.setProperty(
      '--depth-factor',
      depthFactor.toString(),
    )
    document.documentElement.style.setProperty(
      '--control-skin-val1',
      depthFactor.toString(),
    )
  }, [depthFactor, controlSkin])

  useEffect(() => {
    document.documentElement.style.setProperty(
      '--vibrancy-factor',
      vibrancyFactor.toString(),
    )
    document.documentElement.style.setProperty(
      '--control-skin-val2',
      vibrancyFactor.toString(),
    )
  }, [vibrancyFactor, controlSkin])

  return {
    controlSkin,
    setControlSkin,
    depthFactor,
    vibrancyFactor,
    handleDepthFactorChange,
    commitDepthFactorChange,
    handleVibrancyFactorChange,
    commitVibrancyFactorChange,
  }
}
