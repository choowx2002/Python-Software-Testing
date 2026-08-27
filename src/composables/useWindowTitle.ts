import { getCurrentWindow } from '@tauri-apps/api/window'

export function useWindowTitle(baseTitle = 'Testmate') {
  const window = getCurrentWindow()

  function setStatus(status: string) {
    window.setTitle(`${baseTitle} — ${status}`)
  }
  function reset() {
    window.setTitle(baseTitle)
  }
  return { setStatus, reset }
}