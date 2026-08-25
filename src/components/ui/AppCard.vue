<script setup lang="ts">
/**
 * 统一卡片：可选标题栏（title/subtitle + actions 插槽），内容区自动滚动
 */
withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
    /** 内容区是否带内边距（如表格类内容可关闭） */
    padded?: boolean;
  }>(),
  { padded: true },
);
</script>

<template>
  <section class="card flex min-h-0 flex-col overflow-hidden">
    <!-- 标题栏：有 title 或有 actions 插槽时才渲染 -->
    <header
      v-if="title || $slots.actions"
      class="flex shrink-0 items-center justify-between gap-3 border-b border-border px-4 py-3"
    >
      <div v-if="title" class="min-w-0">
        <h2 class="truncate text-sm font-semibold text-zinc-900">{{ title }}</h2>
        <p v-if="subtitle" class="mt-0.5 truncate text-xs text-zinc-500">{{ subtitle }}</p>
      </div>
      <div class="flex shrink-0 items-center gap-2"><slot name="actions" /></div>
    </header>

    <div class="min-h-0 flex-1 overflow-auto" :class="padded ? 'p-4' : ''">
      <slot />
    </div>
  </section>
</template>
