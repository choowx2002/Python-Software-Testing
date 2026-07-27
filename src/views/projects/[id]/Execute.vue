<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, ref, watch } from "vue";
import { useRoute } from "vue-router";
import {
  Play,
  Square,
  RefreshCw,
  Terminal,
  CheckCircle2,
  XCircle,
  Clock3,
  AlertCircle,
  ChevronDown,
  ChevronRight,
  FileCode2,
} from "@lucide/vue";
import { useProjectStore } from "../../../stores/projectStore";

const route = useRoute();
const projectStore = useProjectStore();
const projectId = computed(() => Number(route.params.id));

const isRunning = ref(false);
const selectedTestPath = ref("");
const testMode = ref<"all" | "file" | "test">("all");
const showAdvanced = ref(false);

const commandOutput = ref<string[]>([]);
const executionStatus = ref<"idle" | "running" | "passed" | "failed">("idle");

const statistics = ref({
  total: 0,
  passed: 0,
  failed: 0,
  skipped: 0,
  duration: 0,
});

interface TestFile {
  name: string;
  path: string;
  relativePath: string;
}

const testFiles = ref<TestFile[]>([]);
const isLoadingTests = ref(false);
const testScanError = ref<string | null>(null);

interface TestCase {
  id: string;
  name: string;
  file: string;
  className: string | null;
  line: number | null;
}

const testCases = ref<TestCase[]>([]);
const isCollecting = ref(false);
const collectError = ref<string | null>(null);

const currentProject = computed(() => {
  return projectStore.projects.find(
    (project) => project.id === projectId.value,
  );
});

watch(
  currentProject,
  (project) => {
    if (!project) return;

    collectTestCases();
  },
  {
    immediate: true,
  },
);

const statusText = computed(() => {
  switch (executionStatus.value) {
    case "running":
      return "Running";
    case "passed":
      return "Passed";
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
    case "passed":
      return CheckCircle2;
    case "failed":
      return XCircle;
    default:
      return AlertCircle;
  }
});

const statusClass = computed(() => {
  switch (executionStatus.value) {
    case "running":
      return "bg-blue-50 text-blue-700 border-blue-200";
    case "passed":
      return "bg-emerald-50 text-emerald-700 border-emerald-200";
    case "failed":
      return "bg-rose-50 text-rose-700 border-rose-200";
    default:
      return "bg-slate-50 text-slate-600 border-slate-200";
  }
});

async function runTests() {
  if (isRunning.value) return;

  isRunning.value = true;
  executionStatus.value = "running";
  commandOutput.value = [];
  statistics.value = {
    total: 0,
    passed: 0,
    failed: 0,
    skipped: 0,
    duration: 0,
  };

  commandOutput.value.push(
    `Project ID: ${projectId.value}`,
    "Preparing test environment...",
    "Starting pytest...",
    "",
  );

  // TODO:
  // 后续这里接 Tauri Rust command
  //
  // await invoke('run_tests', {
  //   projectId: projectId.value,
  //   mode: testMode.value,
  //   path: selectedTestPath.value,
  // })

  await new Promise((resolve) => setTimeout(resolve, 1200));

  commandOutput.value.push(
    "============================= test session starts =============================",
    "collected 35 items",
    "",
    "tests/test_example.py ............",
    "tests/test_utils.py ........",
    "tests/test_parser.py ...............",
    "",
    "============================== 35 passed in 1.24s ==============================",
  );

  statistics.value = {
    total: 35,
    passed: 35,
    failed: 0,
    skipped: 0,
    duration: 1.24,
  };

  executionStatus.value = "passed";
  isRunning.value = false;
}

function stopTests() {
  if (!isRunning.value) return;

  // TODO:
  // 后续接 Tauri command 停止当前 process
  //
  // await invoke('stop_test_execution')

  commandOutput.value.push("");
  commandOutput.value.push("Execution stopped by user.");

  executionStatus.value = "failed";
  isRunning.value = false;
}

function clearOutput() {
  commandOutput.value = [];
}

function selectTestFile(path: string) {
  selectedTestPath.value = path;
  testMode.value = "file";
}

async function scanTestFiles() {
  const projectPath = currentProject.value?.path;

  if (!projectPath) {
    testScanError.value = "项目路径不存在";
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
  } catch (error) {
    console.error("[Execute] collect_test_cases failed:", error);

    collectError.value = String(error);
    testCases.value = [];
  } finally {
    isCollecting.value = false;
  }
}
</script>

<template>
  <div class="flex min-h-full flex-col gap-5">
    <!-- Header -->
    <div class="flex items-start justify-between gap-4">
      <div>
        <div class="flex items-center gap-2">
          <div
            class="flex h-9 w-9 items-center justify-center rounded-xl bg-emerald-50 text-emerald-600"
          >
            <Play class="h-4 w-4" />
          </div>

          <div>
            <h2 class="text-lg font-semibold text-slate-900">Test Execution</h2>
            <p class="text-xs text-slate-500">
              Run pytest against the current project
            </p>
          </div>
        </div>
      </div>

      <div
        class="inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium"
        :class="statusClass"
      >
        <component
          :is="statusIcon"
          class="h-3.5 w-3.5"
          :class="{ 'animate-spin': executionStatus === 'running' }"
        />
        {{ statusText }}
      </div>
    </div>

    <!-- Execution settings -->
    <section class="rounded-2xl border border-slate-200 bg-white">
      <div class="border-b border-slate-200 px-5 py-4">
        <div class="text-sm font-semibold text-slate-900">
          Execution Settings
        </div>
        <div class="mt-0.5 text-xs text-slate-500">
          Configure what you want to execute
        </div>
      </div>

      <div class="space-y-5 p-5">
        <!-- Test mode -->
        <div>
          <label class="mb-2 block text-xs font-medium text-slate-700">
            Test Scope
          </label>

          <div class="grid grid-cols-3 gap-2">
            <button
              type="button"
              @click="testMode = 'all'"
              class="rounded-xl border px-4 py-3 text-left transition"
              :class="
                testMode === 'all'
                  ? 'border-emerald-300 bg-emerald-50'
                  : 'border-slate-200 hover:bg-slate-50'
              "
            >
              <div class="text-sm font-medium text-slate-900">All Tests</div>
              <div class="mt-1 text-xs text-slate-500">
                Run entire test suite
              </div>
            </button>

            <button
              type="button"
              @click="testMode = 'file'"
              class="rounded-xl border px-4 py-3 text-left transition"
              :class="
                testMode === 'file'
                  ? 'border-emerald-300 bg-emerald-50'
                  : 'border-slate-200 hover:bg-slate-50'
              "
            >
              <div class="text-sm font-medium text-slate-900">Test File</div>
              <div class="mt-1 text-xs text-slate-500">Run selected file</div>
            </button>

            <button
              type="button"
              @click="testMode = 'test'"
              class="rounded-xl border px-4 py-3 text-left transition"
              :class="
                testMode === 'test'
                  ? 'border-emerald-300 bg-emerald-50'
                  : 'border-slate-200 hover:bg-slate-50'
              "
            >
              <div class="text-sm font-medium text-slate-900">Single Test</div>
              <div class="mt-1 text-xs text-slate-500">
                Run one test function
              </div>
            </button>
          </div>
        </div>

        <!-- Test file -->
        <div v-if="testMode === 'file'">
          <label class="mb-2 block text-xs font-medium text-slate-700">
            Test File
          </label>

          <select
            v-model="selectedTestPath"
            class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm text-slate-700 outline-none transition focus:border-emerald-400 focus:ring-2 focus:ring-emerald-100"
          >
            <option value="" disabled>Select a test file</option>

            <option
              v-for="file in testFiles"
              :key="file.path"
              :value="file.path"
            >
              {{ file.path }}
            </option>
          </select>
        </div>

        <!-- Advanced -->
        <div class="border-t border-slate-100 pt-4">
          <button
            type="button"
            class="flex items-center gap-2 text-xs font-medium text-slate-600 hover:text-slate-900"
            @click="showAdvanced = !showAdvanced"
          >
            <ChevronDown v-if="showAdvanced" class="h-4 w-4" />
            <ChevronRight v-else class="h-4 w-4" />
            Advanced Options
          </button>

          <div v-if="showAdvanced" class="mt-4 grid grid-cols-2 gap-4">
            <div>
              <label class="mb-1.5 block text-xs text-slate-600">
                Pytest Arguments
              </label>
              <input
                type="text"
                placeholder="-v --tb=short"
                class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm outline-none focus:border-emerald-400 focus:ring-2 focus:ring-emerald-100"
              />
            </div>

            <div>
              <label class="mb-1.5 block text-xs text-slate-600">
                Test Directory
              </label>
              <input
                type="text"
                value="tests"
                class="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm outline-none focus:border-emerald-400 focus:ring-2 focus:ring-emerald-100"
              />
            </div>
          </div>
        </div>

        <!-- Actions -->
        <div
          class="flex items-center justify-between border-t border-slate-100 pt-4"
        >
          <button
            type="button"
            class="inline-flex items-center gap-2 rounded-xl px-3 py-2 text-xs font-medium text-slate-500 hover:bg-slate-100 hover:text-slate-700"
            @click="clearOutput"
          >
            <RefreshCw class="h-3.5 w-3.5" />
            Clear Output
          </button>

          <div class="flex items-center gap-2">
            <button
              v-if="isRunning"
              type="button"
              @click="stopTests"
              class="inline-flex items-center gap-2 rounded-xl bg-rose-500 px-5 py-2.5 text-sm font-medium text-white transition hover:bg-rose-600"
            >
              <Square class="h-4 w-4" />
              Stop
            </button>

            <button
              v-else
              type="button"
              @click="runTests"
              class="inline-flex items-center gap-2 rounded-xl bg-emerald-500 px-5 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-600"
            >
              <Play class="h-4 w-4" />
              Run Tests
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- Statistics -->
    <section class="grid grid-cols-4 gap-3">
      <div class="rounded-2xl border border-slate-200 bg-white p-4">
        <div class="text-xs text-slate-500">Total</div>
        <div class="mt-1 text-2xl font-semibold text-slate-900">
          {{ statistics.total }}
        </div>
      </div>

      <div class="rounded-2xl border border-emerald-200 bg-emerald-50 p-4">
        <div class="text-xs text-emerald-700">Passed</div>
        <div class="mt-1 text-2xl font-semibold text-emerald-700">
          {{ statistics.passed }}
        </div>
      </div>

      <div class="rounded-2xl border border-rose-200 bg-rose-50 p-4">
        <div class="text-xs text-rose-700">Failed</div>
        <div class="mt-1 text-2xl font-semibold text-rose-700">
          {{ statistics.failed }}
        </div>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-4">
        <div class="text-xs text-slate-500">Duration</div>
        <div class="mt-1 text-2xl font-semibold text-slate-900">
          {{ statistics.duration.toFixed(2) }}s
        </div>
      </div>
    </section>

    <!-- Output -->
    <section
      class="min-h-80 flex-1 overflow-hidden rounded-2xl border border-slate-200 bg-slate-950"
    >
      <div
        class="flex items-center justify-between border-b border-slate-800 px-4 py-3"
      >
        <div class="flex items-center gap-2">
          <Terminal class="h-4 w-4 text-slate-400" />
          <span class="text-xs font-medium text-slate-300"> Test Output </span>
        </div>

        <span class="font-mono text-[10px] text-slate-500"> pytest </span>
      </div>

      <div class="h-75 overflow-auto p-4 font-mono text-xs leading-6">
        <div v-if="commandOutput.length === 0" class="text-slate-600">
          No test execution yet.
        </div>

        <template v-else>
          <div
            v-for="(line, index) in commandOutput"
            :key="index"
            class="whitespace-pre-wrap"
            :class="
              line.includes('passed')
                ? 'text-emerald-400'
                : line.includes('failed')
                  ? 'text-rose-400'
                  : 'text-slate-300'
            "
          >
            {{ line || " " }}
          </div>
        </template>
      </div>
    </section>

    <!-- Test files -->
    <section class="rounded-2xl border border-slate-200 bg-white">
      <div
        class="flex items-center justify-between border-b border-slate-200 px-5 py-4"
      >
        <div>
          <div class="text-sm font-semibold text-slate-900">Test Cases</div>

          <div class="mt-0.5 text-xs text-slate-500">
            {{ testCases.length }} test cases collected
          </div>
        </div>

        <button
          type="button"
          :disabled="isCollecting"
          @click="collectTestCases"
          class="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-xs font-medium text-slate-600 hover:bg-slate-50 disabled:opacity-50"
        >
          <RefreshCw
            class="h-3.5 w-3.5"
            :class="{ 'animate-spin': isCollecting }"
          />

          {{ isCollecting ? "Collecting..." : "Refresh" }}
        </button>
      </div>

      <!-- Loading -->
      <div
        v-if="isCollecting"
        class="px-5 py-10 text-center text-sm text-slate-500"
      >
        Collecting test cases...
      </div>

      <!-- Error -->
      <div
        v-else-if="collectError"
        class="m-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700"
      >
        {{ collectError }}
      </div>

      <!-- Empty -->
      <div v-else-if="testCases.length === 0" class="px-5 py-10 text-center">
        <div class="text-sm font-medium text-slate-700">
          No test cases found
        </div>

        <div class="mt-1 text-xs text-slate-500">
          pytest did not collect any tests.
        </div>
      </div>

      <!-- Test cases -->
      <div v-else class="divide-y divide-slate-100">
        <div
          v-for="test in testCases"
          :key="test.id"
          class="px-5 py-3 transition hover:bg-slate-50"
        >
          <div class="flex items-center gap-3">
            <input
              type="checkbox"
              class="h-4 w-4 rounded border-slate-300 text-emerald-500 focus:ring-emerald-500"
            />

            <div class="min-w-0 flex-1">
              <div class="truncate text-sm font-medium text-slate-800">
                {{ test.name }}
              </div>

              <div class="mt-0.5 truncate font-mono text-[11px] text-slate-500">
                {{ test.file }}

                <template v-if="test.className">
                  ::{{ test.className }}
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
