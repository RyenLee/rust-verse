import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import i18n from './locales'
import router from './router'

// Iconify web component — enables <iconify-icon icon="mdi:xxx">
// rendering. Must be imported before app mounts.
// Preload all Material Design Icons (mdi) so the app works fully offline.
import 'iconify-icon'
import { addCollection } from 'iconify-icon'
import mdiIcons from '@iconify-json/mdi/icons.json'

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
