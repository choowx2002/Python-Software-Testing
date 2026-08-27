<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { X, ChevronLeft, ChevronRight, Check } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

interface TourStep {
  target: string
  title: string
  content: string
  position?: 'top' | 'bottom' | 'left' | 'right'
}

const props = defineProps<{
  steps: TourStep[]
  onComplete: () => void
  onSkip: () => void
}>()

const { t } = useI18n()
const currentStep = ref(0)
const targetEl = ref<HTMLElement | null>(null)
const cardRef = ref<HTMLDivElement>()

const step = computed(() => props.steps[currentStep.value])

async function findTarget() {
  if (!step.value) return
  await nextTick()
  targetEl.value = document.querySelector(step.value.target) as HTMLElement
  if (!targetEl.value) {
    console.warn(`Tour target not found: ${step.value.target}`)
    nextStep()
  }
}

function nextStep() {
  if (currentStep.value < props.steps.length - 1) {
    currentStep.value++
    findTarget()
  } else {
    props.onComplete()
  }
}

function prevStep() {
  if (currentStep.value > 0) {
    currentStep.value--
    findTarget()
  }
}

function skip() {
  props.onSkip()
}

function positionCard(): Record<string, string> {
  if (!targetEl.value || !step.value) return {}

  const targetRect = targetEl.value.getBoundingClientRect()
  const cardRect = cardRef.value?.getBoundingClientRect()
  const cardWidth = cardRect?.width ?? 320
  const cardHeight = cardRect?.height ?? 200
  const gap = 12

  let top = 0, left = 0

  switch (step.value.position ?? 'bottom') {
    case 'top':
      top = targetRect.top - cardHeight - gap
      left = targetRect.left + (targetRect.width - cardWidth) / 2
      break
    case 'bottom':
      top = targetRect.bottom + gap
      left = targetRect.left + (targetRect.width - cardWidth) / 2
      break
    case 'left':
      top = targetRect.top + (targetRect.height - cardHeight) / 2
      left = targetRect.left - cardWidth - gap
      break
    case 'right':
      top = targetRect.top + (targetRect.height - cardHeight) / 2
      left = targetRect.right + gap
      break
  }

  // Keep within viewport
  const vw = window.innerWidth
  const vh = window.innerHeight
  left = Math.max(12, Math.min(left, vw - cardWidth - 12))
  top = Math.max(12, Math.min(top, vh - cardHeight - 12))

  return { top: `${top}px`, left: `${left}px` }
}

onMounted(() => {
  findTarget()
})

watch(currentStep, findTarget)
</script>

<template>
  <Teleport to="body">
    <Transition name="tour">
      <div v-if="step" class="tour-overlay" @click="skip">
        <!-- Highlight hole -->
        <div
          v-if="targetEl"
          class="tour-highlight"
          :style="{
            top: targetEl.getBoundingClientRect().top + 'px',
            left: targetEl.getBoundingClientRect().left + 'px',
            width: targetEl.getBoundingClientRect().width + 'px',
            height: targetEl.getBoundingClientRect().height + 'px'
          }"
        />

        <!-- Tooltip card -->
        <div
          v-if="targetEl"
          ref="cardRef"
          class="tour-card"
          :style="positionCard()"
          @click.stop
        >
          <div class="tour-header">
            <span class="tour-step-number">{{ currentStep + 1 }} / {{ props.steps.length }}</span>
            <button class="tour-close" @click="skip" aria-label="跳过"><X class="h-4 w-4" /></button>
          </div>
          <h3 class="tour-title">{{ step.title }}</h3>
          <p class="tour-content">{{ step.content }}</p>
          <div class="tour-footer">
            <button v-if="currentStep > 0" class="tour-btn tour-btn--ghost" @click="prevStep">
              <ChevronLeft class="h-3.5 w-3.5" /> {{ t('common.back') }}
            </button>
            <div class="flex-1" />
            <button v-if="currentStep < props.steps.length - 1" class="tour-btn tour-btn--primary" @click="nextStep">
              {{ t('common.next') }} <ChevronRight class="h-3.5 w-3.5" />
            </button>
            <button v-else class="tour-btn tour-btn--primary" @click="props.onComplete">
              <Check class="h-3.5 w-3.5" /> {{ t('common.finish') }}
            </button>
            <button class="tour-btn tour-btn--ghost" @click="skip">{{ t('common.skip') }}</button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.tour-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.5);
}

.tour-highlight {
  position: fixed;
  z-index: 10001;
  border: 2px solid #3b6ef0;
  border-radius: 6px;
  box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.5);
  pointer-events: none;
  animation: tour-highlight-in 200ms ease;
}

@keyframes tour-highlight-in {
  from { box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0); }
  to { box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.5); }
}

.tour-card {
  position: fixed;
  z-index: 10002;
  min-width: 280px;
  max-width: 360px;
  padding: 16px;
  background: #fff;
  border: 1px solid #e4e4e7;
  border-radius: 10px;
  box-shadow: 0 12px 32px rgba(0,0,0,0.15);
  animation: tour-card-in 200ms ease;
}

@keyframes tour-card-in {
  from { opacity: 0; transform: translateY(8px) scale(0.96); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}

.tour-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.tour-step-number {
  font-size: 11px;
  font-weight: 600;
  color: #3b6ef0;
  background: #eef4ff;
  padding: 2px 8px;
  border-radius: 999px;
}

.tour-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: #71717a;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.08s ease, color 0.08s ease;
}

.tour-close:hover {
  background: #f4f4f5;
  color: #18181b;
}

.tour-title {
  margin: 0 0 6px;
  font-size: 14px;
  font-weight: 600;
  color: #18181b;
  line-height: 1.3;
}

.tour-content {
  margin: 0 0 14px;
  font-size: 12px;
  color: #52525b;
  line-height: 1.5;
}

.tour-footer {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tour-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 500;
  border-radius: 6px;
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.08s ease;
}

.tour-btn--primary {
  background: #3b6ef0;
  color: #fff;
  border-color: #3b6ef0;
}

.tour-btn--primary:hover {
  background: #2a53e0;
  border-color: #2a53e0;
}

.tour-btn--ghost {
  background: transparent;
  color: #52525b;
  border-color: transparent;
}

.tour-btn--ghost:hover {
  background: #f4f4f5;
}

@media (prefers-reduced-motion: reduce) {
  .tour-highlight,
  .tour-card {
    animation: none;
  }
  .tour-close,
  .tour-btn {
    transition: none;
  }
}
</style>