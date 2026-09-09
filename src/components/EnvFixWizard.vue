<script setup lang="ts">
/**
 * EnvFixWizard —— 一键修复生成环境（Python 3.11）
 * 流程：安装 Python 3.11（Windows winget / Linux/macOS 用户级下载）→ 重建 .venv → 安装依赖
 *   →（可选）更新项目解释器
 * 进度通过 "env-fix-step" 事件实时展示；完成后 emit('fixed') 由父级刷新状态。
 * 用法：
 *   <EnvFixWizard :project-path="projectPath" :project-id="projectId" @fixed="onFixed" />
 *   Import 流程不传 project-id（项目未保存，由父级重新检测环境）。
 */
import { onBeforeUnmount, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import { CheckCircle2, Loader2, XCircle } from "@lucide/vue";
import AppButton from "./ui/AppButton.vue";

const props = defineProps<{
  projectPath: string;
  projectId?: number;
}>();

const emit = defineEmits<{ (e: "fixed"): void }>();

const { t } = useI18n();

const running = ref(false);
const done = ref(false);
const error = ref<string | null>(null);
const steps = ref<{ stage: string; status: string }[]>([]);

let unlisten: UnlistenFn | undefined;

async function run() {
  if (running.value) return;

  running.value = true;
  done.value = false;
  error.value = null;
  steps.value = [];

  unlisten = await listen<{ stage: string; status: string }>(
    "env-fix-step",
    (event) => {
      const { stage, status } = event.payload;
      const existing = steps.value.find((s) => s.stage === stage);
      if (existing) {
        existing.status = status;
      } else {
        steps.value.push({ stage, status });
      }
    },
  );

  try {
    await invoke<string>("fix_python_env", {
      projectPath: props.projectPath,
      projectId: props.projectId ?? null,
    });
    done.value = true;
    emit("fixed");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    running.value = false;
    unlisten?.();
    unlisten = undefined;
  }
}

/** 阶段 → 可读文案（stage 由 Rust 端发送） */
function stageLabel(stage: string) {
  const key = `envfix.stage.${stage}`;
  return t(key) === key ? stage : t(key);
}

function stageIcon(status: string) {
  if (status === "success") return CheckCircle2;
  if (status === "failed") return XCircle;
  return Loader2;
}

function stageClass(status: string) {
  if (status === "success") return "text-emerald-500";
  if (status === "failed") return "text-rose-500";
  return "text-brand-500";
}

onBeforeUnmount(() => {
  unlisten?.();
});
</script>

<template>
  <div class="min-w-0">
    <div v-if="!running && !done && !error" class="flex items-center gap-2">
      <AppButton variant="primary" size="sm" @click="run">
        {{ t("envfix.run") }}
      </AppButton>
    </div>

    <!-- 进行中：阶段进度列表 -->
    <div v-if="running" class="space-y-1">
      <div class="flex items-center gap-2 text-xs font-medium text-zinc-600">
        <Loader2 class="h-3.5 w-3.5 animate-spin text-brand-500" />
        {{ t("envfix.running") }}
      </div>
      <div
        v-for="step in steps"
        :key="step.stage"
        class="flex items-center gap-2 text-xs text-zinc-500"
      >
        <component
          :is="stageIcon(step.status)"
          class="h-3.5 w-3.5"
          :class="[
            stageClass(step.status),
            step.status === 'running' ? 'animate-spin' : '',
          ]"
        />
        {{ stageLabel(step.stage) }}
      </div>
    </div>

    <!-- 完成 -->
    <div v-if="done" class="flex items-center gap-2 text-xs font-medium text-emerald-600">
      <CheckCircle2 class="h-3.5 w-3.5" />
      {{ t("envfix.done") }}
    </div>

    <!-- 失败 -->
    <div v-if="error" class="space-y-1.5">
      <div class="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-700">
        {{ error }}
      </div>
      <AppButton variant="secondary" size="sm" @click="run">
        {{ t("common.retry") }}
      </AppButton>
    </div>
  </div>
</template>
