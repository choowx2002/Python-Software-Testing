import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getDatabase } from '../utils/db' // 确保路径正确

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
  // ==========================================
  const fetchProjects = async () => {
    isLoading.value = true
    try {
      const db = await getDatabase()

      // 每个项目附带：
      // - tests_passed / tests_failed：test_execution_history 的累计通过/失败数
      // - last_run：最近一次执行时间（无记录时为 NULL，前端显示 "Never"）
      // - coverage：coverage_results 中该项目最新一条的语句覆盖率
      const rows = await db.select<any[]>(`
        SELECT
          p.id,
          p.name,
          p.project_path AS path,
          p.interpreter_path,
          p.status AS env_status,
          p.created_at,
          COALESCE((SELECT SUM(passed) FROM test_execution_history t WHERE t.project_id = p.id), 0) AS tests_passed,
          COALESCE((SELECT SUM(failed) FROM test_execution_history t WHERE t.project_id = p.id), 0) AS tests_failed,
          (SELECT MAX(executed_at) FROM test_execution_history t WHERE t.project_id = p.id) AS last_run,
          COALESCE((SELECT c.total_statement_coverage
                     FROM coverage_results c
                     WHERE c.project_id = p.id
                     ORDER BY c.id DESC LIMIT 1), 0) AS coverage
        FROM projects p
        ORDER BY p.created_at DESC
      `)

      projects.value = rows.map(row => ({
        id: row.id,
        name: row.name,
        path: row.path,
        interpreter_path: row.interpreter_path,
        env_status: row.env_status || 'Warning',
        tests_passed: Number(row.tests_passed) || 0,
        tests_failed: Number(row.tests_failed) || 0,
        coverage: Number(row.coverage) || 0,
        last_run: row.last_run ?? null,
      }))

      // 全局统计：总执行次数、平均通过率、平均覆盖率（每个项目取最新一条覆盖率）
      const statsRows = await db.select<any[]>(`
        SELECT
          (SELECT COUNT(*) FROM test_execution_history) AS total_runs,
          COALESCE((SELECT AVG(
            CASE WHEN (passed + failed + skipped) > 0
              THEN passed * 100.0 / (passed + failed + skipped)
              ELSE NULL END
          ) FROM test_execution_history), 0) AS avg_pass_rate,
          COALESCE((SELECT AVG(c.total_statement_coverage)
            FROM coverage_results c
            WHERE c.id IN (SELECT MAX(id) FROM coverage_results GROUP BY project_id)), 0) AS avg_coverage
      `)

      stats.value = {
        total_projects: projects.value.length,
        total_runs: Number(statsRows[0]?.total_runs) || 0,
        avg_pass_rate: Number(statsRows[0]?.avg_pass_rate) || 0,
        avg_coverage: Number(statsRows[0]?.avg_coverage) || 0,
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
      const db = await getDatabase()
      await db.execute(
        `UPDATE projects SET last_opened_at = CURRENT_TIMESTAMP WHERE id = ?`,
        [projectId]
      )
      console.log(`[Store] ✅ Updated last_opened_at for project ${projectId}`)

      // 可选：如果你想在本地状态中也体现，可以在这里更新，
      // 但通常下次 fetchProjects 时会自然同步。
    } catch (error) {
      console.error(`[Store] ❌ Failed to update last opened for project ${projectId}:`, error)
      throw error
    }
  }

  const updateProject = async (projectId: number, newName: string) => {
    try {
      const db = await getDatabase()
      await db.execute(
        `UPDATE projects SET name = ? WHERE id = ?`,
        [newName, projectId]
      )

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
      const db = await getDatabase()

      await db.execute(
        `DELETE FROM projects WHERE id = ?`,
        [projectId]
      )

      projects.value = projects.value.filter(p => p.id !== projectId)

      stats.value.total_projects = projects.value.length

      console.log(`[Store] ✅ Deleted project ${projectId} and associated data`)
    } catch (error) {
      console.error(`[Store] ❌ Failed to delete project ${projectId}:`, error)
      throw error
    }
  }

  return {
    projects,
    stats,
    isLoading,
    fetchProjects,
    updateLastOpened,
    updateProject,
    deleteProject
  }
})