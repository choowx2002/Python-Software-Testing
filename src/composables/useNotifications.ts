import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'

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

export async function notify(title: string, body: string, _options?: { onClick?: () => void }) {
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
  await notify(
    success ? '测试生成完成' : '测试生成失败',
    `${projectName}: ${success ? `生成了 ${count} 个测试文件` : '请查看日志了解详情'}`,
  )
}

export async function notifyCoverageComplete(projectName: string, percent: number) {
  await notify(
    '覆盖率分析完成',
    `${projectName}: 总覆盖率 ${percent.toFixed(1)}%`,
  )
}

export async function notifyExecutionComplete(projectName: string, passed: number, failed: number) {
  await notify(
    failed > 0 ? '测试执行完成（有失败）' : '测试执行完成（全部通过）',
    `${projectName}: ${passed} 通过, ${failed} 失败`,
  )
}

export async function notifyEnvFixComplete(projectName: string) {
  await notify(
    '环境修复完成',
    `${projectName}: Python 3.11 已安装，虚拟环境已重建，依赖已安装。`,
  )
}
