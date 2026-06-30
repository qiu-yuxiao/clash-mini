import { useEffect, useState } from 'react'

const getIsMinimal = (): boolean => {
  if (typeof document !== 'undefined' && document.body) {
    return document.body.clientWidth <= 285
  }
  if (typeof window !== 'undefined') {
    return window.innerWidth <= 285
  }
  return false
}

// 维护全局的监听状态与订阅者集合
let globalIsMinimal = getIsMinimal()
const listeners = new Set<(val: boolean) => void>()
let isListening = false
let rafId: number | null = null

const handleResize = () => {
  if (rafId !== null) return
  rafId = requestAnimationFrame(() => {
    rafId = null
    const minimal = getIsMinimal()
    if (minimal !== globalIsMinimal) {
      globalIsMinimal = minimal
      listeners.forEach((listener) => listener(minimal))
    }
  })
}

const subscribe = (listener: (val: boolean) => void) => {
  listeners.add(listener)
  if (!isListening) {
    window.addEventListener('resize', handleResize)
    isListening = true
  }
  return () => {
    listeners.delete(listener)
    if (listeners.size === 0 && isListening) {
      if (rafId !== null) {
        cancelAnimationFrame(rafId)
        rafId = null
      }
      window.removeEventListener('resize', handleResize)
      isListening = false
    }
  }
}

export const useWindowWidth = () => {
  const [isMinimal, setIsMinimal] = useState(globalIsMinimal)

  useEffect(() => {
    return subscribe(setIsMinimal)
  }, [])

  return { width: isMinimal ? 270 : 640 }
}

