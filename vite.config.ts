import tailwind from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { defineConfig } from 'vite'
import { nodePolyfills } from 'vite-plugin-node-polyfills'
import topLevelAwait from 'vite-plugin-top-level-await'
import vueDevTools from 'vite-plugin-vue-devtools'
import { version as pkgVersion } from './package.json'

const HOST = process.env.TAURI_DEV_HOST
const PLATFORM = process.env.TAURI_ENV_PLATFORM
process.env.VITE_APP_VERSION = pkgVersion
if (process.env.NODE_ENV === 'production') {
  process.env.VITE_APP_BUILD_EPOCH = new Date().getTime().toString()
}

// https://vitejs.dev/config/
export default defineConfig({
  define: {
    // Disable vue-i18n JIT compilation for CSP compliance in Tauri production builds
    __INTLIFY_JIT_COMPILATION__: JSON.stringify(false),
    // Default locale for first-launch (set via DEFAULT_LOCALE env var at build time).
    // e.g. DEFAULT_LOCALE=zh-CN pnpm build   → Chinese installer
    //      DEFAULT_LOCALE=en    pnpm build   → English installer
    __DEFAULT_LOCALE__: JSON.stringify(process.env.DEFAULT_LOCALE || ''),
  },
  plugins: [
    tailwind(),
    topLevelAwait(),
    nodePolyfills(),
    vue({
      template: {
        compilerOptions: {
          // Treat iconify-icon as a native web component
          isCustomElement: (tag) => tag === 'iconify-icon',
        },
      },
    }),
    vueDevTools(),
    AutoImport({
      imports: [
        'vue',
        'vue-router',
        'pinia',
        {
          '@/store': ['useStore'],
        },
      ],
      dts: 'auto-imports.d.ts',
      vueTemplate: true,
    }),
    Components({
      dts: 'components.d.ts',
    }),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  css: {
    preprocessorMaxWorkers: true,
  },

  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  server: {
    port: 1420,
    strictPort: true,
    host: HOST || false,
    hmr: HOST
      ? {
        protocol: 'ws',
        host: HOST,
        port: 1421,
      }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    outDir: './dist',
    // See https://web-platform-dx.github.io/web-features/ for Vite 8 default targets (baseline-widely-available)
    // See https://v2.tauri.app/reference/webview-versions/ for Tauri details
    target: PLATFORM == 'windows' ? 'chrome111' : 'safari16.4',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    emptyOutDir: true,
    chunkSizeWarningLimit: 3072,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (id.includes('iconify-icon') || id.includes('@iconify-json/mdi')) {
              return 'vendor-icons'
            }
            if (id.includes('/vue/') || id.includes('/vue-router/') || id.includes('/pinia/') || id.includes('/vue-i18n/')) {
              return 'vendor-vue'
            }
            if (id.includes('@tauri-apps')) {
              return 'vendor-tauri'
            }
          }
        },
      },
    },
  },
})
