<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useGlobalShortcuts } from './composables/useKeyboardShortcuts'
import { useUIStore } from './stores/uiStore'
import HistoryDetailWindow from './views/HistoryDetailWindow.vue'

const router = useRouter()
const uiStore = useUIStore()

// 独立详情窗口（label 为 detail）直接渲染详情视图，跳过正常路由
const isDetailWindow = getCurrentWindow().label === 'detail'

// 全局快捷键（Input/Textarea 聚焦时自动忽略）；详情窗口不注册，避免误触发
if (!isDetailWindow) {
  useGlobalShortcuts({
    'ctrl+n': () => void router.push('/projects/import'),
    'ctrl+k': () => window.dispatchEvent(new CustomEvent('testmate:focus-search')),
    'ctrl+f': () => window.dispatchEvent(new CustomEvent('testmate:focus-search')),
    'f5': () => window.dispatchEvent(new CustomEvent('testmate:refresh')),
    'ctrl+b': () => uiStore.toggleSidebar(),
  })
}

onMounted(() => {
  if (!isDetailWindow) console.log('App mounted')
})

onUnmounted(() => {
  // 清理逻辑
})
</script>

<template>
  <div class="h-screen w-screen overflow-hidden bg-surface text-zinc-900 antialiased">
    <HistoryDetailWindow v-if="isDetailWindow" />
    <router-view v-else v-slot="{ Component }">
      <transition name="fade" mode="out-in">
        <component :is="Component" />
      </transition>
    </router-view>
  </div>
</template>

<style>
/* 页面切换过渡动画（尊重系统减少动态效果偏好） */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 150ms ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .fade-enter-active,
  .fade-leave-active {
    transition: none;
  }
}
</style>
