import { useState } from 'react'
export function useLocalStorage(key, initialValue) {
  return useState(initialValue)
}
