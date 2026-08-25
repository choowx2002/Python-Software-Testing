<script setup lang="ts">
/**
 * 统一状态徽章：图标 + 文字，覆盖 环境状态 / 运行状态
 * status 支持：Ready / Warning / Failed / active / running / completed / idle / 其他
 * pulse 用于 running / active 的动态提示
 */
import { computed } from "vue";
import type { Component } from "vue";
import { AlertCircle, CheckCircle2, Circle, Clock3, XCircle } from "@lucide/vue";

const props = withDefaults(
  defineProps<{
    status: string;
    /** 自定义显示文案（默认直接显示 status） */
    label?: string;
    pulse?: boolean;
  }>(),
  { pulse: false },
);

// 完整字面量类名，保证 Tailwind 扫描到
const toneMap: Record<string, { icon: Component; cls: string }> = {
  Ready: { icon: CheckCircle2, cls: "border-emerald-200 bg-emerald-50 text-emerald-700" },
  completed: { icon: CheckCircle2, cls: "border-emerald-200 bg-emerald-50 text-emerald-700" },
  Warning: { icon: AlertCircle, cls: "border-amber-200 bg-amber-50 text-amber-700" },
  Failed: { icon: XCircle, cls: "border-rose-200 bg-rose-50 text-rose-700" },
  active: { icon: Clock3, cls: "border-sky-200 bg-sky-50 text-sky-700" },
  running: { icon: Clock3, cls: "border-sky-200 bg-sky-50 text-sky-700" },
  idle: { icon: Circle, cls: "border-zinc-200 bg-zinc-50 text-zinc-500" },
};

const meta = computed(
  () => toneMap[props.status] ?? { icon: Circle, cls: "border-zinc-200 bg-zinc-50 text-zinc-600" },
);
</script>

<template>
  <span
    class="inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] font-medium"
    :class="meta.cls"
  >
    <component
      :is="meta.icon"
      class="h-3 w-3"
      :class="pulse ? 'animate-pulse' : ''"
      aria-hidden="true"
    />
    {{ label ?? status }}
  </span>
</template>
