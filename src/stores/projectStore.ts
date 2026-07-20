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
  last_run: string
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
  // 1. 获取项目列表 (保持不变)
  // ==========================================
  const fetchProjects = async () => {
    isLoading.value = true
    try {
      const db = await getDatabase()
      const rows = await db.select<any[]>(`
        SELECT id, name, project_path AS path, interpreter_path, status AS env_status, created_at
        FROM projects 
        ORDER BY created_at DESC
      `)

      projects.value = rows.map(row => ({
        id: row.id,
        name: row.name,
        path: row.path,
        interpreter_path: row.interpreter_path,
        env_status: row.env_status || 'Warning',
        tests_passed: 0,
        tests_failed: 0,
        coverage: 0.0,
        last_run: 'Never'
      }))

      stats.value = {
        total_projects: projects.value.length,
        total_runs: 0,
        avg_pass_rate: 0,
        avg_coverage: 0
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