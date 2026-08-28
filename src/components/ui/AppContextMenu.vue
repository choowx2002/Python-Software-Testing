<script setup lang="ts">
import { computed, nextTick, onMounted, onBeforeUnmount, ref, watch } from 'vue'

interface MenuItem {
  label?: string
  icon?: any
  action?: () => void
  disabled?: boolean
  divider?: boolean
  danger?: boolean
}

const props = defineProps<{
  items: MenuItem[]
  open: boolean
  /** 鼠标点击的视口坐标（clientX/clientY），用于定位菜单 */
  x: number
  y: number
  onClose: () => void
}>()

const menuRef = ref<HTMLDivElement>()
const focusedIndex = ref(-1)
const panelStyle = ref<{ left: string; top: string } | undefined>(undefined)

const EDGE_MARGIN = 8

function positionMenu() {
  const el = menuRef.value
  if (!el) return
  const w = el.offsetWidth
  const h = el.offsetHeight
  let left = props.x
  let top = props.y
  // 靠近右边缘时左移，避免溢出窗口
  if (left + w + EDGE_MARGIN > window.innerWidth) {
    left = Math.max(EDGE_MARGIN, window.innerWidth - w - EDGE_MARGIN)
  }
  // 靠近底边缘时翻转到鼠标上方，避免溢出窗口
  if (top + h + EDGE_MARGIN > window.innerHeight) {
    top = Math.max(EDGE_MARGIN, props.y - h - EDGE_MARGIN)
  }
  panelStyle.value = { left: `${left}px`, top: `${top}px` }
}

watch(
  () => [props.open, props.x, props.y],
  () => {
    if (props.open) nextTick(positionMenu)
  },
  { immediate: true }
)

const actionableItems = computed(() =>
  props.items.map((item, i) => ({ ...item, index: i })).filter(i => !i.divider)
)

function handleKeydown(e: KeyboardEvent) {
  if (!actionableItems.value.length) return
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      focusedIndex.value = Math.min(focusedIndex.value + 1, actionableItems.value.length - 1)
      break
    case 'ArrowUp':
      e.preventDefault()
      focusedIndex.value = Math.max(focusedIndex.value - 1, 0)
      break
    case 'Enter':
    case ' ':
      e.preventDefault()
      actionableItems.value[focusedIndex.value]?.action?.()
      props.onClose()
      break
    case 'Escape':
      props.onClose()
      break
  }
}

function handleItemClick(item: MenuItem) {
  if (item.disabled || !item.action) return
  item.action()
  props.onClose()
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
  nextTick(() => { focusedIndex.value = 0 })
})
onBeforeUnmount(() => document.removeEventListener('keydown', handleKeydown))
</script>

<template>
  <Teleport to="body">
    <Transition name="contextmenu">
      <div v-if="open" class="contextmenu-overlay" @click="onClose">
        <div
          ref="menuRef"
          class="contextmenu-panel"
          role="menu"
          :style="panelStyle"
          @click.stop
        >
          <template v-for="(item, idx) in items" :key="idx">
            <hr v-if="item.divider" class="contextmenu-divider" />
            <button
              v-else
              type="button"
              role="menuitem"
              class="contextmenu-item"
              :class="{ 'contextmenu-item--danger': item.danger, 'contextmenu-item--disabled': item.disabled }"
              :disabled="item.disabled"
              @click="handleItemClick(item)"
            >
              <component v-if="item.icon" :is="item.icon" class="h-3.5 w-3.5 shrink-0" />
              <span class="truncate">{{ item.label }}</span>
            </button>
          </template>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.contextmenu-overlay {
  position: fixed;
  inset: 0;
  z-index: 9998;
  background: transparent;
}

.contextmenu-panel {
  position: fixed;
  z-index: 9999;
  min-width: 160px;
  max-width: 280px;
  padding: 4px;
  background: #fff;
  border: 1px solid #e4e4e7;
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.12);
  animation: contextmenu-in 100ms ease;
}

@keyframes contextmenu-in {
  from { opacity: 0; transform: scale(0.96) translateY(-4px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.contextmenu-divider {
  margin: 4px 0;
  border: none;
  border-top: 1px solid #e4e4e7;
}

.contextmenu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: #18181b;
  font-size: 12px;
  font-weight: 500;
  text-align: left;
  cursor: pointer;
  transition: background 0.08s ease, color 0.08s ease;
}

.contextmenu-item:hover:not(.contextmenu-item--disabled) {
  background: #f4f4f5;
}

.contextmenu-item:focus-visible {
  outline: 2px solid #3b6ef0;
  outline-offset: -2px;
}

.contextmenu-item--disabled {
  color: #a1a1aa;
  cursor: not-allowed;
}

.contextmenu-item--danger {
  color: #e11d48;
}

.contextmenu-item--danger:hover:not(.contextmenu-item--disabled) {
  background: #fff1f2;
}

@media (prefers-reduced-motion: reduce) {
  .contextmenu-panel {
    animation: none;
  }
}
</style>