<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRoute } from "vue-router";
import {
  AlertCircle,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  CircleDot,
  ClipboardCopy,
  Clock3,
  Code2,
  Copy,
  ExternalLink,
  FileCode2,
  Folder,
  FolderOpen,
  RefreshCw,
  Search,
  Settings2,
  Sparkles,
  Square,
  Timer,
  XCircle,
} from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import TreeItem from "../../../components/TreeItem.vue";
import { useI18n } from "vue-i18n";

const route = useRoute();
const projectStore = useProjectStore();
const { t } = useI18n();

const projectId = computed(() => Number(route.params.id));

type GenerationStatus = "idle" | "running" | "completed" | "failed";
type Algorithm = "MOSA" | "DYNAMOSA" | "WSPA" | "RANDOM";

interface SourceFile {
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
  expanded: boolean;
  depth: number;
}

interface GenerationOutputEvent {
  runId: string;
  stream: "stdout" | "stderr";
  line: string;
  logId?: number;
}

interface GenerationStartedEvent {
  runId: string;
  totalFiles: number;
}

interface GenerationProgressEvent {
  runId: string;
  currentFile: string;
  completedFiles: number;
  totalFiles: number;
  elapsedTime: number;
}

interface GeneratedFile {
  name: string;
  path: string;
  relativePath: string;
  testCaseCount: number;
  status: "success" | "empty" | "failed";
}

interface GenerationFinishedEvent {
  runId: string;
  success: boolean;
  exitCode: number | null;
  duration: number;
  generatedFiles: GeneratedFile[];
  command: string;
}

const currentProject = computed(() => {
  return projectStore.projects.find((project) => project.id === projectId.value);
});

/* -------------------------------------------------------------------------- */
/* State                                                                      */
/* -------------------------------------------------------------------------- */

const isGenerating = ref(false);
const generationStatus = ref<GenerationStatus>("idle");

const sourceFiles = ref<SourceFile[]>([]);
const selectedSourceFiles = ref<string[]>([]);
const sourceTree = ref<TreeNode[]>([]);
const expandedDirs = ref<Set<string>>(new Set());

const isLoadingSources = ref(false);
const sourceScanError = ref<string | null>(null);

const sourceSearch = ref("");

const maxSearchTime = ref(60);
const algorithm = ref<Algorithm>("MOSA");
const assertionGeneration = ref(true);
const maxTestCases = ref(50);
const outputFolder = ref("tests/generated");

const showAdvanced = ref(false);
const seed = ref<number | null>(null);
const chromosomeLength = ref(40);
const populationSize = ref(50);

const generationOutput = ref<GenerationOutputEvent[]>([]);
const showGenerationLog = ref(false);

const currentRunId = ref<string | null>(null);
const currentFile = ref<string | null>(null);

const totalFiles = ref(0);
const completedFiles = ref(0);
const elapsedTime = ref(0);

const generatedFiles = ref<GeneratedFile[]>([]);

let unlistenStarted: UnlistenFn | undefined;
let unlistenProgress: UnlistenFn | undefined;
let unlistenOutput: UnlistenFn | undefined;
let unlistenFinished: UnlistenFn | undefined;
let logCounter = 0;

/* -------------------------------------------------------------------------- */
/* Source Tree                                                                */
/* -------------------------------------------------------------------------- */

const filteredSourceFiles = computed(() => {
  const query = sourceSearch.value.trim().toLowerCase();

  if (!query) {
    return sourceFiles.value;
  }

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
          expanded: false,
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
            expanded: true,
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
  if (isGenerating.value) return;

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

const selectedCount = computed(() => selectedSourceFiles.value.length);

const estimatedTimeout = computed(() => {
  return maxSearchTime.value * Math.max(1, selectedCount.value);
});

function selectAllVisible() {
  if (isGenerating.value) return;

  const paths = filteredSourceFiles.value.map((f) => f.relativePath);
  selectedSourceFiles.value = Array.from(
    new Set([...selectedSourceFiles.value, ...paths]),
  );
}

function clearSelection() {
  if (isGenerating.value) return;
  selectedSourceFiles.value = [];
}

const allVisibleSelected = computed(() => {
  if (filteredSourceFiles.value.length === 0) return false;

  return filteredSourceFiles.value.every((f) =>
    selectedSourceFiles.value.includes(f.relativePath),
  );
});

function toggleVisibleSelection() {
  if (allVisibleSelected.value) {
    const visiblePaths = new Set(
      filteredSourceFiles.value.map((f) => f.relativePath),
    );
    selectedSourceFiles.value = selectedSourceFiles.value.filter(
      (p) => !visiblePaths.has(p),
    );
  } else {
    selectAllVisible();
  }
}

/* -------------------------------------------------------------------------- */
/* Computed                                                                   */
/* -------------------------------------------------------------------------- */

const canGenerate = computed(() => {
  if (isGenerating.value) return false;
  if (!currentProject.value?.path) return false;
  if (!currentProject.value?.interpreter_path) return false;
  if (selectedSourceFiles.value.length === 0) return false;
  if (maxSearchTime.value <= 0) return false;

  return true;
});

const statusText = computed(() => {
  switch (generationStatus.value) {
    case "running":
      return t("generate.status.generating");
    case "completed":
      return t("generate.status.completed");
    case "failed":
      return t("generate.status.failed");
    default:
      return t("generate.status.ready");
  }
});

const statusIcon = computed(() => {
  switch (generationStatus.value) {
    case "running":
      return Sparkles;
    case "completed":
      return CheckCircle2;
    case "failed":
      return XCircle;
    default:
      return AlertCircle;
  }
});

const statusClass = computed(() => {
  switch (generationStatus.value) {
    case "running":
      return "border-violet-200 bg-violet-50 text-violet-700";
    case "completed":
      return "border-emerald-200 bg-emerald-50 text-emerald-700";
    case "failed":
      return "border-rose-200 bg-rose-50 text-rose-700";
    default:
      return "border-slate-200 bg-slate-50 text-slate-600";
  }
});

const progress = computed(() => {
  if (totalFiles.value <= 0) return 0;
  return Math.min(
    100,
    Math.round((completedFiles.value / totalFiles.value) * 100),
  );
});

const hasResult = computed(() => {
  return (
    generationStatus.value === "completed" ||
    generationStatus.value === "failed"
  );
});

const lastRunSummary = computed(() => {
  if (!hasResult.value) return t("generate.noRunYet");
  return `${generatedFiles.value.length} files · ${elapsedTime.value.toFixed(2)}s`;
});

const resultConclusion = computed(() => {
  if (generationStatus.value === "completed") {
    const successCount = generatedFiles.value.filter(
      (f) => f.status === "success",
    ).length;
    const emptyCount = generatedFiles.value.filter(
      (f) => f.status === "empty",
    ).length;

    if (successCount > 0 && emptyCount === 0) {
      return {
        title: t("generate.conclusion.successTitle"),
        description: t("generate.conclusion.successDesc", { count: successCount }),
        type: "success" as const,
      };
    }

    if (successCount > 0) {
      return {
        title: t("generate.conclusion.partialTitle"),
        description: t("generate.conclusion.partialDesc", {
          success: successCount,
          empty: emptyCount,
        }),
        type: "warning" as const,
      };
    }

    return {
      title: t("generate.noTestsGeneratedTitle"),
      description: t("generate.noTestsGeneratedDesc"),
      type: "warning" as const,
    };
  }

  return {
    title: t("generate.conclusion.failedTitle"),
    description: t("generate.conclusion.failedDesc"),
    type: "error" as const,
  };
});

/* -------------------------------------------------------------------------- */
/* Event listeners                                                            */
/* -------------------------------------------------------------------------- */

async function setupGenerationListeners() {
  unlistenStarted = await listen<GenerationStartedEvent>(
    "generation-started",
    (event) => {
      currentRunId.value = event.payload.runId;
      totalFiles.value = event.payload.totalFiles;
      completedFiles.value = 0;
      elapsedTime.value = 0;
      currentFile.value = null;
      generatedFiles.value = [];
      generationOutput.value = [];
      generationStatus.value = "running";
      isGenerating.value = true;
      showGenerationLog.value = false;
      logCounter = 0;
    },
  );

  unlistenProgress = await listen<GenerationProgressEvent>(
    "generation-progress",
    (event) => {
      if (event.payload.runId !== currentRunId.value) return;

      currentFile.value = event.payload.currentFile;
      completedFiles.value = event.payload.completedFiles;
      elapsedTime.value = event.payload.elapsedTime;
    },
  );

  unlistenOutput = await listen<GenerationOutputEvent>(
    "generation-output",
    (event) => {
      if (event.payload.runId !== currentRunId.value) return;

      generationOutput.value.push({
        ...event.payload,
        logId: logCounter++,
      });

      if (generationOutput.value.length > 1000) {
        generationOutput.value.shift();
      }
    },
  );

  unlistenFinished = await listen<GenerationFinishedEvent>(
    "generation-finished",
    async (event) => {
      if (event.payload.runId !== currentRunId.value) return;

      isGenerating.value = false;
      generationStatus.value = event.payload.success ? "completed" : "failed";
      elapsedTime.value = event.payload.duration;
      generatedFiles.value = event.payload.generatedFiles;
      currentFile.value = null;

      if (!event.payload.success && event.payload.generatedFiles.length === 0) {
        generationOutput.value.push({
          runId: event.payload.runId,
          stream: "stderr",
          line: t("generate.logs.systemWarning"),
          logId: logCounter++,
        });
        showGenerationLog.value = true;
      }
    },
  );
}

/* -------------------------------------------------------------------------- */
/* Source scanning                                                            */
/* -------------------------------------------------------------------------- */

async function scanSourceFiles() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) {
    sourceScanError.value = t("execute.projectPathUnavailable");
    return;
  }

  isLoadingSources.value = true;
  sourceScanError.value = null;

  try {
    sourceFiles.value = await invoke<SourceFile[]>("scan_source_files", {
      projectPath,
    });

    sourceTree.value = buildSourceTree(sourceFiles.value);
  } catch (error) {
    console.error("[Generate] Failed to scan source files:", error);
    sourceScanError.value = String(error);
    sourceFiles.value = [];
    sourceTree.value = [];
  } finally {
    isLoadingSources.value = false;
  }
}

/* -------------------------------------------------------------------------- */
/* Generation                                                                 */
/* -------------------------------------------------------------------------- */

async function generateTests() {
  if (!canGenerate.value) return;

  const projectPath = currentProject.value?.path;
  const interpreterPath = currentProject.value?.interpreter_path;

  if (!projectPath || !interpreterPath) return;

  resetGenerationState();

  try {
    await invoke("generate_tests", {
      projectId: currentProject.value!.id,
      projectPath,
      interpreterPath,
      sourceFiles: selectedSourceFiles.value,
      maxSearchTime: maxSearchTime.value,
      algorithm: algorithm.value,
      assertionGeneration: assertionGeneration.value,
      maxTestCases: maxTestCases.value,
      outputFolder: outputFolder.value,
      seed: seed.value,
      chromosomeLength: chromosomeLength.value,
      populationSize: populationSize.value,
    });
  } catch (error) {
    console.error("[Generate] Failed to start generation:", error);

    isGenerating.value = false;
    generationStatus.value = "failed";
    currentFile.value = null;

    const errorMsg = error instanceof Error ? error.message : String(error);

    generationOutput.value.push({
      runId: currentRunId.value ?? "local",
      stream: "stderr",
      line: t("generate.logs.fatalStart", { msg: errorMsg }),
      logId: logCounter++,
    });
    showGenerationLog.value = true;
  }
}

/**
 * 取消当前生成任务：调用 Rust cancel_run（SIGTERM → SIGKILL）。
 * 进程被终止后，generate_tests 会在下一轮循环检测到取消标记，
 * 发出 generation-finished（success=false），由 listener 复位 UI 状态。
 */
async function stopGeneration() {
  if (!isGenerating.value || !currentRunId.value) {
    return;
  }

  try {
    await invoke("cancel_run", { runId: currentRunId.value });
  } catch (error) {
    console.error("[Generate] Failed to cancel generation:", error);
  }
}

function resetGenerationState() {
  currentRunId.value = null;
  currentFile.value = null;
  totalFiles.value = selectedSourceFiles.value.length;
  completedFiles.value = 0;
  elapsedTime.value = 0;
  generatedFiles.value = [];
  generationOutput.value = [];
  generationStatus.value = "idle";
  showGenerationLog.value = false;
}

/* -------------------------------------------------------------------------- */
/* File actions                                                               */
/* -------------------------------------------------------------------------- */

async function openFile(path: string) {
  try {
    await invoke("open_file", { path });
  } catch (error) {
    console.error("[Generate] Failed to open file:", error);
  }
}

async function revealFile(path: string) {
  try {
    await invoke("reveal_in_folder", { path });
  } catch (error) {
    console.error("[Generate] Failed to reveal file:", error);
  }
}

async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
  } catch (error) {
    console.error("[Generate] Failed to copy path:", error);
  }
}

async function copyLogs() {
  if (generationOutput.value.length === 0) return;

  const text = generationOutput.value.map((o) => o.line).join("\n");

  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    console.error("[Generate] Failed to copy logs:", error);
  }
}

function clearOutput() {
  generationOutput.value = [];
  showGenerationLog.value = false;
}

function getGeneratedFileIcon(status: GeneratedFile["status"]) {
  switch (status) {
    case "success":
      return CheckCircle2;
    case "empty":
      return AlertCircle;
    case "failed":
      return XCircle;
  }
}

function getGeneratedFileClass(status: GeneratedFile["status"]) {
  switch (status) {
    case "success":
      return "text-emerald-500";
    case "empty":
      return "text-amber-500";
    case "failed":
      return "text-rose-500";
  }
}

/* -------------------------------------------------------------------------- */
/* Watchers / lifecycle                                                       */
/* -------------------------------------------------------------------------- */

watch(
  sourceFiles,
  (files) => {
    sourceTree.value = buildSourceTree(files);
  },
);

watch(
  currentProject,
  async (project) => {
    if (!project) return;

    resetGenerationState();
    selectedSourceFiles.value = [];
    sourceSearch.value = "";

    await scanSourceFiles();
  },
  {
    immediate: true,
  },
);

onMounted(() => {
  void setupGenerationListeners();
});

onUnmounted(() => {
  unlistenStarted?.();
  unlistenProgress?.();
  unlistenOutput?.();
  unlistenFinished?.();
});
</script>

<template>
  <div class="flex min-h-full flex-col gap-5 pb-8">
    <!-- Header -->
    <header class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <div
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-violet-50 text-violet-600"
          >
            <Sparkles class="h-4 w-4" />
          </div>

          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-slate-900">
              {{ t("generate.title") }}
            </h1>

            <p class="mt-0.5 truncate text-xs text-slate-500">
              {{
                currentProject?.name
                  ? t("generate.subtitle", { name: currentProject.name })
                  : t("generate.subtitle", { name: t("layout.notLoaded") })
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
          :class="{ 'animate-pulse': generationStatus === 'running' }"
        />

        {{ statusText }}
      </div>
    </header>

    <!-- Main Grid: Source Selection + Configuration -->
    <section class="grid grid-cols-1 gap-5 lg:grid-cols-2">
      <!-- Source Selection -->
      <div class="rounded-2xl border border-slate-200 bg-white">
        <div
          class="flex items-center justify-between gap-4 border-b border-slate-200 px-5 py-4"
        >
          <div>
            <div class="flex items-center gap-2">
              <FolderOpen class="h-4 w-4 text-slate-500" />

              <h2 class="text-sm font-semibold text-slate-900">
                {{ t("generate.selectSource") }}
              </h2>
            </div>

            <p class="mt-1 text-xs text-slate-500">
              {{ t("generate.selectSourceDesc") }}
            </p>
          </div>

          <button
            type="button"
            :disabled="isLoadingSources || isGenerating"
            class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
            @click="scanSourceFiles"
          >
            <RefreshCw
              class="h-3.5 w-3.5"
              :class="{ 'animate-spin': isLoadingSources }"
            />

            {{ isLoadingSources ? t("generate.scanning") : t("common.refresh") }}
          </button>
        </div>

        <div class="p-5">
          <!-- Search -->
          <div class="relative mb-3">
            <Search
              class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400"
            />

            <input
              v-model="sourceSearch"
              type="text"
              :placeholder="t('generate.searchPlaceholder')"
              :disabled="isGenerating"
              class="w-full rounded-xl border border-slate-200 bg-white py-2.5 pl-9 pr-3 text-xs text-slate-700 outline-none transition placeholder:text-slate-400 focus:border-violet-400 focus:ring-2 focus:ring-violet-100 disabled:bg-slate-50"
            />
          </div>

          <!-- Selection summary -->
          <div
            class="mb-3 flex items-center justify-between rounded-xl border border-slate-100 bg-slate-50 px-3 py-2"
          >
            <div class="flex items-center gap-2">
              <button
                type="button"
                :disabled="isGenerating || filteredSourceFiles.length === 0"
                class="flex h-4 w-4 items-center justify-center rounded border transition disabled:cursor-not-allowed"
                :class="
                  allVisibleSelected
                    ? 'border-violet-500 bg-violet-500 text-white'
                    : 'border-slate-300 bg-white'
                "
                @click="toggleVisibleSelection"
              >
                <Check v-if="allVisibleSelected" class="h-3 w-3" />
              </button>

              <span class="text-xs font-medium text-slate-600">
                {{ t("generate.selectedCount", { count: selectedCount }) }}
              </span>
            </div>

            <div class="flex items-center gap-2">
              <button
                type="button"
                :disabled="
                  isGenerating ||
                  filteredSourceFiles.length === 0 ||
                  allVisibleSelected
                "
                class="rounded-lg px-2.5 py-1 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="selectAllVisible"
              >
                {{ t("generate.selectAll") }}
              </button>

              <button
                type="button"
                :disabled="isGenerating || selectedCount === 0"
                class="rounded-lg px-2.5 py-1 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="clearSelection"
              >
                {{ t("generate.clear") }}
              </button>
            </div>
          </div>

          <!-- Tree -->
          <div
            v-if="isLoadingSources"
            class="px-5 py-10 text-center text-sm text-slate-500"
          >
            {{ t("generate.scanningSources") }}
          </div>

          <div
            v-else-if="sourceScanError"
            class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700"
          >
            {{ sourceScanError }}
          </div>

          <div v-else-if="sourceFiles.length === 0" class="px-5 py-10 text-center">
            <div class="text-sm font-medium text-slate-700">
              {{ t("generate.noSources") }}
            </div>

            <div class="mt-1 text-xs text-slate-500">
              {{ t("generate.noSourcesDesc") }}
            </div>
          </div>

          <div
            v-else-if="filteredSourceFiles.length === 0"
            class="px-5 py-10 text-center text-sm text-slate-500"
          >
            {{ t("generate.noSearchResults") }}
          </div>

          <div v-else class="max-h-96 overflow-auto rounded-xl border border-slate-200">
            <template v-for="node in sourceTree" :key="node.id">
              <TreeItem
                :node="node"
                :expanded-dirs="expandedDirs"
                :selected-files="selectedSourceFiles"
                :disabled="isGenerating"
                @toggle-dir="toggleDir"
                @toggle-file="toggleFile"
              />
            </template>
          </div>
        </div>
      </div>

      <!-- Configuration -->
      <div class="rounded-2xl border border-slate-200 bg-white">
        <div class="border-b border-slate-200 px-5 py-4">
          <div class="flex items-center gap-2">
            <Settings2 class="h-4 w-4 text-slate-500" />

            <h2 class="text-sm font-semibold text-slate-900">
              {{ t("generate.configTitle") }}
            </h2>
          </div>

          <p class="mt-1 text-xs text-slate-500">
            Configure how Pynguin generates the tests.
          </p>
        </div>

        <div class="p-5">
          <div class="space-y-5">
            <!-- Max Search Time -->
            <div>
              <div class="mb-2 flex items-center justify-between">
                <label class="text-xs font-medium text-slate-700">
                  {{ t("generate.timeLimit") }}
                </label>

                <div class="flex items-center gap-1 text-xs text-slate-500">
                  <Timer class="h-3 w-3" />
                  <span>{{ t("generate.perFile") }}</span>
                </div>
              </div>

              <div class="flex items-center gap-2">
                <input
                  v-model.number="maxSearchTime"
                  type="number"
                  min="1"
                  max="3600"
                  :disabled="isGenerating"
                  class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-violet-400 focus:ring-2 focus:ring-violet-100 disabled:bg-slate-50"
                />

                <span class="shrink-0 text-xs text-slate-500">{{ t("generate.timeLimitUnit") }}</span>
              </div>

              <p class="mt-1.5 text-[11px] text-slate-400">
                {{ t("generate.estimatedTotal", { seconds: estimatedTimeout }) }}
              </p>
            </div>

            <!-- Algorithm -->
            <div>
              <label class="mb-2 block text-xs font-medium text-slate-700">
                {{ t("generate.algorithm") }}
              </label>

              <div class="grid grid-cols-2 gap-2">
                <button
                  v-for="alg in (['MOSA', 'DYNAMOSA', 'WSPA', 'RANDOM'] as Algorithm[])"
                  :key="alg"
                  type="button"
                  :disabled="isGenerating"
                  class="flex items-center gap-2 rounded-xl border px-3 py-2.5 text-left text-sm transition disabled:cursor-not-allowed"
                  :class="
                    algorithm === alg
                      ? 'border-violet-300 bg-violet-50 text-violet-700'
                      : 'border-slate-200 text-slate-700 hover:bg-slate-50'
                  "
                  @click="algorithm = alg"
                >
                  <CircleDot
                    v-if="algorithm === alg"
                    class="h-4 w-4 text-violet-600"
                  />
                  <Circle v-else class="h-4 w-4 text-slate-300" />
                  <span class="font-medium">{{ alg }}</span>
                </button>
              </div>
            </div>

            <!-- Assertion Generation -->
            <div
              class="flex items-center justify-between rounded-xl border border-slate-200 px-4 py-3"
            >
              <div>
                <div class="text-sm font-medium text-slate-900">
                  {{ t("generate.assertGen") }}
                </div>

                <div class="mt-0.5 text-xs text-slate-500">
                  {{ t("generate.assertGenDesc") }}
                </div>
              </div>

              <button
                type="button"
                :disabled="isGenerating"
                class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition disabled:cursor-not-allowed"
                :class="assertionGeneration ? 'bg-violet-500' : 'bg-slate-200'"
                @click="assertionGeneration = !assertionGeneration"
              >
                <span
                  class="inline-block h-4 w-4 transform rounded-full bg-white transition"
                  :class="assertionGeneration ? 'translate-x-6' : 'translate-x-1'"
                />
              </button>
            </div>

            <!-- Max Test Cases -->
            <div>
              <label class="mb-2 block text-xs font-medium text-slate-700">
                {{ t("generate.maxTestCases") }}
              </label>

              <input
                v-model.number="maxTestCases"
                type="number"
                min="1"
                max="500"
                :disabled="isGenerating"
                class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-violet-400 focus:ring-2 focus:ring-violet-100 disabled:bg-slate-50"
              />

              <p class="mt-1.5 text-[11px] text-slate-400">
                {{ t("generate.maxTestCasesDesc") }}
              </p>
            </div>

            <!-- Output Folder -->
            <div>
              <label class="mb-2 block text-xs font-medium text-slate-700">
                {{ t("generate.outputFolder") }}
              </label>

              <input
                v-model="outputFolder"
                type="text"
                placeholder="tests/generated"
                :disabled="isGenerating"
                class="w-full rounded-xl border border-slate-200 px-3 py-2.5 font-mono text-xs text-slate-700 outline-none transition focus:border-violet-400 focus:ring-2 focus:ring-violet-100 disabled:bg-slate-50"
              />

              <p class="mt-1.5 text-[11px] text-slate-400">
                {{ t("generate.outputFolderDesc") }}
              </p>
            </div>

            <!-- Advanced -->
            <div class="border-t border-slate-100 pt-4">
              <button
                type="button"
                class="flex items-center gap-2 text-xs font-medium text-slate-600 transition hover:text-slate-900"
                @click="showAdvanced = !showAdvanced"
              >
                <ChevronDown v-if="showAdvanced" class="h-4 w-4" />
                <ChevronRight v-else class="h-4 w-4" />

                {{ t("execute.advancedOptions") }}
              </button>

              <div v-if="showAdvanced" class="mt-4 space-y-4">
                <div>
                  <label class="mb-2 block text-xs font-medium text-slate-700">
                    {{ t("generate.seed") }}
                    <span class="text-slate-400">({{ t("common.optional") }})</span>
                  </label>

                  <input
                    v-model.number="seed"
                    type="number"
                    placeholder="None"
                    :disabled="isGenerating"
                    class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-violet-400 focus:ring-2 focus:ring-violet-100 disabled:bg-slate-50"
                  />
                </div>

                <div>
                  <label class="mb-2 block text-xs font-medium text-slate-700">
                    {{ t("generate.chromosomeLength") }}
                  </label>

                  <input
                    v-model.number="chromosomeLength"
                    type="number"
                    min="1"
                    :disabled="isGenerating"
                    class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-violet-400 focus:ring-2 focus:ring-violet-100 disabled:bg-slate-50"
                  />
                </div>

                <div>
                  <label class="mb-2 block text-xs font-medium text-slate-700">
                    {{ t("generate.populationSize") }}
                  </label>

                  <input
                    v-model.number="populationSize"
                    type="number"
                    min="1"
                    :disabled="isGenerating"
                    class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-violet-400 focus:ring-2 focus:ring-violet-100 disabled:bg-slate-50"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Generate Button -->
    <section class="rounded-2xl border border-slate-200 bg-white p-5">
      <div class="flex items-center justify-between gap-4">
        <div>
          <div class="text-sm font-semibold text-slate-900">
            {{ t("generate.generateTests") }}
          </div>

          <div class="mt-1 text-xs text-slate-500">
            {{
              selectedCount > 0
                ? t("generate.generateSelectedDesc", { count: selectedCount, seconds: estimatedTimeout })
                : t("generate.generateDesc")
            }}
          </div>
        </div>

        <button
          v-if="isGenerating"
          type="button"
          class="inline-flex items-center gap-2 rounded-xl bg-rose-500 px-6 py-3 text-sm font-medium text-white transition hover:bg-rose-600"
          @click="stopGeneration"
        >
          <Square class="h-4 w-4" />
          {{ t("generate.stop") }}
        </button>

        <button
          v-else
          type="button"
          :disabled="!canGenerate"
          class="inline-flex items-center gap-2 rounded-xl bg-violet-500 px-6 py-3 text-sm font-medium text-white transition hover:bg-violet-600 disabled:cursor-not-allowed disabled:opacity-50"
          @click="generateTests"
        >
          <Sparkles class="h-4 w-4" />
          {{ t("generate.generateTests") }}
        </button>
      </div>
    </section>

    <!-- Progress -->
    <section v-if="generationStatus === 'running'" class="rounded-2xl border border-violet-200 bg-white">
      <div
        class="flex items-center justify-between gap-4 border-b border-slate-200 px-5 py-4"
      >
        <div>
          <div class="text-sm font-semibold text-slate-900">
            {{ t("generate.progress.title") }}
          </div>

          <div class="mt-1 text-xs text-slate-500">
            {{ t("generate.progress.desc") }}
          </div>
        </div>

        <div class="text-xs font-medium text-violet-600">
          {{ t("generate.progress.filesDone", { completed: completedFiles, total: totalFiles }) }}
        </div>
      </div>

      <div class="p-5">
        <div class="flex items-end justify-between gap-4">
          <div>
            <div class="text-sm font-semibold text-slate-900">
              {{ t("generate.progress.generating") }}
            </div>

            <div class="mt-1 text-xs text-slate-500">
              {{ t("generate.progress.filesProcessed", { completed: completedFiles, total: totalFiles }) }}
            </div>
          </div>

          <div class="text-2xl font-semibold text-slate-900">{{ progress }}%</div>
        </div>

        <div class="mt-4 h-2 overflow-hidden rounded-full bg-slate-100">
          <div
            class="h-full rounded-full bg-violet-500 transition-all duration-300"
            :style="{ width: `${progress}%` }"
          />
        </div>

        <div class="mt-5 grid grid-cols-2 gap-3">
          <div class="rounded-xl border border-violet-100 bg-violet-50 p-4">
            <div class="text-xs text-violet-700">{{ t("generate.progress.currentFile") }}</div>

            <div class="mt-1 truncate font-mono text-xs font-medium text-violet-800">
              {{ currentFile ?? t("generate.progress.preparing") }}
            </div>
          </div>

          <div class="rounded-xl border border-slate-200 bg-slate-50 p-4">
            <div class="text-xs text-slate-600">{{ t("generate.progress.elapsedTime") }}</div>

            <div class="mt-1 flex items-center gap-1.5 text-xl font-semibold text-slate-700">
              <Clock3 class="h-4 w-4" />
              {{ elapsedTime.toFixed(1) }}s
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Generated Files -->
    <section v-if="hasResult" class="rounded-2xl border border-slate-200 bg-white">
      <div class="border-b border-slate-200 px-5 py-4">
        <div class="flex items-center justify-between gap-4">
          <div>
            <div class="text-sm font-semibold text-slate-900">
              {{ t("generate.results.title") }}
            </div>

            <div class="mt-1 text-xs text-slate-500">
              {{ t("generate.results.desc") }}
            </div>
          </div>

          <span class="text-xs font-medium text-slate-500">
            {{ lastRunSummary }}
          </span>
        </div>
      </div>

      <div class="p-5">
        <!-- Conclusion -->
        <div
          class="rounded-2xl border p-5"
          :class="
            resultConclusion.type === 'success'
              ? 'border-emerald-200 bg-emerald-50'
              : resultConclusion.type === 'warning'
                ? 'border-amber-200 bg-amber-50'
                : 'border-rose-200 bg-rose-50'
          "
        >
          <div class="flex items-start gap-3">
            <CheckCircle2
              v-if="resultConclusion.type === 'success'"
              class="mt-0.5 h-5 w-5 shrink-0 text-emerald-600"
            />

            <AlertCircle
              v-else-if="resultConclusion.type === 'warning'"
              class="mt-0.5 h-5 w-5 shrink-0 text-amber-600"
            />

            <XCircle v-else class="mt-0.5 h-5 w-5 shrink-0 text-rose-600" />

            <div>
              <div
                class="text-sm font-semibold"
                :class="
                  resultConclusion.type === 'success'
                    ? 'text-emerald-800'
                    : resultConclusion.type === 'warning'
                      ? 'text-amber-800'
                      : 'text-rose-800'
                "
              >
                {{ resultConclusion.title }}
              </div>

              <div
                class="mt-1 text-xs"
                :class="
                  resultConclusion.type === 'success'
                    ? 'text-emerald-700'
                    : resultConclusion.type === 'warning'
                      ? 'text-amber-700'
                      : 'text-rose-700'
                "
              >
                {{ resultConclusion.description }}
              </div>
            </div>
          </div>
        </div>

        <!-- File list -->
        <div
          v-if="generatedFiles.length > 0"
          class="mt-5 divide-y divide-slate-100"
        >
          <div
            v-for="file in generatedFiles"
            :key="file.path"
            class="flex items-center gap-3 py-3"
          >
            <component
              :is="getGeneratedFileIcon(file.status)"
              class="h-4 w-4 shrink-0"
              :class="getGeneratedFileClass(file.status)"
            />

            <FileCode2 class="h-4 w-4 shrink-0 text-slate-400" />

            <div class="min-w-0 flex-1">
              <div class="truncate text-sm font-medium text-slate-800">
                {{ file.name }}
              </div>

              <div class="mt-0.5 truncate font-mono text-[11px] text-slate-500">
                {{ file.relativePath }}
                <template v-if="file.testCaseCount > 0">
                  · {{ t("generate.results.testCases", { count: file.testCaseCount }) }}
                </template>
              </div>
            </div>

            <div class="flex items-center gap-1">
              <button
                type="button"
                class="flex h-8 w-8 items-center justify-center rounded-lg text-slate-400 transition hover:bg-slate-100 hover:text-slate-700"
                :title="t('generate.actions.openFile')"
                @click="openFile(file.path)"
              >
                <ExternalLink class="h-3.5 w-3.5" />
              </button>

              <button
                type="button"
                class="flex h-8 w-8 items-center justify-center rounded-lg text-slate-400 transition hover:bg-slate-100 hover:text-slate-700"
                :title="t('generate.actions.revealInFolder')"
                @click="revealFile(file.path)"
              >
                <Folder class="h-3.5 w-3.5" />
              </button>

              <button
                type="button"
                class="flex h-8 w-8 items-center justify-center rounded-lg text-slate-400 transition hover:bg-slate-100 hover:text-slate-700"
                :title="t('generate.actions.copyPath')"
                @click="copyPath(file.path)"
              >
                <ClipboardCopy class="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>

        <div v-else class="py-8 text-center text-xs text-slate-500">
          {{ t("generate.results.empty") }}
        </div>
      </div>
    </section>

    <!-- Generation Log -->
    <section
      v-if="generationOutput.length > 0"
      class="rounded-2xl border border-slate-200 bg-white"
    >
      <button
        type="button"
        class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left"
        @click="showGenerationLog = !showGenerationLog"
      >
        <div class="flex min-w-0 items-center gap-3">
          <div
            class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-slate-100"
          >
            <Code2 class="h-4 w-4 text-slate-500" />
          </div>

          <div class="min-w-0">
            <div class="text-sm font-medium text-slate-800">
              {{ t("generate.progress.logTitle") }}
            </div>

            <div class="mt-1 text-xs text-slate-500">
              {{ t("generate.logLines", { count: generationOutput.length }) }}
            </div>
          </div>
        </div>

        <ChevronDown
          class="h-4 w-4 shrink-0 text-slate-400 transition-transform"
          :class="{ 'rotate-180': showGenerationLog }"
        />
      </button>

      <div v-if="showGenerationLog" class="border-t border-slate-200">
        <div
          class="flex items-center justify-end gap-2 border-b border-slate-800 bg-slate-950 px-4 py-2"
        >
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-slate-800 hover:text-white"
            @click="copyLogs"
          >
            <Copy class="h-3 w-3" />
            {{ t("common.copy") }}
          </button>

          <button
            type="button"
            class="rounded-lg px-2.5 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-slate-800 hover:text-white"
            @click="clearOutput"
          >
            {{ t("common.clear") }}
          </button>
        </div>

        <div class="bg-slate-950 p-4">
          <pre
            class="max-h-125 overflow-auto whitespace-pre-wrap wrap-break-word font-mono text-xs leading-5"
          ><span
              v-for="output in generationOutput"
              :key="output.logId"
              :class="output.stream === 'stderr' ? 'text-rose-300' : 'text-slate-300'"
            >{{ output.line }}
          {{ "\n" }}</span></pre>
        </div>
      </div>
    </section>
  </div>
</template>
