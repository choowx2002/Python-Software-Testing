<script setup lang="ts">
/**
 * 统一确认弹窗 —— 替换全部原生 ask()（Tauri 系统对话框）
 * 用法：
 *   <AppConfirmModal
 *     :open="show"
 *     title="Delete project?"
 *     description="…"
 *     confirm-label="Delete"
 *     @confirm="onConfirm"
 *     @update:open="(v) => { if (!v) show = false }"
 *   />
 * variant="danger" → 红色实心确认按钮；variant="primary" → 品牌蓝
 */
import { useI18n } from "vue-i18n";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";

withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    variant?: "danger" | "primary";
    loading?: boolean;
  }>(),
  { variant: "danger", loading: false },
);

const emit = defineEmits<{
  (e: "update:open", value: boolean): void;
  (e: "confirm"): void;
  (e: "cancel"): void;
}>();

const { t } = useI18n();

function onCancel() {
  emit("update:open", false);
  emit("cancel");
}
</script>

<template>
  <AppModal :open="open" :title="title" :description="description" @update:open="onCancel">
    <slot />
    <template #footer>
      <AppButton variant="ghost" :disabled="loading" @click="onCancel">
        {{ cancelLabel ?? t("common.cancel") }}
      </AppButton>
      <AppButton
        :variant="variant === 'danger' ? 'dangerSolid' : 'primary'"
        :loading="loading"
        @click="emit('confirm')"
      >
        {{ confirmLabel ?? t("common.confirm") }}
      </AppButton>
    </template>
  </AppModal>
</template>
