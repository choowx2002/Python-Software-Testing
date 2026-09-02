<script setup lang="ts">
/**
 * 项目上下文 Layout —— 第三批
 *  - 侧边栏：项目卡（含迷你覆盖率条）+ 四个功能区导航（Overview → Generate → Execute → Coverage）
 *  - 右侧概览面板已抽离为独立 Overview 页，主区域全宽给子页面
 *  - 环境状态接入 StatusPill；返回按钮走 AppSidebar footer 插槽
 *  - 键盘快捷键：Ctrl+1~5 切换标签页，Ctrl+B 切换侧边栏
 *  - 窗口标题动态更新
 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ArrowLeft,
  FileText,
  FolderOpen,
  Gauge,
  History,
  LayoutDashboard,
  Play,
  RefreshCw,
  WandSparkles,
} from "@lucide/vue";
import { useProjectStore, type Project } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import AppSidebar from "../../components/AppSidebar.vue";
import AppNavItem from "../../components/ui/AppNavItem.vue";
import AppButton from "../../components/ui/AppButton.vue";
import RunActivityBadge from "../../components/ui/RunActivityBadge.vue";
import StatusPill from "../../components/ui/StatusPill.vue";
import AppTooltip from "../../components/ui/AppTooltip.vue";
import { useWindowTitle } from "../../composables/useWindowTitle";
import { usePageShortcuts } from "../../composables/useKeyboardShortcuts";

type ProjectRouteName =
  | "ProjectOverview"
  | "ProjectExecute"
  | "ProjectGenerate"
  | "ProjectCoverage"
  | "ProjectHistory";

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();
const uiStore = useUIStore();
const { t } = useI18n();

// Window title management
const { setStatus, reset: resetWindowTitle } = useWindowTitle()

const isRefreshing = ref(false);
const localError = ref<string | null>(null);

const projectId = computed(() => Number(route.params.id));

const currentProject = computed<Project | undefined>(() => {
  if (Number.isNaN(projectId.value)) return undefined;
  return projectStore.projects.find((p) => p.id === projectId.value);
});

/* ---------------- 功能区导航（Overview 为项目首页，随后按工作流顺序） ---------------- */
const navItems = computed(() => [
  {
    name: "ProjectOverview" as const,
    label: t("layout.tabs.overview"),
    desc: t("layout.tabsDesc.overview"),
    icon: LayoutDashboard,
    tour: "overview-tab",
    shortcut: "1",
  },
  {
    name: "ProjectGenerate" as const,
    label: t("layout.tabs.generate"),
    desc: t("layout.tabsDesc.generate"),
    icon: WandSparkles,
    tour: "generate-tab",
    shortcut: "2",
  },
  {
    name: "ProjectExecute" as const,
    label: t("layout.tabs.execute"),
    desc: t("layout.tabsDesc.execute"),
    icon: Play,
    tour: "execute-tab",
    shortcut: "3",
  },
  {
    name: "ProjectCoverage" as const,
    label: t("layout.tabs.coverage"),
    desc: t("layout.tabsDesc.coverage"),
    icon: Gauge,
    tour: "coverage-tab",
    shortcut: "4",
  },
  {
    name: "ProjectHistory" as const,
    label: t("layout.tabs.history"),
    desc: t("layout.tabsDesc.history"),
    icon: History,
    tour: "history-tab",
    shortcut: "5",
  },
]);

/* ---------------- 环境状态（文案映射，视觉交给 StatusPill） ---------------- */
const statusLabel = computed(() => {
  const status = currentProject.value?.env_status ?? "Warning";
  switch (status) {
    case "Ready":
      return t("layout.statusReady");
    case "Warning":
      return t("layout.statusWarning");
    case "Failed":
      return t("layout.statusFailed");
    case "active":
      return t("layout.statusActive");
    default:
      return status;
  }
});

/* 侧栏迷你覆盖率条 */
const safeCoverage = computed(() => {
  const value = currentProject.value?.coverage;
  if (typeof value !== "number") return 0;
  return Math.max(0, Math.min(100, value));
});

/* ---------------- 动作 ---------------- */
function goBack() {
  router.push({ name: "Dashboard" });
}

function goToSection(name: ProjectRouteName) {
  if (Number.isNaN(projectId.value)) return;
  router.push({ name, params: { id: projectId.value } });
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
    localError.value = t("layout.invalidId");
    return;
  }

  isRefreshing.value = true;
  localError.value = null;

  try {
    await projectStore.fetchProjects();
    await projectStore.updateLastOpened(projectId.value);
    // 顺带校验/恢复解释器路径，并通知当前子页重新做环境检测
    await projectStore.validateInterpreters();
    window.dispatchEvent(new CustomEvent("testmate:env-check"));
  } catch (error) {
    console.error("[ProjectLayout] refreshProject failed:", error);
    localError.value = t("layout.refreshFailed");
  } finally {
    isRefreshing.value = false;
  }
}

async function ensureProjectLoaded() {
  if (Number.isNaN(projectId.value)) {
    localError.value = t("layout.invalidId");
    return;
  }

  try {
    localError.value = null;

    if (projectStore.projects.length === 0) {
      await projectStore.fetchProjects();
    }

    const found = projectStore.projects.find((p) => p.id === projectId.value);
    if (!found) {
      localError.value = t("layout.projectNotFound", { id: projectId.value });
      return;
    }

    await projectStore.updateLastOpened(projectId.value);
  } catch (error) {
    console.error("[ProjectLayout] ensureProjectLoaded failed:", error);
    localError.value = t("layout.loadFailed");
  }
}

// Keyboard shortcuts for tab switching (Ctrl+1~5)
usePageShortcuts({
  'ctrl+1': () => goToSection('ProjectOverview'),
  'ctrl+2': () => goToSection('ProjectGenerate'),
  'ctrl+3': () => goToSection('ProjectExecute'),
  'ctrl+4': () => goToSection('ProjectCoverage'),
  'ctrl+5': () => goToSection('ProjectHistory'),
}, () => !isRefreshing.value)

// Update window title when project or route changes
watch(
  () => {
    const projectName = currentProject.value?.name
    const routeNameStr = String(route.name)
    return [projectName, routeNameStr] as const
  },
  ([_name, _routeName]) => {
    const name = _name
    const routeName = _routeName
    if (name) {
      const tabLabels: Record<string, string> = {
        ProjectOverview: t('layout.tabs.overview'),
        ProjectGenerate: t('layout.tabs.generate'),
        ProjectExecute: t('layout.tabs.execute'),
        ProjectCoverage: t('layout.tabs.coverage'),
        ProjectHistory: t('layout.tabs.history'),
      }
      const tabLabel = tabLabels[routeName] ?? ''
      setStatus(`${name} — ${tabLabel}`)
    } else {
      resetWindowTitle()
    }
  },
  { immediate: true }
)

watch(
  () => route.params.id,
  () => {
    ensureProjectLoaded();
  },
  { immediate: true },
);

// 记住每个项目上次停留的标签页
watch(
  () => route.name,
  (name) => {
    if (Number.isNaN(projectId.value)) return;
    const tab = String(name);
    const tabs = new Set(['ProjectOverview', 'ProjectGenerate', 'ProjectExecute', 'ProjectCoverage', 'ProjectHistory']);
    if (tabs.has(tab)) {
      uiStore.setLastTab(projectId.value, tab);
    }
  },
  { immediate: true },
);

function onRefreshEvent() {
  void refreshProject();
}

onMounted(() => {
  ensureProjectLoaded();
  window.addEventListener('testmate:refresh', onRefreshEvent);
});

onUnmounted(() => {
  window.removeEventListener('testmate:refresh', onRefreshEvent);
});
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-surface text-zinc-900">
    <!-- 统一侧边栏 -->
    <AppSidebar>
      <!-- 当前项目卡 -->
      <div class="mb-4 rounded-xl border border-border bg-zinc-50 p-3.5">
        <div class="text-[10px] font-semibold uppercase tracking-wider text-zinc-400">
          {{ t("layout.currentProject") }}
        </div>
        <div class="mt-1.5 truncate text-sm font-semibold text-zinc-900">
          {{ currentProject?.name ?? t("layout.notLoaded") }}
        </div>
        <div class="mt-0.5 break-all font-mono text-[10px] leading-4 text-zinc-400">
          {{ currentProject?.path ?? t("layout.loadingPath") }}
        </div>
        <div class="mt-2.5 flex items-center gap-2">
          <StatusPill :status="currentProject?.env_status ?? 'Warning'" :label="statusLabel" />
          <span class="font-mono text-[10px] text-zinc-400">ID {{ projectId }}</span>
        </div>

        <!-- 迷你覆盖率条（替代原右侧面板的"一瞥可见"） -->
        <div class="mt-2.5 h-1 overflow-hidden rounded-full bg-zinc-200">
          <div
            class="h-full rounded-full transition-all"
            :class="safeCoverage >= 80 ? 'bg-emerald-500' : 'bg-amber-500'"
            :style="{ width: `${safeCoverage}%` }"
          />
        </div>
        <div class="mt-1 flex items-center justify-between font-mono text-[10px] text-zinc-400">
          <span>{{ t("layout.overviewItems.coverage") }}</span>
          <span>{{ safeCoverage.toFixed(1) }}%</span>
        </div>
      </div>

      <!-- 功能区导航 -->
      <p class="section-label mb-1.5">{{ t("layout.pageNav") }}</p>
      <div class="space-y-0.5">
        <AppTooltip
          v-for="item in navItems"
          :key="item.name"
          :content="`${item.label} (Ctrl+${item.shortcut})`"
          position="right"
        >
          <AppNavItem
            :data-tour="item.tour"
            :icon="item.icon"
            :label="item.label"
            :description="item.desc"
            :active="route.name === item.name"
            @click="goToSection(item.name)"
          />
        </AppTooltip>
      </div>

      <!-- 返回按钮 -->
      <template #footer>
        <AppButton variant="secondary" class="w-full" @click="goBack">
          <ArrowLeft class="h-4 w-4" />
          {{ t("layout.backToDashboard") }}
        </AppButton>
      </template>
    </AppSidebar>

    <!-- 主区域 -->
    <main class="flex min-w-0 flex-1 flex-col">
      <!-- 顶部工具栏 -->
      <header class="flex h-14 shrink-0 items-center justify-between gap-4 border-b border-border bg-white px-5">
        <div class="flex min-w-0 items-center gap-3">
          <!-- 面包屑：Projects / 项目名 -->
          <div class="flex min-w-0 items-center gap-1.5 text-[13px]">
            <AppTooltip :content="t('layout.backToDashboard')" position="top">
              <button
                type="button"
                class="shrink-0 rounded px-1.5 py-0.5 text-zinc-500 transition-colors hover:bg-zinc-100 hover:text-zinc-700"
                @click="goBack"
              >
                {{ t("layout.breadcrumbProjects") }}
              </button>
            </AppTooltip>
            <span class="text-zinc-300">/</span>
            <h1 class="truncate text-sm font-semibold text-zinc-900">
              {{ currentProject?.name ?? t("layout.context") }}
            </h1>
          </div>

          <!-- 解释器路径徽章 -->
          <span
            class="hidden max-w-64 items-center gap-1.5 truncate rounded-full border border-border px-2 py-0.5 font-mono text-[10px] text-zinc-500 md:inline-flex"
            :title="currentProject?.interpreter_path ?? undefined"
          >
            <FileText class="h-3 w-3 shrink-0" />
            <span class="truncate">{{ currentProject?.interpreter_path ?? t("layout.noInterpreter") }}</span>
          </span>
        </div>

        <div class="flex shrink-0 items-center gap-2">
          <RunActivityBadge />
          <AppTooltip :content="t('layout.openFolder')" position="top">
            <AppButton variant="secondary" @click="openProjectFolder">
              <FolderOpen class="h-4 w-4" />
              {{ t("layout.openFolder") }}
            </AppButton>
          </AppTooltip>
          <AppTooltip :content="t('common.refresh')" position="top">
            <AppButton variant="primary" :loading="isRefreshing" @click="refreshProject">
              <RefreshCw v-if="!isRefreshing" class="h-4 w-4" />
              {{ t("common.refresh") }}
            </AppButton>
          </AppTooltip>
        </div>
      </header>

      <!-- 错误横幅 -->
      <div v-if="localError" class="px-5 pt-4">
        <div class="rounded-lg border border-rose-200 bg-rose-50 px-3.5 py-2.5 text-xs text-rose-700">
          {{ localError }}
        </div>
      </div>

      <!-- 内容区：子页面全宽 -->
      <section class="min-h-0 flex-1 overflow-hidden p-4">
        <div class="card h-full min-h-0 overflow-hidden">
          <div class="h-full min-h-0 overflow-auto">
            <RouterView v-slot="{ Component }">
              <Transition name="fade-slide" mode="out-in">
                <component :is="Component" />
              </Transition>
            </RouterView>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<style scoped>
/* 子页面切换过渡 */
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

@media (prefers-reduced-motion: reduce) {
  .fade-slide-enter-active,
  .fade-slide-leave-active {
    transition: none;
  }
}
</style>
