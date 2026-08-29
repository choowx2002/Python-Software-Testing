<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { HelpCircle } from '@lucide/vue'

const props = defineProps<{
  title: string
  content: string
  iconSize?: number
}>()

const open = ref(false)
const triggerRef = ref<HTMLButtonElement>()
const panelRef = ref<HTMLDivElement>()
const panelStyle = ref<{ left: string; top: string } | undefined>(undefined)

const GAP = 6
const EDGE_MARGIN = 8

function positionPanel() {
  const trigger = triggerRef.value
  const panel = panelRef.value
  if (!trigger || !panel) return
  const triggerRect = trigger.getBoundingClientRect()
  const w = panel.offsetWidth
  const h = panel.offsetHeight
  let left = triggerRect.left
  let top = triggerRect.bottom + GAP
  // 下方放不下时翻转到按钮上方
  if (top + h + EDGE_MARGIN > window.innerHeight) {
    top = Math.max(EDGE_MARGIN, triggerRect.top - h - GAP)
  }
  // 靠右边缘时左移，避免溢出窗口
  if (left + w + EDGE_MARGIN > window.innerWidth) {
    left = Math.max(EDGE_MARGIN, window.innerWidth - w - EDGE_MARGIN)
  }
  panelStyle.value = { left: `${left}px`, top: `${top}px` }
}

watch(open, (o) => {
  if (o) nextTick(positionPanel)
})

// Used in template
const { title, content } = props
</script>

<template>
  <span class="relative inline-flex">
    <button
      ref="triggerRef"
      type="button"
      class="rounded p-0.5 text-zinc-400 hover:text-zinc-600 hover:bg-zinc-100 transition-colors"
      @click="open = !open"
      @blur="open = false"
      aria-label="帮助"
    >
      <HelpCircle :class="iconSize ? `h-${iconSize} w-${iconSize}` : 'h-3.5 w-3.5'" />
    </button>
    <Teleport to="body">
      <Transition name="popover">
        <div v-if="open" class="help-popover-overlay" @click.self="open = false">
          <div ref="panelRef" class="help-popover-panel" :style="panelStyle" @click.stop>
            <h4 class="text-sm font-semibold text-zinc-900">{{ title }}</h4>
            <p class="mt-1 whitespace-pre-line text-xs leading-5 text-zinc-600">{{ content }}</p>
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