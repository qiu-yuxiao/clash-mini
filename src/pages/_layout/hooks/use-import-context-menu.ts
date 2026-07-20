import { useRef, useState } from 'react'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'

interface UseImportContextMenuParams {
  url: string
  setUrl: (val: string) => void
}

export function useImportContextMenu({ url, setUrl }: UseImportContextMenuParams) {
  const [importInputContextMenu, setImportInputContextMenu] = useState<{
    mouseX: number
    mouseY: number
  } | null>(null)
  const importInputRef = useRef<HTMLInputElement>(null)

  const handlePaste = async () => {
    setImportInputContextMenu(null)
    let textToPaste = ''
    try {
      textToPaste = await readText()
    } catch {
      try {
        textToPaste = await navigator.clipboard.readText()
      } catch (e) {
        console.error('Failed to read from clipboard:', e)
      }
    }
    if (!textToPaste) return
    const input = importInputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const currentVal = url || ''
      const newValue = currentVal.substring(0, start) + textToPaste + currentVal.substring(end)
      setUrl(newValue)
      setTimeout(() => {
        input.focus()
        const newCursorPos = start + textToPaste.length
        input.setSelectionRange(newCursorPos, newCursorPos)
      }, 0)
    } else {
      setUrl(textToPaste)
    }
  }

  const handleCopy = async () => {
    setImportInputContextMenu(null)
    const input = importInputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const selectedText = (url || '').substring(start, end)
      if (selectedText) {
        try { await writeText(selectedText) } catch {
          try { await navigator.clipboard.writeText(selectedText) } catch (e) {
            console.error('Failed to copy to clipboard:', e)
          }
        }
      }
    }
  }

  const handleCut = async () => {
    setImportInputContextMenu(null)
    const input = importInputRef.current
    if (input) {
      const start = input.selectionStart ?? 0
      const end = input.selectionEnd ?? 0
      const currentVal = url || ''
      const selectedText = currentVal.substring(start, end)
      if (selectedText) {
        try { await writeText(selectedText) } catch {
          try { await navigator.clipboard.writeText(selectedText) } catch (e) {
            console.error('Failed to copy to clipboard:', e)
          }
        }
        const newValue = currentVal.substring(0, start) + currentVal.substring(end)
        setUrl(newValue)
        setTimeout(() => {
          input.focus()
          input.setSelectionRange(start, start)
        }, 0)
      }
    }
  }

  const handleSelectAll = () => {
    setImportInputContextMenu(null)
    const input = importInputRef.current
    if (input) {
      input.focus()
      input.setSelectionRange(0, (url || '').length)
    }
  }

  const handleClear = () => {
    setImportInputContextMenu(null)
    setUrl('')
    const input = importInputRef.current
    if (input) {
      input.focus()
    }
  }

  return {
    importInputContextMenu,
    setImportInputContextMenu,
    importInputRef,
    handlePaste,
    handleCopy,
    handleCut,
    handleSelectAll,
    handleClear,
  }
}
