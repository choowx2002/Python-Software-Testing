// src/main.ts
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import App from './App.vue'
import { initializeDatabase } from './utils/db'

import './assets/main.css'

// 在应用启动前初始化数据库
async function bootstrap() {
  try {
    console.log('[App] Initializing database...')
    await initializeDatabase()
    console.log('[App] Database ready, starting application...')
  } catch (error) {
    console.error('[App] Failed to initialize database:', error)
    // 可以选择显示错误提示或继续启动（表可能已存在）
  }

  const app = createApp(App)
  app.use(createPinia())
  app.use(router)
  app.mount('#app')
}

bootstrap()