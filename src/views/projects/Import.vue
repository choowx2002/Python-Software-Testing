<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
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
  venv_exists: boolean
  dependencies: {
    name: string
    installed: boolean
    version: string | null
  }[]
}

const envResult = ref<EnvResult | null>(null)
// 🧹 移除了 const installLogs = ref('')

// 🆕 记录每个包的实时状态
const stepStatus = ref<Record<string, 'starting' | 'success' | 'failed'>>({})

// ============================================
// Computed
// ============================================
const missingDeps = computed(() => {
  if (!envResult.value?.venv_exists) return []
  return envResult.value.dependencies
    .filter(d => !d.installed)
    .map(d => d.name)
})

const isEnvReady = computed(() => {
  const env = envResult.value
  if (!env) return false
  if (!env.venv_exists) return false
  return missingDeps.value.length === 0
})

const envStatus = computed<'Ready' | 'Warning' | 'Failed' | 'Action Required'>(() => {
  const env = envResult.value
  if (!env) return 'Failed'
  if (!env.python_path) return 'Failed'
  if (!env.venv_exists) return 'Action Required'
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
    await invoke('validate_project_directory', { projectPath: projectPath.value })
    const result = await invoke<EnvResult>('detect_python_env', { projectPath: projectPath.value })
    envResult.value = result
    phase.value = 'review'
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    phase.value = 'select'
  }
}

async function createVirtualEnv() {
  if (!envResult.value?.python_path) return
  phase.value = 'detecting'
  error.value = null
  try {
    await invoke<string>('create_virtual_env', {
      projectPath: projectPath.value,
      pythonExecutable: envResult.value.python_path
    })
    await detectEnvironment()
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    phase.value = 'review'
  }
}

async function installMissingDeps() {
  if (!envResult.value?.python_path || missingDeps.value.length === 0) return

  phase.value = 'installing'
  error.value = null
  stepStatus.value = {} // 重置状态

  // 监听单步状态事件 (不再需要更新 logs)
  const unlisten = await listen<{ package: string; status: string }>('install_step', (event) => {
    const { package: pkg, status } = event.payload
    stepStatus.value[pkg] = status as 'starting' | 'success' | 'failed'
  })

  try {
    const result = await invoke<{
      success: boolean
      installed: string[]
      failed: string[]
    }>('install_dependencies', {
      pythonPath: envResult.value.python_path,
      packages: missingDeps.value,
    })

    if (result.success) {
      await detectEnvironment() // 刷新状态
    } else {
      // 如果有失败的，直接通过 error 提示，不再用大段 log
      error.value = `Failed to install: ${result.failed.join(', ')}. Please check your network or try again.`
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    unlisten()
    phase.value = 'review'
  }
}

async function saveProject() {
  if (!isEnvReady.value || !envResult.value) return
  phase.value = 'saving'
  try {
    const db = await getDatabase()
    const existing = await db.select<{ id: number }[]>(
      `SELECT id FROM projects WHERE project_path = ?`,
      [projectPath.value]
    )
    if (existing.length > 0) {
      error.value = 'This project has already been imported.'
      phase.value = 'review'
      return
    }
    await db.execute(
      `INSERT INTO projects (name, project_path, interpreter_path, status) VALUES (?, ?, ?, ?)`,
      [projectName.value, projectPath.value, envResult.value.python_path, 'active']
    )
    phase.value = 'done'
    setTimeout(() => router.push("/"), 800)
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
  stepStatus.value = {}
}
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-slate-50">
    <!-- Header (保持不变) -->
    <div class="h-14 px-6 flex items-center justify-between border-b border-zinc-200/80 bg-white">
      <div class="flex items-center gap-3">
        <button @click="router.back()" class="p-1.5 hover:bg-slate-100 rounded-md transition-colors">
          <ArrowLeft class="w-4 h-4 text-slate-600" />
        </button>
        <div class="w-px h-5 bg-zinc-200/80"></div>
        <FolderPlus class="w-4 h-4 text-emerald-500" />
        <h2 class="text-sm font-semibold text-slate-800">Import Project</h2>
      </div>
      <div class="flex items-center gap-2 text-[10px] font-mono text-slate-500">
        <span :class="phase === 'select' ? 'text-emerald-600 font-semibold' : ''">1.SELECT</span>
        <span class="text-slate-300">→</span>
        <span :class="phase === 'detecting' ? 'text-emerald-600 font-semibold' : ''">2.DETECT</span>
        <span class="text-slate-300">→</span>
        <span
          :class="phase === 'review' || phase === 'installing' ? 'text-emerald-600 font-semibold' : ''">3.REVIEW</span>
        <span class="text-slate-300">→</span>
        <span :class="phase === 'saving' || phase === 'done' ? 'text-emerald-600 font-semibold' : ''">4.SAVE</span>
      </div>
    </div>

    <!-- Main Content -->
    <div class="flex-1 overflow-auto flex items-center justify-center p-8">
      <div class="w-full max-w-2xl">

        <!-- Phase 1 & 2 (保持不变) -->
        <div v-if="phase === 'select'" class="bg-white border border-zinc-200/80 rounded-md p-8">
          <div class="text-center mb-6">
            <div class="w-14 h-14 bg-emerald-50 rounded-md flex items-center justify-center mx-auto mb-4">
              <FolderOpen class="w-7 h-7 text-emerald-500" />
            </div>
            <h3 class="text-lg font-semibold text-slate-800 mb-1">Select Python Project</h3>
            <p class="text-xs text-slate-500">Choose a local directory containing your Python source code.</p>
          </div>
          <button @click="selectFolder"
            class="w-full py-3 bg-emerald-500 hover:bg-emerald-600 text-white text-sm font-medium rounded-md transition-colors active:scale-[0.99] flex items-center justify-center gap-2">
            <FolderOpen class="w-4 h-4" /> Browse Directory...
          </button>
          <div v-if="error" class="mt-4 p-3 bg-rose-50 border border-rose-200 rounded-md flex items-start gap-2">
            <X class="w-4 h-4 text-rose-500 flex-shrink-0 mt-0.5" />
            <p class="text-xs text-rose-700">{{ error }}</p>
          </div>
          <div class="mt-6 p-3 bg-slate-50 rounded-md">
            <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">Requirements</p>
            <ul class="space-y-1 text-[11px] text-slate-600">
              <li class="flex items-center gap-2">
                <Check class="w-3 h-3 text-emerald-500" /> Contains <code class="font-mono text-slate-800">.py</code> or
                config files
              </li>
              <li class="flex items-center gap-2">
                <Check class="w-3 h-3 text-emerald-500" /> Python 3.10+ installed globally
              </li>
              <li class="flex items-center gap-2">
                <Sparkles class="w-3 h-3 text-emerald-500" /> Virtual environment will be created automatically
              </li>
            </ul>
          </div>
        </div>

        <div v-else-if="phase === 'detecting'" class="bg-white border border-zinc-200/80 rounded-md p-8 text-center">
          <Loader2 class="w-10 h-10 text-emerald-500 animate-spin mx-auto mb-4" />
          <h3 class="text-sm font-semibold text-slate-800 mb-1">Detecting Environment...</h3>
          <p class="text-xs text-slate-500 font-mono truncate">{{ projectPath }}</p>
        </div>

        <!-- Phase 3: Review (Cleaned up) -->
        <div v-else-if="phase === 'review' || phase === 'installing'"
          class="bg-white border border-zinc-200/80 rounded-md overflow-hidden">

          <!-- Project Info -->
          <div class="p-5 border-b border-zinc-200/80">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-sm font-semibold text-slate-800">Project Configuration</h3>
              <span :class="[
                'inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-full',
                envStatus === 'Ready' ? 'bg-emerald-50 text-emerald-600' :
                  envStatus === 'Warning' ? 'bg-amber-50 text-amber-600' :
                    envStatus === 'Action Required' ? 'bg-blue-50 text-blue-600' : 'bg-rose-50 text-rose-600'
              ]">
                <span :class="[
                  'w-1.5 h-1.5 rounded-full',
                  envStatus === 'Ready' ? 'bg-emerald-500' : envStatus === 'Warning' ? 'bg-amber-500' : envStatus === 'Action Required' ? 'bg-blue-500' : 'bg-rose-500'
                ]"></span>
                {{ envStatus === 'Action Required' ? 'Action Required' : envStatus }}
              </span>
            </div>
            <div class="space-y-2">
              <div>
                <label class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider">Project Name</label>
                <input v-model="projectName" type="text"
                  class="mt-1 w-full px-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-500/40" />
              </div>
              <div>
                <label class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider">Project Path</label>
                <p
                  class="mt-1 px-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-xs font-mono text-slate-600 truncate">
                  {{ projectPath }}</p>
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
                <div v-if="envResult?.python_path && envResult.python_version" class="flex items-center gap-2">
                  <span class="text-xs font-mono text-slate-600">
                    {{ envResult.python_version }}
                    <span v-if="!envResult.venv_exists" class="text-[10px] text-slate-400 ml-1">(Global)</span>
                  </span>
                  <Check class="w-3.5 h-3.5 text-emerald-500" />
                </div>
                <div v-else class="flex items-center gap-2">
                  <span class="text-xs text-rose-600">Not found</span>
                  <X class="w-3.5 h-3.5 text-rose-500" />
                </div>
              </div>

              <!-- Venv -->
              <div class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md">
                <div class="flex items-center gap-2">
                  <Package class="w-3.5 h-3.5 text-slate-500" />
                  <span class="text-xs text-slate-700">Virtual Environment</span>
                </div>
                <div v-if="envResult?.venv_exists" class="flex items-center gap-2">
                  <span class="text-xs font-mono text-slate-600 truncate max-w-[220px]">{{ envResult.venv_path }}</span>
                  <Check class="w-3.5 h-3.5 text-emerald-500" />
                </div>
                <div v-else-if="envResult?.python_path" class="flex items-center gap-2">
                  <span class="text-xs text-blue-600">Ready to create</span>
                  <button @click="createVirtualEnv" :disabled="phase === 'detecting'"
                    class="flex items-center gap-1 px-2 py-0.5 bg-blue-500 hover:bg-blue-600 text-white text-[10px] font-medium rounded transition-colors disabled:opacity-50">
                    <Loader2 v-if="phase === 'detecting'" class="w-3 h-3 animate-spin" />
                    <span v-else>Create .venv</span>
                  </button>
                </div>
                <div v-else class="flex items-center gap-2">
                  <span class="text-xs text-rose-600">Python not installed</span>
                  <AlertCircle class="w-3.5 h-3.5 text-rose-500" />
                </div>
              </div>

              <!-- Dependencies (Cleaned up UI) -->
              <div v-if="envResult?.venv_exists">
                <div v-for="dep in envResult?.dependencies" :key="dep.name"
                  class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md transition-colors">
                  <div class="flex items-center gap-2">
                    <Sparkles class="w-3.5 h-3.5 text-slate-500" />
                    <span class="text-xs text-slate-700">{{ dep.name }}</span>
                  </div>

                  <!-- 动态状态指示器 -->
                  <div v-if="stepStatus[dep.name] === 'starting'" class="flex items-center gap-2">
                    <span class="text-xs font-medium text-blue-600">Installing...</span>
                    <Loader2 class="w-3.5 h-3.5 text-blue-500 animate-spin" />
                  </div>
                  <div v-else-if="stepStatus[dep.name] === 'success' || dep.installed" class="flex items-center gap-2">
                    <span class="text-xs font-mono text-slate-600">v{{ dep.version || 'latest' }}</span>
                    <Check class="w-3.5 h-3.5 text-emerald-500" />
                  </div>
                  <div v-else-if="stepStatus[dep.name] === 'failed' || !dep.installed" class="flex items-center gap-2">
                    <span class="text-xs text-rose-600">
                      {{ stepStatus[dep.name] === 'failed' ? 'Failed' : 'Missing' }}
                    </span>
                    <X class="w-3.5 h-3.5 text-rose-500" />
                  </div>
                </div>
              </div>

              <div v-else-if="envResult?.python_path" class="px-3 py-3 bg-blue-50 rounded-md border border-blue-100">
                <p class="text-xs text-blue-700 flex items-start gap-2">
                  <AlertCircle class="w-3.5 h-3.5 flex-shrink-0 mt-0.5" />
                  A virtual environment is required. Click "Create .venv" above to proceed.
                </p>
              </div>
            </div>
          </div>

          <div v-if="phase === 'installing'" class="px-5 pb-2">
            <div class="p-2 bg-blue-50 border border-blue-200 rounded-md flex items-center gap-2">
              <Loader2 class="w-3.5 h-3.5 text-blue-500 animate-spin shrink-0" />
              <p class="text-xs text-blue-700">
                Installing dependencies. Please do not close this window to avoid corrupting the virtual environment.
              </p>
            </div>
          </div>

          <!-- Actions -->
          <div class="p-5 flex items-center justify-between gap-3">
            <button @click="reset"
              class="px-4 py-2 bg-white border border-zinc-200/80 text-xs font-medium text-slate-600 rounded-md hover:bg-slate-50 transition-colors active:scale-[0.98]">
              Cancel
            </button>

            <div class="flex items-center gap-2">
              <button v-if="missingDeps.length > 0 && phase === 'review'" @click="installMissingDeps"
                class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-white text-xs font-medium rounded-md transition-colors active:scale-[0.98] flex items-center gap-1.5">
                <Sparkles class="w-3.5 h-3.5" />
                Install Missing ({{ missingDeps.length }})
              </button>

              <button @click="saveProject" :disabled="!isEnvReady || phase === 'installing' || phase === 'saving'"
                class="px-4 py-2 bg-emerald-500 hover:bg-emerald-600 disabled:bg-slate-300 disabled:cursor-not-allowed text-white text-xs font-medium rounded-md transition-colors active:scale-[0.98] flex items-center gap-1.5">
                <Loader2 v-if="phase === 'saving'" class="w-3.5 h-3.5 animate-spin" />
                <Check v-else class="w-3.5 h-3.5" />
                {{ phase === 'saving' ? 'Saving...' : 'Import Project' }}
              </button>
            </div>
          </div>

          <!-- 错误提示 (保留，用于显示整体失败或数据库错误) -->
          <div v-if="error" class="px-5 pb-5">
            <div class="p-3 bg-rose-50 border border-rose-200 rounded-md flex items-start gap-2">
              <X class="w-4 h-4 text-rose-500 flex-shrink-0 mt-0.5" />
              <p class="text-xs text-rose-700">{{ error }}</p>
            </div>
          </div>
        </div>

        <!-- Phase 4: Done (保持不变) -->
        <div v-else-if="phase === 'done'" class="bg-white border border-emerald-500/30 rounded-md p-8 text-center">
          <div class="w-14 h-14 bg-emerald-50 rounded-full flex items-center justify-center mx-auto mb-4">
            <Check class="w-7 h-7 text-emerald-500" />
          </div>
          <h3 class="text-lg font-semibold text-slate-800 mb-1">Project Imported Successfully!</h3>
          <p class="text-xs text-slate-500 mb-4">Redirecting to Dashboard...</p>
          <p class="text-[11px] font-mono text-slate-600 truncate">{{ projectName }}</p>
        </div>

      </div>
    </div>
  </div>
</template>