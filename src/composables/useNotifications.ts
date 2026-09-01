import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'
import { useUIStore } from '../stores/uiStore'
import i18n from '../i18n'

let permissionGranted: boolean | null = null

export async function requestNotificationPermission(): Promise<boolean> {
  if (permissionGranted !== null) return permissionGranted
  try {
    const granted = await isPermissionGranted()
    if (granted) {
      permissionGranted = true
      return true
    }
    const permission = await requestPermission()
    permissionGranted = permission === 'granted'
    return permissionGranted
  } catch {
    permissionGranted = false
    return false
  }
}

export async function notify(title: string, body: string) {
  // 检查用户是否启用了通知
  const uiStore = useUIStore()
  if (!uiStore.notificationsEnabled) return

  const granted = await requestNotificationPermission()
  if (!granted) return
  // In Tauri v2, clicking a notification brings the app window to focus by default.
  try {
    sendNotification({ title, body })
  } catch {
    // ignore — notifications are best-effort
  }
}

export async function notifyGenerationComplete(projectName: string, success: boolean, count: number) {
  const t = i18n.global.t
  await notify(
    success ? t('notifications.generationSuccess') : t('notifications.generationFailure'),
    success
      ? t('notifications.generationSuccessBody', { projectName, count })
      : t('notifications.generationFailureBody', { projectName }),
  )
}

export async function notifyCoverageComplete(projectName: string, percent: number) {
  const t = i18n.global.t
  await notify(
    t('notifications.coverageTitle'),
    t('notifications.coverageBody', { projectName, percent: percent.toFixed(1) }),
  )
}

export async function notifyExecutionComplete(projectName: string, passed: number, failed: number) {
  const t = i18n.global.t
  await notify(
    failed > 0 ? t('notifications.executionFailTitle') : t('notifications.executionPassTitle'),
    t('notifications.executionBody', { projectName, passed, failed }),
  )
}

export async function notifyEnvFixComplete(projectName: string) {
  const t = i18n.global.t
  await notify(
    t('notifications.envFixTitle'),
    t('notifications.envFixBody', { projectName }),
  )
}
