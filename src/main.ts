import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './style.css'

createApp(App).use(createPinia()).mount('#app')

// The popup is frameless and non-navigable: suppress browser-ish affordances.
window.addEventListener('contextmenu', (e) => e.preventDefault())
window.addEventListener('dragover', (e) => e.preventDefault())
window.addEventListener('drop', (e) => e.preventDefault())
