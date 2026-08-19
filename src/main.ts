import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import i18n from './i18n'
import { initializeDatabase } from './utils/db' // 确保路径正确

import './assets/main.css'

async function bootstrap() {
  try {
    await initializeDatabase()
    console.log('[Main] ✅ Database initialized successfully')

    const app = createApp(App)
    app.use(createPinia())
    app.use(router)
    app.use(i18n)
    app.mount('#app')
  } catch (error) {
    console.error('[Main] ❌ Fatal: Failed to initialize database', error)
    document.body.innerHTML = '<h1 style="color:red; padding:20px;">Database Initialization Failed. Check Console.</h1>'
  }
}

bootstrap()