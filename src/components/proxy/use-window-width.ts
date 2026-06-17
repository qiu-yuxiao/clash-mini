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

export const useWindowWidth = () => {
  const [isMinimal, setIsMinimal] = useState(getIsMinimal)

  useEffect(() => {
    const handleResize = () => {
      const minimal = getIsMinimal()
      setIsMinimal((prev) => {
        if (prev !== minimal) {
          return minimal
        }
        return prev
      })
    }

    window.addEventListener('resize', handleResize)
    return () => {
      window.removeEventListener('resize', handleResize)
    }
  }, [])

  return { width: isMinimal ? 270 : 640 }
}

