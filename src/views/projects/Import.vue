<script setup lang="ts">
/**
 * Project Import Page - UC01: Setup Project Environment
 * 
 * 流程：
 *   1. 用户点击选择文件夹 → Tauri Dialog
 *   2. 前端校验路径 → invoke('validate_project_directory')
 *   3. Rust 检测环境 → invoke('detect_python_env')
 *   4. 显示环境状态（Python 版本、venv、依赖）
 *   5. 如有缺失依赖，提供"一键安装"按钮
 *   6. 写入 SQLite projects 表（前端 SQL 插件）
 *   7. 跳转回 Dashboard
 */

import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { getDatabase } from '../../utils/db'
import {
  FolderPlus, FolderOpen, Check, X, AlertCircle,
  ArrowLeft, Loader2, Sparkles, Terminal, Package
} from '@lucide/vue'

const router = useRouter()

// ============================================
// State
// ============================================
type Phase = 'select' | 'detecting' | 'review' | 'installing' | 'saving' | 'done'

const phase = ref<Phase>('select')
const projectPath = ref('')
const projectName = ref('')
const error = ref<string | null>(null)

interface EnvResult {
  python_path: string | null
  python_version: string | null
  venv_path: string | null
  venv_activated: boolean
  dependencies: {
    name: string
    installed: boolean
    version: string | null
  }[]
}

const envResult = ref<EnvResult | null>(null)
const installLogs = ref('')

// ============================================
// Computed
// ============================================
const missingDeps = computed(() => {
  if (!envResult.value) return []
  return envResult.value.dependencies.filter(d => !d.installed).map(d => d.name)
})

const isEnvReady = computed(() => {
  if (!envResult.value) return false
  return envResult.value.python_path !== null && missingDeps.value.length === 0
})

const envStatus = computed<'Ready' | 'Warning' | 'Failed'>(() => {
  if (!envResult.value) return 'Failed'
  if (!envResult.value.python_path) return 'Failed'
  if (missingDeps.value.length > 0) return 'Warning'
  return 'Ready'
})

// ============================================
// Actions
// ============================================
async function selectFolder() {
  error.value = null
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Python Project Directory',
    })
    
    if (!selected) return
    
    projectPath.value = selected as string
    // 用文件夹名作为默认项目名
    const parts = projectPath.value.replace(/[/\\]$/, '').split(/[/\\]/)
    projectName.value = parts[parts.length - 1] || 'Untitled Project'
    
    await detectEnvironment()
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  }
}

async function detectEnvironment() {
  phase.value = 'detecting'
  error.value = null
  
  try {
    // 1. Validate directory
    await invoke('validate_project_directory', { 
      projectPath: projectPath.value 
    })
    
    // 2. Detect Python environment
    const result = await invoke<EnvResult>('detect_python_env', {
      projectPath: projectPath.value
    })
    
    envResult.value = result
    phase.value = 'review'
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    phase.value = 'select'
  }
}

async function installMissingDeps() {
  if (!envResult.value?.python_path || missingDeps.value.length === 0) return
  
  phase.value = 'installing'
  installLogs.value = ''
  
  try {
    const result = await invoke<{
      success: boolean
      installed: string[]
      failed: string[]
      logs: string
    }>('install_dependencies', {
      pythonPath: envResult.value.python_path,
      packages: missingDeps.value,
    })
    
    installLogs.value = result.logs
    
    if (result.success) {
      // 重新检测环境以刷新状态
      await detectEnvironment()
    } else {
      error.value = `Failed to install: ${result.failed.join(', ')}`
      phase.value = 'review'
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    phase.value = 'review'
  }
}

async function saveProject() {
  if (!isEnvReady.value || !envResult.value) return
  
  phase.value = 'saving'
  
  try {
    const db = await getDatabase()
    
    // 检查是否已存在
    const existing = await db.select<{ id: number }[]>(
      `SELECT id FROM projects WHERE project_path = ?`,
      [projectPath.value]
    )
    
    if (existing.length > 0) {
      error.value = 'This project has already been imported.'
      phase.value = 'review'
      return
    }
    
    // 写入数据库
    await db.execute(
      `INSERT INTO projects (name, project_path, interpreter_path, status)
       VALUES (?, ?, ?, ?)`,
      [
        projectName.value,
        projectPath.value,
        envResult.value.python_path,
        'active',
      ]
    )
    
    phase.value = 'done'
    
    // 1.5 秒后跳转回 Dashboard
    setTimeout(() => {
      router.push('/')
    }, 1500)
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    phase.value = 'review'
  }
}

function reset() {
  phase.value = 'select'
  projectPath.value = ''
  projectName.value = ''
  envResult.value = null
  error.value = null
  installLogs.value = ''
}
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-slate-50">
    
    <!-- Header -->
    <div class="h-14 px-6 flex items-center justify-between border-b border-zinc-200/80 bg-white">
      <div class="flex items-center gap-3">
        <button 
          @click="router.back()"
          class="p-1.5 hover:bg-slate-100 rounded-md transition-colors"
        >
          <ArrowLeft class="w-4 h-4 text-slate-600" />
        </button>
        <div class="w-px h-5 bg-zinc-200/80"></div>
        <FolderPlus class="w-4 h-4 text-emerald-500" />
        <h2 class="text-sm font-semibold text-slate-800">Import Project</h2>
      </div>
      
      <!-- Phase Indicator -->
      <div class="flex items-center gap-2 text-[10px] font-mono text-slate-500">
        <span :class="phase === 'select' ? 'text-emerald-600 font-semibold' : ''">1.SELECT</span>
        <span class="text-slate-300">→</span>
        <span :class="phase === 'detecting' ? 'text-emerald-600 font-semibold' : ''">2.DETECT</span>
        <span class="text-slate-300">→</span>
        <span :class="phase === 'review' || phase === 'installing' ? 'text-emerald-600 font-semibold' : ''">3.REVIEW</span>
        <span class="text-slate-300">→</span>
        <span :class="phase === 'saving' || phase === 'done' ? 'text-emerald-600 font-semibold' : ''">4.SAVE</span>
      </div>
    </div>

    <!-- Main Content -->
    <div class="flex-1 overflow-auto flex items-center justify-center p-8">
      <div class="w-full max-w-2xl">
        
        <!-- Phase 1: Select Folder -->
        <div v-if="phase === 'select'" class="bg-white border border-zinc-200/80 rounded-md p-8">
          <div class="text-center mb-6">
            <div class="w-14 h-14 bg-emerald-50 rounded-md flex items-center justify-center mx-auto mb-4">
              <FolderOpen class="w-7 h-7 text-emerald-500" />
            </div>
            <h3 class="text-lg font-semibold text-slate-800 mb-1">Select Python Project</h3>
            <p class="text-xs text-slate-500">
              Choose a local directory containing your Python source code.
            </p>
          </div>

          <button
            @click="selectFolder"
            class="w-full py-3 bg-emerald-500 hover:bg-emerald-600 text-white text-sm font-medium rounded-md transition-colors active:scale-[0.99] flex items-center justify-center gap-2"
          >
            <FolderOpen class="w-4 h-4" />
            Browse Directory...
          </button>

          <div v-if="error" class="mt-4 p-3 bg-rose-50 border border-rose-200 rounded-md flex items-start gap-2">
            <X class="w-4 h-4 text-rose-500 flex-shrink-0 mt-0.5" />
            <p class="text-xs text-rose-700">{{ error }}</p>
          </div>

          <div class="mt-6 p-3 bg-slate-50 rounded-md">
            <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">Requirements</p>
            <ul class="space-y-1 text-[11px] text-slate-600">
              <li class="flex items-center gap-2">
                <Check class="w-3 h-3 text-emerald-500" />
                Directory must contain at least one <code class="font-mono text-slate-800">.py</code> file
              </li>
              <li class="flex items-center gap-2">
                <Check class="w-3 h-3 text-emerald-500" />
                Python 3.10+ interpreter (auto-detected or manual)
              </li>
              <li class="flex items-center gap-2">
                <Check class="w-3 h-3 text-emerald-500" />
                Virtual environment recommended (venv / .venv)
              </li>
            </ul>
          </div>
        </div>

        <!-- Phase 2: Detecting -->
        <div v-else-if="phase === 'detecting'" class="bg-white border border-zinc-200/80 rounded-md p-8 text-center">
          <Loader2 class="w-10 h-10 text-emerald-500 animate-spin mx-auto mb-4" />
          <h3 class="text-sm font-semibold text-slate-800 mb-1">Detecting Environment...</h3>
          <p class="text-xs text-slate-500 font-mono truncate">{{ projectPath }}</p>
        </div>

        <!-- Phase 3: Review Environment -->
        <div v-else-if="phase === 'review' || phase === 'installing'" class="bg-white border border-zinc-200/80 rounded-md overflow-hidden">
          <!-- Project Info -->
          <div class="p-5 border-b border-zinc-200/80">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-sm font-semibold text-slate-800">Project Configuration</h3>
              <span 
                :class="[
                  'inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-full',
                  envStatus === 'Ready' ? 'bg-emerald-50 text-emerald-600' :
                  envStatus === 'Warning' ? 'bg-amber-50 text-amber-600' :
                  'bg-rose-50 text-rose-600'
                ]"
              >
                <span 
                  :class="[
                    'w-1.5 h-1.5 rounded-full',
                    envStatus === 'Ready' ? 'bg-emerald-500' :
                    envStatus === 'Warning' ? 'bg-amber-500' :
                    'bg-rose-500'
                  ]"
                ></span>
                {{ envStatus }}
              </span>
            </div>

            <div class="space-y-2">
              <div>
                <label class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider">Project Name</label>
                <input
                  v-model="projectName"
                  type="text"
                  class="mt-1 w-full px-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-500/40"
                />
              </div>
              <div>
                <label class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider">Project Path</label>
                <p class="mt-1 px-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-xs font-mono text-slate-600 truncate">
                  {{ projectPath }}
                </p>
              </div>
            </div>
          </div>

          <!-- Environment Details -->
          <div class="p-5 border-b border-zinc-200/80">
            <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-3">Environment Detection</p>
            <div class="space-y-2">
              <!-- Python -->
              <div class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md">
                <div class="flex items-center gap-2">
                  <Terminal class="w-3.5 h-3.5 text-slate-500" />
                  <span class="text-xs text-slate-700">Python Interpreter</span>
                </div>
                <div v-if="envResult?.python_version" class="flex items-center gap-2">
                  <span class="text-xs font-mono text-slate-600">{{ envResult.python_version }}</span>
                  <Check class="w-3.5 h-3.5 text-emerald-500" />
                </div>
                <div v-else class="flex items-center gap-2">
                  <span class="text-xs text-rose-600">Not Found</span>
                  <X class="w-3.5 h-3.5 text-rose-500" />
                </div>
              </div>

              <!-- Venv -->
              <div class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md">
                <div class="flex items-center gap-2">
                  <Package class="w-3.5 h-3.5 text-slate-500" />
                  <span class="text-xs text-slate-700">Virtual Environment</span>
                </div>
                <div v-if="envResult?.venv_activated" class="flex items-center gap-2">
                  <span class="text-xs font-mono text-slate-600 truncate max-w-[200px]">{{ envResult.venv_path }}</span>
                  <Check class="w-3.5 h-3.5 text-emerald-500" />
                </div>
                <div v-else class="flex items-center gap-2">
                  <span class="text-xs text-amber-600">Not Detected</span>
                  <AlertCircle class="w-3.5 h-3.5 text-amber-500" />
                </div>
              </div>

              <!-- Dependencies -->
              <div 
                v-for="dep in envResult?.dependencies" 
                :key="dep.name"
                class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md"
              >
                <div class="flex items-center gap-2">
                  <Sparkles class="w-3.5 h-3.5 text-slate-500" />
                  <span class="text-xs text-slate-700">{{ dep.name }}</span>
                </div>
                <div v-if="dep.installed" class="flex items-center gap-2">
                  <span class="text-xs font-mono text-slate-600">v{{ dep.version }}</span>
                  <Check class="w-3.5 h-3.5 text-emerald-500" />
                </div>
                <div v-else class="flex items-center gap-2">
                  <span class="text-xs text-rose-600">Missing</span>
                  <X class="w-3.5 h-3.5 text-rose-500" />
                </div>
              </div>
            </div>
          </div>

          <!-- Install Logs (if installing) -->
          <div v-if="phase === 'installing' && installLogs" class="p-5 border-b border-zinc-200/80 bg-slate-900">
            <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">Installation Logs</p>
            <pre class="text-[11px] font-mono text-slate-300 whitespace-pre-wrap">{{ installLogs }}</pre>
          </div>

          <!-- Actions -->
          <div class="p-5 flex items-center justify-between gap-3">
            <button
              @click="reset"
              class="px-4 py-2 bg-white border border-zinc-200/80 text-xs font-medium text-slate-600 rounded-md hover:bg-slate-50 transition-colors active:scale-[0.98]"
            >
              Cancel
            </button>

            <div class="flex items-center gap-2">
              <button
                v-if="missingDeps.length > 0 && phase === 'review'"
                @click="installMissingDeps"
                class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-white text-xs font-medium rounded-md transition-colors active:scale-[0.98] flex items-center gap-1.5"
              >
                <Sparkles class="w-3.5 h-3.5" />
                Install Missing ({{ missingDeps.length }})
              </button>

              <button
                @click="saveProject"
                :disabled="!isEnvReady || phase === 'installing'"
                class="px-4 py-2 bg-emerald-500 hover:bg-emerald-600 disabled:bg-slate-300 disabled:cursor-not-allowed text-white text-xs font-medium rounded-md transition-colors active:scale-[0.98] flex items-center gap-1.5"
              >
                <Loader2 v-if="phase === 'saving'" class="w-3.5 h-3.5 animate-spin" />
                <Check v-else class="w-3.5 h-3.5" />
                {{ phase === 'saving' ? 'Saving...' : 'Import Project' }}
              </button>
            </div>
          </div>

          <div v-if="error" class="px-5 pb-5">
            <div class="p-3 bg-rose-50 border border-rose-200 rounded-md flex items-start gap-2">
              <X class="w-4 h-4 text-rose-500 flex-shrink-0 mt-0.5" />
              <p class="text-xs text-rose-700">{{ error }}</p>
            </div>
          </div>
        </div>

        <!-- Phase 4: Done -->
        <div v-else-if="phase === 'done'" class="bg-white border border-emerald-500/30 rounded-md p-8 text-center">
          <div class="w-14 h-14 bg-emerald-50 rounded-full flex items-center justify-center mx-auto mb-4">
            <Check class="w-7 h-7 text-emerald-500" />
          </div>
          <h3 class="text-lg font-semibold text-slate-800 mb-1">Project Imported Successfully!</h3>
          <p class="text-xs text-slate-500 mb-4">
            Redirecting to Dashboard...
          </p>
          <p class="text-[11px] font-mono text-slate-600 truncate">{{ projectName }}</p>
        </div>

      </div>
    </div>

  </div>
</template>