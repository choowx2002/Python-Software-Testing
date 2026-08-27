import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

type ProgressState = 'none' | 'indeterminate' | 'normal' | 'error' | 'paused'

export function useTaskbarProgress() {
  const state = ref<ProgressState>('none')
  const value = ref(0)

  async function setProgress(val: number, max = 100, status: ProgressState = 'normal') {
    state.value = status
    value.value = Math.min(100, Math.max(0, Math.round((val / max) * 100)))
    await invoke('set_taskbar_progress', { progress: value.value, state: status })
  }

  async function clear() {
    state.value = 'none'
    await invoke('set_taskbar_progress', { progress: 0, state: 'none' })
  }

  return { setProgress, clear, state, value }
}