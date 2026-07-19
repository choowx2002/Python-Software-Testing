import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
// import { invoke } from '@tauri-apps/api/core' // 后续对接 Tauri 时取消注释

export interface Project {
  id: string
  name: string
  path: string
  python_version: string
  env_status: 'Ready' | 'Warning' | 'Failed'
  env_details: string[] // 例如: ["Python 3.13", "venv activated", "Dependencies OK"]
  tests_passed: number
  tests_failed: number
  coverage: number
  last_run: string // 例如: "2h ago"
}

export interface GlobalStats {
  total_projects: number
  total_runs: number
  avg_pass_rate: number
  avg_coverage: number
}

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const stats = ref<GlobalStats>({ total_projects: 0, total_runs: 0, avg_pass_rate: 0, avg_coverage: 0 })
  const isLoading = ref(true)

  // 获取项目列表
  const fetchProjects = async () => {
    isLoading.value = true
    try {
      // TODO: 对接 Tauri 后端
      // const data = await invoke<Project[]>('get_projects')
      // projects.value = data
      
      // 🛑 Mock 数据 (供前端开发和论文展示使用，后端就绪后删除)
      await new Promise(resolve => setTimeout(resolve, 600)) // 模拟网络延迟
      projects.value = [
        {
          id: '1', name: 'E-Commerce API', path: '~/projects/ecommerce-api',
          python_version: '3.13', env_status: 'Ready',
          env_details: ['Python 3.13', 'venv activated', 'Dependencies OK'],
          tests_passed: 147, tests_failed: 9, coverage: 94.2, last_run: '2h ago'
        },
        {
          id: '2', name: 'Auth Microservice', path: '~/projects/auth-service',
          python_version: '3.11', env_status: 'Warning',
          env_details: ['Python 3.11', 'venv activated', 'Pynguin missing'],
          tests_passed: 98, tests_failed: 14, coverage: 72.3, last_run: '1d ago'
        },
        {
          id: '3', name: 'Analytics Dashboard', path: '~/projects/analytics-dash',
          python_version: '3.11', env_status: 'Failed',
          env_details: ['Python 3.11', 'venv not found', 'pytest missing'],
          tests_passed: 110, tests_failed: 24, coverage: 82.1, last_run: '2d ago'
        }
      ]
      
      // 计算全局统计
      stats.value = {
        total_projects: projects.value.length,
        total_runs: projects.value.reduce((acc, p) => acc + p.tests_passed + p.tests_failed, 0),
        avg_pass_rate: 94.2, // 可从后端聚合查询
        avg_coverage: 82.8
      }
    } catch (error) {
      console.error('Failed to fetch projects:', error)
    } finally {
      isLoading.value = false
    }
  }

  return { projects, stats, isLoading, fetchProjects }
})