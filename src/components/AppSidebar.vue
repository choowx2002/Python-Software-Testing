<script setup lang="ts">
/**
 * 统一侧边栏外壳：品牌区（brand 插槽）+ 导航区（默认插槽）+ 可选底部插槽 + 版本/语言切换
 * Dashboard 与项目 Layout 共用，保证两处视觉一致。
 * 用法：
 *   <AppSidebar>
 *     <AppNavItem … />   ← 导航内容
 *     <template #footer>…</template>  ← 额外底部（如"返回"按钮）
 *   </AppSidebar>
 */
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { setLocale } from "../i18n";

const { t, locale } = useI18n();
const currentLocale = computed(() => locale.value);

function switchLocale(lang: "en" | "zh") {
  setLocale(lang);
}
</script>

<template>
  <aside class="flex w-60 shrink-0 flex-col border-r border-border bg-white">
    <!-- 品牌区 -->
    <div class="flex h-14 shrink-0 items-center gap-2.5 border-b border-border px-4">
      <slot name="brand">
        <div class="flex h-6 w-6 items-center justify-center overflow-hidden rounded-md bg-brand-500">
          <img src="/src/assets/app-icon-sm.png" alt="" class="h-4 w-4" />
        </div>
        <div class="min-w-0 leading-tight">
          <div class="text-[13px] font-semibold text-zinc-900">{{ t("app.name") }}</div>
          <div class="truncate text-[10px] text-zinc-400">{{ t("app.tagline") }}</div>
        </div>
      </slot>
    </div>

    <!-- 导航内容 -->
    <nav class="min-h-0 flex-1 overflow-y-auto px-2.5 py-3"><slot /></nav>

    <!-- 可选底部（如"返回 Dashboard"按钮） -->
    <div v-if="$slots.footer" class="shrink-0 border-t border-border px-2.5 py-2.5">
      <slot name="footer" />
    </div>

    <!-- 底部栏：版本 + 语言切换 -->
    <div class="flex shrink-0 items-center justify-between border-t border-border px-4 py-2.5">
      <span class="font-mono text-[10px] text-zinc-400">v1.2.0</span>
      <div class="flex items-center gap-0.5 rounded-md border border-border p-0.5">
        <button
          v-for="lang in (['en', 'zh'] as const)"
          :key="lang"
          type="button"
          class="rounded px-1.5 py-0.5 text-[10px] font-medium transition-colors"
          :class="
            currentLocale === lang
              ? 'bg-brand-500 text-white'
              : 'text-zinc-500 hover:bg-zinc-100'
          "
          @click="switchLocale(lang)"
        >
          {{ lang === "en" ? "EN" : "中文" }}
        </button>
      </div>
    </div>
  </aside>
</template>
