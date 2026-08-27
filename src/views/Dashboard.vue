<script setup lang="ts">
/**
 * Dashboard —— 首页
 * 重构要点（第一批）：
 *  - 侧边栏统一为 AppSidebar（导航/版本/语言）
 *  - 主色统一为 brand 蓝，删除克隆弹窗的蓝色孤岛
 *  - 新手三步引导（导入 → 生成 → 运行测量）替代裸空状态
 *  - 删除无功能的 Filter 按钮；Sort 改为真实可用的排序
 *  - 删除项目增加悬停可见的删除按钮（新手不需要知道右键菜单）
 */
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useProjectStore, type Project } from "../stores/projectStore";
import { useUIStore } from "../stores/uiStore";
import {
  ArrowDown,
  ArrowUp,
  BookOpen,
  Bug,
  Code2,
  Database,
  FolderPlus,
  GitBranch,
  Loader2,
  Search,
  Sparkles,
  Terminal,
  Trash2,
  Edit,
  Copy,
  ExternalLink,
} from "@lucide/vue";
import { open } from "@tauri-apps/plugin-dialog";
import AppSidebar from "../components/AppSidebar.vue";
import AppNavItem from "../components/ui/AppNavItem.vue";
import AppButton from "../components/ui/AppButton.vue";
import AppEmptyState from "../components/ui/AppEmptyState.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import AppModal from "../components/ui/AppModal.vue";
import AppConfirmModal from "../components/ui/AppConfirmModal.vue";
import AppTooltip from "../components/ui/AppTooltip.vue";
import AppContextMenu from "../components/ui/AppContextMenu.vue";
import AppTour from "../components/ui/AppTour.vue";
import { useDashboardTour } from "../composables/useTour";

const router = useRouter();
const projectStore = useProjectStore();
const uiStore = useUIStore();
const { t } = useI18n();

/* Tour: 首次访问引导 */
const tourSteps = useDashboardTour()
const showTour = ref(!uiStore.tourCompleted && projectStore.projects.length === 0)

function onTourComplete() {
  uiStore.completeTour()
  showTour.value = false
}

/* 右键菜单状态 */
const projectMenu = ref<{ project: Project; x: number; y: number } | null>(null)

function showProjectMenu(project: Project, e: MouseEvent) {
  projectMenu.value = { project, x: e.clientX, y: e.clientY }
}

const projectMenuItems = computed(() => {
  if (!projectMenu.value) return []
  const p = projectMenu.value.project
  return [
    { label: t('contextmenu.openProject'), icon: ExternalLink, action: () => router.push(`/projects/${p.id}`) },
    { label: t('contextmenu.openInFolder'), icon: FolderPlus, action: () => invoke('open_in_file_manager', { path: p.path }) },
    { label: t('contextmenu.rename'), icon: Edit, action: () => renameProject(p) },
    { label: t('contextmenu.copyPath'), icon: Copy, action: () => navigator.clipboard.writeText(p.path) },
    { divider: true },
    { label: t('contextmenu.delete'), icon: Trash2, action: () => handleDelete(p), danger: true }
  ]
})

function renameProject(project: Project) {
  const newName = prompt(t('dashboard.renamePrompt', { name: project.name }))
  if (newName && newName.trim() !== project.name) {
    projectStore.updateProject(project.id, newName.trim())
  }
}

/* ------------------------------------------------------------------ */
/* Clone Repository 状态（原逻辑保持不变）                              */
/* ------------------------------------------------------------------ */
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
      const env = await invoke<{ pythonPath: string | null }>("detect_python_env", {
        projectPath: clonedPath,
      });
      interpreterPath = env.pythonPath;
    } catch (error) {
      console.error("[Dashboard] detect_python_env failed after clone:", error);
    }

    try {
      await invoke("add_project", {
        name: repoName,
        projectPath: clonedPath,
        interpreterPath: interpreterPath,
      });
    } catch (error) {
      cloneError.value = error instanceof Error ? error.message : String(error);
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

/* ------------------------------------------------------------------ */
/* 搜索 + 排序                                                         */
/* ------------------------------------------------------------------ */
const searchQuery = ref("");
/** true = 最近打开在前（默认），false = 最久在前 */
const sortDesc = ref(true);

const visibleProjects = computed(() => {
  const projects = projectStore.projects || [];
  const query = searchQuery.value.trim().toLowerCase();

  const filtered = query
    ? projects.filter(
        (p) =>
          p.name?.toLowerCase().includes(query) ||
          p.path?.toLowerCase().includes(query),
      )
    : projects;

  // 按 last_run 排序（从未运行的项目排最后）
  return [...filtered].sort((a, b) => {
    const ta = a.last_run ? new Date(a.last_run).getTime() : 0;
    const tb = b.last_run ? new Date(b.last_run).getTime() : 0;
    return sortDesc.value ? tb - ta : ta - tb;
  });
});

const sortIcon = computed(() => (sortDesc.value ? ArrowDown : ArrowUp));

/* ------------------------------------------------------------------ */
/* 新手三步引导（对应真实工作流：导入 → 生成 → 执行）                    */
/* ------------------------------------------------------------------ */
const welcomeSteps = computed(() => [
  { title: t("dashboard.welcome.step1Title"), desc: t("dashboard.welcome.step1Desc") },
  { title: t("dashboard.welcome.step2Title"), desc: t("dashboard.welcome.step2Desc") },
  { title: t("dashboard.welcome.step3Title"), desc: t("dashboard.welcome.step3Desc") },
]);

/* ------------------------------------------------------------------ */
/* 状态栏：Python 环境探测（原逻辑保持不变）                            */
/* ------------------------------------------------------------------ */
const pythonVersion = ref<string | null>(null);
const pythonVenv = ref<boolean | null>(null);

async function refreshPythonVersion() {
  const candidate =
    projectStore.projects.find((p) => p.interpreter_path) ?? projectStore.projects[0];

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

onMounted(async () => {
  await projectStore.fetchProjects();
  await refreshPythonVersion();
});

watch(
  () => projectStore.projects,
  () => void refreshPythonVersion(),
);

/* ------------------------------------------------------------------ */
/* 导航与删除                                                          */
/* ------------------------------------------------------------------ */
const goToImport = () => router.push("/projects/import");
const goToProject = (id: number) => router.push(`/projects/${id}`);

/** 待删除项目（确认弹窗） */
const deleteTarget = ref<Project | null>(null);

function handleDelete(project: Project) {
  deleteTarget.value = project;
}

async function confirmDeleteProject() {
  const target = deleteTarget.value;
  if (!target) return;

  try {
    await projectStore.deleteProject(target.id);
    console.log(t("dashboard.deleteSuccess"));
  } catch (error) {
    console.error(t("dashboard.deleteFailed"), error);
  } finally {
    deleteTarget.value = null;
  }
}

/** 覆盖率进度条颜色：>=80% 绿，否则琥珀（沿用原阈值） */
const coverageTone = (coverage: number) =>
  coverage >= 80 ? "bg-emerald-500" : "bg-amber-500";
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-surface text-zinc-900">
    <!-- 统一侧边栏 -->
    <AppSidebar>
      <p class="section-label mb-1.5">{{ t("dashboard.quickActions") }}</p>
      <div class="space-y-0.5">
        <AppTooltip :content="t('dashboard.importProjectDesc')" position="right">
          <AppNavItem
            data-tour="import-project"
            :icon="FolderPlus"
            :label="t('dashboard.importProject')"
            :description="t('dashboard.importProjectDesc')"
            @click="goToImport"
          />
        </AppTooltip>
        <AppTooltip :content="t('dashboard.cloneRepositoryDesc')" position="right">
          <AppNavItem
            data-tour="clone-repo"
            :icon="GitBranch"
            :label="t('dashboard.cloneRepository')"
            :description="t('dashboard.cloneRepositoryDesc')"
            @click="openCloneModal"
          />
        </AppTooltip>
        <AppTooltip :content="t('dashboard.comingSoon')" position="right">
          <AppNavItem
            :icon="Sparkles"
            :label="t('dashboard.aiTestGenerator')"
            description="Pynguin"
            disabled
            :title="t('dashboard.comingSoon')"
          />
        </AppTooltip>
      </div>

      <p class="section-label mb-1.5 mt-5">Resources</p>
      <div class="space-y-0.5">
        <AppTooltip :content="t('dashboard.comingSoon')" position="right">
          <AppNavItem :icon="BookOpen" label="Documentation" disabled :title="t('dashboard.comingSoon')" />
        </AppTooltip>
        <AppTooltip :content="t('dashboard.comingSoon')" position="right">
          <AppNavItem :icon="Bug" label="Report Issue" disabled :title="t('dashboard.comingSoon')" />
        </AppTooltip>
      </div>

      <p class="section-label mb-1.5 mt-5">Debug</p>
      <div class="space-y-0.5">
        <AppTooltip :content="t('dashboard.viewDbSchema')" position="right">
          <AppNavItem :icon="Database" label="View DB Schema" @click="router.push('/debug/schema')" />
        </AppTooltip>
      </div>
    </AppSidebar>

    <!-- 主区域 -->
    <main class="flex min-w-0 flex-1 flex-col">
      <!-- 工具栏 -->
      <header class="flex h-14 shrink-0 items-center justify-between gap-4 border-b border-border bg-white px-5">
        <div class="flex items-center gap-2.5">
          <h1 class="text-sm font-semibold text-zinc-900">{{ t("dashboard.projectsTitle") }}</h1>
          <span class="rounded-md bg-zinc-100 px-1.5 py-0.5 font-mono text-[10px] text-zinc-500">
            {{ visibleProjects.length }}
          </span>
        </div>

        <div class="flex items-center gap-2">
          <div class="relative">
            <Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400" />
            <input
              v-model="searchQuery"
              type="text"
              :placeholder="t('dashboard.searchPlaceholder')"
              class="input h-8 pl-8 pr-3"
              aria-label="Search projects"
            />
          </div>
          <AppTooltip :content="sortDesc ? t('dashboard.sortNewest') : t('dashboard.sortOldest')" position="top">
            <AppButton
              variant="secondary"
              size="sm"
              @click="sortDesc = !sortDesc"
            >
              <component :is="sortIcon" class="h-3.5 w-3.5" />
              {{ t("dashboard.sortLastOpened") }}
            </AppButton>
          </AppTooltip>
        </div>
      </header>

      <!-- 内容区 -->
      <div class="min-h-0 flex-1 overflow-auto">
        <!-- 加载中 -->
        <div v-if="projectStore.isLoading" class="flex h-full items-center justify-center">
          <Loader2 class="h-6 w-6 animate-spin text-zinc-300" />
          <span class="ml-2 text-sm text-zinc-400">{{ t("dashboard.loadingProjects") }}</span>
        </div>

        <!-- 首次启动：新手三步引导（导入 → 生成 → 运行测量） -->
        <AppEmptyState
          v-else-if="projectStore.projects.length === 0"
          :icon="Code2"
          :title="t('dashboard.welcome.title')"
          :description="t('dashboard.welcome.description')"
        >
          <ol class="mx-auto mt-2 grid w-full max-w-2xl grid-cols-3 gap-3 text-left">
            <li v-for="(step, i) in welcomeSteps" :key="i" class="card p-4">
              <div class="font-mono text-[10px] font-semibold text-brand-500">0{{ i + 1 }}</div>
              <div class="mt-1.5 text-[13px] font-semibold text-zinc-900">{{ step.title }}</div>
              <div class="mt-1 text-xs leading-5 text-zinc-500">{{ step.desc }}</div>
            </li>
          </ol>
          <div class="mt-6 flex items-center justify-center gap-2">
            <AppButton variant="primary" @click="goToImport">
              <FolderPlus class="h-4 w-4" />
              {{ t("dashboard.importProject") }}
            </AppButton>
            <AppButton variant="secondary" @click="openCloneModal">
              <GitBranch class="h-4 w-4" />
              {{ t("dashboard.cloneRepository") }}
            </AppButton>
          </div>
        </AppEmptyState>

        <!-- 搜索无结果 -->
        <AppEmptyState
          v-else-if="visibleProjects.length === 0"
          :icon="Search"
          :title="t('dashboard.emptyState.searchNoMatch')"
        />

        <!-- 项目表格 -->
        <div v-else class="px-5 py-4" data-tour="project-list">
          <div class="card overflow-hidden">
            <!-- 表头 -->
            <div
              class="grid grid-cols-[3fr_1.2fr_1.3fr_1.6fr_1fr] gap-4 border-b border-border bg-zinc-50/70 px-4 py-2 text-[10px] font-semibold uppercase tracking-wider text-zinc-400"
            >
              <div>{{ t("dashboard.projectTable.project") }}</div>
              <div>{{ t("dashboard.projectTable.environment") }}</div>
              <div>{{ t("dashboard.projectTable.testsPassed") }}</div>
              <div>{{ t("dashboard.projectTable.coverage") }}</div>
              <div class="text-right">{{ t("dashboard.projectTable.lastRun") }}</div>
            </div>

            <!-- 行 -->
            <div class="divide-y divide-border">
              <div
                v-for="project in visibleProjects"
                :key="project.id"
                @click="goToProject(project.id)"
                @contextmenu.prevent="showProjectMenu(project, $event)"
                class="group grid cursor-pointer grid-cols-[3fr_1.2fr_1.3fr_1.6fr_1fr] items-center gap-4 px-4 py-3 transition-colors hover:bg-zinc-50"
              >
                <!-- 项目信息 + 右键菜单 -->
                <div class="flex min-w-0 items-center gap-3">
                  <div
                    class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-zinc-100 text-zinc-500"
                  >
                    <Code2 class="h-4 w-4" />
                  </div>
                  <div class="min-w-0">
                    <div class="flex items-center gap-1.5">
                      <span class="truncate text-[13px] font-medium text-zinc-800">{{ project.name }}</span>
                      <!-- 删除按钮保留作为快速入口，右键菜单提供完整操作 -->
                      <button
                        type="button"
                        class="shrink-0 rounded p-1 text-zinc-300 opacity-0 transition-all hover:bg-rose-50 hover:text-rose-500 focus-visible:opacity-100 group-hover:opacity-100"
                        :aria-label="t('dashboard.deleteProject')"
                        @click.stop="handleDelete(project)"
                      >
                        <Trash2 class="h-3.5 w-3.5" />
                      </button>
                    </div>
                    <p class="truncate font-mono text-[10px] text-zinc-400">{{ project.path }}</p>
                  </div>
                </div>

                <!-- 环境状态 -->
                <div>
                  <StatusPill :status="project.env_status" />
                </div>

                <!-- 测试结果 -->
                <div class="flex items-center gap-1.5 font-mono text-[11px]">
                  <span class="text-emerald-600">{{
                    t("dashboard.passedCount", { count: project.tests_passed })
                  }}</span>
                  <span class="text-zinc-300">·</span>
                  <span class="text-rose-600">{{
                    t("dashboard.failedCount", { count: project.tests_failed })
                  }}</span>
                </div>

                <!-- 覆盖率 -->
                <div class="flex items-center gap-2.5">
                  <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-zinc-100">
                    <div
                      class="h-full rounded-full transition-all"
                      :class="coverageTone(project.coverage)"
                      :style="{ width: `${Math.min(100, Math.max(0, project.coverage))}%` }"
                    />
                  </div>
                  <span class="w-9 text-right font-mono text-[11px] text-zinc-600"
                    >{{ project.coverage.toFixed(2) }}%</span
                  >
                </div>

                <!-- 上次运行 -->
                <div class="text-right font-mono text-[11px] text-zinc-400">
                  {{ project.last_run ?? t("common.never") }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 底部状态栏 -->
      <footer
        class="flex h-8 shrink-0 items-center justify-between border-t border-border bg-white px-4 text-[10px] text-zinc-500"
      >
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-1.5 font-mono">
            <Terminal class="h-3 w-3" />
            <span>{{
              pythonVersion
                ? `${pythonVersion}${pythonVenv ? " (venv)" : " (global)"}`
                : t("app.statusBar.pythonNotDetected")
            }}</span>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
            <span>{{ t("app.statusBar.ipcConnected") }}</span>
          </div>
        </div>
        <div class="flex items-center gap-4 font-mono">
          <span>{{ t("app.statusBar.projects", { count: projectStore.stats.total_projects }) }}</span>
          <span>{{ t("app.statusBar.totalRuns", { count: projectStore.stats.total_runs }) }}</span>
        </div>
      </footer>
    </main>

    <!-- 克隆仓库弹窗（统一 AppModal + 统一主色按钮） -->
    <AppModal
      v-model:open="showCloneModal"
      :title="t('dashboard.cloneModal.title')"
      :description="t('dashboard.cloneModal.description')"
    >
      <div class="space-y-4">
        <div>
          <label class="label" for="clone-url">{{ t("dashboard.cloneModal.repoUrl") }}</label>
          <input
            id="clone-url"
            v-model="cloneUrl"
            type="text"
            class="input mt-1.5"
            :placeholder="t('dashboard.cloneModal.repoUrlPlaceholder')"
            @keyup.enter="cloneRepository"
          />
        </div>

        <div>
          <span class="label">{{ t("dashboard.cloneModal.destFolder") }}</span>
          <button
            type="button"
            class="input mt-1.5 flex items-center justify-between text-left transition-colors hover:bg-zinc-50"
            @click="chooseCloneDir"
          >
            <span class="truncate" :class="cloneTargetDir ? 'text-zinc-800' : 'text-zinc-400'">
              {{ cloneTargetDir || t("dashboard.cloneModal.chooseFolder") }}
            </span>
            <FolderPlus class="h-4 w-4 shrink-0 text-zinc-400" />
          </button>
        </div>

        <div
          v-if="cloneError"
          class="rounded-lg border border-rose-200 bg-rose-50 px-3.5 py-2.5 text-xs text-rose-700"
        >
          {{ cloneError }}
        </div>
      </div>

      <template #footer>
        <AppButton variant="ghost" @click="showCloneModal = false">{{ t("common.cancel") }}</AppButton>
        <AppButton
          variant="primary"
          :loading="isCloning"
          :disabled="!cloneUrl.trim() || !cloneTargetDir"
          @click="cloneRepository"
        >
          <GitBranch class="h-4 w-4" />
          {{ isCloning ? t("dashboard.cloneModal.cloning") : t("dashboard.cloneModal.cloneImport") }}
        </AppButton>
      </template>
    </AppModal>

    <!-- 删除项目确认（统一弹窗，替代原生 ask） -->
    <AppConfirmModal
      :open="deleteTarget !== null"
      :title="t('common.delete')"
      :description="deleteTarget ? t('dashboard.deleteConfirm', { name: deleteTarget.name }) : ''"
      :confirm-label="t('common.delete')"
      @confirm="confirmDeleteProject"
      @update:open="(open: boolean) => { if (!open) deleteTarget = null }"
    />

    <!-- 项目右键菜单 -->
    <AppContextMenu
      v-if="projectMenu"
      :items="projectMenuItems"
      :open="!!projectMenu"
      @close="projectMenu = null"
    />

    <!-- 新手引导 Tour -->
    <AppTour
      v-if="showTour"
      :steps="tourSteps"
      @complete="onTourComplete"
      @skip="onTourComplete"
    />
  </div>
</template>
