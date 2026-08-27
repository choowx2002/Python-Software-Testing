<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { AlertCircle, CheckCircle2, Clock3, FileCode2, Loader2, RotateCcw, XCircle } from "@lucide/vue";
import AppTooltip from "./ui/AppTooltip.vue";

type DetailType = "execute" | "coverage" | "generation";

const props = withDefaults(
  defineProps<{
    type: DetailType;
    id: number;
    /** 是否允许"重跑失败用例"（主窗口弹窗允许，独立详情窗口禁用） */
    allowRerun?: boolean;
    /** 覆盖率对比的上一次记录 id（传入则渲染本次 vs 上次对比） */
    compareId?: number | null;
  }>(),
  { allowRerun: true, compareId: null },
);

const emit = defineEmits<{
  (e: "rerun-failed", ids: string[]): void;
}>();

const { t } = useI18n();

interface ExecutionDetailRow {
  id: number;
  executionId: number;
  name: string;
  file: string | null;
  status: string;
  duration: number;
  errorMessage: string | null;
}

interface FileCoverageRow {
  path: string;
  percentCovered: number;
  executedLines: number[];
  missingLines: number[];
  excludedLines: number[];
}

interface GenerationFileRow {
  id: number;
  generationId: number;
  name: string;
  relativePath: string | null;
  testCaseCount: number;
  status: string | null;
}

const loading = ref(false);
const error = ref<string | null>(null);

const executionRows = ref<ExecutionDetailRow[]>([]);
const coverageRows = ref<FileCoverageRow[]>([]);
const generationRows = ref<GenerationFileRow[]>([]);

const expandedError = ref<Set<number>>(new Set());

function toggleError(id: number) {
  const next = new Set(expandedError.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expandedError.value = next;
}

const isEmpty = ref(false);

/* 覆盖率对比：上一次记录的文件级数据 */
const prevCoverageRows = ref<FileCoverageRow[]>([]);

async function load() {
  if (!props.id) return;
  loading.value = true;
  error.value = null;
  isEmpty.value = false;
  executionRows.value = [];
  coverageRows.value = [];
  prevCoverageRows.value = [];
  generationRows.value = [];
  expandedError.value = new Set();

  try {
    if (props.type === "execute") {
      executionRows.value = await invoke<ExecutionDetailRow[]>(
        "list_execution_result_details",
        { executionId: props.id },
      );
      isEmpty.value = executionRows.value.length === 0;
    } else if (props.type === "coverage") {
      const files = await invoke<FileCoverageRow[] | null>("get_coverage_history_files", {
        historyId: props.id,
      });
      coverageRows.value = files ?? [];
      if (props.compareId) {
        const prev = await invoke<FileCoverageRow[] | null>("get_coverage_history_files", {
          historyId: props.compareId,
        });
        prevCoverageRows.value = prev ?? [];
      }
      isEmpty.value = coverageRows.value.length === 0;
    } else {
      generationRows.value = await invoke<GenerationFileRow[]>(
        "list_generation_file_details",
        { generationId: props.id },
      );
      isEmpty.value = generationRows.value.length === 0;
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => [props.type, props.id, props.compareId] as const, load);
onMounted(load);

/* 失败/错误用例的 id（用于一键重跑） */
const failedIds = computed<string[]>(() =>
  executionRows.value
    .filter((r) => r.status === "failed" || r.status === "error")
    .map((r) => (r.file ? `${r.file}::${r.name}` : r.name)),
);

interface CoverageCompareRow {
  path: string;
  current: number;
  previous: number | null;
  missingLines: number[];
}

/** 合并本次与上次的文件级覆盖率，计算差值 */
const coverageComparison = computed<CoverageCompareRow[]>(() => {
  const prevMap = new Map(prevCoverageRows.value.map((f) => [f.path, f.percentCovered]));
  const merged = new Map<string, CoverageCompareRow>();
  for (const f of coverageRows.value) {
    merged.set(f.path, {
      path: f.path,
      current: f.percentCovered,
      previous: prevMap.has(f.path) ? prevMap.get(f.path)! : null,
      missingLines: f.missingLines,
    });
  }
  // 仅存在于上一次的文件（本次已移除）
  for (const f of prevCoverageRows.value) {
    if (!merged.has(f.path)) {
      merged.set(f.path, {
        path: f.path,
        current: -1, // 标记为已移除
        previous: f.percentCovered,
        missingLines: [],
      });
    }
  }
  return [...merged.values()].sort((a, b) => a.path.localeCompare(b.path));
});

function statusColor(status: string): string {
  switch (status) {
    case "passed":
      return "text-emerald-600";
    case "failed":
    case "error":
      return "text-rose-600";
    case "skipped":
      return "text-amber-600";
    default:
      return "text-zinc-500";
  }
}

function statusIcon(status: string) {
  if (status === "passed") return CheckCircle2;
  if (status === "failed" || status === "error") return XCircle;
  if (status === "skipped") return Clock3;
  return AlertCircle;
}

function statusLabel(status: string): string {
  switch (status) {
    case "passed":
      return t("execute.results.passed");
    case "failed":
      return t("execute.results.failed");
    case "error":
      return t("execute.results.error");
    case "skipped":
      return t("execute.results.skipped");
    default:
      return status;
  }
}

/** 执行结果的状态解释（含 pytest 可能的 xfail/xpassed） */
function statusExplain(status: string): string {
  switch (status) {
    case "passed":
      return t("history.detail.status.passed");
    case "failed":
      return t("history.detail.status.failed");
    case "error":
      return t("history.detail.status.error");
    case "skipped":
      return t("history.detail.status.skipped");
    case "xfailed":
      return t("history.detail.status.xfailed");
    case "xpassed":
      return t("history.detail.status.xpassed");
    default:
      return t("history.detail.status.unknown");
  }
}

/** 生成文件的状态解释 */
function genStatusExplain(status: string | null): string {
  switch (status) {
    case "success":
      return t("history.detail.status.genSuccess");
    case "empty":
      return t("history.detail.status.genEmpty");
    case "failed":
      return t("history.detail.status.genFailed");
    default:
      return t("history.detail.status.genUnknown");
  }
}

/** 非 passed 的状态均可展开查看解释 / 错误详情 */
function isExpandable(status: string): boolean {
  return status !== "passed";
}

function coverageTone(percent: number): string {
  if (percent >= 80) return "bg-emerald-500";
  if (percent >= 50) return "bg-amber-500";
  return "bg-rose-500";
}
</script>

<template>
  <div class="min-h-40">
    <!-- 加载中 -->
    <div v-if="loading" class="flex items-center justify-center py-10 text-sm text-zinc-400">
      <Loader2 class="mr-2 h-4 w-4 animate-spin" />
      {{ t("history.detail.loading") }}
    </div>

    <!-- 加载失败 -->
    <div
      v-else-if="error"
      class="rounded-lg border border-rose-200 bg-rose-50 px-3.5 py-2.5 text-xs text-rose-700"
    >
      {{ error }}
    </div>

    <!-- 空状态（旧记录无明细数据） -->
    <div
      v-else-if="isEmpty"
      class="flex flex-col items-center justify-center py-10 text-center"
    >
      <AlertCircle class="mb-2 h-8 w-8 text-zinc-200" />
      <p class="text-xs text-zinc-400">{{ t("history.detail.empty") }}</p>
    </div>

    <!-- ===== 执行结果明细 ===== -->
    <div v-else-if="type === 'execute'">
      <div v-if="allowRerun && failedIds.length > 0" class="mb-3">
        <button
          type="button"
          class="btn btn-primary btn-sm"
          @click="emit('rerun-failed', failedIds)"
        >
          <RotateCcw class="h-3.5 w-3.5" />
          {{ t("history.detail.rerunFailed", { count: failedIds.length }) }}
        </button>
        <p class="mt-1 text-[11px] text-zinc-400">{{ t("history.detail.rerunFailedHint") }}</p>
      </div>
      <div class="divide-y divide-border rounded-lg border border-border">
      <div
        v-for="row in executionRows"
        :key="row.id"
        class="px-3.5 py-2.5"
      >
        <div class="flex items-start gap-2.5">
          <component
            :is="statusIcon(row.status)"
            class="mt-0.5 h-4 w-4 shrink-0"
            :class="statusColor(row.status)"
          />
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="truncate text-[13px] font-medium text-zinc-800">{{ row.name }}</span>
              <span class="rounded-full bg-zinc-100 px-2 py-0.5 text-[10px] font-medium" :class="statusColor(row.status)">
                {{ statusLabel(row.status) }}
              </span>
              <span class="font-mono text-[10px] text-zinc-400">{{ row.duration.toFixed(3) }}s</span>
            </div>
            <div v-if="row.file" class="mt-0.5 truncate font-mono text-[10px] text-zinc-400">
              {{ row.file }}
            </div>

            <!-- 非 passed 状态：展开查看解释 + 错误详情 -->
            <div
              v-if="isExpandable(row.status)"
              class="mt-1.5"
            >
              <button
                type="button"
                class="text-[11px] font-medium text-rose-600 transition hover:text-rose-700"
                @click="toggleError(row.id)"
              >
                {{ expandedError.has(row.id) ? t("history.detail.collapse") : t("history.detail.showReason") }}
              </button>
              <div
                v-if="expandedError.has(row.id)"
                class="mt-1 rounded-md bg-zinc-50 px-3 py-2"
              >
                <p class="text-[11px] leading-5 text-zinc-600">{{ statusExplain(row.status) }}</p>
                <pre
                  v-if="row.errorMessage"
                  class="mt-1.5 max-h-48 overflow-auto whitespace-pre-wrap rounded-md bg-zinc-950 px-3 py-2 font-mono text-[10px] leading-4 text-rose-300"
                >{{ row.errorMessage }}</pre>
              </div>
            </div>
            <!-- passed：悬停显示简要解释 -->
            <AppTooltip v-else :content="statusExplain(row.status)" position="top">
              <span class="mt-1 inline-block text-[11px] text-zinc-400">
                {{ statusExplain(row.status) }}
              </span>
            </AppTooltip>
          </div>
        </div>
      </div>
      </div>
    </div>

    <!-- ===== 覆盖率文件级明细（含 vs 上次对比） ===== -->
    <div v-else-if="type === 'coverage'" class="divide-y divide-border rounded-lg border border-border">
      <!-- 对比模式 -->
      <template v-if="compareId">
        <div
          v-for="row in coverageComparison"
          :key="row.path"
          class="px-3.5 py-2.5"
        >
          <div class="flex items-center gap-3">
            <div class="min-w-0 flex-1">
              <div class="truncate font-mono text-xs text-zinc-700">{{ row.path }}</div>
              <div class="mt-1.5 flex items-center gap-2">
                <div class="h-1 w-24 overflow-hidden rounded-full bg-zinc-100">
                  <div
                    v-if="row.current >= 0"
                    class="h-full rounded-full"
                    :class="coverageTone(row.current)"
                    :style="{ width: `${Math.min(100, Math.max(0, row.current))}%` }"
                  />
                </div>
                <template v-if="row.current >= 0">
                  <span class="font-mono text-[11px]" :class="row.current >= 50 ? 'text-emerald-600' : 'text-rose-600'">
                    {{ row.current.toFixed(1) }}%
                  </span>
                  <span
                    v-if="row.previous !== null"
                    class="font-mono text-[10px]"
                    :class="row.current > row.previous ? 'text-emerald-600' : row.current < row.previous ? 'text-rose-600' : 'text-zinc-400'"
                  >
                    {{ row.current > row.previous ? "▲" : row.current < row.previous ? "▼" : "—" }}
                    {{ (row.current - row.previous) >= 0 ? "+" : "" }}{{ (row.current - row.previous).toFixed(1) }}%
                  </span>
                  <span v-else class="rounded bg-emerald-50 px-1.5 py-0.5 text-[10px] font-medium text-emerald-600">
                    {{ t("history.detail.newFile") }}
                  </span>
                </template>
                <span v-else class="rounded bg-zinc-100 px-1.5 py-0.5 text-[10px] font-medium text-zinc-500">
                  {{ t("history.detail.removedFile") }} ({{ row.previous?.toFixed(1) }}%)
                </span>
              </div>
            </div>
            <div v-if="row.current >= 0 && row.missingLines.length > 0" class="shrink-0 text-right">
              <div class="text-[11px] font-medium text-rose-600">
                {{ t("history.detail.missingLines", { count: row.missingLines.length }) }}
              </div>
              <div class="mt-0.5 max-w-40 truncate font-mono text-[10px] text-zinc-400">
                {{ row.missingLines.join(", ") }}
              </div>
            </div>
            <div v-else-if="row.current >= 0" class="shrink-0 text-[11px] font-medium text-emerald-600">
              {{ t("history.detail.fullyCovered") }}
            </div>
          </div>
        </div>
      </template>

      <!-- 普通模式 -->
      <template v-else>
        <div
          v-for="row in coverageRows"
          :key="row.path"
          class="px-3.5 py-2.5"
        >
          <div class="flex items-center gap-3">
            <div class="min-w-0 flex-1">
              <div class="truncate font-mono text-xs text-zinc-700">{{ row.path }}</div>
              <div class="mt-1.5 flex items-center gap-2">
                <div class="h-1 w-24 overflow-hidden rounded-full bg-zinc-100">
                  <div
                    class="h-full rounded-full"
                    :class="coverageTone(row.percentCovered)"
                    :style="{ width: `${Math.min(100, Math.max(0, row.percentCovered))}%` }"
                  />
                </div>
                <span class="font-mono text-[11px]" :class="row.percentCovered >= 50 ? 'text-emerald-600' : 'text-rose-600'">
                  {{ row.percentCovered.toFixed(1) }}%
                </span>
              </div>
            </div>
            <div v-if="row.missingLines.length > 0" class="shrink-0 text-right">
              <div class="text-[11px] font-medium text-rose-600">
                {{ t("history.detail.missingLines", { count: row.missingLines.length }) }}
              </div>
              <div class="mt-0.5 max-w-40 truncate font-mono text-[10px] text-zinc-400">
                {{ row.missingLines.join(", ") }}
              </div>
            </div>
            <div v-else class="shrink-0 text-[11px] font-medium text-emerald-600">
              {{ t("history.detail.fullyCovered") }}
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- ===== 生成文件明细 ===== -->
    <div v-else class="divide-y divide-border rounded-lg border border-border">
      <div
        v-for="row in generationRows"
        :key="row.id"
        class="flex items-center gap-3 px-3.5 py-2.5"
      >
        <FileCode2 class="h-4 w-4 shrink-0 text-zinc-400" />
        <div class="min-w-0 flex-1">
          <div class="truncate text-[13px] font-medium text-zinc-800">{{ row.name }}</div>
          <div v-if="row.relativePath" class="mt-0.5 truncate font-mono text-[10px] text-zinc-400">
            {{ row.relativePath }}
          </div>
        </div>
        <AppTooltip :content="genStatusExplain(row.status)" position="top">
          <span
            class="shrink-0 rounded-full bg-zinc-100 px-2 py-0.5 text-[10px] font-medium"
            :class="row.status === 'success' ? 'text-emerald-600' : row.status === 'empty' ? 'text-amber-600' : row.status === 'failed' ? 'text-rose-600' : 'text-zinc-500'"
          >
            {{ row.status }}
          </span>
        </AppTooltip>
        <span class="shrink-0 font-mono text-[11px] text-zinc-500">
          {{ t("generate.results.testCases", { count: row.testCaseCount }) }}
        </span>
      </div>
    </div>
  </div>
</template>
