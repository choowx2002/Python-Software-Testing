<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'
import EnvFixWizard from '../../components/EnvFixWizard.vue'
import AppTooltip from '../../components/ui/AppTooltip.vue'
import AppErrorBanner from '../../components/ui/AppErrorBanner.vue'
import {
  FolderPlus, FolderOpen, Check, X, AlertCircle,
  ArrowLeft, Loader2, Sparkles, Terminal, Package
} from '@lucide/vue'

const router = useRouter()
const { t } = useI18n()

// ============================================
// State
// ============================================
type Phase = 'select' | 'detecting' | 'review' | 'installing' | 'saving' | 'done'

const phase = ref<Phase>('select')
const projectPath = ref('')
const projectName = ref('')
const error = ref<string | null>(null)

interface EnvResult {
  pythonPath: string | null
  pythonVersion: string | null
  venvPath: string | null
  venvExists: boolean
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
// 🆕 安装失败时的 pip 真实报错（stderr 摘要）
const installErrorReason = ref('')

// ============================================
// Computed
// ============================================
const missingDeps = computed(() => {
  if (!envResult.value?.venvExists) return []
  return envResult.value.dependencies
    .filter(d => !d.installed)
    .map(d => d.name)
})

const isEnvReady = computed(() => {
  const env = envResult.value
  if (!env) return false
  if (!env.venvExists) return false
  return missingDeps.value.length === 0
})

const envStatus = computed<'Ready' | 'Warning' | 'Failed' | 'Action Required'>(() => {
  const env = envResult.value
  if (!env) return 'Failed'
  if (!env.pythonPath) return 'Failed'
  if (!env.venvExists) return 'Action Required'
  if (missingDeps.value.length > 0) return 'Warning'
  return 'Ready'
})

/** Python 3.12+ → Pynguin 生成测试有已知兼容性问题，提醒用户一键换 3.11 */
const pyVersionRisk = computed(() => {
  const v = envResult.value?.pythonVersion ?? ''
  const m = v.match(/3\.(\d+)/)
  return !!m && Number(m[1]) >= 12
})

/** 一键修复完成后重新检测环境 */
async function onEnvFixed() {
  await detectEnvironment()
}

// ============================================
// Actions
// ============================================
async function selectFolder() {
  error.value = null
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('import.selectStep.title'),
    })
    if (!selected) return
    projectPath.value = selected as string
    const parts = projectPath.value.replace(/[/\\]$/, '').split(/[/\\]/)
    projectName.value = parts[parts.length - 1] || t('import.selectStep.untitled')
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
  if (!envResult.value?.pythonPath) return
  phase.value = 'detecting'
  error.value = null
  try {
    await invoke<string>('create_virtual_env', {
      projectPath: projectPath.value,
      pythonExecutable: envResult.value.pythonPath
    })
    await detectEnvironment()
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    phase.value = 'review'
  }
}

async function installMissingDeps() {
  if (!envResult.value?.pythonPath || missingDeps.value.length === 0) return

  phase.value = 'installing'
  error.value = null
  installErrorReason.value = ''
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
      failedReasons?: string[]
    }>('install_dependencies', {
      pythonPath: envResult.value.pythonPath,
      packages: missingDeps.value,
    })

    if (result.success) {
      await detectEnvironment() // 刷新状态
    } else {
      // 显示失败包 + pip 的真实报错（stderr 摘要），不再只给笼统的提示
      error.value = t('import.reviewStep.installFailed', { packages: result.failed.join(', ') })
      installErrorReason.value = result.failedReasons?.[0] ?? ''
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
    // NFR008：写入 projects 表走 Rust 类型化命令（含重复路径拦截）
    await invoke("add_project", {
      name: projectName.value,
      projectPath: projectPath.value,
      interpreterPath: envResult.value.pythonPath,
    })
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

// 模板内 phase 比较：vue-tsc 会基于外层 v-if 收窄联合类型，
// 用函数比较可避免误报（phase 是响应式，运行时会变化）
function isPhase(p: Phase): boolean {
  return phase.value === p
}
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-slate-50">
    <!-- Header -->
    <div class="h-14 px-6 flex items-center justify-between border-b border-zinc-200/80 bg-white">
      <div class="flex items-center gap-3">
        <AppTooltip :content="t('common.back')" position="top">
          <button @click="router.back()" class="p-1.5 hover:bg-slate-100 rounded-md transition-colors">
            <ArrowLeft class="w-4 h-4 text-slate-600" />
          </button>
        </AppTooltip>
        <div class="w-px h-5 bg-zinc-200/80"></div>
        <FolderPlus class="w-4 h-4 text-emerald-500" />
        <h2 class="text-sm font-semibold text-slate-800">{{ t('import.title') }}</h2>
      </div>
      <div class="flex items-center gap-2 text-[10px] font-mono text-slate-500">
        <span :class="phase === 'select' ? 'text-emerald-600 font-semibold' : ''">{{ t('import.stepSelect') }}</span>
        <span class="text-slate-300">→</span>
        <span :class="phase === 'detecting' ? 'text-emerald-600 font-semibold' : ''">{{ t('import.stepDetect') }}</span>
        <span class="text-slate-300">→</span>
        <span
          :class="phase === 'review' || phase === 'installing' ? 'text-emerald-600 font-semibold' : ''">{{ t('import.stepReview') }}</span>
        <span class="text-slate-300">→</span>
        <span :class="phase === 'saving' || phase === 'done' ? 'text-emerald-600 font-semibold' : ''">{{ t('import.stepSave') }}</span>
      </div>
    </div>

    <!-- Main Content -->
    <div class="flex-1 overflow-auto flex items-center justify-center p-8">
      <div class="w-full max-w-2xl">

        <!-- Phase 1 & 2 -->
        <div v-if="phase === 'select'" class="bg-white border border-zinc-200/80 rounded-md p-8">
          <div class="text-center mb-6">
            <div class="w-14 h-14 bg-emerald-50 rounded-md flex items-center justify-center mx-auto mb-4">
              <FolderOpen class="w-7 h-7 text-emerald-500" />
            </div>
            <h3 class="text-lg font-semibold text-slate-800 mb-1">{{ t('import.selectStep.title') }}</h3>
            <p class="text-xs text-slate-500">{{ t('import.selectStep.description') }}</p>
          </div>
          <AppTooltip :content="t('import.selectStep.pickFolder')" position="top">
            <button @click="selectFolder" class="btn btn-primary btn-md w-full">
              <FolderOpen class="w-4 h-4" /> {{ t('import.selectStep.pickFolder') }}
            </button>
          </AppTooltip>
          <!-- 错误使用 ErrorBanner 替代 -->
          <AppErrorBanner
            v-if="error"
            :message="error"
            :hint="t('import.selectStep.errorHint')"
            :actions="[{ label: t('common.retry'), action: selectFolder, variant: 'primary' }]"
            dismissible
            @dismiss="error = null"
          />
          <div class="mt-6 p-3 bg-slate-50 rounded-md">
            <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">{{ t('import.selectStep.requirements') }}</p>
            <ul class="space-y-1 text-[11px] text-slate-600">
              <li class="flex items-center gap-2">
                <Check class="w-3 h-3 text-emerald-500" /> {{ t('import.selectStep.reqPyFiles') }}
              </li>
              <li class="flex items-center gap-2">
                <Check class="w-3 h-3 text-emerald-500" /> {{ t('import.selectStep.reqPython310') }}
              </li>
              <li class="flex items-center gap-2">
                <Sparkles class="w-3 h-3 text-emerald-500" /> {{ t('import.selectStep.reqVenvAuto') }}
              </li>
            </ul>
          </div>
        </div>

        <div v-else-if="phase === 'detecting'" class="bg-white border border-zinc-200/80 rounded-md p-8 text-center">
          <Loader2 class="w-10 h-10 text-emerald-500 animate-spin mx-auto mb-4" />
          <h3 class="text-sm font-semibold text-slate-800 mb-1">{{ t('import.detectStep.title') }}</h3>
          <p class="text-xs text-slate-500 font-mono truncate">{{ projectPath }}</p>
        </div>

        <!-- Phase 3: Review -->
        <div v-else-if="phase === 'review' || phase === 'installing'"
          class="bg-white border border-zinc-200/80 rounded-md overflow-hidden">

          <!-- Project Info -->
          <div class="p-5 border-b border-zinc-200/80">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-sm font-semibold text-slate-800">{{ t('import.reviewStep.projectConfig') }}</h3>
              <span :class="[
                'inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-full',
                envStatus === 'Ready' ? 'bg-emerald-50 text-emerald-600' :
                  envStatus === 'Warning' ? 'bg-amber-50 text-amber-600' :
                    envStatus === 'Action Required' ? 'bg-blue-50 text-blue-600' : 'bg-rose-50 text-rose-600'
              ]">
                <span :class="[
                  'w-1.5 h-1.5 rounded-full',
                  envStatus === 'Ready' ? 'bg-emerald-500' : envStatus === 'Warning' ? 'bg-amber-500' : envStatus === 'Action Required' ? 'bg-sky-500' : 'bg-rose-500'
                ]"></span>
                {{ envStatus === 'Action Required' ? t('import.reviewStep.actionRequired') : envStatus }}
              </span>
            </div>
            <div class="space-y-2">
              <div>
                <AppTooltip :content="t('import.reviewStep.projectNameHint')" position="top">
                  <label class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider">{{ t('import.reviewStep.projectName') }}</label>
                </AppTooltip>
                <input v-model="projectName" type="text"
                  class="mt-1 w-full px-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-brand-500/15 focus:border-brand-500" />
              </div>
              <div>
                <label class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider">{{ t('import.reviewStep.projectPath') }}</label>
                <AppTooltip :content="projectPath" position="top">
                  <p class="mt-1 px-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-xs font-mono text-slate-600 truncate">{{ projectPath }}</p>
                </AppTooltip>
              </div>
            </div>
          </div>

          <!-- Environment Details -->
          <div class="p-5 border-b border-zinc-200/80">
            <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-3">{{ t('import.reviewStep.envTitle') }}</p>

            <!-- Python 3.12 + Pynguin 兼容性提醒 + 一键换 3.11 -->
            <div v-if="pyVersionRisk" class="mb-3 p-3 bg-amber-50 border border-amber-200 rounded-md">
              <div class="flex items-start gap-2">
                <AlertCircle class="w-4 h-4 text-amber-600 flex-shrink-0 mt-0.5" />
                <p class="text-xs leading-5 text-amber-700">{{ t('generate.py312Warning') }}</p>
              </div>
              <div class="mt-2">
                <EnvFixWizard :project-path="projectPath" @fixed="onEnvFixed" />
              </div>
            </div>

            <div class="space-y-2">
              <!-- Python -->
              <div class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md">
                <div class="flex items-center gap-2">
                  <Terminal class="w-3.5 h-3.5 text-slate-500" />
                  <span class="text-xs text-slate-700">{{ t('import.reviewStep.pythonInterpreter') }}</span>
                </div>
                <div v-if="envResult?.pythonPath && envResult.pythonVersion" class="flex items-center gap-2">
                  <span class="text-xs font-mono text-slate-600">
                    {{ envResult.pythonVersion }}
                    <span v-if="!envResult.venvExists" class="text-[10px] text-slate-400 ml-1">{{ t('import.reviewStep.global') }}</span>
                  </span>
                  <Check class="w-3.5 h-3.5 text-emerald-500" />
                </div>
                <div v-else class="flex items-center gap-2">
                  <span class="text-xs text-rose-600">{{ t('import.reviewStep.notFound') }}</span>
                  <X class="w-3.5 h-3.5 text-rose-500" />
                </div>
              </div>

              <!-- Venv -->
              <div class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md">
                <div class="flex items-center gap-2">
                  <Package class="w-3.5 h-3.5 text-slate-500" />
                  <span class="text-xs text-slate-700">{{ t('import.reviewStep.virtualEnv') }}</span>
                </div>
                <div v-if="envResult?.venvExists" class="flex items-center gap-2">
                  <span class="text-xs font-mono text-slate-600 truncate max-w-[220px]">{{ envResult.venvPath }}</span>
                  <Check class="w-3.5 h-3.5 text-emerald-500" />
                </div>
                <div v-else-if="envResult?.pythonPath" class="flex items-center gap-2">
                  <span class="text-xs text-blue-600">{{ t('import.reviewStep.readyToCreate') }}</span>
                  <button @click="createVirtualEnv" :disabled="isPhase('detecting')"
                    class="btn btn-primary btn-sm">
                    <Loader2 v-if="isPhase('detecting')" class="w-3 h-3 animate-spin" />
                    <span v-else>{{ t('import.reviewStep.createVenv') }}</span>
                  </button>
                </div>
                <div v-else class="flex items-center gap-2">
                  <span class="text-xs text-rose-600">{{ t('import.reviewStep.pythonNotInstalled') }}</span>
                  <AlertCircle class="w-3.5 h-3.5 text-rose-500" />
                </div>
              </div>

              <!-- Dependencies (Cleaned up UI) -->
              <div v-if="envResult?.venvExists">
                <div v-for="dep in envResult?.dependencies" :key="dep.name"
                  class="flex items-center justify-between px-3 py-2 bg-slate-50 rounded-md transition-colors">
                  <div class="flex items-center gap-2">
                    <Sparkles class="w-3.5 h-3.5 text-slate-500" />
                    <span class="text-xs text-slate-700">{{ dep.name }}</span>
                  </div>

                  <!-- 动态状态指示器 -->
                  <div v-if="stepStatus[dep.name] === 'starting'" class="flex items-center gap-2">
                    <span class="text-xs font-medium text-blue-600">{{ t('import.reviewStep.installing') }}</span>
                    <Loader2 class="w-3.5 h-3.5 text-blue-500 animate-spin" />
                  </div>
                  <div v-else-if="stepStatus[dep.name] === 'success' || dep.installed" class="flex items-center gap-2">
                    <span class="text-xs font-mono text-slate-600">v{{ dep.version || 'latest' }}</span>
                    <Check class="w-3.5 h-3.5 text-emerald-500" />
                  </div>
                  <div v-else-if="stepStatus[dep.name] === 'failed' || !dep.installed" class="flex items-center gap-2">
                    <span class="text-xs text-rose-600">
                      {{ stepStatus[dep.name] === 'failed' ? t('import.reviewStep.failed') : t('import.reviewStep.missing') }}
                    </span>
                    <X class="w-3.5 h-3.5 text-rose-500" />
                  </div>
                </div>
              </div>

              <div v-else-if="envResult?.pythonPath" class="px-3 py-3 bg-blue-50 rounded-md border border-blue-100">
                <p class="text-xs text-blue-700 flex items-start gap-2">
                  <AlertCircle class="w-3.5 h-3.5 flex-shrink-0 mt-0.5" />
                  {{ t('import.reviewStep.venvRequired') }}
                </p>
              </div>
            </div>
          </div>

          <div v-if="phase === 'installing'" class="px-5 pb-2">
            <div class="p-2 bg-blue-50 border border-blue-200 rounded-md flex items-center gap-2">
              <Loader2 class="w-3.5 h-3.5 text-blue-500 animate-spin shrink-0" />
              <p class="text-xs text-blue-700">
                {{ t('import.reviewStep.installingWarning') }}
              </p>
            </div>
          </div>

          <!-- Actions -->
          <div class="p-5 flex items-center justify-between gap-3">
            <button @click="reset"
              class="px-4 py-2 bg-white border border-zinc-200/80 text-xs font-medium text-slate-600 rounded-md hover:bg-slate-50 transition-colors active:scale-[0.98]">
              {{ t('common.cancel') }}
            </button>

            <div class="flex items-center gap-2">
              <button v-if="missingDeps.length > 0 && phase === 'review'" @click="installMissingDeps"
                class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-white text-xs font-medium rounded-md transition-colors active:scale-[0.98] flex items-center gap-1.5">
                <Sparkles class="w-3.5 h-3.5" />
                {{ t('import.reviewStep.installMissing', { count: missingDeps.length }) }}
              </button>

              <button @click="saveProject" :disabled="!isEnvReady || isPhase('installing') || isPhase('saving')"
                class="btn btn-primary btn-sm">
                <Loader2 v-if="isPhase('saving')" class="w-3.5 h-3.5 animate-spin" />
                <Check v-else class="w-3.5 h-3.5" />
                {{ isPhase('saving') ? t('common.saving') : t('import.reviewStep.importProject') }}
              </button>
            </div>
          </div>

          <!-- 错误提示 (保留，用于显示整体失败或数据库错误) -->
          <div v-if="error" class="px-5 pb-5">
            <div class="p-3 bg-rose-50 border border-rose-200 rounded-md flex items-start gap-2">
              <X class="w-4 h-4 text-rose-500 flex-shrink-0 mt-0.5" />
              <p class="text-xs text-rose-700">{{ error }}</p>
            </div>
            <!-- 安装失败：显示 pip 真实报错（stderr 摘要）与排查提示 -->
            <div v-if="installErrorReason" class="mt-2">
              <pre
                class="max-h-40 overflow-auto rounded-md bg-zinc-950 px-3 py-2 font-mono text-[10px] leading-4 whitespace-pre-wrap text-rose-300">{{ installErrorReason }}</pre>
              <p class="mt-1.5 text-[11px] text-zinc-500">
                {{ t('import.reviewStep.installFailedHint') }}
              </p>
            </div>
          </div>
        </div>

        <!-- Phase 4: Done (保持不变) -->
        <div v-else-if="phase === 'done'" class="bg-white border border-emerald-500/30 rounded-md p-8 text-center">
          <div class="w-14 h-14 bg-emerald-50 rounded-full flex items-center justify-center mx-auto mb-4">
            <Check class="w-7 h-7 text-emerald-500" />
          </div>
          <h3 class="text-lg font-semibold text-slate-800 mb-1">{{ t('import.doneStep.title') }}</h3>
          <p class="text-xs text-slate-500 mb-4">{{ t('import.doneStep.goToDashboard') }}</p>
          <p class="text-[11px] font-mono text-slate-600 truncate">{{ projectName }}</p>
        </div>

      </div>
    </div>
  </div>
</template>