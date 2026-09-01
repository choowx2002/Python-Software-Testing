<script setup lang="ts">
/**
 * ReportIssue —— 反馈问题
 * 收集环境信息 + 可选标题/描述，一键跳转到 GitHub Issues 新建页（预填内容）。
 * 纯跳转方案：不落盘、无后端；非 Tauri 环境降级 window.open。
 */
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ArrowLeft,
  Bug,
  ExternalLink,
  MonitorCog,
  ScrollText,
} from "@lucide/vue";
import AppButton from "../components/ui/AppButton.vue";
import AppTooltip from "../components/ui/AppTooltip.vue";

const router = useRouter();
const { t } = useI18n();

const REPO_URL = "https://github.com/choowx2002/Python-Software-Testing";
const ISSUES_URL = `${REPO_URL}/issues`;

/** 从 userAgent 解析操作系统与架构（不引入 Rust 改动） */
function detectPlatform(): { os: string; arch: string } {
  const ua = navigator.userAgent;
  let os = "Unknown";
  if (/Windows NT 10/i.test(ua)) os = "Windows 10/11";
  else if (/Windows NT/i.test(ua)) os = "Windows";
  else if (/Mac OS X/i.test(ua)) os = "macOS";
  else if (/Android/i.test(ua)) os = "Android";
  else if (/Linux/i.test(ua)) os = "Linux";

  let arch = "unknown";
  if (/arm64|aarch64/i.test(ua)) arch = "arm64";
  else if (/x86_64|amd64|Win64/i.test(ua)) arch = "x86_64";
  else if (/i[3-6]86/i.test(ua)) arch = "x86";

  return { os, arch };
}

const platform = detectPlatform();
const appVersion = ref("0.1.0");
const issueTitle = ref("");
const description = ref("");
const openFailed = ref(false);

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch {
    // 浏览器预览或版本接口不可用时保留回退值
  }
});

function buildIssueUrl(): string {
  const params = new URLSearchParams();
  const title = issueTitle.value.trim();
  const desc = description.value.trim();

  if (title) params.set("title", title);

  const bodyParts: string[] = [];
  if (desc) bodyParts.push(desc);
  bodyParts.push(
    "",
    "---",
    "### Environment",
    `- **OS**: ${platform.os} (${platform.arch})`,
    `- **App version**: v${appVersion.value}`,
  );
  params.set("body", bodyParts.join("\n"));

  return `${ISSUES_URL}/new?${params.toString()}`;
}

async function openIssue() {
  openFailed.value = false;
  const url = buildIssueUrl();
  try {
    await openUrl(url);
  } catch {
    // 非 Tauri 环境（浏览器预览）或打开失败：降级 window.open
    const win = window.open(url, "_blank", "noopener,noreferrer");
    openFailed.value = win === null;
  }
}
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
        <Bug class="h-4 w-4 text-brand-500" />
        <h2 class="text-sm font-semibold text-slate-800">{{ t("reportIssue.title") }}</h2>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-2xl space-y-5 p-6">
        <!-- 简介 -->
        <section class="rounded-md border border-zinc-200/80 bg-white p-5">
          <p class="text-[13px] leading-relaxed text-zinc-600">{{ t("reportIssue.subtitle") }}</p>
          <div class="mt-3 flex items-center gap-2 text-xs text-zinc-500">
            <span class="font-medium text-zinc-700">{{ t("reportIssue.repoLabel") }}:</span>
            <a
              :href="ISSUES_URL"
              target="_blank"
              rel="noopener noreferrer"
              class="truncate font-mono text-brand-600 underline-offset-2 hover:underline"
            >
              {{ ISSUES_URL }}
            </a>
          </div>
        </section>

        <!-- 环境信息 -->
        <section class="rounded-md border border-zinc-200/80 bg-white p-5">
          <div class="mb-4 flex items-center gap-2">
            <MonitorCog class="h-4 w-4 text-zinc-400" />
            <h3 class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
              {{ t("reportIssue.envTitle") }}
            </h3>
          </div>
          <p class="mb-3 text-xs text-zinc-500">{{ t("reportIssue.envDesc") }}</p>
          <dl class="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div class="rounded-md bg-zinc-50 p-3">
              <dt class="text-[10px] font-medium uppercase tracking-wider text-zinc-400">{{ t("reportIssue.os") }}</dt>
              <dd class="mt-1 text-[13px] font-medium text-zinc-800">{{ platform.os }}</dd>
            </div>
            <div class="rounded-md bg-zinc-50 p-3">
              <dt class="text-[10px] font-medium uppercase tracking-wider text-zinc-400">{{ t("reportIssue.arch") }}</dt>
              <dd class="mt-1 text-[13px] font-medium text-zinc-800">{{ platform.arch }}</dd>
            </div>
            <div class="rounded-md bg-zinc-50 p-3">
              <dt class="text-[10px] font-medium uppercase tracking-wider text-zinc-400">{{ t("reportIssue.version") }}</dt>
              <dd class="mt-1 text-[13px] font-medium text-zinc-800">v{{ appVersion }}</dd>
            </div>
          </dl>
        </section>

        <!-- Issue 详情 -->
        <section class="rounded-md border border-zinc-200/80 bg-white p-5">
          <div class="mb-4 flex items-center gap-2">
            <ScrollText class="h-4 w-4 text-zinc-400" />
            <h3 class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
              {{ t("reportIssue.formTitle") }}
            </h3>
          </div>

          <div class="space-y-4">
            <div>
              <label class="mb-1.5 block text-sm font-medium text-zinc-900">{{ t("reportIssue.titleLabel") }}</label>
              <input
                v-model="issueTitle"
                type="text"
                :placeholder="t('reportIssue.titlePlaceholder')"
                class="h-9 w-full rounded-lg border border-zinc-200 bg-white px-3 text-[13px] text-zinc-800 focus:border-brand-500 focus:outline-none focus:ring-2 focus:ring-brand-500/15"
              />
            </div>

            <div>
              <label class="mb-1.5 block text-sm font-medium text-zinc-900">{{ t("reportIssue.descLabel") }}</label>
              <textarea
                v-model="description"
                rows="5"
                :placeholder="t('reportIssue.descPlaceholder')"
                class="w-full resize-y rounded-lg border border-zinc-200 bg-white px-3 py-2 text-[13px] leading-relaxed text-zinc-800 focus:border-brand-500 focus:outline-none focus:ring-2 focus:ring-brand-500/15"
              />
            </div>

            <div class="flex flex-wrap items-center gap-3">
              <AppButton variant="primary" @click="openIssue">
                <ExternalLink class="h-3.5 w-3.5" />
                {{ t("reportIssue.openButton") }}
              </AppButton>
              <span v-if="openFailed" class="text-xs text-rose-600">
                {{ t("reportIssue.openFailed", { url: ISSUES_URL }) }}
              </span>
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>
