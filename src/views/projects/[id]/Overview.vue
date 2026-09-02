<script setup lang="ts">
/**
 * 项目概览页（项目首页）
 *  - 环境状态：解释器 / Python 版本 / venv / 依赖缺失提示
 *  - 数据卡片：覆盖率、最近测试结果、环境
 *  - 新手三步工作流引导：生成 → 执行 → 覆盖率（带跳转 CTA）
 */
import { computed, ref, watch, onMounted, onBeforeUnmount } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import { Download, Gauge, Loader2, Play, WandSparkles, Wrench } from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import AppCard from "../../../components/ui/AppCard.vue";
import AppButton from "../../../components/ui/AppButton.vue";
import AppConfirmModal from "../../../components/ui/AppConfirmModal.vue";
import StatusPill from "../../../components/ui/StatusPill.vue";

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();
const { t } = useI18n();

const projectId = computed(() => Number(route.params.id));

const currentProject = computed(() =>
  projectStore.projects.find((p) => p.id === projectId.value),
);

/* ------------------------------------------------------------------ */
/* 环境探测（detect_python_env，与 Dashboard 同一命令）                 */
/* ------------------------------------------------------------------ */
interface EnvResult {
  pythonPath: string | null;
  pythonVersion: string | null;
  venvPath: string | null;
  venvExists: boolean;
  dependencies: { name: string; installed: boolean; version: string | null }[];
}

const env = ref<EnvResult | null>(null);
const envLoading = ref(false);
const envError = ref(false);

async function detectEnv() {
  const project = currentProject.value;
  if (!project?.path) return;

  envLoading.value = true;
  envError.value = false;
  try {
    env.value = await invoke<EnvResult>("detect_python_env", { projectPath: project.path });
    await mergeMissingImports();
  } catch (error) {
    console.error("[Overview] detect_python_env failed:", error);
    envError.value = true;
  } finally {
    envLoading.value = false;
  }
}

/** 静态 import 扫描（requirements.txt 之外的第三方依赖，如 numpy） */
async function mergeMissingImports() {
  const project = currentProject.value;
  const current = env.value;
  if (!project?.path || !current?.pythonPath) return;
  try {
    const missing = await invoke<string[]>("scan_missing_imports", {
      projectPath: project.path,
      interpreterPath: current.pythonPath,
    });
    for (const name of missing) {
      if (!current.dependencies.some((d) => d.name === name)) {
        current.dependencies.push({ name, installed: false, version: null });
      }
    }
  } catch (error) {
    console.error("[Overview] scan_missing_imports failed:", error);
  }
}

// 项目路径就绪（含首次进入时 store 尚未加载完成的情况）即触发探测
watch(
  () => currentProject.value?.path,
  () => {
    void detectEnv();
    void loadHistoryTrends();
  },
  { immediate: true },
);

const missingDeps = computed(() =>
  (env.value?.dependencies ?? []).filter((d) => !d.installed),
);

/** 当前 Python 是否为 3.11（Pynguin 0.43+ 兼容版本） */
const isPython311 = computed(() =>
  (env.value?.pythonVersion ?? "").startsWith("Python 3.11"),
);

/** venv 已存在但建在非 3.11 上 → 建议重建 */
const venvNeedsRebuild = computed(() =>
  Boolean(
    env.value?.venvExists &&
      env.value?.pythonVersion &&
      !isPython311.value,
  ),
);

/** idle(检测中/无数据) / Ready / Warning / Failed */
const envStatus = computed<"idle" | "Ready" | "Warning" | "Failed">(() => {
  if (envError.value) return "Failed";
  if (!env.value) return "idle";
  if (!env.value.venvExists) return "Warning";
  return missingDeps.value.length === 0 ? "Ready" : "Warning";
});

const envStatusLabel = computed(() => {
  if (envLoading.value) return t("overview.checking");
  switch (envStatus.value) {
    case "Ready":
      return t("layout.statusReady");
    case "Warning":
      return t("layout.statusWarning");
    case "Failed":
      return t("layout.statusFailed");
    default:
      return t("overview.noEnv");
  }
});

/* ------------------------------------------------------------------ */
/* 环境修复：无 venv 时用检测到的解释器重建，再按需装依赖                 */
/* ------------------------------------------------------------------ */
const envFixing = ref(false);
const envFixError = ref<string | null>(null);
const envFixNote = ref<string | null>(null);
const showRebuildConfirm = ref(false);
let unlistenInstall: UnlistenFn | undefined;
let unlistenEnvFix: UnlistenFn | undefined;

/** 订阅 pip 安装进度（install_step 事件），仅用于沉浸式提示 */
async function subscribeInstallStep() {
  unlistenInstall?.();
  unlistenInstall = await listen<{ package: string; status: string }>(
    "install_step",
    (event) => {
      const { package: pkg, status } = event.payload;
      if (status === "starting") {
        envFixNote.value = t("overview.installing", { package: pkg });
      } else if (status === "success") {
        envFixNote.value = null;
      }
    },
  );
}

/** 一键修复（fix_python_env）的阶段性提示：winget/venv/deps/db/done */
async function subscribeEnvFixStep() {
  unlistenEnvFix?.();
  unlistenEnvFix = await listen<{ stage: string; status: string }>(
    "env-fix-step",
    (event) => {
      const { stage, status } = event.payload;
      if (status === "success") {
        envFixNote.value = null;
        return;
      }
      if (stage === "winget") envFixNote.value = t("overview.envStageWinget");
      else if (stage === "venv") envFixNote.value = t("overview.envStageVenv");
      else if (stage === "deps") envFixNote.value = t("overview.envStageDeps");
      else if (stage === "db") envFixNote.value = t("overview.envStageDb");
    },
  );
}

onBeforeUnmount(() => {
  unlistenInstall?.();
  unlistenEnvFix?.();
  window.removeEventListener('testmate:env-check', onEnvCheck);
});

/** Header 刷新（testmate:env-check）→ 重做环境检测 */
function onEnvCheck() {
  void detectEnv();
}

onMounted(() => {
  window.addEventListener('testmate:env-check', onEnvCheck);
});

/** 确保 Python 3.11 并用它（重新）创建 .venv，随后补装剩余缺失依赖并重探测 */
async function runEnvFix() {
  const project = currentProject.value;
  if (!project?.path || envFixing.value) return;

  envFixing.value = true;
  envFixError.value = null;
  envFixNote.value = null;
  await Promise.all([subscribeInstallStep(), subscribeEnvFixStep()]);
  try {
    await invoke("fix_python_env", {
      projectPath: project.path,
      projectId: project.id,
    });
    await detectEnv();
    if (missingDeps.value.length) {
      await invoke("install_dependencies", {
        pythonPath: env.value?.pythonPath,
        packages: missingDeps.value.map((d) => d.name),
      });
      await detectEnv();
    }
  } catch (error) {
    console.error("[Overview] fix_python_env failed:", error);
    envFixError.value = error instanceof Error ? error.message : String(error);
  } finally {
    envFixing.value = false;
    envFixNote.value = null;
    unlistenInstall?.();
    unlistenInstall = undefined;
    unlistenEnvFix?.();
    unlistenEnvFix = undefined;
  }
}

/** 无 venv：创建（优先用 Python 3.11） */
async function createEnv() {
  await runEnvFix();
}

/** venv 存在但非 3.11：确认后重建 */
async function rebuildVenv311() {
  showRebuildConfirm.value = false;
  await runEnvFix();
}

/** 仅补装缺失依赖（venv 已存在时） */
async function installDeps() {
  const py = env.value?.pythonPath;
  if (!py || !missingDeps.value.length || envFixing.value) return;

  envFixing.value = true;
  envFixError.value = null;
  envFixNote.value = null;
  await subscribeInstallStep();
  try {
    await invoke("install_dependencies", {
      pythonPath: py,
      packages: missingDeps.value.map((d) => d.name),
    });
    await detectEnv();
  } catch (error) {
    console.error("[Overview] install_dependencies failed:", error);
    envFixError.value = error instanceof Error ? error.message : String(error);
  } finally {
    envFixing.value = false;
    envFixNote.value = null;
    unlistenInstall?.();
    unlistenInstall = undefined;
  }
}

/* ------------------------------------------------------------------ */
/* 数据计算                                                            */
/* ------------------------------------------------------------------ */
const safeCoverage = computed(() => {
  const value = currentProject.value?.coverage;
  if (typeof value !== "number") return 0;
  return Math.max(0, Math.min(100, value));
});

/* ------------------------------------------------------------------ */
/* 历史对比（较上次运行：覆盖率 / 通过率 差值）                          */
/* ------------------------------------------------------------------ */

interface CoverageHistoryItem {
  id: number;
  percentCovered: number;
}

interface ExecutionHistoryItem {
  id: number;
  passed: number;
  failed: number;
  skipped: number;
}

const coverageHistory = ref<CoverageHistoryItem[]>([]);
const executionHistory = ref<ExecutionHistoryItem[]>([]);

async function loadHistoryTrends() {
  if (Number.isNaN(projectId.value)) return;

  try {
    const [cov, ex] = await Promise.all([
      invoke<CoverageHistoryItem[]>("list_coverage_history", {
        projectId: projectId.value,
      }),
      invoke<ExecutionHistoryItem[]>("list_execution_history", {
        projectId: projectId.value,
      }),
    ]);
    coverageHistory.value = (cov ?? []).map((c) => ({
      id: c.id,
      percentCovered: Number(c.percentCovered) || 0,
    }));
    executionHistory.value = (ex ?? []).map((e) => ({
      id: e.id,
      passed: Number(e.passed) || 0,
      failed: Number(e.failed) || 0,
      skipped: Number(e.skipped) || 0,
    }));
  } catch (error) {
    console.error("[Overview] Failed to load history trends:", error);
  }
}

/** 覆盖率差值（最新 - 上一次） */
const coverageDelta = computed<number | null>(() => {
  if (coverageHistory.value.length < 2) return null;
  return (
    coverageHistory.value[0].percentCovered -
    coverageHistory.value[1].percentCovered
  );
});

/** 通过率差值（最新执行 - 上一次执行） */
const passRateDelta = computed<number | null>(() => {
  const list = executionHistory.value;
  if (list.length < 2) return null;

  const rate = (item: ExecutionHistoryItem) => {
    const total = item.passed + item.failed + item.skipped;
    return total > 0 ? (item.passed / total) * 100 : null;
  };
  const cur = rate(list[0]);
  const prev = rate(list[1]);
  if (cur === null || prev === null) return null;
  return cur - prev;
});

/** 差值展示元信息（≈0 时中性显示） */
function deltaMeta(delta: number) {
  if (Math.abs(delta) < 0.05) {
    return { arrow: "—", cls: "text-zinc-400", text: "0.0" };
  }
  return {
    arrow: delta > 0 ? "▲" : "▼",
    cls: delta > 0 ? "text-emerald-600" : "text-rose-600",
    text: Math.abs(delta).toFixed(1),
  };
}

const coverageDeltaMeta = computed(() =>
  coverageDelta.value === null ? null : deltaMeta(coverageDelta.value),
);
const passRateDeltaMeta = computed(() =>
  passRateDelta.value === null ? null : deltaMeta(passRateDelta.value),
);

/** 上一次的数值（用于悬停提示） */
const prevCoverage = computed(() =>
  coverageHistory.value.length >= 2
    ? coverageHistory.value[1].percentCovered
    : null,
);
const prevPassRate = computed(() => {
  const list = executionHistory.value;
  if (list.length < 2) return null;
  const total = list[1].passed + list[1].failed + list[1].skipped;
  return total > 0 ? (list[1].passed / total) * 100 : null;
});

/* ------------------------------------------------------------------ */
/* 新手三步工作流（真实顺序：生成 → 执行 → 覆盖率）                     */
/* ------------------------------------------------------------------ */
const steps = computed(() => [
  {
    num: "01",
    icon: WandSparkles,
    title: t("overview.step1Title"),
    desc: t("overview.step1Desc"),
    cta: t("overview.gotoGenerate"),
    name: "ProjectGenerate" as const,
  },
  {
    num: "02",
    icon: Play,
    title: t("overview.step2Title"),
    desc: t("overview.step2Desc"),
    cta: t("overview.gotoExecute"),
    name: "ProjectExecute" as const,
  },
  {
    num: "03",
    icon: Gauge,
    title: t("overview.step3Title"),
    desc: t("overview.step3Desc"),
    cta: t("overview.gotoCoverage"),
    name: "ProjectCoverage" as const,
  },
]);

function goTo(name: "ProjectGenerate" | "ProjectExecute" | "ProjectCoverage") {
  if (Number.isNaN(projectId.value)) return;
  router.push({ name, params: { id: projectId.value } });
}
</script>

<template>
  <div class="flex h-full flex-col gap-4 overflow-auto p-5">
    <!-- 页头 -->
    <div class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <h1 class="text-lg font-semibold text-zinc-900">{{ t("overview.title") }}</h1>
        <p class="mt-0.5 text-xs text-zinc-500">
          {{ t("overview.subtitle", { name: currentProject?.name ?? t("layout.notLoaded") }) }}
        </p>
      </div>
      <StatusPill :status="envStatus" :label="envStatusLabel" :pulse="envLoading" />
    </div>

    <!-- 数据卡片 -->
    <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
      <AppCard :title="t('overview.coverageTitle')">
        <div class="text-3xl font-semibold text-zinc-900">{{ safeCoverage.toFixed(1) }}%</div>

        <!-- 较上次覆盖率运行的差值 -->
        <div
          v-if="coverageDeltaMeta"
          class="mt-1.5 flex items-center gap-1.5"
          :title="
            prevCoverage !== null
              ? t('overview.prevValue', { value: `${prevCoverage.toFixed(1)}%` })
              : undefined
          "
        >
          <span
            class="font-mono text-[11px] font-semibold"
            :class="coverageDeltaMeta.cls"
          >
            {{ coverageDeltaMeta.arrow }} {{ coverageDeltaMeta.text }}%
          </span>
          <span class="text-[10px] text-zinc-400">{{ t("overview.vsPrev") }}</span>
        </div>

        <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-100">
          <div
            class="h-full rounded-full bg-emerald-500 transition-all"
            :style="{ width: `${safeCoverage}%` }"
          />
        </div>
      </AppCard>

      <AppCard :title="t('overview.lastRunTitle')">
        <div class="flex items-center gap-6">
          <div>
            <div class="text-[10px] text-zinc-500">{{ t("layout.pass") }}</div>
            <div class="mt-0.5 text-xl font-semibold text-emerald-600">
              {{ currentProject?.tests_passed ?? 0 }}
            </div>
          </div>
          <div>
            <div class="text-[10px] text-zinc-500">{{ t("layout.fail") }}</div>
            <div class="mt-0.5 text-xl font-semibold text-rose-600">
              {{ currentProject?.tests_failed ?? 0 }}
            </div>
          </div>
        </div>

        <!-- 较上次执行的通过率差值 -->
        <div
          v-if="passRateDeltaMeta"
          class="mt-2 flex items-center gap-1.5"
          :title="
            prevPassRate !== null
              ? t('overview.prevValue', { value: `${prevPassRate.toFixed(1)}%` })
              : undefined
          "
        >
          <span class="text-[10px] text-zinc-500">{{ t("overview.passRate") }}</span>
          <span class="font-mono text-[11px] font-semibold" :class="passRateDeltaMeta.cls">
            {{ passRateDeltaMeta.arrow }} {{ passRateDeltaMeta.text }}%
          </span>
          <span class="text-[10px] text-zinc-400">{{ t("overview.vsPrev") }}</span>
        </div>

        <div class="mt-3 border-t border-border pt-3 font-mono text-[11px] text-zinc-500">
          {{ currentProject?.last_run ?? t("layout.overviewItems.never") }}
        </div>
      </AppCard>

      <AppCard :title="t('overview.environmentTitle')">
        <div v-if="envLoading" class="flex items-center gap-2 text-xs text-zinc-400">
          <Loader2 class="h-3.5 w-3.5 animate-spin" />
          {{ t("overview.checking") }}
        </div>

        <template v-else-if="env">
          <div class="space-y-2.5 text-xs">
            <div class="flex items-center justify-between gap-2">
              <span class="text-zinc-500">{{ t("overview.python") }}</span>
              <span
                class="font-mono"
                :class="isPython311 ? 'text-emerald-600' : 'text-amber-600'"
              >
                {{ env.pythonVersion ?? "—" }}
              </span>
            </div>
            <div class="flex items-center justify-between gap-2">
              <span class="text-zinc-500">{{ t("overview.venv") }}</span>
              <span class="font-mono" :class="env.venvExists ? 'text-emerald-600' : 'text-amber-600'">
                {{ env.venvExists ? t("overview.venvActive") : t("overview.venvMissing") }}
              </span>
            </div>
            <div class="flex items-center justify-between gap-2">
              <span class="text-zinc-500">{{ t("overview.dependencies") }}</span>
              <span
                class="font-mono"
                :class="missingDeps.length ? 'text-amber-600' : 'text-emerald-600'"
              >
                {{
                  missingDeps.length
                    ? t("overview.depsMissing", { count: missingDeps.length })
                    : t("overview.depsOk")
                }}
              </span>
            </div>
            <p v-if="venvNeedsRebuild" class="text-[11px] leading-4 text-amber-600">
              {{ t("overview.venvNot311Hint") }}
            </p>
          </div>

          <!-- 环境修复（无 venv 创建 / 非 3.11 重建 / 缺依赖补装） -->
          <div v-if="envFixing" class="mt-3 space-y-1">
            <div class="flex items-center gap-2 text-xs font-medium text-brand-600">
              <Loader2 class="h-3.5 w-3.5 animate-spin" />
              {{ t("overview.fixingEnv") }}
            </div>
            <div v-if="envFixNote" class="pl-5 text-xs text-zinc-500">{{ envFixNote }}</div>
          </div>

          <div v-else-if="envFixError" class="mt-3 space-y-1.5">
            <div class="rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-700">
              {{ envFixError }}
            </div>
            <AppButton
              variant="secondary"
              size="sm"
              @click="env.venvExists && !venvNeedsRebuild ? installDeps() : runEnvFix()"
            >
              {{ t("common.retry") }}
            </AppButton>
          </div>

          <div v-else class="mt-3 flex flex-wrap items-center gap-2">
            <AppButton v-if="!env.venvExists" variant="primary" size="sm" @click="createEnv">
              <Wrench class="h-3.5 w-3.5" />
              {{ t("overview.createEnv") }}
            </AppButton>
            <AppButton
              v-if="env.venvExists && missingDeps.length"
              variant="secondary"
              size="sm"
              @click="installDeps"
            >
              <Download class="h-3.5 w-3.5" />
              {{ t("overview.installDeps") }}
            </AppButton>
            <AppButton
              v-if="venvNeedsRebuild"
              variant="secondary"
              size="sm"
              @click="showRebuildConfirm = true"
            >
              <Wrench class="h-3.5 w-3.5" />
              {{ t("overview.rebuildVenv311") }}
            </AppButton>
          </div>
        </template>

        <div v-else class="text-xs text-zinc-400">{{ t("overview.noEnv") }}</div>
      </AppCard>
    </div>

    <!-- 依赖缺失提示（教学引导：先装依赖再跑测试） -->
    <div
      v-if="missingDeps.length"
      class="rounded-lg border border-amber-200 bg-amber-50 px-3.5 py-2.5 text-xs text-amber-700"
    >
      {{ t("overview.missingHint", { list: missingDeps.map((d) => d.name).join(", ") }) }}
    </div>

    <!-- 新手三步工作流 -->
    <AppCard :title="t('overview.workflowTitle')" :subtitle="t('overview.workflowDesc')">
      <div class="grid grid-cols-1 gap-3 md:grid-cols-3">
        <div
          v-for="step in steps"
          :key="step.num"
          class="flex flex-col rounded-lg border border-border p-4"
        >
          <div class="flex items-center gap-2">
            <span class="font-mono text-[10px] font-semibold text-brand-500">{{ step.num }}</span>
            <component :is="step.icon" class="h-4 w-4 text-zinc-400" aria-hidden="true" />
          </div>
          <h3 class="mt-2 text-[13px] font-semibold text-zinc-900">{{ step.title }}</h3>
          <p class="mt-1 flex-1 text-xs leading-5 text-zinc-500">{{ step.desc }}</p>
          <AppButton variant="primary" size="sm" class="mt-3 self-start" @click="goTo(step.name)">
            {{ step.cta }}
          </AppButton>
        </div>
      </div>
    </AppCard>

    <!-- 用 Python 3.11 重建 .venv（破坏性，需确认） -->
    <AppConfirmModal
      :open="showRebuildConfirm"
      :title="t('overview.rebuildTitle')"
      :description="t('overview.rebuildConfirmDesc')"
      :confirm-label="t('overview.rebuildVenv311')"
      :variant="'danger'"
      :loading="envFixing"
      @update:open="(v) => { if (!v) showRebuildConfirm = false }"
      @confirm="rebuildVenv311"
    />
  </div>
</template>
