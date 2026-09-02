<script setup lang="ts">
/**
 * Execute —— 测试执行页（彻底重构版）
 * 设计：单一主流程
 *   ① 运行条（一张卡片）：范围三段选择 + 目标选择 + Options 折叠 + Run/Stop
 *   ② 运行监视器（一张卡片三态）：空闲引导 / 运行中终端日志+进度 / 完成结论+失败列表
 *   ③ 已保存选择：底部折叠区
 * 所有业务逻辑（事件监听 / 收集 / 执行 / 套件 / 持久化）保持不变。
 */
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRoute, useRouter } from "vue-router";
import {
  AlertCircle,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock3,
  Copy,
  Eye,
  FileCode2,
  FolderOpen,
  Gauge,
  ListChecks,
  Loader2,
  Play,
  RefreshCw,
  RotateCcw,
  Save,
  Search,
  Square,
  Trash2,
  WandSparkles,
  XCircle,
} from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import { useUIStore } from "../../../stores/uiStore";
import { parseArguments } from "../../../helper/execute";
import { interpretError, type InterpretedError } from "../../../utils/errors";
import { useI18n } from "vue-i18n";
import AppModal from "../../../components/ui/AppModal.vue";
import AppButton from "../../../components/ui/AppButton.vue";
import AppConfirmModal from "../../../components/ui/AppConfirmModal.vue";
import AppTooltip from "../../../components/ui/AppTooltip.vue";
import AppContextMenu from "../../../components/ui/AppContextMenu.vue";
import AppHelpPopover from "../../../components/ui/AppHelpPopover.vue";
import TerminalPanel from "../../../components/ui/TerminalPanel.vue";
import StatusPill from "../../../components/ui/StatusPill.vue";
import { useTaskbarProgress } from "../../../composables/useTaskbarProgress";
import { useWindowTitle } from "../../../composables/useWindowTitle";
import { usePageShortcuts } from "../../../composables/useKeyboardShortcuts";
import { notifyExecutionComplete } from "../../../composables/useNotifications";

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();
const uiStore = useUIStore();
const { t } = useI18n();

const { setProgress: setTaskProgress, clear: clearTaskProgress } = useTaskbarProgress();
const { setStatus: setWinStatus, reset: resetWinStatus } = useWindowTitle();

/* 右键菜单：测试用例列表 */
const testCaseMenu = ref<{ test: TestCase; x: number; y: number } | null>(null);
function showTestCaseMenu(test: TestCase, e: MouseEvent) {
  if (isRunning.value) return;
  testCaseMenu.value = { test, x: e.clientX, y: e.clientY };
}
const testCaseMenuItems = computed(() => {
  if (!testCaseMenu.value) return [];
  const test = testCaseMenu.value.test;
  return [
    {
      label: t("contextmenu.runThisTest"),
      icon: Play,
      action: () => {
        selectedTestCases.value = [test.id];
        testScope.value = "selected";
        void runTests();
      },
    },
    { divider: true },
    { label: t("contextmenu.openFile"), icon: FileCode2, disabled: !test.file, action: () => openTestFile(test.file, test.line) },
    { label: t("contextmenu.preview"), icon: Eye, disabled: !test.file, action: () => openPreview(test.file, test.line) },
    { label: t("contextmenu.copyPath"), icon: Copy, disabled: !test.file, action: () => copyPathWithLine(test.file, test.line) },
    { label: t("contextmenu.openInFolder"), icon: FolderOpen, disabled: !test.file, action: () => revealInFileManager(test.file) },
    { divider: true },
    { label: t("contextmenu.copyTestId"), icon: Copy, action: () => copyText(test.id) },
  ];
});

/* 右键菜单：测试结果列表 */
const resultMenu = ref<{ result: TestResult; x: number; y: number } | null>(null);
function showResultMenu(result: TestResult, e: MouseEvent) {
  resultMenu.value = { result, x: e.clientX, y: e.clientY };
}
const resultMenuItems = computed(() => {
  if (!resultMenu.value) return [];
  const r = resultMenu.value.result;
  const isFailed = r.status === "failed" || r.status === "error";
  return [
    { label: t("contextmenu.openFile"), icon: FileCode2, disabled: !r.file, action: () => openTestFile(r.file, r.line) },
    { label: t("contextmenu.preview"), icon: Eye, disabled: !r.file, action: () => openPreview(r.file, r.line) },
    { label: t("contextmenu.copyPath"), icon: Copy, disabled: !r.file, action: () => copyPathWithLine(r.file, r.line) },
    { label: t("contextmenu.openInFolder"), icon: FolderOpen, disabled: !r.file, action: () => revealInFileManager(r.file) },
    { divider: true },
    {
      label: t("contextmenu.copyError"),
      icon: Copy,
      disabled: !r.errorMessage,
      action: () => copyText(r.errorMessage ?? ""),
    },
    {
      label: t("contextmenu.rerunFailed"),
      icon: RotateCcw,
      disabled: !isFailed,
      action: () => {
        if (isFailed) {
          selectedTestCases.value = failedResults.value.map((x) => x.id);
          testScope.value = "selected";
          void runTests();
        }
      },
    },
  ];
});

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    console.error("[Execute] copy failed:", error);
  }
}

const projectId = computed(() => Number(route.params.id));

type ExecutionStatus = "idle" | "running" | "completed" | "failed";
type TestScope = "all" | "file" | "selected" | "suite";
type ResultFilter = "all" | "passed" | "failed" | "skipped" | "xfailed";

interface TestFile {
  name: string;
  path: string;
  relativePath: string;
}

interface TestCase {
  id: string;
  name: string;
  file: string;
  className: string | null;
  line: number | null;
}

interface TestOutputEvent {
  runId: string;
  stream: "stdout" | "stderr";
  line: string;
  logId?: number;
}

interface TestResult {
  id: string;
  name: string;
  file: string;
  status: "passed" | "failed" | "skipped" | "error" | "xfailed" | "xpassed";
  duration: number;
  errorMessage: string | null;
  line: number | null;
  skipReason: string | null;
}

interface TestStartedEvent {
  runId: string;
  total: number;
}

interface TestFinishedEvent {
  runId: string;
  success: boolean;
  exitCode: number | null;
  duration: number;
  passed: number;
  failed: number;
  skipped: number;
  results: TestResult[];
  command: string;
  executionType: string;
  regressionSuiteId: number | null;
}

interface Preset {
  id: string;
  name: string;
  description: string;
  args: string[];
}

const currentProject = computed(() => {
  return projectStore.projects.find(
    (project) => project.id === projectId.value,
  );
});

/* -------------------------------------------------------------------------- */
/* State                                                                      */
/* -------------------------------------------------------------------------- */

const isRunning = ref(false);
const executionStatus = ref<ExecutionStatus>("idle");

const testScope = ref<TestScope>("all");
const selectedTestFile = ref("");
const selectedTestCases = ref<string[]>([]);

const testFiles = ref<TestFile[]>([]);
const testCases = ref<TestCase[]>([]);

const isLoadingTests = ref(false);
const isCollecting = ref(false);
const testScanError = ref<InterpretedError | null>(null);
const collectError = ref<InterpretedError | null>(null);

const testSearch = ref("");
const resultFilter = ref<ResultFilter>("all");

const selectedPreset = ref("standard");
const showAdvanced = ref(false);
const customArguments = ref("");

const pytestOutput = ref<TestOutputEvent[]>([]);

/**
 * 日志批量冲刷（NFR003 修复）：
 * 事件到达先入「非响应式」缓冲，每帧（rAF）最多合并一次进 pytestOutput。
 * 修复前每行事件都触发 push + shift + 整表 500 行重渲染，10 万行洪流下
 * 每秒约 1.6 万次重渲染会把 UI 主线程拖垮（实测 UI 卡死、内存 273MB）。
 * 批量后渲染次数降到 ~60 次/秒，UI 在洪流中保持流畅；窗口被隐藏时 rAF
 * 停发，用安全阀在缓冲过大时立即冲刷，避免内存无限增长。
 */
const pendingOutput: TestOutputEvent[] = [];
let flushScheduled = false;

function flushPendingOutput() {
  flushScheduled = false;
  if (pendingOutput.length === 0) return;
  const batch = pendingOutput.splice(0, pendingOutput.length);
  pytestOutput.value.push(...batch);
  if (pytestOutput.value.length > 500) {
    pytestOutput.value.splice(0, pytestOutput.value.length - 500);
  }
}

function scheduleFlushOutput() {
  if (flushScheduled) return;
  if (pendingOutput.length > 5000) {
    flushPendingOutput();
    return;
  }
  flushScheduled = true;
  requestAnimationFrame(flushPendingOutput);
}

const currentRunId = ref<string | null>(null);
const currentTest = ref<string | null>(null);

const totalTests = ref(0);
const completedTests = ref(0);
const passedTests = ref(0);
const failedTests = ref(0);
const skippedTests = ref(0);

const executionDuration = ref(0);
const testResults = ref<TestResult[]>([]);

const expandedFailures = ref<Set<string>>(new Set());

/* 运行完成弹窗（手动运行后询问下一步：覆盖率 / 保存选择 / 稍后） */
const showPostRunModal = ref(false);
const postRunSummary = ref<{
  passed: number;
  failed: number;
  skipped: number;
  duration: number;
} | null>(null);

let unlistenStarted: UnlistenFn | undefined;
let unlistenOutput: UnlistenFn | undefined;
let unlistenFinished: UnlistenFn | undefined;
let logCounter = 0;

/* -------------------------------------------------------------------------- */
/* Presets                                                                    */
/* -------------------------------------------------------------------------- */

const presets: Preset[] = [
  {
    id: "standard",
    name: t("execute.presets.standard"),
    description: t("execute.presets.standardDesc"),
    args: ["-v"],
  },
  {
    id: "quick",
    name: t("execute.presets.quick"),
    description: t("execute.presets.quickDesc"),
    args: ["-q"],
  },
  {
    id: "debug",
    name: t("execute.presets.debug"),
    description: t("execute.presets.debugDesc"),
    args: ["-v", "--tb=short"],
  },
  {
    id: "stop-on-failure",
    name: t("execute.presets.stopOnFailure"),
    description: t("execute.presets.stopOnFailureDesc"),
    args: ["-v", "-x"],
  },
];

const activePreset = computed(() => {
  return presets.find((preset) => preset.id === selectedPreset.value);
});

const presetHelpMap: Record<string, string> = {
  standard: t("help.presetStandard"),
  quick: t("help.presetQuick"),
  debug: t("help.presetDebug"),
  "stop-on-failure": t("help.presetStopOnFailure"),
};

const activeArguments = computed(() => {
  if (customArguments.value.trim()) {
    return parseArguments(customArguments.value);
  }

  return activePreset.value?.args ?? ["-v"];
});

/* -------------------------------------------------------------------------- */
/* Test selection                                                             */
/* -------------------------------------------------------------------------- */

/** 范围四段选择（紧凑分段控件） */
const scopeOptions = computed(() => [
  { id: "all" as TestScope, label: t("execute.scope.all") },
  { id: "file" as TestScope, label: t("execute.scope.file") },
  { id: "selected" as TestScope, label: t("execute.scope.selected") },
  { id: "suite" as TestScope, label: t("execute.scope.suite") },
]);

const filteredTestCases = computed(() => {
  const query = testSearch.value.trim().toLowerCase();

  if (!query) {
    return testCases.value;
  }

  return testCases.value.filter((test) => {
    return (
      test.name.toLowerCase().includes(query) ||
      test.file.toLowerCase().includes(query) ||
      test.id.toLowerCase().includes(query) ||
      test.className?.toLowerCase().includes(query)
    );
  });
});

const selectedCount = computed(() => selectedTestCases.value.length);

const allVisibleSelected = computed(() => {
  if (filteredTestCases.value.length === 0) {
    return false;
  }

  return filteredTestCases.value.every((test) =>
    selectedTestCases.value.includes(test.id),
  );
});

const selectedScopeDescription = computed(() => {
  switch (testScope.value) {
    case "all":
      return t("execute.scopeDesc.all", { count: testCases.value.length });
    case "file": {
      const file = testFiles.value.find(
        (item) => item.relativePath === selectedTestFile.value,
      );

      if (!file) {
        return t("execute.scopeDesc.selectFile");
      }

      const count = testCases.value.filter(
        (test) => test.file === file.relativePath,
      ).length;

      return t("execute.scopeDesc.file", { count, file: file.relativePath });
    }
    case "selected":
      return t("execute.scopeDesc.selected", { count: selectedCount.value });
    case "suite":
      return t("execute.scopeDesc.suite", { count: suites.value.length });
    default:
      return "";
  }
});

const executionTargets = computed<string[]>(() => {
  switch (testScope.value) {
    case "all":
      return [];

    case "file":
      return selectedTestFile.value ? [selectedTestFile.value] : [];

    case "selected":
      return [...selectedTestCases.value];
    default:
      return [];
  }
});

const canRun = computed(() => {
  if (isRunning.value) {
    return false;
  }

  if (!currentProject.value?.path) {
    return false;
  }

  if (!currentProject.value?.interpreter_path) {
    return false;
  }

  if (testCases.value.length === 0) {
    return false;
  }

  if (testScope.value === "file" && !selectedTestFile.value) {
    return false;
  }

  if (testScope.value === "selected" && selectedTestCases.value.length === 0) {
    return false;
  }

  // suite 模式：在列表中直接点 Run，主按钮不可用
  if (testScope.value === "suite") {
    return false;
  }

  return true;
});

/* -------------------------------------------------------------------------- */
/* Results                                                                    */
/* -------------------------------------------------------------------------- */

const totalResultCount = computed(() => testResults.value.length);

const filteredResults = computed(() => {
  if (resultFilter.value === "all") {
    return testResults.value;
  }

  if (resultFilter.value === "passed") {
    // 意外通过（strict-xpass）语义上属于通过，并入 passed 筛选
    return testResults.value.filter(
      (result) => result.status === "passed" || result.status === "xpassed",
    );
  }

  if (resultFilter.value === "failed") {
    return testResults.value.filter(
      (result) => result.status === "failed" || result.status === "error",
    );
  }

  if (resultFilter.value === "skipped") {
    // skipped 筛选包含普通跳过与预期失败（xfail 属于 skipped 子类，汇总计数一致）
    return testResults.value.filter(
      (result) => result.status === "skipped" || result.status === "xfailed",
    );
  }

  return testResults.value.filter(
    (result) => result.status === resultFilter.value,
  );
});

const xfailedTests = computed(() =>
  testResults.value.filter((result) => result.status === "xfailed").length,
);

const xpassedTests = computed(() =>
  testResults.value.filter((result) => result.status === "xpassed").length,
);

const failedResults = computed(() => {
  return testResults.value.filter(
    (result) => result.status === "failed" || result.status === "error",
  );
});

const progress = computed(() => {
  if (totalTests.value <= 0) {
    return isRunning.value ? 0 : testResults.value.length > 0 ? 100 : 0;
  }

  return Math.min(
    100,
    Math.round((completedTests.value / totalTests.value) * 100),
  );
});

const resultConclusion = computed(() => {
  if (executionStatus.value === "completed" && failedTests.value === 0) {
    return {
      title: t("execute.results.conclusionPassed"),
      description: t("execute.results.conclusionPassedDesc"),
    };
  }

  if (executionStatus.value === "failed") {
    if (failedTests.value > 0) {
      return {
        title: t("execute.results.conclusionFailed"),
        description: t("execute.results.conclusionFailedDesc", {
          count: failedTests.value,
        }),
      };
    }

    return {
      title: t("execute.results.conclusionExecFailed"),
      description: t("execute.results.conclusionExecFailedDesc"),
    };
  }

  return {
    title: t("execute.results.conclusionReady"),
    description: t("execute.results.conclusionReadyDesc"),
  };
});

const statusText = computed(() => {
  switch (executionStatus.value) {
    case "running":
      return t("execute.results.statusRunning");
    case "completed":
      return failedTests.value > 0
        ? t("execute.results.statusFailed")
        : t("execute.results.statusPassed");
    case "failed":
      return t("execute.results.statusFailed");
    default:
      return t("execute.results.statusReady");
  }
});

/** 页头 StatusPill 状态映射（视觉交给 StatusPill 组件） */
const statusPillStatus = computed(() => {
  switch (executionStatus.value) {
    case "running":
      return "running";
    case "completed":
      return failedTests.value > 0 ? "Failed" : "completed";
    case "failed":
      return "Failed";
    default:
      return "idle";
  }
});

const hasResult = computed(() => {
  return (
    executionStatus.value === "completed" || executionStatus.value === "failed"
  );
});

const lastRunSummary = computed(() => {
  if (!hasResult.value) {
    return t("execute.execution.noRunYet");
  }

  return `${totalResultCount.value} tests · ${executionDuration.value.toFixed(2)}s`;
});

/** 结果筛选（全部 / 通过 / 失败 / 跳过） */
const resultFilters = computed(() => [
  {
    id: "all" as ResultFilter,
    label: t("execute.results.filterAll", { count: testResults.value.length }),
  },
  {
    id: "passed" as ResultFilter,
    label: t("execute.results.filterPassed", { count: passedTests.value }),
  },
  {
    id: "failed" as ResultFilter,
    label: t("execute.results.filterFailed", { count: failedTests.value }),
  },
  {
    id: "skipped" as ResultFilter,
    label: t("execute.results.filterSkipped", { count: skippedTests.value }),
  },
  {
    id: "xfailed" as ResultFilter,
    label: t("execute.results.filterXfailed", { count: xfailedTests.value }),
  },
]);

/* -------------------------------------------------------------------------- */
/* Event listeners                                                            */
/* -------------------------------------------------------------------------- */

async function setupTestListeners() {
  unlistenStarted = await listen<TestStartedEvent>("test-started", (event) => {
    currentRunId.value = event.payload.runId;

    const expectedTotal = getExpectedTestCount();

    totalTests.value =
      event.payload.total > 0 ? event.payload.total : expectedTotal;

    completedTests.value = 0;
    passedTests.value = 0;
    failedTests.value = 0;
    skippedTests.value = 0;

    executionDuration.value = 0;
    currentTest.value = null;

    testResults.value = [];
    pytestOutput.value = [];
    pendingOutput.length = 0;
    expandedFailures.value = new Set();

    executionStatus.value = "running";
    isRunning.value = true;
    logCounter = 0;

    setWinStatus(t("execute.results.statusRunning"));
    const expected = event.payload.total > 0 ? event.payload.total : expectedTotal;
    void setTaskProgress(0, expected || 1, "indeterminate");
  });

  unlistenOutput = await listen<TestOutputEvent>("test-output", (event) => {
    if (event.payload.runId !== currentRunId.value) {
      return;
    }

    pendingOutput.push({
      ...event.payload,
      logId: logCounter++,
    });
    scheduleFlushOutput();

    parseRealtimeProgress(event.payload.line);
  });

  unlistenFinished = await listen<TestFinishedEvent>(
    "test-finished",
    async (event) => {
      if (event.payload.runId !== currentRunId.value) {
        return;
      }

      // 冲刷残留缓冲，确保日志尾部完整显示
      flushPendingOutput();

      isRunning.value = false;
      isRunningSuite.value = false;

      executionStatus.value = event.payload.success ? "completed" : "failed";

      executionDuration.value = event.payload.duration;

      passedTests.value = event.payload.passed;
      failedTests.value = event.payload.failed;
      skippedTests.value = event.payload.skipped;

      testResults.value = event.payload.results;

      totalTests.value =
        event.payload.results.length > 0
          ? event.payload.results.length
          : totalTests.value;

      completedTests.value = event.payload.results.length;

      currentTest.value = null;

      clearTaskProgress();
      resetWinStatus();

      // 完成后系统通知（长任务在后台时提醒用户）
      if (currentProject.value?.name) {
        void notifyExecutionComplete(
          currentProject.value.name,
          event.payload.passed,
          event.payload.failed,
        );
      }

      await saveExecutionToDb(event.payload);

      // 手动运行且有测试通过时：弹出下一步询问（覆盖率 / 保存选择 / 稍后）
      if (event.payload.passed > 0 && event.payload.executionType !== "REGRESSION") {
        postRunSummary.value = {
          passed: event.payload.passed,
          failed: event.payload.failed,
          skipped: event.payload.skipped,
          duration: event.payload.duration,
        };
        showPostRunModal.value = true;
      }

      if (!event.payload.success && event.payload.results.length === 0) {
        pytestOutput.value.push({
          runId: event.payload.runId,
          stream: "stderr",
          line: t("execute.logs.systemWarning"),
          logId: logCounter++,
        });
      }
    },
  );
}

function parseRealtimeProgress(line: string) {
  const match = line.match(/^(.+?)::(.+?)\s+(PASSED|FAILED|SKIPPED|ERROR)/);

  if (!match) {
    return;
  }

  const [, file, name] = match;

  currentTest.value = `${file}::${name}`;

  const completed = passedTests.value + failedTests.value + skippedTests.value;

  if (completed < totalTests.value) {
    completedTests.value = completed + 1;
  }

  if (totalTests.value > 0) {
    void setTaskProgress(Math.min(completed + 1, totalTests.value), totalTests.value, "normal");
  }
}

/* -------------------------------------------------------------------------- */
/* Test collection                                                            */
/* -------------------------------------------------------------------------- */

async function refreshTests() {
  await Promise.all([collectTestCases(), scanTestFiles()]);
}

async function scanTestFiles() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) {
    testScanError.value = { message: t("execute.projectPathUnavailable") };
    return;
  }

  isLoadingTests.value = true;
  testScanError.value = null;

  try {
    testFiles.value = await invoke<TestFile[]>("scan_test_files", {
      projectPath,
    });
  } catch (error) {
    console.error("[Execute] Failed to scan test files:", error);
    testScanError.value = interpretError(error);
    testFiles.value = [];
  } finally {
    isLoadingTests.value = false;
  }
}

async function collectTestCases() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) {
    collectError.value = { message: t("execute.projectPathUnavailable") };
    return;
  }

  isCollecting.value = true;
  collectError.value = null;

  try {
    testCases.value = await invoke<TestCase[]>("collect_test_cases", {
      projectPath,
    });

    selectedTestCases.value = selectedTestCases.value.filter((id) =>
      testCases.value.some((test) => test.id === id),
    );

    if (
      selectedTestFile.value &&
      !testFiles.value.some(
        (file) => file.relativePath === selectedTestFile.value,
      )
    ) {
      selectedTestFile.value = "";
    }
  } catch (error) {
    console.error("[Execute] collect_test_cases failed:", error);

    collectError.value = interpretError(error);
    testCases.value = [];
    selectedTestCases.value = [];
  } finally {
    isCollecting.value = false;
  }
}

/* -------------------------------------------------------------------------- */
/* Selection                                                                  */
/* -------------------------------------------------------------------------- */

function setScope(scope: TestScope) {
  if (isRunning.value) {
    return;
  }

  testScope.value = scope;

  if (scope === "all") {
    return;
  }

  if (scope === "file") {
    if (!selectedTestFile.value && testFiles.value.length > 0) {
      selectedTestFile.value = testFiles.value[0].relativePath;
    }

    return;
  }
}

let lastSelectedIndex: number | null = null;

function toggleTest(testId: string, event?: MouseEvent) {
  if (isRunning.value) {
    return;
  }

  const currentIndex = filteredTestCases.value.findIndex((t) => t.id === testId);

  // Shift+Click：选中从上次点击到当前项之间的连续范围（并入当前选择）
  if (event?.shiftKey && lastSelectedIndex !== null && currentIndex !== -1) {
    const [start, end] = currentIndex >= lastSelectedIndex
      ? [lastSelectedIndex, currentIndex]
      : [currentIndex, lastSelectedIndex];
    const rangeIds = filteredTestCases.value
      .slice(start, end + 1)
      .map((t) => t.id);
    selectedTestCases.value = Array.from(
      new Set([...selectedTestCases.value, ...rangeIds]),
    );
    lastSelectedIndex = currentIndex;
    testScope.value = "selected";
    return;
  }

  const index = selectedTestCases.value.indexOf(testId);

  if (index === -1) {
    selectedTestCases.value.push(testId);
  } else {
    selectedTestCases.value.splice(index, 1);
  }

  lastSelectedIndex = currentIndex;
  testScope.value = "selected";
}

function isTestSelected(testId: string) {
  return selectedTestCases.value.includes(testId);
}

function selectAllVisible() {
  if (isRunning.value) {
    return;
  }

  const ids = filteredTestCases.value.map((test) => test.id);

  selectedTestCases.value = Array.from(
    new Set([...selectedTestCases.value, ...ids]),
  );
}

function clearSelection() {
  if (isRunning.value) {
    return;
  }

  selectedTestCases.value = [];
}

function getExpectedTestCount() {
  switch (testScope.value) {
    case "all":
      return testCases.value.length;

    case "selected":
      return selectedTestCases.value.length;

    case "file":
      return testCases.value.filter(
        (test) => test.file === selectedTestFile.value,
      ).length;

    default:
      return 0;
  }
}

/* -------------------------------------------------------------------------- */
/* Execution                                                                  */
/* -------------------------------------------------------------------------- */

async function runTests() {
  if (!canRun.value) {
    return;
  }

  const projectPath = currentProject.value?.path;
  const interpreterPath = currentProject.value?.interpreter_path;

  if (!projectPath || !interpreterPath) {
    return;
  }

  const args = activeArguments.value;
  const targets = executionTargets.value;

  resetExecutionState();

  try {
    await invoke("run_tests", {
      projectId: currentProject.value!.id,
      projectPath,
      interpreterPath,
      testCases: targets,
      pytestArgs: args,
    });
  } catch (error) {
    console.error("[Execute] Failed to run tests:", error);

    isRunning.value = false;
    executionStatus.value = "failed";
    currentTest.value = null;

    const errorMsg = error instanceof Error ? error.message : String(error);

    pytestOutput.value.push({
      runId: currentRunId.value ?? "local",
      stream: "stderr",
      line: t("execute.logs.fatalStart", { msg: errorMsg }),
      logId: logCounter++,
    });
  }
}

function resetExecutionState() {
  currentRunId.value = null;
  currentTest.value = null;

  totalTests.value = getExpectedTestCount();
  completedTests.value = 0;

  passedTests.value = 0;
  failedTests.value = 0;
  skippedTests.value = 0;

  executionDuration.value = 0;

  testResults.value = [];
  pytestOutput.value = [];
  pendingOutput.length = 0;

  expandedFailures.value = new Set();

  executionStatus.value = "idle";
}

/**
 * 取消当前测试运行：调用 Rust cancel_run（SIGTERM → SIGKILL）。
 * 进程被终止后，run_tests 会照常发出 test-finished（success=false），
 * 由 listener 负责复位 UI 状态。
 */
async function stopTests() {
  if (!isRunning.value || !currentRunId.value) {
    return;
  }

  try {
    await invoke("cancel_run", { runId: currentRunId.value });
  } catch (error) {
    console.error("[Execute] Failed to cancel run:", error);
  }
}

/* -------------------------------------------------------------------------- */
/* Regression Suites (FR007)                                                  */
/* -------------------------------------------------------------------------- */

interface RegressionSuite {
  id: number;
  projectId: number;
  suiteName: string;
  targetPaths: string[];
  customParams: string[] | null;
  createdAt: string;
}

const suites = ref<RegressionSuite[]>([]);
const showSaveSuiteModal = ref(false);
const newSuiteName = ref("");
const isSavingSuite = ref(false);
const isRunningSuite = ref(false);

async function loadSuites() {
  if (!currentProject.value?.id) return;

  try {
    const rows = await invoke<RegressionSuite[]>("list_regression_suites", {
      projectId: currentProject.value.id,
    });

    // 兼容 target_paths 为 NULL 的历史数据（scope=all 保存或旧版本记录）
    suites.value = (rows ?? []).map((suite) => ({
      ...suite,
      targetPaths: suite.targetPaths ?? [],
      customParams: suite.customParams ?? null,
    }));
  } catch (error) {
    console.error("[Execute] Failed to load regression suites:", error);
  }
}

function openSaveSuiteModal() {
  newSuiteName.value = "";
  showSaveSuiteModal.value = true;
}

async function saveSuite() {
  const name = newSuiteName.value.trim();
  if (!name) return;

  isSavingSuite.value = true;

  try {
    await invoke("save_regression_suite", {
      projectId: currentProject.value!.id,
      suiteName: name,
      targetPaths: executionTargets.value,
      customParams: activeArguments.value.length > 0 ? activeArguments.value : null,
    });

    newSuiteName.value = "";
    showSaveSuiteModal.value = false;
    await loadSuites();
  } catch (error) {
    console.error("[Execute] Failed to save regression suite:", error);
  } finally {
    isSavingSuite.value = false;
  }
}

async function runSuite(suite: RegressionSuite) {
  if (isRunning.value || isRunningSuite.value) return;

  isRunningSuite.value = true;

  try {
    resetExecutionState();
    await invoke("run_regression_suite", { suiteId: suite.id });
  } catch (error) {
    console.error("[Execute] Failed to run regression suite:", error);
    isRunning.value = false;
    isRunningSuite.value = false;
    executionStatus.value = "failed";
    currentTest.value = null;
  }
}

/** 待删除套件（确认弹窗） */
const deleteSuiteTarget = ref<RegressionSuite | null>(null);

/**
 * 套件目标摘要：
 *  - target_paths 为空（"全部测试"范围保存）→ "All tests"
 *  - 单个文件路径（不含 ::）→ 显示文件名
 *  - 否则 → 数量（测试用例 id）
 */
function suiteTargetsLabel(suite: RegressionSuite) {
  const paths = suite.targetPaths ?? [];
  if (paths.length === 0) return t("execute.suiteScopeAll");
  if (paths.length === 1 && !paths[0].includes("::")) {
    return t("execute.suiteTargetsFile", { file: paths[0] });
  }
  return t("execute.suiteTargets", { count: paths.length });
}

async function confirmDeleteSuite() {
  const suite = deleteSuiteTarget.value;
  if (!suite) return;

  try {
    await invoke("delete_regression_suite", { suiteId: suite.id });
    await loadSuites();
  } catch (error) {
    console.error("[Execute] Failed to delete regression suite:", error);
  } finally {
    deleteSuiteTarget.value = null;
  }
}

/** 运行完成弹窗 → 保存当前选择 */
function saveFromPostRun() {
  showPostRunModal.value = false;
  openSaveSuiteModal();
}

/** 运行完成弹窗 → 跳转覆盖率页（自动运行） */
function runCoverageFromPostRun() {
  showPostRunModal.value = false;
  router.push({
    name: "ProjectCoverage",
    params: { id: projectId.value },
    query: { autoRun: "1" },
  });
}

/**
 * 将测试执行结果持久化到本地 SQLite 数据库（NFR008：走 Rust 类型化命令）
 * execution_type 对齐论文 Table 5.2：MANUAL（手动执行）/ REGRESSION（回归套件重跑）
 */
async function saveExecutionToDb(payload: TestFinishedEvent) {
  if (!currentProject.value?.id) return;

  try {
    const totalTests = payload.passed + payload.failed + payload.skipped;
    const executionStatus = payload.success ? "success" : "failed";

    const executionId = await invoke<number>("save_execution_history", {
      projectId: currentProject.value.id,
      executionType: payload.executionType,
      regressionSuiteId: payload.regressionSuiteId,
      executionStatus,
      command: payload.command || null,
      totalTests,
      passed: payload.passed,
      failed: payload.failed,
      skipped: payload.skipped,
      executionTime: payload.duration,
    });

    // 保存单条结果明细（供历史详情查看失败原因等）
    if (payload.results.length > 0) {
      await invoke("save_execution_result_details", {
        executionId,
        results: payload.results.map((r) => ({
          name: r.name,
          file: r.file || null,
          status: r.status,
          duration: r.duration,
          errorMessage: r.errorMessage,
          line: r.line,
          skipReason: r.skipReason,
        })),
      });
    }
    console.log("[DB] ✅ Execution history saved successfully");
  } catch (error) {
    console.error("[DB] ❌ Failed to save execution history:", error);
  }
}

/* -------------------------------------------------------------------------- */
/* Results                                                                    */
/* -------------------------------------------------------------------------- */

function toggleFailure(resultId: string) {
  const next = new Set(expandedFailures.value);

  if (next.has(resultId)) {
    next.delete(resultId);
  } else {
    next.add(resultId);
  }

  expandedFailures.value = next;
}

function isFailureExpanded(resultId: string) {
  return expandedFailures.value.has(resultId);
}

/** 展开区容器样式（按状态区分颜色） */
function resultDetailBoxClass(status: TestResult["status"]) {
  switch (status) {
    case "skipped":
      return "border-amber-200 bg-amber-50";
    case "xfailed":
      return "border-violet-200 bg-violet-50";
    case "xpassed":
      return "border-sky-200 bg-sky-50";
    default:
      return "border-rose-200 bg-rose-50";
  }
}

/** 展开区文字样式 */
function resultDetailTextClass(status: TestResult["status"]) {
  switch (status) {
    case "skipped":
      return "text-amber-800";
    case "xfailed":
      return "text-violet-800";
    case "xpassed":
      return "text-sky-800";
    default:
      return "text-rose-800";
  }
}

/** 展开区标题 */
function resultDetailTitle(status: TestResult["status"]) {
  switch (status) {
    case "skipped":
      return t("execute.results.skipDetails");
    case "xfailed":
      return t("execute.results.xfailDetails");
    case "xpassed":
      return t("execute.results.xpassDetails");
    default:
      return t("execute.results.failureDetails");
  }
}

/** 该结果是否有可展开的详情（failed/error 的错误信息，或 skipped/xfailed/xpassed 的详情） */
function hasResultDetail(result: TestResult): boolean {
  if (result.status === "failed" || result.status === "error") {
    return !!result.errorMessage;
  }
  if (
    result.status === "skipped" ||
    result.status === "xfailed" ||
    result.status === "xpassed"
  ) {
    return true; // 即使无原因也展开，展示「未提供原因」与文件:行号
  }
  return false;
}

/** 展开区展示的主文本：failed/error → 错误信息；skipped → 跳过原因；xfailed → 固定解释（+原因）；xpassed → 固定解释（+错误内容） */
function getResultDetail(result: TestResult): string | null {
  if (result.status === "failed" || result.status === "error") {
    return result.errorMessage;
  }
  if (result.status === "skipped") {
    return result.skipReason;
  }
  if (result.status === "xfailed") {
    const fixed = t("execute.results.xfailExplanation");
    return result.skipReason ? `${fixed}\n${result.skipReason}` : fixed;
  }
  if (result.status === "xpassed") {
    const fixed = t("execute.results.xpassExplanation");
    return result.errorMessage ? `${fixed}\n${result.errorMessage}` : fixed;
  }
  return null;
}

function getResultClass(status: TestResult["status"]) {
  switch (status) {
    case "passed":
      return "border-emerald-200 bg-emerald-50 text-emerald-700";

    case "failed":
    case "error":
      return "border-rose-200 bg-rose-50 text-rose-700";

    case "skipped":
      return "border-amber-200 bg-amber-50 text-amber-700";

    case "xfailed":
      return "border-violet-200 bg-violet-50 text-violet-700";

    case "xpassed":
      return "border-sky-200 bg-sky-50 text-sky-700";

    default:
      return "border-zinc-200 bg-zinc-50 text-zinc-600";
  }
}

function getResultLabel(status: TestResult["status"]) {
  switch (status) {
    case "passed":
      return t("execute.results.passed");
    case "failed":
      return t("execute.results.failed");
    case "error":
      return t("execute.results.error");
    case "skipped":
      return t("execute.results.skipped");
    case "xfailed":
      return t("execute.results.xfailed");
    case "xpassed":
      return t("execute.results.xpassed");
    default:
      return status;
  }
}

function getResultIcon(status: TestResult["status"]) {
  switch (status) {
    case "passed":
      return CheckCircle2;

    case "failed":
    case "error":
      return XCircle;

    case "skipped":
      return Clock3;

    case "xfailed":
      return AlertCircle;

    case "xpassed":
      return CheckCircle2;

    default:
      return AlertCircle;
  }
}

function rerunFailedTests() {
  if (isRunning.value || failedResults.value.length === 0) {
    return;
  }

  selectedTestCases.value = failedResults.value.map((result) => result.id);

  testScope.value = "selected";

  void runTests();
}

function clearOutput() {
  pytestOutput.value = [];
  pendingOutput.length = 0;
}

/** 把结果里的相对路径解析为绝对路径 */
function absTestPath(file: string): string {
  const projectPath = currentProject.value?.path;
  if (!projectPath) return file;
  return file.startsWith("/") || /^[A-Za-z]:/.test(file)
    ? file
    : `${projectPath}/${file}`;
}

/** 用系统编辑器打开文件（带行号；编辑器遵循设置里的 editorCommand） */
async function openTestFile(file: string, line: number | null) {
  if (!file) return;
  const full = absTestPath(file);
  try {
    await invoke("open_file", {
      path: full,
      editor: uiStore.editorCommand,
      line: line ?? undefined,
    });
  } catch (error) {
    console.error("[Execute] open_file failed:", error);
  }
}

function openResultFile(result: TestResult) {
  void openTestFile(result.file, result.line);
}

/** 复制绝对路径（带行号：/abs/path/test.py:12；无行号只复制路径） */
function copyPathWithLine(file: string, line: number | null) {
  const abs = absTestPath(file);
  void copyText(line ? `${abs}:${line}` : abs);
}

/** 在文件管理器中定位文件 */
async function revealInFileManager(file: string) {
  if (!file) return;
  try {
    await invoke("reveal_in_folder", { path: absTestPath(file) });
  } catch (error) {
    console.error("[Execute] reveal_in_folder failed:", error);
  }
}

/** 应用内预览：读取测试文件并在弹窗中显示，目标行高亮 */
const preview = ref<{ file: string; line: number | null } | null>(null);
const previewContent = ref("");
const previewLoading = ref(false);
const previewError = ref("");
const previewLines = computed(() => previewContent.value.split("\n"));

async function openPreview(file: string, line: number | null) {
  if (!file) return;
  const full = absTestPath(file);
  preview.value = { file: full, line };
  previewContent.value = "";
  previewError.value = "";
  previewLoading.value = true;
  try {
    previewContent.value = await invoke<string>("read_text_file", { path: full });
  } catch (error) {
    previewError.value = error instanceof Error ? error.message : String(error);
  } finally {
    previewLoading.value = false;
  }
}

/* -------------------------------------------------------------------------- */
/* Watchers / lifecycle                                                       */
/* -------------------------------------------------------------------------- */

watch(
  currentProject,
  async (project) => {
    if (!project) {
      return;
    }

    resetExecutionState();

    selectedTestCases.value = [];
    selectedTestFile.value = "";
    testSearch.value = "";

    await Promise.all([refreshTests(), loadSuites()]);

    // 消费"历史详情一键重跑失败用例"：选中失败用例并自动运行
    const pendingProject = uiStore.pendingRerunProjectId;
    const pendingIds = uiStore.pendingRerunTestIds;
    if (pendingProject !== null && pendingProject === project.id && pendingIds.length > 0) {
      const matched = testCases.value.filter((t) =>
        pendingIds.some(
          (pid) =>
            t.id === pid ||
            t.id.endsWith(`::${pid.split("::").pop()}`),
        ),
      );
      if (matched.length > 0) {
        selectedTestCases.value = matched.map((t) => t.id);
        testScope.value = "selected";
        await nextTick();
        void runTests();
      }
      uiStore.clearPendingRerun();
    }
  },
  {
    immediate: true,
  },
);

onMounted(() => {
  void setupTestListeners();
  window.addEventListener('testmate:focus-search', onFocusSearch);
});

// Ctrl+Enter 快速运行当前选中的测试
usePageShortcuts(
  {
    'ctrl+enter': () => {
      if (canRun.value) void runTests();
    },
    'ctrl+`': () => terminalRef.value?.toggle(),
  },
  () => !isRunning.value && testScope.value !== 'suite',
);

const terminalRef = ref<InstanceType<typeof TerminalPanel> | null>(null);

onUnmounted(() => {
  unlistenStarted?.();
  unlistenOutput?.();
  unlistenFinished?.();
  window.removeEventListener('testmate:focus-search', onFocusSearch);
});

const testSearchInput = ref<HTMLInputElement | null>(null);
function onFocusSearch() {
  testSearchInput.value?.focus();
}
</script>

<template>
  <div class="flex h-full min-h-0 gap-4 p-5">
    <!-- 左：主内容 -->
    <div class="flex min-w-0 flex-1 flex-col gap-4 overflow-auto">
    <!-- 页头 -->
    <header class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <div
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-zinc-100 text-zinc-500"
          >
            <Play class="h-4 w-4" />
          </div>
          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-zinc-900">{{ t("execute.title") }}</h1>
            <p class="mt-0.5 truncate text-xs text-zinc-500">
              {{
                currentProject?.name
                  ? t("execute.subtitle", { name: currentProject.name })
                  : t("execute.subtitle", { name: t("layout.notLoaded") })
              }}
            </p>
          </div>
        </div>
      </div>
      <StatusPill :status="statusPillStatus" :label="statusText" :pulse="isRunning" />
    </header>

    <!-- ══════════ 运行条（唯一主卡片） ══════════ -->
    <section class="card">
      <div class="px-5 py-4">
        <!-- 头部：标题 + 刷新 -->
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 class="text-sm font-semibold text-zinc-900">{{ t("execute.selectTests") }}</h2>
            <p class="mt-0.5 text-xs text-zinc-500">{{ t("execute.selectTestsDesc") }}</p>
          </div>
          <div class="flex items-center gap-2">
            <span class="rounded-md bg-zinc-100 px-2 py-1 font-mono text-[10px] text-zinc-500">
              {{ t("execute.results.testsTotal", { count: testCases.length }) }}
            </span>
            <AppButton
              variant="secondary"
              size="sm"
              :loading="isCollecting || isLoadingTests"
              :disabled="isRunning"
              @click="refreshTests"
            >
              <RefreshCw class="h-3.5 w-3.5" />
              {{ t("common.refresh") }}
            </AppButton>
          </div>
        </div>

        <!-- 范围：紧凑分段选择 -->
        <div class="mt-4 inline-flex rounded-lg border border-border bg-zinc-50 p-0.5">
          <button
            v-for="opt in scopeOptions"
            :key="opt.id"
            type="button"
            :disabled="isRunning"
            class="rounded-md px-3 py-1.5 text-xs font-medium transition disabled:cursor-not-allowed"
            :class="
              testScope === opt.id
                ? 'bg-white text-zinc-900 shadow-sm'
                : 'text-zinc-500 hover:text-zinc-800'
            "
            @click="setScope(opt.id)"
          >
            {{ opt.label }}
          </button>
        </div>

        <!-- 目标选择：文件下拉 -->
        <div v-if="testScope === 'file'" class="mt-3">
          <select
            v-model="selectedTestFile"
            :disabled="isRunning || testFiles.length === 0"
            class="input"
          >
            <option value="" disabled>{{ t("execute.chooseFile") }}</option>
            <option
              v-for="file in testFiles"
              :key="file.relativePath"
              :value="file.relativePath"
            >
              {{ file.relativePath }}
            </option>
          </select>
        </div>

        <!-- 目标选择：测试用例列表（搜索 + 多选） -->
        <div v-if="testScope === 'selected'" class="mt-3 rounded-lg border border-border">
          <div class="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
            <div class="relative min-w-0 flex-1">
              <Search
                class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400"
              />
              <input
                v-model="testSearch"
                ref="testSearchInput"
                type="text"
                :placeholder="t('execute.searchPlaceholder')"
                :disabled="isRunning"
                class="input h-8 pl-8 pr-3"
              />
            </div>
            <div class="flex shrink-0 items-center gap-1">
              <AppButton
                variant="ghost"
                size="sm"
                :disabled="isRunning || filteredTestCases.length === 0 || allVisibleSelected"
                @click="selectAllVisible"
              >
                {{ t("common.selectAll") }}
              </AppButton>
              <AppButton
                variant="ghost"
                size="sm"
                :disabled="isRunning || selectedTestCases.length === 0"
                @click="clearSelection"
              >
                {{ t("common.clear") }}
              </AppButton>
            </div>
          </div>

          <div class="max-h-56 overflow-auto">
            <div v-if="isCollecting" class="px-4 py-8 text-center text-sm text-zinc-500">
              {{ t("execute.collecting") }}
            </div>
            <div
              v-else-if="collectError"
              class="m-3 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2.5 text-xs text-rose-700"
            >
              {{ collectError.message }}
              <p v-if="collectError.hint" class="mt-1 text-[11px] text-rose-600">
                {{ collectError.hint }}
              </p>
            </div>
            <div v-else-if="testCases.length === 0" class="px-4 py-8 text-center">
              <div class="text-sm font-medium text-zinc-700">{{ t("execute.noTestCases") }}</div>
              <div class="mt-1 text-xs text-zinc-500">{{ t("execute.noTestCasesDesc") }}</div>
              <div class="mt-4 flex items-center justify-center gap-2">
                <AppButton variant="secondary" size="sm" :disabled="isRunning" @click="refreshTests">
                  <RefreshCw class="h-3.5 w-3.5" />
                  {{ t("common.refresh") }}
                </AppButton>
                <AppButton
                  variant="primary"
                  size="sm"
                  @click="router.push({ name: 'ProjectGenerate', params: { id: projectId } })"
                >
                  <WandSparkles class="h-3.5 w-3.5" />
                  {{ t("execute.goGenerate") }}
                </AppButton>
              </div>
            </div>
            <div v-else-if="filteredTestCases.length === 0" class="px-4 py-8 text-center text-sm text-zinc-500">
              {{ t("execute.noSearchResults") }}
            </div>
            <button
              v-for="test in filteredTestCases"
              :key="test.id"
              type="button"
              :disabled="isRunning"
              class="flex w-full items-center gap-3 border-b border-border px-3 py-2 text-left transition last:border-b-0 hover:bg-zinc-50 disabled:cursor-not-allowed"
              @click="toggleTest(test.id, $event)"
              @contextmenu.prevent="showTestCaseMenu(test, $event)"
            >
              <span
                class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition"
                :class="
                  isTestSelected(test.id)
                    ? 'border-brand-500 bg-brand-500 text-white'
                    : 'border-zinc-300 bg-white'
                "
              >
                <Check v-if="isTestSelected(test.id)" class="h-3 w-3" />
              </span>
              <FileCode2 class="h-4 w-4 shrink-0 text-zinc-400" />
              <span class="min-w-0 flex-1">
                <span class="block truncate text-[13px] font-medium text-zinc-800">
                  {{ test.name }}
                </span>
                <span class="mt-0.5 block truncate font-mono text-[10px] text-zinc-500">
                  {{ test.file }}
                  <template v-if="test.className">::{{ test.className }}</template>
                </span>
              </span>
            </button>
          </div>
        </div>

        <!-- 目标选择：已保存选择列表 -->
        <div v-if="testScope === 'suite'" class="mt-3 rounded-lg border border-border">
          <div v-if="suites.length === 0" class="px-4 py-6 text-center">
            <div class="text-sm font-medium text-zinc-700">{{ t("execute.noSuites") }}</div>
            <div class="mt-1 text-xs text-zinc-500">{{ t("execute.regressionSuitesDesc") }}</div>
          </div>
          <div v-else class="divide-y divide-border">
            <div
              v-for="suite in suites"
              :key="suite.id"
              class="flex items-center justify-between gap-3 px-3.5 py-2.5"
            >
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span class="truncate text-[13px] font-medium text-zinc-800">
                    {{ suite.suiteName }}
                  </span>
                  <!-- 已保存的实际参数（Q4：配置可见） -->
                  <span
                    v-if="suite.customParams && suite.customParams.length > 0"
                    class="hidden truncate rounded-full bg-zinc-100 px-2 py-0.5 font-mono text-[10px] text-zinc-500 sm:inline"
                  >
                    {{ suite.customParams.join(" ") }}
                  </span>
                </div>
                <div class="mt-0.5 truncate text-xs text-zinc-500">
                  {{ suiteTargetsLabel(suite) }}
                  · {{ suite.createdAt }}
                </div>
              </div>
              <div class="flex shrink-0 items-center gap-2">
                <AppButton
                  variant="secondary"
                  size="sm"
                  :disabled="isRunning || isRunningSuite"
                  @click="runSuite(suite)"
                >
                  <Play class="h-3.5 w-3.5" />
                  {{ t("common.run") }}
                </AppButton>
                <AppTooltip :content="t('execute.deleteSuiteTitle')" position="top">
                  <button
                    type="button"
                    :disabled="isRunning || isRunningSuite"
                    class="btn btn-danger btn-sm"
                    @click="deleteSuiteTarget = suite"
                  >
                    <Trash2 class="h-3.5 w-3.5" />
                  </button>
                </AppTooltip>
              </div>
            </div>
          </div>
        </div>

        <div
          v-if="testScanError"
          class="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2.5 text-xs text-amber-700"
        >
          {{ testScanError.message }}
          <p v-if="testScanError.hint" class="mt-1 text-[11px] text-amber-600">
            {{ testScanError.hint }}
          </p>
        </div>

        <!-- 底部：范围摘要 + 参数预览 + 主操作 -->
        <div class="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
          <div class="min-w-0">
            <div class="text-xs font-medium text-zinc-700">{{ selectedScopeDescription }}</div>
            <div class="mt-0.5 truncate font-mono text-[10px] text-zinc-400">
              {{ t("execute.argumentsLine") }} {{ activeArguments.join(" ") }}
            </div>
          </div>
          <div v-if="testScope !== 'suite'" class="flex shrink-0 items-center gap-2">
            <AppButton v-if="isRunning" variant="danger" @click="stopTests">
              <Square class="h-4 w-4" />
              {{ t("common.stop") }}
            </AppButton>
            <template v-else>
              <AppButton
                variant="secondary"
                :disabled="!canRun"
                :title="t('execute.saveAsSuite')"
                @click="openSaveSuiteModal"
              >
                <Save class="h-4 w-4" />
                {{ t("execute.saveAsSuite") }}
              </AppButton>
              <AppButton variant="primary" :disabled="!canRun" @click="runTests">
                <Play class="h-4 w-4" />
                {{ t("execute.runTests") }}
              </AppButton>
            </template>
          </div>
        </div>
      </div>

      <!-- Options 折叠：预设 + 自定义参数 -->
      <div class="border-t border-border px-5 py-3">
        <button
          type="button"
          class="flex items-center gap-2 text-xs font-medium text-zinc-600 transition hover:text-zinc-900"
          @click="showAdvanced = !showAdvanced"
        >
          <ChevronDown v-if="showAdvanced" class="h-4 w-4" />
          <ChevronRight v-else class="h-4 w-4" />
          {{ t("execute.runConfiguration") }}
        </button>

        <div v-if="showAdvanced" class="mt-3">
          <div class="grid grid-cols-2 gap-2">
            <button
              v-for="preset in presets"
              :key="preset.id"
              type="button"
              :disabled="isRunning"
              class="relative rounded-lg border px-3.5 py-2.5 text-left transition disabled:cursor-not-allowed"
              :class="
                selectedPreset === preset.id && !customArguments.trim()
                  ? 'border-brand-200 bg-brand-50'
                  : 'border-border hover:bg-zinc-50'
              "
              @click="
                selectedPreset = preset.id;
                customArguments = '';
              "
            >
              <div class="text-[13px] font-medium text-zinc-900">{{ preset.name }}</div>
              <div class="mt-0.5 text-[11px] leading-4 text-zinc-500">{{ preset.description }}</div>
              <div class="mt-1 font-mono text-[10px] text-zinc-400">{{ preset.args.join(" ") }}</div>
              <span class="absolute right-2 top-2" @click.stop>
                <AppHelpPopover :title="preset.name" :content="presetHelpMap[preset.id] ?? ''" />
              </span>
            </button>
          </div>

          <div class="mt-3 border-t border-border pt-3">
            <label class="label" for="custom-args">{{ t("execute.customArgs") }}</label>
            <input
              id="custom-args"
              v-model="customArguments"
              :disabled="isRunning"
              type="text"
              :placeholder="t('execute.argsPlaceholder')"
              class="input mt-1.5 font-mono"
            />
            <p class="mt-1 text-[11px] text-zinc-400">{{ t("execute.customArgsDesc") }}</p>
          </div>
        </div>
      </div>
    </section>

    <!-- ══════════ 运行监视器（空闲 / 运行中 / 完成 三态） ══════════ -->
    <section class="card">
      <!-- 空闲：教你下一步 -->
      <div
        v-if="!hasResult && !isRunning"
        class="flex min-h-64 flex-col items-center justify-center px-6 py-12 text-center"
      >
        <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-100 text-zinc-400">
          <ListChecks class="h-6 w-6" />
        </div>
        <h3 class="mt-4 text-sm font-semibold text-zinc-900">
          {{ t("execute.execution.readyToRun") }}
        </h3>
        <p class="mt-1 max-w-sm text-xs leading-5 text-zinc-500">
          {{ t("execute.execution.desc") }} {{ selectedScopeDescription }}
        </p>
      </div>

      <template v-else>
        <!-- 进度区 -->
        <div class="border-b border-border px-5 py-4">
          <div class="flex items-end justify-between gap-4">
            <div class="min-w-0">
              <div class="text-sm font-semibold text-zinc-900">
                {{ isRunning ? t("execute.execution.running") : t("execute.execution.completed") }}
              </div>
              <div class="mt-0.5 text-xs text-zinc-500">
                {{
                  isRunning
                    ? t("execute.execution.processed", {
                        completed: completedTests,
                        total: totalTests,
                      })
                    : lastRunSummary
                }}
              </div>
            </div>
            <div class="text-2xl font-semibold text-zinc-900">{{ progress }}%</div>
          </div>

          <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-100">
            <div
              class="h-full rounded-full transition-all duration-300"
              :class="executionStatus === 'failed' ? 'bg-rose-500' : 'bg-emerald-500'"
              :style="{ width: `${progress}%` }"
            />
          </div>

          <div
            v-if="isRunning && currentTest"
            class="mt-3 flex items-center gap-2 rounded-lg border border-sky-200 bg-sky-50 px-3 py-2"
          >
            <Clock3 class="h-3.5 w-3.5 shrink-0 text-sky-600" />
            <span class="truncate font-mono text-[11px] text-sky-800">{{ currentTest }}</span>
          </div>
        </div>

        <!-- 完成：结论横幅 + 统计 + 结果列表 -->
        <div v-if="hasResult" class="px-5 py-4">
          <div
            class="flex flex-wrap items-center justify-between gap-3 rounded-lg border px-4 py-3.5"
            :class="
              failedTests > 0 || executionStatus === 'failed'
                ? 'border-rose-200 bg-rose-50'
                : 'border-emerald-200 bg-emerald-50'
            "
          >
            <div class="flex min-w-0 items-start gap-3">
              <CheckCircle2
                v-if="executionStatus === 'completed' && failedTests === 0"
                class="mt-0.5 h-5 w-5 shrink-0 text-emerald-600"
              />
              <XCircle v-else class="mt-0.5 h-5 w-5 shrink-0 text-rose-600" />
              <div class="min-w-0">
                <div
                  class="text-sm font-semibold"
                  :class="
                    failedTests > 0 || executionStatus === 'failed'
                      ? 'text-rose-800'
                      : 'text-emerald-800'
                  "
                >
                  {{ resultConclusion.title }}
                </div>
                <div
                  class="mt-0.5 text-xs"
                  :class="
                    failedTests > 0 || executionStatus === 'failed'
                      ? 'text-rose-700'
                      : 'text-emerald-700'
                  "
                >
                  {{ resultConclusion.description }}
                </div>
              </div>
            </div>
            <AppButton
              v-if="failedResults.length > 0"
              variant="secondary"
              size="sm"
              :disabled="isRunning"
              @click="rerunFailedTests"
            >
              <RotateCcw class="h-3.5 w-3.5" />
              {{ t("execute.rerunFailed") }}
            </AppButton>
          </div>

          <!-- 统计条 -->
          <div class="mt-4 grid grid-cols-4 gap-3">
            <div class="rounded-lg border border-border p-3">
              <div class="text-[10px] text-zinc-500">{{ t("execute.results.total") }}</div>
              <div class="mt-0.5 text-xl font-semibold text-zinc-900">{{ totalResultCount }}</div>
            </div>
            <div class="rounded-lg border border-emerald-200 bg-emerald-50 p-3">
              <div class="text-[10px] text-emerald-700">{{ t("execute.results.passed") }}</div>
              <div class="mt-0.5 text-xl font-semibold text-emerald-700">{{ passedTests }}</div>
            </div>
            <div class="rounded-lg border border-rose-200 bg-rose-50 p-3">
              <div class="text-[10px] text-rose-700">{{ t("execute.results.failed") }}</div>
              <div class="mt-0.5 text-xl font-semibold text-rose-700">{{ failedTests }}</div>
            </div>
            <div class="rounded-lg border border-border bg-zinc-50 p-3">
              <div class="text-[10px] text-zinc-500">{{ t("execute.results.duration") }}</div>
              <div class="mt-0.5 text-xl font-semibold text-zinc-700">
                {{ executionDuration.toFixed(2) }}s
              </div>
            </div>
          </div>

          <!-- strict-xfail 意外通过提示 -->
          <div
            v-if="xpassedTests > 0"
            class="mt-3 flex items-start gap-2 rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-xs text-sky-800"
          >
            <AlertCircle class="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-600" />
            <span>{{ t("execute.results.xpassSummary", { count: xpassedTests }) }}</span>
          </div>

          <!-- 结果筛选 + 列表 -->
          <div v-if="testResults.length > 0" class="mt-4">
            <div class="flex items-center gap-1 border-b border-border pb-2">
              <button
                v-for="filter in resultFilters"
                :key="filter.id"
                type="button"
                class="rounded-md px-2.5 py-1 text-xs font-medium transition"
                :class="
                  resultFilter === filter.id
                    ? 'bg-zinc-900 text-white'
                    : 'text-zinc-500 hover:bg-zinc-100'
                "
                @click="resultFilter = filter.id"
              >
                {{ filter.label }}
              </button>
            </div>

            <div class="mt-1 divide-y divide-border">
              <div v-for="result in filteredResults" :key="result.id" class="py-2.5" @contextmenu.prevent="showResultMenu(result, $event)" @dblclick="openResultFile(result)">
                <div class="flex items-center gap-3">
                  <component
                    :is="getResultIcon(result.status)"
                    class="h-4 w-4 shrink-0"
                    :class="
                      result.status === 'passed'
                        ? 'text-emerald-500'
                        : result.status === 'skipped'
                          ? 'text-amber-500'
                          : result.status === 'xfailed'
                            ? 'text-violet-500'
                            : result.status === 'xpassed'
                              ? 'text-sky-500'
                              : 'text-rose-500'
                    "
                  />
                  <div class="min-w-0 flex-1">
                    <div class="truncate text-[13px] font-medium text-zinc-800">
                      {{ result.name }}
                    </div>
                    <div class="mt-0.5 truncate font-mono text-[10px] text-zinc-500">
                      {{ result.file }}
                    </div>
                  </div>
                  <span
                    class="hidden rounded-full border px-2 py-0.5 text-[10px] font-medium sm:inline-flex"
                    :class="getResultClass(result.status)"
                  >
                    {{ getResultLabel(result.status) }}
                  </span>
                  <span class="w-12 text-right font-mono text-[11px] text-zinc-400">
                    {{ result.duration.toFixed(2) }}s
                  </span>
                  <button
                    v-if="hasResultDetail(result)"
                    type="button"
                    class="flex h-7 w-7 items-center justify-center rounded-lg text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700"
                    @click="toggleFailure(result.id)"
                  >
                    <ChevronDown v-if="isFailureExpanded(result.id)" class="h-4 w-4" />
                    <ChevronRight v-else class="h-4 w-4" />
                  </button>
                </div>

                <div
                  v-if="hasResultDetail(result) && isFailureExpanded(result.id)"
                  class="mt-2 rounded-lg border p-3"
                  :class="resultDetailBoxClass(result.status)"
                >
                  <div class="flex items-center justify-between gap-3">
                    <span class="text-xs font-medium" :class="resultDetailTextClass(result.status)">
                      {{ resultDetailTitle(result.status) }}
                    </span>
                    <button
                      type="button"
                      class="inline-flex items-center gap-1.5 rounded-lg px-2 py-1 text-[11px] font-medium transition hover:bg-black/5"
                      :class="resultDetailTextClass(result.status)"
                      @click="copyText(getResultDetail(result) ?? '')"
                    >
                      <Copy class="h-3 w-3" />
                      {{ t("common.copy") }}
                    </button>
                  </div>
                  <div
                    v-if="
                      (result.status === 'skipped' ||
                        result.status === 'xfailed' ||
                        result.status === 'xpassed') &&
                      result.line
                    "
                    class="mt-1 font-mono text-[10px]"
                    :class="resultDetailTextClass(result.status)"
                  >
                    {{ result.file }}:{{ result.line }}
                  </div>
                  <pre
                    class="mt-2 max-h-56 overflow-auto whitespace-pre-wrap wrap-break-word font-mono text-[11px] leading-5"
                    :class="resultDetailTextClass(result.status)"
                    >{{ getResultDetail(result) || t("execute.results.noSkipReason") }}</pre
                  >
                </div>
              </div>
            </div>

            <div v-if="filteredResults.length === 0" class="py-6 text-center text-xs text-zinc-500">
              {{ t("execute.results.noResultsFilter") }}
            </div>
          </div>
        </div>
      </template>
    </section>
    </div><!-- /左：主内容 -->

    <!-- Save Selection Modal（统一 AppModal） -->
    <AppModal
      v-model:open="showSaveSuiteModal"
      :title="t('execute.saveSuiteModal.title')"
      :description="
        t('execute.suiteTargets', { count: executionTargets.length }) +
        ' · ' +
        (activeArguments.join(' ') || t('execute.saveSuiteModal.defaultArgs'))
      "
    >
      <label class="label" for="suite-name">
        {{ t("execute.saveSuiteModal.name") }}
        <input
          id="suite-name"
          v-model="newSuiteName"
          type="text"
          class="input mt-1.5"
          :placeholder="t('execute.saveSuiteModal.namePlaceholder')"
          @keyup.enter="saveSuite"
        />
      </label>

      <template #footer>
        <AppButton variant="ghost" @click="showSaveSuiteModal = false">{{ t("common.cancel") }}</AppButton>
        <AppButton
          variant="primary"
          :disabled="!newSuiteName.trim() || isSavingSuite"
          :loading="isSavingSuite"
          @click="saveSuite"
        >
          <Save class="h-4 w-4" />
          {{ t("execute.saveSuiteModal.save") }}
        </AppButton>
      </template>
    </AppModal>

    <!-- 运行完成弹窗（手动运行）：覆盖率 / 保存选择 / 稍后 -->
    <AppModal
      v-model:open="showPostRunModal"
      :title="t('execute.postRun.title')"
      :description="
        postRunSummary
          ? t('execute.postRun.description', {
              passed: postRunSummary.passed,
              failed: postRunSummary.failed,
              skipped: postRunSummary.skipped,
              duration: postRunSummary.duration.toFixed(2),
            })
          : ''
      "
    >
      <p class="text-xs leading-5 text-zinc-500">{{ t("execute.postRun.hint") }}</p>
      <template #footer>
        <AppButton variant="ghost" @click="showPostRunModal = false">
          {{ t("execute.postRun.notNow") }}
        </AppButton>
        <AppButton variant="secondary" @click="saveFromPostRun">
          <Save class="h-4 w-4" />
          {{ t("execute.postRun.saveSelection") }}
        </AppButton>
        <AppButton variant="primary" @click="runCoverageFromPostRun">
          <Gauge class="h-4 w-4" />
          {{ t("execute.postRun.runCoverage") }}
        </AppButton>
      </template>
    </AppModal>

    <!-- 删除已保存选择确认 -->
    <AppConfirmModal
      :open="deleteSuiteTarget !== null"
      :title="t('execute.deleteSuiteTitle')"
      :description="
        deleteSuiteTarget
          ? t('execute.deleteSuiteConfirm', { name: deleteSuiteTarget.suiteName })
          : ''
      "
      :confirm-label="t('common.delete')"
      @confirm="confirmDeleteSuite"
      @update:open="(open: boolean) => { if (!open) deleteSuiteTarget = null }"
    />

    <!-- 测试文件应用内预览 -->
    <AppModal
      :open="!!preview"
      :title="preview?.file ?? ''"
      @update:open="(open: boolean) => { if (!open) preview = null }"
    >
      <div v-if="previewLoading" class="flex items-center justify-center gap-2 py-10 text-sm text-zinc-400">
        <Loader2 class="h-4 w-4 animate-spin" />
        {{ t("common.loading") }}
      </div>
      <div
        v-else-if="previewError"
        class="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2.5 text-xs text-rose-700"
      >
        {{ previewError }}
      </div>
      <div v-else class="max-h-[60vh] overflow-auto rounded-lg border border-border bg-zinc-50">
        <div
          v-for="(lineText, i) in previewLines"
          :key="i"
          class="flex"
          :class="[preview?.line === i + 1 ? 'bg-amber-100' : '', i % 2 === 1 ? 'bg-zinc-50/50' : '']"
        >
          <span
            class="w-10 shrink-0 select-none border-r border-zinc-200 px-2 py-0.5 text-right font-mono text-[10px] text-zinc-400"
          >
            {{ i + 1 }}
          </span>
          <span class="min-w-0 flex-1 whitespace-pre px-3 py-0.5 font-mono text-[11px] leading-5 text-zinc-700">
            {{ lineText }}
          </span>
        </div>
      </div>
    </AppModal>

    <!-- 测试用例右键菜单 -->
    <AppContextMenu
      v-if="testCaseMenu"
      :items="testCaseMenuItems"
      :open="!!testCaseMenu"
      :x="testCaseMenu.x"
      :y="testCaseMenu.y"
      @close="testCaseMenu = null"
    />

    <!-- 测试结果右键菜单 -->
    <AppContextMenu
      v-if="resultMenu"
      :items="resultMenuItems"
      :open="!!resultMenu"
      :x="resultMenu.x"
      :y="resultMenu.y"
      @close="resultMenu = null"
    />

    <!-- 终端右侧面板 -->
    <TerminalPanel
      ref="terminalRef"
      :title="t('execute.output.title')"
      :lines="pytestOutput"
      :active="isRunning"
      width-key="execute"
      @clear="clearOutput"
    />
  </div>
</template>
