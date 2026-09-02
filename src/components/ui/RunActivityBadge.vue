<script setup lang="ts">
/**
 * 全局运行指示器 —— 显示当前正在后台执行的任务（generation / test / coverage）。
 * 常驻于 Layout 顶部工具栏，切到任意子页（或其它项目）都能看到并停止。
 */
import { computed } from "vue";
import { Loader2, Square } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { useRunStore, type RunType } from "../../stores/runStore";

const runStore = useRunStore();
const { t } = useI18n();

const active = computed(() => runStore.activeRuns);

function labelFor(type: RunType): string {
  return t(`runActivity.${type}`);
}

function stop(runId: string) {
  void runStore.cancel(runId);
}
</script>

<template>
  <div v-if="active.length" class="flex items-center gap-1.5">
    <div
      v-for="run in active"
      :key="run.runId"
      class="flex max-w-64 items-center gap-2 rounded-full border border-sky-200 bg-sky-50 py-1 pl-2.5 pr-1 text-[11px] text-sky-800"
      :title="`${labelFor(run.type)} · ${run.projectName}`"
    >
      <Loader2 class="h-3 w-3 shrink-0 animate-spin text-sky-600" />
      <span class="shrink-0 font-medium">{{ labelFor(run.type) }}</span>
      <span class="max-w-24 truncate text-sky-600">{{ run.projectName }}</span>
      <button
        type="button"
        class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-sky-600 transition hover:bg-sky-100 hover:text-sky-800"
        :aria-label="t('common.stop')"
        :title="t('common.stop')"
        @click="stop(run.runId)"
      >
        <Square class="h-2.5 w-2.5" fill="currentColor" />
      </button>
    </div>
  </div>
</template>
