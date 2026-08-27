<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "@lucide/vue";
import HistoryDetailContent from "../components/HistoryDetailContent.vue";

const route = useRoute();
const { t } = useI18n();

type DetailType = "execute" | "coverage" | "generation";

const detailType = computed<DetailType>(() => {
  const raw = route.query.detail;
  if (raw === "coverage") return "coverage";
  if (raw === "generation") return "generation";
  return "execute";
});

const detailId = computed<number | null>(() => {
  const raw = route.query.id;
  const n = Number(raw);
  return Number.isNaN(n) ? null : n;
});

const title = computed(() => {
  switch (detailType.value) {
    case "execute":
      return t("history.detail.titleExecute");
    case "coverage":
      return t("history.detail.titleCoverage");
    case "generation":
      return t("history.detail.titleGeneration");
  }
});

async function closeWindow() {
  await getCurrentWindow().close();
}
</script>

<template>
  <div class="flex h-screen w-screen flex-col bg-surface text-zinc-900">
    <!-- 顶部栏 -->
    <header class="flex h-14 shrink-0 items-center justify-between border-b border-border bg-white px-5">
      <h1 class="text-sm font-semibold text-zinc-900">{{ title }}</h1>
      <button
        type="button"
        class="rounded-lg p-1.5 text-zinc-400 transition-colors hover:bg-zinc-100 hover:text-zinc-600"
        :aria-label="t('common.close')"
        @click="closeWindow"
      >
        <X class="h-4 w-4" />
      </button>
    </header>

    <!-- 内容 -->
    <div class="min-h-0 flex-1 overflow-auto p-5">
      <HistoryDetailContent v-if="detailId !== null" :type="detailType" :id="detailId" :allow-rerun="false" />
    </div>
  </div>
</template>
