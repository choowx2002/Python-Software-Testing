<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch, nextTick } from 'vue'

const props = withDefaults(
  defineProps<{
    content: string
    position?: 'top' | 'bottom' | 'left' | 'right'
    delay?: number
  }>(),
  {
    position: 'top',
    delay: 200,
  },
)

const trigger = ref<HTMLElement | null>(null)
const tooltipEl = ref<HTMLElement | null>(null)
const show = ref(false)
const pos = ref({ top: 0, left: 0 })
let timeout: number | undefined
let hideTimeout: number | undefined

onMounted(() => {
  window.addEventListener('scroll', hide, true)
  window.addEventListener('resize', hide)
})

onBeforeUnmount(() => {
  window.removeEventListener('scroll', hide, true)
  window.removeEventListener('resize', hide)
  window.clearTimeout(timeout)
  window.clearTimeout(hideTimeout)
})

watch(() => props.content, hide)

function showTooltip() {
  window.clearTimeout(timeout)
  window.clearTimeout(hideTimeout)
  timeout = window.setTimeout(async () => {
    show.value = true
    await nextTick()
    positionTooltip()
  }, props.delay)
}

function hide() {
  window.clearTimeout(timeout)
  window.clearTimeout(hideTimeout)
  hideTimeout = window.setTimeout(() => {
    show.value = false
  }, 100)
}

function keepAlive() {
  window.clearTimeout(timeout)
  window.clearTimeout(hideTimeout)
}

function positionTooltip() {
  if (!trigger.value || !tooltipEl.value) return

  const triggerRect = trigger.value.getBoundingClientRect()
  const elRect = tooltipEl.value.getBoundingClientRect()
  const gap = 8
  const margin = 8

  let top = 0
  let left = 0

  switch (props.position) {
    case 'top':
      top = triggerRect.top - elRect.height - gap
      left = triggerRect.left + (triggerRect.width - elRect.width) / 2
      break
    case 'bottom':
      top = triggerRect.bottom + gap
      left = triggerRect.left + (triggerRect.width - elRect.width) / 2
      break
    case 'left':
      top = triggerRect.top + (triggerRect.height - elRect.height) / 2
      left = triggerRect.left - elRect.width - gap
      break
    case 'right':
      top = triggerRect.top + (triggerRect.height - elRect.height) / 2
      left = triggerRect.right + gap
      break
  }

  // Clamp within viewport
  const vw = window.innerWidth
  const vh = window.innerHeight
  left = Math.max(margin, Math.min(left, vw - elRect.width - margin))
  top = Math.max(margin, Math.min(top, vh - elRect.height - margin))

  pos.value = { top, left }
}
</script>

<template>
  <span
    ref="trigger"
    class="relative inline-block"
    @mouseenter="showTooltip"
    @mouseleave="hide"
    @focus="showTooltip"
    @blur="hide"
  >
    <slot />
    <Teleport to="body">
      <Transition name="tooltip">
        <div
          v-if="show"
          ref="tooltipEl"
          class="tooltip-arrow"
          :data-position="position"
          :style="{ top: `${pos.top}px`, left: `${pos.left}px` }"
          @mouseenter="keepAlive"
          @mouseleave="hide"
        >
          {{ content }}
        </div>
      </Transition>
    </Teleport>
  </span>
</template>

<style scoped>
.tooltip-arrow {
  position: fixed;
  z-index: 9999;
  max-width: 280px;
  padding: 6px 10px;
  background: #18181b;
  color: #fafafa;
  font-size: 11px;
  line-height: 1.4;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  pointer-events: none;
  white-space: normal;
  word-break: break-word;
}

.tooltip-arrow[data-position='top']::after {
  content: '';
  position: absolute;
  bottom: -5px;
  left: 50%;
  transform: translateX(-50%);
  border-width: 5px 5px 0;
  border-style: solid;
  border-color: #18181b transparent transparent;
}

.tooltip-arrow[data-position='bottom']::after {
  content: '';
  position: absolute;
  top: -5px;
  left: 50%;
  transform: translateX(-50%);
  border-width: 0 5px 5px;
  border-style: solid;
  border-color: transparent transparent #18181b;
}

.tooltip-arrow[data-position='left']::after {
  content: '';
  position: absolute;
  right: -5px;
  top: 50%;
  transform: translateY(-50%);
  border-width: 5px 0 5px 5px;
  border-style: solid;
  border-color: transparent transparent transparent #18181b;
}

.tooltip-arrow[data-position='right']::after {
  content: '';
  position: absolute;
  left: -5px;
  top: 50%;
  transform: translateY(-50%);
  border-width: 5px 5px 5px 0;
  border-style: solid;
  border-color: transparent #18181b transparent transparent;
}

.tooltip-enter-active,
.tooltip-leave-active {
  transition:
    opacity 120ms ease,
    transform 120ms ease;
}
.tooltip-enter-from,
.tooltip-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

@media (prefers-reduced-motion: reduce) {
  .tooltip-enter-active,
  .tooltip-leave-active {
    transition: none;
  }
}
</style>