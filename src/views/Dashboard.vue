<script setup lang="ts">
import { onMounted, computed, ref } from "vue";
import { useRouter } from "vue-router";
import { useProjectStore } from "../stores/projectStore";
import {
  FolderPlus,
  GitBranch,
  Sparkles,
  BookOpen,
  Bug,
  Search,
  SlidersHorizontal,
  ArrowUpDown,
  Code2,
  Database,
  Check,
  AlertCircle,
  X,
  Terminal,
} from "@lucide/vue";

const router = useRouter();
const projectStore = useProjectStore();

// 搜索关键词
const searchQuery = ref("");

// 过滤后的项目列表
const filteredProjects = computed(() => {
  if (!searchQuery.value) return projectStore.projects;
  const query = searchQuery.value.toLowerCase();
  return projectStore.projects.filter(
    (p) =>
      p.name.toLowerCase().includes(query) ||
      p.path.toLowerCase().includes(query),
  );
});

// 状态颜色映射
const getStatusColor = (status: string) => {
  switch (status) {
    case "Ready":
      return "bg-emerald-500";
    case "Warning":
      return "bg-amber-500";
    case "Failed":
      return "bg-rose-500";
    default:
      return "bg-slate-400";
  }
};

const getStatusTextColor = (status: string) => {
  switch (status) {
    case "Ready":
      return "text-emerald-600";
    case "Warning":
      return "text-amber-600";
    case "Failed":
      return "text-rose-600";
    default:
      return "text-slate-600";
  }
};

const getEnvIcon = (detail: string) => {
  if (
    detail.toLowerCase().includes("missing") ||
    detail.toLowerCase().includes("not found")
  )
    return X;
  if (
    detail.toLowerCase().includes("ok") ||
    detail.toLowerCase().includes("activated")
  )
    return Check;
  return AlertCircle;
};

// 页面挂载时拉取数据
onMounted(() => {
  projectStore.fetchProjects();
});

// 路由跳转
const goToImport = () => router.push("/projects/import");
const goToProject = (id: string) => router.push(`/projects/${id}/execute`);
</script>

<template>
  <div
    class="flex h-screen w-screen overflow-hidden bg-slate-50 text-slate-900"
  >
    <!-- LEFT PANE: Actions & Navigation -->
    <aside class="w-72 bg-white border-r border-zinc-200/80 flex flex-col">
      <div class="px-6 pt-8 pb-6">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 bg-linear-to-br from-emerald-500 to-emerald-600 rounded-md flex items-center justify-center shadow-sm"
          >
            <img src="/src/assets/app-icon-sm.png" />
          </div>
          <div>
            <h1 class="text-sm font-semibold text-slate-800">PyTest Auto</h1>
            <p class="text-[10px] text-slate-500">Testing & Coverage Suite</p>
          </div>
        </div>
      </div>

      <div class="px-4 flex-1">
        <p
          class="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2"
        >
          Quick Actions
        </p>
        <div class="space-y-0.5">
          <button
            @click="goToImport"
            class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left group active:scale-[0.99]"
          >
            <div
              class="w-7 h-7 bg-emerald-50 rounded-md flex items-center justify-center group-hover:bg-emerald-100 transition-colors"
            >
              <FolderPlus class="w-3.5 h-3.5 text-emerald-500" />
            </div>
            <div>
              <p class="text-[13px] font-medium text-slate-800">
                Import Project
              </p>
              <p class="text-[10px] text-slate-500">Open local directory</p>
            </div>
          </button>

          <button
            class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left group active:scale-[0.99]"
          >
            <div
              class="w-7 h-7 bg-blue-50 rounded-md flex items-center justify-center group-hover:bg-blue-100 transition-colors"
            >
              <GitBranch class="w-3.5 h-3.5 text-blue-500" />
            </div>
            <div>
              <p class="text-[13px] font-medium text-slate-800">
                Clone Repository
              </p>
              <p class="text-[10px] text-slate-500">Get from Git remote</p>
            </div>
          </button>

          <button
            class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left group active:scale-[0.99]"
          >
            <div
              class="w-7 h-7 bg-indigo-50 rounded-md flex items-center justify-center group-hover:bg-indigo-100 transition-colors"
            >
              <Sparkles class="w-3.5 h-3.5 text-indigo-500" />
            </div>
            <div>
              <p class="text-[13px] font-medium text-slate-800">
                AI Test Generator
              </p>
              <p class="text-[10px] text-slate-500">Powered by Pynguin</p>
            </div>
          </button>
        </div>

        <p
          class="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-wider mt-6 mb-2"
        >
          Resources
        </p>
        <div class="space-y-0.5">
          <a
            href="#"
            class="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left"
          >
            <BookOpen class="w-3.5 h-3.5 text-slate-400" />
            <span class="text-[13px] text-slate-600">Documentation</span>
          </a>
          <a
            href="#"
            class="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left"
          >
            <Bug class="w-3.5 h-3.5 text-slate-400" />
            <span class="text-[13px] text-slate-600">Report Issue</span>
          </a>
        </div>
      </div>
       <div class="px-4 mt-4 border-t border-zinc-200/80 pt-4">
          <p
            class="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-2"
          >
            Debug
          </p>
          <button
            @click="$router.push('/debug/schema')"
            class="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-slate-50 transition-all text-left"
          >
            <Database class="w-3.5 h-3.5 text-slate-400" />
            <span class="text-[13px] text-slate-600">View DB Schema</span>
          </button>
        </div>

      <div class="px-6 py-4 border-t border-zinc-200/80">
        <p class="text-[10px] text-slate-400 font-mono">
          v1.2.0 · Build 2024.3
        </p>
      </div>
    </aside>

    <!-- RIGHT PANE: Project Management -->
    <main class="flex-1 flex flex-col overflow-hidden bg-slate-50">
      <!-- Toolbar -->
      <div
        class="h-14 px-6 flex items-center justify-between border-b border-zinc-200/80 bg-white"
      >
        <div class="flex items-center gap-3">
          <h2 class="text-sm font-semibold text-slate-800">Projects</h2>
          <span
            class="px-1.5 py-0.5 bg-slate-100 text-slate-500 text-[10px] font-mono rounded"
          >
            {{ projectStore.projects.length }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <div class="relative">
            <Search
              class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2"
            />
            <input
              v-model="searchQuery"
              type="text"
              placeholder="Search projects..."
              class="pl-8 pr-3 py-1.5 bg-slate-50 border border-zinc-200/80 rounded-md text-xs w-56 focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-500/40 transition-all"
            />
          </div>
          <button
            class="flex items-center gap-1.5 px-2.5 py-1.5 bg-white border border-zinc-200/80 rounded-md text-xs text-slate-600 hover:bg-slate-50 transition-all active:scale-[0.98]"
          >
            <SlidersHorizontal class="w-3.5 h-3.5" />
            <span>Filter</span>
          </button>
          <button
            class="flex items-center gap-1.5 px-2.5 py-1.5 bg-white border border-zinc-200/80 rounded-md text-xs text-slate-600 hover:bg-slate-50 transition-all active:scale-[0.98]"
          >
            <ArrowUpDown class="w-3.5 h-3.5" />
            <span>Last Opened</span>
          </button>
        </div>
      </div>

      <!-- Content Area -->
      <div class="flex-1 overflow-auto">
        <!-- Empty State (当没有项目或搜索无结果时) -->
        <div
          v-if="projectStore.isLoading"
          class="flex flex-col items-center justify-center h-full text-slate-400"
        >
          <div
            class="w-8 h-8 border-2 border-slate-200 border-t-emerald-500 rounded-full animate-spin mb-3"
          ></div>
          <p class="text-sm">Loading projects...</p>
        </div>

        <div
          v-else-if="filteredProjects.length === 0"
          class="flex flex-col items-center justify-center h-full text-slate-400"
        >
          <FolderPlus class="w-12 h-12 mb-3 text-slate-300" />
          <p class="text-sm font-medium text-slate-600 mb-1">
            {{
              searchQuery
                ? "No projects match your search"
                : "No projects imported yet"
            }}
          </p>
          <p class="text-xs mb-4">
            Get started by importing a local Python directory.
          </p>
          <button
            @click="goToImport"
            class="px-4 py-2 bg-emerald-500 text-white text-xs font-medium rounded-md hover:bg-emerald-600 transition-colors active:scale-[0.98]"
          >
            Import Project
          </button>
        </div>

        <!-- Project List -->
        <div v-else>
          <!-- Table Header -->
          <div
            class="px-6 py-2 bg-slate-50 border-b border-zinc-200/80 grid grid-cols-[4fr_1.5fr_2fr_2.5fr_1.5fr] gap-4 text-[10px] font-semibold text-slate-400 uppercase tracking-wider sticky top-0 z-10"
          >
            <div>Project</div>
            <div>Environment</div>
            <div>Last Test Result</div>
            <div>Coverage</div>
            <div class="text-right">Last Run</div>
          </div>

          <!-- Rows -->
          <div class="divide-y divide-zinc-200/80 bg-white">
            <div
              v-for="project in filteredProjects"
              :key="project.id"
              @click="goToProject(project.id)"
              class="px-6 py-3.5 hover:bg-slate-50 transition-colors cursor-pointer grid grid-cols-[4fr_1.5fr_2fr_2.5fr_1.5fr] gap-4 items-center group"
            >
              <!-- Project Info -->
              <div class="flex items-center gap-3 min-w-0">
                <div
                  class="w-8 h-8 bg-gradient-to-br from-blue-500 to-blue-600 rounded-md flex items-center justify-center flex-shrink-0"
                >
                  <Code2 class="w-4 h-4 text-white" />
                </div>
                <div class="min-w-0">
                  <h4 class="text-[13px] font-medium text-slate-800 truncate">
                    {{ project.name }}
                  </h4>
                  <p class="text-[10px] text-slate-500 font-mono truncate">
                    {{ project.path }}
                  </p>
                </div>
              </div>

              <!-- Environment Status with Tooltip -->
              <div class="relative">
                <button
                  class="flex items-center gap-1.5 px-2 py-1 rounded hover:bg-slate-100 transition-colors"
                >
                  <span
                    :class="`w-1.5 h-1.5 rounded-full ${getStatusColor(project.env_status)}`"
                  ></span>
                  <span
                    :class="`text-[11px] ${getStatusTextColor(project.env_status)}`"
                    >{{ project.env_status }}</span
                  >
                </button>
                <!-- Tooltip -->
                <div
                  class="absolute top-full left-0 mt-1.5 w-52 bg-white border border-zinc-200 rounded-md shadow-md p-2.5 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-20 pointer-events-none"
                >
                  <p
                    class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-1.5"
                  >
                    Environment Status
                  </p>
                  <ul class="space-y-1">
                    <li
                      v-for="(detail, idx) in project.env_details"
                      :key="idx"
                      class="flex items-center gap-2 text-[11px]"
                    >
                      <component
                        :is="getEnvIcon(detail)"
                        :class="`w-3 h-3 ${detail.toLowerCase().includes('missing') || detail.toLowerCase().includes('not found') ? 'text-rose-500' : 'text-emerald-500'}`"
                      />
                      <span
                        :class="
                          detail.toLowerCase().includes('missing') ||
                          detail.toLowerCase().includes('not found')
                            ? 'text-rose-700'
                            : 'text-slate-700'
                        "
                      >
                        {{ detail }}
                      </span>
                    </li>
                  </ul>
                </div>
              </div>

              <!-- Test Result -->
              <div class="flex items-center gap-2">
                <span class="text-[11px] font-mono text-emerald-600"
                  >{{ project.tests_passed }} passed</span
                >
                <span class="text-slate-300">·</span>
                <span class="text-[11px] font-mono text-rose-600"
                  >{{ project.tests_failed }} failed</span
                >
              </div>

              <!-- Coverage -->
              <div class="flex items-center gap-2">
                <div class="flex-1 bg-slate-100 rounded-full h-1.5">
                  <div
                    :class="`h-1.5 rounded-full ${project.coverage < 80 ? 'bg-amber-500' : 'bg-emerald-500'}`"
                    :style="{ width: `${project.coverage}%` }"
                  ></div>
                </div>
                <span
                  class="text-[11px] font-mono text-slate-600 w-10 text-right"
                  >{{ project.coverage }}%</span
                >
              </div>

              <!-- Last Run -->
              <div class="text-right">
                <span class="text-[11px] text-slate-500">{{
                  project.last_run
                }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- BOTTOM STATUS BAR -->
      <div
        class="h-7 px-4 flex items-center justify-between border-t border-zinc-200/80 bg-white text-[10px] text-slate-500"
      >
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-1.5">
            <Terminal class="w-3 h-3" />
            <span class="font-mono">Python 3.13.2 (venv)</span>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 bg-emerald-500 rounded-full"></span>
            <span>Tauri IPC: Connected</span>
          </div>
        </div>
        <div class="flex items-center gap-4 font-mono">
          <span>{{ projectStore.stats.total_projects }} Projects</span>
          <span>{{ projectStore.stats.total_runs }} Total Runs</span>
        </div>
      </div>
    </main>
  </div>
</template>
