<script setup lang="ts">
/**
 * 统一弹窗：Teleport 到 body + Esc 关闭 + 点击遮罩关闭 + 打开时聚焦关闭按钮
 * 用法：
 *   <AppModal v-model:open="show" title="…" description="…">
 *     <p>正文</p>
 *     <template #footer><AppButton>确定</AppButton></template>
 *   </AppModal>
 */
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { X } from "@lucide/vue";
import { useI18n } from "vue-i18n";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title?: string;
    description?: string;
    /** 弹窗宽度（Tailwind 类，默认 max-w-md） */
    width?: string;
  }>(),
  { width: "max-w-md" },
);

const emit = defineEmits<{
  (e: "update:open", value: boolean): void;
  (e: "close"): void;
}>();

const { t } = useI18n();
const closeBtn = ref<HTMLButtonElement | null>(null);

function close() {
  emit("update:open", false);
  emit("close");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") close();
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      document.addEventListener("keydown", onKeydown);
      // 打开后把焦点移入弹窗，便于键盘用户操作
      await nextTick();
      closeBtn.value?.focus();
    } else {
      document.removeEventListener("keydown", onKeydown);
    }
  },
);

onBeforeUnmount(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="open"
        class="fixed inset-0 z-50 flex items-center justify-center bg-zinc-900/40 p-4"
        @click.self="close"
      >
        <div
          role="dialog"
          aria-modal="true"
          :aria-labelledby="title ? 'app-modal-title' : undefined"
          class="modal-panel w-full rounded-xl bg-white shadow-xl max-h-[90vh]"
          :class="width"
        >
          <!-- 标题栏 -->
          <header class="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
            <div class="min-w-0">
              <h3
                id="app-modal-title"
                class="truncate text-sm font-semibold text-zinc-900"
                :title="title"
              >
                {{ title }}
              </h3>
              <p v-if="description" class="mt-1 text-xs text-zinc-500">{{ description }}</p>
            </div>
            <button
              ref="closeBtn"
              type="button"
              class="shrink-0 rounded-lg p-1.5 text-zinc-400 transition-colors hover:bg-zinc-100 hover:text-zinc-600"
              :aria-label="t('common.close')"
              @click="close"
            >
              <X class="h-4 w-4" />
            </button>
          </header>

          <!-- 正文 -->
          <div class="px-5 py-4 overflow-auto max-h-[50vh]"><slot /></div>

          <!-- 底部操作区（可选） -->
          <footer
            v-if="$slots.footer"
            class="flex items-center justify-end gap-2 border-t border-border px-5 py-3.5"
          >
            <slot name="footer" />
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 160ms ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
.modal-enter-active .modal-panel,
.modal-leave-active .modal-panel {
  transition:
    transform 160ms ease,
    opacity 160ms ease;
}
.modal-enter-from .modal-panel,
.modal-leave-to .modal-panel {
  opacity: 0;
  transform: translateY(8px) scale(0.98);
}
</style>
