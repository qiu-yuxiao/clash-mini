import { useCallback, useEffect, useState } from 'react'

const DEFAULT_SKIN = 'retro-3d'

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

export function useSkinControls() {
  const [controlSkin, setControlSkin] = useState(readSkin)

  const [depthFactor, setDepthFactor] = useState<number>(() =>
    readFactor('val1', readSkin(), 0.3, 'clash-mini-depth-factor'),
  )

  const [vibrancyFactor, setVibrancyFactor] = useState<number>(() =>
    readFactor('val2', readSkin(), 1.0, 'clash-mini-vibrancy-factor'),
  )

  // 监听皮肤切换自定义事件
  useEffect(() => {
    const handleSkinChanged = () => {
      const newSkin = readSkin()
      setControlSkin(newSkin)
      setDepthFactor(
        readFactor('val1', newSkin, 0.3, 'clash-mini-depth-factor'),
      )
      setVibrancyFactor(
        readFactor('val2', newSkin, 1.0, 'clash-mini-vibrancy-factor'),
      )
    }
    window.addEventListener('clash-mini-skin-changed', handleSkinChanged)
    return () => {
      window.removeEventListener('clash-mini-skin-changed', handleSkinChanged)
    }
  }, [])

  // BUG-358: 拖动过程仅更新内存态与 CSS 变量（60fps 丝滑无阻塞），松手提交时（commit）再一次性写盘
  const handleDepthFactorChange = useCallback((val: number) => {
    setDepthFactor(val)
    document.documentElement.style.setProperty('--depth-factor', val.toString())
    document.documentElement.style.setProperty(
      '--control-skin-val1',
      val.toString(),
    )
  }, [])

  const commitDepthFactorChange = useCallback((val: number) => {
    const skin = readSkin()
    localStorage.setItem(`clash-mini-${skin}-val1`, val.toString())
    if (skin === DEFAULT_SKIN) {
      localStorage.setItem('clash-mini-depth-factor', val.toString())
    }
  }, [])

  const handleVibrancyFactorChange = useCallback((val: number) => {
    setVibrancyFactor(val)
    document.documentElement.style.setProperty(
      '--vibrancy-factor',
      val.toString(),
    )
    document.documentElement.style.setProperty(
      '--control-skin-val2',
      val.toString(),
    )
  }, [])

  const commitVibrancyFactorChange = useCallback((val: number) => {
    const skin = readSkin()
    localStorage.setItem(`clash-mini-${skin}-val2`, val.toString())
    if (skin === DEFAULT_SKIN) {
      localStorage.setItem('clash-mini-vibrancy-factor', val.toString())
    }
  }, [])

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
