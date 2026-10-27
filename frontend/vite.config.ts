import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vuetify({ autoImport: true }),
  ],
  server: {
    port: 8001,
    host: '0.0.0.0',
    headers: {
      'Cache-Control': 'no-store',
    },
    fs: {
      allow: ['..'],
    },
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8000',
        changeOrigin: true,
      },
      // 公开分享的 RSS 端点由后端直接提供，不在 /api 下
      '/share': {
        target: 'http://127.0.0.1:8000',
        changeOrigin: true,
      },
    }
  },
})