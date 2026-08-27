import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface Project {
  id: number
  name: string
  path: string
  interpreter_path: string | null
  env_status: 'Ready' | 'Warning' | 'Failed' | 'active'
  tests_passed: number
  tests_failed: number
  coverage: number
  last_run: string | null
}

export interface GlobalStats {
  total_projects: number
  total_runs: number
  avg_pass_rate: number
  avg_coverage: number
}

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const stats = ref<GlobalStats>({
    total_projects: 0,
    total_runs: 0,
    avg_pass_rate: 0,
    avg_coverage: 0
  })
  const isLoading = ref(true)

  // ==========================================
  // 1. 获取项目列表（附带真实统计：执行历史 + 覆盖率）
  //    聚合查询由 Rust 端执行（NFR008：前端不直连裸 SQL）
  // ==========================================
  const fetchProjects = async () => {
    isLoading.value = true
    try {
      const rows = await invoke<
        {
          id: number
          name: string
          path: string
          interpreterPath: string | null
          envStatus: string
          testsPassed: number
          testsFailed: number
          coverage: number
          lastRun: string | null
        }[]
      >("get_projects")

      projects.value = rows.map(row => ({
        id: row.id,
        name: row.name,
        path: row.path,
        interpreter_path: row.interpreterPath,
        env_status: (row.envStatus || 'Warning') as Project['env_status'],
        tests_passed: Number(row.testsPassed) || 0,
        tests_failed: Number(row.testsFailed) || 0,
        coverage: Number(row.coverage) || 0,
        last_run: row.lastRun ?? null,
      }))

      const totalRuns = await invoke<number>("count_execution_history")
      const statsRows = await invoke<{
        avgPassRate: number
        avgCoverage: number
      }>("get_global_stats")

      stats.value = {
        total_projects: projects.value.length,
        total_runs: totalRuns,
        avg_pass_rate: statsRows.avgPassRate || 0,
        avg_coverage: statsRows.avgCoverage || 0,
      }
    } catch (error) {
      console.error('[Store] ❌ Failed to fetch projects:', error)
      projects.value = []
    } finally {
      isLoading.value = false
    }
  }


  const updateLastOpened = async (projectId: number) => {
    try {
      await invoke("update_last_opened", { projectId })
      console.log(`[Store] ✅ Updated last_opened_at for project ${projectId}`)
    } catch (error) {
      console.error(`[Store] ❌ Failed to update last opened for project ${projectId}:`, error)
      throw error
    }
  }

  const updateProject = async (projectId: number, newName: string) => {
    try {
      await invoke("update_project_name", { projectId, newName })

      // 立即同步更新本地 Pinia 状态，无需重新请求数据库，UI 会瞬间响应
      const projectIndex = projects.value.findIndex(p => p.id === projectId)
      if (projectIndex !== -1) {
        projects.value[projectIndex].name = newName
      }

      console.log(`[Store] ✅ Updated project name to "${newName}"`)
    } catch (error) {
      console.error(`[Store] ❌ Failed to update project ${projectId}:`, error)
      throw error
    }
  }


  const deleteProject = async (projectId: number) => {
    try {
      await invoke("delete_project", { projectId })

      projects.value = projects.value.filter(p => p.id !== projectId)

      stats.value.total_projects = projects.value.length

      console.log(`[Store] ✅ Deleted project ${projectId} and associated data`)
    } catch (error) {
      console.error(`[Store] ❌ Failed to delete project ${projectId}:`, error)
      throw error
    }
  }

  /**
   * 启动校验：逐项目检查 interpreter_path 是否仍有效。
   * 失效则后端自动重探测并更新 DB，这里同步本地状态。
   */
  const validateInterpreters = async () => {
    const list = [...projects.value]
    const results = await Promise.all(
      list.map(async (p) => {
        try {
          const res = await invoke<{ status: string; interpreterPath: string | null }>(
            "validate_project_interpreter",
            { projectId: p.id },
          )
          return { id: p.id, res }
        } catch {
          return { id: p.id, res: null }
        }
      }),
    )
    let changed = false
    for (const { id, res } of results) {
      if (!res) continue
      const proj = projects.value.find((x) => x.id === id)
      if (!proj) continue
      if (res.status === "recovered" && res.interpreterPath) {
        proj.interpreter_path = res.interpreterPath
        proj.env_status = "Warning"
        changed = true
      } else if (res.status === "missing") {
        proj.interpreter_path = null
        proj.env_status = "Failed"
        changed = true
      }
    }
    if (changed) {
      console.log("[Store] ✅ Interpreter paths validated/recovered")
    }
  }

  return {
    projects,
    stats,
    isLoading,
    fetchProjects,
    updateLastOpened,
    updateProject,
    deleteProject,
    validateInterpreters
  }
})
