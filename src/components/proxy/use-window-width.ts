import { useEffect, useState } from 'react'

export const useWindowWidth = () => {
  const [isMinimal, setIsMinimal] = useState(() => document.body.clientWidth <= 285)

  useEffect(() => {
    const handleResize = () => {
      const minimal = document.body.clientWidth <= 285
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
