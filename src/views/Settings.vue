<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useI18n } from "vue-i18n";
import {
  AlertCircle,
  ArrowLeft,
  CheckCircle2,
  RefreshCw,
  RotateCcw,
  Settings,
  Trash2,
  WandSparkles,
} from "@lucide/vue";
import AppButton from "../components/ui/AppButton.vue";
import AppCard from "../components/ui/AppCard.vue";
import AppConfirmModal from "../components/ui/AppConfirmModal.vue";
import AppTooltip from "../components/ui/AppTooltip.vue";
import DatabaseSchemaViewer from "../components/DatabaseSchemaViewer.vue";
import { useUIStore } from "../stores/uiStore";
import { setLocale } from "../i18n";

const router = useRouter();
const uiStore = useUIStore();
const { t, tm, locale } = useI18n();

// ---------- 通用：语言 ----------
const currentLocale = computed(() => locale.value);

function switchLocale(lang: "en" | "zh") {
  setLocale(lang);
}

// ---------- 通用：终端主题（与 TerminalPanel 共用 uiStore.terminalDark） ----------
const terminalTheme = computed({
  get: () => (uiStore.terminalDark ? "dark" : "light"),
  set: (v: "light" | "dark") => uiStore.setTerminalDark(v === "dark"),
});

// ---------- 通用：编辑器（打开测试/源码文件用的软件） ----------
const EDITOR_PRESETS: { value: string; labelKey: string }[] = [
  { value: "", labelKey: "settings.editorPresetDefault" },
  { value: "code -g {path}:{line}", labelKey: "settings.editorPresetVSCode" },
  { value: "pycharm --line {line} {path}", labelKey: "settings.editorPresetPyCharm" },
  { value: "subl {path}:{line}", labelKey: "settings.editorPresetSublime" },
];

const editorSelection = computed({
  get: () =>
    EDITOR_PRESETS.some((p) => p.value === uiStore.editorCommand)
      ? uiStore.editorCommand
      : "custom",
  set: (v: string) => {
    if (v !== "custom") uiStore.setEditorCommand(v);
  },
});

// ---------- 布局：重置 ----------
const resetMessage = ref("");

function flashResetMessage() {
  resetMessage.value = t("settings.resetSuccess");
  window.setTimeout(() => (resetMessage.value = ""), 2000);
}

function resetProjectColWidths() {
  uiStore.resetProjectColWidths();
  flashResetMessage();
}

function resetTerminalWidth() {
  uiStore.resetTerminalWidths();
  flashResetMessage();
}

// ---------- 危险区：清空全部历史 ----------
const showClearConfirm = ref(false);
const isClearing = ref(false);
const clearMessage = ref("");
const clearFailed = ref(false);

async function clearAllHistory() {
  isClearing.value = true;
  clearMessage.value = "";
  clearFailed.value = false;
  try {
    await invoke("clear_all_history");
    clearMessage.value = t("settings.danger.cleared");
  } catch (error) {
    clearFailed.value = true;
    clearMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isClearing.value = false;
    showClearConfirm.value = false;
    if (clearMessage.value) {
      window.setTimeout(() => (clearMessage.value = ""), 3000);
    }
  }
}

// ---------- 关于 ----------
const version = ref("0.1.0");
const stackItems = computed(() => (tm("about.stack") as unknown as string[]) ?? []);

onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch {
    // 拿不到版本号时保留回退值
  }
});
</script>

<template>
  <div class="flex h-screen w-screen flex-col bg-slate-50">
    <!-- Header -->
    <div class="flex h-14 shrink-0 items-center justify-between border-b border-zinc-200/80 bg-white px-6">
      <div class="flex items-center gap-3">
        <AppTooltip :content="t('common.back')" position="top">
          <button
            type="button"
            @click="router.back()"
            class="rounded-md p-1.5 transition-colors hover:bg-slate-100"
            aria-label="Back"
          >
            <ArrowLeft class="h-4 w-4 text-slate-600" />
          </button>
        </AppTooltip>
        <div class="h-5 w-px bg-zinc-200/80"></div>
        <Settings class="h-4 w-4 text-brand-500" />
        <h2 class="text-sm font-semibold text-slate-800">{{ t("settings.title") }}</h2>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-3xl space-y-5 p-6">
        <!-- 通用 -->
        <section class="rounded-md border border-zinc-200/80 bg-white p-5">
          <h3 class="mb-4 text-xs font-semibold uppercase tracking-wider text-zinc-500">
            {{ t("settings.general") }}
          </h3>
          <div class="space-y-5">
            <!-- 语言 -->
            <div>
              <label class="mb-2 block text-sm font-medium text-zinc-900">{{ t("settings.language") }}</label>
              <div class="flex items-center gap-2">
                <button
                  v-for="lang in (['en', 'zh'] as const)"
                  :key="lang"
                  type="button"
                  @click="switchLocale(lang)"
                  :class="[
                    'rounded-md border px-3 py-1.5 text-xs font-medium transition-colors',
                    currentLocale === lang
                      ? 'border-brand-500 bg-brand-500 text-white'
                      : 'border-zinc-200 bg-white text-zinc-600 hover:bg-zinc-50',
                  ]"
                >
                  {{ lang === "en" ? "EN" : "中文" }}
                </button>
              </div>
            </div>

            <!-- 系统通知 -->
            <div>
              <label class="flex cursor-pointer select-none items-start gap-3">
                <input
                  type="checkbox"
                  v-model="uiStore.notificationsEnabled"
                  class="mt-0.5 h-4 w-4 rounded border-zinc-300 text-brand-500 focus:ring-2 focus:ring-brand-500"
                />
                <span>
                  <span class="block text-sm font-medium text-zinc-900">{{ t("settings.notifications") }}</span>
                  <span class="mt-0.5 block text-xs text-zinc-500">{{ t("settings.notificationsHint") }}</span>
                </span>
              </label>
            </div>

            <!-- 终端主题 -->
            <div>
              <label class="mb-2 block text-sm font-medium text-zinc-900">{{ t("settings.terminalTheme") }}</label>
              <div class="flex items-center gap-2">
                <button
                  type="button"
                  @click="terminalTheme = 'light'"
                  :class="[
                    'rounded-md border px-3 py-1.5 text-xs font-medium transition-colors',
                    terminalTheme === 'light'
                      ? 'border-brand-500 bg-brand-500 text-white'
                      : 'border-zinc-200 bg-white text-zinc-600 hover:bg-zinc-50',
                  ]"
                >
                  {{ t("settings.terminalLight") }}
                </button>
                <button
                  type="button"
                  @click="terminalTheme = 'dark'"
                  :class="[
                    'rounded-md border px-3 py-1.5 text-xs font-medium transition-colors',
                    terminalTheme === 'dark'
                      ? 'border-brand-500 bg-brand-500 text-white'
                      : 'border-zinc-200 bg-white text-zinc-600 hover:bg-zinc-50',
                  ]"
                >
                  {{ t("settings.terminalDark") }}
                </button>
              </div>
            </div>

            <!-- 编辑器：用哪个软件打开测试/源码文件 -->
            <div>
              <label class="mb-2 block text-sm font-medium text-zinc-900">{{ t("settings.editor") }}</label>
              <select
                v-model="editorSelection"
                class="h-9 w-full rounded-lg border border-zinc-200 bg-white px-3 text-[13px] text-zinc-800 focus:border-brand-500 focus:outline-none focus:ring-2 focus:ring-brand-500/15"
              >
                <option v-for="p in EDITOR_PRESETS" :key="p.value" :value="p.value">
                  {{ t(p.labelKey) }}
                </option>
                <option value="custom">{{ t("settings.editorCustom") }}</option>
              </select>
              <input
                v-if="editorSelection === 'custom'"
                v-model="uiStore.editorCommand"
                type="text"
                :placeholder="t('settings.editorPlaceholder')"
                class="mt-2 h-9 w-full rounded-lg border border-zinc-200 bg-white px-3 font-mono text-[13px] text-zinc-800 focus:border-brand-500 focus:outline-none focus:ring-2 focus:ring-brand-500/15"
              />
              <p class="mt-1.5 text-xs text-zinc-500">{{ t("settings.editorHint") }}</p>
            </div>
          </div>
        </section>

        <!-- 布局 -->
        <section class="rounded-md border border-zinc-200/80 bg-white p-5">
          <h3 class="mb-4 text-xs font-semibold uppercase tracking-wider text-zinc-500">
            {{ t("settings.layout") }}
          </h3>
          <div class="flex flex-wrap items-center gap-3">
            <AppButton variant="secondary" size="sm" @click="resetProjectColWidths">
              <RefreshCw class="h-3.5 w-3.5" />
              {{ t("settings.resetColWidths") }}
            </AppButton>
            <AppButton variant="secondary" size="sm" @click="resetTerminalWidth">
              <RotateCcw class="h-3.5 w-3.5" />
              {{ t("settings.resetTerminalWidth") }}
            </AppButton>
            <span v-if="resetMessage" class="text-xs text-emerald-600">{{ resetMessage }}</span>
          </div>
        </section>

        <!-- 数据与调试 -->
        <section>
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zinc-500">
            {{ t("settings.data") }}
          </h3>
          <AppCard :title="t('settings.dbSchema')" :subtitle="t('settings.dbSchemaDesc')">
            <DatabaseSchemaViewer />
          </AppCard>
        </section>

        <!-- 危险区 -->
        <section class="rounded-md border border-rose-200 bg-white p-5">
          <div class="flex items-start gap-3">
            <AlertCircle class="mt-0.5 h-4 w-4 shrink-0 text-rose-500" />
            <div class="min-w-0 flex-1">
              <h3 class="text-sm font-semibold text-rose-800">{{ t("settings.danger.title") }}</h3>
              <p class="mt-1 text-xs text-rose-600">{{ t("settings.danger.description") }}</p>
              <div class="mt-3 flex flex-wrap items-center gap-3">
                <AppButton variant="dangerSolid" size="sm" @click="showClearConfirm = true">
                  <Trash2 class="h-3.5 w-3.5" />
                  {{ t("settings.danger.clearAll") }}
                </AppButton>
                <span
                  v-if="clearMessage"
                  class="text-xs"
                  :class="clearFailed ? 'text-rose-600' : 'text-emerald-600'"
                >
                  {{ clearMessage }}
                </span>
              </div>
            </div>
          </div>
        </section>

        <!-- 关于 -->
        <section class="rounded-md border border-zinc-200/80 bg-white p-5">
          <h3 class="mb-4 text-xs font-semibold uppercase tracking-wider text-zinc-500">
            {{ t("settings.about") }}
          </h3>
          <div class="flex items-center gap-3">
            <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-brand-500">
              <WandSparkles class="h-5 w-5 text-white" />
            </div>
            <div>
              <p class="text-sm font-semibold text-zinc-900">{{ t("about.title") }}</p>
              <p class="text-xs text-zinc-500">v{{ version }}</p>
            </div>
          </div>
          <p class="mt-3 text-xs text-zinc-500">{{ t("about.description") }}</p>
          <div class="mt-4 border-t border-zinc-100 pt-3">
            <p class="text-xs font-medium text-zinc-700">{{ t("about.techStack") }}</p>
            <ul class="mt-1.5 space-y-1.5">
              <li
                v-for="item in stackItems"
                :key="item"
                class="flex items-center gap-1.5 text-xs text-zinc-500"
              >
                <CheckCircle2 class="h-3.5 w-3.5 shrink-0 text-emerald-500" />
                {{ item }}
              </li>
            </ul>
          </div>
        </section>
      </div>
    </div>

    <AppConfirmModal
      :open="showClearConfirm"
      :title="t('settings.danger.clearAll')"
      :description="t('settings.danger.confirmClear')"
      :confirm-label="t('settings.danger.clearAll')"
      variant="danger"
      :loading="isClearing"
      @update:open="(v: boolean) => { if (!v) showClearConfirm = false }"
      @confirm="clearAllHistory"
    />
  </div>
</template>
