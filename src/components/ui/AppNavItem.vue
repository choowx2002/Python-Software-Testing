<script setup lang="ts">
/**
 * 侧边栏导航项：图标 + 标题 +（可选）描述；支持 active / disabled 态
 */
import type { Component } from "vue";

withDefaults(
  defineProps<{
    icon: Component;
    label: string;
    description?: string;
    active?: boolean;
    disabled?: boolean;
  }>(),
  { active: false, disabled: false },
);

defineEmits<{ (e: "click"): void }>();
</script>

<template>
  <button
    type="button"
    class="group flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-50"
    :class="active ? 'bg-zinc-100' : 'hover:bg-zinc-50'"
    :disabled="disabled"
    @click="$emit('click')"
  >
    <span
      class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md transition-colors"
      :class="
        active
          ? 'bg-brand-500 text-white'
          : 'bg-zinc-100 text-zinc-500 group-hover:bg-zinc-200/70 group-disabled:group-hover:bg-zinc-100'
      "
    >
      <component :is="icon" class="h-3.5 w-3.5" aria-hidden="true" />
    </span>
    <span class="min-w-0">
      <span
        class="block truncate text-[13px] font-medium"
        :class="active ? 'text-zinc-900' : 'text-zinc-700'"
        >{{ label }}</span
      >
      <span v-if="description" class="block truncate text-[10px] text-zinc-400">{{
        description
      }}</span>
    </span>
  </button>
</template>
