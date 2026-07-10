import { listen, EventCallback } from '@tauri-apps/api/event'
import { useCallback } from 'react'

export const useListen = () => {
  const addListener = useCallback(
    <T>(eventName: string, handler: EventCallback<T>) => listen(eventName, handler),
    [],
  )

  return {
    addListener,
  }
}
