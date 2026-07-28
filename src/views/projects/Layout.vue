<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { openPath } from '@tauri-apps/plugin-opener'
import {
  ArrowLeft,
  RefreshCw,
  FolderOpen,
  Play,
  WandSparkles,
  Gauge,
  FileText,
  CheckCircle2,
  AlertCircle,
  XCircle,
  Clock3,
  ExternalLink,
  Settings2,
} from '@lucide/vue'
import { useProjectStore, type Project } from '../../stores/projectStore'
import { invoke } from '@tauri-apps/api/core'

type ProjectRouteName = 'ProjectExecute' | 'ProjectGenerate' | 'ProjectCoverage'

const route = useRoute()
const router = useRouter()
const projectStore = useProjectStore()

const isRefreshing = ref(false)
const localError = ref<string | null>(null)

const projectId = computed(() => Number(route.params.id))

const currentProject = computed<Project | undefined>(() => {
  if (Number.isNaN(projectId.value)) return undefined
  return projectStore.projects.find((p) => p.id === projectId.value)
})

const navItems = computed(() => [
  {
    name: 'ProjectExecute' as const,
    label: '执行',
    desc: '运行测试、查看结果',
    icon: Play,
  },
  {
    name: 'ProjectGenerate' as const,
    label: '生成',
    desc: '生成测试用例 / 回归测试',
    icon: WandSparkles,
  },
  {
    name: 'ProjectCoverage' as const,
    label: '覆盖率',
    desc: '查看覆盖率统计',
    icon: Gauge,
  },
])

const currentSection = computed(() => {
  return navItems.value.find((item) => item.name === route.name) ?? navItems.value[0]
})

const statusMeta = computed(() => {
  const status = currentProject.value?.env_status ?? 'Warning'

  switch (status) {
    case 'Ready':
      return {
        text: 'Ready',
        color: 'bg-emerald-50 text-emerald-700 border-emerald-200',
        icon: CheckCircle2,
      }
    case 'Warning':
      return {
        text: 'Warning',
        color: 'bg-amber-50 text-amber-700 border-amber-200',
        icon: AlertCircle,
      }
    case 'Failed':
      return {
        text: 'Failed',
        color: 'bg-rose-50 text-rose-700 border-rose-200',
        icon: XCircle,
      }
    case 'active':
      return {
        text: 'Active',
        color: 'bg-sky-50 text-sky-700 border-sky-200',
        icon: Clock3,
      }
    default:
      return {
        text: String(status),
        color: 'bg-slate-50 text-slate-700 border-slate-200',
        icon: AlertCircle,
      }
  }
})

const safeCoverage = computed(() => {
  const value = currentProject.value?.coverage
  if (typeof value !== 'number') return 0
  return Math.max(0, Math.min(100, value))
})

function goBack() {
  router.push({ name: 'Dashboard' })
}

function goToSection(name: ProjectRouteName) {
  if (Number.isNaN(projectId.value)) return
  router.push({
    name,
    params: { id: projectId.value },
  })
}

async function openProjectFolder() {
  const projectPath = currentProject.value?.path;
  if (!projectPath) return;

  try {
    await invoke("open_in_file_manager", { path: projectPath });
  } catch (error) {
    console.error("[ProjectLayout] openProjectFolder failed:", error);
  }
}

async function refreshProject() {
  if (Number.isNaN(projectId.value)) {
    localError.value = '项目 ID 无效'
    return
  }

  isRefreshing.value = true
  localError.value = null

  try {
    await projectStore.fetchProjects()
    await projectStore.updateLastOpened(projectId.value)
  } catch (error) {
    console.error('[ProjectLayout] refreshProject failed:', error)
    localError.value = '刷新项目数据失败'
  } finally {
    isRefreshing.value = false
  }
}

async function ensureProjectLoaded() {
  if (Number.isNaN(projectId.value)) {
    localError.value = '项目 ID 无效'
    return
  }

  try {
    localError.value = null

    if (projectStore.projects.length === 0) {
      await projectStore.fetchProjects()
    }

    const found = projectStore.projects.find((p) => p.id === projectId.value)
    if (!found) {
      localError.value = `找不到项目：${projectId.value}`
      return
    }

    await projectStore.updateLastOpened(projectId.value)
  } catch (error) {
    console.error('[ProjectLayout] ensureProjectLoaded failed:', error)
    localError.value = '加载项目失败'
  }
}

watch(
  () => route.params.id,
  () => {
    ensureProjectLoaded()
  },
  { immediate: true }
)

onMounted(() => {
  ensureProjectLoaded()
})
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-slate-50 text-slate-900">
    <!-- Left sidebar -->
    <aside class="flex w-75 shrink-0 flex-col border-r border-slate-200 bg-white">
      <div class="border-b border-slate-200 px-5 pt-6 pb-5">
        <div class="flex items-center gap-3">
          <div class="flex h-10 w-10 items-center justify-center rounded-2xl bg-emerald-500 text-white shadow-sm">
            <Settings2 class="h-5 w-5" />
          </div>
          <div>
            <div class="text-sm font-semibold text-slate-900">Project Context</div>
            <div class="text-xs text-slate-500">执行 / 生成 / 覆盖率</div>
          </div>
        </div>

        <div class="mt-4 rounded-2xl border border-slate-200 bg-slate-50 p-4">
          <div class="text-[11px] font-medium uppercase tracking-wider text-slate-500">
            当前项目
          </div>

          <div class="mt-2 text-base font-semibold text-slate-900">
            {{ currentProject?.name ?? '未加载' }}
          </div>

          <div class="mt-1 break-all text-xs leading-5 text-slate-500">
            {{ currentProject?.path ?? '正在读取项目路径...' }}
          </div>

          <div class="mt-3 flex items-center gap-2">
            <span
              class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium"
              :class="statusMeta.color"
            >
              <component :is="statusMeta.icon" class="h-3.5 w-3.5" />
              {{ statusMeta.text }}
            </span>

            <span class="text-[11px] text-slate-500">
              ID {{ projectId }}
            </span>
          </div>
        </div>
      </div>

      <div class="flex-1 overflow-auto px-3 py-4">
        <div class="px-2 pb-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400">
          页面导航
        </div>

        <div class="space-y-1">
          <button
            v-for="item in navItems"
            :key="item.name"
            type="button"
            @click="goToSection(item.name)"
            class="w-full rounded-2xl border px-3 py-3 text-left transition hover:bg-slate-50"
            :class="
              route.name === item.name
                ? 'border-emerald-200 bg-emerald-50'
                : 'border-transparent bg-white'
            "
          >
            <div class="flex items-start gap-3">
              <div
                class="mt-0.5 flex h-9 w-9 items-center justify-center rounded-xl"
                :class="
                  route.name === item.name
                    ? 'bg-emerald-500 text-white'
                    : 'bg-slate-100 text-slate-600'
                "
              >
                <component :is="item.icon" class="h-4 w-4" />
              </div>

              <div class="min-w-0 flex-1">
                <div class="text-sm font-medium text-slate-900">
                  {{ item.label }}
                </div>
                <div class="mt-0.5 text-xs leading-5 text-slate-500">
                  {{ item.desc }}
                </div>
              </div>
            </div>
          </button>
        </div>
      </div>

      <div class="border-t border-slate-200 p-4">
        <button
          type="button"
          @click="goBack"
          class="flex w-full items-center justify-center gap-2 rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm font-medium text-slate-700 transition hover:bg-slate-50"
        >
          <ArrowLeft class="h-4 w-4" />
          返回 Dashboard
        </button>
      </div>
    </aside>

    <!-- Main area -->
    <main class="flex min-w-0 flex-1 flex-col">
      <header class="border-b border-slate-200 bg-white px-6 py-4">
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <div class="flex items-center gap-2 text-sm text-slate-500">
              <button
                type="button"
                class="inline-flex items-center gap-1 rounded-lg px-2 py-1 transition hover:bg-slate-100"
                @click="goBack"
              >
                <ArrowLeft class="h-4 w-4" />
                Projects
              </button>
              <span>/</span>
              <span class="font-medium text-slate-700">
                Project {{ projectId }}
              </span>
              <span>/</span>
              <span class="font-medium text-slate-900">
                {{ currentSection.label }}
              </span>
            </div>

            <div class="mt-2 flex items-center gap-3">
              <h1 class="truncate text-xl font-semibold text-slate-900">
                {{ currentProject?.name ?? 'Project Context' }}
              </h1>

              <span
                class="inline-flex items-center rounded-full border px-2.5 py-1 text-[11px] font-medium text-slate-500"
              >
                <FileText class="mr-1.5 h-3.5 w-3.5" />
                {{ currentProject?.interpreter_path ?? 'No interpreter' }}
              </span>
            </div>

            <p class="mt-1 truncate text-sm text-slate-500">
              {{ currentProject?.path ?? '等待项目数据加载...' }}
            </p>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <button
              type="button"
              @click="openProjectFolder"
              class="inline-flex items-center gap-2 rounded-2xl border border-slate-200 bg-white px-4 py-2.5 text-sm font-medium text-slate-700 transition hover:bg-slate-50"
            >
              <FolderOpen class="h-4 w-4" />
              打开目录
            </button>

            <button
              type="button"
              @click="refreshProject"
              :disabled="isRefreshing"
              class="inline-flex items-center gap-2 rounded-2xl bg-emerald-500 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-600 disabled:cursor-not-allowed disabled:opacity-60"
            >
              <RefreshCw :class="['h-4 w-4', isRefreshing ? 'animate-spin' : '']" />
              刷新
            </button>
          </div>
        </div>
      </header>

      <div v-if="localError" class="px-6 pt-4">
        <div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
          {{ localError }}
        </div>
      </div>

      <section class="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-hidden p-4 xl:grid-cols-[1fr_320px]">
        <!-- Child page area -->
        <div class="min-h-0 overflow-hidden rounded-3xl border border-slate-200 bg-white shadow-sm">
          <div class="flex items-center justify-between border-b border-slate-200 px-5 py-4">
            <div>
              <div class="text-sm font-semibold text-slate-900">
                {{ currentSection.label }}
              </div>
              <div class="mt-0.5 text-xs text-slate-500">
                {{ currentSection.desc }}
              </div>
            </div>

            <div class="flex items-center gap-2 text-xs text-slate-500">
              <span class="rounded-full bg-slate-100 px-2.5 py-1">
                Route: {{ String(route.name ?? '') }}
              </span>
            </div>
          </div>

          <div class="h-[calc(100%-65px)] min-h-0 overflow-auto p-5">
            <RouterView />
          </div>
        </div>

        <!-- Right info panel -->
        <aside class="min-h-0 overflow-auto rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-slate-900">
            <ExternalLink class="h-4 w-4 text-slate-500" />
            项目概览
          </div>

          <div class="mt-4 space-y-3">
            <div class="rounded-2xl bg-slate-50 p-4">
              <div class="text-[11px] font-medium uppercase tracking-wider text-slate-500">
                Coverage
              </div>
              <div class="mt-2 text-3xl font-semibold text-slate-900">
                {{ safeCoverage.toFixed(1) }}%
              </div>
              <div class="mt-3 h-2 rounded-full bg-slate-200">
                <div
                  class="h-2 rounded-full bg-emerald-500 transition-all"
                  :style="{ width: `${safeCoverage}%` }"
                />
              </div>
            </div>

            <div class="grid grid-cols-2 gap-3">
              <div class="rounded-2xl border border-slate-200 p-4">
                <div class="text-[11px] text-slate-500">Pass</div>
                <div class="mt-1 text-xl font-semibold text-slate-900">
                  {{ currentProject?.tests_passed ?? 0 }}
                </div>
              </div>

              <div class="rounded-2xl border border-slate-200 p-4">
                <div class="text-[11px] text-slate-500">Fail</div>
                <div class="mt-1 text-xl font-semibold text-slate-900">
                  {{ currentProject?.tests_failed ?? 0 }}
                </div>
              </div>
            </div>

            <div class="rounded-2xl border border-slate-200 p-4">
              <div class="text-[11px] font-medium uppercase tracking-wider text-slate-500">
                Last Run
              </div>
              <div class="mt-2 text-sm font-medium text-slate-900">
                {{ currentProject?.last_run ?? 'Never' }}
              </div>
            </div>

            <div class="rounded-2xl border border-slate-200 p-4">
              <div class="text-[11px] font-medium uppercase tracking-wider text-slate-500">
                快速提示
              </div>
              <ul class="mt-2 space-y-2 text-sm leading-6 text-slate-600">
                <li>• 执行页负责跑测试</li>
                <li>• 生成页负责生成用例</li>
                <li>• 覆盖率页负责展示统计</li>
              </ul>
            </div>
          </div>
        </aside>
      </section>
    </main>
  </div>
</template>

<style scoped>
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition:
    opacity 160ms ease,
    transform 160ms ease;
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>