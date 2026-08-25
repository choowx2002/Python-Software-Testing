<script setup lang="ts">
/**
 * 统一按钮 —— 全部样式来自 main.css 的 .btn 系列基类（单一事实来源）
 * 用法：<AppButton variant="primary" size="sm" :loading="busy" @click="go">…
 */
import { Loader2 } from "@lucide/vue";

type Variant = "primary" | "secondary" | "ghost" | "danger" | "dangerSolid";
type Size = "sm" | "md";

withDefaults(
  defineProps<{
    variant?: Variant;
    size?: Size;
    loading?: boolean;
    disabled?: boolean;
    type?: "button" | "submit";
    ariaLabel?: string;
  }>(),
  {
    variant: "secondary",
    size: "md",
    loading: false,
    disabled: false,
    type: "button",
  },
);

const emit = defineEmits<{ (e: "click", ev: MouseEvent): void }>();

// 变体 -> 基类（完整字面量，保证 Tailwind 可扫描到）
const variantClass: Record<Variant, string> = {
  primary: "btn-primary",
  secondary: "btn-secondary",
  ghost: "btn-ghost",
  danger: "btn-danger",
  dangerSolid: "btn-danger-solid",
};
</script>

<template>
  <button
    :type="type"
    class="btn"
    :class="[size === 'sm' ? 'btn-sm' : 'btn-md', variantClass[variant]]"
    :disabled="disabled || loading"
    :aria-label="ariaLabel"
    :aria-busy="loading || undefined"
    @click="(ev: MouseEvent) => emit('click', ev)"
  >
    <!-- loading 时显示 spinner，隐藏原有内容以保证宽度稳定 -->
    <Loader2 v-if="loading" class="h-3.5 w-3.5 animate-spin" aria-hidden="true" />
    <template v-else><slot /></template>
  </button>
</template>
