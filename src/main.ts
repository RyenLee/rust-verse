import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import i18n from './locales'
import router from './router'

// Iconify web component — enables <iconify-icon icon="mdi:xxx">
// rendering. Must be imported before app mounts.
// P0: Use build-time extracted minimal icon set instead of full @iconify-json/mdi
// (29KB vs 3MB, only includes icons actually referenced in source code).
import 'iconify-icon'
import { addCollection } from 'iconify-icon'
import mdiIcons from './assets/used-icons.json'

addCollection(mdiIcons)

import './assets/main.css'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(i18n)
app.use(router)

// Mount immediately so the splash screen shows without delay.
// Locale initialization is handled in App.vue onMounted,
// which runs after the splash screen is already visible.
app.mount('#app')
