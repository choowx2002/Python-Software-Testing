<script setup lang="ts">
/**
 * Generate —— 测试生成页（彻底重构版）
 * 设计：单一主流程
 *   ① 主卡两栏：左 = 源文件树（搜索 + 全选），右 = 设置（3 项基础 + 高级折叠）
 *   ② 生成运行区：大按钮 + 进度 + 当前文件 + 终端日志
 *   ③ 完成：结论横幅 + 生成文件列表 + 行内操作
 * 所有业务逻辑（Pynguin 事件流 / 源扫描 / 历史持久化）保持不变。
 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRoute, useRouter } from "vue-router";
import {
  AlertCircle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  CircleDot,
  Copy,
  Folder,
  FolderOpen,
  Loader2,
  Play,
  Search,
  Settings2,
  Square,
  Timer,
  WandSparkles,
  X,
  XCircle,
} from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import TreeItem from "../../../components/TreeItem.vue";
import { useI18n } from "vue-i18n";
import AppButton from "../../../components/ui/AppButton.vue";
import AppConfirmModal from "../../../components/ui/AppConfirmModal.vue";
import AppTooltip from "../../../components/ui/AppTooltip.vue";
import AppContextMenu from "../../../components/ui/AppContextMenu.vue";
import AppHelpPopover from "../../../components/ui/AppHelpPopover.vue";
import StatusPill from "../../../components/ui/StatusPill.vue";
import EnvFixWizard from "../../../components/EnvFixWizard.vue";
import { useTaskbarProgress } from "../../../composables/useTaskbarProgress";
import { useWindowTitle } from "../../../composables/useWindowTitle";
import { notifyGenerationComplete } from "../../../composables/useNotifications";

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();
const { t } = useI18n();

const { setProgress: setTaskProgress, clear: clearTaskProgress } = useTaskbarProgress();
const { setStatus: setWinStatus, reset: resetWinStatus } = useWindowTitle();

/* 右键菜单：源文件树 */
const sourceMenu = ref<{ path: string; relativePath: string; x: number; y: number } | null>(null);
function showSourceMenu(node: { path: string; relativePath: string }, e: MouseEvent) {
  if (isGenerating.value) return;
  sourceMenu.value = { path: node.path, relativePath: node.relativePath, x: e.clientX, y: e.clientY };
}
const sourceMenuItems = computed(() => {
  if (!sourceMenu.value) return [];
  const { path, relativePath: rel } = sourceMenu.value;
  return [
    {
      label: t("contextmenu.generateForFile"),
      icon: WandSparkles,
      action: () => {
        if (rel && !selectedSourceFiles.value.includes(rel)) {
          selectedSourceFiles.value = [rel];
        }
        void generateTests();
      },
    },
    { label: t("contextmenu.openInFolder"), icon: FolderOpen, action: () => openFile(path) },
    { divider: true },
    { label: t("contextmenu.copyPath"), icon: Copy, action: () => copyPath(path) },
  ];
});

/* 右键菜单：生成结果文件 */
const resultMenu = ref<{ file: GeneratedFile; x: number; y: number } | null>(null);
function showResultMenu(file: GeneratedFile, e: MouseEvent) {
  resultMenu.value = { file, x: e.clientX, y: e.clientY };
}
const resultMenuItems = computed(() => {
  if (!resultMenu.value) return [];
  const f = resultMenu.value.file;
  if (!f.path) return [];
  return [
    { label: t("generate.actions.openFile"), icon: FolderOpen, action: () => openFile(f.path) },
    { label: t("generate.actions.revealInFolder"), icon: Folder, action: () => revealFile(f.path) },
    { divider: true },
    { label: t("contextmenu.copyPath"), icon: Copy, action: () => copyPath(f.path) },
    {
      label: t("contextmenu.runThisTest"),
      icon: Play,
      action: () => void router.push({ name: "ProjectExecute", params: { id: projectId.value } }),
    },
  ];
});

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

/* UTF-8 BOM 检测（Pynguin 无法解析带 BOM 的源码） */
const bomFiles = ref<string[]>([]);
const showBomModal = ref(false);

/* 生成环境健康检查（Python / pynguin / bytecode 版本） */
interface GenerationEnv {
  pythonVersion: string;
  pynguinVersion: string | null;
  bytecodeVersion: string | null;
}
const genEnv = ref<GenerationEnv | null>(null);
const envIssueDismissed = ref(false);

async function refreshGenEnv() {
  const interpreterPath = currentProject.value?.interpreter_path;
  if (!interpreterPath) return;

  try {
    genEnv.value = await invoke<GenerationEnv>("check_generation_env", {
      interpreterPath,
    });
  } catch (error) {
    console.error("[Generate] check_generation_env failed:", error);
  }
}

/** 环境问题清单（生成前提前提示，替代踩坑后才知道） */
const envIssues = computed(() => {
  const issues: { level: "warning" | "error"; text: string }[] = [];
  const env = genEnv.value;
  if (!env) return issues;

  // pynguin 未安装 → 无法生成（硬性）
  if (!env.pynguinVersion) {
    issues.push({ level: "error", text: t("generate.envNoPynguin") });
    return issues;
  }

  const parts = env.pythonVersion.split(".").map(Number);
  const isPy312Plus = parts[0] === 3 && parts[1] >= 12;

  if (isPy312Plus) {
    issues.push({ level: "warning", text: t("generate.envPy312Hint") });
    if (env.bytecodeVersion === "0.17.0") {
      issues.push({ level: "warning", text: t("generate.envBytecodeHint") });
    }
  }

  return issues;
});

/** 是否处于 Python 3.12+（显示一键换 3.11 的入口） */
const isPy312Risk = computed(() => {
  const env = genEnv.value;
  if (!env) return false;
  const parts = env.pythonVersion.split(".").map(Number);
  return parts[0] === 3 && parts[1] >= 12;
});

/** 一键修复完成后：刷新项目解释器信息 + 重新体检 */
async function onEnvFixed() {
  await projectStore.fetchProjects();
  void refreshGenEnv();
}

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

/** 页头 StatusPill 状态映射 */
const statusPillStatus = computed(() => {
  switch (generationStatus.value) {
    case "running":
      return "running";
    case "completed":
      return "completed";
    case "failed":
      return "Failed";
    default:
      return "idle";
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

/**
 * Pynguin 兼容性问题检测：
 * 0.43+ 在 Python 3.12 上存在插桩缺陷（上游 bug），表现为
 * "Failed to compute stacksize" / "Failed to load SUT" /
 * controlflow.py 的 AssertionError，即使生成了文件也是无效产物。
 */
const hasPynguinCompatIssue = computed(() => {
  const text = generationOutput.value.map((o) => o.line).join("\n");
  return (
    /Failed to compute stacksize/.test(text) ||
    /Failed to load SUT/.test(text) ||
    (/AssertionError/.test(text) && /instrumentation|controlflow/.test(text))
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

      setWinStatus(t("generate.status.generating"));
      void setTaskProgress(0, event.payload.totalFiles, "indeterminate");
    },
  );

  unlistenProgress = await listen<GenerationProgressEvent>(
    "generation-progress",
    (event) => {
      if (event.payload.runId !== currentRunId.value) return;

      currentFile.value = event.payload.currentFile;
      completedFiles.value = event.payload.completedFiles;
      elapsedTime.value = event.payload.elapsedTime;

      if (event.payload.totalFiles > 0) {
        void setTaskProgress(
          event.payload.completedFiles,
          event.payload.totalFiles,
          "normal",
        );
      }
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

      clearTaskProgress();
      resetWinStatus();

      // 完成后系统通知（长任务在后台时提醒用户）
      if (currentProject.value?.name) {
        const okCount = event.payload.generatedFiles.filter(
          (f) => f.status === "success",
        ).length;
        void notifyGenerationComplete(
          currentProject.value.name,
          event.payload.success,
          okCount,
        );
      }

      if (!event.payload.success && event.payload.generatedFiles.length === 0) {
        generationOutput.value.push({
          runId: event.payload.runId,
          stream: "stderr",
          line: t("generate.logs.systemWarning"),
          logId: logCounter++,
        });
        showGenerationLog.value = true;
      }

      // 持久化生成历史（NFR008：走 Rust 类型化命令）
      if (currentProject.value?.id) {
        const generatedCount = event.payload.generatedFiles.filter(
          (f) => f.status === "success",
        ).length;
        void invoke("save_generation_history", {
          projectId: currentProject.value.id,
          generationStatus: event.payload.success ? "success" : "failed",
          totalFiles: event.payload.generatedFiles.length,
          generatedFiles: generatedCount,
          duration: event.payload.duration,
          command: event.payload.command || null,
        }).catch((e) => console.error("[Generate] save history failed:", e));
      }

      // 生成结束后刷新环境状态（用户可能中途修复了依赖）
      void refreshGenEnv();
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

  // 源文件 BOM 检查：Pynguin 用 ast.parse(字符串) 解析源码，
  // 带 UTF-8 BOM 的文件会触发 "invalid non-printable character U+FEFF"
  try {
    const found = await invoke<string[]>("check_python_bom", {
      projectPath,
      files: selectedSourceFiles.value,
    });
    if (found.length > 0) {
      bomFiles.value = found;
      showBomModal.value = true;
      return; // 等待用户确认是否自动修复
    }
  } catch (error) {
    console.error("[Generate] check_python_bom failed:", error);
  }

  await startGeneration(projectPath, interpreterPath);
}

/** 用户确认修复 BOM 后：无损移除 → 继续生成 */
async function confirmFixBom() {
  showBomModal.value = false;

  const projectPath = currentProject.value?.path;
  const interpreterPath = currentProject.value?.interpreter_path;
  if (!projectPath || !interpreterPath) return;

  try {
    const fixed = await invoke<string[]>("strip_python_bom", {
      projectPath,
      files: selectedSourceFiles.value,
    });
    if (fixed.length > 0) {
      generationOutput.value.push({
        runId: "system",
        stream: "stdout",
        line: t("generate.bomFixed", { count: fixed.length }),
        logId: logCounter++,
      });
    }
  } catch (error) {
    console.error("[Generate] strip_python_bom failed:", error);
  }

  await startGeneration(projectPath, interpreterPath);
}

/** 实际启动 Pynguin 生成 */
async function startGeneration(projectPath: string, interpreterPath: string) {
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

function fileStatusLabel(status: GeneratedFile["status"]) {
  switch (status) {
    case "success":
      return t("generate.results.status.success");
    case "empty":
      return t("generate.results.status.empty");
    case "failed":
      return t("generate.results.status.failed");
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
    envIssueDismissed.value = false;

    await scanSourceFiles();
    void refreshGenEnv();
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
  <div class="flex min-h-full flex-col gap-4 p-5">
    <!-- 页头 -->
    <header class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <div
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-zinc-100 text-zinc-500"
          >
            <WandSparkles class="h-4 w-4" />
          </div>
          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-zinc-900">{{ t("generate.title") }}</h1>
            <p class="mt-0.5 truncate text-xs text-zinc-500">
              {{
                currentProject?.name
                  ? t("generate.subtitle", { name: currentProject.name })
                  : t("generate.subtitle", { name: t("layout.notLoaded") })
              }}
            </p>
          </div>
        </div>
      </div>
      <StatusPill :status="statusPillStatus" :label="statusText" :pulse="isGenerating" />
    </header>

    <!-- 生成环境提示（Pynguin / Python 兼容性问题，提前预警，可关闭） -->
    <div
      v-if="envIssues.length > 0 && !envIssueDismissed"
      class="rounded-lg border px-4 py-3"
      :class="
        envIssues.some((i) => i.level === 'error')
          ? 'border-rose-200 bg-rose-50'
          : 'border-amber-200 bg-amber-50'
      "
    >
      <div class="flex items-start gap-3">
        <AlertCircle
          class="mt-0.5 h-4 w-4 shrink-0"
          :class="
            envIssues.some((i) => i.level === 'error')
              ? 'text-rose-600'
              : 'text-amber-600'
          "
        />
        <div class="min-w-0 flex-1 space-y-1">
          <p
            v-for="(issue, i) in envIssues"
            :key="i"
            class="text-xs leading-5"
            :class="issue.level === 'error' ? 'text-rose-700' : 'text-amber-700'"
          >
            {{ issue.text }}
          </p>
          <p v-if="genEnv" class="font-mono text-[10px] text-zinc-400">
            Python {{ genEnv.pythonVersion }} · pynguin {{ genEnv.pynguinVersion ?? "—" }} ·
            bytecode {{ genEnv.bytecodeVersion ?? "—" }}
          </p>

          <!-- 一键换 Python 3.11（修复 Pynguin 兼容性问题） -->
          <div v-if="isPy312Risk && genEnv?.pynguinVersion && currentProject?.path" class="mt-2">
            <EnvFixWizard
              :project-path="currentProject.path"
              :project-id="currentProject.id"
              @fixed="onEnvFixed"
            />
          </div>
        </div>
        <button
          type="button"
          class="shrink-0 rounded p-1 text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-600"
          :aria-label="t('common.close')"
          @click="envIssueDismissed = true"
        >
          <X class="h-3.5 w-3.5" />
        </button>
      </div>
    </div>

    <!-- ══════════ 主卡：源文件（左） + 设置（右） ══════════ -->
    <section class="card overflow-hidden">
      <div class="grid min-h-0 grid-cols-1 lg:grid-cols-[1fr_360px]">
        <!-- 左：源文件树 -->
        <div class="min-h-0 border-b border-border lg:border-b-0 lg:border-r">
          <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
            <div class="min-w-0">
              <h2 class="text-sm font-semibold text-zinc-900">
                {{ t("generate.selectSource") }}
              </h2>
              <p class="mt-0.5 text-xs text-zinc-500">{{ t("generate.selectSourceDesc") }}</p>
            </div>
            <span
              class="shrink-0 rounded-full bg-zinc-100 px-2 py-0.5 font-mono text-[10px] text-zinc-500"
            >
              {{ t("generate.selectedCount", { count: selectedCount }) }}
            </span>
          </div>

          <div class="flex items-center gap-2 border-b border-border px-3 py-2">
            <div class="relative min-w-0 flex-1">
              <Search
                class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400"
              />
              <input
                v-model="sourceSearch"
                type="text"
                :placeholder="t('generate.searchPlaceholder')"
                :disabled="isGenerating"
                class="input h-8 pl-8 pr-3"
              />
            </div>
            <AppButton
              variant="ghost"
              size="sm"
              :disabled="isGenerating || filteredSourceFiles.length === 0 || allVisibleSelected"
              @click="selectAllVisible"
            >
              {{ t("common.selectAll") }}
            </AppButton>
            <AppButton
              variant="ghost"
              size="sm"
              :disabled="isGenerating || selectedCount === 0"
              @click="clearSelection"
            >
              {{ t("common.clear") }}
            </AppButton>
          </div>

          <div class="max-h-80 overflow-auto">
            <div v-if="isLoadingSources" class="px-4 py-10 text-center text-sm text-zinc-500">
              <Loader2 class="mx-auto mb-2 h-5 w-5 animate-spin" />
              {{ t("generate.scanningSources") }}
            </div>
            <div
              v-else-if="sourceScanError"
              class="m-3 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2.5 text-xs text-rose-700"
            >
              {{ sourceScanError }}
            </div>
            <div v-else-if="sourceTree.length === 0" class="px-4 py-10 text-center">
              <div class="text-sm font-medium text-zinc-700">{{ t("generate.noSources") }}</div>
              <div class="mt-1 text-xs text-zinc-500">{{ t("generate.noSourcesDesc") }}</div>
            </div>
            <div v-else class="py-1">
              <TreeItem
                v-for="node in sourceTree"
                :key="node.id"
                :node="node"
                :expanded-dirs="expandedDirs"
                :selected-files="selectedSourceFiles"
                :disabled="isGenerating"
                @toggle-dir="toggleDir"
                @toggle-file="toggleFile"
                @context-menu="showSourceMenu($event.node, $event.ev)"
              />
            </div>
          </div>
        </div>

        <!-- 右：设置 -->
        <div class="min-h-0 p-4">
          <div class="flex items-center gap-2">
            <Settings2 class="h-4 w-4 text-zinc-400" />
            <h2 class="text-sm font-semibold text-zinc-900">{{ t("generate.configTitle") }}</h2>
          </div>
          <p class="mt-1 text-xs text-zinc-500">{{ t("generate.configDesc") }}</p>

          <div class="mt-4 space-y-4">
            <!-- 每个文件的用时 -->
            <div>
              <div class="mb-2 flex items-center justify-between">
                <label class="text-xs font-medium text-zinc-700" for="max-search-time">
                  {{ t("generate.timeLimit") }}
                </label>
                <span class="flex items-center gap-1 text-xs text-zinc-500">
                  <Timer class="h-3 w-3" />
                  {{ t("generate.perFile") }}
                </span>
              </div>
              <div class="flex items-center gap-2">
                <input
                  id="max-search-time"
                  v-model.number="maxSearchTime"
                  type="number"
                  min="1"
                  max="3600"
                  :disabled="isGenerating"
                  class="input"
                />
                <span class="shrink-0 text-xs text-zinc-500">{{ t("generate.timeLimitUnit") }}</span>
              </div>
              <p class="mt-1.5 text-[11px] text-zinc-400">
                {{ t("generate.estimatedTotal", { seconds: estimatedTimeout }) }}
              </p>
            </div>

            <!-- 策略 -->
            <div>
              <div class="mb-2 flex items-center gap-1.5">
                <label class="block text-xs font-medium text-zinc-700">
                  {{ t("generate.algorithm") }}
                </label>
                <AppHelpPopover :title="t('generate.algorithm')" :content="t('help.algorithm')" />
              </div>
              <div class="grid grid-cols-2 gap-2">
                <button
                  v-for="alg in (['MOSA', 'DYNAMOSA', 'WSPA', 'RANDOM'] as Algorithm[])"
                  :key="alg"
                  type="button"
                  :disabled="isGenerating"
                  class="flex items-center gap-2 rounded-lg border px-3 py-2 text-left text-xs transition disabled:cursor-not-allowed"
                  :class="
                    algorithm === alg
                      ? 'border-brand-200 bg-brand-50 text-brand-700'
                      : 'border-border text-zinc-700 hover:bg-zinc-50'
                  "
                  @click="algorithm = alg"
                >
                  <CircleDot v-if="algorithm === alg" class="h-3.5 w-3.5 text-brand-600" />
                  <Circle v-else class="h-3.5 w-3.5 text-zinc-300" />
                  <span class="font-medium">{{ alg }}</span>
                </button>
              </div>
              <p class="mt-1.5 text-[11px] text-zinc-400">{{ t("generate.algorithmHint") }}</p>
            </div>

            <!-- 生成检查 -->
            <div class="flex items-center justify-between gap-3 rounded-lg border border-border px-3.5 py-3">
              <div class="min-w-0">
                <div class="text-[13px] font-medium text-zinc-900">{{ t("generate.assertGen") }}</div>
                <div class="mt-0.5 text-[11px] text-zinc-500">{{ t("generate.assertGenDesc") }}</div>
              </div>
              <button
                type="button"
                :disabled="isGenerating"
                class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition disabled:cursor-not-allowed"
                :class="assertionGeneration ? 'bg-brand-500' : 'bg-zinc-200'"
                @click="assertionGeneration = !assertionGeneration"
              >
                <span
                  class="inline-block h-4 w-4 transform rounded-full bg-white transition"
                  :class="assertionGeneration ? 'translate-x-6' : 'translate-x-1'"
                />
              </button>
            </div>

            <!-- 高级选项 -->
            <div class="border-t border-border pt-3">
              <button
                type="button"
                class="flex items-center gap-2 text-xs font-medium text-zinc-600 transition hover:text-zinc-900"
                @click="showAdvanced = !showAdvanced"
              >
                <ChevronDown v-if="showAdvanced" class="h-4 w-4" />
                <ChevronRight v-else class="h-4 w-4" />
                {{ t("execute.advancedOptions") }}
              </button>

              <div v-if="showAdvanced" class="mt-3 space-y-3">
                <div>
                  <label class="mb-2 block text-xs font-medium text-zinc-700">
                    {{ t("generate.seed") }}
                    <span class="text-zinc-400">({{ t("common.optional") }})</span>
                  </label>
                  <input
                    v-model.number="seed"
                    type="number"
                    placeholder="None"
                    :disabled="isGenerating"
                    class="input"
                  />
                  <p class="mt-1 text-[11px] text-zinc-400">{{ t("generate.seedDesc") }}</p>
                </div>

                <div>
                  <div class="mb-2 flex items-center gap-1.5">
                    <label class="block text-xs font-medium text-zinc-700">
                      {{ t("generate.chromosomeLength") }}
                    </label>
                    <AppHelpPopover :title="t('generate.chromosomeLength')" :content="t('help.chromosomeLength')" />
                  </div>
                  <input
                    v-model.number="chromosomeLength"
                    type="number"
                    min="1"
                    :disabled="isGenerating"
                    class="input"
                  />
                  <p class="mt-1 text-[11px] text-zinc-400">
                    {{ t("generate.chromosomeLengthDesc") }}
                  </p>
                </div>

                <div>
                  <div class="mb-2 flex items-center gap-1.5">
                    <label class="block text-xs font-medium text-zinc-700">
                      {{ t("generate.populationSize") }}
                    </label>
                    <AppHelpPopover :title="t('generate.populationSize')" :content="t('help.populationSize')" />
                  </div>
                  <input
                    v-model.number="populationSize"
                    type="number"
                    min="1"
                    :disabled="isGenerating"
                    class="input"
                  />
                  <p class="mt-1 text-[11px] text-zinc-400">{{ t("generate.populationSizeDesc") }}</p>
                </div>

                <div>
                  <label class="mb-2 block text-xs font-medium text-zinc-700">
                    {{ t("generate.maxTestCases") }}
                  </label>
                  <input
                    v-model.number="maxTestCases"
                    type="number"
                    min="1"
                    max="500"
                    :disabled="isGenerating"
                    class="input"
                  />
                  <p class="mt-1 text-[11px] text-zinc-400">{{ t("generate.maxTestCasesDesc") }}</p>
                </div>

                <div>
                  <label class="mb-2 block text-xs font-medium text-zinc-700">
                    {{ t("generate.outputFolder") }}
                  </label>
                  <input
                    v-model="outputFolder"
                    type="text"
                    placeholder="tests/generated"
                    :disabled="isGenerating"
                    class="input font-mono"
                  />
                  <p class="mt-1 text-[11px] text-zinc-400">{{ t("generate.outputFolderDesc") }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- ══════════ 生成运行区 ══════════ -->
    <section class="card">
      <div class="flex flex-wrap items-center justify-between gap-3 px-5 py-4">
        <div class="min-w-0">
          <h2 class="text-sm font-semibold text-zinc-900">{{ t("generate.generateTests") }}</h2>
          <p class="mt-0.5 text-xs text-zinc-500">
            {{
              canGenerate
                ? t("generate.generateSelectedDesc", {
                    count: selectedCount,
                    seconds: estimatedTimeout,
                  })
                : t("generate.generateDesc")
            }}
          </p>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <AppButton v-if="isGenerating" variant="danger" @click="stopGeneration">
            <Square class="h-4 w-4" />
            {{ t("generate.stop") }}
          </AppButton>
          <AppButton v-else variant="primary" :disabled="!canGenerate" @click="generateTests">
            <WandSparkles class="h-4 w-4" />
            {{ t("generate.generateTests") }}
          </AppButton>
        </div>
      </div>

      <!-- 进度（运行中） -->
      <div v-if="isGenerating" class="border-t border-border px-5 py-4">
        <div class="flex items-end justify-between gap-4">
          <div class="min-w-0">
            <div class="text-sm font-semibold text-zinc-900">
              {{ t("generate.progress.generating") }}
            </div>
            <div class="mt-0.5 text-xs text-zinc-500">
              {{ t("generate.progress.filesProcessed", { completed: completedFiles, total: totalFiles }) }}
            </div>
          </div>
          <div class="text-2xl font-semibold text-zinc-900">{{ progress }}%</div>
        </div>
        <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-100">
          <div
            class="h-full rounded-full bg-brand-500 transition-all duration-300"
            :style="{ width: `${progress}%` }"
          />
        </div>
        <div
          v-if="currentFile"
          class="mt-3 flex items-center gap-2 rounded-lg border border-brand-100 bg-brand-50 px-3 py-2"
        >
          <Loader2 class="h-3.5 w-3.5 shrink-0 animate-spin text-brand-600" />
          <span class="truncate font-mono text-[11px] text-brand-800">{{ currentFile }}</span>
        </div>
      </div>

      <!-- 终端日志（运行中自动显示） -->
      <div v-if="isGenerating || generationOutput.length > 0" class="border-t border-border">
        <div class="flex items-center justify-between gap-3 bg-zinc-950 px-4 py-2">
          <span class="truncate font-mono text-[10px] text-zinc-400">
            {{ t("generate.logLines", { count: generationOutput.length }) }}
          </span>
          <div class="flex shrink-0 items-center gap-1">
            <button
              type="button"
              class="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium text-zinc-300 transition hover:bg-zinc-800 hover:text-white"
              @click="copyLogs"
            >
              <Copy class="h-3 w-3" />
              {{ t("common.copy") }}
            </button>
            <button
              type="button"
              class="rounded-md px-2 py-1 text-[11px] font-medium text-zinc-300 transition hover:bg-zinc-800 hover:text-white"
              @click="clearOutput"
            >
              {{ t("common.clear") }}
            </button>
          </div>
        </div>
        <pre class="max-h-56 overflow-auto bg-zinc-950 px-4 py-3 font-mono text-[11px] leading-5">
          <template v-for="output in generationOutput" :key="output.logId">
            <span :class="output.stream === 'stderr' ? 'text-rose-300' : 'text-zinc-300'">{{
              output.line
            }}</span>{{ "\n" }}
          </template>
        </pre>
      </div>

      <!-- 结果（完成） -->
      <div v-if="hasResult" class="border-t border-border px-5 py-4">
        <!-- Pynguin / Python 3.12 兼容性问题警告 -->
        <div
          v-if="hasPynguinCompatIssue"
          class="mb-4 rounded-lg border border-rose-200 bg-rose-50 px-4 py-3"
        >
          <div class="flex items-start gap-3">
            <XCircle class="mt-0.5 h-4 w-4 shrink-0 text-rose-600" />
            <div class="min-w-0">
              <div class="text-[13px] font-semibold text-rose-800">
                {{ t("generate.pynguinCompatTitle") }}
              </div>
              <p class="mt-1 text-xs leading-5 text-rose-700">
                {{ t("generate.pynguinCompatDesc") }}
              </p>
            </div>
          </div>
        </div>
        <!-- 结论横幅 -->
        <div
          class="flex flex-wrap items-center justify-between gap-3 rounded-lg border px-4 py-3.5"
          :class="
            resultConclusion.type === 'success'
              ? 'border-emerald-200 bg-emerald-50'
              : resultConclusion.type === 'warning'
                ? 'border-amber-200 bg-amber-50'
                : 'border-rose-200 bg-rose-50'
          "
        >
          <div class="flex min-w-0 items-start gap-3">
            <CheckCircle2
              v-if="resultConclusion.type === 'success'"
              class="mt-0.5 h-5 w-5 shrink-0 text-emerald-600"
            />
            <AlertCircle
              v-else-if="resultConclusion.type === 'warning'"
              class="mt-0.5 h-5 w-5 shrink-0 text-amber-600"
            />
            <XCircle v-else class="mt-0.5 h-5 w-5 shrink-0 text-rose-600" />
            <div class="min-w-0">
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
                class="mt-0.5 text-xs"
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
          <span class="shrink-0 font-mono text-[11px] text-zinc-500">{{ lastRunSummary }}</span>
        </div>

        <!-- 生成文件列表 -->
        <div v-if="generatedFiles.length > 0" class="mt-4 divide-y divide-border rounded-lg border border-border">
          <div
            v-for="file in generatedFiles"
            :key="file.path"
            class="flex flex-wrap items-center gap-3 px-4 py-2.5 transition-colors hover:bg-zinc-50"
            @contextmenu.prevent="showResultMenu(file, $event)"
          >
            <component
              :is="getGeneratedFileIcon(file.status)"
              class="h-4 w-4 shrink-0"
              :class="getGeneratedFileClass(file.status)"
            />
            <div class="min-w-0 flex-1">
              <div class="truncate text-[13px] font-medium text-zinc-800">{{ file.name }}</div>
              <div class="mt-0.5 truncate font-mono text-[10px] text-zinc-500">
                {{ file.relativePath }}
              </div>
            </div>
            <span class="shrink-0 rounded-full bg-zinc-100 px-2 py-0.5 text-[10px] font-medium" :class="getGeneratedFileClass(file.status)">
              {{ fileStatusLabel(file.status) }}
            </span>
            <span class="shrink-0 font-mono text-[11px] text-zinc-500">
              {{ t("generate.results.testCases", { count: file.testCaseCount }) }}
            </span>
            <div class="flex shrink-0 items-center gap-1">
              <AppTooltip :content="t('generate.actions.openFile')" position="top">
                <button
                  type="button"
                  class="rounded-lg p-1.5 text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700"
                  @click="openFile(file.path)"
                >
                  <FolderOpen class="h-3.5 w-3.5" />
                </button>
              </AppTooltip>
              <AppTooltip :content="t('generate.actions.revealInFolder')" position="top">
                <button
                  type="button"
                  class="rounded-lg p-1.5 text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700"
                  @click="revealFile(file.path)"
                >
                  <Folder class="h-3.5 w-3.5" />
                </button>
              </AppTooltip>
              <AppTooltip :content="t('generate.actions.copyPath')" position="top">
                <button
                  type="button"
                  class="rounded-lg p-1.5 text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700"
                  @click="copyPath(file.path)"
                >
                  <Copy class="h-3.5 w-3.5" />
                </button>
              </AppTooltip>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- UTF-8 BOM 确认弹窗：无损移除后继续生成 -->
    <AppConfirmModal
      :open="showBomModal"
      :title="t('generate.bomTitle')"
      :description="t('generate.bomDesc', { count: bomFiles.length })"
      variant="primary"
      :confirm-label="t('generate.bomFix')"
      @confirm="confirmFixBom"
      @update:open="(open: boolean) => { if (!open) showBomModal = false }"
    >
      <ul
        class="max-h-40 overflow-auto rounded-lg bg-zinc-50 p-3 font-mono text-[11px] leading-5 text-zinc-600"
      >
        <li v-for="f in bomFiles" :key="f">{{ f }}</li>
      </ul>
    </AppConfirmModal>

    <!-- 源文件树右键菜单 -->
    <AppContextMenu
      v-if="sourceMenu"
      :items="sourceMenuItems"
      :open="!!sourceMenu"
      @close="sourceMenu = null"
    />

    <!-- 生成结果右键菜单 -->
    <AppContextMenu
      v-if="resultMenu"
      :items="resultMenuItems"
      :open="!!resultMenu"
      @close="resultMenu = null"
    />
  </div>
</template>
