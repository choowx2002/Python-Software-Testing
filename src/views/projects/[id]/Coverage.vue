<script setup lang="ts">
/**
 * Coverage —— 覆盖率分析页（彻底重构版）
 * 设计：单一主流程
 *   ① 范围卡：源目录树（左）+ 测试文件（右）+ coverage.py 状态 + 运行按钮
 *   ② 结果卡：摘要条（总覆盖率/代码行/分支/文件）+ 文件列表 + 内嵌源码查看器
 *   ③ 导出与日志收进结果卡底部
 * 所有业务逻辑（coverage 事件流 / 扫描 / 详情缓存 / 导出 / 历史持久化）保持不变。
 */
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
  Copy,
  Download,
  FileCode2,
  FolderOpen,
  Gauge,
  Loader2,
  Play,
  RefreshCw,
  Search,
  WandSparkles,
} from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import TreeItem from "../../../components/TreeItem.vue";
import { interpretError, type InterpretedError } from "../../../utils/errors";
import { useI18n } from "vue-i18n";
import AppButton from "../../../components/ui/AppButton.vue";
import AppContextMenu from "../../../components/ui/AppContextMenu.vue";
import AppHelpPopover from "../../../components/ui/AppHelpPopover.vue";
import StatusPill from "../../../components/ui/StatusPill.vue";
import { useTaskbarProgress } from "../../../composables/useTaskbarProgress";
import { useWindowTitle } from "../../../composables/useWindowTitle";
import { notifyCoverageComplete } from "../../../composables/useNotifications";

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();
const { t } = useI18n();

const { setProgress: setTaskProgress, clear: clearTaskProgress } = useTaskbarProgress();
const { setStatus: setWinStatus, reset: resetWinStatus } = useWindowTitle();

/* 右键菜单：文件列表 */
const fileMenu = ref<{ file: FileCoverage; x: number; y: number } | null>(null);
function showFileMenu(file: FileCoverage, e: MouseEvent) {
  fileMenu.value = { file, x: e.clientX, y: e.clientY };
}
const fileMenuItems = computed(() => {
  if (!fileMenu.value) return [];
  const f = fileMenu.value.file;
  return [
    {
      label: t("coverage.openInFileManager"),
      icon: FolderOpen,
      action: () => {
        const p = currentProject.value?.path;
        if (p) void invoke("reveal_in_folder", { path: `${p}/${f.path}` });
      },
    },
    { divider: true },
    { label: t("contextmenu.copyPath"), icon: Copy, action: () => copyText(f.path) },
    { label: t("contextmenu.exportFile"), icon: Download, action: () => toggleDetail(f.path) },
  ];
});

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    console.error("[Coverage] copy failed:", error);
  }
}

const projectId = computed(() => Number(route.params.id));

type CoverageStatus = "idle" | "running" | "completed" | "failed";
type SortOrder = "asc" | "desc";
type CoverageFilter = "all" | "high" | "mid" | "low";

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
  totalBranches: number | null;
  coveredBranches: number | null;
  branchPercent: number | null;
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
const scanError = ref<InterpretedError | null>(null);

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

/** FR013 函数级过滤：当前选中的函数名，"__all__" 表示显示全部 */
const functionFilter = ref<string>("__all__");

interface FunctionRange {
  name: string;
  startLine: number;
  endLine: number;
  covered: number;
  missing: number;
}

/**
 * 从源码行中提取函数/方法区间（FR013）。
 * - 顶层 def / async def，类内方法记为 "ClassName.method"；离开类后前缀重置
 * - 装饰器行（@xxx，缩进与 def 相同）归属其后的函数
 * - 函数结束于第一条缩进 <= 当前 def 的非空行（兄弟函数/装饰器/模块代码）
 * - 函数体必然比 def 缩进更深，因此"同级即结束"无需特判边界类型
 */
const functionStats = computed<FunctionRange[]>(() => {
  const lines = detail.value?.lines ?? [];
  if (lines.length === 0) return [];

  const ranges: FunctionRange[] = [];
  let currentClass = "";
  let classIndent = -1;

  for (let i = 0; i < lines.length; i++) {
    const { lineNumber, source } = lines[i];
    const trimmed = source.trim();
    if (!trimmed) continue;

    const classMatch = trimmed.match(/^class\s+(\w+)/);
    if (classMatch) {
      currentClass = classMatch[1];
      classIndent = source.length - source.trimStart().length;
      continue;
    }

    const defMatch = trimmed.match(/^(?:async\s+)?def\s+(\w+)/);
    if (!defMatch) continue;

    const defIndent = source.length - source.trimStart().length;
    // 顶层 def（缩进 <= 类缩进）说明已离开类作用域，重置类前缀
    if (currentClass && defIndent <= classIndent) {
      currentClass = "";
    }

    // 结束行：第一条缩进 <= 当前 def 的非空行（装饰器行同样结束上一函数）
    let end = lineNumber;
    for (let j = i + 1; j < lines.length; j++) {
      const next = lines[j].source;
      const nextTrimmed = next.trim();
      if (!nextTrimmed) continue;
      if (next.length - next.trimStart().length <= defIndent) break;
      end = lines[j].lineNumber;
    }

    // 起始行：向上回溯同缩进的装饰器行
    let start = lineNumber;
    for (let k = i - 1; k >= 0; k--) {
      const prev = lines[k];
      const prevTrimmed = prev.source.trim();
      const prevIndent = prev.source.length - prev.source.trimStart().length;
      if (prevTrimmed.startsWith("@") && prevIndent === defIndent) {
        start = prev.lineNumber;
      } else {
        break;
      }
    }

    let covered = 0;
    let missing = 0;
    for (const line of lines) {
      if (line.lineNumber < start || line.lineNumber > end) continue;
      if (line.status === "covered") covered++;
      else if (line.status === "missing") missing++;
    }

    const name = currentClass ? `${currentClass}.${defMatch[1]}` : defMatch[1];
    ranges.push({ name, startLine: start, endLine: end, covered, missing });
  }

  return ranges;
});

/** 当前选中的函数（用于显示该函数的覆盖率统计） */
const activeFunction = computed<FunctionRange | null>(
  () => functionStats.value.find((f) => f.name === functionFilter.value) ?? null,
);

/** 按函数过滤后的展示行 */
const filteredDetailLines = computed(() => {
  const lines = detail.value?.lines ?? [];
  if (functionFilter.value === "__all__") return lines;
  const fn = activeFunction.value;
  if (!fn) return lines;
  return lines.filter(
    (l) => l.lineNumber >= fn.startLine && l.lineNumber <= fn.endLine,
  );
});

const coverageOutput = ref<CoverageOutputEvent[]>([]);
const showCoverageLog = ref(false);
const logContainer = ref<HTMLElement | null>(null);

const errorMessage = ref<InterpretedError | null>(null);

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

/** 当前可见源文件是否已全部选中（用于全选按钮禁用态） */
const allVisibleSelected = computed(() => {
  if (filteredSourceFiles.value.length === 0) return false;
  return filteredSourceFiles.value.every((f) =>
    selectedSourceFiles.value.includes(f.relativePath),
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

/** 由选中的源文件推导 coverage --source 目标（目录语义）：
 *  - 子目录文件 → 所属目录
 *  - 根目录文件 → "."（项目根）
 *  目录语义保证：未导入的文件也会出现在报告中（0% 红条），
 *  展示层再按 isSelectedTarget 过滤，实现"只显示选中文件"。
 */
function deriveSourceDirs(): string[] {
  const specs = new Set<string>();

  for (const rel of selectedSourceFiles.value) {
    const idx = rel.lastIndexOf("/");

    if (idx === -1) {
      specs.add(".");
    } else {
      specs.add(rel.slice(0, idx));
    }
  }

  return [...specs].sort();
}

/** 当前选择是否构成过滤视图（只展示选中目标的文件） */
const hasTargetFilter = computed(() => selectedSourceFiles.value.length > 0);

/** 文件路径是否属于当前选中的测量目标 */
function isSelectedTarget(path: string) {
  return selectedSourceFiles.value.some((rel) => {
    const idx = rel.lastIndexOf("/");
    if (idx === -1) return path === rel; // 根目录文件：精确匹配
    return path === rel || path.startsWith(rel.slice(0, idx) + "/"); // 目录：前缀匹配
  });
}

/** 结果视图中展示的文件（按当前选择过滤；未选择则全部） */
const visibleFiles = computed(() => {
  const files = summary.value?.files ?? [];
  if (!hasTargetFilter.value) return files;
  return files.filter((f) => isSelectedTarget(f.path));
});

/** 过滤视图下的语句级统计（重算，保证与展示文件一致） */
const visibleStats = computed(() => {
  let total = 0;
  let covered = 0;
  for (const f of visibleFiles.value) {
    total += f.executedLines.length + f.missingLines.length + f.excludedLines.length;
    covered += f.executedLines.length;
  }
  return {
    total,
    covered,
    percent: total > 0 ? (covered / total) * 100 : 0,
  };
});

/** 展示用总覆盖率（过滤视图重算；否则用运行级数值） */
const displayPercent = computed(() =>
  hasTargetFilter.value
    ? visibleStats.value.percent
    : summary.value?.percentCovered ?? 0,
);

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
      return t("coverage.status.running");
    case "completed":
      return t("coverage.status.completed");
    case "failed":
      return t("coverage.status.failed");
    default:
      return t("coverage.status.ready");
  }
});

/** 页头 StatusPill 状态映射 */
const statusPillStatus = computed(() => {
  switch (coverageStatus.value) {
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

const hasResult = computed(() => summary.value !== null);

/** 文件列表：按选择过滤 + 状态筛选 + 排序 */
const displayFiles = computed(() => {
  let list = visibleFiles.value;

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
  const files = visibleFiles.value;
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

/** 纯语句覆盖率（过滤视图下用选中文件重算；否则用运行级语句计数） */
const statementPercent = computed(() => {
  if (hasTargetFilter.value) return visibleStats.value.percent;
  const s = summary.value;
  if (!s || s.totalStatements === 0) return 0;
  return (s.coveredStatements / s.totalStatements) * 100;
});

/** 文件列表筛选标签 */
const coverageFilters = computed(() => [
  {
    id: "all" as CoverageFilter,
    label: t("coverage.fileList.filter.all", { count: fileStats.value.total }),
  },
  {
    id: "high" as CoverageFilter,
    label: t("coverage.fileList.filter.high", { count: fileStats.value.high }),
  },
  {
    id: "mid" as CoverageFilter,
    label: t("coverage.fileList.filter.mid", { count: fileStats.value.mid }),
  },
  {
    id: "low" as CoverageFilter,
    label: t("coverage.fileList.filter.low", { count: fileStats.value.low }),
  },
]);

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
      return "text-zinc-400";
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

      setWinStatus(t("coverage.status.running"));
      void setTaskProgress(0, 1, "indeterminate");
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

      clearTaskProgress();
      resetWinStatus();

      // 完成后系统通知（长任务在后台时提醒用户）
      if (currentProject.value?.name && event.payload.summary) {
        void notifyCoverageComplete(
          currentProject.value.name,
          event.payload.summary.percentCovered,
        );
      }

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
            ? t("coverage.logs.warningWithSummary")
            : t("coverage.logs.systemWarning"),
          logId: logCounter++,
        });

        if (!event.payload.summary) {
          showCoverageLog.value = true;
        }
      }

      // 持久化覆盖率运行历史（NFR008：走 Rust 类型化命令）
      if (currentProject.value?.id) {
        void invoke("save_coverage_history", {
          projectId: currentProject.value.id,
          coverageStatus: event.payload.summary
            ? event.payload.success
              ? "success"
              : "warning"
            : "failed",
          percentCovered: event.payload.summary?.percentCovered ?? 0,
          totalStatements: event.payload.summary?.totalStatements ?? 0,
          coveredStatements: event.payload.summary?.coveredStatements ?? 0,
          duration: event.payload.duration,
          command: event.payload.command || null,
          filesJson: event.payload.summary
            ? JSON.stringify(event.payload.summary.files)
            : null,
        }).catch((e) => console.error("[Coverage] save history failed:", e));
      }
    },
  );

  unlistenError = await listen<CoverageErrorEvent>(
    "coverage-error",
    (event) => {
      errorMessage.value = interpretError(event.payload.message);

      coverageOutput.value.push({
        runId: event.payload.runId,
        stream: "stderr",
        line: t("coverage.logs.coverageError", { msg: event.payload.message }),
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
    scanError.value = interpretError(error);
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
  installMessage.value = t("coverage.installMessage.installing");

  try {
    const result = await invoke<{
      success: boolean;
      installed: string[];
      failed: string[];
      failedReasons?: string[];
    }>("install_dependencies", {
      pythonPath: interpreterPath,
      packages: ["coverage"],
    });

    if (result.success) {
      installMessage.value = t("coverage.installMessage.success");
      await checkCoverageInstalled();
    } else {
      installMessage.value = t("coverage.installMessage.failed", {
        packages: result.failed.join(", "),
      });
      // 追加 pip 真实报错摘要（截断，避免过长）
      const reason = result.failedReasons?.[0] ?? "";
      if (reason) {
        installMessage.value += ` · ${reason.slice(0, 200)}`;
      }
    }
  } catch (error) {
    console.error("[Coverage] install_dependencies failed:", error);
    installMessage.value = t("coverage.installMessage.failedDetail", {
      msg: error instanceof Error ? error.message : String(error),
    });
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

    const rawMsg = error instanceof Error ? error.message : String(error);

    // 统一转为带解释的友好提示（含 "No data to report" 等场景）
    errorMessage.value = interpretError(rawMsg);

    coverageOutput.value.push({
      runId: currentRunId.value ?? "local",
      stream: "stderr",
      line: t("coverage.logs.fatalStart", { msg: rawMsg }),
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
 * NFR008：走 Rust 类型化命令
 */
async function saveCoverageToDb(payload: CoverageFinishedEvent) {
  if (!currentProject.value?.id) return;
  if (!payload.summary) return;

  try {
    const runSummary = payload.summary;
    const coveredFileCount = runSummary.files.filter(
      (f) => f.percentCovered > 0,
    ).length;

    await invoke("save_coverage_result", {
      projectId: currentProject.value.id,
      executionId: null, // 当前覆盖率运行未关联 test_execution_history
      totalStatementCoverage: runSummary.percentCovered,
      totalBranchCoverage: runSummary.branchPercent ?? null, // --branch 模式：分支覆盖率
      fileCount: runSummary.files.length,
      coveredFileCount,
      detailJsonPath: runSummary.jsonPath,
    });
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
  functionFilter.value = "__all__"; // 切换文件时重置函数过滤（FR013）

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
    exportMessage.value = t("coverage.export.saved", { path: savedPath });
  } catch (error) {
    console.error("[Coverage] export_coverage_report failed:", error);
    exportMessage.value = t("coverage.export.failed", {
      msg: error instanceof Error ? error.message : String(error),
    });
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

async function openSourceFile(file: FileCoverage) {
  const projectPath = currentProject.value?.path;
  if (!projectPath || !file.path) return;
  const full = file.path.startsWith("/") || /^[A-Za-z]:/.test(file.path)
    ? file.path
    : `${projectPath}/${file.path}`;
  try {
    await invoke("open_file", { path: full });
  } catch (error) {
    console.error("[Coverage] open_file failed:", error);
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
  window.addEventListener('testmate:focus-search', onFocusSearch);
});

onUnmounted(() => {
  unlistenStarted?.();
  unlistenOutput?.();
  unlistenFinished?.();
  unlistenError?.();
  window.removeEventListener('testmate:focus-search', onFocusSearch);
});

const sourceSearchInput = ref<HTMLInputElement | null>(null);
function onFocusSearch() {
  sourceSearchInput.value?.focus();
}
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
            <Gauge class="h-4 w-4" />
          </div>
          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-zinc-900">{{ t("coverage.title") }}</h1>
            <p class="mt-0.5 truncate text-xs text-zinc-500">
              {{
                currentProject?.name
                  ? t("coverage.subtitle", { name: currentProject.name })
                  : t("coverage.subtitle", { name: t("layout.notLoaded") })
              }}
            </p>
          </div>
        </div>
      </div>
      <StatusPill :status="statusPillStatus" :label="statusText" :pulse="isRunning" />
    </header>

    <!-- ══════════ 范围 + 运行 ══════════ -->
    <section class="card">
      <div class="px-5 py-4">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 class="text-sm font-semibold text-zinc-900">{{ t("coverage.scopeTitle") }}</h2>
            <p class="mt-0.5 text-xs text-zinc-500">{{ t("coverage.scopeDesc") }}</p>
          </div>
          <AppButton
            variant="secondary"
            size="sm"
            :loading="isLoadingSources || isLoadingTests"
            :disabled="isRunning"
            @click="refreshScans"
          >
            <RefreshCw class="h-3.5 w-3.5" />
            {{ t("common.refresh") }}
          </AppButton>
        </div>

        <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
          <!-- 源目录 -->
          <div class="rounded-lg border border-border">
            <div class="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
              <span class="text-xs font-medium text-zinc-700">{{ t("coverage.targetSource") }}</span>
              <span class="shrink-0 font-mono text-[10px] text-zinc-400">
                {{ t("coverage.targetsSelected", { count: selectedSourceCount }) }}
              </span>
            </div>

            <div class="flex items-center gap-2 border-b border-border px-3 py-2">
              <div class="relative min-w-0 flex-1">
                <Search
                  class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400"
                />
                <input
                  v-model="sourceSearch"
                  ref="sourceSearchInput"
                  type="text"
                  :placeholder="t('coverage.searchPlaceholder')"
                  :disabled="isRunning"
                  class="input h-8 pl-8 pr-3"
                />
              </div>
              <AppButton
                variant="ghost"
                size="sm"
                :disabled="isRunning || filteredSourceFiles.length === 0 || allVisibleSelected"
                @click="selectAllVisible"
              >
                {{ t("common.selectAll") }}
              </AppButton>
              <AppButton
                variant="ghost"
                size="sm"
                :disabled="isRunning || selectedSourceCount === 0"
                @click="clearSourceSelection"
              >
                {{ t("common.clear") }}
              </AppButton>
            </div>

            <div class="max-h-56 overflow-auto">
              <div v-if="isLoadingSources" class="px-4 py-8 text-center text-sm text-zinc-500">
                <Loader2 class="mx-auto mb-2 h-5 w-5 animate-spin" />
                {{ t("coverage.scanningSources") }}
              </div>
              <div
                v-else-if="scanError"
                class="m-2 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2.5 text-xs text-rose-700"
              >
                {{ scanError.message }}
                <p v-if="scanError.hint" class="mt-1 text-[11px] text-rose-600">
                  {{ scanError.hint }}
                </p>
              </div>
              <div v-else-if="sourceTree.length === 0" class="px-4 py-8 text-center text-sm text-zinc-500">
                {{ t("coverage.noSources") }}
                <div class="mt-3 flex items-center justify-center gap-2">
                  <AppButton variant="secondary" size="sm" :disabled="isRunning" @click="scanSourceFiles">
                    <RefreshCw class="h-3.5 w-3.5" />
                    {{ t("common.refresh") }}
                  </AppButton>
                </div>
              </div>
              <div v-else class="py-1">
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

            <div
              v-if="selectedSourceCount > 0"
              class="border-t border-border px-3 py-2 text-[11px] text-zinc-400"
            >
              {{ t("coverage.derivedSourceHint") }}:
              <span class="font-mono">{{
                deriveSourceDirs()
                  .map((d) => (d === "." ? t("coverage.rootDir") : d))
                  .join(", ")
              }}</span>
            </div>
          </div>

          <!-- 测试文件 -->
          <div class="rounded-lg border border-border">
            <div class="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
              <span class="text-xs font-medium text-zinc-700">{{ t("coverage.testFiles") }}</span>
              <span class="shrink-0 font-mono text-[10px] text-zinc-400">
                {{ t("coverage.targetsSelected", { count: selectedTestCount }) }}
              </span>
            </div>

            <div class="flex items-center gap-1 border-b border-border px-3 py-2">
              <AppButton
                variant="ghost"
                size="sm"
                :disabled="isRunning || testFiles.length === 0"
                @click="selectAllTests"
              >
                {{ t("common.selectAll") }}
              </AppButton>
              <AppButton
                variant="ghost"
                size="sm"
                :disabled="isRunning || selectedTestCount === 0"
                @click="clearTestSelection"
              >
                {{ t("common.clear") }}
              </AppButton>
              <span class="ml-auto shrink-0 text-[10px] text-zinc-400">
                {{ t("coverage.allTestsHint") }}
              </span>
            </div>

            <div class="max-h-56 overflow-auto">
              <div v-if="isLoadingTests" class="px-4 py-8 text-center text-sm text-zinc-500">
                <Loader2 class="mx-auto mb-2 h-5 w-5 animate-spin" />
                {{ t("coverage.scanningTests") }}
              </div>
              <div v-else-if="testFiles.length === 0" class="px-4 py-8 text-center text-sm text-zinc-500">
                {{ t("coverage.noTestFiles") }}
                <div class="mt-3 flex items-center justify-center gap-2">
                  <AppButton variant="secondary" size="sm" :disabled="isRunning" @click="scanTestFiles">
                    <RefreshCw class="h-3.5 w-3.5" />
                    {{ t("common.refresh") }}
                  </AppButton>
                  <AppButton
                    variant="primary"
                    size="sm"
                    @click="router.push({ name: 'ProjectGenerate', params: { id: projectId } })"
                  >
                    <WandSparkles class="h-3.5 w-3.5" />
                    {{ t("coverage.goGenerate") }}
                  </AppButton>
                </div>
              </div>
              <button
                v-for="file in testFiles"
                :key="file.relativePath"
                type="button"
                :disabled="isRunning"
                class="flex w-full items-center gap-3 border-b border-border px-3 py-2 text-left transition last:border-b-0 hover:bg-zinc-50 disabled:cursor-not-allowed"
                @click="toggleTestFile(file.relativePath)"
              >
                <span
                  class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition"
                  :class="
                    selectedTestFiles.includes(file.relativePath)
                      ? 'border-brand-500 bg-brand-500 text-white'
                      : 'border-zinc-300 bg-white'
                  "
                >
                  <Check v-if="selectedTestFiles.includes(file.relativePath)" class="h-3 w-3" />
                </span>
                <FileCode2 class="h-4 w-4 shrink-0 text-zinc-400" />
                <span class="min-w-0 flex-1 truncate font-mono text-xs text-zinc-700">
                  {{ file.relativePath }}
                </span>
              </button>
            </div>
          </div>
        </div>

        <!-- 底部：coverage.py 状态 + 运行按钮 -->
        <div class="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
          <div class="flex flex-wrap items-center gap-2">
            <span
              v-if="checkingCoverage"
              class="inline-flex items-center gap-1.5 text-xs text-zinc-500"
            >
              <Loader2 class="h-3.5 w-3.5 animate-spin" />
              {{ t("coverage.badge.checking") }}
            </span>
            <template v-else-if="coverageInstalled === true">
              <span
                class="inline-flex items-center gap-1.5 rounded-full border border-emerald-200 bg-emerald-50 px-2 py-0.5 text-[11px] font-medium text-emerald-700"
              >
                <CheckCircle2 class="h-3 w-3" />
                {{ t("coverage.badge.installed") }}
              </span>
            </template>
            <template v-else-if="coverageInstalled === false">
              <span
                class="inline-flex items-center gap-1.5 rounded-full border border-amber-200 bg-amber-50 px-2 py-0.5 text-[11px] font-medium text-amber-700"
              >
                <AlertCircle class="h-3 w-3" />
                {{ t("coverage.badge.notInstalled") }}
              </span>
              <AppButton
                variant="secondary"
                size="sm"
                :loading="installingCoverage"
                @click="installCoverage"
              >
                {{ t("coverage.installCoverage") }}
              </AppButton>
            </template>
            <span v-if="installMessage" class="text-[11px] text-zinc-400">{{ installMessage }}</span>
          </div>

          <AppButton
            variant="primary"
            :disabled="!canRun"
            :loading="isRunning"
            @click="runCoverage"
          >
            <Play v-if="!isRunning" class="h-4 w-4" />
            {{ isRunning ? t("coverage.running") : t("coverage.run") }}
          </AppButton>
        </div>
      </div>
    </section>

    <!-- 错误横幅 -->
    <div
      v-if="errorMessage"
      class="rounded-lg border border-rose-200 bg-rose-50 px-3.5 py-2.5 text-xs text-rose-700"
    >
      {{ errorMessage.message }}
      <p v-if="errorMessage.hint" class="mt-1 text-[11px] text-rose-600">
        {{ errorMessage.hint }}
      </p>
    </div>

    <!-- ══════════ 结果区 ══════════ -->
    <section v-if="hasResult" class="card overflow-hidden">
      <!-- 摘要条 -->
      <div class="grid grid-cols-2 gap-3 border-b border-border p-4 md:grid-cols-4">
        <div class="rounded-lg border p-3" :class="cardBorderClass(displayPercent)">
          <div class="text-[10px] text-zinc-500">{{ t("coverage.summary.totalCoverage") }}</div>
          <div class="mt-0.5 text-2xl font-semibold" :class="percentTextClass(displayPercent)">
            {{ displayPercent.toFixed(1) }}%
          </div>
        </div>

        <div class="rounded-lg border border-border p-3">
          <div class="text-[10px] text-zinc-500">{{ t("coverage.summary.statementCoverage") }}</div>
          <div class="mt-0.5 text-sm font-semibold text-zinc-900">
            {{
              hasTargetFilter
                ? t("coverage.summary.statements", {
                    covered: visibleStats.covered,
                    total: visibleStats.total,
                  })
                : t("coverage.summary.statements", {
                    covered: summary!.coveredStatements,
                    total: summary!.totalStatements,
                  })
            }}
          </div>
          <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-zinc-100">
            <div
              class="h-full rounded-full"
              :class="barBgClass(statementPercent)"
              :style="{ width: `${statementPercent}%` }"
            />
          </div>
        </div>

        <div class="rounded-lg border border-border p-3">
          <div class="flex items-center gap-1 text-[10px] text-zinc-500">
            <span>{{ t("coverage.summary.branchCoverage") }}</span>
            <AppHelpPopover :title="t('coverage.summary.branchCoverage')" :content="t('help.branchCoverage')" />
          </div>
          <template v-if="summary!.branchPercent !== null">
            <div class="mt-0.5 text-sm font-semibold text-zinc-900">
              {{ t("coverage.summary.branches", { covered: summary!.coveredBranches ?? 0, total: summary!.totalBranches ?? 0 }) }}
            </div>
            <div class="mt-0.5 text-xs text-zinc-500">{{ summary!.branchPercent.toFixed(1) }}%</div>
          </template>
          <div v-else class="mt-0.5 text-xs text-zinc-400">
            {{ t("coverage.summary.branchNotEnabled") }}
          </div>
        </div>

        <div class="rounded-lg border border-border p-3">
          <div class="text-[10px] text-zinc-500">{{ t("coverage.summary.filesCovered") }}</div>
          <div class="mt-0.5 text-sm font-semibold text-zinc-900">
            {{ fileStats.covered }} / {{ fileStats.total }}
          </div>
          <div class="mt-0.5 text-xs text-zinc-400">{{ t("coverage.summary.filesCoveredHint") }}</div>
        </div>
      </div>

      <!-- 文件列表 + 源码查看器 -->
      <div class="grid min-h-0 grid-cols-1 xl:grid-cols-[1fr_1.2fr]">
        <div class="min-h-0 border-b border-border xl:border-b-0 xl:border-r">
          <div class="flex flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-2">
            <div class="flex items-center gap-1">
              <button
                v-for="f in coverageFilters"
                :key="f.id"
                type="button"
                class="rounded-md px-2 py-1 text-[11px] font-medium transition"
                :class="
                  filter === f.id
                    ? 'bg-zinc-900 text-white'
                    : 'text-zinc-500 hover:bg-zinc-100'
                "
                @click="filter = f.id"
              >
                {{ f.label }}
              </button>
            </div>
            <button
              type="button"
              class="flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-800"
              @click="sortOrder = sortOrder === 'desc' ? 'asc' : 'desc'"
            >
              <ArrowUpDown class="h-3 w-3" />
              {{ sortOrder === "desc" ? t("coverage.fileList.highestFirst") : t("coverage.fileList.lowestFirst") }}
            </button>
          </div>

          <div class="max-h-96 divide-y divide-border overflow-auto">
            <div
              v-if="displayFiles.length === 0"
              class="px-4 py-10 text-center text-xs text-zinc-400"
            >
              {{ t("coverage.fileList.noResults") }}
            </div>
            <button
              v-for="file in displayFiles"
              :key="file.path"
              type="button"
              :disabled="isRunning"
              class="flex w-full items-center gap-3 px-4 py-2.5 text-left transition hover:bg-zinc-50 disabled:cursor-not-allowed"
              @click="toggleDetail(file.path)"
              @contextmenu.prevent="showFileMenu(file, $event)"
              @dblclick="openSourceFile(file)"
            >
              <FileCode2 class="h-4 w-4 shrink-0 text-zinc-400" />
              <span class="min-w-0 flex-1">
                <span class="block truncate font-mono text-xs text-zinc-700">{{ file.path }}</span>
              </span>
              <span class="w-20 shrink-0">
                <div class="h-1 overflow-hidden rounded-full bg-zinc-100">
                  <div
                    class="h-full rounded-full"
                    :class="barBgClass(file.percentCovered)"
                    :style="{ width: `${file.percentCovered}%` }"
                  />
                </div>
              </span>
              <span class="w-12 shrink-0 text-right font-mono text-[11px]" :class="percentTextClass(file.percentCovered)">
                {{ file.percentCovered.toFixed(0) }}%
              </span>
              <ChevronDown v-if="expandedFile === file.path" class="h-3.5 w-3.5 shrink-0 text-zinc-400" />
              <ChevronRight v-else class="h-3.5 w-3.5 shrink-0 text-zinc-400" />
            </button>
          </div>
        </div>

        <!-- 源码查看器 -->
        <div class="min-h-64 overflow-auto xl:max-h-96">
          <div
            v-if="!expandedFile"
            class="flex h-full min-h-64 flex-col items-center justify-center px-6 text-center"
          >
            <FileCode2 class="mb-2 h-8 w-8 text-zinc-200" />
            <p class="text-xs text-zinc-400">{{ t("coverage.detail.title") }}</p>
          </div>
          <div
            v-else-if="detailLoading"
            class="flex h-full min-h-64 items-center justify-center text-xs text-zinc-400"
          >
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            {{ t("coverage.detail.loading") }}
          </div>
          <div
            v-else-if="detailError"
            class="m-4 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2.5 text-xs text-rose-700"
          >
            {{ detailError }}
          </div>
          <div v-else-if="detail" class="py-2">
            <div class="flex items-center justify-between gap-3 border-b border-border px-4 pb-2">
              <span class="min-w-0 truncate font-mono text-[11px] text-zinc-500">
                {{ detail.path }}
              </span>
              <span class="shrink-0 font-mono text-[11px] font-semibold" :class="percentTextClass(detail.percentCovered)">
                {{ t("coverage.detail.percentCovered", { percent: detail.percentCovered.toFixed(1) }) }}
              </span>
            </div>

            <div class="flex items-center gap-3 border-b border-border px-4 py-1.5 text-[10px] text-zinc-500">
              <span class="flex items-center gap-1">
                <span class="h-2 w-2 rounded-sm bg-emerald-500" />
                {{ t("coverage.detail.legend.covered") }}
              </span>
              <span class="flex items-center gap-1">
                <span class="h-2 w-2 rounded-sm bg-rose-500" />
                {{ t("coverage.detail.legend.missing") }}
              </span>
              <span class="flex items-center gap-1">
                <span class="h-2 w-2 rounded-sm bg-amber-500" />
                {{ t("coverage.detail.legend.excluded") }}
              </span>
            </div>

            <!-- FR013：函数级过滤 -->
            <div
              v-if="functionStats.length > 0"
              class="flex flex-wrap items-center gap-2 border-b border-border px-4 py-1.5"
            >
              <label class="text-[10px] font-medium text-zinc-500">
                {{ t("coverage.detail.functionFilter") }}
              </label>
              <select
                v-model="functionFilter"
                class="max-w-56 rounded-md border border-border bg-white px-2 py-1 text-[11px] text-zinc-700 focus:outline-none focus:ring-1 focus:ring-sky-500"
              >
                <option value="__all__">{{ t("coverage.detail.allFunctions") }}</option>
                <option v-for="fn in functionStats" :key="fn.name" :value="fn.name">
                  {{ fn.name }}
                  ({{ fn.covered + fn.missing > 0
                    ? `${Math.round((fn.covered / (fn.covered + fn.missing)) * 100)}%`
                    : "—" }})
                </option>
              </select>
              <span v-if="activeFunction" class="text-[10px] text-zinc-400">
                {{ t("coverage.detail.functionCoverage", {
                  covered: activeFunction.covered,
                  total: activeFunction.covered + activeFunction.missing,
                }) }}
              </span>
            </div>

            <div
              v-for="line in filteredDetailLines"
              :key="line.lineNumber"
              class="flex gap-3 px-4 py-0.5 font-mono text-[11px] leading-5"
              :class="lineRowClass(line.status)"
            >
              <span class="w-10 shrink-0 select-none text-right text-zinc-400">
                {{ line.lineNumber }}
              </span>
              <span class="min-w-0 flex-1 whitespace-pre-wrap" :class="lineTextClass(line.status)">
                {{ line.source }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- 底部：导出 + 日志 -->
      <div class="flex flex-wrap items-center justify-between gap-3 border-t border-border px-4 py-3">
        <div class="flex flex-wrap items-center gap-2">
          <AppButton
            variant="secondary"
            size="sm"
            :loading="exporting === 'csv'"
            :disabled="exporting !== null"
            @click="exportReport('csv')"
          >
            <Download class="h-3.5 w-3.5" />
            {{ t("coverage.export.csv") }}
          </AppButton>
          <AppButton
            variant="secondary"
            size="sm"
            :loading="exporting === 'json'"
            :disabled="exporting !== null"
            @click="exportReport('json')"
          >
            <Download class="h-3.5 w-3.5" />
            {{ t("coverage.export.json") }}
          </AppButton>
          <AppButton variant="ghost" size="sm" @click="openInFileManager">
            <FolderOpen class="h-3.5 w-3.5" />
            {{ t("coverage.openInFileManager") }}
          </AppButton>
          <span v-if="exportMessage" class="max-w-64 truncate text-[11px] text-zinc-400">
            {{ exportMessage }}
          </span>
        </div>

        <button
          type="button"
          class="flex items-center gap-1.5 text-xs font-medium text-zinc-500 transition hover:text-zinc-800"
          @click="showCoverageLog = !showCoverageLog"
        >
          <ChevronDown v-if="showCoverageLog" class="h-3.5 w-3.5" />
          <ChevronRight v-else class="h-3.5 w-3.5" />
          {{ t("coverage.outputTitle") }} ({{ coverageOutput.length }})
        </button>
      </div>

      <!-- 日志 -->
      <div v-if="showCoverageLog && coverageOutput.length > 0" class="border-t border-border">
        <div class="flex items-center justify-between gap-3 bg-zinc-950 px-4 py-2">
          <span class="truncate font-mono text-[10px] text-zinc-400">
            {{ t("coverage.logLines", { count: coverageOutput.length }) }}
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
        <pre
          ref="logContainer"
          class="max-h-56 overflow-auto bg-zinc-950 px-4 py-3 font-mono text-[11px] leading-5"
        >
          <template v-for="output in coverageOutput" :key="output.logId">
            <span :class="output.stream === 'stderr' ? 'text-rose-300' : 'text-zinc-300'">{{
              output.line
            }}</span>{{ "\n" }}
          </template>
        </pre>
      </div>
    </section>

    <!-- 文件列表右键菜单 -->
    <AppContextMenu
      v-if="fileMenu"
      :items="fileMenuItems"
      :open="!!fileMenu"
      @close="fileMenu = null"
    />
  </div>
</template>
