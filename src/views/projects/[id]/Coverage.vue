<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRoute, useRouter } from "vue-router";
import {
  AlertCircle,
  ArrowUpDown,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock3,
  Copy,
  Download,
  FileCode2,
  FolderOpen,
  Gauge,
  ListChecks,
  Play,
  RefreshCw,
  Search,
  Square,
  XCircle,
} from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import { getDatabase } from "../../../utils/db";
import TreeItem from "../../../components/TreeItem.vue";

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();

const projectId = computed(() => Number(route.params.id));

type CoverageStatus = "idle" | "running" | "completed" | "failed";
type SortOrder = "asc" | "desc";
type CoverageFilter = "all" | "high" | "mid" | "low";

/* -------------------------------------------------------------------------- */
/* Types (与 Rust 端 camelCase payload 对应)                                   */
/* -------------------------------------------------------------------------- */

interface SourceFile {
  name: string;
  path: string;
  relativePath: string;
}

interface TestFile {
  name: string;
  path: string;
  relativePath: string;
}

interface TreeNode {
  id: string;
  name: string;
  path: string;
  relativePath: string;
  type: "file" | "directory";
  children: TreeNode[];
  depth: number;
}

interface FileCoverage {
  path: string;
  percentCovered: number;
  executedLines: number[];
  missingLines: number[];
  excludedLines: number[];
}

interface CoverageSummary {
  runId: string;
  projectId: number;
  percentCovered: number;
  totalStatements: number;
  coveredStatements: number;
  files: FileCoverage[];
  jsonPath: string;
  command: string;
}

interface CoverageStartedEvent {
  runId: string;
  projectId: number;
}

interface CoverageOutputEvent {
  runId: string;
  stream: "stdout" | "stderr";
  line: string;
  logId?: number;
}

interface CoverageFinishedEvent {
  runId: string;
  success: boolean;
  exitCode: number | null;
  duration: number;
  summary: CoverageSummary | null;
  command: string;
}

interface CoverageErrorEvent {
  runId: string;
  message: string;
}

interface LineCoverage {
  lineNumber: number;
  source: string;
  status: "covered" | "missing" | "excluded" | "not-executable";
}

interface FileCoverageDetail {
  path: string;
  percentCovered: number;
  lines: LineCoverage[];
}

const currentProject = computed(() => {
  return projectStore.projects.find((project) => project.id === projectId.value);
});

/* -------------------------------------------------------------------------- */
/* State                                                                      */
/* -------------------------------------------------------------------------- */

const isRunning = ref(false);
const coverageStatus = ref<CoverageStatus>("idle");

const sourceFiles = ref<SourceFile[]>([]);
const sourceTree = ref<TreeNode[]>([]);
const expandedDirs = ref<Set<string>>(new Set());
const selectedSourceFiles = ref<string[]>([]);
const sourceSearch = ref("");

const testFiles = ref<TestFile[]>([]);
const selectedTestFiles = ref<string[]>([]);

const isLoadingSources = ref(false);
const isLoadingTests = ref(false);
const scanError = ref<string | null>(null);

const coverageInstalled = ref<boolean | null>(null);
const checkingCoverage = ref(false);
const installingCoverage = ref(false);
const installMessage = ref<string | null>(null);

const summary = ref<CoverageSummary | null>(null);
const executionDuration = ref(0);

const sortOrder = ref<SortOrder>("desc");
const filter = ref<CoverageFilter>("all");

const expandedFile = ref<string | null>(null);
const detail = ref<FileCoverageDetail | null>(null);
const detailLoading = ref(false);
const detailError = ref<string | null>(null);
const detailCache = new Map<string, FileCoverageDetail>();

const coverageOutput = ref<CoverageOutputEvent[]>([]);
const showCoverageLog = ref(false);
const logContainer = ref<HTMLElement | null>(null);

const errorMessage = ref<string | null>(null);

const exporting = ref<"json" | "csv" | null>(null);
const exportMessage = ref<string | null>(null);

const currentRunId = ref<string | null>(null);

let unlistenStarted: UnlistenFn | undefined;
let unlistenOutput: UnlistenFn | undefined;
let unlistenFinished: UnlistenFn | undefined;
let unlistenError: UnlistenFn | undefined;
let logCounter = 0;

/* -------------------------------------------------------------------------- */
/* Source tree                                                                 */
/* -------------------------------------------------------------------------- */

const filteredSourceFiles = computed(() => {
  const query = sourceSearch.value.trim().toLowerCase();

  if (!query) return sourceFiles.value;

  return sourceFiles.value.filter(
    (file) =>
      file.name.toLowerCase().includes(query) ||
      file.relativePath.toLowerCase().includes(query),
  );
});

function buildSourceTree(files: SourceFile[]): TreeNode[] {
  const root: TreeNode[] = [];
  const dirMap = new Map<string, TreeNode>();

  for (const file of files) {
    const parts = file.relativePath.split("/").filter(Boolean);
    let currentChildren = root;
    let currentPath = "";

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isFile = i === parts.length - 1;
      currentPath = currentPath ? `${currentPath}/${part}` : part;

      if (isFile) {
        currentChildren.push({
          id: file.path,
          name: part,
          path: file.path,
          relativePath: file.relativePath,
          type: "file",
          children: [],
          depth: i,
        });
      } else {
        let dirNode = dirMap.get(currentPath);

        if (!dirNode) {
          dirNode = {
            id: `dir-${currentPath}`,
            name: part,
            path: currentPath,
            relativePath: currentPath,
            type: "directory",
            children: [],
            depth: i,
          };
          dirMap.set(currentPath, dirNode);
          currentChildren.push(dirNode);
          expandedDirs.value.add(currentPath);
        }

        currentChildren = dirNode.children;
      }
    }
  }

  return root;
}

function toggleFile(path: string) {
  if (isRunning.value) return;

  const index = selectedSourceFiles.value.indexOf(path);

  if (index === -1) {
    selectedSourceFiles.value.push(path);
  } else {
    selectedSourceFiles.value.splice(index, 1);
  }
}

function toggleDir(path: string) {
  if (expandedDirs.value.has(path)) {
    expandedDirs.value.delete(path);
  } else {
    expandedDirs.value.add(path);
  }
}

function selectAllVisible() {
  if (isRunning.value) return;

  const paths = filteredSourceFiles.value.map((f) => f.relativePath);
  selectedSourceFiles.value = Array.from(
    new Set([...selectedSourceFiles.value, ...paths]),
  );
}

function clearSourceSelection() {
  if (isRunning.value) return;
  selectedSourceFiles.value = [];
}

/** 由选中的源文件推导 coverage --source 参数：取每个文件所属目录（根目录文件直接用文件自身） */
function deriveSourceDirs(): string[] {
  const dirs = new Set<string>();

  for (const rel of selectedSourceFiles.value) {
    const idx = rel.lastIndexOf("/");

    if (idx === -1) {
      dirs.add(rel);
    } else {
      dirs.add(rel.slice(0, idx));
    }
  }

  return [...dirs].sort();
}

/* -------------------------------------------------------------------------- */
/* Test file selection                                                         */
/* -------------------------------------------------------------------------- */

function toggleTestFile(path: string) {
  if (isRunning.value) return;

  const index = selectedTestFiles.value.indexOf(path);

  if (index === -1) {
    selectedTestFiles.value.push(path);
  } else {
    selectedTestFiles.value.splice(index, 1);
  }
}

function selectAllTests() {
  if (isRunning.value) return;
  selectedTestFiles.value = testFiles.value.map((f) => f.relativePath);
}

function clearTestSelection() {
  if (isRunning.value) return;
  selectedTestFiles.value = [];
}

/* -------------------------------------------------------------------------- */
/* Computed                                                                   */
/* -------------------------------------------------------------------------- */

const selectedSourceCount = computed(() => selectedSourceFiles.value.length);
const selectedTestCount = computed(() => selectedTestFiles.value.length);

const canRun = computed(() => {
  if (isRunning.value) return false;
  if (!currentProject.value?.path) return false;
  if (!currentProject.value?.interpreter_path) return false;
  if (selectedSourceFiles.value.length === 0) return false;

  return true;
});

const statusText = computed(() => {
  switch (coverageStatus.value) {
    case "running":
      return "Running";
    case "completed":
      return "Completed";
    case "failed":
      return "Failed";
    default:
      return "Ready";
  }
});

const statusIcon = computed(() => {
  switch (coverageStatus.value) {
    case "running":
      return Clock3;
    case "completed":
      return CheckCircle2;
    case "failed":
      return XCircle;
    default:
      return AlertCircle;
  }
});

const statusClass = computed(() => {
  switch (coverageStatus.value) {
    case "running":
      return "border-sky-200 bg-sky-50 text-sky-700";
    case "completed":
      return "border-emerald-200 bg-emerald-50 text-emerald-700";
    case "failed":
      return "border-rose-200 bg-rose-50 text-rose-700";
    default:
      return "border-slate-200 bg-slate-50 text-slate-600";
  }
});

const hasResult = computed(() => summary.value !== null);

/** 文件列表：过滤 + 排序 */
const displayFiles = computed(() => {
  let list = summary.value?.files ?? [];

  if (filter.value === "high") {
    list = list.filter((f) => f.percentCovered > 80);
  } else if (filter.value === "mid") {
    list = list.filter((f) => f.percentCovered >= 50 && f.percentCovered <= 80);
  } else if (filter.value === "low") {
    list = list.filter((f) => f.percentCovered < 50);
  }

  return [...list].sort((a, b) =>
    sortOrder.value === "asc"
      ? a.percentCovered - b.percentCovered
      : b.percentCovered - a.percentCovered,
  );
});

const fileStats = computed(() => {
  const files = summary.value?.files ?? [];
  const covered = files.filter((f) => f.percentCovered > 0).length;

  return {
    total: files.length,
    covered,
    high: files.filter((f) => f.percentCovered > 80).length,
    mid: files.filter((f) => f.percentCovered >= 50 && f.percentCovered <= 80)
      .length,
    low: files.filter((f) => f.percentCovered < 50).length,
  };
});

/* -------------------------------------------------------------------------- */
/* Coverage color helpers                                                      */
/* -------------------------------------------------------------------------- */

function coverageTone(percent: number): "emerald" | "amber" | "rose" {
  if (percent >= 80) return "emerald";
  if (percent >= 50) return "amber";
  return "rose";
}

function percentTextClass(percent: number): string {
  const tone = coverageTone(percent);
  return tone === "emerald"
    ? "text-emerald-600"
    : tone === "amber"
      ? "text-amber-600"
      : "text-rose-600";
}

function barBgClass(percent: number): string {
  const tone = coverageTone(percent);
  return tone === "emerald"
    ? "bg-emerald-500"
    : tone === "amber"
      ? "bg-amber-500"
      : "bg-rose-500";
}

function cardBorderClass(percent: number): string {
  const tone = coverageTone(percent);
  return tone === "emerald"
    ? "border-emerald-200 bg-emerald-50"
    : tone === "amber"
      ? "border-amber-200 bg-amber-50"
      : "border-rose-200 bg-rose-50";
}

function lineRowClass(status: LineCoverage["status"]): string {
  switch (status) {
    case "covered":
      return "bg-emerald-50";
    case "missing":
      return "bg-rose-50";
    case "excluded":
      return "bg-amber-50";
    default:
      return "";
  }
}

function lineTextClass(status: LineCoverage["status"]): string {
  switch (status) {
    case "covered":
      return "text-emerald-900";
    case "missing":
      return "text-rose-900";
    case "excluded":
      return "text-amber-900";
    default:
      return "text-slate-400";
  }
}

/* -------------------------------------------------------------------------- */
/* Event listeners                                                            */
/* -------------------------------------------------------------------------- */

async function setupCoverageListeners() {
  unlistenStarted = await listen<CoverageStartedEvent>(
    "coverage-started",
    (event) => {
      currentRunId.value = event.payload.runId;
      isRunning.value = true;
      coverageStatus.value = "running";
      errorMessage.value = null;
      summary.value = null;
      expandedFile.value = null;
      detail.value = null;
      detailError.value = null;
      executionDuration.value = 0;
      coverageOutput.value = [];
      showCoverageLog.value = false;
      logCounter = 0;
    },
  );

  unlistenOutput = await listen<CoverageOutputEvent>(
    "coverage-output",
    (event) => {
      if (event.payload.runId !== currentRunId.value) return;

      coverageOutput.value.push({
        ...event.payload,
        logId: logCounter++,
      });

      if (coverageOutput.value.length > 1000) {
        coverageOutput.value.shift();
      }
    },
  );

  unlistenFinished = await listen<CoverageFinishedEvent>(
    "coverage-finished",
    async (event) => {
      if (event.payload.runId !== currentRunId.value) return;

      isRunning.value = false;
      executionDuration.value = event.payload.duration;

      if (event.payload.summary) {
        summary.value = event.payload.summary;
        coverageStatus.value = "completed";

        await saveCoverageToDb(event.payload);
      } else {
        coverageStatus.value = "failed";
      }

      if (!event.payload.success) {
        coverageOutput.value.push({
          runId: event.payload.runId,
          stream: "stderr",
          line: event.payload.summary
            ? "\n[警告] 测试存在失败项，但覆盖率数据已生成（summary 可用）。"
            : "\n[系统警告] 覆盖率进程异常退出，未生成有效的 coverage.json。请检查上方的日志。",
          logId: logCounter++,
        });

        if (!event.payload.summary) {
          showCoverageLog.value = true;
        }
      }
    },
  );

  unlistenError = await listen<CoverageErrorEvent>(
    "coverage-error",
    (event) => {
      errorMessage.value = event.payload.message;

      coverageOutput.value.push({
        runId: event.payload.runId,
        stream: "stderr",
        line: `\n[Coverage Error] ${event.payload.message}`,
        logId: logCounter++,
      });
      showCoverageLog.value = true;
    },
  );
}

/* -------------------------------------------------------------------------- */
/* Scanning                                                                   */
/* -------------------------------------------------------------------------- */

async function refreshScans() {
  await Promise.all([scanSourceFiles(), scanTestFiles()]);
}

async function scanSourceFiles() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) return;

  isLoadingSources.value = true;
  scanError.value = null;

  try {
    sourceFiles.value = await invoke<SourceFile[]>("scan_source_files", {
      projectPath,
    });
    sourceTree.value = buildSourceTree(sourceFiles.value);
  } catch (error) {
    console.error("[Coverage] Failed to scan source files:", error);
    scanError.value = String(error);
    sourceFiles.value = [];
    sourceTree.value = [];
  } finally {
    isLoadingSources.value = false;
  }
}

async function scanTestFiles() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) return;

  isLoadingTests.value = true;

  try {
    testFiles.value = await invoke<TestFile[]>("scan_test_files", {
      projectPath,
    });

    selectedTestFiles.value = selectedTestFiles.value.filter((path) =>
      testFiles.value.some((file) => file.relativePath === path),
    );
  } catch (error) {
    console.error("[Coverage] Failed to scan test files:", error);
    testFiles.value = [];
    selectedTestFiles.value = [];
  } finally {
    isLoadingTests.value = false;
  }
}

/* -------------------------------------------------------------------------- */
/* Coverage environment                                                        */
/* -------------------------------------------------------------------------- */

async function checkCoverageInstalled() {
  const interpreterPath = currentProject.value?.interpreter_path;

  if (!interpreterPath) {
    coverageInstalled.value = null;
    return;
  }

  checkingCoverage.value = true;

  try {
    coverageInstalled.value = await invoke<boolean>(
      "check_coverage_installed",
      { interpreterPath },
    );
  } catch (error) {
    console.error("[Coverage] check_coverage_installed failed:", error);
    coverageInstalled.value = false;
  } finally {
    checkingCoverage.value = false;
  }
}

async function installCoverage() {
  const interpreterPath = currentProject.value?.interpreter_path;

  if (!interpreterPath || installingCoverage.value) return;

  installingCoverage.value = true;
  installMessage.value = "正在安装 coverage.py ...";

  try {
    const result = await invoke<{
      success: boolean;
      installed: string[];
      failed: string[];
    }>("install_dependencies", {
      pythonPath: interpreterPath,
      packages: ["coverage"],
    });

    if (result.success) {
      installMessage.value = "✅ coverage.py 安装成功";
      await checkCoverageInstalled();
    } else {
      installMessage.value = `❌ 安装失败: ${result.failed.join(", ")}`;
    }
  } catch (error) {
    console.error("[Coverage] install_dependencies failed:", error);
    installMessage.value = `❌ 安装失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
  } finally {
    installingCoverage.value = false;
  }
}

/* -------------------------------------------------------------------------- */
/* Execution                                                                  */
/* -------------------------------------------------------------------------- */

async function runCoverage() {
  if (!canRun.value) return;

  const projectPath = currentProject.value?.path;
  const interpreterPath = currentProject.value?.interpreter_path;

  if (!projectPath || !interpreterPath) return;

  resetRunState();

  try {
    await invoke("run_coverage", {
      projectId: currentProject.value!.id,
      projectPath,
      interpreterPath,
      testFiles: selectedTestFiles.value,
      sourceDirs: deriveSourceDirs(),
    });
  } catch (error) {
    console.error("[Coverage] Failed to run coverage:", error);

    isRunning.value = false;
    coverageStatus.value = "failed";

    const errorMsg = error instanceof Error ? error.message : String(error);
    errorMessage.value = errorMsg;

    coverageOutput.value.push({
      runId: currentRunId.value ?? "local",
      stream: "stderr",
      line: `\n[Fatal Error] 覆盖率执行启动失败: ${errorMsg}`,
      logId: logCounter++,
    });
    showCoverageLog.value = true;
  }
}

function resetRunState() {
  currentRunId.value = null;
  summary.value = null;
  executionDuration.value = 0;
  expandedFile.value = null;
  detail.value = null;
  detailError.value = null;
  coverageOutput.value = [];
  errorMessage.value = null;
  coverageStatus.value = "idle";
  showCoverageLog.value = false;
}

/**
 * 将覆盖率摘要持久化到本地 SQLite 数据库（coverage_results 表）
 */
async function saveCoverageToDb(payload: CoverageFinishedEvent) {
  if (!currentProject.value?.id) return;
  if (!payload.summary) return;

  try {
    const db = await getDatabase();
    const runSummary = payload.summary;
    const coveredFileCount = runSummary.files.filter(
      (f) => f.percentCovered > 0,
    ).length;

    await db.execute(
      `INSERT INTO coverage_results
       (project_id, execution_id, total_statement_coverage, total_branch_coverage,
        file_count, covered_file_count, detail_json_path)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
      [
        currentProject.value.id,
        null, // 当前覆盖率运行未关联 test_execution_history
        runSummary.percentCovered,
        null, // 未启用 --branch，暂无分支覆盖率数据
        runSummary.files.length,
        coveredFileCount,
        runSummary.jsonPath,
      ],
    );
    console.log("[DB] ✅ Coverage summary saved successfully");
  } catch (error) {
    console.error("[DB] ❌ Failed to save coverage summary:", error);
  }
}

/* -------------------------------------------------------------------------- */
/* Detail viewer                                                              */
/* -------------------------------------------------------------------------- */

async function toggleDetail(path: string) {
  if (isRunning.value) return;

  if (expandedFile.value === path) {
    expandedFile.value = null;
    detail.value = null;
    return;
  }

  expandedFile.value = path;
  detail.value = null;
  detailError.value = null;

  const cached = detailCache.get(path);

  if (cached) {
    detail.value = cached;
    return;
  }

  detailLoading.value = true;

  try {
    const result = await invoke<FileCoverageDetail>("get_coverage_detail", {
      projectId: projectId.value,
      filePath: path,
    });
    detailCache.set(path, result);
    detail.value = result;
  } catch (error) {
    console.error("[Coverage] get_coverage_detail failed:", error);
    detailError.value = error instanceof Error ? error.message : String(error);
  } finally {
    detailLoading.value = false;
  }
}

/* -------------------------------------------------------------------------- */
/* Bottom actions                                                             */
/* -------------------------------------------------------------------------- */

async function exportReport(format: "json" | "csv") {
  if (exporting.value) return;

  exporting.value = format;
  exportMessage.value = null;

  try {
    const savedPath = await invoke<string>("export_coverage_report", {
      projectId: projectId.value,
      format,
    });
    exportMessage.value = `报告已保存到: ${savedPath}`;
  } catch (error) {
    console.error("[Coverage] export_coverage_report failed:", error);
    exportMessage.value = `导出失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
  } finally {
    exporting.value = null;
  }
}

async function openInFileManager() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) return;

  try {
    await invoke("reveal_in_folder", { path: projectPath });
  } catch (error) {
    console.error("[Coverage] reveal_in_folder failed:", error);
  }
}

/* -------------------------------------------------------------------------- */
/* Logs                                                                       */
/* -------------------------------------------------------------------------- */

async function copyLogs() {
  if (coverageOutput.value.length === 0) return;

  const text = coverageOutput.value.map((output) => output.line).join("\n");

  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    console.error("[Coverage] Failed to copy logs:", error);
  }
}

function clearOutput() {
  coverageOutput.value = [];
  showCoverageLog.value = false;
}

watch(
  () => coverageOutput.value.length,
  async () => {
    await nextTick();
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight;
    }
  },
);

/* -------------------------------------------------------------------------- */
/* Watchers / lifecycle                                                       */
/* -------------------------------------------------------------------------- */

watch(
  currentProject,
  async (project) => {
    if (!project) return;

    resetRunState();
    selectedSourceFiles.value = [];
    selectedTestFiles.value = [];
    sourceSearch.value = "";
    installMessage.value = null;

    await Promise.all([refreshScans(), checkCoverageInstalled()]);

    // 从 Execute 页确认跳转而来（?autoRun=1）：自动全选源文件并触发一次覆盖率运行
    if (route.query.autoRun === "1") {
      // 先清除 query，避免刷新页面时重复自动运行
      await router.replace({
        name: "ProjectCoverage",
        params: { id: projectId.value },
        query: {},
      });

      selectedSourceFiles.value = sourceFiles.value.map(
        (f) => f.relativePath,
      );

      void runCoverage();
    }
  },
  {
    immediate: true,
  },
);

onMounted(() => {
  void setupCoverageListeners();
});

onUnmounted(() => {
  unlistenStarted?.();
  unlistenOutput?.();
  unlistenFinished?.();
  unlistenError?.();
});
</script>

<template>
  <div class="flex min-h-full flex-col gap-5 pb-8">
    <!-- Header -->
    <header class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <div
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-sky-50 text-sky-600"
          >
            <Gauge class="h-4 w-4" />
          </div>

          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-slate-900">
              Coverage
            </h1>

            <p class="mt-0.5 truncate text-xs text-slate-500">
              {{
                currentProject?.name
                  ? `Measure test coverage for ${currentProject.name}`
                  : "Measure test coverage with coverage.py"
              }}
            </p>
          </div>
        </div>
      </div>

      <div
        class="inline-flex shrink-0 items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium"
        :class="statusClass"
      >
        <component
          :is="statusIcon"
          class="h-3.5 w-3.5"
          :class="{ 'animate-spin': coverageStatus === 'running' }"
        />

        {{ statusText }}
      </div>
    </header>

    <!-- Error banner -->
    <div
      v-if="errorMessage"
      class="flex items-start gap-2.5 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3"
    >
      <AlertCircle class="mt-0.5 h-4 w-4 shrink-0 text-rose-600" />

      <div class="min-w-0 flex-1">
        <div class="text-xs font-semibold text-rose-800">
          Coverage Error
        </div>

        <pre
          class="mt-1 whitespace-pre-wrap wrap-break-word font-mono text-[11px] leading-5 text-rose-700"
        >{{ errorMessage }}</pre>
      </div>

      <button
        type="button"
        class="rounded-lg px-2 py-1 text-[11px] font-medium text-rose-600 transition hover:bg-rose-100"
        @click="errorMessage = null"
      >
        Dismiss
      </button>
    </div>

    <!-- 1. Operation -->
    <section class="rounded-2xl border border-slate-200 bg-white">
      <div
        class="flex items-center justify-between gap-4 border-b border-slate-200 px-5 py-4"
      >
        <div>
          <div class="flex items-center gap-2">
            <ListChecks class="h-4 w-4 text-slate-500" />

            <h2 class="text-sm font-semibold text-slate-900">
              Coverage Scope
            </h2>
          </div>

          <p class="mt-1 text-xs text-slate-500">
            Choose source directories to measure and the tests to run.
          </p>
        </div>

        <button
          type="button"
          :disabled="isRunning || isLoadingSources || isLoadingTests"
          class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
          @click="refreshScans"
        >
          <RefreshCw
            class="h-3.5 w-3.5"
            :class="{ 'animate-spin': isLoadingSources || isLoadingTests }"
          />

          {{
            isLoadingSources || isLoadingTests ? "Refreshing..." : "Refresh"
          }}
        </button>
      </div>

      <div class="grid gap-5 p-5 lg:grid-cols-2">
        <!-- Source files -->
        <div class="rounded-xl border border-slate-200">
          <div
            class="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 px-4 py-3"
          >
            <div class="relative">
              <Search
                class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400"
              />

              <input
                v-model="sourceSearch"
                type="text"
                placeholder="Search source files..."
                :disabled="isRunning"
                class="w-56 rounded-lg border border-slate-200 bg-white py-2 pl-9 pr-3 text-xs text-slate-700 outline-none transition placeholder:text-slate-400 focus:border-sky-400 focus:ring-2 focus:ring-sky-100 disabled:bg-slate-50"
              />
            </div>

            <div class="flex items-center gap-1.5">
              <button
                type="button"
                :disabled="isRunning || filteredSourceFiles.length === 0"
                class="rounded-lg px-2.5 py-1.5 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="selectAllVisible"
              >
                Select All
              </button>

              <button
                type="button"
                :disabled="isRunning || selectedSourceFiles.length === 0"
                class="rounded-lg px-2.5 py-1.5 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="clearSourceSelection"
              >
                Clear
              </button>
            </div>
          </div>

          <div
            class="flex items-center justify-between border-b border-slate-100 bg-slate-50 px-4 py-2"
          >
            <span class="text-xs font-medium text-slate-600">
              {{ selectedSourceCount }} selected
            </span>

            <span class="text-xs text-slate-500">
              派生为 --source 目录
            </span>
          </div>

          <div v-if="isLoadingSources" class="px-5 py-10 text-center text-sm text-slate-500">
            Scanning source files...
          </div>

          <div
            v-else-if="sourceTree.length === 0"
            class="px-5 py-10 text-center text-sm text-slate-500"
          >
            No source files found.
          </div>

          <div v-else class="max-h-72 overflow-auto">
            <TreeItem
              v-for="node in sourceTree"
              :key="node.id"
              :node="node"
              :expanded-dirs="expandedDirs"
              :selected-files="selectedSourceFiles"
              :disabled="isRunning"
              @toggle-dir="toggleDir"
              @toggle-file="toggleFile"
            />
          </div>
        </div>

        <!-- Test files -->
        <div class="rounded-xl border border-slate-200">
          <div
            class="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 px-4 py-3"
          >
            <span class="text-xs font-medium text-slate-700">
              Test Files
              <span class="ml-1 text-slate-400">{{ testFiles.length }}</span>
            </span>

            <div class="flex items-center gap-1.5">
              <button
                type="button"
                :disabled="isRunning || testFiles.length === 0"
                class="rounded-lg px-2.5 py-1.5 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="selectAllTests"
              >
                Select All
              </button>

              <button
                type="button"
                :disabled="isRunning || selectedTestFiles.length === 0"
                class="rounded-lg px-2.5 py-1.5 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="clearTestSelection"
              >
                Clear
              </button>
            </div>
          </div>

          <div
            class="flex items-center justify-between border-b border-slate-100 bg-slate-50 px-4 py-2"
          >
            <span class="text-xs font-medium text-slate-600">
              {{ selectedTestCount }} selected
            </span>

            <span class="text-xs text-slate-500">不选则运行全部测试</span>
          </div>

          <div v-if="isLoadingTests" class="px-5 py-10 text-center text-sm text-slate-500">
            Scanning test files...
          </div>

          <div
            v-else-if="testFiles.length === 0"
            class="px-5 py-10 text-center text-sm text-slate-500"
          >
            No test files found.
          </div>

          <div v-else class="max-h-72 overflow-auto">
            <div
              v-for="file in testFiles"
              :key="file.relativePath"
              class="flex items-center gap-3 border-b border-slate-100 px-4 py-2.5 transition last:border-b-0 hover:bg-slate-50"
            >
              <button
                type="button"
                :disabled="isRunning"
                class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition disabled:cursor-not-allowed"
                :class="
                  selectedTestFiles.includes(file.relativePath)
                    ? 'border-sky-500 bg-sky-500 text-white'
                    : 'border-slate-300 bg-white'
                "
                @click="toggleTestFile(file.relativePath)"
              >
                <Check v-if="selectedTestFiles.includes(file.relativePath)" class="h-3 w-3" />
              </button>

              <FileCode2 class="h-4 w-4 shrink-0 text-slate-400" />

              <button
                type="button"
                :disabled="isRunning"
                class="min-w-0 flex-1 truncate text-left text-sm text-slate-800 disabled:cursor-not-allowed"
                @click="toggleTestFile(file.relativePath)"
              >
                {{ file.relativePath }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Scan error -->
      <div
        v-if="scanError"
        class="mx-5 mb-4 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700"
      >
        {{ scanError }}
      </div>

      <!-- Run row -->
      <div
        class="flex flex-wrap items-center justify-between gap-3 border-t border-slate-100 px-5 py-4"
      >
        <div class="flex min-w-0 items-center gap-2">
          <span
            class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium"
            :class="
              checkingCoverage
                ? 'border-slate-200 bg-slate-50 text-slate-500'
                : coverageInstalled
                  ? 'border-emerald-200 bg-emerald-50 text-emerald-700'
                  : 'border-rose-200 bg-rose-50 text-rose-700'
            "
          >
            {{
              checkingCoverage
                ? "checking coverage.py..."
                : coverageInstalled
                  ? "coverage.py installed"
                  : "coverage.py not installed"
            }}
          </span>

          <button
            v-if="coverageInstalled === false && !checkingCoverage"
            type="button"
            :disabled="installingCoverage || !currentProject?.interpreter_path"
            class="inline-flex items-center gap-2 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-xs font-medium text-rose-700 transition hover:bg-rose-100 disabled:cursor-not-allowed disabled:opacity-50"
            @click="installCoverage"
          >
            <Download class="h-3.5 w-3.5" />
            {{ installingCoverage ? "Installing..." : "Install coverage.py" }}
          </button>

          <span v-if="installMessage" class="text-[11px] text-slate-500">
            {{ installMessage }}
          </span>
        </div>

        <div class="flex items-center gap-2">
          <span v-if="coverageInstalled === false && !checkingCoverage" class="text-[11px] text-slate-400">
            需要先在用户项目的 venv 中安装 coverage.py
          </span>

          <button
            v-if="isRunning"
            type="button"
            disabled
            class="inline-flex cursor-not-allowed items-center gap-2 rounded-xl bg-slate-200 px-5 py-2.5 text-sm font-medium text-slate-500"
          >
            <Square class="h-4 w-4" />
            Running
          </button>

          <button
            v-else
            type="button"
            :disabled="!canRun"
            class="inline-flex items-center gap-2 rounded-xl bg-sky-500 px-5 py-2.5 text-sm font-medium text-white transition hover:bg-sky-600 disabled:cursor-not-allowed disabled:opacity-50"
            @click="runCoverage"
          >
            <Play class="h-4 w-4" />
            Run Coverage
          </button>
        </div>
      </div>
    </section>

    <!-- 2. Summary cards -->
    <section v-if="hasResult && summary" class="rounded-2xl border border-slate-200 bg-white">
      <div class="border-b border-slate-200 px-5 py-4">
        <div class="flex items-center justify-between gap-4">
          <div>
            <div class="text-sm font-semibold text-slate-900">
              Coverage Summary
            </div>

            <div class="mt-1 text-xs text-slate-500">
              {{ summary.files.length }} files · {{
                executionDuration.toFixed(2)
              }}s · run {{ summary.runId.slice(0, 8) }}
            </div>
          </div>

          <div class="text-right">
            <div
              class="text-2xl font-semibold"
              :class="percentTextClass(summary.percentCovered)"
            >
              {{ summary.percentCovered.toFixed(1) }}%
            </div>

            <div class="text-[11px] text-slate-400">overall</div>
          </div>
        </div>
      </div>

      <div class="grid gap-3 p-5 sm:grid-cols-3">
        <div
          class="rounded-xl border p-4"
          :class="cardBorderClass(summary.percentCovered)"
        >
          <div class="text-xs text-slate-600">Statement Coverage</div>

          <div
            class="mt-1 text-xl font-semibold"
            :class="percentTextClass(summary.percentCovered)"
          >
            {{ summary.percentCovered.toFixed(1) }}%
          </div>

          <div class="mt-1 text-[11px] text-slate-500">
            {{ summary.coveredStatements }} /
            {{ summary.totalStatements }} statements
          </div>
        </div>

        <div class="rounded-xl border border-slate-200 bg-slate-50 p-4">
          <div class="text-xs text-slate-600">Branch Coverage</div>

          <div class="mt-1 text-xl font-semibold text-slate-400">—</div>

          <div class="mt-1 text-[11px] text-slate-500">
            未启用 --branch 参数
          </div>
        </div>

        <div class="rounded-xl border border-slate-200 bg-slate-50 p-4">
          <div class="text-xs text-slate-600">Files Covered</div>

          <div class="mt-1 text-xl font-semibold text-slate-900">
            {{ fileStats.covered }}
            <span class="text-sm font-normal text-slate-400">
              / {{ fileStats.total }}
            </span>
          </div>

          <div class="mt-1 text-[11px] text-slate-500">
            覆盖率 &gt; 0% 的文件数
          </div>
        </div>
      </div>

      <div class="px-5 pb-5">
        <div class="h-2.5 overflow-hidden rounded-full bg-slate-100">
          <div
            class="h-full rounded-full transition-all duration-300"
            :class="barBgClass(summary.percentCovered)"
            :style="{ width: `${Math.min(100, summary.percentCovered)}%` }"
          />
        </div>
      </div>
    </section>

    <!-- 3. File list -->
    <section
      v-if="hasResult && summary && summary.files.length > 0"
      class="rounded-2xl border border-slate-200 bg-white"
    >
      <div
        class="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 px-5 py-4"
      >
        <div class="flex items-center gap-2">
          <FileCode2 class="h-4 w-4 text-slate-500" />

          <h2 class="text-sm font-semibold text-slate-900">
            Files
          </h2>

          <span class="text-xs text-slate-400">
            {{ displayFiles.length }} shown
          </span>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-2.5 py-1.5 text-xs font-medium text-slate-600 transition hover:bg-slate-50"
            @click="sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'"
          >
            <ArrowUpDown class="h-3 w-3" />

            {{ sortOrder === "asc" ? "Lowest first" : "Highest first" }}
          </button>

          <div class="flex items-center gap-1 rounded-lg border border-slate-200 p-0.5">
            <button
              v-for="option in [
                { id: 'all', label: `All ${fileStats.total}` },
                { id: 'high', label: `>80% ${fileStats.high}` },
                { id: 'mid', label: `50-80% ${fileStats.mid}` },
                { id: 'low', label: `<50% ${fileStats.low}` },
              ]"
              :key="option.id"
              type="button"
              class="rounded-md px-2 py-1 text-[11px] font-medium transition"
              :class="
                filter === option.id
                  ? 'bg-slate-900 text-white'
                  : 'text-slate-500 hover:bg-slate-100'
              "
              @click="filter = option.id as CoverageFilter"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
      </div>

      <div v-if="displayFiles.length === 0" class="px-5 py-10 text-center text-xs text-slate-500">
        No files match the current filter.
      </div>

      <div v-else class="divide-y divide-slate-100">
        <div v-for="file in displayFiles" :key="file.path">
          <button
            type="button"
            :disabled="isRunning"
            class="flex w-full items-center gap-3 px-5 py-3 text-left transition hover:bg-slate-50 disabled:cursor-not-allowed"
            @click="toggleDetail(file.path)"
          >
            <ChevronDown
              v-if="expandedFile === file.path"
              class="h-3.5 w-3.5 shrink-0 text-slate-400"
            />

            <ChevronRight
              v-else
              class="h-3.5 w-3.5 shrink-0 text-slate-400"
            />

            <span class="min-w-0 flex-1 truncate font-mono text-xs text-slate-700">
              {{ file.path }}
            </span>

            <span class="hidden w-32 shrink-0 sm:block">
              <span class="block h-1.5 overflow-hidden rounded-full bg-slate-100">
                <span
                  class="block h-full rounded-full"
                  :class="barBgClass(file.percentCovered)"
                  :style="{ width: `${Math.min(100, file.percentCovered)}%` }"
                />
              </span>
            </span>

            <span
              class="w-14 shrink-0 text-right text-xs font-semibold"
              :class="percentTextClass(file.percentCovered)"
            >
              {{ file.percentCovered.toFixed(1) }}%
            </span>
          </button>

          <!-- Code viewer -->
          <div
            v-if="expandedFile === file.path"
            class="border-t border-slate-100 bg-slate-50/60"
          >
            <div class="flex items-center justify-between gap-3 border-b border-slate-200 bg-white px-5 py-2.5">
              <div class="min-w-0">
                <span class="truncate font-mono text-xs font-medium text-slate-700">
                  {{ detail?.path ?? file.path }}
                </span>

                <span
                  v-if="detail"
                  class="ml-2 text-xs"
                  :class="percentTextClass(detail.percentCovered)"
                >
                  {{ detail.percentCovered.toFixed(1) }}% covered
                </span>
              </div>

              <div class="flex shrink-0 items-center gap-3 text-[10px] text-slate-500">
                <span class="flex items-center gap-1">
                  <span class="h-2 w-2 rounded-sm bg-emerald-400" /> covered
                </span>
                <span class="flex items-center gap-1">
                  <span class="h-2 w-2 rounded-sm bg-rose-400" /> missing
                </span>
                <span class="flex items-center gap-1">
                  <span class="h-2 w-2 rounded-sm bg-slate-300" /> not executable
                </span>
              </div>
            </div>

            <div v-if="detailLoading" class="px-5 py-8 text-center text-xs text-slate-500">
              Loading line details...
            </div>

            <div
              v-else-if="detailError"
              class="px-5 py-8 text-center text-xs text-rose-600"
            >
              {{ detailError }}
            </div>

            <div v-else-if="detail" class="max-h-96 overflow-auto">
              <div
                v-for="line in detail.lines"
                :key="line.lineNumber"
                class="flex font-mono text-[11px] leading-5"
                :class="lineRowClass(line.status)"
              >
                <span
                  class="w-12 shrink-0 select-none border-r border-slate-200/70 pr-2 text-right text-slate-400"
                >{{ line.lineNumber }}</span>

                <span
                  class="min-w-0 flex-1 whitespace-pre px-2"
                  :class="lineTextClass(line.status)"
                >{{ line.source }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 4. Bottom actions -->
    <section v-if="hasResult" class="rounded-2xl border border-slate-200 bg-white">
      <div class="flex flex-wrap items-center justify-between gap-3 px-5 py-4">
        <div class="min-w-0">
          <div class="text-sm font-semibold text-slate-900">Report</div>

          <div class="mt-1 truncate text-xs text-slate-500">
            {{
              exportMessage ??
              (summary ? "Export the cached coverage data or reveal it in the file manager." : "No coverage data yet.")
            }}
          </div>
        </div>

        <div class="flex shrink-0 items-center gap-2">
          <button
            type="button"
            :disabled="!summary || exporting !== null"
            class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
            @click="exportReport('json')"
          >
            <Download class="h-3.5 w-3.5" />
            {{ exporting === "json" ? "Exporting..." : "Export JSON" }}
          </button>

          <button
            type="button"
            :disabled="!summary || exporting !== null"
            class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
            @click="exportReport('csv')"
          >
            <Download class="h-3.5 w-3.5" />
            {{ exporting === "csv" ? "Exporting..." : "Export CSV" }}
          </button>

          <button
            type="button"
            :disabled="!currentProject?.path"
            class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
            @click="openInFileManager"
          >
            <FolderOpen class="h-3.5 w-3.5" />
            Open in File Manager
          </button>
        </div>
      </div>
    </section>

    <!-- 5. Realtime log -->
    <section
      v-if="coverageOutput.length > 0"
      class="rounded-2xl border border-slate-200 bg-white"
    >
      <button
        type="button"
        class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left"
        @click="showCoverageLog = !showCoverageLog"
      >
        <div class="flex min-w-0 items-center gap-3">
          <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-slate-100">
            <FileCode2 class="h-4 w-4 text-slate-500" />
          </div>

          <div class="min-w-0">
            <div class="text-sm font-medium text-slate-800">
              Coverage Output
            </div>

            <div class="mt-1 text-xs text-slate-500">
              {{ coverageOutput.length }} log lines
            </div>
          </div>
        </div>

        <ChevronDown
          class="h-4 w-4 shrink-0 text-slate-400 transition-transform"
          :class="{ 'rotate-180': showCoverageLog }"
        />
      </button>

      <div v-if="showCoverageLog" class="border-t border-slate-200">
        <div
          class="flex items-center justify-end gap-2 border-b border-slate-800 bg-slate-950 px-4 py-2"
        >
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-slate-800 hover:text-white"
            @click="copyLogs"
          >
            <Copy class="h-3 w-3" />
            Copy
          </button>

          <button
            type="button"
            class="rounded-lg px-2.5 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-slate-800 hover:text-white"
            @click="clearOutput"
          >
            Clear
          </button>
        </div>

        <div class="bg-slate-950 p-4">
          <pre
            ref="logContainer"
            class="max-h-125 overflow-auto whitespace-pre-wrap wrap-break-word font-mono text-xs leading-5"
          ><span
            v-for="output in coverageOutput"
            :key="output.logId"
            :class="output.stream === 'stderr' ? 'text-rose-300' : 'text-slate-300'"
          >{{ output.line }}
          {{ "\n" }}</span></pre>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
/* 细滚动条（日志区 / 代码查看器） */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-thumb {
  border-radius: 8px;
  background-color: rgb(148 163 184 / 0.4);
}

::-webkit-scrollbar-thumb:hover {
  background-color: rgb(148 163 184 / 0.6);
}

::-webkit-scrollbar-track {
  background: transparent;
}
</style>
