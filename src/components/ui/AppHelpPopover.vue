<script setup lang="ts">
import { ref } from 'vue'
import { HelpCircle } from '@lucide/vue'
// @ts-expect-error Teleport used only in template
import { Teleport } from 'vue'

const props = defineProps<{
  title: string
  content: string
  iconSize?: number
}>()

const open = ref(false)

// Used in template
const { title, content } = props
</script>

<template>
  <span class="relative inline-flex">
    <button
      type="button"
      class="rounded p-0.5 text-zinc-400 hover:text-zinc-600 hover:bg-zinc-100 transition-colors"
      @click="open = !open"
      @blur="open = false"
      aria-label="帮助"
    >
      <HelpCircle :class="iconSize ? `h-${iconSize} w-${iconSize}` : 'h-3.5 w-3.5'" />
    </button>
    <Teleport to="body" v-if="open">
      <Transition name="popover">
        <div class="help-popover-overlay" @click.self="open = false">
          <div class="help-popover-panel" @click.stop>
            <h4 class="text-sm font-semibold text-zinc-900">{{ title }}</h4>
            <p class="mt-1 text-xs leading-5 text-zinc-600">{{ content }}</p>
          </div>
        </div>
      </Transition>
    </Teleport>
  </span>
</template>

<style scoped>
.help-popover-overlay {
  position: fixed;
  inset: 0;
  z-index: 9998;
  background: transparent;
}

.help-popover-panel {
  position: fixed;
  z-index: 9999;
  max-width: 320px;
  padding: 12px 14px;
  background: #fff;
  border: 1px solid #e4e4e7;
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.12);
  animation: popover-in 120ms ease;
}

@keyframes popover-in {
  from { opacity: 0; transform: scale(0.96) translateY(-4px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.popover-enter-active,
.popover-leave-active {
  transition: opacity 120ms ease, transform 120ms ease;
}
.popover-enter-from,
.popover-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}

@media (prefers-reduced-motion: reduce) {
  .help-popover-panel {
    animation: none;
  }
  .popover-enter-active,
  .popover-leave-active {
    transition: none;
  }
}
</style>