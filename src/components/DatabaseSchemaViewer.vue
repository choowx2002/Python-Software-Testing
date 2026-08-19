<script setup lang="ts">
/**
 * Database Schema Viewer
 * 
 * 纯前端实现：通过 @tauri-apps/plugin-sql 直接查询 SQLite 的 sqlite_master 和 PRAGMA 指令。
 * 不再依赖 Rust 后端的 IPC command。
 * 
 * 功能：
 *   1. 列出所有用户表（排除 sqlite_ 系统表）
 *   2. 展示每张表的列信息（名称、类型、PK、NOT NULL、默认值）
 *   3. 展示每张表的索引
 *   4. 显示数据库文件路径
 *   5. 支持点击表名查看该表的最新 N 条数据（调试用）
 */

import { ref, onMounted } from 'vue'
import { getDatabase } from '../utils/db'
import {
  Database, Table, Key, Type, Check, X,
  ChevronRight, ChevronDown, Rows3, RefreshCw
} from '@lucide/vue'
import { appDataDir, join } from '@tauri-apps/api/path'

// ============================================
// Types
// ============================================
interface ColumnInfo {
  cid: number
  name: string
  type: string
  notnull: number
  dflt_value: string | null
  pk: number
}

interface IndexInfo {
  seq: number
  name: string
  unique: number
  origin: string
  partial: number
}

interface TableSchema {
  name: string
  columns: ColumnInfo[]
  indexes: IndexInfo[]
  rowCount: number
}

// ============================================
// State
// ============================================
const schemas = ref<TableSchema[]>([])
const dbPath = ref('')
const isLoading = ref(true)
const error = ref<string | null>(null)
const expandedTables = ref<Set<string>>(new Set())
const tableData = ref<Record<string, any[]>>({})
const loadingDataFor = ref<string | null>(null)

// ============================================
// Core Logic
// ============================================
onMounted(async () => {
  await loadSchema()
})

async function loadSchema() {
  isLoading.value = true
  error.value = null

  try {
    const dataDir = await appDataDir()
    dbPath.value = await join(dataDir, 'pytest_auto.db')

    // 2. 获取数据库连接
    const db = await getDatabase()

    // 3. 查询所有用户表
    const tables = await db.select<{ name: string }[]>(
      `SELECT name FROM sqlite_master 
       WHERE type='table' 
         AND name NOT LIKE 'sqlite_%' 
       ORDER BY name`
    )

    const result: TableSchema[] = []

    for (const table of tables) {
      const tableName = table.name

      // 4. 查询列信息
      const columns = await db.select<ColumnInfo[]>(
        `PRAGMA table_info(${tableName})`
      )

      // 5. 查询索引信息
      const indexes = await db.select<IndexInfo[]>(
        `PRAGMA index_list(${tableName})`
      )

      // 6. 查询行数（用于预览）
      const countResult = await db.select<{ count: number }[]>(
        `SELECT COUNT(*) as count FROM ${tableName}`
      )
      const rowCount = countResult[0]?.count ?? 0

      result.push({
        name: tableName,
        columns: columns || [],
        indexes: indexes || [],
        rowCount,
      })
    }

    schemas.value = result
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    console.error('[Schema Viewer] Failed to load schema:', err)
  } finally {
    isLoading.value = false
  }
}

// 切换表展开状态
function toggleTable(tableName: string) {
  if (expandedTables.value.has(tableName)) {
    expandedTables.value.delete(tableName)
  } else {
    expandedTables.value.add(tableName)
  }
}

// 加载表数据预览（最多 10 条）
async function loadTableData(tableName: string) {
  if (tableData.value[tableName]) return // 已加载过
  loadingDataFor.value = tableName
  try {
    const db = await getDatabase()
    const rows = await db.select<any[]>(
      `SELECT * FROM ${tableName} LIMIT 10`
    )
    tableData.value[tableName] = rows || []
  } catch (err) {
    console.error(`[Schema Viewer] Failed to load data for ${tableName}:`, err)
    tableData.value[tableName] = []
  } finally {
    loadingDataFor.value = null
  }
}
</script>

<template>
  <div class="h-full flex flex-col bg-slate-50">
    <!-- Header -->
    <div class="px-6 py-4 bg-white border-b border-zinc-200/80">
      <div class="flex items-center justify-between mb-2">
        <div class="flex items-center gap-3">
          <Database class="w-5 h-5 text-emerald-500" />
          <h2 class="text-lg font-semibold text-slate-800">Database Schema</h2>
          <span class="px-1.5 py-0.5 bg-slate-100 text-slate-500 text-[10px] font-mono rounded">
            {{ schemas.length }} tables
          </span>
        </div>
        <button
          @click="loadSchema"
          class="flex items-center gap-1.5 px-2.5 py-1.5 bg-white border border-zinc-200/80 rounded-md text-xs text-slate-600 hover:bg-slate-50 transition-all active:scale-[0.98]"
        >
          <RefreshCw class="w-3.5 h-3.5" />
          <span>Reload</span>
        </button>
      </div>
      <p class="text-xs text-slate-500 font-mono truncate">{{ dbPath }}</p>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto p-6">
      <!-- Loading State -->
      <div v-if="isLoading" class="flex items-center justify-center h-64">
        <div class="text-center">
          <div class="w-8 h-8 border-2 border-slate-200 border-t-emerald-500 rounded-full animate-spin mx-auto mb-3"></div>
          <p class="text-sm text-slate-500">Loading schema...</p>
        </div>
      </div>

      <!-- Error State -->
      <div v-else-if="error" class="flex items-center justify-center h-64">
        <div class="text-center">
          <X class="w-12 h-12 text-rose-500 mx-auto mb-3" />
          <p class="text-sm font-medium text-slate-800 mb-1">Failed to load schema</p>
          <p class="text-xs text-rose-600 font-mono max-w-md">{{ error }}</p>
          <button
            @click="loadSchema"
            class="mt-4 px-3 py-1.5 bg-emerald-500 text-white text-xs font-medium rounded-md hover:bg-emerald-600 transition-colors"
          >
            Retry
          </button>
        </div>
      </div>

      <!-- Schema Display -->
      <div v-else class="space-y-4">
        <div
          v-for="table in schemas"
          :key="table.name"
          class="bg-white border border-zinc-200/80 rounded-md overflow-hidden"
        >
          <!-- Table Header (Clickable) -->
          <div
            @click="toggleTable(table.name)"
            class="px-4 py-3 bg-slate-50 border-b border-zinc-200/80 flex items-center gap-2 cursor-pointer hover:bg-slate-100 transition-colors"
          >
            <component
              :is="expandedTables.has(table.name) ? ChevronDown : ChevronRight"
              class="w-4 h-4 text-slate-500 transition-transform"
            />
            <Table class="w-4 h-4 text-slate-600" />
            <h3 class="text-sm font-semibold text-slate-800 font-mono">{{ table.name }}</h3>
            <div class="ml-auto flex items-center gap-3 text-[10px] text-slate-500 font-mono">
              <span>{{ table.columns.length }} columns</span>
              <span>·</span>
              <span>{{ table.indexes.length }} indexes</span>
              <span>·</span>
              <span>{{ table.rowCount }} rows</span>
            </div>
          </div>

          <!-- Expanded Content -->
          <div v-if="expandedTables.has(table.name)" class="divide-y divide-zinc-200/80">
            <!-- Columns Section -->
            <div class="p-4">
              <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">Columns</p>
              <div class="overflow-x-auto">
                <table class="w-full text-xs">
                  <thead class="bg-slate-50/50">
                    <tr class="text-left text-[10px] font-semibold text-slate-500 uppercase tracking-wider">
                      <th class="px-3 py-2 w-10">#</th>
                      <th class="px-3 py-2">Name</th>
                      <th class="px-3 py-2">Type</th>
                      <th class="px-3 py-2 text-center">PK</th>
                      <th class="px-3 py-2 text-center">NOT NULL</th>
                      <th class="px-3 py-2">Default</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-zinc-200/80">
                    <tr v-for="col in table.columns" :key="col.cid" class="hover:bg-slate-50 transition-colors">
                      <td class="px-3 py-2 text-slate-400 font-mono">{{ col.cid }}</td>
                      <td class="px-3 py-2">
                        <div class="flex items-center gap-2">
                          <Key v-if="col.pk > 0" class="w-3 h-3 text-amber-500" />
                          <span class="font-mono text-slate-700">{{ col.name }}</span>
                        </div>
                      </td>
                      <td class="px-3 py-2">
                        <span class="inline-flex items-center gap-1 px-1.5 py-0.5 bg-blue-50 text-blue-600 text-[10px] font-mono rounded">
                          <Type class="w-2.5 h-2.5" />
                          {{ col.type || 'TEXT' }}
                        </span>
                      </td>
                      <td class="px-3 py-2 text-center">
                        <Check v-if="col.pk > 0" class="w-3.5 h-3.5 text-emerald-500 mx-auto" />
                        <span v-else class="text-slate-300">—</span>
                      </td>
                      <td class="px-3 py-2 text-center">
                        <Check v-if="col.notnull > 0" class="w-3.5 h-3.5 text-emerald-500 mx-auto" />
                        <span v-else class="text-slate-300">—</span>
                      </td>
                      <td class="px-3 py-2 font-mono text-slate-500">
                        {{ col.dflt_value ?? '—' }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Indexes Section (if any) -->
            <div v-if="table.indexes.length > 0" class="p-4">
              <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2">Indexes</p>
              <div class="space-y-1">
                <div
                  v-for="idx in table.indexes"
                  :key="idx.name"
                  class="flex items-center gap-2 px-3 py-1.5 bg-slate-50 rounded-md"
                >
                  <span class="w-1.5 h-1.5 bg-indigo-500 rounded-full"></span>
                  <span class="text-[11px] font-mono text-slate-700">{{ idx.name }}</span>
                  <span v-if="idx.unique" class="px-1.5 py-0.5 bg-indigo-50 text-indigo-600 text-[10px] font-mono rounded">
                    UNIQUE
                  </span>
                </div>
              </div>
            </div>

            <!-- Data Preview Section -->
            <div class="p-4">
              <div class="flex items-center justify-between mb-2">
                <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider">
                  Data Preview (Top 10)
                </p>
                <button
                  v-if="!tableData[table.name]"
                  @click.stop="loadTableData(table.name)"
                  :disabled="loadingDataFor === table.name"
                  class="flex items-center gap-1 px-2 py-1 bg-white border border-zinc-200/80 rounded text-[10px] text-slate-600 hover:bg-slate-50 transition-all active:scale-[0.98] disabled:opacity-50"
                >
                  <Rows3 class="w-3 h-3" />
                  <span>{{ loadingDataFor === table.name ? 'Loading...' : 'Load' }}</span>
                </button>
              </div>

              <!-- Loading data -->
              <div v-if="loadingDataFor === table.name" class="text-center py-4">
                <div class="w-5 h-5 border-2 border-slate-200 border-t-emerald-500 rounded-full animate-spin mx-auto"></div>
              </div>

              <!-- Data Table -->
              <div v-else-if="tableData[table.name]" class="overflow-x-auto">
                <div v-if="tableData[table.name].length === 0" class="text-center py-4 text-xs text-slate-400">
                  (empty table)
                </div>
                <table v-else class="w-full text-[11px]">
                  <thead class="bg-slate-50/50">
                    <tr class="text-left text-[10px] font-semibold text-slate-500 uppercase tracking-wider">
                      <th v-for="col in table.columns" :key="col.name" class="px-3 py-1.5 font-mono">
                        {{ col.name }}
                      </th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-zinc-200/80">
                    <tr v-for="(row, i) in tableData[table.name]" :key="i" class="hover:bg-slate-50">
                      <td v-for="col in table.columns" :key="col.name" class="px-3 py-1.5 font-mono text-slate-600 max-w-[200px] truncate">
                        {{ row[col.name] ?? 'NULL' }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>

        <!-- Summary -->
        <div class="mt-6 p-4 bg-white border border-zinc-200/80 rounded-md">
          <p class="text-xs text-slate-500">
            <span class="font-semibold text-slate-700">{{ schemas.length }}</span> tables loaded.
            Total rows:
            <span class="font-mono text-emerald-600">
              {{ schemas.reduce((sum, t) => sum + t.rowCount, 0) }}
            </span>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>