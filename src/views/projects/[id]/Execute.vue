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
  Clock3,
  Copy,
  FileCode2,
  ListChecks,
  Play,
  RefreshCw,
  RotateCcw,
  Search,
  SlidersHorizontal,
  Square,
  XCircle,
} from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";
import { parseArguments } from "../../../helper/execute";

const route = useRoute();
const projectStore = useProjectStore();

const projectId = computed(() => Number(route.params.id));

type ExecutionStatus = "idle" | "running" | "completed" | "failed";
type TestScope = "all" | "file" | "selected";
type ResultFilter = "all" | "passed" | "failed" | "skipped";

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
}

interface TestResult {
  id: string;
  name: string;
  file: string;
  status: "passed" | "failed" | "skipped" | "error";
  duration: number;
  errorMessage: string | null;
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
const testScanError = ref<string | null>(null);
const collectError = ref<string | null>(null);

const testSearch = ref("");
const resultFilter = ref<ResultFilter>("all");

const selectedPreset = ref("standard");
const showAdvanced = ref(false);
const customArguments = ref("");

const pytestOutput = ref<TestOutputEvent[]>([]);
const showPytestOutput = ref(false);

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

let unlistenStarted: UnlistenFn | undefined;
let unlistenOutput: UnlistenFn | undefined;
let unlistenFinished: UnlistenFn | undefined;

/* -------------------------------------------------------------------------- */
/* Presets                                                                    */
/* -------------------------------------------------------------------------- */

const presets: Preset[] = [
  {
    id: "standard",
    name: "Standard",
    description: "Readable test output for normal execution",
    args: ["-v"],
  },
  {
    id: "quick",
    name: "Quick",
    description: "Compact output for a quick test run",
    args: ["-q"],
  },
  {
    id: "debug",
    name: "Debug",
    description: "Show concise failure details",
    args: ["-v", "--tb=short"],
  },
  {
    id: "stop-on-failure",
    name: "Stop on Failure",
    description: "Stop after the first failed test",
    args: ["-v", "-x"],
  },
];

const activePreset = computed(() => {
  return presets.find((preset) => preset.id === selectedPreset.value);
});

const activeArguments = computed(() => {
  if (customArguments.value.trim()) {
    return parseArguments(customArguments.value);
  }

  return activePreset.value?.args ?? ["-v"];
});

/* -------------------------------------------------------------------------- */
/* Test selection                                                             */
/* -------------------------------------------------------------------------- */

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
      return `${testCases.value.length} collected tests`;
    case "file": {
      const file = testFiles.value.find(
        (item) => item.relativePath === selectedTestFile.value,
      );

      if (!file) {
        return "Select a test file";
      }

      const count = testCases.value.filter(
        (test) => test.file === file.relativePath,
      ).length;

      return `${count} tests in ${file.relativePath}`;
    }
    case "selected":
      return `${selectedCount.value} selected tests`;
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

  if (resultFilter.value === "failed") {
    return testResults.value.filter(
      (result) => result.status === "failed" || result.status === "error",
    );
  }

  return testResults.value.filter(
    (result) => result.status === resultFilter.value,
  );
});

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
      title: "Test Suite Passed",
      description: "All executed tests completed successfully.",
    };
  }

  if (executionStatus.value === "failed") {
    if (failedTests.value > 0) {
      return {
        title: "Test Suite Failed",
        description: `${failedTests.value} test${failedTests.value === 1 ? "" : "s"} failed during execution.`,
      };
    }

    return {
      title: "Execution Failed",
      description: "The test process did not complete successfully.",
    };
  }

  return {
    title: "Ready to Run",
    description: "Configure the test scope and run the selected tests.",
  };
});

const statusText = computed(() => {
  switch (executionStatus.value) {
    case "running":
      return "Running";
    case "completed":
      return failedTests.value > 0 ? "Failed" : "Passed";
    case "failed":
      return "Failed";
    default:
      return "Ready";
  }
});

const statusIcon = computed(() => {
  switch (executionStatus.value) {
    case "running":
      return Clock3;
    case "completed":
      return failedTests.value > 0 ? XCircle : CheckCircle2;
    case "failed":
      return XCircle;
    default:
      return AlertCircle;
  }
});

const statusClass = computed(() => {
  switch (executionStatus.value) {
    case "running":
      return "border-blue-200 bg-blue-50 text-blue-700";
    case "completed":
      return failedTests.value > 0
        ? "border-rose-200 bg-rose-50 text-rose-700"
        : "border-emerald-200 bg-emerald-50 text-emerald-700";
    case "failed":
      return "border-rose-200 bg-rose-50 text-rose-700";
    default:
      return "border-slate-200 bg-slate-50 text-slate-600";
  }
});

const hasResult = computed(() => {
  return (
    executionStatus.value === "completed" || executionStatus.value === "failed"
  );
});

const lastRunSummary = computed(() => {
  if (!hasResult.value) {
    return "No test execution completed yet";
  }

  return `${totalResultCount.value} tests · ${executionDuration.value.toFixed(2)}s`;
});

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
    expandedFailures.value = new Set();

    executionStatus.value = "running";
    isRunning.value = true;
    showPytestOutput.value = false;
  });

  unlistenOutput = await listen<TestOutputEvent>("test-output", (event) => {
    if (event.payload.runId !== currentRunId.value) {
      return;
    }

    pytestOutput.value.push(event.payload);
    parseRealtimeProgress(event.payload.line);
  });

  unlistenFinished = await listen<TestFinishedEvent>(
    "test-finished",
    (event) => {
      if (event.payload.runId !== currentRunId.value) {
        return;
      }

      isRunning.value = false;

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
    testScanError.value = "Project path is not available";
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
    testScanError.value = String(error);
    testFiles.value = [];
  } finally {
    isLoadingTests.value = false;
  }
}

async function collectTestCases() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) {
    collectError.value = "Project path is not available";
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

    collectError.value = String(error);
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

function toggleTest(testId: string) {
  if (isRunning.value) {
    return;
  }

  const index = selectedTestCases.value.indexOf(testId);

  if (index === -1) {
    selectedTestCases.value.push(testId);
  } else {
    selectedTestCases.value.splice(index, 1);
  }

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

function toggleVisibleSelection() {
  if (allVisibleSelected.value) {
    const visibleIds = new Set(filteredTestCases.value.map((test) => test.id));

    selectedTestCases.value = selectedTestCases.value.filter(
      (id) => !visibleIds.has(id),
    );
  } else {
    selectAllVisible();
  }
}

// function selectFile(file: TestFile) {
//   selectedTestFile.value = file.relativePath;
//   testScope.value = "file";
// }

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

    if (pytestOutput.value.length === 0) {
      pytestOutput.value.push({
        runId: currentRunId.value ?? "local",
        stream: "stderr",
        line: String(error),
      });
    }
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

  expandedFailures.value = new Set();

  executionStatus.value = "idle";
  showPytestOutput.value = false;
}

// function stopTests() {
//   if (!isRunning.value) {
//     return;
//   }

//   /*
//    * The current Rust implementation does not expose a process cancellation
//    * command yet. Keep the button disabled from pretending that pytest was
//    * actually terminated.
//    *
//    * A real stop action should call a dedicated Tauri command that owns the
//    * running Child process and terminates it.
//    */
//   console.warn(
//     "[Execute] Stop is not available until a process cancellation command is implemented.",
//   );
// }

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

function getResultClass(status: TestResult["status"]) {
  switch (status) {
    case "passed":
      return "border-emerald-200 bg-emerald-50 text-emerald-700";

    case "failed":
    case "error":
      return "border-rose-200 bg-rose-50 text-rose-700";

    case "skipped":
      return "border-amber-200 bg-amber-50 text-amber-700";

    default:
      return "border-slate-200 bg-slate-50 text-slate-600";
  }
}

function getResultLabel(status: TestResult["status"]) {
  switch (status) {
    case "passed":
      return "Passed";
    case "failed":
      return "Failed";
    case "error":
      return "Error";
    case "skipped":
      return "Skipped";
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

async function copyError(message: string | null) {
  if (!message) {
    return;
  }

  try {
    await navigator.clipboard.writeText(message);
  } catch (error) {
    console.error("[Execute] Failed to copy error:", error);
  }
}

async function copyLogs() {
  if (pytestOutput.value.length === 0) {
    return;
  }

  const text = pytestOutput.value.map((output) => output.line).join("\n");

  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    console.error("[Execute] Failed to copy pytest output:", error);
  }
}

function clearOutput() {
  pytestOutput.value = [];
  showPytestOutput.value = false;
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

    await refreshTests();
  },
  {
    immediate: true,
  },
);

onMounted(() => {
  void setupTestListeners();
});

onUnmounted(() => {
  unlistenStarted?.();
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
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-emerald-50 text-emerald-600"
          >
            <Play class="h-4 w-4" />
          </div>

          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-slate-900">Test Execution</h1>

            <p class="mt-0.5 truncate text-xs text-slate-500">
              {{
                currentProject?.name
                  ? `Run pytest tests for ${currentProject.name}`
                  : "Run pytest tests for the current project"
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
          :class="{ 'animate-spin': executionStatus === 'running' }"
        />

        {{ statusText }}
      </div>
    </header>

    <!-- Select Tests -->
    <section class="rounded-2xl border border-slate-200 bg-white">
      <div
        class="flex items-center justify-between gap-4 border-b border-slate-200 px-5 py-4"
      >
        <div>
          <div class="flex items-center gap-2">
            <ListChecks class="h-4 w-4 text-slate-500" />

            <h2 class="text-sm font-semibold text-slate-900">Select Tests</h2>
          </div>

          <p class="mt-1 text-xs text-slate-500">
            Choose which tests should be executed.
          </p>
        </div>

        <button
          type="button"
          :disabled="isCollecting || isLoadingTests || isRunning"
          class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
          @click="refreshTests"
        >
          <RefreshCw
            class="h-3.5 w-3.5"
            :class="{
              'animate-spin': isCollecting || isLoadingTests,
            }"
          />

          {{ isCollecting || isLoadingTests ? "Refreshing..." : "Refresh" }}
        </button>
      </div>

      <div class="p-5">
        <!-- Scope -->
        <div class="grid grid-cols-3 gap-2">
          <button
            type="button"
            :disabled="isRunning"
            class="rounded-xl border px-4 py-3 text-left transition disabled:cursor-not-allowed"
            :class="
              testScope === 'all'
                ? 'border-emerald-300 bg-emerald-50'
                : 'border-slate-200 hover:bg-slate-50'
            "
            @click="setScope('all')"
          >
            <div class="text-sm font-medium text-slate-900">All Tests</div>

            <div class="mt-1 text-xs text-slate-500">
              Run the complete test suite
            </div>
          </button>

          <button
            type="button"
            :disabled="isRunning"
            class="rounded-xl border px-4 py-3 text-left transition disabled:cursor-not-allowed"
            :class="
              testScope === 'file'
                ? 'border-emerald-300 bg-emerald-50'
                : 'border-slate-200 hover:bg-slate-50'
            "
            @click="setScope('file')"
          >
            <div class="text-sm font-medium text-slate-900">Test File</div>

            <div class="mt-1 text-xs text-slate-500">
              Run all tests from one file
            </div>
          </button>

          <button
            type="button"
            :disabled="isRunning"
            class="rounded-xl border px-4 py-3 text-left transition disabled:cursor-not-allowed"
            :class="
              testScope === 'selected'
                ? 'border-emerald-300 bg-emerald-50'
                : 'border-slate-200 hover:bg-slate-50'
            "
            @click="setScope('selected')"
          >
            <div class="text-sm font-medium text-slate-900">Selected Tests</div>

            <div class="mt-1 text-xs text-slate-500">
              Run only selected test cases
            </div>
          </button>
        </div>

        <!-- File selection -->
        <div v-if="testScope === 'file'" class="mt-4">
          <label class="mb-2 block text-xs font-medium text-slate-700">
            Test File
          </label>

          <select
            v-model="selectedTestFile"
            :disabled="isRunning || testFiles.length === 0"
            class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-emerald-400 focus:ring-2 focus:ring-emerald-100 disabled:bg-slate-50 disabled:text-slate-400"
          >
            <option value="" disabled>Select a test file</option>

            <option
              v-for="file in testFiles"
              :key="file.relativePath"
              :value="file.relativePath"
            >
              {{ file.relativePath }}
            </option>
          </select>
        </div>

        <!-- Test list -->
        <div class="mt-5 rounded-xl border border-slate-200">
          <div
            class="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 px-4 py-3"
          >
            <div class="flex min-w-0 items-center gap-3">
              <div class="relative">
                <Search
                  class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400"
                />

                <input
                  v-model="testSearch"
                  type="text"
                  placeholder="Search tests..."
                  :disabled="isRunning"
                  class="w-64 rounded-lg border border-slate-200 bg-white py-2 pl-9 pr-3 text-xs text-slate-700 outline-none transition placeholder:text-slate-400 focus:border-emerald-400 focus:ring-2 focus:ring-emerald-100 disabled:bg-slate-50"
                />
              </div>

              <span class="text-xs text-slate-500">
                {{ testCases.length }} tests
              </span>
            </div>

            <div class="flex items-center gap-2">
              <button
                type="button"
                :disabled="
                  isRunning ||
                  filteredTestCases.length === 0 ||
                  allVisibleSelected
                "
                class="rounded-lg px-2.5 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="selectAllVisible"
              >
                Select All
              </button>

              <button
                type="button"
                :disabled="isRunning || selectedTestCases.length === 0"
                class="rounded-lg px-2.5 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-40"
                @click="clearSelection"
              >
                Clear
              </button>
            </div>
          </div>

          <div
            class="flex items-center justify-between border-b border-slate-100 bg-slate-50 px-4 py-2.5"
          >
            <div class="flex items-center gap-2">
              <button
                type="button"
                :disabled="isRunning || filteredTestCases.length === 0"
                class="flex h-4 w-4 items-center justify-center rounded border transition disabled:cursor-not-allowed"
                :class="
                  allVisibleSelected
                    ? 'border-emerald-500 bg-emerald-500 text-white'
                    : 'border-slate-300 bg-white'
                "
                @click="toggleVisibleSelection"
              >
                <Check v-if="allVisibleSelected" class="h-3 w-3" />
              </button>

              <span class="text-xs font-medium text-slate-600">
                {{ selectedCount }} selected
              </span>
            </div>

            <span class="text-xs text-slate-500">
              {{ selectedScopeDescription }}
            </span>
          </div>

          <div
            v-if="isCollecting"
            class="px-5 py-10 text-center text-sm text-slate-500"
          >
            Collecting test cases...
          </div>

          <div
            v-else-if="collectError"
            class="m-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700"
          >
            {{ collectError }}
          </div>

          <div
            v-else-if="testCases.length === 0"
            class="px-5 py-10 text-center"
          >
            <div class="text-sm font-medium text-slate-700">
              No test cases found
            </div>

            <div class="mt-1 text-xs text-slate-500">
              Make sure pytest is installed and the project contains
              discoverable tests.
            </div>
          </div>

          <div
            v-else-if="filteredTestCases.length === 0"
            class="px-5 py-10 text-center text-sm text-slate-500"
          >
            No tests match your search.
          </div>

          <div v-else class="max-h-80 overflow-auto">
            <button
              v-for="test in filteredTestCases"
              :key="test.id"
              type="button"
              :disabled="isRunning"
              class="flex w-full items-center gap-3 border-b border-slate-100 px-4 py-3 text-left transition last:border-b-0 hover:bg-slate-50 disabled:cursor-not-allowed"
              @click="toggleTest(test.id)"
            >
              <span
                class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition"
                :class="
                  isTestSelected(test.id)
                    ? 'border-emerald-500 bg-emerald-500 text-white'
                    : 'border-slate-300 bg-white'
                "
              >
                <Check v-if="isTestSelected(test.id)" class="h-3 w-3" />
              </span>

              <FileCode2 class="h-4 w-4 shrink-0 text-slate-400" />

              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm font-medium text-slate-800">
                  {{ test.name }}
                </span>

                <span
                  class="mt-0.5 block truncate font-mono text-[11px] text-slate-500"
                >
                  {{ test.file }}
                  <template v-if="test.className">
                    ::{{ test.className }}
                  </template>
                </span>
              </span>
            </button>
          </div>
        </div>

        <div
          v-if="testScanError"
          class="mt-3 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700"
        >
          {{ testScanError }}
        </div>
      </div>
    </section>

    <!-- Run Configuration -->
    <section class="rounded-2xl border border-slate-200 bg-white">
      <div class="border-b border-slate-200 px-5 py-4">
        <div class="flex items-center gap-2">
          <SlidersHorizontal class="h-4 w-4 text-slate-500" />

          <h2 class="text-sm font-semibold text-slate-900">
            Run Configuration
          </h2>
        </div>

        <p class="mt-1 text-xs text-slate-500">
          Choose a simple pytest configuration or provide advanced arguments.
        </p>
      </div>

      <div class="p-5">
        <div class="grid grid-cols-2 gap-3">
          <button
            v-for="preset in presets"
            :key="preset.id"
            type="button"
            :disabled="isRunning"
            class="rounded-xl border p-4 text-left transition disabled:cursor-not-allowed"
            :class="
              selectedPreset === preset.id && !customArguments.trim()
                ? 'border-emerald-300 bg-emerald-50'
                : 'border-slate-200 hover:bg-slate-50'
            "
            @click="
              selectedPreset = preset.id;
              customArguments = '';
            "
          >
            <div class="flex items-center justify-between gap-3">
              <span class="text-sm font-medium text-slate-900">
                {{ preset.name }}
              </span>

              <span
                v-if="selectedPreset === preset.id && !customArguments.trim()"
                class="text-xs font-medium text-emerald-600"
              >
                Selected
              </span>
            </div>

            <div class="mt-1 text-xs leading-5 text-slate-500">
              {{ preset.description }}
            </div>

            <div class="mt-2 font-mono text-[11px] text-slate-400">
              {{ preset.args.join(" ") }}
            </div>
          </button>
        </div>

        <div class="mt-4 border-t border-slate-100 pt-4">
          <button
            type="button"
            class="flex items-center gap-2 text-xs font-medium text-slate-600 transition hover:text-slate-900"
            @click="showAdvanced = !showAdvanced"
          >
            <ChevronDown v-if="showAdvanced" class="h-4 w-4" />

            <ChevronRight v-else class="h-4 w-4" />

            Advanced Options
          </button>

          <div v-if="showAdvanced" class="mt-4">
            <label class="mb-1.5 block text-xs font-medium text-slate-700">
              Custom pytest arguments
            </label>

            <input
              v-model="customArguments"
              :disabled="isRunning"
              type="text"
              placeholder="Example: --maxfail=3 --tb=long"
              class="w-full rounded-xl border border-slate-200 px-3 py-2.5 font-mono text-xs text-slate-700 outline-none transition placeholder:font-sans placeholder:text-slate-400 focus:border-emerald-400 focus:ring-2 focus:ring-emerald-100 disabled:bg-slate-50"
            />

            <p class="mt-1.5 text-[11px] text-slate-400">
              Custom arguments override the selected preset.
            </p>
          </div>
        </div>

        <div
          class="mt-5 flex items-center justify-between border-t border-slate-100 pt-4"
        >
          <div>
            <div class="text-xs font-medium text-slate-700">
              {{ selectedScopeDescription }}
            </div>

            <div class="mt-1 text-[11px] text-slate-400">
              Arguments:
              <span class="font-mono">
                {{ activeArguments.join(" ") }}
              </span>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <button
              v-if="isRunning"
              type="button"
              disabled
              class="inline-flex cursor-not-allowed items-center gap-2 rounded-xl bg-slate-200 px-5 py-2.5 text-sm font-medium text-slate-500"
              title="Process cancellation is not implemented yet"
            >
              <Square class="h-4 w-4" />
              Running
            </button>

            <button
              v-else
              type="button"
              :disabled="!canRun"
              class="inline-flex items-center gap-2 rounded-xl bg-emerald-500 px-5 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-600 disabled:cursor-not-allowed disabled:opacity-50"
              @click="runTests"
            >
              <Play class="h-4 w-4" />
              Run Tests
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- Execution -->
    <section class="rounded-2xl border border-slate-200 bg-white">
      <div
        class="flex items-center justify-between gap-4 border-b border-slate-200 px-5 py-4"
      >
        <div>
          <div class="text-sm font-semibold text-slate-900">Execution</div>

          <div class="mt-1 text-xs text-slate-500">
            Monitor the current test run.
          </div>
        </div>

        <div v-if="isRunning" class="text-xs font-medium text-blue-600">
          {{ completedTests }} / {{ totalTests }}
        </div>
      </div>

      <div class="p-5">
        <div
          v-if="executionStatus === 'idle'"
          class="rounded-xl border border-dashed border-slate-200 px-5 py-10 text-center"
        >
          <div class="text-sm font-medium text-slate-700">Ready to run</div>

          <div class="mt-1 text-xs text-slate-500">
            {{ selectedScopeDescription }}
          </div>
        </div>

        <div v-else>
          <div class="flex items-end justify-between gap-4">
            <div>
              <div class="text-sm font-semibold text-slate-900">
                {{
                  executionStatus === "running"
                    ? "Running tests..."
                    : "Execution completed"
                }}
              </div>

              <div class="mt-1 text-xs text-slate-500">
                {{ completedTests }} of {{ totalTests }} tests processed
              </div>
            </div>

            <div class="text-2xl font-semibold text-slate-900">
              {{ progress }}%
            </div>
          </div>

          <div class="mt-4 h-2 overflow-hidden rounded-full bg-slate-100">
            <div
              class="h-full rounded-full transition-all duration-300"
              :class="
                executionStatus === 'failed' ? 'bg-rose-500' : 'bg-emerald-500'
              "
              :style="{ width: `${progress}%` }"
            />
          </div>

          <div class="mt-5 grid grid-cols-3 gap-3">
            <div class="rounded-xl border border-emerald-100 bg-emerald-50 p-4">
              <div class="text-xs text-emerald-700">Passed</div>

              <div class="mt-1 text-xl font-semibold text-emerald-700">
                {{ passedTests }}
              </div>
            </div>

            <div class="rounded-xl border border-rose-100 bg-rose-50 p-4">
              <div class="text-xs text-rose-700">Failed</div>

              <div class="mt-1 text-xl font-semibold text-rose-700">
                {{ failedTests }}
              </div>
            </div>

            <div class="rounded-xl border border-slate-200 bg-slate-50 p-4">
              <div class="text-xs text-slate-600">Skipped</div>

              <div class="mt-1 text-xl font-semibold text-slate-700">
                {{ skippedTests }}
              </div>
            </div>
          </div>

          <div
            v-if="isRunning && currentTest"
            class="mt-5 rounded-xl border border-blue-100 bg-blue-50 px-4 py-3"
          >
            <div
              class="text-[10px] font-semibold uppercase tracking-wide text-blue-500"
            >
              Current Test
            </div>

            <div class="mt-1 truncate font-mono text-xs text-blue-800">
              {{ currentTest }}
            </div>
          </div>

          <div
            v-if="!isRunning"
            class="mt-5 flex items-center justify-between border-t border-slate-100 pt-4"
          >
            <span class="text-xs text-slate-500">
              Duration:
              <span class="font-medium text-slate-700">
                {{ executionDuration.toFixed(2) }}s
              </span>
            </span>

            <span class="text-xs text-slate-400">
              {{ lastRunSummary }}
            </span>
          </div>
        </div>
      </div>
    </section>

    <!-- Result -->
    <section
      v-if="hasResult"
      class="rounded-2xl border border-slate-200 bg-white"
    >
      <div class="border-b border-slate-200 px-5 py-4">
        <div class="flex items-center justify-between gap-4">
          <div>
            <div class="text-sm font-semibold text-slate-900">Result</div>

            <div class="mt-1 text-xs text-slate-500">
              Review the conclusion and individual test results.
            </div>
          </div>

          <button
            v-if="failedResults.length > 0"
            type="button"
            :disabled="isRunning"
            class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
            @click="rerunFailedTests"
          >
            <RotateCcw class="h-3.5 w-3.5" />
            Rerun Failed
          </button>
        </div>
      </div>

      <div class="p-5">
        <!-- Conclusion -->
        <div
          class="rounded-2xl border p-5"
          :class="
            failedTests > 0 || executionStatus === 'failed'
              ? 'border-rose-200 bg-rose-50'
              : 'border-emerald-200 bg-emerald-50'
          "
        >
          <div class="flex items-start gap-3">
            <CheckCircle2
              v-if="executionStatus === 'completed' && failedTests === 0"
              class="mt-0.5 h-5 w-5 shrink-0 text-emerald-600"
            />

            <XCircle v-else class="mt-0.5 h-5 w-5 shrink-0 text-rose-600" />

            <div>
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
                class="mt-1 text-xs"
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
        </div>

        <!-- Summary -->
        <div class="mt-5 grid grid-cols-4 gap-3">
          <div class="rounded-xl border border-slate-200 p-4">
            <div class="text-xs text-slate-500">Total</div>

            <div class="mt-1 text-xl font-semibold text-slate-900">
              {{ totalResultCount }}
            </div>
          </div>

          <div class="rounded-xl border border-emerald-200 bg-emerald-50 p-4">
            <div class="text-xs text-emerald-700">Passed</div>

            <div class="mt-1 text-xl font-semibold text-emerald-700">
              {{ passedTests }}
            </div>
          </div>

          <div class="rounded-xl border border-rose-200 bg-rose-50 p-4">
            <div class="text-xs text-rose-700">Failed</div>

            <div class="mt-1 text-xl font-semibold text-rose-700">
              {{ failedTests }}
            </div>
          </div>

          <div class="rounded-xl border border-slate-200 bg-slate-50 p-4">
            <div class="text-xs text-slate-600">Duration</div>

            <div class="mt-1 text-xl font-semibold text-slate-700">
              {{ executionDuration.toFixed(2) }}s
            </div>
          </div>
        </div>

        <!-- Result filters -->
        <div
          v-if="testResults.length > 0"
          class="mt-5 flex items-center gap-2 border-b border-slate-100 pb-3"
        >
          <button
            v-for="filter in [
              { id: 'all', label: `All ${testResults.length}` },
              { id: 'passed', label: `Passed ${passedTests}` },
              { id: 'failed', label: `Failed ${failedTests}` },
              { id: 'skipped', label: `Skipped ${skippedTests}` },
            ]"
            :key="filter.id"
            type="button"
            class="rounded-lg px-3 py-1.5 text-xs font-medium transition"
            :class="
              resultFilter === filter.id
                ? 'bg-slate-900 text-white'
                : 'text-slate-500 hover:bg-slate-100'
            "
            @click="resultFilter = filter.id as ResultFilter"
          >
            {{ filter.label }}
          </button>
        </div>

        <!-- Failed tests -->
        <div
          v-if="filteredResults.length > 0"
          class="mt-3 divide-y divide-slate-100"
        >
          <div v-for="result in filteredResults" :key="result.id" class="py-3">
            <div class="flex items-center gap-3">
              <component
                :is="getResultIcon(result.status)"
                class="h-4 w-4 shrink-0"
                :class="
                  result.status === 'passed'
                    ? 'text-emerald-500'
                    : result.status === 'skipped'
                      ? 'text-amber-500'
                      : 'text-rose-500'
                "
              />

              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-medium text-slate-800">
                  {{ result.name }}
                </div>

                <div
                  class="mt-0.5 truncate font-mono text-[11px] text-slate-500"
                >
                  {{ result.file }}
                </div>
              </div>

              <span
                class="hidden rounded-full border px-2 py-1 text-[10px] font-medium sm:inline-flex"
                :class="getResultClass(result.status)"
              >
                {{ getResultLabel(result.status) }}
              </span>

              <span class="w-12 text-right text-xs text-slate-400">
                {{ result.duration.toFixed(2) }}s
              </span>

              <button
                v-if="
                  result.errorMessage &&
                  (result.status === 'failed' || result.status === 'error')
                "
                type="button"
                class="flex h-7 w-7 items-center justify-center rounded-lg text-slate-400 transition hover:bg-slate-100 hover:text-slate-700"
                @click="toggleFailure(result.id)"
              >
                <ChevronDown
                  v-if="isFailureExpanded(result.id)"
                  class="h-4 w-4"
                />

                <ChevronRight v-else class="h-4 w-4" />
              </button>
            </div>

            <div
              v-if="
                result.errorMessage &&
                (result.status === 'failed' || result.status === 'error') &&
                isFailureExpanded(result.id)
              "
              class="mt-3 rounded-xl border border-rose-200 bg-rose-50 p-3"
            >
              <div class="flex items-center justify-between gap-3">
                <span class="text-xs font-medium text-rose-800">
                  Failure Details
                </span>

                <button
                  type="button"
                  class="inline-flex items-center gap-1.5 rounded-lg px-2 py-1 text-[11px] font-medium text-rose-700 transition hover:bg-rose-100"
                  @click="copyError(result.errorMessage)"
                >
                  <Copy class="h-3 w-3" />
                  Copy
                </button>
              </div>

              <pre
                class="mt-2 max-h-64 overflow-auto whitespace-pre-wrap wrap-break-word font-mono text-[11px] leading-5 text-rose-800"
                >{{ result.errorMessage }}</pre>
            </div>
          </div>
        </div>

        <div v-else class="py-8 text-center text-xs text-slate-500">
          No results match the current filter.
        </div>
      </div>
    </section>

    <!-- Pytest Output -->
    <section
      v-if="pytestOutput.length > 0"
      class="rounded-2xl border border-slate-200 bg-white"
    >
      <button
        type="button"
        class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left"
        @click="showPytestOutput = !showPytestOutput"
      >
        <div class="flex min-w-0 items-center gap-3">
          <div
            class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-slate-100"
          >
            <SlidersHorizontal class="h-4 w-4 text-slate-500" />
          </div>

          <div class="min-w-0">
            <div class="text-sm font-medium text-slate-800">Pytest Output</div>

            <div class="mt-1 text-xs text-slate-500">
              {{ pytestOutput.length }} log lines
            </div>
          </div>
        </div>

        <ChevronDown
          class="h-4 w-4 shrink-0 text-slate-400 transition-transform"
          :class="{
            'rotate-180': showPytestOutput,
          }"
        />
      </button>

      <div v-if="showPytestOutput" class="border-t border-slate-200">
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
            class="max-h-125 overflow-auto whitespace-pre-wrap wrap-break-word font-mono text-xs leading-5"
          ><span
            v-for="(output, index) in pytestOutput"
            :key="index"
            :class="
              output.stream === 'stderr'
                ? 'text-rose-300'
                : 'text-slate-300'
            "
          >{{ output.line }}
{{ "\n" }}</span></pre>
        </div>
      </div>
    </section>
  </div>
</template>
