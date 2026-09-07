import { createApp } from 'vue'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import App from './App.vue'
import router from './router'
import i18n from './i18n'
import { initializeDatabase } from './utils/db'

import './assets/main.css'

async function bootstrap() {
  try {
    await initializeDatabase()
    console.log('[Main] ✅ Database initialized successfully')

    const pinia = createPinia()
    pinia.use(piniaPluginPersistedstate)

    const app = createApp(App)
    app.use(pinia)
    app.use(router)
    app.use(i18n)
    app.mount('#app')
  } catch (error) {
    console.error('[Main] ❌ Fatal: Failed to initialize database', error)
    const message = error instanceof Error ? error.message : String(error)
    const safe = message.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    document.body.innerHTML =
      '<h1 style="color:red; padding:20px; font-family: sans-serif;">' +
      'Database Initialization Failed. Check Console.<br>' +
      '<pre style="margin-top:16px; font-size:14px; color:#c0392b; white-space:pre-wrap;">' +
      safe +
      '</pre></h1>'
  }
}

bootstrap()