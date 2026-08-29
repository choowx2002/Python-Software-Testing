import { defineStore } from 'pinia'

export const useUIStore = defineStore('ui', {
  state: () => ({
    tourCompleted: false,
    tourStep: 0,
    sidebarCollapsed: false,
    dashboardSortDesc: true,
    projectColWidths: [3, 1.2, 1.3, 1.6, 1] as number[],
    lastTabByProject: {} as Record<number, string>,
    /** 终端面板宽度（相对窗口的比例，0~1） */
    terminalWidths: {} as Record<string, number>,
    /** 终端主题：true=深色，false=浅色（默认） */
    terminalDark: false,
    /** 系统通知开关：true=开启（默认），false=关闭 */
    notificationsEnabled: true,
    /** 打开测试/源码文件使用的编辑器命令模板；'' = 系统默认打开。
     *  支持 {path}（文件绝对路径）与 {line}（行号，可选）占位符，
     *  例如 'code -g {path}:{line}'。 */
    editorCommand: "",
    /** 待重跑的失败用例（由历史详情跳转到 Execute 页自动选中并运行） */
    pendingRerunProjectId: null as number | null,
    pendingRerunTestIds: [] as string[]
  }),
  actions: {
    completeTour() { this.tourCompleted = true },
    resetTour() { this.tourCompleted = false; this.tourStep = 0 },
    toggleSidebar() { this.sidebarCollapsed = !this.sidebarCollapsed },
    setDashboardSort(desc: boolean) { this.dashboardSortDesc = desc },
    setProjectColWidths(ratios: number[]) { this.projectColWidths = ratios },
    setTerminalWidth(key: string, ratio: number) { this.terminalWidths[key] = ratio },
    setTerminalDark(v: boolean) { this.terminalDark = v },
    setNotificationsEnabled(enabled: boolean) { this.notificationsEnabled = enabled },
    setEditorCommand(command: string) { this.editorCommand = command },
    resetProjectColWidths() { this.projectColWidths = [3, 1.2, 1.3, 1.6, 1] as number[] },
    resetTerminalWidths() { this.terminalWidths = {} },
    setLastTab(projectId: number, tab: string) {
      this.lastTabByProject[projectId] = tab
    },
    getLastTab(projectId: number): string | undefined {
      return this.lastTabByProject[projectId]
    },
    setPendingRerun(projectId: number, testIds: string[]) {
      this.pendingRerunProjectId = projectId
      this.pendingRerunTestIds = testIds
    },
    clearPendingRerun() {
      this.pendingRerunProjectId = null
      this.pendingRerunTestIds = []
    },
  },
  persist: {
    omit: ['pendingRerunProjectId', 'pendingRerunTestIds']
  }
})
