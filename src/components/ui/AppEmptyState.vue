<script setup lang="ts">
/**
 * 统一空状态：图标 + 标题 + 描述 + 操作区（默认插槽）
 * 用法：<AppEmptyState :icon="FolderPlus" title="…" description="…"><AppButton>…</AppButton></AppEmptyState>
 */
import type { Component } from "vue";

defineProps<{
  icon?: Component;
  title: string;
  description?: string;
}>();
</script>

<template>
  <div class="flex h-full flex-col items-center justify-center px-8 py-16 text-center">
    <div
      v-if="icon"
      class="mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-100 text-zinc-400"
    >
      <component :is="icon" class="h-6 w-6" aria-hidden="true" />
    </div>

    <h3 class="text-sm font-semibold text-zinc-900">{{ title }}</h3>
    <p v-if="description" class="mt-1.5 max-w-md text-[13px] leading-5 text-zinc-500">
      {{ description }}
    </p>

    <div class="mt-5 flex flex-wrap items-center justify-center gap-2"><slot /></div>
  </div>
</template>
