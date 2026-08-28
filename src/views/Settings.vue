<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { Copy, Database, Download, Eraser, ExternalLink, FolderPlus, GitBranch, HelpCircle, Loader2, Moon, Palette, RefreshCw, RotateCcw, Save, Settings, Sun, Trash2, WandSparkles, AlertCircle, ArrowLeft, CheckCircle2, RotateCcw, Sun, Trash2, XCircle, Eye, EyeOff } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import DatabaseSchemaViewer from "../../components/DatabaseSchemaViewer.vue";
import AppButton from "../components/ui/AppButton.vue";
import AppModal from "../components/ui/AppModal.vue";
import AppConfirmModal from "../components/ui/AppConfirmModal.vue";
import AppTooltip from "../components/ui/AppTooltip.vue";
import AppButton from "../components/ui/AppButton.vue";
import AppCard from "../components/ui/AppCard.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import { useUIStore } from "../../stores/uiStore";

const router = useRouter();
const uiStore = useUIStore();
const { t } = useI18n();

const showClearConfirm = ref(false);
const isClearing = ref(false);
const clearMessage = ref("");

async function clearAllHistory() {
  if (!confirm(t("settings.danger.confirmClear"))) return;
  isClearing.value = true;
  clearMessage.value = "";
  try {
    await invoke("clear_all_history");
    clearMessage.value = t("settings.danger.cleared");
    setTimeout(() => { clearMessage.value = ""; }, 3000);
  } catch (error) {
    clearMessage.value = error instanceof Error ? error.message : String(error);
  }
}

function resetColWidths() {
  uiStore.resetProjectColWidths();
  uiStore.resetTerminalWidths();
}

const version = "1.2.0";

const currentLocale = ref(localStorage.getItem("testmate-locale") || "en");
const switchLocale = (lang: "en" | "zh") => {
  localStorage.setItem("testmate-locale", lang);
  window.location.reload();
};

const notificationsEnabled = ref(localStorage.getItem("notifications-enabled") !== "false");
const setNotificationsEnabled = (enabled: boolean) => {
  localStorage.setItem("notifications-enabled", String(enabled));
};

const terminalTheme = computed(() => localStorage.getItem("terminal-theme") || "light");
const setTerminalTheme = (theme: "light" | "dark") => {
  localStorage.setItem("terminal-theme", theme);
  document.documentElement.classList.toggle("dark", theme === "dark");
};

function resetColWidths() {
  uiStore.resetProjectColWidths();
  uiStore.resetTerminalWidths();
}

const version = "1.2.0";

const currentLocale = ref(localStorage.getItem("testmate-locale") || "en");
const switchLocale = (lang: "en" | "zh") => {
  localStorage.setItem("testmate-locale", lang);
  window.location.reload();
};

const notificationsEnabled = ref(localStorage.getItem("notifications-enabled") !== "false");
const setNotificationsEnabled = (enabled: boolean) => {
  localStorage.setItem("notifications-enabled", String(enabled));
};

const terminalTheme = computed(() => localStorage.getItem("terminal-theme") || "light");
const setTerminalTheme = (theme: "light" | "dark") => {
  localStorage.setItem("terminal-theme", theme);
  document.documentElement.classList.toggle("dark", theme === "dark");
};

function resetColWidths() {
  uiStore.resetProjectColWidths();
  uiStore.resetTerminalWidths();
}

const version = "1.2.0";
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4 p-5">
    <!-- Header -->
    <header class="flex items-center justify-between gap-4 border-b border-border bg-white px-5 py-4">
      <h1 class="text-lg font-semibold text-zinc-900">{{ t("settings.title") }}</h1>
      <AppButton variant="ghost" @click="router.back()">
        <ArrowLeft class="h-4 w-4" />
        {{ t("common.back") }}
      </AppButton>
    </header>

    <div class="flex-1 overflow-y-auto space-y-6">
      <!-- General -->
      <section class="space-y-4">
        <h2 class="text-sm font-semibold text-zinc-900 uppercase tracking-wider mb-4">{{ t("settings.general") }}</section>

        <div class="space-y-4">
          <!-- Language -->
          <div>
            <label class="block text-xs font-medium text-zinc-700 mb-2">{{ t("settings.language") }}</label>
            <div class="flex items-center gap-2">
              <button
                v-for="lang in (['en', 'zh'] as const)"
                :key="lang"
                @click="switchLocale(lang)"
                :class="[
                  'px-3 py-1.5 rounded-md text-xs font-medium transition-colors',
                  currentLocale === lang
                    ? 'bg-brand-500 text-white'
                    : 'text-zinc-500 hover:bg-zinc-100'
                ]"
              >
                {{ lang === "en" ? "EN" : "中文" }}
              </button>
            </div>
          </div>

          <!-- Notifications -->
          <div>
            <label class="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                :checked="notificationsEnabled"
                @change="setNotificationsEnabled($event.target.checked)"
                class="h-4 w-4 rounded border-zinc-300 text-brand-500 focus:ring-2 focus:ring-brand-500"
              />
              <div>
                <p class="text-sm font-medium text-zinc-900">{{ t("settings.notifications") }}</p>
                <p class="text-xs text-zinc-500">{{ t("settings.notificationsHint") }}</p>
              </div>
            </label>
          </div>

          <!-- Terminal Theme -->
          <div>
            <label class="block text-xs font-medium text-zinc-700 mb-2">{{ t("settings.terminalTheme") }}</label>
            <div class="flex items-center gap-2">
              <button
                @click="setTerminalTheme('light')"
                :class="[
                  'px-3 py-1.5 rounded-md text-xs font-medium transition-colors',
                  terminalTheme.value === 'light'
                    ? 'bg-brand-500 text-white'
                    : 'bg-zinc-100 text-zinc-600 hover:bg-zinc-200'
                ]"
              >
                {{ t("settings.terminal.light") }}
              </button>
              <button
                @click="setTerminalTheme('dark')"
                :class="[
                  'px-3 py-1.5 rounded-md text-xs font-medium transition-colors',
                  terminalTheme.value === 'dark'
                    ? 'bg-brand-500 text-white'
                    : 'bg-zinc-100 text-zinc-600 hover:bg-zinc-200'
                ]"
              >
                {{ t("settings.terminal.dark") }}
              </button>
            </div>
          </div>
        </div>

        <div class="space-y-4">
          <!-- Layout -->
          <h2 class="text-sm font-semibold text-zinc-900 uppercase tracking-wider mb-4">{{ t("settings.layout") }}</section>

          <div class="space-y-4">
            <AppButton variant="secondary" @click="resetColWidths" :disabled="isClearing">
              <RefreshCw class="h-4 w-4" />
              {{ t("settings.resetColWidths") }}
            </AppButton>

            <AppButton variant="secondary" @click="resetColWidths" :disabled="isClearing">
              <RotateCcw class="h-4 w-4" />
              {{ t("settings.resetTerminalWidth") }}
            </AppButton>
          </div>
        </div>

        <!-- Data & Debug -->
        <section class="space-y-4">
          <h2 class="text-sm font-semibold text-zinc-900 uppercase tracking-wider mb-4">{{ t("settings.data") }}</section>

          <AppCard :title="t('settings.dbSchema')" :subtitle="t('settings.dbSchemaDesc')">
            <DatabaseSchemaViewer />
          </AppCard>
        </section>

        <!-- Danger Zone -->
        <section class="space-y-4">
          <h2 class="text-sm font-semibold text-zinc-900 uppercase tracking-wider mb-4">{{ t("settings.danger") }}</section>

          <div v-if="showClearConfirm" class="rounded-lg border border-rose-200 bg-rose-50 p-4">
            <p class="text-xs text-rose-700 mb-2">{{ t("settings.danger.confirmClear") }}</p>
            <div class="mt-3 flex gap-2">
              <AppButton variant="danger" size="sm" :loading="isClearing" @click="clearAllHistory">
                <Trash2 class="h-3.5 w-3.5" />
                {{ t("settings.danger.clearAll") }}
              </AppButton>
              <AppButton variant="ghost" size="sm" @click="showClearConfirm = false">
                {{ t("common.cancel") }}
              </AppButton>
            </div>
            <div v-else class="rounded-lg border border-rose-200 bg-rose-50 p-4">
              <div class="flex items-start gap-3">
                <AlertCircle class="h-4 w-4 shrink-0 text-rose-500" />
                <div>
                  <p class="text-xs font-medium text-rose-800">{{ t("settings.danger.title") }}</p>
                  <p class="mt-1 text-[11px] text-rose-600">{{ t("settings.danger.description") }}</p>
                </div>
              </div>
              <div class="mt-3 flex justify-end gap-2">
                <AppButton variant="ghost" size="sm" @click="showClearConfirm = false">
                  {{ t("common.cancel") }}
                </AppButton>
                <AppButton variant="danger" size="sm" :loading="isClearing" @click="clearAllHistory">
                  <Trash2 class="h-3.5 w-3.5" />
                  {{ t("settings.danger.clearAll") }}
                </AppButton>
              </div>
            </div>
          </div>
        </section>

        <!-- About -->
        <section class="space-y-4 pt-4 border-t border-border">
          <h2 class="text-sm font-semibold text-zinc-900 uppercase tracking-wider mb-4">{{ t("settings.about") }}</section>

          <div class="space-y-4">
            <div class="flex items-center gap-3">
              <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-brand-500">
                <WandSparkles class="h-5 w-5 text-white" />
              </div>
              <div>
                <p class="font-semibold text-zinc-900">{{ t("about.title") }}</p>
                <p class="text-xs text-zinc-500">v{{ version }}</p>
              </div>
            </div>
            <p class="text-xs text-zinc-500">{{ t("about.description") }}</p>
            <div class="pt-2 border-t border-border">
              <p class="text-xs font-medium text-zinc-700">{{ t("about.techStack") }}</p>
              <ul class="mt-1 space-y-1 text-xs text-zinc-500">
                <li v-for="item in t('about.stack', { count: 3 }).split('|')" :key="item">{{ item }}</li>
              </ul>
            </div>
          </div>
        </section>
      </div>
    </div>
</template>

<style scoped>
@media (prefers-reduced-motion: reduce) {
  * {
    transition: none !important;
  }
}
</style>