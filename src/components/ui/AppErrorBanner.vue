<script setup lang="ts">
import { X } from '@lucide/vue'
import { AlertCircle } from '@lucide/vue'
import AppButton from './AppButton.vue'

interface ErrorAction {
  label: string
  action: () => void
  variant?: 'primary' | 'secondary'
}

const props = defineProps<{
  message: string
  hint?: string
  actions?: ErrorAction[]
  dismissible?: boolean
}>()

defineEmits<{ (e: 'dismiss'): void }>()

// Use props in script to satisfy TypeScript (they're used in template)
const { message, hint, actions, dismissible } = props
</script>

<template>
  <div class="error-banner" :class="{ 'error-banner--with-actions': actions?.length }">
    <div class="error-banner__content">
      <AlertCircle class="h-4 w-4 text-rose-500 flex-shrink-0" />
      <div class="flex-1 min-w-0">
        <p class="text-xs font-medium text-rose-800">{{ message }}</p>
        <p v-if="hint" class="mt-1 text-[11px] text-rose-600">{{ hint }}</p>
      </div>
    </div>
    <div v-if="actions?.length" class="error-banner__actions">
      <AppButton v-for="a in actions" :key="a.label" :variant="a.variant ?? 'secondary'" @click="a.action">
        {{ a.label }}
      </AppButton>
    </div>
    <button v-if="dismissible" @click="$emit('dismiss')" class="error-banner__dismiss" aria-label="关闭">
      <X class="h-3.5 w-3.5" />
    </button>
  </div>
</template>

<style scoped>
.error-banner {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 14px;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 8px;
  color: #991b1b;
}

.error-banner__content {
  display: flex;
  gap: 10px;
  flex: 1;
  min-width: 0;
}

.error-banner__actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-left: 26px;
}

.error-banner__dismiss {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: #e11d48;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.08s ease;
}

.error-banner__dismiss:hover {
  background: #fee2e2;
}

@media (prefers-reduced-motion: reduce) {
  .error-banner__dismiss {
    transition: none;
  }
}
</style>