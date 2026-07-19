<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
// import { listen } from '@tauri-apps/api/event' // 后续需要时取消注释

onMounted(() => {
  // 预留 Tauri 全局事件监听
  console.log('App mounted')
})

onUnmounted(() => {
  // 清理逻辑
})
</script>

<template>
  <div
    class="h-screen w-screen overflow-hidden bg-slate-50 text-slate-900 antialiased"
    data-tauri-drag-region
  >
    <router-view v-slot="{ Component }">
      <transition name="fade" mode="out-in">
        <component :is="Component" />
      </transition>
    </router-view>
  </div>
</template>

<style>
/* 页面切换过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 150ms ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>