<script setup lang="ts">
/**
 * Docs —— 应用内用户指南
 * 内容与 docs/user-manual.md 保持一致（精简版，双语 i18n）。
 * 左侧锚点目录 + 右侧内容区，风格与 Settings 页一致。
 */
import { computed } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import {
  ArrowLeft,
  BookOpen,
  CheckCircle2,
  HelpCircle,
  Keyboard,
  ListChecks,
  LifeBuoy,
} from "@lucide/vue";
import AppTooltip from "../components/ui/AppTooltip.vue";

const router = useRouter();
const { t } = useI18n();

type Shortcut = { keys: string; desc: string };
type TroubleRow = { issue: string; fix: string };

const quickStartSteps = computed<string[]>(() => t("docs.quickStart.steps") as unknown as string[]);
const executePoints = computed<string[]>(() => t("docs.modules.execute.points") as unknown as string[]);
const generatePoints = computed<string[]>(() => t("docs.modules.generate.points") as unknown as string[]);
const coveragePoints = computed<string[]>(() => t("docs.modules.coverage.points") as unknown as string[]);
const globalShortcuts = computed<Shortcut[]>(() => t("docs.shortcuts.global") as unknown as Shortcut[]);
const projectShortcuts = computed<Shortcut[]>(() => t("docs.shortcuts.project") as unknown as Shortcut[]);
const troubleRows = computed<TroubleRow[]>(() => t("docs.troubleshooting.rows") as unknown as TroubleRow[]);
const systemPoints = computed<string[]>(() => t("docs.system.points") as unknown as string[]);

const tocItems = computed(() => [
  { href: "#quick-start", label: t("docs.quickStart.title"), icon: ListChecks },
  { href: "#modules", label: t("docs.modules.title"), icon: BookOpen },
  { href: "#shortcuts", label: t("docs.shortcuts.title"), icon: Keyboard },
  { href: "#troubleshooting", label: t("docs.troubleshooting.title"), icon: HelpCircle },
  { href: "#system", label: t("docs.system.title"), icon: LifeBuoy },
]);
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
        <BookOpen class="h-4 w-4 text-brand-500" />
        <h2 class="text-sm font-semibold text-slate-800">{{ t("docs.title") }}</h2>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto flex w-full max-w-5xl gap-8 p-6">
        <!-- 左侧锚点目录 -->
        <nav class="sticky top-6 hidden h-fit w-44 shrink-0 space-y-0.5 lg:block">
          <p class="section-label mb-1.5">{{ t("docs.title") }}</p>
          <a
            v-for="item in tocItems"
            :key="item.href"
            :href="item.href"
            class="flex items-center gap-2 rounded-md px-2.5 py-1.5 text-[13px] text-zinc-500 transition-colors hover:bg-zinc-100 hover:text-zinc-900"
          >
            <component :is="item.icon" class="h-3.5 w-3.5 shrink-0" />
            <span class="truncate">{{ item.label }}</span>
          </a>
        </nav>

        <!-- 右侧内容 -->
        <div class="min-w-0 flex-1 space-y-5">
          <p class="text-[13px] leading-relaxed text-zinc-500">{{ t("docs.subtitle") }}</p>

          <!-- 快速上手 -->
          <section id="quick-start" class="scroll-mt-20 rounded-md border border-zinc-200/80 bg-white p-5">
            <h3 class="mb-4 text-xs font-semibold uppercase tracking-wider text-zinc-500">
              {{ t("docs.quickStart.title") }}
            </h3>
            <ol class="list-decimal space-y-1.5 pl-5 text-[13px] leading-relaxed text-zinc-600">
              <li v-for="step in quickStartSteps" :key="step">{{ step }}</li>
            </ol>
          </section>

          <!-- 功能模块 -->
          <section id="modules" class="scroll-mt-20 rounded-md border border-zinc-200/80 bg-white p-5">
            <h3 class="mb-1 text-xs font-semibold uppercase tracking-wider text-zinc-500">
              {{ t("docs.modules.title") }}
            </h3>
            <p class="mb-4 text-xs text-zinc-500">{{ t("docs.modules.desc") }}</p>

            <div class="space-y-4">
              <div>
                <h4 class="mb-2 text-sm font-semibold text-zinc-800">{{ t("docs.modules.execute.title") }}</h4>
                <ul class="space-y-1.5">
                  <li v-for="p in executePoints" :key="p" class="flex gap-2 text-[13px] leading-relaxed text-zinc-600">
                    <CheckCircle2 class="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-500" />
                    <span>{{ p }}</span>
                  </li>
                </ul>
              </div>

              <div>
                <h4 class="mb-2 text-sm font-semibold text-zinc-800">{{ t("docs.modules.generate.title") }}</h4>
                <ul class="space-y-1.5">
                  <li v-for="p in generatePoints" :key="p" class="flex gap-2 text-[13px] leading-relaxed text-zinc-600">
                    <CheckCircle2 class="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-500" />
                    <span>{{ p }}</span>
                  </li>
                </ul>
              </div>

              <div>
                <h4 class="mb-2 text-sm font-semibold text-zinc-800">{{ t("docs.modules.coverage.title") }}</h4>
                <ul class="space-y-1.5">
                  <li v-for="p in coveragePoints" :key="p" class="flex gap-2 text-[13px] leading-relaxed text-zinc-600">
                    <CheckCircle2 class="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-500" />
                    <span>{{ p }}</span>
                  </li>
                </ul>
              </div>
            </div>
          </section>

          <!-- 键盘快捷键 -->
          <section id="shortcuts" class="scroll-mt-20 rounded-md border border-zinc-200/80 bg-white p-5">
            <h3 class="mb-1 text-xs font-semibold uppercase tracking-wider text-zinc-500">
              {{ t("docs.shortcuts.title") }}
            </h3>
            <p class="mb-4 text-xs text-zinc-500">{{ t("docs.shortcuts.desc") }}</p>

            <div class="space-y-4">
              <div>
                <h4 class="mb-2 text-[13px] font-semibold text-zinc-700">{{ t("docs.shortcuts.globalTitle") }}</h4>
                <table class="w-full border-collapse text-left text-[13px]">
                  <tbody>
                    <tr v-for="s in globalShortcuts" :key="s.keys" class="border-b border-zinc-100 last:border-0">
                      <td class="w-44 py-2 pr-4 align-top">
                        <code class="rounded bg-zinc-100 px-1.5 py-0.5 font-mono text-xs text-zinc-700">{{ s.keys }}</code>
                      </td>
                      <td class="py-2 text-zinc-600">{{ s.desc }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div>
                <h4 class="mb-2 text-[13px] font-semibold text-zinc-700">{{ t("docs.shortcuts.projectTitle") }}</h4>
                <table class="w-full border-collapse text-left text-[13px]">
                  <tbody>
                    <tr v-for="s in projectShortcuts" :key="s.keys" class="border-b border-zinc-100 last:border-0">
                      <td class="w-44 py-2 pr-4 align-top">
                        <code class="rounded bg-zinc-100 px-1.5 py-0.5 font-mono text-xs text-zinc-700">{{ s.keys }}</code>
                      </td>
                      <td class="py-2 text-zinc-600">{{ s.desc }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </section>

          <!-- 故障排查 -->
          <section id="troubleshooting" class="scroll-mt-20 rounded-md border border-zinc-200/80 bg-white p-5">
            <h3 class="mb-4 text-xs font-semibold uppercase tracking-wider text-zinc-500">
              {{ t("docs.troubleshooting.title") }}
            </h3>
            <table class="w-full border-collapse text-left text-[13px]">
              <thead>
                <tr class="border-b border-zinc-200 text-xs uppercase tracking-wider text-zinc-400">
                  <th class="w-2/5 pb-2 pr-4 font-medium">{{ t("docs.troubleshooting.issue") }}</th>
                  <th class="pb-2 font-medium">{{ t("docs.troubleshooting.fix") }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in troubleRows" :key="row.issue" class="border-b border-zinc-100 last:border-0">
                  <td class="py-2 pr-4 align-top text-zinc-700">{{ row.issue }}</td>
                  <td class="py-2 text-zinc-500">{{ row.fix }}</td>
                </tr>
              </tbody>
            </table>
          </section>

          <!-- 系统要求与数据 -->
          <section id="system" class="scroll-mt-20 rounded-md border border-zinc-200/80 bg-white p-5">
            <h3 class="mb-4 text-xs font-semibold uppercase tracking-wider text-zinc-500">
              {{ t("docs.system.title") }}
            </h3>
            <ul class="space-y-1.5">
              <li v-for="p in systemPoints" :key="p" class="flex gap-2 text-[13px] leading-relaxed text-zinc-600">
                <CheckCircle2 class="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-500" />
                <span>{{ p }}</span>
              </li>
            </ul>
          </section>

          <p class="pb-2 text-center text-xs text-zinc-400">{{ t("docs.footerNote") }}</p>
        </div>
      </div>
    </div>
  </div>
</template>
