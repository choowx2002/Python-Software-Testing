import { onMounted, onBeforeUnmount } from 'vue'

export function useGlobalShortcuts(handlers: Record<string, () => void>) {
  onMounted(() => {
    window.addEventListener('keydown', handleKeydown)
  })
  onBeforeUnmount(() => {
    window.removeEventListener('keydown', handleKeydown)
  })

  function handleKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return

    const key = e.key.toLowerCase()
    const combo = [
      e.ctrlKey && 'ctrl',
      e.metaKey && 'meta',
      e.shiftKey && 'shift',
      e.altKey && 'alt',
      key.length === 1 ? key : key
    ].filter(Boolean).join('+')

    handlers[combo]?.()
  }
}

export function usePageShortcuts(handlers: Record<string, () => void>, enabled: () => boolean = () => true) {
  onMounted(() => {
    window.addEventListener('keydown', handleKeydown)
  })
  onBeforeUnmount(() => {
    window.removeEventListener('keydown', handleKeydown)
  })

  function handleKeydown(e: KeyboardEvent) {
    if (!enabled()) return
    const target = e.target as HTMLElement
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return

    const key = e.key.toLowerCase()
    const combo = [
      e.ctrlKey && 'ctrl',
      e.metaKey && 'meta',
      e.shiftKey && 'shift',
      e.altKey && 'alt',
      key.length === 1 ? key : key
    ].filter(Boolean).join('+')

    handlers[combo]?.()
  }
}