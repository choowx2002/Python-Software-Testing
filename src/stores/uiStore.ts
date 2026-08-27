import { defineStore } from 'pinia'

export const useUIStore = defineStore('ui', {
  state: () => ({
    tourCompleted: false,
    tourStep: 0,
    sidebarCollapsed: false,
    dashboardSortDesc: true,
    projectColWidths: [3, 1.2, 1.3, 1.6, 1] as number[],
    lastTabByProject: {} as Record<number, string>,
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
    }
  },
  persist: {
    omit: ['pendingRerunProjectId', 'pendingRerunTestIds']
  }
})