import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useTaskbarProgress } from '../composables/useTaskbarProgress'
import {
  notifyGenerationComplete,
  notifyExecutionComplete,
  notifyCoverageComplete,
} from '../composables/useNotifications'

export type RunType = 'generate' | 'execute' | 'coverage'

export interface ActiveRun {
  type: RunType
  runId: string
  projectId: number
  projectName: string
  since: number
}

export interface GenFileInfo {
  name: string
  relativePath: string | null
  testCaseCount: number
  status: string
}

export interface ExecResultInfo {
  id: string
  name: string
  file: string
  status: string
  duration: number
  errorMessage: string | null
  line: number | null
  skipReason: string | null
}

export interface CovFileInfo {
  path: string
  percentCovered: number
  executedLines: number[]
  missingLines: number[]
  excludedLines: number[]
}

export interface CovSummaryInfo {
  runId: string
  projectId: number
  percentCovered: number
  totalStatements: number
  coveredStatements: number
  totalBranches: number | null
  coveredBranches: number | null
  branchPercent: number | null
  files: CovFileInfo[]
  jsonPath: string
  command: string
}

export interface RunSnapshot {
  type: RunType
  runId: string
  projectId: number
  success: boolean
  status: 'completed' | 'failed'
  finishedAt: number
  duration: number
  command: string | null
  // generate
  generatedFiles?: GenFileInfo[]
  // execute
  passed?: number
  failed?: number
  skipped?: number
  results?: ExecResultInfo[]
  executionType?: string
  regressionSuiteId?: number | null
  // coverage
  summary?: CovSummaryInfo | null
}

interface RunCtx {
  type: RunType
  projectId: number
  projectName: string
}

/** 发起运行前的待配对上下文（started 事件到来后按 type FIFO 配对出 runId） */
const pendingCtx: Record<RunType, RunCtx[]> = {
  generate: [],
  execute: [],
  coverage: [],
}

export const useRunStore = defineStore('run', () => {
  const activeRuns = ref<ActiveRun[]>([])
  const finished = ref<Record<number, Partial<Record<RunType, RunSnapshot>>>>({})

  /** runId → 上下文（started 时登记） */
  const runs = new Map<string, RunCtx>()

  const { clear: clearTaskProgress } = useTaskbarProgress()

  // ---------------- 内部登记 ----------------
  function trackStarted(type: RunType, runId: string) {
    const pending = pendingCtx[type]
    const ctx = pending.shift()
    if (!ctx) return
    runs.set(runId, ctx)
    activeRuns.value.push({
      type,
      runId,
      projectId: ctx.projectId,
      projectName: ctx.projectName,
      since: Date.now(),
    })
  }

  function setFinished(projectId: number, type: RunType, snap: RunSnapshot) {
    if (!finished.value[projectId]) finished.value[projectId] = {}
    finished.value[projectId][type] = snap
  }

  // ---------------- 收尾副作用（DB / 通知 / 任务栏清理） ----------------
  async function finalizeGenerate(payload: {
    runId: string
    success: boolean
    duration: number
    generatedFiles: GenFileInfo[]
    command: string
  }) {
    const ctx = runs.get(payload.runId)
    if (!ctx) return
    runs.delete(payload.runId)
    const type: RunType = 'generate'
    const rid = payload.runId

    removeActive(rid)
    const snap: RunSnapshot = {
      type,
      runId: rid,
      projectId: ctx.projectId,
      success: payload.success,
      status: payload.success ? 'completed' : 'failed',
      finishedAt: Date.now(),
      duration: payload.duration,
      command: payload.command || null,
      generatedFiles: payload.generatedFiles,
    }

    const okCount = payload.generatedFiles.filter((f) => f.status === 'success').length
    void notifyGenerationComplete(ctx.projectName, payload.success, okCount)
    void clearTaskProgress()

    if (ctx.projectId) {
      const generatedCount = payload.generatedFiles.filter(
        (f) => f.status === 'success',
      ).length
      try {
        const generationId = await invoke<number>('save_generation_history', {
          projectId: ctx.projectId,
          generationStatus: payload.success ? 'success' : 'failed',
          totalFiles: payload.generatedFiles.length,
          generatedFiles: generatedCount,
          duration: payload.duration,
          command: payload.command || null,
        })
        if (payload.generatedFiles.length > 0) {
          await invoke('save_generation_file_details', {
            generationId,
            files: payload.generatedFiles.map((f) => ({
              name: f.name,
              relativePath: f.relativePath || null,
              testCaseCount: f.testCaseCount,
              status: f.status,
            })),
          })
        }
      } catch (e) {
        console.error('[RunStore] save generation history failed:', e)
      }
    }

    setFinished(ctx.projectId, type, snap)
  }

  async function finalizeExecute(payload: {
    runId: string
    success: boolean
    duration: number
    passed: number
    failed: number
    skipped: number
    results: ExecResultInfo[]
    command: string
    executionType: string
    regressionSuiteId?: number | null
  }) {
    const ctx = runs.get(payload.runId)
    if (!ctx) return
    runs.delete(payload.runId)
    const type: RunType = 'execute'
    const rid = payload.runId

    removeActive(rid)
    const snap: RunSnapshot = {
      type,
      runId: rid,
      projectId: ctx.projectId,
      success: payload.success,
      status: payload.success ? 'completed' : 'failed',
      finishedAt: Date.now(),
      duration: payload.duration,
      command: payload.command || null,
      passed: payload.passed,
      failed: payload.failed,
      skipped: payload.skipped,
      results: payload.results,
      executionType: payload.executionType,
      regressionSuiteId: payload.regressionSuiteId ?? null,
    }

    void notifyExecutionComplete(ctx.projectName, payload.passed, payload.failed)
    void clearTaskProgress()

    if (ctx.projectId) {
      const totalTests = payload.passed + payload.failed + payload.skipped
      try {
        const executionId = await invoke<number>('save_execution_history', {
          projectId: ctx.projectId,
          executionType: payload.executionType,
          regressionSuiteId: payload.regressionSuiteId ?? null,
          executionStatus: payload.success ? 'success' : 'failed',
          command: payload.command || null,
          totalTests,
          passed: payload.passed,
          failed: payload.failed,
          skipped: payload.skipped,
          executionTime: payload.duration,
        })
        if (payload.results.length > 0) {
          await invoke('save_execution_result_details', {
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
          })
        }
        console.log('[RunStore] ✅ Execution history saved successfully')
      } catch (e) {
        console.error('[RunStore] ❌ Failed to save execution history:', e)
      }
    }

    setFinished(ctx.projectId, type, snap)
  }

  async function finalizeCoverage(payload: {
    runId: string
    success: boolean
    duration: number
    summary: CovSummaryInfo | null
    command: string
  }) {
    const ctx = runs.get(payload.runId)
    if (!ctx) return
    runs.delete(payload.runId)
    const type: RunType = 'coverage'
    const rid = payload.runId

    removeActive(rid)
    const snap: RunSnapshot = {
      type,
      runId: rid,
      projectId: ctx.projectId,
      success: payload.success,
      status: payload.summary ? 'completed' : 'failed',
      finishedAt: Date.now(),
      duration: payload.duration,
      command: payload.command || null,
      summary: payload.summary,
    }

    if (payload.summary) {
      void notifyCoverageComplete(ctx.projectName, payload.summary.percentCovered)
    }
    void clearTaskProgress()

    if (ctx.projectId) {
      const summary = payload.summary
      try {
        if (summary) {
          const coveredFileCount = summary.files.filter(
            (f) => f.percentCovered > 0,
          ).length
          await invoke('save_coverage_result', {
            projectId: ctx.projectId,
            executionId: null,
            totalStatementCoverage: summary.percentCovered,
            totalBranchCoverage: summary.branchPercent ?? null,
            fileCount: summary.files.length,
            coveredFileCount,
            detailJsonPath: summary.jsonPath,
          })
        }
        await invoke('save_coverage_history', {
          projectId: ctx.projectId,
          coverageStatus: summary
            ? payload.success
              ? 'success'
              : 'warning'
            : 'failed',
          percentCovered: summary?.percentCovered ?? 0,
          totalStatements: summary?.totalStatements ?? 0,
          coveredStatements: summary?.coveredStatements ?? 0,
          duration: payload.duration,
          command: payload.command || null,
          filesJson: summary ? JSON.stringify(summary.files) : null,
        })
        console.log('[RunStore] ✅ Coverage history saved successfully')
      } catch (e) {
        console.error('[RunStore] ❌ Failed to save coverage:', e)
      }
    }

    setFinished(ctx.projectId, type, snap)
  }

  function removeActive(runId: string) {
    const i = activeRuns.value.findIndex((r) => r.runId === runId)
    if (i !== -1) activeRuns.value.splice(i, 1)
  }

  // ---------------- 事件监听（App 级常驻） ----------------
  let inited = false
  async function init() {
    if (inited) return
    inited = true

    // generation
    void listen<{ runId: string }>('generation-started', (ev) => {
      trackStarted('generate', ev.payload.runId)
    })
    void listen<{
      runId: string
      success: boolean
      duration: number
      generatedFiles: GenFileInfo[]
      command: string
    }>('generation-finished', (ev) => {
      void finalizeGenerate(ev.payload)
    })

    // execution / test run
    void listen<{ runId: string }>('test-started', (ev) => {
      trackStarted('execute', ev.payload.runId)
    })
    void listen<{
      runId: string
      success: boolean
      duration: number
      passed: number
      failed: number
      skipped: number
      results: ExecResultInfo[]
      command: string
      executionType: string
      regressionSuiteId?: number | null
    }>('test-finished', (ev) => {
      void finalizeExecute(ev.payload)
    })

    // coverage
    void listen<{ runId: string }>('coverage-started', (ev) => {
      trackStarted('coverage', ev.payload.runId)
    })
    void listen<{
      runId: string
      success: boolean
      duration: number
      summary: CovSummaryInfo | null
      command: string
    }>('coverage-finished', (ev) => {
      void finalizeCoverage(ev.payload)
    })
  }

  // ---------------- 对外 API ----------------
  function prepare(type: RunType, ctx: { projectId: number; projectName: string }) {
    pendingCtx[type].push({ type, projectId: ctx.projectId, projectName: ctx.projectName })
  }

  async function cancel(runId: string) {
    try {
      await invoke('cancel_run', { runId })
    } catch (error) {
      console.error('[RunStore] cancel_run failed:', error)
    }
  }

  function activeForProject(projectId: number): ActiveRun[] {
    return activeRuns.value.filter((r) => r.projectId === projectId)
  }

  function lastRun(projectId: number, type: RunType): RunSnapshot | undefined {
    return finished.value[projectId]?.[type]
  }

  return {
    activeRuns,
    finished,
    init,
    prepare,
    cancel,
    activeForProject,
    lastRun,
  }
})
