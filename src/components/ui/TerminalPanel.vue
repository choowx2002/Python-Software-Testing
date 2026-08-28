<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  ChevronDown,
  ChevronLeft,
  Copy,
  Eraser,
  Moon,
  MoveDown,
  SquareTerminal,
  Sun,
  WrapText,
} from "@lucide/vue";
import { useUIStore } from "../../stores/uiStore";

export interface TerminalLine {
  stream: "stdout" | "stderr";
  line: string;
  logId?: number;
}

const props = withDefaults(
  defineProps<{
    title: string;
    lines: TerminalLine[];
    /** 运行中 → 自动展开 */
    active?: boolean;
    /** 用于按页持久化宽度（存比例） */
    widthKey?: string;
  }>(),
  { active: false, widthKey: "default" },
);

const emit = defineEmits<{
  (e: "clear"): void;
}>();

const { t } = useI18n();
const uiStore = useUIStore();

const MIN_PX = 160;
const MAX_PX = 480;
const DEFAULT_PX = 320;

const winW = ref(window.innerWidth);

/* ---------------- 宽度（按比例持久化） ---------------- */
const ratio = ref(uiStore.terminalWidths[props.widthKey] ?? DEFAULT_PX / winW.value);
const widthPx = computed(() =>
  Math.max(MIN_PX, Math.min(MAX_PX, Math.round(ratio.value * winW.value))),
);

function persistRatio() {
  uiStore.setTerminalWidth(props.widthKey, ratio.value);
}

function onWindowResize() {
  winW.value = window.innerWidth;
}

/* ---------------- 主题（浅色默认 / 深色） ---------------- */
const dark = ref(uiStore.terminalDark);
function toggleTheme() {
  dark.value = !dark.value;
  uiStore.setTerminalDark(dark.value);
}

const theme = computed(() =>
  dark.value
    ? {
        panel: "bg-zinc-900 border-zinc-800",
        handle: "hover:bg-brand-600",
        header: "border-zinc-800",
        title: "text-zinc-200",
        badge: "bg-zinc-800 text-zinc-400",
        icon: "text-brand-400",
        btn: "text-zinc-400 hover:bg-zinc-800 hover:text-white",
        btnActive: "text-brand-400",
        stdout: "text-zinc-300",
        stderr: "text-rose-300",
        emptyIcon: "text-zinc-700",
        emptyText: "text-zinc-500",
        backBtn: "border-zinc-700 bg-zinc-800 text-zinc-300 hover:bg-zinc-700 hover:text-white",
        tabIcon: "text-brand-400",
        tabTitle: "text-zinc-500",
        tabCount: "text-zinc-400",
        tabChevron: "text-zinc-600",
      }
    : {
        panel: "bg-white border-border",
        handle: "hover:bg-brand-300",
        header: "border-border",
        title: "text-zinc-900",
        badge: "bg-zinc-100 text-zinc-500",
        icon: "text-brand-500",
        btn: "text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800",
        btnActive: "text-brand-500",
        stdout: "text-zinc-700",
        stderr: "text-rose-600",
        emptyIcon: "text-zinc-300",
        emptyText: "text-zinc-400",
        backBtn: "border-border bg-white text-zinc-600 hover:bg-zinc-100 hover:text-zinc-800",
        tabIcon: "text-brand-500",
        tabTitle: "text-zinc-500",
        tabCount: "text-zinc-400",
        tabChevron: "text-zinc-400",
      },
);

/* ---------------- 展开 / 折叠 ---------------- */
const expanded = ref(false);
const follow = ref(true);
const wrap = ref(false);
const bodyEl = ref<HTMLElement | null>(null);
const showBackToBottom = ref(false);

/** 记录"上次是否有内容/运行中"，用于自动折叠判定 */
const wasContent = ref(props.active || props.lines.length > 0);

watch(
  () => [props.active, props.lines.length] as const,
  () => {
    if (props.active) {
      expanded.value = true;
    } else if (props.lines.length === 0 && wasContent.value) {
      expanded.value = false;
    }
    wasContent.value = props.active || props.lines.length > 0;
  },
);

watch(
  () => props.lines.length,
  async () => {
    if (!follow.value || !expanded.value) return;
    await nextTick();
    scrollToBottom();
  },
);

function scrollToBottom() {
  const el = bodyEl.value;
  if (el) el.scrollTop = el.scrollHeight;
}

function onBodyScroll() {
  const el = bodyEl.value;
  if (!el) return;
  const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
  follow.value = nearBottom;
  showBackToBottom.value = !nearBottom;
}

function goToBottom() {
  follow.value = true;
  showBackToBottom.value = false;
  scrollToBottom();
}

async function copyLogs() {
  const text = props.lines.map((l) => l.line).join("\n");
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    console.error("[Terminal] copy failed:", error);
  }
}

function toggleExpanded() {
  expanded.value = !expanded.value;
  if (expanded.value) {
    follow.value = true;
    void nextTick(scrollToBottom);
  }
}

/* ---------------- 横向拖拽调宽 ---------------- */
let dragState: { startX: number; startWidth: number } | null = null;

function startResize(e: MouseEvent) {
  e.preventDefault();
  dragState = { startX: e.clientX, startWidth: widthPx.value };
  window.addEventListener("mousemove", onResize);
  window.addEventListener("mouseup", stopResize);
}

function onResize(e: MouseEvent) {
  if (!dragState) return;
  const deltaX = dragState.startX - e.clientX;
  const next = Math.max(MIN_PX, Math.min(MAX_PX, dragState.startWidth + deltaX));
  ratio.value = next / winW.value;
}

function stopResize() {
  if (dragState) {
    persistRatio();
  }
  dragState = null;
  window.removeEventListener("mousemove", onResize);
  window.removeEventListener("mouseup", stopResize);
}

const hasStderr = computed(() => props.lines.some((l) => l.stream === "stderr"));

window.addEventListener("resize", onWindowResize);
onBeforeUnmount(() => {
  stopResize();
  persistRatio();
  window.removeEventListener("resize", onWindowResize);
});

defineExpose({ toggle: toggleExpanded });
</script>

<template>
  <div
    class="relative flex h-full shrink-0 flex-col overflow-hidden rounded-xl border shadow-sm transition-[width] duration-200"
    :class="theme.panel"
    :style="{ width: expanded ? `${widthPx}px` : '44px' }"
  >
    <!-- 拖拽手柄（左边缘，展开时显示） -->
    <div
      v-if="expanded"
      class="absolute -left-1 top-0 z-10 h-full w-1 cursor-col-resize rounded transition-colors"
      :class="theme.handle"
      title="拖拽调整宽度"
      @mousedown="startResize"
    />

    <!-- ═══ 展开态：页头 + 输出 ═══ -->
    <template v-if="expanded">
      <div class="flex h-9 shrink-0 items-center justify-between gap-2 border-b px-3" :class="theme.header">
        <button
          type="button"
          class="flex min-w-0 items-center gap-2 text-left"
          :title="t('terminal.collapse')"
          @click="toggleExpanded"
        >
          <SquareTerminal class="h-3.5 w-3.5 shrink-0" :class="theme.icon" />
          <span class="truncate text-[11px] font-medium" :class="theme.title">{{ title }}</span>
          <span class="shrink-0 rounded-full px-1.5 py-0.5 font-mono text-[10px]" :class="theme.badge">
            {{ t("terminal.lines", { count: lines.length }) }}
          </span>
        </button>

        <div class="flex shrink-0 items-center gap-0.5">
          <button
            type="button"
            class="rounded-md p-1.5 transition-colors"
            :class="[theme.btn, follow ? theme.btnActive : '']"
            :title="follow ? t('terminal.unfollow') : t('terminal.follow')"
            @click="follow = !follow"
          >
            <MoveDown class="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            class="rounded-md p-1.5 transition-colors"
            :class="[theme.btn, wrap ? theme.btnActive : '']"
            :title="wrap ? t('terminal.unwrap') : t('terminal.wrap')"
            @click="wrap = !wrap"
          >
            <WrapText class="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            class="rounded-md p-1.5 transition-colors"
            :class="theme.btn"
            :title="t('common.copy')"
            @click="copyLogs"
          >
            <Copy class="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            class="rounded-md p-1.5 transition-colors"
            :class="theme.btn"
            :title="t('common.clear')"
            @click="emit('clear')"
          >
            <Eraser class="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            class="rounded-md p-1.5 transition-colors"
            :class="theme.btn"
            :title="dark ? t('terminal.lightTheme') : t('terminal.darkTheme')"
            @click="toggleTheme"
          >
            <Moon v-if="dark" class="h-3.5 w-3.5" />
            <Sun v-else class="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            class="ml-1 rounded-md p-1.5 transition-colors"
            :class="theme.btn"
            :title="t('terminal.collapse')"
            @click="toggleExpanded"
          >
            <ChevronDown class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      <div class="relative min-h-0 flex-1">
        <!-- 空状态 -->
        <div
          v-if="!active && lines.length === 0"
          class="flex h-full flex-col items-center justify-center px-4 text-center"
        >
          <SquareTerminal class="mb-2 h-6 w-6" :class="theme.emptyIcon" />
          <p class="text-[11px] leading-5" :class="theme.emptyText">{{ t("terminal.empty") }}</p>
        </div>

        <div
          v-else
          ref="bodyEl"
          class="h-full overflow-auto px-3 py-2 font-mono text-[11px] leading-5"
          :class="wrap ? 'whitespace-pre-wrap wrap-break-word' : 'whitespace-pre'"
          @scroll="onBodyScroll"
        >
          <template v-for="output in lines" :key="output.logId ?? `${output.stream}-${output.line}`">
            <span :class="output.stream === 'stderr' ? theme.stderr : theme.stdout">{{
              output.line
            }}</span>{{ "\n" }}
          </template>
        </div>

        <button
          v-if="showBackToBottom"
          type="button"
          class="absolute bottom-2 right-3 inline-flex items-center gap-1 rounded-md border px-2 py-1 text-[10px] font-medium shadow transition-colors"
          :class="theme.backBtn"
          @click="goToBottom"
        >
          <MoveDown class="h-3 w-3" />
          {{ t("terminal.backToBottom") }}
        </button>
      </div>
    </template>

    <!-- ═══ 收起态：垂直窄 tab ═══ -->
    <template v-else>
      <button
        type="button"
        class="flex h-full w-full flex-col items-center gap-2 py-2"
        :title="t('terminal.expand')"
        @click="toggleExpanded"
      >
        <SquareTerminal class="h-4 w-4 shrink-0" :class="theme.tabIcon" />
        <span class="text-[10px] font-medium leading-tight [writing-mode:vertical-rl]" :class="theme.tabTitle">
          {{ title }}
        </span>
        <span class="flex-1" />
        <span
          v-if="hasStderr"
          class="h-1.5 w-1.5 rounded-full bg-rose-500"
          :title="t('terminal.lines', { count: lines.length })"
        />
        <span
          v-else-if="lines.length > 0"
          class="font-mono text-[10px]"
          :class="theme.tabCount"
        >{{ lines.length }}</span>
        <ChevronLeft class="h-3.5 w-3.5" :class="theme.tabChevron" />
      </button>
    </template>
  </div>
</template>
