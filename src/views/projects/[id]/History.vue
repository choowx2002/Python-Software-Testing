<script setup lang="ts">
/**
 * History —— 历史记录页（Timeline / Charts 双视图）
 *  Timeline：三类运行合并为时间线（类型筛选 + 日期分组 + 覆盖率差值箭头）
 *  Charts：三类合并为一张组合趋势图（覆盖率/通过率折线 + 生成事件标记）
 * 数据源：list_execution_history / list_generation_history / list_coverage_history
 */
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { Copy, Gauge, History, Loader2, Play, WandSparkles } from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import StatusPill from "../../../components/ui/StatusPill.vue";
import AppContextMenu from "../../../components/ui/AppContextMenu.vue";
import CombinedTrendChart from "../../../components/ui/CombinedTrendChart.vue";

interface ExecutionHistoryRow {
  id: number;
  projectId: number;
  executionType: string;
  regressionSuiteId: number | null;
  executionStatus: string;
  command: string | null;
  totalTests: number;
  passed: number;
  failed: number;
  skipped: number;
  executionTime: number;
  executedAt: string;
}

interface GenerationHistoryRow {
  id: number;
  projectId: number;
  generationStatus: string;
  totalFiles: number;
  generatedFiles: number;
  duration: number;
  command: string | null;
  executedAt: string;
}

interface CoverageHistoryRow {
  id: number;
  projectId: number;
  coverageStatus: string;
  percentCovered: number;
  totalStatements: number;
  coveredStatements: number;
  duration: number;
  command: string | null;
  executedAt: string;
}

type EntryType = "execute" | "generate" | "coverage";
type ViewMode = "timeline" | "charts";
type TypeFilter = "all" | EntryType;

interface TimelineEntry {
  key: string;
  type: EntryType;
  time: string;
  hm: string;
  pill: string;
  statusLabel: string;
  summary: string;
  duration: string;
  /** 覆盖率条目：与上一次覆盖率运行的差值（%），无则 null */
  delta: number | null;
}

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();
const { t } = useI18n();

/* 右键菜单：时间线条目 */
const timelineMenu = ref<{ entry: TimelineEntry; x: number; y: number } | null>(null);
function showTimelineMenu(entry: TimelineEntry, e: MouseEvent) {
  timelineMenu.value = { entry, x: e.clientX, y: e.clientY };
}
const timelineMenuItems = computed(() => {
  if (!timelineMenu.value) return [];
  const entry = timelineMenu.value.entry;
  return [
    { label: t("contextmenu.copyLine"), icon: Copy, action: () => copyText(entry.summary) },
    {
      label: t("contextmenu.viewDetails"),
      icon: History,
      action: () => void router.push({ name: "ProjectHistory", params: { id: projectId.value } }),
    },
  ];
});
async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    console.error("[History] copy failed:", error);
  }
}

const projectId = computed(() => Number(route.params.id));

const currentProject = computed(() =>
  projectStore.projects.find((p) => p.id === projectId.value),
);

const executions = ref<ExecutionHistoryRow[]>([]);
const generations = ref<GenerationHistoryRow[]>([]);
const coverages = ref<CoverageHistoryRow[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

/* ---------------- 视图状态 ---------------- */
const viewMode = ref<ViewMode>("timeline");
const typeFilter = ref<TypeFilter>("all");

const filterOptions = computed(() => [
  { id: "all" as const, label: t("history.filterAll") },
  { id: "execute" as const, label: t("history.filterExecute") },
  { id: "generate" as const, label: t("history.filterGenerate") },
  { id: "coverage" as const, label: t("history.filterCoverage") },
]);

/* ---------------- 数据加载 ---------------- */
async function loadHistory() {
  if (Number.isNaN(projectId.value)) return;

  loading.value = true;
  error.value = null;

  try {
    const [ex, gen, cov] = await Promise.all([
      invoke<ExecutionHistoryRow[]>("list_execution_history", {
        projectId: projectId.value,
      }),
      invoke<GenerationHistoryRow[]>("list_generation_history", {
        projectId: projectId.value,
      }),
      invoke<CoverageHistoryRow[]>("list_coverage_history", {
        projectId: projectId.value,
      }),
    ]);
    executions.value = ex ?? [];
    generations.value = gen ?? [];
    coverages.value = cov ?? [];
  } catch (e) {
    console.error("[History] Failed to load history:", e);
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => projectId.value, loadHistory, { immediate: true });

onMounted(() => {
  void loadHistory();
});

/* ---------------- 时间格式化 ---------------- */
function fmtFull(ts: string | null) {
  if (!ts) return "—";
  return ts.replace("T", " ").slice(0, 19);
}

function dateLabel(ts: string) {
  const d = ts.slice(0, 10);
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  const yest = new Date(now.getTime() - 86400000);
  const yesterday = `${yest.getFullYear()}-${String(yest.getMonth() + 1).padStart(2, "0")}-${String(yest.getDate()).padStart(2, "0")}`;
  if (d === today) return t("history.today");
  if (d === yesterday) return t("history.yesterday");
  return d;
}

/* ---------------- 时间线（合并三类 + 覆盖率差值） ---------------- */
const timelineEntries = computed<TimelineEntry[]>(() => {
  const raws: Omit<TimelineEntry, "hm" | "delta">[] = [];

  executions.value.forEach((r) => {
    const ok = r.executionStatus === "success";
    raws.push({
      key: `e-${r.id}`,
      type: "execute",
      time: r.executedAt,
      pill: ok ? "completed" : "Failed",
      statusLabel: ok ? t("history.statusSuccess") : t("history.statusFailed"),
      summary: `${r.passed}✓ · ${r.failed}✗${r.skipped > 0 ? ` · ${r.skipped} skip` : ""}`,
      duration: `${r.executionTime.toFixed(2)}s`,
    });
  });

  generations.value.forEach((r) => {
    const ok = r.generationStatus === "success";
    raws.push({
      key: `g-${r.id}`,
      type: "generate",
      time: r.executedAt,
      pill: ok ? "completed" : "Failed",
      statusLabel: ok ? t("history.statusSuccess") : t("history.statusFailed"),
      summary: t("history.genSummary", { generated: r.generatedFiles, total: r.totalFiles }),
      duration: `${r.duration.toFixed(1)}s`,
    });
  });

  coverages.value.forEach((r) => {
    const ok = r.coverageStatus === "success";
    raws.push({
      key: `c-${r.id}`,
      type: "coverage",
      time: r.executedAt,
      pill: ok ? "completed" : r.coverageStatus === "warning" ? "Warning" : "Failed",
      statusLabel: ok
        ? t("history.statusSuccess")
        : r.coverageStatus === "warning"
          ? t("layout.statusWarning")
          : t("history.statusFailed"),
      summary: `${r.percentCovered.toFixed(1)}% · ${t("history.covStatements", {
        covered: r.coveredStatements,
        total: r.totalStatements,
      })}`,
      duration: `${r.duration.toFixed(1)}s`,
    });
  });

  // 升序排列以计算覆盖率差值（与上一次覆盖率运行对比）
  raws.sort((a, b) => a.time.localeCompare(b.time));

  const coverageValues = new Map<string, number>();
  coverages.value.forEach((c) => coverageValues.set(`c-${c.id}`, c.percentCovered));

  let prevCoverage: number | null = null;
  const entries: TimelineEntry[] = raws.map((r) => {
    let delta: number | null = null;
    if (r.type === "coverage") {
      const value = coverageValues.get(r.key) ?? null;
      if (value !== null && prevCoverage !== null) delta = value - prevCoverage;
      if (value !== null) prevCoverage = value;
    }
    return { ...r, hm: r.time.slice(11, 16), delta };
  });

  return entries.reverse(); // 最新在前
});

const dateGroups = computed(() => {
  const filtered = timelineEntries.value.filter(
    (e) => typeFilter.value === "all" || e.type === typeFilter.value,
  );
  const groups: { label: string; entries: TimelineEntry[] }[] = [];
  for (const entry of filtered) {
    const label = dateLabel(entry.time);
    const last = groups[groups.length - 1];
    if (last && last.label === label) {
      last.entries.push(entry);
    } else {
      groups.push({ label, entries: [entry] });
    }
  }
  return groups;
});

function entryTypeLabel(type: EntryType) {
  switch (type) {
    case "execute":
      return t("history.filterExecute");
    case "generate":
      return t("history.filterGenerate");
    case "coverage":
      return t("history.filterCoverage");
  }
}

function entryTypeIcon(type: EntryType) {
  switch (type) {
    case "execute":
      return Play;
    case "generate":
      return WandSparkles;
    case "coverage":
      return Gauge;
  }
}

function entryTypeTile(type: EntryType) {
  switch (type) {
    case "execute":
      return "bg-emerald-50 text-emerald-600";
    case "generate":
      return "bg-brand-50 text-brand-600";
    case "coverage":
      return "bg-amber-50 text-amber-600";
  }
}

/* ---------------- 图表视图（合并单图） ---------------- */
interface TrendEvent {
  type: "coverage" | "passRate" | "generation";
  value?: number;
  time: string;
  detail?: string;
}

const chartEvents = computed<TrendEvent[]>(() => {
  const events: TrendEvent[] = [];

  coverages.value.forEach((r) =>
    events.push({
      type: "coverage",
      value: r.percentCovered,
      time: r.executedAt,
      detail: `${fmtFull(r.executedAt)} · ${r.percentCovered.toFixed(1)}%`,
    }),
  );

  executions.value.forEach((r) => {
    const total = r.passed + r.failed + r.skipped;
    if (total > 0) {
      events.push({
        type: "passRate",
        value: (r.passed / total) * 100,
        time: r.executedAt,
        detail: `${fmtFull(r.executedAt)} · ${r.passed}/${total} passed`,
      });
    }
  });

  generations.value.forEach((r) =>
    events.push({
      type: "generation",
      time: r.executedAt,
      detail: `${fmtFull(r.executedAt)} · ${r.generatedFiles}/${r.totalFiles} files`,
    }),
  );

  return events.sort((a, b) => a.time.localeCompare(b.time));
});

/** 最新覆盖率（列表为倒序） */
const latestCoverage = computed(() => {
  const c = coverages.value[0];
  return c ? c.percentCovered : null;
});

/** 最新一次执行的通过率 */
const latestPassRate = computed(() => {
  const e = executions.value[0];
  if (!e) return null;
  const total = e.passed + e.failed + e.skipped;
  return total > 0 ? (e.passed / total) * 100 : null;
});
</script>

<template>
  <div class="flex min-h-full flex-col gap-4 p-5">
    <!-- 页头 -->
    <header class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <div
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-zinc-100 text-zinc-500"
          >
            <History class="h-4 w-4" />
          </div>
          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-zinc-900">{{ t("history.title") }}</h1>
            <p class="mt-0.5 truncate text-xs text-zinc-500">
              {{ t("history.subtitle", { name: currentProject?.name ?? t("layout.notLoaded") }) }}
            </p>
          </div>
        </div>
      </div>

      <!-- 视图切换：Timeline / Charts -->
      <div class="inline-flex shrink-0 rounded-lg border border-border bg-zinc-50 p-0.5">
        <button
          v-for="mode in (['timeline', 'charts'] as ViewMode[])"
          :key="mode"
          type="button"
          class="rounded-md px-3 py-1.5 text-xs font-medium transition"
          :class="
            viewMode === mode
              ? 'bg-white text-zinc-900 shadow-sm'
              : 'text-zinc-500 hover:text-zinc-800'
          "
          @click="viewMode = mode"
        >
          {{ mode === "timeline" ? t("history.viewTimeline") : t("history.viewCharts") }}
        </button>
      </div>
    </header>

    <!-- 加载中 -->
    <div v-if="loading" class="flex items-center justify-center py-16 text-sm text-zinc-400">
      <Loader2 class="mr-2 h-5 w-5 animate-spin" />
      {{ t("history.loading") }}
    </div>

    <!-- 加载失败 -->
    <div
      v-else-if="error"
      class="rounded-lg border border-rose-200 bg-rose-50 px-3.5 py-2.5 text-xs text-rose-700"
    >
      {{ error }}
    </div>

    <template v-else>
      <section class="card overflow-hidden">
        <!-- 类型筛选（时间线模式） -->
        <div
          v-if="viewMode === 'timeline'"
          class="flex flex-wrap items-center gap-1 border-b border-border px-4 py-2.5"
        >
          <button
            v-for="opt in filterOptions"
            :key="opt.id"
            type="button"
            class="rounded-md px-2.5 py-1 text-[11px] font-medium transition"
            :class="
              typeFilter === opt.id
                ? 'bg-zinc-900 text-white'
                : 'text-zinc-500 hover:bg-zinc-100'
            "
            @click="typeFilter = opt.id"
          >
            {{ opt.label }}
          </button>
        </div>

        <!-- ══════ 时间线视图 ══════ -->
        <template v-if="viewMode === 'timeline'">
          <div v-if="dateGroups.length === 0" class="px-5 py-14 text-center">
            <div class="text-sm text-zinc-400">
              {{
                executions.length === 0 && generations.length === 0 && coverages.length === 0
                  ? t("history.emptyAll")
                  : t("history.emptyFiltered")
              }}
            </div>
          </div>

          <div v-else class="px-5 py-4">
            <div v-for="group in dateGroups" :key="group.label">
              <div
                class="mt-4 flex items-center gap-3 text-[10px] font-semibold uppercase tracking-wider text-zinc-400 first:mt-0"
              >
                {{ group.label }}
                <span class="h-px flex-1 bg-border" />
              </div>

              <div class="mt-2 space-y-0.5">
                <div
                  v-for="entry in group.entries"
                  :key="entry.key"
                  class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg px-2 py-1.5 transition hover:bg-zinc-50"
                  @contextmenu.prevent="showTimelineMenu(entry, $event)"
                >
                  <!-- 类型标识 -->
                  <span
                    class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md"
                    :class="entryTypeTile(entry.type)"
                  >
                    <component :is="entryTypeIcon(entry.type)" class="h-3.5 w-3.5" />
                  </span>

                  <span class="w-12 shrink-0 font-mono text-[11px] text-zinc-400">
                    {{ entry.hm }}
                  </span>

                  <span class="w-24 shrink-0 text-[11px] font-medium text-zinc-500">
                    {{ entryTypeLabel(entry.type) }}
                  </span>

                  <StatusPill :status="entry.pill" :label="entry.statusLabel" />

                  <span class="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-700">
                    {{ entry.summary }}
                  </span>

                  <!-- 覆盖率差值箭头（与上一次覆盖率运行对比） -->
                  <span
                    v-if="entry.delta !== null"
                    class="shrink-0 font-mono text-[11px] font-semibold"
                    :class="entry.delta >= 0 ? 'text-emerald-600' : 'text-rose-600'"
                  >
                    {{ entry.delta >= 0 ? "▲" : "▼" }} {{ Math.abs(entry.delta).toFixed(1) }}%
                  </span>

                  <span class="w-14 shrink-0 text-right font-mono text-[11px] text-zinc-400">
                    {{ entry.duration }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </template>

        <!-- ══════ 图表视图（合并单图） ══════ -->
        <template v-else>
          <div class="px-5 py-4">
            <!-- 图例 + 最新值 -->
            <div class="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-zinc-600">
              <span class="flex items-center gap-1.5">
                <span class="h-2 w-2 rounded-full bg-amber-500" />
                {{ t("history.legendCoverage") }}
                <span class="font-mono font-semibold text-zinc-800">
                  {{ latestCoverage !== null ? `${latestCoverage.toFixed(1)}%` : "—" }}
                </span>
              </span>
              <span class="flex items-center gap-1.5">
                <span class="h-2 w-2 rounded-full bg-emerald-500" />
                {{ t("history.legendPassRate") }}
                <span class="font-mono font-semibold text-zinc-800">
                  {{ latestPassRate !== null ? `${latestPassRate.toFixed(1)}%` : "—" }}
                </span>
              </span>
              <span class="flex items-center gap-1.5">
                <span class="h-2 w-2 rounded-full bg-brand-500" />
                {{ t("history.legendGeneration") }}
                <span class="font-mono font-semibold text-zinc-800">{{ generations.length }}</span>
              </span>
            </div>

            <!-- 组合趋势图 -->
            <div v-if="chartEvents.length > 1" class="mt-4">
              <CombinedTrendChart :events="chartEvents" />
            </div>
            <div v-else class="mt-4 flex h-44 flex-col items-center justify-center text-center">
              <Gauge class="mb-2 h-8 w-8 text-zinc-200" />
              <p class="max-w-sm text-xs text-zinc-400">{{ t("history.chartEmpty") }}</p>
            </div>
          </div>
        </template>
      </section>
    </template>

    <!-- 时间线条目右键菜单 -->
    <AppContextMenu
      v-if="timelineMenu"
      :items="timelineMenuItems"
      :open="!!timelineMenu"
      @close="timelineMenu = null"
    />
  </div>
</template>
