<script setup lang="ts">
import { onMounted, computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useProjectStore } from "../stores/projectStore";
import { setLocale } from "../i18n";
import {
  FolderPlus,
  GitBranch,
  Sparkles,
  BookOpen,
  Bug,
  Search,
  SlidersHorizontal,
  ArrowUpDown,
  Code2,
  Database,
  X,
  Terminal,
} from "@lucide/vue";
import { ask, open } from "@tauri-apps/plugin-dialog";

const router = useRouter();
const projectStore = useProjectStore();
const { t, locale } = useI18n();
const currentLocale = computed(() => locale.value);

function switchLocale(lang: "en" | "zh") {
  setLocale(lang);
}

// Clone Repository 状态
const showCloneModal = ref(false);
const cloneUrl = ref("");
const cloneTargetDir = ref("");
const isCloning = ref(false);
const cloneError = ref<string | null>(null);

function openCloneModal() {
  cloneUrl.value = "";
  cloneTargetDir.value = "";
  cloneError.value = null;
  showCloneModal.value = true;
}

async function chooseCloneDir() {
  const selected = await open({
    directory: true,
    title: "Choose destination folder",
  });
  if (typeof selected === "string") {
    cloneTargetDir.value = selected;
  }
}

async function cloneRepository() {
  if (!cloneUrl.value.trim() || !cloneTargetDir.value) return;

  isCloning.value = true;
  cloneError.value = null;

  try {
    const clonedPath = await invoke<string>("clone_repository", {
      repoUrl: cloneUrl.value.trim(),
      targetDir: cloneTargetDir.value,
    });

    const repoName = clonedPath.split(/[\\/]/).pop() || "Cloned Project";

    // 克隆后自动探测 Python 环境并写入解释器路径
    let interpreterPath: string | null = null;
    try {
      const env = await invoke<{
        pythonPath: string | null;
      }>("detect_python_env", { projectPath: clonedPath });
      interpreterPath = env.pythonPath;
    } catch (error) {
      console.error("[Dashboard] detect_python_env failed after clone:", error);
    }

    // NFR008：写入 projects 表走 Rust 类型化命令（含重复路径拦截）
    try {
      await invoke("add_project", {
        name: repoName,
        projectPath: clonedPath,
        interpreterPath: interpreterPath,
      });
    } catch (error) {
      cloneError.value =
        error instanceof Error ? error.message : String(error);
      isCloning.value = false;
      return;
    }

    await projectStore.fetchProjects();
    showCloneModal.value = false;
  } catch (error) {
    console.error("[Dashboard] Failed to clone repository:", error);
    cloneError.value = error instanceof Error ? error.message : String(error);
  } finally {
    isCloning.value = false;
  }
}

// 搜索关键词
const searchQuery = ref("");

const filteredProjects = computed(() => {
  // 确保 projects 始终是一个数组，防止 undefined 报错
  const projects = projectStore.projects || [];

  if (!searchQuery.value) return projects;

  const query = searchQuery.value.toLowerCase();
  return projects.filter(
    (p) =>
      // 使用 ?. 防止 name 或 path 为 null/undefined 时 toLowerCase 报错
      p.name?.toLowerCase().includes(query) ||
      p.path?.toLowerCase().includes(query)
  );
});

// 状态颜色映射
const getStatusColor = (status: string) => {
  switch (status) {
    case "Ready":
      return "bg-emerald-500";
    case "Warning":
      return "bg-amber-500";
    case "Failed":
      return "bg-rose-500";
    default:
      return "bg-slate-400";
  }
};

const getStatusTextColor = (status: string) => {
  switch (status) {
    case "Ready":
      return "text-emerald-600";
    case "Warning":
      return "text-amber-600";
    case "Failed":
      return "text-rose-600";
    default:
      return "text-slate-600";
  }
};

// 页面挂载时拉取数据
onMounted(async () => {
  await projectStore.fetchProjects();
  await refreshPythonVersion();
});

// 项目列表变化（导入/删除/刷新）后重新探测 Python 环境
watch(
  () => projectStore.projects,
  () => {
    void refreshPythonVersion();
  },
);

// 状态栏 Python 版本：调用 detect_python_env 获取真实版本
// 取第一个带 interpreter_path 的项目（否则取第一个项目）作为代表环境
const pythonVersion = ref<string | null>(null);
const pythonVenv = ref<boolean | null>(null);

async function refreshPythonVersion() {
  const candidate =
    projectStore.projects.find((p) => p.interpreter_path) ??
    projectStore.projects[0];

  if (!candidate?.path) {
    pythonVersion.value = null;
    pythonVenv.value = null;
    return;
  }

  try {
    const result = await invoke<{
      pythonPath: string | null;
      pythonVersion: string | null;
      venvPath: string | null;
      venvExists: boolean;
      dependencies: {
        name: string;
        installed: boolean;
        version: string | null;
      }[];
    }>("detect_python_env", { projectPath: candidate.path });

    pythonVersion.value = result.pythonVersion;
    pythonVenv.value = result.venvExists;
  } catch (error) {
    console.error("[Dashboard] detect_python_env failed:", error);
    pythonVersion.value = null;
    pythonVenv.value = null;
  }
}

// 路由跳转
const goToImport = () => router.push("/projects/import");
const goToProject = (id: number) => router.push(`/projects/${id}`);

const handleDelete = async (projectId: number, projectName: string) => {
  const deleteConfirm = await ask(t("dashboard.deleteConfirm", { name: projectName }))
  if (!deleteConfirm) {
    return
  }

  try {
    await projectStore.deleteProject(projectId)
    console.log(t("dashboard.deleteSuccess"))
  } catch (error) {
    console.error(t("dashboard.deleteFailed"), error)
  }
}
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-slate-50 text-slate-900">
    <!-- LEFT PANE: Actions & Navigation -->
    <aside class="w-64 bg-white border-r border-zinc-200/80 flex flex-col">
      <div class="px-6 pt-8 pb-6">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 bg-linear-to-br from-emerald-500 to-emerald-600 rounded-md flex items-center justify-center shadow-sm">
            <img src="/src/assets/app-icon-sm.png" />
          </div>
          <div>
            <h1 class="text-sm font-semibold text-slate-800">Testmate</h1>
            <p class="text-[10px] text-slate-500">Testing & Coverage Suite</p>
          </div>
        </div>
      </div>

      <div class="px-4 flex-1">
        <p class="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">
          {{ t("dashboard.quickActions") }}
        </p>
        <div class="space-y-0.5">
          <button @click="goToImport"
            class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left group active:scale-[0.99]">
            <div
              class="w-7 h-7 bg-emerald-50 rounded-md flex items-center justify-center group-hover:bg-emerald-100 transition-colors">
              <FolderPlus class="w-3.5 h-3.5 text-emerald-500" />
            </div>
            <div>
              <p class="text-[13px] font-medium text-slate-800">
                {{ t("dashboard.importProject") }}
              </p>
              <p class="text-[10px] text-slate-500">{{ t("dashboard.importProjectDesc") }}</p>
            </div>
          </button>

          <button @click="openCloneModal"
            class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left group active:scale-[0.99]">
            <div
              class="w-7 h-7 bg-blue-50 rounded-md flex items-center justify-center group-hover:bg-blue-100 transition-colors">
              <GitBranch class="w-3.5 h-3.5 text-blue-500" />
            </div>
            <div>
              <p class="text-[13px] font-medium text-slate-800">
                {{ t("dashboard.cloneRepository") }}
              </p>
              <p class="text-[10px] text-slate-500">{{ t("dashboard.cloneRepositoryDesc") }}</p>
            </div>
          </button>

          <button
            class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left group active:scale-[0.99]">
            <div
              class="w-7 h-7 bg-indigo-50 rounded-md flex items-center justify-center group-hover:bg-indigo-100 transition-colors">
              <Sparkles class="w-3.5 h-3.5 text-indigo-500" />
            </div>
            <div>
              <p class="text-[13px] font-medium text-slate-800">
                {{ t("dashboard.aiTestGenerator") }}
              </p>
              <p class="text-[10px] text-slate-500">Powered by Pynguin</p>
            </div>
          </button>
        </div>

        <p class="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-wider mt-6 mb-2">
          Resources
        </p>
        <div class="space-y-0.5">
          <a href="#" class="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left">
            <BookOpen class="w-3.5 h-3.5 text-slate-400" />
            <span class="text-[13px] text-slate-600">Documentation</span>
          </a>
          <a href="#" class="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left">
            <Bug class="w-3.5 h-3.5 text-slate-400" />
            <span class="text-[13px] text-slate-600">Report Issue</span>
          </a>
        </div>
      </div>
      <div class="px-4 mt-4 border-t border-zinc-200/80 pt-4">
        <p class="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">
          Debug
        </p>
        <button @click="$router.push('/debug/schema')"
          class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left">
          <Database class="w-3.5 h-3.5 text-slate-400" />
          <span class="text-[13px] text-slate-600">View DB Schema</span>
        </button>
      </div>

      <div class="px-6 py-4 border-t border-zinc-200/80 flex items-center justify-between gap-2">
        <p class="text-[10px] text-slate-400 font-mono">
          v1.2.0 · Build 2024.3
        </p>
        <div class="flex items-center gap-1 rounded-md border border-zinc-200 p-0.5">
          <button
            v-for="lang in ['en', 'zh'] as const"
            :key="lang"
            type="button"
            class="px-1.5 py-0.5 text-[10px] font-medium rounded transition"
            :class="currentLocale === lang ? 'bg-emerald-500 text-white' : 'text-slate-500 hover:bg-slate-100'"
            @click="switchLocale(lang)"
          >
            {{ lang === "en" ? "EN" : "中文" }}
          </button>
        </div>
      </div>
    </aside>

    <!-- RIGHT PANE: Project Management -->
    <main class="flex-1 flex flex-col overflow-hidden bg-slate-50">
      <!-- Toolbar -->
      <div class="h-14 px-6 flex items-center justify-between border-b border-zinc-200/80 bg-white">
        <div class="flex items-center gap-3">
          <h2 class="text-sm font-semibold text-slate-800">{{ t("dashboard.projectsTitle") }}</h2>
          <span class="px-1.5 py-0.5 bg-slate-100 text-slate-500 text-[10px] font-mono rounded">
            {{ projectStore.projects.length }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <div class="relative">
            <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
            <input v-model="searchQuery" type="text" :placeholder="t('dashboard.searchPlaceholder')"
              class="pl-8 pr-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-xs w-56 focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-500/40 transition-all" />
          </div>
          <button
            class="flex items-center gap-1.5 px-2.5 py-1.5 bg-white border border-zinc-200/80 rounded-md text-xs text-slate-600 hover:bg-slate-50 transition-all active:scale-[0.98]">
            <SlidersHorizontal class="w-3.5 h-3.5" />
            <span>{{ t("dashboard.filter") }}</span>
          </button>
          <button
            class="flex items-center gap-1.5 px-2.5 py-1.5 bg-white border border-zinc-200/80 rounded-md text-xs text-slate-600 hover:bg-slate-50 transition-all active:scale-[0.98]">
            <ArrowUpDown class="w-3.5 h-3.5" />
            <span>{{ t("dashboard.sortLastOpened") }}</span>
          </button>
        </div>
      </div>

      <!-- Content Area -->
      <div class="flex-1 overflow-auto">
        <!-- Empty State (当没有项目或搜索无结果时) -->
        <div v-if="projectStore.isLoading" class="flex flex-col items-center justify-center h-full text-slate-400">
          <div class="w-8 h-8 border-2 border-slate-200 border-t-emerald-500 rounded-full animate-spin mb-3"></div>
          <p class="text-sm">{{ t("dashboard.loadingProjects") }}</p>
        </div>

        <div v-else-if="filteredProjects.length === 0"
          class="flex flex-col items-center justify-center h-full text-slate-400">
          <FolderPlus class="w-12 h-12 mb-3 text-slate-300" />
          <p class="text-sm font-medium text-slate-600 mb-1">
            {{
              searchQuery
                ? t("dashboard.emptyState.searchNoMatch")
                : t("dashboard.emptyState.title")
            }}
          </p>
          <p class="text-xs mb-4">
            {{ t("dashboard.emptyState.description") }}
          </p>
          <button @click="goToImport"
            class="px-4 py-2 bg-emerald-500 text-white text-xs font-medium rounded-md hover:bg-emerald-600 transition-colors active:scale-[0.98]">
            {{ t("dashboard.importProject") }}
          </button>
        </div>

        <!-- Project List -->
        <div v-else>
          <!-- Table Header -->
          <div
            class="px-6 py-2 bg-slate-50 border-b border-zinc-200/80 grid grid-cols-[4fr_1.5fr_2fr_2.5fr_1.5fr] gap-4 text-[10px] font-semibold text-slate-400 uppercase tracking-wider sticky top-0 z-10">
            <div>{{ t("dashboard.projectTable.project") }}</div>
            <div>{{ t("dashboard.projectTable.environment") }}</div>
            <div>{{ t("dashboard.projectTable.testsPassed") }}</div>
            <div>{{ t("dashboard.projectTable.coverage") }}</div>
            <div class="text-right">{{ t("dashboard.projectTable.lastRun") }}</div>
          </div>

          <!-- Rows -->
          <div class="divide-y divide-zinc-200/80 bg-white">
            <div v-for="project in filteredProjects" :key="project.id" @click="goToProject(project.id)"
              @contextmenu="handleDelete(project.id, project.name)"
              class="px-6 py-3.5 hover:bg-slate-50 transition-colors cursor-pointer grid grid-cols-[4fr_1.5fr_2fr_2.5fr_1.5fr] gap-4 items-center group">
              <!-- Project Info -->
              <div class="flex items-center gap-3 min-w-0">
                <div
                  class="w-8 h-8 bg-linear-to-br from-blue-500 to-blue-600 rounded-md flex items-center justify-center shrink-0">
                  <Code2 class="w-4 h-4 text-white" />
                </div>
                <div class="min-w-0">
                  <h4 class="text-[13px] font-medium text-slate-800 truncate">
                    {{ project.name }}
                  </h4>
                  <p class="text-[10px] text-slate-500 font-mono truncate">
                    {{ project.path }}
                  </p>
                </div>
              </div>

              <!-- Environment Status with Tooltip -->
              <div class="relative">
                <button class="flex items-center gap-1.5 px-2 py-1 rounded hover:bg-slate-100 transition-colors">
                  <span :class="`w-1.5 h-1.5 rounded-full ${getStatusColor(project.env_status)}`"></span>
                  <span :class="`text-[11px] ${getStatusTextColor(project.env_status)}`">{{ project.env_status }}</span>
                </button>
                <!-- Tooltip -->
                <!-- <div
                  class="absolute top-full left-0 mt-1.5 w-52 bg-white border border-zinc-200 rounded-md shadow-md p-2.5 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-20 pointer-events-none">
                  <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-1.5">
                    Environment Status
                  </p>
                  <ul class="space-y-1">
                    <li v-for="(detail, idx) in project.env_details" :key="idx"
                      class="flex items-center gap-2 text-[11px]">
                      <component :is="getEnvIcon(detail)"
                        :class="`w-3 h-3 ${detail.toLowerCase().includes('missing') || detail.toLowerCase().includes('not found') ? 'text-rose-500' : 'text-emerald-500'}`" />
                      <span :class="detail.toLowerCase().includes('missing') ||
                        detail.toLowerCase().includes('not found')
                        ? 'text-rose-700'
                        : 'text-slate-700'
                        ">
                        {{ detail }}
                      </span>
                    </li>
                  </ul>
                </div> -->
              </div>

              <!-- Test Result -->
              <div class="flex items-center gap-2">
                <span class="text-[11px] font-mono text-emerald-600">{{ t("dashboard.passedCount", { count: project.tests_passed }) }}</span>
                <span class="text-slate-300">·</span>
                <span class="text-[11px] font-mono text-rose-600">{{ t("dashboard.failedCount", { count: project.tests_failed }) }}</span>
              </div>

              <!-- Coverage -->
              <div class="flex items-center gap-2">
                <div class="flex-1 bg-slate-100 rounded-full h-1.5">
                  <div :class="`h-1.5 rounded-full ${project.coverage < 80 ? 'bg-amber-500' : 'bg-emerald-500'}`"
                    :style="{ width: `${project.coverage}%` }"></div>
                </div>
                <span class="text-[11px] font-mono text-slate-600 w-10 text-right">{{ project.coverage }}%</span>
              </div>

              <!-- Last Run -->
              <div class="text-right">
                <span class="text-[11px] text-slate-500">{{
                  project.last_run ?? t("common.never")
                }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- BOTTOM STATUS BAR -->
      <div
        class="h-7 px-4 flex items-center justify-between border-t border-zinc-200/80 bg-white text-[10px] text-slate-500">
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-1.5">
            <Terminal class="w-3 h-3" />
            <span class="font-mono">{{
              pythonVersion
                ? `${pythonVersion}${pythonVenv ? " (venv)" : " (global)"}`
                : t("app.statusBar.pythonNotDetected")
            }}</span>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 bg-emerald-500 rounded-full"></span>
            <span>{{ t("app.statusBar.ipcConnected") }}</span>
          </div>
        </div>
        <div class="flex items-center gap-4 font-mono">
          <span>{{ t("app.statusBar.projects", { count: projectStore.stats.total_projects }) }}</span>
          <span>{{ t("app.statusBar.totalRuns", { count: projectStore.stats.total_runs }) }}</span>
        </div>
      </div>
    </main>

    <!-- Clone Repository Modal -->
    <div v-if="showCloneModal" class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4"
      @click.self="showCloneModal = false">
      <div class="w-full max-w-md rounded-2xl bg-white p-6 shadow-xl">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h3 class="text-sm font-semibold text-slate-900">{{ t("dashboard.cloneModal.title") }}</h3>

            <p class="mt-1 text-xs text-slate-500">
              {{ t("dashboard.cloneModal.description") }}
            </p>
          </div>

          <button type="button" class="rounded-lg p-1 text-slate-400 transition hover:bg-slate-100 hover:text-slate-600"
            @click="showCloneModal = false">
            <X class="h-4 w-4" />
          </button>
        </div>

        <label class="mt-4 block text-xs font-medium text-slate-700">
          {{ t("dashboard.cloneModal.repoUrl") }}
          <input v-model="cloneUrl" type="text" :placeholder="t('dashboard.cloneModal.repoUrlPlaceholder')"
            class="mt-1.5 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-blue-400 focus:ring-2 focus:ring-blue-100"
            @keyup.enter="cloneRepository" />
        </label>

        <label class="mt-4 block text-xs font-medium text-slate-700">
          {{ t("dashboard.cloneModal.destFolder") }}
          <button type="button"
            class="mt-1.5 flex w-full items-center justify-between gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm transition hover:bg-slate-50"
            @click="chooseCloneDir">
            <span :class="cloneTargetDir ? 'text-slate-700' : 'text-slate-400'">
              {{ cloneTargetDir || t("dashboard.cloneModal.chooseFolder") }}
            </span>
            <FolderPlus class="h-4 w-4 shrink-0 text-slate-400" />
          </button>
        </label>

        <div v-if="cloneError" class="mt-3 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-xs text-rose-700">
          {{ cloneError }}
        </div>

        <div class="mt-5 flex items-center justify-end gap-2">
          <button type="button"
            class="rounded-xl px-4 py-2 text-sm font-medium text-slate-600 transition hover:bg-slate-100"
            @click="showCloneModal = false">
            {{ t("common.cancel") }}
          </button>

          <button type="button" :disabled="!cloneUrl.trim() || !cloneTargetDir || isCloning"
            class="inline-flex items-center gap-2 rounded-xl bg-blue-500 px-5 py-2 text-sm font-medium text-white transition hover:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-50"
            @click="cloneRepository">
            <GitBranch class="h-4 w-4" />
            {{ isCloning ? t("dashboard.cloneModal.cloning") : t("dashboard.cloneModal.cloneImport") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
